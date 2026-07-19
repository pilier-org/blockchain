// Copyright (C) 2026 Pilier Team.
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the mutable-validator-set work (Phase 4b): proving that
//! the *composed* runtime — `pallet-validator-set`, `pallet-session` and the validators' council
//! (`pallet-collective`, instance `Council`) wired together in `runtime/src/configs/mod.rs` —
//! actually behaves the way Phase 4a's wiring claims, not merely that it compiles.
//!
//! These tests build the real `Runtime` type (not a hand-rolled mock), because the risk Phase 4b
//! guards against is specifically a mismatch between the mock's wiring and the runtime's wiring;
//! testing a mock would not catch that. Every test below builds its own fresh, in-memory
//! externalities (`sp_io::TestExternalities`) from a `RuntimeGenesisConfig`, the same genesis
//! struct `runtime/src/genesis_config_presets.rs` builds for the real chain, so the account/key
//! plumbing here matches production exactly.
//!
//! `pallet-session` needs a `pallet-collective` recap for two mechanics these tests lean on
//! directly:
//!
//! - **A validator-set change reaches `Session::validators()` two session rotations after it is
//!   made, not one.** `pallet_session::Pallet::rotate_session()` promotes whatever was *queued*
//!   at the previous rotation to be the current validator set, and only then asks
//!   `SessionManager::new_session` to compute what gets queued *next*. So a change made before
//!   rotation N is queued by rotation N, and only becomes the active set at rotation N+1.
//! - **A newly added validator is silently dropped from that queue unless it already has session
//!   keys registered** (`pallet_session::Pallet::load_keys` gates entry into `QueuedKeys` during
//!   `rotate_session`). Real nodes call `session.set_keys` before asking to be admitted; these
//!   tests do the same via the `Session::set_keys` extrinsic.
//!
//! Driving these rotations calls `Session::rotate_session()` directly rather than stepping
//! `System::block_number()` up to a real `Period` boundary and letting `on_initialize` discover
//! it: at `Period = 100` that would mean 100 no-op blocks per rotation for no additional
//! assurance, and pushing blocks through Aura/Timestamp inherents just to reach that block number
//! would require slot digests this test harness has no reason to construct.

// Gated as `#[cfg(test)] mod tests;` in `lib.rs`; no need to repeat the gate here.

extern crate alloc;

use alloc::boxed::Box;

use codec::Encode;
use frame_support::instances::Instance1;
use frame_support::{assert_noop, assert_ok};
use sp_core::{Pair, Public, sr25519};
use sp_runtime::{
    DispatchError,
    traits::{BlakeTwo256, Hash as _, IdentifyAccount},
};

use crate::{
    AccountId, AccountPublic, AuraId, Balance, BalancesConfig, BuildStorage, Council,
    CouncilConfig, GrandpaId, Runtime, RuntimeCall, RuntimeEvent, RuntimeGenesisConfig,
    RuntimeOrigin, Session, SessionKeys, Sudo, SudoConfig, System, UNIT, ValidatorSet,
};

/// Derives a deterministic keypair from a `//<seed>` derivation path — the same scheme
/// `node/src/chain_spec.rs` uses for its `dev`/`local` presets — and returns the public key in
/// whatever concrete key type the caller asks for (`sr25519::Public`, `AuraId`, `GrandpaId`, …).
fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{seed}"), None)
        .expect("hard-coded test seed is a valid derivation path; qed")
        .public()
}

/// The sovereign `AccountId` derived from an sr25519 seed, the same derivation
/// `runtime/src/genesis_config_presets.rs` and `node/src/chain_spec.rs` use for validators (their
/// account IS their Aura public key, wrapped as an `AccountId`).
fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// A validator's sovereign account plus its Aura and GRANDPA session keys, all derived from the
/// same seed — mirrors `validator_keys_from_seed` in `node/src/chain_spec.rs`.
fn validator_keys_from_seed(seed: &str) -> (AccountId, AuraId, GrandpaId) {
    (
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<AuraId>(seed),
        get_from_seed::<GrandpaId>(seed),
    )
}

