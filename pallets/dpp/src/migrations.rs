//! Storage migrations for this pallet.
//!
//! This pallet is expected to arrive on an already-running chain by a forkless runtime upgrade
//! rather than by a fresh genesis, so [`PublicationPrice`](crate::PublicationPrice) needs a
//! starting value seeded by code, not by a genesis config that would never run for it.

use crate::{BalanceOf, Config, Pallet, PublicationPrice};
use frame_support::{
    migrations::VersionedMigration,
    traits::{Get, UncheckedOnRuntimeUpgrade},
    weights::Weight,
};

/// The publication price this migration seeds a version-0 storage with: four thousand of the
/// chain's smallest unit — 0.004 PIL at the runtime's one-million-unit token, the figure the
/// published tokenomics names for creating a passport.
const INITIAL_PUBLICATION_PRICE: u32 = 4_000;

/// The unversioned migration logic. Kept private so it can only be reached through
/// [`InitializePublicationPrice`], which guards it with the pallet's on-chain storage version —
/// calling this directly, outside that guard, would reseed the price on every runtime upgrade
/// this pallet is part of, not just the first one.
mod version_unchecked {
    use super::*;

    pub struct InitializePublicationPrice<T>(core::marker::PhantomData<T>);

    impl<T: Config> UncheckedOnRuntimeUpgrade for InitializePublicationPrice<T> {
        fn on_runtime_upgrade() -> Weight {
            PublicationPrice::<T>::put(BalanceOf::<T>::from(INITIAL_PUBLICATION_PRICE));
            <T as frame_system::Config>::DbWeight::get().writes(1)
        }
    }
}

/// Seeds [`PublicationPrice`](crate::PublicationPrice) with its initial value and raises this
/// pallet's on-chain storage version from 0 to 1. A no-op if the on-chain version is already 1
/// or higher, so re-running it — for example if a later forkless upgrade bundles it again by
/// mistake — touches nothing: neither the price nor the version.
pub type InitializePublicationPrice<T> = VersionedMigration<
    0,
    1,
    version_unchecked::InitializePublicationPrice<T>,
    Pallet<T>,
    <T as frame_system::Config>::DbWeight,
>;
