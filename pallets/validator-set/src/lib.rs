//! # Validator Set Pallet
//!
//! Stores the current set of validator account IDs for Pilier's Proof-of-Authority chain and
//! feeds that list to `pallet-session` (which in turn drives Aura block authoring and GRANDPA
//! finality) through the [`pallet_session::SessionManager`] trait.
//!
//! The set can only be changed through `T::AddRemoveOrigin` — in the runtime this will be
//! wired to "the validators' council reaches its threshold, or the root (Sudo) key acts as an
//! emergency lever". This pallet itself has no opinion on what that origin is; it only enforces
//! that some origin passed `ensure_origin` before mutating storage.
//!
//! This is a small, purpose-built pallet rather than a dependency on an external crate, so that
//! Pilier does not have to keep an extra crate's release cadence in sync with the `polkadot-sdk`
//! branch it tracks. Its logic was checked against `substrate-validator-set`
//! (<https://github.com/gautamdhameja/substrate-validator-set>) as an external reference, but the
//! code here is original and deliberately simpler: it keys everything off `T::AccountId` directly
//! instead of introducing a separate `ValidatorId` type.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

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
    /// Weight for [`Pallet::add_validator`].
    fn add_validator() -> frame_support::pallet_prelude::Weight;
    /// Weight for [`Pallet::remove_validator`].
    fn remove_validator() -> frame_support::pallet_prelude::Weight;
}

impl WeightInfo for () {
    fn add_validator() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0)
    }
    fn remove_validator() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0)
    }
}

#[frame_support::pallet]
pub mod pallet {
    use super::{Vec, WeightInfo};
    use frame_support::pallet_prelude::*;
    use frame_support::traits::ChangeMembers;
    use frame_system::pallet_prelude::*;

    /// The pallet's placeholder struct, used to implement traits, methods and dispatchables.
    ///
    /// `without_storage_info` is required because `Validators` stores a plain, unbounded `Vec`
    /// rather than a `BoundedVec` — there is no hard cap on the validator set size, and this
    /// pallet is not used with `#[pallet::without_storage_info]`-incompatible tooling (such as
    /// the storage-info-driven proof size estimation some parachains rely on), so the trade-off
    /// is acceptable for a solo Proof-of-Authority chain with a small, admin/council-controlled
    /// validator set.
    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching runtime event type.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The origin allowed to add or remove a validator. In the runtime this is wired to
        /// "council supermajority, or root as an emergency lever"; unit tests use a fixed
        /// signed account instead.
        type AddRemoveOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Notified whenever the validator set changes, so that a dependent membership (for
        /// example, the validators' council) can be kept in sync with the validator set.
        type MembershipChanged: ChangeMembers<Self::AccountId>;

        /// The lower bound the validator set may never drop below. This guards against a
        /// `remove_validator` call (however it was authorised) taking the number of block
        /// producers down to a level that would stall or endanger the network.
        #[pallet::constant]
        type MinValidators: Get<u32>;

        /// Weight information for this pallet's dispatchables.
        type WeightInfo: WeightInfo;
    }

    /// The current set of validator account IDs.
    #[pallet::storage]
    pub type Validators<T: Config> = StorageValue<_, Vec<T::AccountId>, ValueQuery>;

    /// Events that functions in this pallet can emit.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A validator was added to the set.
        ValidatorAdded { who: T::AccountId },
        /// A validator was removed from the set.
        ValidatorRemoved { who: T::AccountId },
    }

    /// Errors that can be returned by this pallet.
    #[pallet::error]
    pub enum Error<T> {
        /// The account to add is already a validator.
        AlreadyValidator,
        /// The account to remove is not a validator.
        NotValidator,
        /// Removing this validator would take the set below `MinValidators`.
        TooFewValidators,
    }

    /// The pallet's genesis configuration: the validator set the chain starts with.
    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        /// The validator account IDs the chain starts with.
        pub initial_validators: Vec<T::AccountId>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            Validators::<T>::put(&self.initial_validators);
        }
    }

    /// The pallet's dispatchable functions ("calls").
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Add `who` to the validator set.
        ///
        /// Must be called by `T::AddRemoveOrigin`. The new validator's session keys should
        /// already be set in the Session pallet (via `session.set_keys`) before this call, so
        /// that it can author and finalise blocks as soon as the next session picks it up.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::add_validator())]
        pub fn add_validator(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
            T::AddRemoveOrigin::ensure_origin(origin)?;

            let mut validators = Validators::<T>::get();
            ensure!(!validators.contains(&who), Error::<T>::AlreadyValidator);

            validators.push(who.clone());
            validators.sort();
            Validators::<T>::put(&validators);

            T::MembershipChanged::change_members_sorted(
                core::slice::from_ref(&who),
                &[],
                &validators,
            );
            Self::deposit_event(Event::ValidatorAdded { who });

            Ok(())
        }

        /// Remove `who` from the validator set.
        ///
        /// Must be called by `T::AddRemoveOrigin`. Rejected if `who` is not currently a
        /// validator, or if removing it would take the set below `T::MinValidators`.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::remove_validator())]
        pub fn remove_validator(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
            T::AddRemoveOrigin::ensure_origin(origin)?;

            let mut validators = Validators::<T>::get();
            ensure!(validators.contains(&who), Error::<T>::NotValidator);
            ensure!(
                validators.len().saturating_sub(1) as u32 >= T::MinValidators::get(),
                Error::<T>::TooFewValidators
            );

            validators.retain(|v| v != &who);
            Validators::<T>::put(&validators);

            T::MembershipChanged::change_members_sorted(
                &[],
                core::slice::from_ref(&who),
                &validators,
            );
            Self::deposit_event(Event::ValidatorRemoved { who });

            Ok(())
        }
    }
}

// This is the link between our validator set and `pallet-session`: whenever session asks "who
// should the next session's validators be", we hand back whatever is currently in `Validators`.
impl<T: Config> pallet_session::SessionManager<T::AccountId> for Pallet<T> {
    fn new_session(_new_index: u32) -> Option<Vec<T::AccountId>> {
        Some(Validators::<T>::get())
    }

    fn start_session(_start_index: u32) {}

    fn end_session(_end_index: u32) {}
}
