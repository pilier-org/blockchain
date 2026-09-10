use crate as pallet_pilier_dpp;
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

    #[runtime::pallet_index(2)]
    pub type Dpp = pallet_pilier_dpp::Pallet<Test>;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
}

parameter_types! {
    /// `pallet-pilier-registry`'s own configuration, composed into this mock so passport
    /// pallet tests exercise the real `RegistryAccess` implementation rather than a stand-in.
    pub const MaxRegistryCompanyRegistrationNumberLen: u32 = 32;
    pub const MaxGs1IdLenForRegistry: u32 = 128;
    pub const MaxRegistryTypeNameLen: u32 = 64;
    pub const MaxRegistryEntryValueLen: u32 = 256;
    pub const MaxStorageEndpointAddressLen: u32 = 256;
    pub const MaxSchemaDescriptionLen: u32 = 16 * 1024;
}

impl pallet_pilier_registry::Config for Test {
    type AdminOrigin = EnsureRoot<AccountId>;
    type MaxCompanyRegistrationNumberLen = MaxRegistryCompanyRegistrationNumberLen;
    type MaxGs1IdLen = MaxGs1IdLenForRegistry;
    type MaxRegistryTypeNameLen = MaxRegistryTypeNameLen;
    type MaxRegistryEntryValueLen = MaxRegistryEntryValueLen;
    type MaxStorageEndpointAddressLen = MaxStorageEndpointAddressLen;
    type MaxSchemaDescriptionLen = MaxSchemaDescriptionLen;
    type WeightInfo = ();
}

parameter_types! {
    /// This pallet's own bounds. `MaxRecordBodyLen` and `MaxEventLen` mirror the pallet's own
    /// ceilings exactly (four kibibytes and one hundred twenty-eight bytes), so a test
    /// that exercises the configured bound is exercising the real limit, not a stand-in for it.
    pub const MaxCompanyRegistrationNumberLen: u32 = 32;
    pub const MaxGs1IdLen: u32 = 128;
    pub const MaxRecordBodyLen: u32 = 4 * 1024;
    pub const MaxEventLen: u32 = 128;
    pub const MaxFilePathLen: u32 = 256;
    pub const MaxFileContentTypeLen: u32 = 64;
}

impl pallet_pilier_dpp::Config for Test {
    type Registry = Registry;
    type MaxCompanyRegistrationNumberLen = MaxCompanyRegistrationNumberLen;
    type MaxGs1IdLen = MaxGs1IdLen;
    type MaxRecordBodyLen = MaxRecordBodyLen;
    type MaxEventLen = MaxEventLen;
    type MaxFilePathLen = MaxFilePathLen;
    type MaxFileContentTypeLen = MaxFileContentTypeLen;
    type WeightInfo = ();
}

/// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let storage = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    storage.into()
}

/// Test fixture: advance the mock chain to block `number`. Every test in this file that does
/// not call this starts and stays at block zero, since `new_test_ext` only builds genesis
/// storage and never advances the block number itself — a proof that a stored value tracks the
/// block it was written at needs this call, because comparing a stored zero against a block
/// number that was never moved off zero proves nothing.
pub fn set_block_number(number: frame_system::pallet_prelude::BlockNumberFor<Test>) {
    System::set_block_number(number);
}

/// Test fixture: create a project owned by `owner`, grant it `company_registration_number`,
/// register a schema under `schema_id`'s expected identifier order, and create a storage
/// endpoint under the same company. Returns nothing — callers assert against the fixed
/// identifiers this produces, starting at zero for each of `pallet-pilier-registry`'s own
/// counters, since each test runs against a fresh instance of the mock runtime's storage.
pub fn setup_project_with_permission(owner: AccountId, company_registration_number: &[u8]) {
    assert!(Registry::create_project(RuntimeOrigin::root(), owner).is_ok());
    let project_id = pallet_pilier_registry::NextProjectId::<Test>::get() - 1;
    assert!(Registry::grant_registration_number(
        RuntimeOrigin::root(),
        project_id,
        company_registration_number.to_vec(),
    )
    .is_ok());
}

/// Test fixture: register a schema and return its identifier.
pub fn register_schema() -> u32 {
    let schema_id = pallet_pilier_registry::NextSchemaId::<Test>::get();
    assert!(Registry::register_schema(RuntimeOrigin::root(), 1, 1, b"test schema".to_vec()).is_ok());
    schema_id
}

/// Test fixture: read back a head record by its unbounded key, bounding it the same way the
/// pallet's own dispatchables do. Returns `None` if no record exists at that key.
pub fn get_head(
    company_registration_number: &[u8],
    gs1_id: &[u8],
) -> Option<crate::HeadRecord<Test>> {
    let company_registration_number: crate::CompanyRegistrationNumber<Test> =
        company_registration_number.to_vec().try_into().unwrap();
    let gs1_id: crate::Gs1Id<Test> = gs1_id.to_vec().try_into().unwrap();
    crate::Heads::<Test>::get(&company_registration_number, &gs1_id)
}

/// Test fixture: create a storage endpoint under `company_registration_number`, called by
/// `who`, and return its identifier. `who` must belong to the project holding the write
/// permission for `company_registration_number`.
pub fn create_storage_endpoint(who: AccountId, company_registration_number: &[u8]) -> u32 {
    let endpoint_id = pallet_pilier_registry::NextStorageEndpointId::<Test>::get();
    assert!(Registry::create_storage_endpoint(
        RuntimeOrigin::signed(who),
        company_registration_number.to_vec(),
        b"https://storage.pilier.net/dpp/evidence/".to_vec(),
    )
    .is_ok());
    endpoint_id
}
