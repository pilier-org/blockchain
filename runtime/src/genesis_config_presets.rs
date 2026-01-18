// Copyright (C) 2026 Pilier Team.
// SPDX-License-Identifier: Apache-2.0

use crate::{AccountId, Balance, BalancesConfig, RuntimeGenesisConfig, SudoConfig, UNIT};
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

fn build_genesis_config(
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    root: AccountId,
    endowed_accounts: Vec<(AccountId, Balance)>,
) -> Value {
    let config = RuntimeGenesisConfig {
        balances: BalancesConfig {
            balances: endowed_accounts,
            dev_accounts: None,
        },
        aura: pallet_aura::GenesisConfig {
            authorities: initial_authorities.iter().map(|x| x.0.clone()).collect(),
        },
        grandpa: pallet_grandpa::GenesisConfig {
            authorities: initial_authorities
                .iter()
                .map(|x| (x.1.clone(), 1))
                .collect(),
            ..Default::default()
        },
        sudo: SudoConfig { key: Some(root) },
        ..Default::default()
    };

    serde_json::to_value(config).expect("Could not build genesis config.")
}

// Только testnet preset в runtime
pub fn pilier_testnet_config_genesis() -> Value {
    let sudo_key = account_id_from_ss58("5FEjCCNshkU2ptLe943S5KxGXrtXVbbXVBZJzotBD5TGdnFC");
    let eco_pool = account_id_from_ss58("5DXmUXXz3xpQ7jyuBoGE2w5UzfrRhwF7qexgS6VcUmcTfpw7");
    let treasury_pool = account_id_from_ss58("5Hbm2dCBEbwSUc9KxDtFP55LxZe9Mby1Nsnj3maCrVUGZ3yK");
    let civic_pool = account_id_from_ss58("5CksTfcaZFzLV4Hvz29Lwv51Ug1322v9ZQFXDwpN62FaX4EF");
    let team_pool = account_id_from_ss58("5HC3v3Vde9rREMrjzuawAoLRtkob33HWjaMf9Ki6uMNPpcov");
    let reserve_pool = account_id_from_ss58("5EPWmLyfSzqHH6hZkrkra5tJcHHFKttuiQi9PWxriysQbLaP");
    let faucet = account_id_from_ss58("5CqKvhTuH7Dhic9YrCrkEd9AdtEmBct1FF1HYAz24SpAmf9T");

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

    build_genesis_config(
        vec![
            (node1_aura.into(), node1_grandpa.into()),
            (node2_aura.into(), node2_grandpa.into()),
        ],
        sudo_key,
        vec![
            (faucet, 1_000_000 * UNIT),
            (eco_pool, 1_200_000 * UNIT),
            (treasury_pool, 450_000 * UNIT),
            (civic_pool, 600_000 * UNIT),
            (team_pool, 450_000 * UNIT),
            (reserve_pool, 300_000 * UNIT),
            (node1_aura.into(), 100 * UNIT),
            (node2_aura.into(), 100 * UNIT),
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