/// Builds a fresh in-memory externalities seeded the same way the real chain's genesis is
/// seeded: `validators` become both the initial `ValidatorSet` membership and the council's
/// initial membership (see `build_genesis_config` in `genesis_config_presets.rs`, which this
/// mirrors), `root` becomes the Sudo key, and every account in `extra_endowed` gets a small
/// balance so it already "exists" on chain — needed, for instance, before an account can call
/// `session.set_keys` (`pallet_session::Pallet::do_set_keys` requires `can_inc_consumer`, which
/// requires the account to already have a provider reference).
fn new_test_ext(
    validators: &[(AccountId, AuraId, GrandpaId)],
    root: &AccountId,
    extra_endowed: &[AccountId],
) -> sp_io::TestExternalities {
    let initial_validators: alloc::vec::Vec<AccountId> = validators
        .iter()
        .map(|(account, _, _)| account.clone())
        .collect();

    let session_keys: alloc::vec::Vec<(AccountId, AccountId, SessionKeys)> = validators
        .iter()
        .cloned()
        .map(|(account, aura, grandpa)| (account.clone(), account, SessionKeys { aura, grandpa }))
        .collect();

    let mut balances: alloc::vec::Vec<(AccountId, Balance)> = initial_validators
        .iter()
        .cloned()
        .map(|account| (account, 10 * UNIT))
        .collect();
    balances.push((root.clone(), 10 * UNIT));
    for account in extra_endowed {
        balances.push((account.clone(), 10 * UNIT));
    }

    let config = RuntimeGenesisConfig {
        balances: BalancesConfig {
            balances,
            dev_accounts: None,
        },
        aura: pallet_aura::GenesisConfig {
            authorities: alloc::vec::Vec::new(),
        },
        grandpa: pallet_grandpa::GenesisConfig {
            authorities: alloc::vec::Vec::new(),
            ..Default::default()
        },
        session: pallet_session::GenesisConfig {
            keys: session_keys,
            ..Default::default()
        },
        council: CouncilConfig {
            members: initial_validators.clone(),
            phantom: Default::default(),
        },
        validator_set: pallet_validator_set::GenesisConfig { initial_validators },
        sudo: SudoConfig {
            key: Some(root.clone()),
        },
        ..Default::default()
    };

    let storage = config
        .build_storage()
        .expect("test genesis config must build");
    sp_io::TestExternalities::new(storage)
}

/// Advances the chain by one block and forces `pallet-session` to rotate immediately, regardless
/// of `Period`/`Offset` — see the module doc comment for why this is preferable to stepping
/// through real block numbers in a test.
fn advance_session() {
    let now = System::block_number();
    System::set_block_number(now + 1);
    Session::rotate_session();
}

/// Asserts that `actual` holds exactly the same accounts as `expected`, ignoring order.
///
/// Needed wherever we compare `Validators`/`Members` storage *after* an `add_validator` or
/// `remove_validator` call has run: `Pallet::add_validator` sorts the vector before writing it
/// back (`validators.sort()`), so its order after a mutation is "sorted by `AccountId` bytes",
/// not "genesis insertion order" — unlike the genesis build itself, which stores the vector
/// exactly as given, and unlike `remove_validator`'s `retain`, which preserves whatever order was
/// already there. Comparing by exact `Vec` equality after a mutation would make these tests
/// depend on incidental byte-ordering of unrelated test seeds.
fn assert_same_members(actual: alloc::vec::Vec<AccountId>, expected: &[AccountId]) {
    let mut actual = actual;
    actual.sort();
    let mut expected = expected.to_vec();
    expected.sort();
    assert_eq!(actual, expected);
}

/// The dispatch result of the most recent `pallet_collective::Event::Executed` — i.e. whether the
/// call a just-closed council motion tried to run actually succeeded once it reached
/// `T::AddRemoveOrigin::ensure_origin`, as opposed to whether the *motion itself* resolved.
/// Those are two different questions: `Council::close` can return `Ok` (the motion resolved)
/// while the call it executed returned `Err(BadOrigin)` (the origin gate rejected it).
fn last_council_execution_result() -> sp_runtime::DispatchResult {
    System::events()
        .into_iter()
        .rev()
        .find_map(|record| match record.event {
            RuntimeEvent::Council(pallet_collective::Event::Executed { result, .. }) => {
                Some(result)
            }
            _ => None,
        })
        .expect("a Council Executed event must have been deposited by the close just performed")
}

