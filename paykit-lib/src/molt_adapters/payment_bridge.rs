//! S9 `PaymentPluginBridge`: wraps any existing
//! [`crate::methods::PaymentMethodPlugin`] as a Molt adapter
//! `Value{…, Self} → Value{…, Counterparty}`.
//!
//! Per-method manifests (plan v14, S9 table):
//!
//! - **onchain** (`Value{bitcoin}`): the `Chain` witness learns
//!   `NETWORK_IDENTIFIER | AMOUNT | TIME`; the hop preserves
//!   `{AMOUNT, "btc.sats"}` and `{TIME, "time.unix"}`. No segment.
//! - **bolt11** (`Value{lightning}`): the `LnPeer` witness learns
//!   `AMOUNT | TIME | NETWORK_LOCATION`; the hop opens **and** closes a
//!   segment carrying `{TRANSACTION_ID, "lightning.payment_hash"}` (the hash
//!   must stay continuous across the HTLC path and must not cross past the
//!   close), and preserves `{AMOUNT, "btc.sats"}` / `{TIME, "time.unix"}`.
//! - **other methods**: a deliberately conservative manifest — the
//!   counterparty learns `RELATIONSHIP_IDENTITY | AMOUNT | TIME` and
//!   amount/time are preserved — until the method ships its own Molt
//!   profile. See `DECISIONS.md`.

use crate::methods::PaymentMethodPlugin;
use crate::MethodId;
use pubky_molt::route::{Adapter, Holder, Quote, RecoverySemantics, RouteState};
use pubky_molt::witness::{
    CorrelatorSpec, Field, Manifest, ObservationDomain, OperatorId, Segment, SegmentEffects,
    SegmentId, Witness, WitnessRole,
};
use std::sync::Arc;

/// Value network name for on-chain (Bitcoin) payments.
pub const BITCOIN_NETWORK: &str = "bitcoin";
/// Value network name for Lightning (bolt11) payments.
pub const LIGHTNING_NETWORK: &str = "lightning";

/// Adapter id for the on-chain bridge.
pub const ONCHAIN_BRIDGE_ADAPTER_ID: &str = "paykit.molt.payment_bridge.onchain";
/// Adapter id for the bolt11 (Lightning) bridge.
pub const BOLT11_BRIDGE_ADAPTER_ID: &str = "paykit.molt.payment_bridge.bolt11";

/// Segment id of the Lightning payment-hash continuity span.
pub const BOLT11_PAYMENT_HASH_SEGMENT: &str = "paykit.molt.payment_bridge.bolt11.payment_hash";

/// Namespace of the satoshi amount correlator.
pub const BTC_SATS_NAMESPACE: &str = "btc.sats";
/// Namespace of the unix-time correlator.
pub const TIME_UNIX_NAMESPACE: &str = "time.unix";
/// Namespace of the Lightning payment-hash correlator.
pub const LIGHTNING_PAYMENT_HASH_NAMESPACE: &str = "lightning.payment_hash";

/// Which value network a wrapped payment method settles on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentNetworkKind {
    /// On-chain Bitcoin (`Value{bitcoin}`).
    Onchain,
    /// Lightning via bolt11 invoices (`Value{lightning}`).
    Lightning,
    /// Any other method; the bridge declares a conservative generic manifest
    /// and routes on the method id as the network name.
    Other(String),
}

impl PaymentNetworkKind {
    /// Map a Paykit [`MethodId`] to its value network. `"onchain"` is
    /// on-chain; `"lightning"` and `"bolt11"` settle on Lightning; anything
    /// else is [`PaymentNetworkKind::Other`].
    pub fn for_method(method_id: &MethodId) -> Self {
        match method_id.0.as_str() {
            "onchain" => PaymentNetworkKind::Onchain,
            "lightning" | "bolt11" => PaymentNetworkKind::Lightning,
            other => PaymentNetworkKind::Other(other.to_string()),
        }
    }

