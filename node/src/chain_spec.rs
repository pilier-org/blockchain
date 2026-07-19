use pilier_runtime::{
    AccountId, AccountPublic, AuraId, Balance, GrandpaId, SessionKeys, UNIT, WASM_BINARY,
};
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

/// A validator's sovereign account plus its Aura and GRANDPA session keys, all derived from the
/// same well-known seed (e.g. "Alice", "Bob"). Used to seed `session` and `validator_set` in the
/// `dev`/`local` genesis presets, mirroring how the `pilier_testnet` preset in
/// `runtime/src/genesis_config_presets.rs` derives a validator's account from its Aura key.
fn validator_keys_from_seed(s: &str) -> (AccountId, AuraId, GrandpaId) {
    (
        get_account_id_from_seed::<sr25519::Public>(s),
        get_from_seed::<AuraId>(s),
        get_from_seed::<GrandpaId>(s),
    )
}

/// Builds the genesis config patch shared by `dev` and `local`.
///
/// `aura` and `grandpa` genesis authorities are deliberately left empty: since Phase 2,
/// `pallet-session` owns the live authority set and populates both Aura and GRANDPA from the
/// `session` genesis keys below, at the first session boundary. `validatorSet` seeds our own
/// `pallet-validator-set`. The full documented treasury layout (see
/// `runtime/src/genesis_config_presets.rs`) is not repeated here — the existing dev/local test
/// endowment is kept as-is.
fn testnet_genesis(
    initial_authorities: Vec<(AccountId, AuraId, GrandpaId)>,
    root_key: AccountId,
    endowed_accounts: Vec<(AccountId, Balance)>,
) -> serde_json::Value {
    let session_keys = initial_authorities
        .iter()
        .map(|(account, aura, grandpa)| {
            (
                account.clone(),
                account.clone(),
                SessionKeys {
                    aura: aura.clone(),
                    grandpa: grandpa.clone(),
                },
            )
        })
        .collect::<Vec<_>>();
    let initial_validators = initial_authorities
        .iter()
        .map(|(account, _, _)| account.clone())
        .collect::<Vec<_>>();

    serde_json::json!({
        "balances": {
            "balances": endowed_accounts,
        },
        "aura": {
            "authorities": Vec::<AuraId>::new(),
        },
        "grandpa": {
            "authorities": Vec::<(GrandpaId, u64)>::new(),
        },
        "session": {
            "keys": session_keys,
        },
        "validatorSet": {
            "initialValidators": initial_validators,
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
                vec![validator_keys_from_seed("Alice")],
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
                    validator_keys_from_seed("Alice"),
                    validator_keys_from_seed("Bob"),
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
    use serde_json::json;

    Ok(
        ChainSpec::builder(WASM_BINARY.ok_or("Testnet wasm not available")?, None)
            .with_name("Pilier Testnet")
            .with_id("pilier_testnet")
            .with_chain_type(ChainType::Live)
            .with_genesis_config_preset_name("pilier_testnet")
            .with_properties(
                json!({
                    "tokenDecimals": 6,
                    "tokenSymbol": "PIL",
                    "ss58Format": 42
                })
                .as_object()
                .expect("Map given; qed")
                .clone(),
            )
            .build(),
    )
}