#[test]
fn council_supermajority_admits_a_validator_and_it_reaches_the_active_set() {
    let v1 = validator_keys_from_seed("Val1");
    let v2 = validator_keys_from_seed("Val2");
    let v3 = validator_keys_from_seed("Val3");
    let root = get_account_id_from_seed::<sr25519::Public>("Root");
    let candidate = get_account_id_from_seed::<sr25519::Public>("Candidate");
    let candidate_aura = get_from_seed::<AuraId>("Candidate");
    let candidate_grandpa = get_from_seed::<GrandpaId>("Candidate");

    let mut ext = new_test_ext(
        &[v1.clone(), v2.clone(), v3.clone()],
        &root,
        core::slice::from_ref(&candidate),
    );

    ext.execute_with(|| {
        // Events are silently dropped at block 0 (`frame_system::Pallet::deposit_event` no-ops
        // on the genesis block), and `last_council_execution_result` below depends on seeing
        // them, so every test advances past genesis first — the same `System::set_block_number(1)`
        // idiom `pallet-session`'s own tests use.
        System::set_block_number(1);

        assert_eq!(
            pallet_validator_set::Validators::<Runtime>::get(),
            alloc::vec![v1.0.clone(), v2.0.clone(), v3.0.clone()]
        );
        assert_eq!(
            pallet_collective::Members::<Runtime, Instance1>::get(),
            alloc::vec![v1.0.clone(), v2.0.clone(), v3.0.clone()]
        );

        // The candidate registers its session keys up front, exactly as a real node would before
        // asking the council to admit it.
        assert_ok!(Session::set_keys(
            RuntimeOrigin::signed(candidate.clone()),
            SessionKeys {
                aura: candidate_aura,
                grandpa: candidate_grandpa
            },
            alloc::vec::Vec::new(),
        ));

        let call = RuntimeCall::ValidatorSet(pallet_validator_set::Call::add_validator {
            who: candidate.clone(),
        });
        let proposal_hash = BlakeTwo256::hash_of(&call);
        let proposal_len = call.encode().len() as u32;
        let weight_bound = crate::configs::CouncilMaxProposalWeight::get();

        // --- Phase A: 2 of 3 ayes must NOT clear the 75% supermajority origin check. ---
        //
        // `Collective::propose`'s `threshold` argument (2 here) is the collective's OWN
        // close-quorum: how many ayes let `close` resolve the motion before `MotionDuration`
        // elapses. It is a different number from the 75% supermajority
        // (`EnsureProportionAtLeast<.., 3, 4>`) that gates the *dispatched* call once the motion
        // closes. Setting the collective's own quorum to 2 lets the motion close early on 2 ayes
        // out of 3 members, so we can show that the *origin check* still rejects it even though
        // the collective itself considers the motion settled: `2 * 4 = 8 < 3 * 3 = 9`.
        assert_ok!(Council::propose(
            RuntimeOrigin::signed(v1.0.clone()),
            2,
            Box::new(call.clone()),
            proposal_len,
        ));
        assert_ok!(Council::vote(
            RuntimeOrigin::signed(v1.0.clone()),
            proposal_hash,
            0,
            true
        ));
        assert_ok!(Council::vote(
            RuntimeOrigin::signed(v2.0.clone()),
            proposal_hash,
            0,
            true
        ));
        // `close` succeeds at the collective level (the motion resolved on 2/3 ayes)...
        assert_ok!(Council::close(
            RuntimeOrigin::signed(v1.0.clone()),
            proposal_hash,
            0,
            weight_bound,
            proposal_len,
        ));
        // ...but the call it tried to run was rejected by `AddRemoveOrigin`, and the validator
        // set is untouched.
        assert_eq!(
            last_council_execution_result(),
            Err(DispatchError::BadOrigin)
        );
        assert_eq!(
            pallet_validator_set::Validators::<Runtime>::get(),
            alloc::vec![v1.0.clone(), v2.0.clone(), v3.0.clone()]
        );

        // --- Phase B: all 3 of 3 ayes clears the supermajority and executes the call. ---
        //
        // The first motion was removed from storage when it closed (approved or not), so
        // re-proposing the identical call is not a duplicate; its proposal index is 1 (the
        // collective's `ProposalCount` keeps incrementing across proposals, closed or not).
        assert_ok!(Council::propose(
            RuntimeOrigin::signed(v1.0.clone()),
            3,
            Box::new(call.clone()),
            proposal_len,
        ));
        assert_ok!(Council::vote(
            RuntimeOrigin::signed(v1.0.clone()),
            proposal_hash,
            1,
            true
        ));
        assert_ok!(Council::vote(
            RuntimeOrigin::signed(v2.0.clone()),
            proposal_hash,
            1,
            true
        ));
        assert_ok!(Council::vote(
            RuntimeOrigin::signed(v3.0.clone()),
            proposal_hash,
            1,
            true
        ));
        assert_ok!(Council::close(
            RuntimeOrigin::signed(v1.0.clone()),
            proposal_hash,
            1,
            weight_bound,
            proposal_len,
        ));
        assert_eq!(last_council_execution_result(), Ok(()));

        // The validator-set change, and the council-membership sync it triggers via
        // `MembershipChanged`, are immediate — no session rotation is needed to observe them.
        assert_same_members(
            pallet_validator_set::Validators::<Runtime>::get(),
            &[v1.0.clone(), v2.0.clone(), v3.0.clone(), candidate.clone()],
        );
        assert_same_members(
            pallet_collective::Members::<Runtime, Instance1>::get(),
            &[v1.0.clone(), v2.0.clone(), v3.0.clone(), candidate.clone()],
        );

        // But `pallet-session` only *plans* the new set at the next rotation, and only *applies*
        // it — makes it visible in `Session::validators()` — one rotation after that.
        assert!(!Session::validators().contains(&candidate));
        advance_session();
        assert!(
            !Session::validators().contains(&candidate),
            "still only queued, not yet active"
        );
        advance_session();
        assert!(
            Session::validators().contains(&candidate),
            "now active after the second rotation"
        );
    });
}

