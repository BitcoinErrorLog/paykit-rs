//! S9 `IntroAdapter`: `Identity{Root, Self} → Identity{Pairwise, Self}`.
//!
//! The adapter models the S4 intro/first-contact hop that turns a public
//! Pubky relationship into a pairwise bond. It declares one
//! [`pubky_molt::witness::Manifest`] per authenticity mode:
//!
//! - **SessionAuthenticated** (intro delivered inside a live Noise IK
//!   session, deniable): the counterparty learns `ROOT_IDENTITY` only.
//! - **ExternallyAuthenticated** (intro delivered as an SB2 to the
//!   recipient's InboxKey, transferable): additionally the recipient's
//!   homeserver necessarily learns `TIME`, `CONTENT_SIZE`, and
//!   `DEST_ENDPOINT` from storing the blob.
//!
//! The adapter opens no segments and preserves no correlators: the root
//! identity does not cross into the pairwise state.

use pubky_crypto::molt::Authenticity;
use pubky_molt::route::{Adapter, Quote, RecoverySemantics, RouteState};
use pubky_molt::route::{Holder, IdentityScope};
use pubky_molt::witness::{
    Field, Manifest, ObservationDomain, OperatorId, SegmentEffects, Witness, WitnessRole,
};

/// Adapter id for the session-authenticated intro hop.
pub const INTRO_SESSION_ADAPTER_ID: &str = "paykit.molt.intro.session";
/// Adapter id for the externally-authenticated intro hop.
pub const INTRO_EXTERNAL_ADAPTER_ID: &str = "paykit.molt.intro.external";

/// Molt adapter for the S4 intro hop (`Identity{Root} → Identity{Pairwise}`).
///
/// Construct with [`IntroAdapter::session`] or [`IntroAdapter::external`];
/// the authenticity mode selects which manifest is declared. Both modes
/// accept and produce the same states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntroAdapter {
    mode: Authenticity,
}

impl IntroAdapter {
    /// An intro delivered inside a live authenticated session (Noise IK;
    /// deniable). The counterparty learns `ROOT_IDENTITY` only.
    pub fn session() -> Self {
        IntroAdapter {
            mode: Authenticity::SessionAuthenticated,
        }
    }

    /// An intro delivered as an SB2 to the recipient's InboxKey
    /// (transferable; the body carries its own AppKey signature). Adds the
    /// recipient homeserver as a witness of `TIME | CONTENT_SIZE |
    /// DEST_ENDPOINT`.
    pub fn external() -> Self {
        IntroAdapter {
            mode: Authenticity::ExternallyAuthenticated,
        }
    }

    /// The authenticity mode this adapter declares.
    pub fn mode(&self) -> Authenticity {
        self.mode
    }

    /// The state this adapter accepts: `Identity{Root, Self}`.
    pub fn accepts_state() -> RouteState {
        RouteState::Identity {
            scope: IdentityScope::Root,
            holder: Holder::Self_,
        }
    }

    /// The state this adapter produces: `Identity{Pairwise, Self}`.
    pub fn produces_state() -> RouteState {
        RouteState::Identity {
            scope: IdentityScope::Pairwise,
            holder: Holder::Self_,
        }
    }
}

impl Adapter for IntroAdapter {
    fn id(&self) -> &str {
        match self.mode {
            Authenticity::SessionAuthenticated => INTRO_SESSION_ADAPTER_ID,
            Authenticity::ExternallyAuthenticated => INTRO_EXTERNAL_ADAPTER_ID,
        }
    }

    fn accepts(&self, s: &RouteState) -> bool {
        s == &Self::accepts_state()
    }

    fn produces(&self, s: &RouteState) -> Option<RouteState> {
        self.accepts(s).then(Self::produces_state)
    }