    /// The [`RouteState::Value`] network name for this kind.
    pub fn network(&self) -> String {
        match self {
            PaymentNetworkKind::Onchain => BITCOIN_NETWORK.to_string(),
            PaymentNetworkKind::Lightning => LIGHTNING_NETWORK.to_string(),
            PaymentNetworkKind::Other(id) => id.clone(),
        }
    }

    fn adapter_id(&self) -> String {
        match self {
            PaymentNetworkKind::Onchain => ONCHAIN_BRIDGE_ADAPTER_ID.to_string(),
            PaymentNetworkKind::Lightning => BOLT11_BRIDGE_ADAPTER_ID.to_string(),
            PaymentNetworkKind::Other(id) => format!("paykit.molt.payment_bridge.{id}"),
        }
    }
}

fn spec(kind: Field, namespace: &str) -> CorrelatorSpec {
    // All namespaces used here are compile-time constants that satisfy the
    // grammar; `CorrelatorSpec::new` validates them again at construction.
    CorrelatorSpec::new(kind, namespace).expect("constant correlator namespace is valid")
}

fn amount_time_preserves() -> Vec<CorrelatorSpec> {
    vec![
        spec(Field::AMOUNT, BTC_SATS_NAMESPACE),
        spec(Field::TIME, TIME_UNIX_NAMESPACE),
    ]
}

/// Molt adapter bridging a Paykit payment method onto its value network.
///
/// The bridge is manifest-level only in v1: it declares what executing the
/// payment discloses; actual execution stays with the wrapped
/// [`PaymentMethodPlugin`].
pub struct PaymentPluginBridge {
    plugin: Arc<dyn PaymentMethodPlugin>,
    kind: PaymentNetworkKind,
    id: String,
}

impl std::fmt::Debug for PaymentPluginBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaymentPluginBridge")
            .field("method_id", &self.plugin.method_id())
            .field("kind", &self.kind)
            .finish()
    }
}

impl PaymentPluginBridge {
    /// Wrap a payment method plugin. The value network is derived from the
    /// plugin's [`MethodId`] via [`PaymentNetworkKind::for_method`].
    pub fn new(plugin: Arc<dyn PaymentMethodPlugin>) -> Self {
        let kind = PaymentNetworkKind::for_method(&plugin.method_id());
        let id = kind.adapter_id();
        PaymentPluginBridge { plugin, kind, id }
    }

    /// The wrapped plugin.
    pub fn plugin(&self) -> &Arc<dyn PaymentMethodPlugin> {
        &self.plugin
    }

    /// The value network kind this bridge settles on.
    pub fn kind(&self) -> &PaymentNetworkKind {
        &self.kind
    }

    /// The state this adapter accepts for `network`
    /// (`Value{network, _, Self}`).
    pub fn accepts_state(network: &str) -> RouteState {
        RouteState::Value {
            network: network.into(),
            amount: None,
            holder: Holder::Self_,
        }
    }

    fn manifest_witnesses(&self) -> Vec<Witness> {
        let no_domains = Vec::<ObservationDomain>::new();
        match &self.kind {
            PaymentNetworkKind::Onchain => vec![Witness {
                role: WitnessRole::Chain,
                operator: OperatorId("unknown".into()),
                domains: no_domains,
                learns_in: Field::NETWORK_IDENTIFIER | Field::AMOUNT | Field::TIME,
                learns_out: Field::NETWORK_IDENTIFIER | Field::AMOUNT | Field::TIME,
            }],
            PaymentNetworkKind::Lightning => vec![Witness {
                role: WitnessRole::LnPeer,
                operator: OperatorId("unknown".into()),
                domains: no_domains,
                learns_in: Field::AMOUNT | Field::TIME | Field::NETWORK_LOCATION,
                learns_out: Field::AMOUNT | Field::TIME | Field::NETWORK_LOCATION,
            }],
            PaymentNetworkKind::Other(_) => vec![Witness {
                role: WitnessRole::Counterparty,
                operator: OperatorId("counterparty".into()),
                domains: no_domains,
                learns_in: Field::RELATIONSHIP_IDENTITY | Field::AMOUNT | Field::TIME,
                learns_out: Field::RELATIONSHIP_IDENTITY | Field::AMOUNT | Field::TIME,
            }],
        }
    }
}

