//! Molt v1 adapters — Paykit as a Molt client (plan v11, S9).
//!
//! Paykit supplies three [`pubky_molt::route::Adapter`] implementations to the
//! protocol-neutral routing core in `pubky-molt`:
//!
//! - [`intro_adapter::IntroAdapter`]:
//!   `Identity{Root, Self} → Identity{Pairwise, Self}`, with one manifest per
//!   intro authenticity mode (SessionAuthenticated vs ExternallyAuthenticated).
//! - [`drop_adapter::DropTransportAdapter`]:
//!   `Transport{pubky-storage, Self} → Transport{drop, Self}`, carrying the
//!   S8 Drop relay manifest (embedded verbatim as
//!   [`drop_adapter::DROP_RELAY_MANIFEST_JSON`]).
//! - [`payment_bridge::PaymentPluginBridge`]: wraps any existing
//!   [`crate::methods::PaymentMethodPlugin`] as
//!   `Value{…, Self} → Value{…, Counterparty}` with per-method manifests
//!   (onchain and bolt11 in v1).
//!
//! No routing logic lives here: planning and scoring are delegated to
//! `pubky-molt` via [`crate::selection::select_route`].

pub mod drop_adapter;
pub mod intro_adapter;
pub mod payment_bridge;

pub use drop_adapter::DropTransportAdapter;
pub use intro_adapter::IntroAdapter;
pub use payment_bridge::{PaymentNetworkKind, PaymentPluginBridge};

/// Bridge newtype converting between `pubky-molt`'s own
/// [`pubky_molt::PeerId`] and `pubky-crypto`'s
/// [`pubky_crypto::molt::PeerId`].
///
/// Both crates deliberately define their own `PeerId([u8; 32])` newtype so
/// the routing core depends on no other Pubky crate. Rust's orphan rule
/// forbids implementing `From` between two foreign types, so the conversion
/// lives on this local newtype (see `DECISIONS.md`):
///
/// ```
/// use paykit_lib::molt_adapters::PeerIdBridge;
///
/// let crypto_id = pubky_crypto::molt::PeerId([7u8; 32]);
/// let route_id: pubky_molt::PeerId = PeerIdBridge::from(crypto_id).into();
/// let back: pubky_crypto::molt::PeerId = PeerIdBridge::from(route_id).into();
/// assert_eq!(crypto_id, back);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PeerIdBridge(pub [u8; 32]);

impl From<pubky_crypto::molt::PeerId> for PeerIdBridge {
    fn from(id: pubky_crypto::molt::PeerId) -> Self {
        PeerIdBridge(id.0)
    }
}

impl From<pubky_molt::PeerId> for PeerIdBridge {
    fn from(id: pubky_molt::PeerId) -> Self {
        PeerIdBridge(*id.as_bytes())
    }
}

impl From<PeerIdBridge> for pubky_crypto::molt::PeerId {
    fn from(b: PeerIdBridge) -> Self {
        pubky_crypto::molt::PeerId(b.0)
    }
}

impl From<PeerIdBridge> for pubky_molt::PeerId {
    fn from(b: PeerIdBridge) -> Self {
        pubky_molt::PeerId::new(b.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_id_conversion_round_trips_both_directions() {
        let bytes = [0xabu8; 32];
        let crypto_id = pubky_crypto::molt::PeerId(bytes);
        let route_id = pubky_molt::PeerId::new(bytes);

        let to_route: pubky_molt::PeerId = PeerIdBridge::from(crypto_id).into();
        assert_eq!(to_route, route_id);

        let to_crypto: pubky_crypto::molt::PeerId = PeerIdBridge::from(route_id).into();
        assert_eq!(to_crypto, crypto_id);

        // Full round trip preserves the raw bytes exactly.
        let via_route: pubky_molt::PeerId = PeerIdBridge::from(crypto_id).into();
        let back_crypto: pubky_crypto::molt::PeerId = PeerIdBridge::from(via_route).into();
        assert_eq!(back_crypto, crypto_id);
    }

    #[test]
    fn peer_id_conversion_is_not_identity_swapping() {
        // Distinct inputs must not collapse onto one output.
        let a = pubky_crypto::molt::PeerId([1u8; 32]);
        let b = pubky_crypto::molt::PeerId([2u8; 32]);
        let ra: pubky_molt::PeerId = PeerIdBridge::from(a).into();
        let rb: pubky_molt::PeerId = PeerIdBridge::from(b).into();
        assert_ne!(ra, rb);
    }
}
