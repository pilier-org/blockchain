// Copyright (C) 2026 Pilier Team.
// SPDX-License-Identifier: Apache-2.0

extern crate alloc;
use alloc::vec;

// Substrate and Polkadot dependencies
use frame_support::{
    parameter_types,
    traits::{ConstBool, ConstU8, ConstU32, ConstU64, ConstU128, VariantCountOf},
    weights::{
        ConstantMultiplier, Weight, WeightToFeeCoefficients, WeightToFeePolynomial,
        constants::{RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
    },
};
use frame_system::limits::{BlockLength, BlockWeights};
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter, Multiplier};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_runtime::{Perbill, traits::One};
use sp_version::RuntimeVersion;

use crate::MICRO_UNIT;

// Local module imports from lib.rs
use super::{
    AccountId, Aura, Balance, Balances, Block, BlockNumber, Council, EXISTENTIAL_DEPOSIT, Hash,
    Nonce, Runtime, RuntimeCall, RuntimeEvent, RuntimeFreezeReason, RuntimeHoldReason,
    RuntimeOrigin, RuntimeTask, SLOT_DURATION, SessionKeys, System, VERSION, ValidatorSet,
};

/// The council's `pallet-collective` instance. A type alias only — `pallet_collective::Instance1`
/// is used directly, there being exactly one collective in this runtime for now.
type CouncilCollective = pallet_collective::Instance1;

/// We allow for 75% of the block to be occupied by Normal transactions.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

parameter_types! {
  pub const BlockHashCount: BlockNumber = 2400;
  pub const Version: RuntimeVersion = VERSION;

  /// We allow for 2 seconds of compute with a 6 second average block time.
  pub RuntimeBlockWeights: BlockWeights = BlockWeights::with_sensible_defaults(
    Weight::from_parts(2u64 * WEIGHT_REF_TIME_PER_SECOND, u64::MAX),
    NORMAL_DISPATCH_RATIO,
  );
  /// Max block length is 5MB.
  pub RuntimeBlockLength: BlockLength = BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
  /// SS58 Prefix 42 is the generic Substrate prefix.
  pub const SS58Prefix: u8 = 42;
}

/// System configuration
impl frame_system::Config for Runtime {
    type Block = Block;
    type BlockWeights = RuntimeBlockWeights;
    type BlockLength = RuntimeBlockLength;
    type AccountId = AccountId;
    type Nonce = Nonce;
    type Hash = Hash;
    type BlockHashCount = BlockHashCount;
    type DbWeight = RocksDbWeight;
    type Version = Version;
    type AccountData = pallet_balances::AccountData<Balance>;
    type SS58Prefix = SS58Prefix;
    type MaxConsumers = frame_support::traits::ConstU32<16>;

    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeTask = RuntimeTask;
    type PalletInfo = crate::PalletInfo;
    type OnSetCode = ();
    type Lookup = sp_runtime::traits::AccountIdLookup<AccountId, ()>;
    type SystemWeightInfo = ();
    type BaseCallFilter = frame_support::traits::Everything;
    type Hashing = sp_runtime::traits::BlakeTwo256;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type ExtensionsWeightInfo = ();
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
}

/// Aura consensus configuration for block production.
impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = ConstU32<32>;
    type AllowMultipleBlocksPerSlot = ConstBool<false>;
    type SlotDuration = pallet_aura::MinimumPeriodTimesTwo<Runtime>;
}

/// Grandpa consensus configuration for block finalization.
impl pallet_grandpa::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
    type MaxAuthorities = ConstU32<32>;
    type MaxNominators = ConstU32<0>;
    // Now that `pallet-session` rotates sessions, GRANDPA needs to remember more than zero past
    // set IDs so that equivocation/finality-proof lookups spanning a session boundary still find
    // the authority set they refer to. 168 is a generous, testnet-appropriate cushion (a week's
    // worth of sessions at `Period = 100` blocks / ~10 minutes per session).
    type MaxSetIdSessionEntries = ConstU64<168>;
    type KeyOwnerProof = sp_core::Void;
    type EquivocationReportSystem = ();
}

parameter_types! {
    /// How many blocks a session lasts. ~10 minutes at the chain's 6-second block time — short
    /// enough to demonstrate a validator-set change quickly on a testnet, long enough not to spam
    /// logs with constant session rotation. See the plan's cost note for the alternatives
    /// considered (10 blocks vs. 600 blocks).
    pub const Period: BlockNumber = 100;
    pub const Offset: BlockNumber = 0;
}

/// Identity conversion from a validator's sovereign `AccountId` to its `ValidatorId`. Pilier does
/// not distinguish a "stash" account from a "controller" account the way `pallet-staking` does,
/// so the two ID types are the same and this conversion always succeeds.
pub struct ValidatorIdOf;
impl sp_runtime::traits::Convert<AccountId, Option<AccountId>> for ValidatorIdOf {
    fn convert(a: AccountId) -> Option<AccountId> {
        Some(a)
    }
}