impl Adapter for PaymentPluginBridge {
    fn id(&self) -> &str {
        &self.id
    }

    fn accepts(&self, s: &RouteState) -> bool {
        matches!(
            s,
            RouteState::Value {
                network,
                holder: Holder::Self_,
                ..
            } if network == &self.kind.network()
        )
    }

    fn produces(&self, s: &RouteState) -> Option<RouteState> {
        match s {
            RouteState::Value {
                network,
                amount,
                holder: Holder::Self_,
            } if network == &self.kind.network() => Some(RouteState::Value {
                network: network.clone(),
                amount: amount.clone(),
                holder: Holder::Counterparty,
            }),
            _ => None,
        }
    }

    fn manifest(&self) -> Manifest {
        Manifest {
            adapter_id: self.kind.adapter_id(),
            witnesses: self.manifest_witnesses(),
            preserves: amount_time_preserves(),
            // Settlement latency is not bounded by the method; unknown is
            // conservatively treated as within the scorer's time window.
            latency_bound_secs: None,
        }
    }

    fn quote(&self, _s: &RouteState) -> Quote {
        Quote {
            // Fees are method- and traffic-dependent and unknown at planning
            // time; declared as no quoted cost (see DECISIONS.md).
            costs: Vec::new(),
            latency_secs: match self.kind {
                PaymentNetworkKind::Onchain => 600,
                PaymentNetworkKind::Lightning => 10,
                PaymentNetworkKind::Other(_) => 60,
            },
        }
    }

    fn recovery(&self) -> RecoverySemantics {
        match self.kind {
            // A bolt11 payment either settles across the path or fails back.
            PaymentNetworkKind::Lightning => RecoverySemantics::Atomic,
            // A broadcast transaction offers no protocol-level recovery.
            PaymentNetworkKind::Onchain | PaymentNetworkKind::Other(_) => {
                RecoverySemantics::BestEffort
            }
        }
    }