#[test]
fn council_supermajority_removes_a_validator_and_it_leaves_the_active_set() {
    let v1 = validator_keys_from_seed("Val1");
    let v2 = validator_keys_from_seed("Val2");
    let v3 = validator_keys_from_seed("Val3");
    let root = get_account_id_from_seed::<sr25519::Public>("Root");

    let mut ext = new_test_ext(&[v1.clone(), v2.clone(), v3.clone()], &root, &[]);

    ext.execute_with(|| {
        System::set_block_number(1);

        let call = RuntimeCall::ValidatorSet(pallet_validator_set::Call::remove_validator {
            who: v3.0.clone(),
        });
        let proposal_hash = BlakeTwo256::hash_of(&call);
        let proposal_len = call.encode().len() as u32;
        let weight_bound = crate::configs::CouncilMaxProposalWeight::get();

        // Unanimous vote (3 of 3), including the validator being removed voting on its own
        // removal — nothing in the pallet or the origin forbids that.
        assert_ok!(Council::propose(
            RuntimeOrigin::signed(v1.0.clone()),
            3,
            Box::new(call.clone()),
            proposal_len,
        ));
        assert_ok!(Council::vote(
            RuntimeOrigin::signed(v1.0.clone()),
            proposal_hash,
            0,
            true
        ));
        assert_ok!(Council::vote(
            RuntimeOrigin::signed(v2.0.clone()),
            proposal_hash,
            0,
            true
        ));
        assert_ok!(Council::vote(
            RuntimeOrigin::signed(v3.0.clone()),
            proposal_hash,
            0,
            true
        ));
        assert_ok!(Council::close(
            RuntimeOrigin::signed(v1.0.clone()),
            proposal_hash,
            0,
            weight_bound,
            proposal_len,
        ));
        assert_eq!(last_council_execution_result(), Ok(()));

        // `MinValidators = ConstU32<1>` is not violated: 3 - 1 = 2 remain. Immediate effects,
        // same as the add case: `ValidatorSet` storage and the council's membership both update
        // synchronously.
        assert_eq!(
            pallet_validator_set::Validators::<Runtime>::get(),
            alloc::vec![v1.0.clone(), v2.0.clone()]
        );
        assert_eq!(
            pallet_collective::Members::<Runtime, Instance1>::get(),
            alloc::vec![v1.0.clone(), v2.0.clone()]
        );

        // `Session::validators()` still shows all 3 until two more rotations have passed.
        assert!(Session::validators().contains(&v3.0));
        advance_session();
        assert!(
            Session::validators().contains(&v3.0),
            "still queued out, not yet gone"
        );
        advance_session();
        assert!(
            !Session::validators().contains(&v3.0),
            "removed after the second rotation"
        );
        assert!(Session::validators().contains(&v1.0));
        assert!(Session::validators().contains(&v2.0));
    });
}

