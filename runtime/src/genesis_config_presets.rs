// Copyright (C) 2026 Pilier Team.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    AccountId, Balance, BalancesConfig, CouncilConfig, RuntimeGenesisConfig, SessionKeys,
    SudoConfig, UNIT,
};
use alloc::{vec, vec::Vec};
use serde_json::Value;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_core::crypto::Ss58Codec;
use sp_core::sr25519;
use sp_genesis_builder::{self, PresetId};

pub const PILIER_TESTNET_PRESET: &str = "pilier_testnet";

fn account_id_from_ss58(s: &str) -> AccountId {
    AccountId::from_ss58check(s).expect("Invalid SS58 address")
}

/// Builds the genesis config from each validator's sovereign account plus its Aura and GRANDPA
/// keys, the Sudo root key, and the balances endowment.
///
/// `aura` and `grandpa` genesis authorities are deliberately left empty here: since Phase 2,
/// `pallet-session` owns the live authority set and populates both Aura and GRANDPA from the
/// `session` genesis keys below, at the first session boundary. `validator_set` seeds our own
/// `pallet-validator-set`, which is what `pallet-session` asks for the validator list going
/// forward (see `pallet_session::SessionManager` impl in `pallets/validator-set`). `council`
/// (Phase 4a) seeds the same three validator accounts as the council's initial membership;
/// `pallet_validator_set::Config::MembershipChanged` keeps the two lists in sync after genesis.
fn build_genesis_config(
    validators: Vec<(AccountId, AuraId, GrandpaId)>,
    root: AccountId,
    endowed_accounts: Vec<(AccountId, Balance)>,
) -> Value {
    let session_keys: Vec<(AccountId, AccountId, SessionKeys)> = validators
        .iter()
        .cloned()
        .map(|(account, aura, grandpa)| (account.clone(), account, SessionKeys { aura, grandpa }))
        .collect();

    let initial_validators: Vec<AccountId> = validators
        .iter()
        .map(|(account, _, _)| account.clone())
        .collect();

    let config = RuntimeGenesisConfig {
        balances: BalancesConfig {
            balances: endowed_accounts,
            dev_accounts: None,
        },
        aura: pallet_aura::GenesisConfig {
            authorities: vec![],
        },
        grandpa: pallet_grandpa::GenesisConfig {
            authorities: vec![],
            ..Default::default()
        },
        session: pallet_session::GenesisConfig {
            keys: session_keys,
            ..Default::default()
        },
        // The council's initial membership is the same three validator accounts (see
        // `MembershipChanged` in `pallet_validator_set::Config`, which keeps the two in sync from
        // here on). `phantom` is the instance-marker field the multi-instance genesis struct
        // carries; it always serializes away and is filled with `Default::default()`.
        council: CouncilConfig {
            members: initial_validators.clone(),
            phantom: Default::default(),
        },
        validator_set: pallet_validator_set::GenesisConfig { initial_validators },
        sudo: SudoConfig { key: Some(root) },
        ..Default::default()
    };

    serde_json::to_value(config).expect("Could not build genesis config.")
}