/// Session configuration: stores each validator's session keys and, on every session boundary,
/// asks `ValidatorSet` (our own pallet) who the next session's validators should be, then hands
/// that list to Aura and GRANDPA via `SessionHandler`.
impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = AccountId;
    type ValidatorIdOf = ValidatorIdOf;
    type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
    type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
    type SessionManager = ValidatorSet;
    type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    // `()` disables validator disabling on equivocation/misbehaviour reports for now; this is a
    // testnet-phase PoA chain with no staking/slashing wired up yet.
    type DisablingStrategy = ();
    type WeightInfo = ();
    // Session keys can be held with a deposit to discourage spam; on this fixed, admin/council
    // controlled validator set there is no open registration to spam, so the deposit is zero.
    type Currency = Balances;
    type KeyDeposit = ();
}

parameter_types! {
    /// How long a council motion stays open for voting before it lapses unresolved. 50 blocks is
    /// ~5 minutes at the chain's 6-second block time — short enough to demonstrate a vote to
    /// completion quickly on a testnet.
    pub const CouncilMotionDuration: BlockNumber = 50;
    /// How many motions can be in flight at once. 64 is generous headroom for a small validator
    /// council that is not expected to propose more than a handful of changes at a time.
    pub const CouncilMaxProposals: u32 = 64;
    /// Upper bound on council size used for weight estimation, not an enforced cap on today's
    /// three-validator council. 100 leaves plenty of room to grow the validator set.
    pub const CouncilMaxMembers: u32 = 100;
    /// The heaviest call a council motion may propose: up to half of a block's total weight,
    /// mirroring the polkadot-sdk `node-template` reference (`MaxCollectivesProposalWeight`).
    pub CouncilMaxProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
}

/// The validators' council: a `pallet-collective` instance whose members are the sovereign
/// accounts of the current validator set (kept in sync via `MembershipChanged` below). It votes
/// on `pallet_validator_set` calls to add or remove a validator; a proposal must clear the 75%
/// threshold wired into `AddRemoveOrigin` there to take effect. `DefaultVote =
/// PrimeDefaultVote` means an abstaining member's vote defaults to the prime member's vote if a
/// prime is set, and to "no" otherwise — the standard FRAME choice, and there is no prime-setting
/// UI in this plan, so it behaves like a plain "no" default for now. `Consideration = ()` charges
/// no deposit for submitting a proposal: this council has no open membership to spam, so the
/// anti-spam deposit machinery is unnecessary.
impl pallet_collective::Config<CouncilCollective> for Runtime {
    type RuntimeOrigin = RuntimeOrigin;
    type Proposal = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type MotionDuration = CouncilMotionDuration;
    type MaxProposals = CouncilMaxProposals;
    type MaxMembers = CouncilMaxMembers;
    type DefaultVote = pallet_collective::PrimeDefaultVote;
    type WeightInfo = ();
    // Membership is driven by the validator set (see `MembershipChanged` below), not set directly;
    // root retains an emergency override to fix the council's membership by hand if it ever
    // desyncs from `ValidatorSet`.
    type SetMembersOrigin = frame_system::EnsureRoot<AccountId>;
    type MaxProposalWeight = CouncilMaxProposalWeight;
    type DisapproveOrigin = frame_system::EnsureRoot<AccountId>;
    type KillOrigin = frame_system::EnsureRoot<AccountId>;
    type Consideration = ();
}

/// Validator-set configuration. `AddRemoveOrigin` allows either root (Sudo, an emergency lever)
/// or a council supermajority of at least 75% (`EnsureProportionAtLeast<.., 3, 4>`) to add or
/// remove a validator — this is the "council supermajority, or root as an emergency lever" design
/// from Phase 4 of the mutable-validator-set plan, and the 3/4 threshold matches the 75% approval
/// documented publicly. `MembershipChanged = Council` keeps the council's member list in lock-step
/// with the validator set: whenever `pallet_validator_set` adds or removes a validator, it calls
/// `Council::change_members_sorted(..)` so the same accounts that hold validator seats also hold
/// council votes.
impl pallet_validator_set::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AddRemoveOrigin = frame_support::traits::EitherOfDiverse<
        frame_system::EnsureRoot<AccountId>,
        pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 4>,
    >;
    type MembershipChanged = Council;
    type MinValidators = ConstU32<1>;
    type WeightInfo = ();
}

/// Timestamp configuration. Minimum period is half of the slot duration.
impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
    type WeightInfo = ();
}

/// Balances configuration.
impl pallet_balances::Config for Runtime {
    type MaxLocks = ConstU32<50>;
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU128<EXISTENTIAL_DEPOSIT>;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type FreezeIdentifier = RuntimeFreezeReason;
    type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type DoneSlashHandler = ();
}

// ✅ FEES CONFIGURATION
parameter_types! {
    pub const TransactionByteFee: Balance = 10 * MICRO_UNIT;
    pub FeeMultiplier: Multiplier = Multiplier::one();
}

pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
    type Balance = Balance;

    fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
        use frame_support::weights::WeightToFeeCoefficient;

        let p = MICRO_UNIT;
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

/// Transaction payment configuration.
impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = FungibleAdapter<Balances, ()>;
    type OperationalFeeMultiplier = ConstU8<5>;
    type WeightToFee = WeightToFee;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
    type WeightInfo = pallet_transaction_payment::weights::SubstrateWeight<Runtime>;
}

/// Sudo configuration for administrative access during Testnet Phase 1.
impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}
