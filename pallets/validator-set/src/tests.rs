use crate::{Error, Event, Validators, mock::*};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError;

/// Shorthand for reading the current validator set out of storage in these tests.
fn validators() -> Vec<AccountId> {
    Validators::<Test>::get()
}

#[test]
fn add_validator_adds() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_eq!(validators(), vec![1, 2, 3]);

        assert_ok!(ValidatorSet::add_validator(RuntimeOrigin::root(), 4));

        assert_eq!(validators(), vec![1, 2, 3, 4]);
        System::assert_last_event(Event::ValidatorAdded { who: 4 }.into());
    });
}

#[test]
fn add_validator_rejects_duplicate() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ValidatorSet::add_validator(RuntimeOrigin::root(), 2),
            Error::<Test>::AlreadyValidator
        );
    });
}

#[test]
fn remove_validator_removes() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // Genesis starts with three validators and `MinValidators` is two, so removing one
        // still leaves the set above the floor.
        assert_ok!(ValidatorSet::remove_validator(RuntimeOrigin::root(), 3));

        assert_eq!(validators(), vec![1, 2]);
        System::assert_last_event(Event::ValidatorRemoved { who: 3 }.into());
    });
}

#[test]
fn remove_validator_below_min_validators_is_rejected() {
    new_test_ext().execute_with(|| {
        // Bring the set down to exactly `MinValidators` (2)...
        assert_ok!(ValidatorSet::remove_validator(RuntimeOrigin::root(), 3));
        assert_eq!(validators(), vec![1, 2]);

        // ...then a further removal would breach the floor and must be rejected.
        assert_noop!(
            ValidatorSet::remove_validator(RuntimeOrigin::root(), 2),
            Error::<Test>::TooFewValidators
        );
        assert_eq!(validators(), vec![1, 2]);
    });
}

#[test]
fn remove_validator_rejects_unknown_account() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ValidatorSet::remove_validator(RuntimeOrigin::root(), 42),
            Error::<Test>::NotValidator
        );
    });
}

#[test]
fn non_add_remove_origin_is_rejected() {
    new_test_ext().execute_with(|| {
        // `AddRemoveOrigin` is `EnsureRoot` in the mock; any merely-signed account (even one
        // that is itself a validator) must be turned away.
        assert_noop!(
            ValidatorSet::add_validator(RuntimeOrigin::signed(1), 4),
            DispatchError::BadOrigin
        );
        assert_noop!(
            ValidatorSet::remove_validator(RuntimeOrigin::signed(1), 2),
            DispatchError::BadOrigin
        );

        // Confirm nothing changed.
        assert_eq!(validators(), vec![1, 2, 3]);
    });
}
