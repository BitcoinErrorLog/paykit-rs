//! S9 `DropTransportAdapter`: `Transport{pubky-storage, Self} →
//! Transport{drop, Self}`.
//!
//! The adapter models moving relationship traffic off publicly rooted
//! homeserver storage onto S8 Drop dead-drop channels. Its manifest is the
//! relay's own S8 manifest — the same JSON `pubky-core`'s
//! `drop_relay_manifest()` returns, embedded verbatim as
//! [`DROP_RELAY_MANIFEST_JSON`]: the `RelayOperator` learns
//! `NETWORK_LOCATION | TIME | CONTENT_SIZE | RELATIONSHIP_LINK` (poll-pattern
//! linkage only; channel ids and bodies are opaque to the relay) and
//! preserves no correlator across the hop.
//!
//! The adapter opens no segments.

use pubky_molt::route::{Adapter, Holder, Quote, RecoverySemantics, RouteState};
use pubky_molt::witness::{
    Field, Manifest, ObservationDomain, OperatorId, SegmentEffects, Witness, WitnessRole,
};

/// Adapter id of the executing Drop transport adapter. Matches the
/// `adapter_id` inside [`DROP_RELAY_MANIFEST_JSON`].
pub const DROP_TRANSPORT_ADAPTER_ID: &str = "http-relay.drop.v1";

/// Transport network name accepted by this adapter (publicly rooted
/// homeserver storage).
pub const PUBKY_STORAGE_NETWORK: &str = "pubky-storage";
/// Transport network name produced by this adapter (S8 Drop dead-drops).
pub const DROP_NETWORK: &str = "drop";
/// Endpoint kind produced by this adapter: an opaque, rotating channel id.
pub const DROP_ENDPOINT_KIND: &str = "opaque-channel";

/// The S8 Drop relay manifest, byte-for-byte the JSON returned by
/// `pubky-core`'s `http-relay::drop_relay_manifest()` (quoted in plan v11,
/// S9). The executing adapter's [`Manifest`] is field-for-field equivalent;
/// the JSON is embedded for cross-implementation pinning because
/// `pubky-molt`'s `Field` text form (`"A | B"`) differs from this JSON's
/// array-of-strings form (see `DECISIONS.md`).
pub const DROP_RELAY_MANIFEST_JSON: &str = r#"{
    "adapter_id": "http-relay.drop.v1",
    "witnesses": [
        {
            "role": "RelayOperator",
            "operator": "unknown",
            "domains": [],
            "learns_in": ["NETWORK_LOCATION", "TIME", "CONTENT_SIZE", "RELATIONSHIP_LINK"],
            "learns_out": ["NETWORK_LOCATION", "TIME", "CONTENT_SIZE", "RELATIONSHIP_LINK"]
        }
    ],
    "preserves": [],
    "latency_bound_secs": null
}"#;

/// Molt adapter moving relationship traffic from publicly rooted homeserver
/// storage onto S8 Drop channels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DropTransportAdapter;

impl DropTransportAdapter {
    /// Construct the adapter.
    pub fn new() -> Self {
        DropTransportAdapter
    }

    /// The state this adapter accepts: `Transport{pubky-storage, _, Self}`.
    pub fn accepts_state() -> RouteState {
        RouteState::Transport {
            network: PUBKY_STORAGE_NETWORK.into(),
            endpoint_kind: "public-path".into(),
            holder: Holder::Self_,
        }
    }

    /// The state this adapter produces: `Transport{drop, opaque-channel,
    /// Self}`.
    pub fn produces_state() -> RouteState {
        RouteState::Transport {
            network: DROP_NETWORK.into(),
            endpoint_kind: DROP_ENDPOINT_KIND.into(),
            holder: Holder::Self_,
        }
    }

    /// The relay manifest as a `pubky-molt` [`Manifest`], field-for-field
    /// equivalent to [`DROP_RELAY_MANIFEST_JSON`].
    pub fn relay_manifest() -> Manifest {
        let relay_learns =
            Field::NETWORK_LOCATION | Field::TIME | Field::CONTENT_SIZE | Field::RELATIONSHIP_LINK;
        Manifest {
            adapter_id: DROP_TRANSPORT_ADAPTER_ID.into(),
            witnesses: vec![Witness {
                role: WitnessRole::RelayOperator,
                operator: OperatorId("unknown".into()),
                domains: Vec::<ObservationDomain>::new(),
                learns_in: relay_learns,
                learns_out: relay_learns,
            }],
            preserves: Vec::new(),
            latency_bound_secs: None,
        }
    }
}