#[test]
fn root_adds_a_validator_directly_bypassing_the_council() {
    let v1 = validator_keys_from_seed("Val1");
    let v2 = validator_keys_from_seed("Val2");
    let v3 = validator_keys_from_seed("Val3");
    let root = get_account_id_from_seed::<sr25519::Public>("Root");
    let candidate = get_account_id_from_seed::<sr25519::Public>("Candidate");

    let mut ext = new_test_ext(
        &[v1.clone(), v2.clone(), v3.clone()],
        &root,
        core::slice::from_ref(&candidate),
    );

    ext.execute_with(|| {
        System::set_block_number(1);

        // The council is never consulted here: this exercises the "root as an emergency lever"
        // half of `AddRemoveOrigin = EitherOfDiverse<EnsureRoot<AccountId>, EnsureProportionAtLeast<..>>`,
        // going through the real `pallet-sudo` onboarding path (a signed call from the Sudo key,
        // dispatched onward with `RawOrigin::Root`).
        assert_ok!(Sudo::sudo(
            RuntimeOrigin::signed(root.clone()),
            Box::new(RuntimeCall::ValidatorSet(
                pallet_validator_set::Call::add_validator {
                    who: candidate.clone(),
                }
            )),
        ));

        // `Sudo::sudo` itself always returns `Ok` once the caller is the stored Sudo key — the
        // inner call's own result is what proves the origin check actually let it through.
        // `add_validator` sorts before writing back, so compare membership, not exact order.
        assert_same_members(
            pallet_validator_set::Validators::<Runtime>::get(),
            &[v1.0.clone(), v2.0.clone(), v3.0.clone(), candidate.clone()],
        );
        assert_same_members(
            pallet_collective::Members::<Runtime, Instance1>::get(),
            &[v1.0.clone(), v2.0.clone(), v3.0.clone(), candidate.clone()],
        );
    });
}

#[test]
fn plain_signed_origin_is_rejected() {
    let v1 = validator_keys_from_seed("Val1");
    let v2 = validator_keys_from_seed("Val2");
    let v3 = validator_keys_from_seed("Val3");
    let root = get_account_id_from_seed::<sr25519::Public>("Root");
    let outsider = get_account_id_from_seed::<sr25519::Public>("Outsider");
    let candidate = get_account_id_from_seed::<sr25519::Public>("Candidate");

    let mut ext = new_test_ext(
        &[v1.clone(), v2.clone(), v3.clone()],
        &root,
        core::slice::from_ref(&outsider),
    );

    ext.execute_with(|| {
        // A plain signed account that is neither root nor a council member (let alone a council
        // supermajority) clears neither half of `EitherOfDiverse<EnsureRoot<..>, EnsureProportionAtLeast<..>>`.
        assert_noop!(
            ValidatorSet::add_validator(RuntimeOrigin::signed(outsider.clone()), candidate.clone()),
            DispatchError::BadOrigin,
        );

        // For completeness: even a council MEMBER calling directly with its own plain signed
        // origin (i.e. not via `Council::propose`/`vote`/`close`, which alone produces a
        // `RawOrigin::Members` origin) is rejected the same way — being on the council is not
        // itself sufficient, only a `RawOrigin::Members` origin carrying a 75% majority is.
        assert_noop!(
            ValidatorSet::add_validator(RuntimeOrigin::signed(v1.0.clone()), candidate.clone()),
            DispatchError::BadOrigin,
        );

        assert_eq!(
            pallet_validator_set::Validators::<Runtime>::get(),
            alloc::vec![v1.0.clone(), v2.0.clone(), v3.0.clone()]
        );
    });
}