    fn manifest(&self) -> Manifest {
        let counterparty = Witness {
            role: WitnessRole::Counterparty,
            operator: OperatorId("counterparty".into()),
            // The counterparty's observation domain is relationship-specific
            // and unknown to the planner; bilateral knowledge of the root is
            // bounded trust, not a leak (S5).
            domains: Vec::<ObservationDomain>::new(),
            learns_in: Field::ROOT_IDENTITY,
            learns_out: Field::ROOT_IDENTITY,
        };
        let mut witnesses = vec![counterparty];
        if self.mode == Authenticity::ExternallyAuthenticated {
            // The intro is stored on the recipient's homeserver as an SB2
            // addressed to the InboxKey: the homeserver necessarily observes
            // the write time, the blob size, and the destination endpoint.
            witnesses.push(Witness {
                role: WitnessRole::Homeserver,
                operator: OperatorId("recipient-homeserver".into()),
                domains: Vec::<ObservationDomain>::new(),
                learns_in: Field::TIME | Field::CONTENT_SIZE | Field::DEST_ENDPOINT,
                learns_out: Field::empty(),
            });
        }
        Manifest {
            adapter_id: self.id().to_string(),
            witnesses,
            preserves: Vec::new(),
            // Unknown delivery latency; conservatively treated as within the
            // scorer's time window (see DECISIONS.md).
            latency_bound_secs: None,
        }
    }

    fn quote(&self, _s: &RouteState) -> Quote {
        Quote {
            // No monetary cost to an intro.
            costs: Vec::new(),
            latency_secs: match self.mode {
                // Live session connect vs. store-and-forward homeserver write.
                // Illustrative defaults; preference-only, never leak-counting.
                Authenticity::SessionAuthenticated => 5,
                Authenticity::ExternallyAuthenticated => 60,
            },
        }
    }

    fn recovery(&self) -> RecoverySemantics {
        // Re-delivering an intro is safe: intro verification is deterministic
        // and re-derivation of the bond yields the same K_AB.
        RecoverySemantics::Idempotent
    }

    fn segments(&self) -> SegmentEffects {
        SegmentEffects::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_self() -> RouteState {
        IntroAdapter::accepts_state()
    }

    fn pairwise_self() -> RouteState {
        IntroAdapter::produces_state()
    }

    #[test]
    fn intro_session_manifest_matches_spec_table() {
        let ad = IntroAdapter::session();
        let m = ad.manifest();
        assert_eq!(m.adapter_id, INTRO_SESSION_ADAPTER_ID);
        assert_eq!(m.witnesses.len(), 1);
        let cp = &m.witnesses[0];
        assert_eq!(cp.role, WitnessRole::Counterparty);
        assert_eq!(cp.learns_in, Field::ROOT_IDENTITY);
        assert_eq!(cp.learns_out, Field::ROOT_IDENTITY);
        assert!(m.preserves.is_empty());
    }

    #[test]
    fn intro_external_manifest_adds_recipient_homeserver() {
        let ad = IntroAdapter::external();
        let m = ad.manifest();
        assert_eq!(m.adapter_id, INTRO_EXTERNAL_ADAPTER_ID);
        assert_eq!(m.witnesses.len(), 2);
        let hs = &m.witnesses[1];
        assert_eq!(hs.role, WitnessRole::Homeserver);
        assert_eq!(
            hs.learns_in,
            Field::TIME | Field::CONTENT_SIZE | Field::DEST_ENDPOINT
        );
        assert_eq!(hs.learns_out, Field::empty());
    }

    #[test]
    fn intro_adapter_accepts_root_and_produces_pairwise() {
        for ad in [IntroAdapter::session(), IntroAdapter::external()] {
            assert!(ad.accepts(&root_self()));
            assert_eq!(ad.produces(&root_self()), Some(pairwise_self()));
            assert!(ad.segments().opens.is_empty());
            assert!(ad.segments().closes.is_empty());
            assert_eq!(ad.recovery(), RecoverySemantics::Idempotent);
            assert!(ad.route_constraints().is_empty());
        }
    }

    #[test]
    fn intro_adapter_rejects_non_root_or_non_self_states() {
        let ad = IntroAdapter::session();
        for bad in [
            pairwise_self(),
            RouteState::Identity {
                scope: IdentityScope::Root,
                holder: Holder::Counterparty,
            },
            RouteState::Identity {
                scope: IdentityScope::Anonymous,
                holder: Holder::Self_,
            },
            RouteState::Transport {
                network: "drop".into(),
                endpoint_kind: "opaque-channel".into(),
                holder: Holder::Self_,
            },
        ] {
            assert!(!ad.accepts(&bad), "accepted {bad:?}");
            assert_eq!(ad.produces(&bad), None);
        }
    }
}