// Only testnet preset в runtime
//
// Treasury layout below follows the public tokenomics table
// (`governance/tokenomics.md`, ai/plans/runtime-mutable-validator-set.md section 3), sums to
// exactly 4_000_000 PIL. Old variable names (pre-Phase-3) are noted in comments since several
// addresses are reused for different pools than before.
pub fn pilier_testnet_config_genesis() -> Value {
    let sudo_key = account_id_from_ss58("5FEjCCNshkU2ptLe943S5KxGXrtXVbbXVBZJzotBD5TGdnFC");

    // 30% / 1,200,000 PIL — was `eco_pool`.
    let commercial_treasury =
        account_id_from_ss58("5DXmUXXz3xpQ7jyuBoGE2w5UzfrRhwF7qexgS6VcUmcTfpw7");
    // 25% / 1,000,000 PIL, also the manual faucet — was `faucet`.
    let civic_treasury = account_id_from_ss58("5CqKvhTuH7Dhic9YrCrkEd9AdtEmBct1FF1HYAz24SpAmf9T");
    // 15% / 600,000 PIL minus the 1,000 PIL Validité carve-out below = 599,000 PIL — was `reserve_pool`.
    let flagship_product_reserve =
        account_id_from_ss58("5EPWmLyfSzqHH6hZkrkra5tJcHHFKttuiQi9PWxriysQbLaP");
    // 15% / 600,000 PIL — was `team_pool`.
    let team_and_advisory =
        account_id_from_ss58("5HC3v3Vde9rREMrjzuawAoLRtkob33HWjaMf9Ki6uMNPpcov");
    // 10% / 400,000 PIL minus the 300 PIL node carve-out below = 399,700 PIL — was `civic_pool`.
    let validator_bootstrap_pool =
        account_id_from_ss58("5CksTfcaZFzLV4Hvz29Lwv51Ug1322v9ZQFXDwpN62FaX4EF");
    // 5% / 200,000 PIL — new pool, closes the 95%/100% gap in the documented tokenomics; fresh
    // sr25519 keypair generated 2026-07-19, secret handed to Alex outside the repository.
    let foundation_reserve =
        account_id_from_ss58("5GYrFVhEz58LQWHoYRuWtBJ8iEtT1GaFnKvDG8JthqNc3Bt2");
    // 1,000 PIL test account for Validité, carved out of Flagship Product Reserve — was `treasury_pool`.
    let validite_test_account =
        account_id_from_ss58("5Hbm2dCBEbwSUc9KxDtFP55LxZe9Mby1Nsnj3maCrVUGZ3yK");

    let node1_aura =
        sr25519::Public::from_ss58check("5EcdgAQ99gftvNbEdfu7zuRZonev14s9YMmact12UEuQ9ndV")
            .unwrap();
    let node1_grandpa = sp_core::ed25519::Public::from_ss58check(
        "5DqEQReLbazbLWsoB9QTqmLWmZdM5KoS8x8tMKqzfBP8TdJ3",
    )
    .unwrap();

    let node2_aura =
        sr25519::Public::from_ss58check("5H3Efoj3JcwJu7oZdtj2PvRkEDS4UJ2utS1JRGXT6hDn7Ph5")
            .unwrap();
    let node2_grandpa = sp_core::ed25519::Public::from_ss58check(
        "5EbdWSs3qTQrnjhNQv3vNJ2ggMLKMFB3k5KY9oDPz5FLZzQc",
    )
    .unwrap();

    let node3_aura =
        sr25519::Public::from_ss58check("5H6bqYD2XFXdHyMKC8EMb4pLijC1rntftZUmchUiESdwPSHR")
            .unwrap();
    let node3_grandpa = sp_core::ed25519::Public::from_ss58check(
        "5DHhVJFNSv9RxCQccqCpP3orPccZ7jXU1aQnuwhRNsaHsExd",
    )
    .unwrap();

    build_genesis_config(
        vec![
            (node1_aura.into(), node1_aura.into(), node1_grandpa.into()),
            (node2_aura.into(), node2_aura.into(), node2_grandpa.into()),
            (node3_aura.into(), node3_aura.into(), node3_grandpa.into()),
        ],
        sudo_key,
        vec![
            // Documented pools — sum 1_200_000 + 1_000_000 + 599_000 + 600_000 + 399_700 +
            // 200_000 + 1_000 + 300 = 4_000_000 PIL exactly.
            (commercial_treasury, 1_200_000 * UNIT),
            (civic_treasury, 1_000_000 * UNIT),
            (flagship_product_reserve, 599_000 * UNIT),
            (team_and_advisory, 600_000 * UNIT),
            (validator_bootstrap_pool, 399_700 * UNIT),
            (foundation_reserve, 200_000 * UNIT),
            (validite_test_account, 1_000 * UNIT),
            // Practical carve-out from Validator Bootstrap Pool: 100 PIL per node account so
            // each has a small operating balance for technical transactions (chiefly
            // `session.set_keys`) — see plan section 3, "Практическая строка на валидаторов".
            (node1_aura.into(), 100 * UNIT),
            (node2_aura.into(), 100 * UNIT),
            (node3_aura.into(), 100 * UNIT),
        ],
    )
}

pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
    let patch = match id.as_ref() {
        PILIER_TESTNET_PRESET => pilier_testnet_config_genesis(),
        _ => return None,
    };
    Some(
        serde_json::to_string(&patch)
            .expect("serialization to json is expected to work. qed.")
            .into_bytes(),
    )
}

pub fn preset_names() -> Vec<PresetId> {
    vec![PresetId::from(PILIER_TESTNET_PRESET)]
}