    fn segments(&self) -> SegmentEffects {
        match &self.kind {
            PaymentNetworkKind::Lightning => {
                let segment = Segment {
                    id: SegmentId(BOLT11_PAYMENT_HASH_SEGMENT.into()),
                    carries: vec![spec(
                        Field::TRANSACTION_ID,
                        LIGHTNING_PAYMENT_HASH_NAMESPACE,
                    )],
                };
                SegmentEffects {
                    opens: vec![segment.clone()],
                    continues: Vec::new(),
                    closes: vec![segment.id],
                }
            }
            _ => SegmentEffects::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::methods::{LightningPlugin, OnchainPlugin};

    fn onchain_bridge() -> PaymentPluginBridge {
        PaymentPluginBridge::new(Arc::new(OnchainPlugin::new()))
    }

    fn bolt11_bridge() -> PaymentPluginBridge {
        PaymentPluginBridge::new(Arc::new(LightningPlugin::new()))
    }

    fn value(network: &str, holder: Holder) -> RouteState {
        RouteState::Value {
            network: network.into(),
            amount: None,
            holder,
        }
    }

    #[test]
    fn onchain_manifest_matches_spec_table() {
        let ad = onchain_bridge();
        let m = ad.manifest();
        assert_eq!(m.adapter_id, ONCHAIN_BRIDGE_ADAPTER_ID);
        assert_eq!(m.witnesses.len(), 1);
        let chain = &m.witnesses[0];
        assert_eq!(chain.role, WitnessRole::Chain);
        let expected = Field::NETWORK_IDENTIFIER | Field::AMOUNT | Field::TIME;
        assert_eq!(chain.learns_in, expected);
        assert_eq!(chain.learns_out, expected);
        assert_eq!(
            m.preserves,
            vec![
                spec(Field::AMOUNT, BTC_SATS_NAMESPACE),
                spec(Field::TIME, TIME_UNIX_NAMESPACE),
            ]
        );
        assert!(ad.segments().opens.is_empty());
        assert_eq!(ad.recovery(), RecoverySemantics::BestEffort);
    }

    #[test]
    fn bolt11_manifest_and_segment_match_spec_table() {
        let ad = bolt11_bridge();
        let m = ad.manifest();
        assert_eq!(m.adapter_id, BOLT11_BRIDGE_ADAPTER_ID);
        assert_eq!(m.witnesses.len(), 1);
        let peer = &m.witnesses[0];
        assert_eq!(peer.role, WitnessRole::LnPeer);
        let expected = Field::AMOUNT | Field::TIME | Field::NETWORK_LOCATION;
        assert_eq!(peer.learns_in, expected);
        assert_eq!(peer.learns_out, expected);

        // Opens AND closes the payment-hash segment in one hop; the hash is
        // carried inside the segment and never listed in preserves.
        let effects = ad.segments();
        assert_eq!(effects.opens.len(), 1);
        let seg = &effects.opens[0];
        assert_eq!(seg.id, SegmentId(BOLT11_PAYMENT_HASH_SEGMENT.into()));
        assert_eq!(
            seg.carries,
            vec![spec(
                Field::TRANSACTION_ID,
                LIGHTNING_PAYMENT_HASH_NAMESPACE
            )]
        );
        assert_eq!(effects.closes, vec![seg.id.clone()]);
        assert!(!m.preserves.iter().any(|p| p.kind == Field::TRANSACTION_ID));
        assert_eq!(ad.recovery(), RecoverySemantics::Atomic);
    }

    #[test]
    fn bridge_accepts_self_value_and_produces_counterparty_value() {
        let ad = bolt11_bridge();
        let from = RouteState::Value {
            network: LIGHTNING_NETWORK.into(),
            amount: Some(pubky_molt::route::Amount {
                asset: "BTC".into(),
                units: "sat".into(),
                value: 21_000,
            }),
            holder: Holder::Self_,
        };
        assert!(ad.accepts(&from));
        let out = ad.produces(&from).expect("produces");
        match out {
            RouteState::Value {
                network,
                amount,
                holder,
            } => {
                assert_eq!(network, LIGHTNING_NETWORK);
                assert_eq!(holder, Holder::Counterparty);
                assert_eq!(amount.expect("amount preserved").value, 21_000);
            }
            other => panic!("unexpected state {other:?}"),
        }
        // Amount-less states are accepted too.
        assert!(ad.accepts(&value(LIGHTNING_NETWORK, Holder::Self_)));
    }

    #[test]
    fn bridge_rejects_wrong_network_and_holder() {
        let ad = onchain_bridge();
        for bad in [
            value(LIGHTNING_NETWORK, Holder::Self_),
            value(BITCOIN_NETWORK, Holder::Counterparty),
            value(BITCOIN_NETWORK, Holder::Intermediary),
            RouteState::Identity {
                scope: pubky_molt::route::IdentityScope::Root,
                holder: Holder::Self_,
            },
        ] {
            assert!(!ad.accepts(&bad), "accepted {bad:?}");
            assert_eq!(ad.produces(&bad), None);
        }
    }

    #[test]
    fn network_kind_mapping() {
        assert_eq!(
            PaymentNetworkKind::for_method(&MethodId::new("onchain")),
            PaymentNetworkKind::Onchain
        );
        assert_eq!(
            PaymentNetworkKind::for_method(&MethodId::new("lightning")),
            PaymentNetworkKind::Lightning
        );
        assert_eq!(
            PaymentNetworkKind::for_method(&MethodId::new("bolt11")),
            PaymentNetworkKind::Lightning
        );
        assert_eq!(
            PaymentNetworkKind::for_method(&MethodId::new("ethereum")),
            PaymentNetworkKind::Other("ethereum".into())
        );
        assert_eq!(PaymentNetworkKind::Onchain.network(), BITCOIN_NETWORK);
        assert_eq!(PaymentNetworkKind::Lightning.network(), LIGHTNING_NETWORK);
        assert_eq!(PaymentNetworkKind::Other("x".into()).network(), "x");
    }
}
