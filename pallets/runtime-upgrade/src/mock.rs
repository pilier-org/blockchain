use crate as pallet_runtime_upgrade;
use frame_support::derive_impl;
use frame_system::EnsureRoot;
use sp_runtime::BuildStorage;

type Block = frame_system::mocking::MockBlock<Test>;

/// `frame_system`'s `TestDefaultConfig` already sets `AccountId = u64`; this alias just makes
/// the intent readable at the call sites below (`RuntimeOrigin::signed(1)` and so on).
pub type AccountId = u64;

#[frame_support::runtime]
mod runtime {
    // The main runtime
    #[runtime::runtime]
    // Runtime Types to be generated
    #[runtime::derive(
        RuntimeCall,
        RuntimeEvent,
        RuntimeError,
        RuntimeOrigin,
        RuntimeFreezeReason,
        RuntimeHoldReason,
        RuntimeSlashReason,
        RuntimeLockId,
        RuntimeTask
    )]
    pub struct Test;

    #[runtime::pallet_index(0)]
    pub type System = frame_system::Pallet<Test>;

    #[runtime::pallet_index(1)]
    pub type RuntimeUpgrade = pallet_runtime_upgrade::Pallet<Test>;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
}

impl pallet_runtime_upgrade::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    // Stand-in for "council supermajority, or root" until the runtime wires the real
    // `CouncilOrRoot`; root lets us assert both that an authorised call (root) succeeds and that
    // an unauthorised one (any merely-signed account) is rejected. The runtime's own tests
    // (`runtime/src/tests.rs`) exercise the real council vote through `Council::propose`,
    // `Council::vote` and `Council::close`.
    type AuthorizeOrigin = EnsureRoot<AccountId>;
    type WeightInfo = ();
}

/// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let storage = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    storage.into()
}