impl Adapter for DropTransportAdapter {
    fn id(&self) -> &str {
        DROP_TRANSPORT_ADAPTER_ID
    }

    fn accepts(&self, s: &RouteState) -> bool {
        matches!(
            s,
            RouteState::Transport {
                network,
                holder: Holder::Self_,
                ..
            } if network == PUBKY_STORAGE_NETWORK
        )
    }

    fn produces(&self, s: &RouteState) -> Option<RouteState> {
        self.accepts(s).then(Self::produces_state)
    }

    fn manifest(&self) -> Manifest {
        Self::relay_manifest()
    }

    fn quote(&self, _s: &RouteState) -> Quote {
        Quote {
            // Appending to a Drop channel has no monetary cost.
            costs: Vec::new(),
            // Store-and-forward poll interval; illustrative, preference-only.
            latency_secs: 60,
        }
    }

    fn recovery(&self) -> RecoverySemantics {
        // The relay offers no delivery guarantee (TTL, bounded queues);
        // duplicates from a retried PUT are rejected by the receiver's
        // ratchet replay protection.
        RecoverySemantics::BestEffort
    }

    fn segments(&self) -> SegmentEffects {
        SegmentEffects::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_manifest_matches_relay_json() {
        let m = DropTransportAdapter::relay_manifest();
        let j: serde_json::Value =
            serde_json::from_str(DROP_RELAY_MANIFEST_JSON).expect("embedded JSON parses");

        assert_eq!(m.adapter_id, j["adapter_id"].as_str().expect("id"));
        assert_eq!(m.witnesses.len(), 1);
        let w = &m.witnesses[0];
        let jw = &j["witnesses"][0];
        assert_eq!(jw["role"].as_str().expect("role"), "RelayOperator");
        assert_eq!(w.role, WitnessRole::RelayOperator);
        assert_eq!(w.operator.0, jw["operator"].as_str().expect("operator"));
        assert!(w.domains.is_empty());
        assert_eq!(jw["domains"].as_array().expect("domains").len(), 0);

        let expected: Field = jw["learns_in"]
            .as_array()
            .expect("learns_in array")
            .iter()
            .map(|s| match s.as_str().expect("field name") {
                "NETWORK_LOCATION" => Field::NETWORK_LOCATION,
                "TIME" => Field::TIME,
                "CONTENT_SIZE" => Field::CONTENT_SIZE,
                "RELATIONSHIP_LINK" => Field::RELATIONSHIP_LINK,
                other => panic!("unexpected field {other}"),
            })
            .fold(Field::empty(), |acc, f| acc | f);
        assert_eq!(w.learns_in, expected);
        assert_eq!(w.learns_out, expected);

        assert!(m.preserves.is_empty());
        assert_eq!(j["preserves"].as_array().expect("preserves").len(), 0);
        assert!(m.latency_bound_secs.is_none());
        assert!(j["latency_bound_secs"].is_null());
    }

    #[test]
    fn drop_adapter_accepts_pubky_storage_and_produces_drop() {
        let ad = DropTransportAdapter::new();
        let from = DropTransportAdapter::accepts_state();
        assert!(ad.accepts(&from));
        assert_eq!(
            ad.produces(&from),
            Some(DropTransportAdapter::produces_state())
        );
        // Acceptance ignores the endpoint kind (any pubky-storage endpoint).
        let any_endpoint = RouteState::Transport {
            network: PUBKY_STORAGE_NETWORK.into(),
            endpoint_kind: "whatever".into(),
            holder: Holder::Self_,
        };
        assert!(ad.accepts(&any_endpoint));
        assert_eq!(ad.recovery(), RecoverySemantics::BestEffort);
        assert!(ad.segments().opens.is_empty());
        assert!(ad.route_constraints().is_empty());
    }

    #[test]
    fn drop_adapter_rejects_other_networks_and_holders() {
        let ad = DropTransportAdapter::new();
        for bad in [
            RouteState::Transport {
                network: DROP_NETWORK.into(),
                endpoint_kind: DROP_ENDPOINT_KIND.into(),
                holder: Holder::Self_,
            },
            RouteState::Transport {
                network: PUBKY_STORAGE_NETWORK.into(),
                endpoint_kind: "public-path".into(),
                holder: Holder::Counterparty,
            },
            RouteState::Identity {
                scope: pubky_molt::route::IdentityScope::Root,
                holder: Holder::Self_,
            },
        ] {
            assert!(!ad.accepts(&bad), "accepted {bad:?}");
            assert_eq!(ad.produces(&bad), None);
        }
    }
}
