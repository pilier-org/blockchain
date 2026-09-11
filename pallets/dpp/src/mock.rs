use crate as pallet_pilier_dpp;
use codec::Encode;
use frame_support::{
    ConsensusEngineId, derive_impl,
    dispatch::{GetDispatchInfo, PostDispatchInfo},
    parameter_types,
    traits::{ConstU8, ConstU32, EitherOfDiverse, FindAuthor, VariantCountOf, fungible::Mutate},
    weights::{
        ConstantMultiplier, Weight, WeightToFeeCoefficient, WeightToFeeCoefficients,
        WeightToFeePolynomial,
    },
};
use frame_system::EnsureRoot;
use pallet_transaction_payment::{
    ChargeTransactionPayment, ConstFeeMultiplier, FungibleAdapter, Multiplier,
};
use sp_runtime::{
    BuildStorage, Perbill,
    traits::{DispatchTransaction, One},
};
use std::cell::RefCell;

type Block = frame_system::mocking::MockBlock<Test>;

/// `frame_system`'s `TestDefaultConfig` already sets `AccountId = u64`; this alias just makes
/// the intent readable at the call sites below (`RuntimeOrigin::signed(1)` and so on).
pub type AccountId = u64;

/// This mock's own currency, matching the runtime's own `Balance` type (`u128`).
pub type Balance = u128;

/// The council's `pallet-collective` instance, mirroring the runtime's own alias exactly so a
/// test exercising [`CouncilCollective`] proves the same origin composition the runtime wires
/// `pallet-validator-set`'s `AddRemoveOrigin` to.
pub type CouncilCollective = pallet_collective::Instance1;

#[frame_support::runtime]
mod runtime {
    use frame_support::instances::Instance1;

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

    #[runtime::pallet_index(3)]
    pub type Balances = pallet_balances::Pallet<Test>;

    #[runtime::pallet_index(4)]
    pub type Authorship = pallet_authorship::Pallet<Test>;

    #[runtime::pallet_index(5)]
    pub type Council = pallet_collective::Pallet<Test, Instance1>;

    #[runtime::pallet_index(6)]
    pub type TransactionPayment = pallet_transaction_payment::Pallet<Test>;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
    type Block = Block;
    type AccountData = pallet_balances::AccountData<Balance>;
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
    pub const ExistentialDeposit: Balance = 1;
}

impl pallet_balances::Config for Test {
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type DoneSlashHandler = ();
}

thread_local! {
    /// The account [`MockFindAuthor`] reports as the current block's author. `None` (the
    /// default, restored by [`clear_block_author`]) simulates the "no author determined" crash
    /// case; [`set_block_author`] simulates an ordinary block with a known author.
    static BLOCK_AUTHOR: RefCell<Option<AccountId>> = const { RefCell::new(None) };
}

/// Test fixture: set the account `Authorship::author()` reports as the current block's author.
pub fn set_block_author(author: AccountId) {
    BLOCK_AUTHOR.with(|a| *a.borrow_mut() = Some(author));
}

/// Test fixture: clear the current block's author, so `Authorship::author()` reports `None` —
/// the crash case this pallet's `Pallet::charge_publication_price` burns for.
pub fn clear_block_author() {
    BLOCK_AUTHOR.with(|a| *a.borrow_mut() = None);
}

/// Reports whatever [`BLOCK_AUTHOR`] currently holds, standing in for the runtime's own
/// `pallet_session::FindAccountFromAuthorIndex<Self, Aura>` — this mock has neither a session
/// nor an Aura pallet, and full control from each test over who the author is (or that there is
/// none) is exactly what the burn-on-no-author tests need.
pub struct MockFindAuthor;
impl FindAuthor<AccountId> for MockFindAuthor {
    fn find_author<'a, I>(_digests: I) -> Option<AccountId>
    where
        I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
    {
        BLOCK_AUTHOR.with(|a| *a.borrow())
    }
}

impl pallet_authorship::Config for Test {
    type FindAuthor = MockFindAuthor;
    type EventHandler = ();
}

parameter_types! {
    pub const CouncilMotionDuration: u64 = 3;
    pub const CouncilMaxProposals: u32 = 10;
    pub const CouncilMaxMembers: u32 = 10;
    pub CouncilMaxProposalWeight: Weight = Weight::from_parts(1_000_000_000_000, u64::MAX);
}

impl pallet_collective::Config<CouncilCollective> for Test {
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = CouncilMotionDuration;
    type MaxProposals = CouncilMaxProposals;
    type MaxMembers = CouncilMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = ();
    type SetMembersOrigin = EnsureRoot<AccountId>;
    type MaxProposalWeight = CouncilMaxProposalWeight;
    type DisapproveOrigin = EnsureRoot<AccountId>;
    type KillOrigin = EnsureRoot<AccountId>;
    type Consideration = ();
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
    type Currency = Balances;
    // Mirrors `pallet_validator_set::Config::AddRemoveOrigin` in `runtime/src/configs/mod.rs`
    // exactly — root as an emergency lever, or a council supermajority of at least 75% — so the
    // acceptor's own comparison between the two origin types has something identical to compare.
    type AdminOrigin = EitherOfDiverse<
        EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>,
    >;
    type MaxCompanyRegistrationNumberLen = MaxCompanyRegistrationNumberLen;
    type MaxGs1IdLen = MaxGs1IdLen;
    type MaxRecordBodyLen = MaxRecordBodyLen;
    type MaxEventLen = MaxEventLen;
    type MaxFilePathLen = MaxFilePathLen;
    type MaxFileContentTypeLen = MaxFileContentTypeLen;
    type WeightInfo = ();
}

