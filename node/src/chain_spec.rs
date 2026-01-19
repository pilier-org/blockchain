use pilier_runtime::{AccountId, AccountPublic, AuraId, Balance, GrandpaId, UNIT, WASM_BINARY};
use sc_service::ChainType;
use sp_core::{Pair, Public, sr25519};
use sp_runtime::traits::IdentifyAccount;

pub type ChainSpec = sc_service::GenericChainSpec;

/// Generate a crypto pair from seed.
fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate an account ID from seed
fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

fn authority_keys_from_seed(s: &str) -> (AuraId, GrandpaId) {
    (get_from_seed::<AuraId>(s), get_from_seed::<GrandpaId>(s))
}

fn testnet_genesis(
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    root_key: AccountId,
    endowed_accounts: Vec<(AccountId, Balance)>,
) -> serde_json::Value {
    serde_json::json!({
        "balances": {
            "balances": endowed_accounts,
        },
        "aura": {
            "authorities": initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
        },
        "grandpa": {
            "authorities": initial_authorities.iter().map(|x| (x.1.clone(), 1)).collect::<Vec<_>>(),
        },
        "sudo": { "key": Some(root_key) },
    })
}

pub fn development_config() -> Result<ChainSpec, String> {
    Ok(
        ChainSpec::builder(WASM_BINARY.ok_or("Development wasm not available")?, None)
            .with_name("Development")
            .with_id("dev")
            .with_chain_type(ChainType::Development)
            .with_genesis_config_patch(testnet_genesis(
                vec![authority_keys_from_seed("Alice")],
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                vec![(
                    get_account_id_from_seed::<sr25519::Public>("Alice"),
                    3_000_000 * UNIT,
                )],
            ))
            .build(),
    )
}

pub fn local_testnet_config() -> Result<ChainSpec, String> {
    Ok(
        ChainSpec::builder(WASM_BINARY.ok_or("Local testnet wasm not available")?, None)
            .with_name("Local Testnet")
            .with_id("local_testnet")
            .with_chain_type(ChainType::Local)
            .with_genesis_config_patch(testnet_genesis(
                vec![
                    authority_keys_from_seed("Alice"),
                    authority_keys_from_seed("Bob"),
                ],
                get_account_id_from_seed::<sr25519::Public>("Alice"),
                vec![
                    (
                        get_account_id_from_seed::<sr25519::Public>("Alice"),
                        1_500_000 * UNIT,
                    ),
                    (
                        get_account_id_from_seed::<sr25519::Public>("Bob"),
                        1_500_000 * UNIT,
                    ),
                ],
            ))
            .build(),
    )
}

pub fn pilier_testnet_config() -> Result<ChainSpec, String> {
    Ok(
        ChainSpec::builder(WASM_BINARY.ok_or("Testnet wasm not available")?, None)
            .with_name("Pilier Testnet")
            .with_id("pilier_testnet")
            .with_chain_type(ChainType::Live)
            .with_genesis_config_preset_name("pilier_testnet")
            .build(),
    )
}
