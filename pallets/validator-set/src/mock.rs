use crate as pallet_validator_set;
use frame_support::{derive_impl, parameter_types, traits::ChangeMembers};
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
    pub type ValidatorSet = pallet_validator_set::Pallet<Test>;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
}

parameter_types! {
    /// Chosen so that the genesis set of three validators (accounts 1, 2, 3) can shed one
    /// validator and still be above the floor, letting the same fixture cover both "remove
    /// succeeds" and "remove rejected because it would breach the floor".
    pub const MinValidators: u32 = 2;
}

/// A no-op stand-in for `ChangeMembers`, used only in these unit tests. The real runtime will
/// wire this to the validators' council so that council membership tracks the validator set;
/// here we only need to confirm the hook is invoked without panicking, so both methods are
/// empty.
pub struct TestMembershipChanged;

impl ChangeMembers<AccountId> for TestMembershipChanged {
    fn change_members_sorted(
        _incoming: &[AccountId],
        _outgoing: &[AccountId],
        _sorted_new: &[AccountId],
    ) {
    }
}

impl pallet_validator_set::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    // Stand-in for "council supermajority, or root" until the council pallet exists (see the
    // parent plan's Phase 4). Root is a realistic and simple choice for unit tests: it lets us
    // assert both that an authorised call (root) succeeds and an unauthorised one (any signed
    // account) is rejected.
    type AddRemoveOrigin = EnsureRoot<AccountId>;
    type MembershipChanged = TestMembershipChanged;
    type MinValidators = MinValidators;
    type WeightInfo = ();
}

/// Build genesis storage according to the mock runtime, seeded with three validators
/// (accounts 1, 2, 3).
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut storage = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    pallet_validator_set::GenesisConfig::<Test> {
        initial_validators: vec![1, 2, 3],
    }
    .assimilate_storage(&mut storage)
    .unwrap();
    storage.into()
}