parameter_types! {
    /// Ten smallest units per byte — the same value `runtime/src/configs/mod.rs` computes as
    /// `10 * MICRO_UNIT` (`MICRO_UNIT` is one smallest unit there). Named here as a direct
    /// literal rather than imported from the runtime crate, which this pallet cannot depend on
    /// without a cycle; [`mock_transaction_byte_fee_matches_runtime_literal`] in `tests.rs`
    /// exists precisely so the two numbers drifting apart fails a test instead of staying
    /// invisible.
    pub const TransactionByteFee: Balance = 10;
    /// Fixed at one, exactly as the runtime's own `ConstFeeMultiplier<FeeMultiplier>` is: this
    /// mock never exercises congestion-driven fee adjustment, so every fee this pipeline charges
    /// is deterministic from `TransactionByteFee`, [`WeightToFee`] and a call's declared weight
    /// alone.
    pub FeeMultiplier: Multiplier = Multiplier::one();
}

/// Mirrors `runtime/src/configs/mod.rs`'s own `WeightToFee` polynomial exactly: one smallest
/// unit of fee per unit of `ref_time` weight, expressed as the fraction `1 / 1_000_000`.
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
    type Balance = Balance;

    fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
        let p = 1u128;
        let q = Balance::from(1_000_000u32);

        vec![WeightToFeeCoefficient {
            degree: 1,
            coeff_frac: Perbill::from_rational(p, q),
            coeff_integer: 0u128,
            negative: false,
        }]
        .into()
    }
}

impl pallet_transaction_payment::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    // The standard fee's destination does not matter to this mock — phase C measures what the
    // *publisher* pays in total, not who ends up with the standard fee — so it is simply burned,
    // unlike the runtime's own `ToAuthor`.
    type OnChargeTransaction = FungibleAdapter<Balances, ()>;
    type OperationalFeeMultiplier = ConstU8<5>;
    type WeightToFee = WeightToFee;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
    type WeightInfo = ();
}

/// Runs `call`, signed by `who`, through the real fee pipeline exactly as a submitted extrinsic
/// would: `pallet-transaction-payment`'s `ChargeTransactionPayment` extension withdraws the
/// standard fee up front (base weight fee, length fee, weight fee, plus `tip`), the call
/// actually dispatches, and the extension's own post-dispatch step refunds whatever the call's
/// `PostDispatchInfo` says to refund (the whole standard fee but not `tip`, when the call returns
/// `Pays::No`). Returns the raw dispatch outcome together with the standard fee the extension
/// computed and charged up front, computed by `pallet-transaction-payment` itself
/// (`Pallet::compute_fee`) rather than recomputed by the caller, so a test comparing against it
/// can never drift from what the extension actually charges.
pub fn dispatch_through_fee_pipeline(
    who: AccountId,
    tip: Balance,
    call: RuntimeCall,
) -> (
    sp_runtime::ApplyExtrinsicResultWithInfo<PostDispatchInfo>,
    Balance,
) {
    let info = call.get_dispatch_info();
    let len = call.encode().len() as u32;
    let standard_fee = TransactionPayment::compute_fee(len, &info, tip);
    let outcome = ChargeTransactionPayment::<Test>::from(tip).dispatch_transaction(
        RuntimeOrigin::signed(who),
        call,
        &info,
        len as usize,
        0,
    );
    (outcome, standard_fee)
}

/// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let storage = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();
    let mut ext: sp_io::TestExternalities = storage.into();
    // Every test starts with no author determined, exactly like a freshly built externalities —
    // stated here explicitly so a test earlier in the same process that called
    // `set_block_author` can never leak into a later one (the thread-local would otherwise
    // survive across `new_test_ext()` calls within the same test thread).
    ext.execute_with(clear_block_author);
    ext
}

/// Test fixture: mint `amount` of the mock currency directly into `who`'s account, for tests
/// that need a funded payer before exercising [`Pallet::charge_publication_price`] — `who`
/// starts with no balance otherwise, since [`new_test_ext`] seeds no `pallet_balances` genesis.
pub fn fund_account(who: AccountId, amount: Balance) {
    Balances::mint_into(&who, amount).expect("minting into a test account must not fail");
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
    assert!(
        Registry::grant_registration_number(
            RuntimeOrigin::root(),
            project_id,
            company_registration_number.to_vec(),
        )
        .is_ok()
    );
}

/// Test fixture: register a schema and return its identifier.
pub fn register_schema() -> u32 {
    let schema_id = pallet_pilier_registry::NextSchemaId::<Test>::get();
    assert!(
        Registry::register_schema(RuntimeOrigin::root(), 1, 1, b"test schema".to_vec()).is_ok()
    );
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
    assert!(
        Registry::create_storage_endpoint(
            RuntimeOrigin::signed(who),
            company_registration_number.to_vec(),
            b"https://storage.pilier.net/dpp/evidence/".to_vec(),
        )
        .is_ok()
    );
    endpoint_id
}
