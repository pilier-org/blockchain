use crate as pallet_pilier_registry;
use frame_support::{derive_impl, parameter_types};
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
    pub type Registry = pallet_pilier_registry::Pallet<Test>;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
}

parameter_types! {
    /// The seven bounds below mirror the pallet's own bounds exactly, so a test that
    /// exercises the configured bound is exercising the real limit, not a stand-in for it.
    pub const MaxCompanyRegistrationNumberLen: u32 = 32;
    pub const MaxGs1IdLen: u32 = 128;
    pub const MaxRegistryTypeNameLen: u32 = 64;
    pub const MaxRegistryEntryValueLen: u32 = 256;
    pub const MaxStorageEndpointAddressLen: u32 = 256;
    pub const MaxSchemaDescriptionLen: u32 = 16 * 1024;
    pub const MaxProjectWriters: u32 = 32;
}

impl pallet_pilier_registry::Config for Test {
    // Stand-in for "council supermajority, or root" until the council pallet is wired into a
    // runtime that carries this pallet.
    // Root is a realistic and simple choice for unit tests: it lets us assert both that an
    // authorised call (root) succeeds and an unauthorised one (any signed account) is rejected.
    type AdminOrigin = EnsureRoot<AccountId>;
    type MaxCompanyRegistrationNumberLen = MaxCompanyRegistrationNumberLen;
    type MaxGs1IdLen = MaxGs1IdLen;
    type MaxRegistryTypeNameLen = MaxRegistryTypeNameLen;
    type MaxRegistryEntryValueLen = MaxRegistryEntryValueLen;
    type MaxStorageEndpointAddressLen = MaxStorageEndpointAddressLen;
    type MaxSchemaDescriptionLen = MaxSchemaDescriptionLen;
    type MaxProjectWriters = MaxProjectWriters;
    type WeightInfo = ();
}

/// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let storage = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    storage.into()
}
