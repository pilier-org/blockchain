// Copyright (C) 2026 Pilier Team.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Substrate and Polkadot dependencies
use frame_support::{
    parameter_types,
    traits::{ConstBool, ConstU8, ConstU32, ConstU64, ConstU128, VariantCountOf},
    weights::{
        IdentityFee, Weight,
        constants::{RocksDbWeight, WEIGHT_REF_TIME_PER_SECOND},
    },
};
use frame_system::limits::{BlockLength, BlockWeights};
use pallet_transaction_payment::{ConstFeeMultiplier, FungibleAdapter, Multiplier};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_runtime::{Perbill, traits::One};
use sp_version::RuntimeVersion;

// Local module imports from lib.rs
use super::{
    AccountId, Aura, Balance, Balances, Block, BlockNumber, EXISTENTIAL_DEPOSIT, Hash, Nonce,
    Runtime, RuntimeCall, RuntimeEvent, RuntimeFreezeReason, RuntimeHoldReason, RuntimeOrigin,
    RuntimeTask, SLOT_DURATION, System, VERSION,
};

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

/// System configuration using the Solochain default prelude.

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

    // Добавь все недостающие:
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type RuntimeEvent = RuntimeEvent;
    type RuntimeTask = RuntimeTask;
    type PalletInfo = crate::PalletInfo; // Изменено!
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
    type MaxSetIdSessionEntries = ConstU64<0>;
    type KeyOwnerProof = sp_core::Void;
    type EquivocationReportSystem = ();
}

/// Timestamp configuration. Minimum period is half of the slot duration.
impl pallet_timestamp::Config for Runtime {
    type Moment = u64;
    type OnTimestampSet = Aura;
    type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
    type WeightInfo = ();
}

/// Balances configuration.
/// ExistentialDeposit is pulled from lib.rs (set to 0.01 PIL).
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

parameter_types! {
  pub FeeMultiplier: Multiplier = Multiplier::one();
}

/// Transaction payment configuration. Uses IdentityFee for simple 1:1 weight-to-fee mapping.
impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = FungibleAdapter<Balances, ()>;
    type OperationalFeeMultiplier = ConstU8<5>;
    type WeightToFee = IdentityFee<Balance>;
    type LengthToFee = IdentityFee<Balance>;
    type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
    type WeightInfo = pallet_transaction_payment::weights::SubstrateWeight<Runtime>;
}

/// Sudo configuration for administrative access during Testnet Phase 1.
impl pallet_sudo::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = pallet_sudo::weights::SubstrateWeight<Runtime>;
}
