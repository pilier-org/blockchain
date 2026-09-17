//! # Runtime Upgrade Pallet
//!
//! Lets a configurable origin authorize a runtime code hash for
//! [`frame_system::Pallet::apply_authorized_upgrade`], the same public, permissionless call
//! `frame_system` already exposes for anyone to submit the actual (large) runtime blob once its
//! hash has been authorized.
//!
//! `frame_system` itself only lets Root authorize an upgrade (`authorize_upgrade`,
//! `authorize_upgrade_without_checks`). This pallet exists so that Pilier's validators' council
//! can authorize an upgrade by its own supermajority vote, with Root kept only as an emergency
//! lever — the same shape the validator set and the registry/passport pallets already use for
//! their own origins. It never bypasses `frame_system`'s own version and name checks: the single
//! call here always authorizes with `check_version = true`, exactly like `authorize_upgrade`
//! does; there is no path that mirrors `authorize_upgrade_without_checks`.

#![cfg_attr(not(feature = "std"), no_std)]

// Re-export pallet items so they can be accessed from the crate namespace.
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

/// Weight functions needed for this pallet's dispatchables.
///
/// This is a testnet-phase pallet with no generated (benchmarked) weights yet; `()` is a valid
/// implementation below that charges a small placeholder weight. Replace with generated weights
/// once `runtime-benchmarks` support is added for this pallet.
pub trait WeightInfo {
    /// Weight for [`Pallet::authorize_upgrade`].
    fn authorize_upgrade() -> frame_support::pallet_prelude::Weight;
}

impl WeightInfo for () {
    fn authorize_upgrade() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0)
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::WeightInfo;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    /// The pallet's placeholder struct, used to implement traits, methods and dispatchables.
    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The origin allowed to authorize a runtime upgrade. In the runtime this is wired to
        /// "council supermajority, or root as an emergency lever" (`CouncilOrRoot`); unit tests
        /// use a fixed signed account instead.
        type AuthorizeOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Weight information for this pallet's dispatchables.
        type WeightInfo: WeightInfo;
    }

    /// Events that functions in this pallet can emit.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// `code_hash` was authorized as the next runtime upgrade by `T::AuthorizeOrigin`.
        UpgradeAuthorized { code_hash: T::Hash },
    }

    /// The pallet's dispatchable functions ("calls").
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Authorize `code_hash` as the next runtime upgrade.
        ///
        /// Must be called by `T::AuthorizeOrigin`. Records the authorization in `frame_system`
        /// exactly as `frame_system::authorize_upgrade` would under Root — with the version and
        /// spec-name checks `apply_authorized_upgrade` always runs still in force
        /// (`check_version = true`) — the only difference is which origin may call it. Anyone can
        /// then submit the matching runtime blob via `frame_system::apply_authorized_upgrade`;
        /// that call checks the blob's hash against this authorization, checks its name and
        /// version, applies it, and clears the authorization (so a second submission fails with
        /// `frame_system::Error::NothingAuthorized`).
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::authorize_upgrade())]
        pub fn authorize_upgrade(origin: OriginFor<T>, code_hash: T::Hash) -> DispatchResult {
            T::AuthorizeOrigin::ensure_origin(origin)?;

            frame_system::Pallet::<T>::do_authorize_upgrade(code_hash, true);
            Self::deposit_event(Event::UpgradeAuthorized { code_hash });

            Ok(())
        }
    }
}
