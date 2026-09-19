use crate::{Call, DocumentPrice, Error, FileFingerprint, Files, mock::*};
use frame_support::{assert_noop, assert_ok, dispatch::Pays};
use sp_runtime::{DispatchError, TokenError};

/// A plain outsider account — belonging to no project at all — is rejected with
/// `Error::NotProjectMember`.
#[test]
fn outsider_belonging_to_no_project_is_rejected() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");
        let project_id = create_project(1);

        assert_noop!(
            Documents::register_file(
                RuntimeOrigin::signed(99),
                project_id,
                [7u8; 32],
                endpoint_id,
                b"certificates/iso-14001.pdf".to_vec(),
                b"application/pdf".to_vec(),
            ),
            Error::<Test>::NotProjectMember
        );
    });
}

/// A project's owner may register a file under it.
#[test]
fn project_owner_may_register_file() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");
        let project_id = create_project(1);
        fund_account(1, 10_000);

        assert_ok!(Documents::register_file(
            RuntimeOrigin::signed(1),
            project_id,
            [7u8; 32],
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        ));
    });
}

/// An account added to a project's writer list may register a file under it.
#[test]
fn project_writer_may_register_file() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");
        let project_id = create_project(1);
        set_project_writers(project_id, vec![2]);
        fund_account(2, 10_000);

        assert_ok!(Documents::register_file(
            RuntimeOrigin::signed(2),
            project_id,
            [7u8; 32],
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        ));
    });
}

/// An account removed from a project's writer list by `set_project_writers` is rejected by the
/// very next `register_file` call under that project.
#[test]
fn writer_removed_by_set_project_writers_is_rejected_on_next_call() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");
        let project_id = create_project(1);
        set_project_writers(project_id, vec![2]);
        fund_account(2, 10_000);
        assert_ok!(Documents::register_file(
            RuntimeOrigin::signed(2),
            project_id,
            [7u8; 32],
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        ));

        set_project_writers(project_id, vec![]);

        assert_noop!(
            Documents::register_file(
                RuntimeOrigin::signed(2),
                project_id,
                [8u8; 32],
                endpoint_id,
                b"certificates/other.pdf".to_vec(),
                b"application/pdf".to_vec(),
            ),
            Error::<Test>::NotProjectMember
        );
    });
}

/// A project identifier that names no existing project is rejected with
/// `Error::NotProjectMember`, the same error a real but foreign project gets.
#[test]
fn nonexistent_project_is_rejected() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");

        assert_noop!(
            Documents::register_file(
                RuntimeOrigin::signed(1),
                999,
                [7u8; 32],
                endpoint_id,
                b"certificates/iso-14001.pdf".to_vec(),
                b"application/pdf".to_vec(),
            ),
            Error::<Test>::NotProjectMember
        );
    });
}

/// Registering a new fingerprint stores the named project as `registered_by` and the block
/// number it was registered in.
#[test]
fn new_registration_stores_project_and_block_number() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");
        let project_id = create_project(1);
        fund_account(1, 10_000);
        set_block_number(5);

        assert_ok!(Documents::register_file(
            RuntimeOrigin::signed(1),
            project_id,
            [7u8; 32],
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        ));

        let stored = Files::<Test>::get([7u8; 32]).expect("file must be registered");
        assert_eq!(stored.registered_by, project_id);
        assert_eq!(stored.registered_at, 5);
        assert_eq!(stored.storage_endpoint_id, endpoint_id);
        assert_eq!(&stored.path[..], b"certificates/iso-14001.pdf");
        assert_eq!(&stored.content_type[..], b"application/pdf");
    });
}

/// Repeating an already-registered fingerprint with matching data, from the same project,
/// changes nothing and charges nothing.
#[test]
fn repeating_matching_data_same_project_is_free_noop() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");
        let project_id = create_project(1);
        fund_account(1, 100_000);

        assert_ok!(Documents::register_file(
            RuntimeOrigin::signed(1),
            project_id,
            [7u8; 32],
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        ));
        let balance_after_first = pallet_balances::Pallet::<Test>::free_balance(1);

        assert_ok!(Documents::register_file(
            RuntimeOrigin::signed(1),
            project_id,
            [7u8; 32],
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        ));

        assert_eq!(
            pallet_balances::Pallet::<Test>::free_balance(1),
            balance_after_first
        );
    });
}

/// Repeating an already-registered fingerprint with matching data, from a *different*
/// council-approved project, still changes nothing: `registered_by` keeps naming the first
/// project, and nothing is charged.
#[test]
fn repeating_matching_data_different_project_does_not_change_registered_by() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");
        let first_project = create_project(1);
        let second_project = create_project(2);
        fund_account(1, 100_000);
        fund_account(2, 100_000);

        assert_ok!(Documents::register_file(
            RuntimeOrigin::signed(1),
            first_project,
            [7u8; 32],
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        ));

        let balance_before = pallet_balances::Pallet::<Test>::free_balance(2);
        assert_ok!(Documents::register_file(
            RuntimeOrigin::signed(2),
            second_project,
            [7u8; 32],
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        ));

        let stored = Files::<Test>::get([7u8; 32]).expect("file must still be registered");
        assert_eq!(stored.registered_by, first_project);
        assert_eq!(
            pallet_balances::Pallet::<Test>::free_balance(2),
            balance_before,
            "the second, matching-data call must not be charged"
        );
    });
}

/// Registering an already-known fingerprint with a different address is rejected with
/// `Error::FileDataMismatch`, and the existing entry is not overwritten.
#[test]
fn registering_existing_fingerprint_with_different_data_is_rejected() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");
        let project_id = create_project(1);
        fund_account(1, 10_000);

        assert_ok!(Documents::register_file(
            RuntimeOrigin::signed(1),
            project_id,
            [7u8; 32],
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        ));

        assert_noop!(
            Documents::register_file(
                RuntimeOrigin::signed(1),
                project_id,
                [7u8; 32],
                endpoint_id,
                b"certificates/different-file.pdf".to_vec(),
                b"application/pdf".to_vec(),
            ),
            Error::<Test>::FileDataMismatch
        );

        let stored = Files::<Test>::get([7u8; 32]).expect("original entry must remain");
        assert_eq!(&stored.path[..], b"certificates/iso-14001.pdf");
    });
}

/// Registering a file against a storage endpoint that does not exist is rejected, exercising
/// `RegistryAccess::storage_endpoint_exists`.
#[test]
fn registering_file_against_unknown_storage_endpoint_is_rejected() {
    new_test_ext().execute_with(|| {
        let project_id = create_project(1);

        assert_noop!(
            Documents::register_file(
                RuntimeOrigin::signed(1),
                project_id,
                [1u8; 32],
                999,
                b"certificates/iso-14001.pdf".to_vec(),
                b"application/pdf".to_vec(),
            ),
            Error::<Test>::StorageEndpointNotFound
        );
    });
}

/// A path at the exact configured ceiling is accepted; one byte longer is rejected with
/// `Error::FilePathTooLong`.
#[test]
fn path_at_exact_ceiling_passes_one_byte_over_is_rejected() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");
        let project_id = create_project(1);
        fund_account(1, 10_000);

        let at_ceiling = vec![0u8; MaxFilePathLen::get() as usize];
        assert_ok!(Documents::register_file(
            RuntimeOrigin::signed(1),
            project_id,
            [1u8; 32],
            endpoint_id,
            at_ceiling,
            b"application/pdf".to_vec(),
        ));

        let over_ceiling = vec![0u8; MaxFilePathLen::get() as usize + 1];
        assert_noop!(
            Documents::register_file(
                RuntimeOrigin::signed(1),
                project_id,
                [2u8; 32],
                endpoint_id,
                over_ceiling,
                b"application/pdf".to_vec(),
            ),
            Error::<Test>::FilePathTooLong
        );
    });
}

/// A content type at the exact configured ceiling is accepted; one byte longer is rejected with
/// `Error::FileContentTypeTooLong`.
#[test]
fn content_type_at_exact_ceiling_passes_one_byte_over_is_rejected() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");
        let project_id = create_project(1);
        fund_account(1, 10_000);

        let at_ceiling = vec![0u8; MaxFileContentTypeLen::get() as usize];
        assert_ok!(Documents::register_file(
            RuntimeOrigin::signed(1),
            project_id,
            [1u8; 32],
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            at_ceiling,
        ));

        let over_ceiling = vec![0u8; MaxFileContentTypeLen::get() as usize + 1];
        assert_noop!(
            Documents::register_file(
                RuntimeOrigin::signed(1),
                project_id,
                [2u8; 32],
                endpoint_id,
                b"certificates/iso-14001.pdf".to_vec(),
                over_ceiling,
            ),
            Error::<Test>::FileContentTypeTooLong
        );
    });
}

/// A posterior outsider — belonging to no project — who names an already-registered fingerprint
/// with matching data still gets `Error::NotProjectMember`, never the free no-op: the membership
/// check runs before the fingerprint is even looked up.
#[test]
fn outsider_repeating_already_registered_fingerprint_still_gets_not_project_member() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");
        let project_id = create_project(1);
        fund_account(1, 10_000);
        assert_ok!(Documents::register_file(
            RuntimeOrigin::signed(1),
            project_id,
            [7u8; 32],
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        ));

        assert_noop!(
            Documents::register_file(
                RuntimeOrigin::signed(99),
                999,
                [7u8; 32],
                endpoint_id,
                b"certificates/iso-14001.pdf".to_vec(),
                b"application/pdf".to_vec(),
            ),
            Error::<Test>::NotProjectMember
        );
    });
}

/// A new registration charges exactly `DocumentPrice` to the caller.
#[test]
fn new_registration_charges_exactly_document_price() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");
        let project_id = create_project(1);
        fund_account(1, 10_000);
        DocumentPrice::<Test>::put(2_500);

        assert_ok!(Documents::register_file(
            RuntimeOrigin::signed(1),
            project_id,
            [7u8; 32],
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        ));

        assert_eq!(pallet_balances::Pallet::<Test>::free_balance(1), 7_500);
    });
}

/// The charged amount is credited to the current block's author.
#[test]
fn charged_amount_is_credited_to_current_block_author() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");
        let project_id = create_project(1);
        set_block_author(2);
        fund_account(1, 10_000);
        DocumentPrice::<Test>::put(2_500);

        let result = Documents::register_file(
            RuntimeOrigin::signed(1),
            project_id,
            [7u8; 32],
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        );

        assert_eq!(result.expect("call must succeed").pays_fee, Pays::No);
        assert_eq!(pallet_balances::Pallet::<Test>::free_balance(1), 7_500);
        assert_eq!(pallet_balances::Pallet::<Test>::free_balance(2), 2_500);
    });
}

/// When no block author can be determined, the charged amount is burned instead of credited to
/// anyone, and the registration still succeeds.
#[test]
fn charge_is_burned_when_no_block_author_determined() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");
        let project_id = create_project(1);
        fund_account(1, 10_000);
        DocumentPrice::<Test>::put(2_500);
        assert_eq!(pallet_authorship::Pallet::<Test>::author(), None);
        let issuance_before = pallet_balances::Pallet::<Test>::total_issuance();

        assert_ok!(Documents::register_file(
            RuntimeOrigin::signed(1),
            project_id,
            [7u8; 32],
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        ));

        assert_eq!(pallet_balances::Pallet::<Test>::free_balance(1), 7_500);
        assert_eq!(
            pallet_balances::Pallet::<Test>::total_issuance(),
            issuance_before - 2_500
        );
        assert!(Files::<Test>::contains_key([7u8; 32]));
    });
}

/// A caller who cannot afford `DocumentPrice` is rejected with `TokenError::FundsUnavailable`,
/// and no file table entry is created.
#[test]
fn insufficient_funds_for_document_price_leaves_no_record() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");
        let project_id = create_project(1);
        DocumentPrice::<Test>::put(2_500);
        fund_account(1, 2_499);

        let result = Documents::register_file(
            RuntimeOrigin::signed(1),
            project_id,
            [7u8; 32],
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        );

        let err = result.expect_err("must fail: payer cannot afford the document price");
        assert_eq!(
            err.error,
            DispatchError::Token(TokenError::FundsUnavailable)
        );
        assert!(!Files::<Test>::contains_key([7u8; 32]));
    });
}

/// Through the real fee pipeline, a new registration does not pay the standard transaction fee
/// (refunded via `Pays::No`) — its total charge is `DocumentPrice` alone — while a repeat
/// registration of the same data pays the standard fee in full, since it returns `Pays::Yes`.
#[test]
fn new_registration_skips_standard_fee_but_repeat_pays_it() {
    new_test_ext().execute_with(|| {
        let endpoint_id = setup_storage_endpoint(1, b"552100554");
        let project_id = create_project(1);
        fund_account(1, 1_000_000);
        DocumentPrice::<Test>::put(2_500);

        let first_call = RuntimeCall::Documents(Call::<Test>::register_file {
            project_id,
            fingerprint: [7u8; 32],
            storage_endpoint_id: endpoint_id,
            path: b"certificates/iso-14001.pdf".to_vec(),
            content_type: b"application/pdf".to_vec(),
        });
        let balance_before_first = pallet_balances::Pallet::<Test>::free_balance(1);
        let (outcome, _standard_fee) = dispatch_through_fee_pipeline(1, 0, first_call);
        outcome
            .expect("transaction must validate")
            .expect("first registration must succeed");
        let balance_after_first = pallet_balances::Pallet::<Test>::free_balance(1);
        assert_eq!(balance_before_first - balance_after_first, 2_500);

        let repeat_call = RuntimeCall::Documents(Call::<Test>::register_file {
            project_id,
            fingerprint: [7u8; 32],
            storage_endpoint_id: endpoint_id,
            path: b"certificates/iso-14001.pdf".to_vec(),
            content_type: b"application/pdf".to_vec(),
        });
        let (outcome, standard_fee) = dispatch_through_fee_pipeline(1, 0, repeat_call);
        assert!(
            standard_fee > 0,
            "the standard fee must be nonzero for this test to prove anything"
        );
        outcome
            .expect("transaction must validate")
            .expect("repeat registration must succeed as a no-op");
        let balance_after_repeat = pallet_balances::Pallet::<Test>::free_balance(1);
        assert_eq!(balance_after_first - balance_after_repeat, standard_fee);
    });
}

/// Reading `DocumentPrice` on storage where it has never been written returns the pallet's own
/// storage default of two thousand five hundred of the chain's smallest unit, and the storage
/// key itself does not exist.
#[test]
fn document_price_reads_as_default_when_never_written() {
    new_test_ext().execute_with(|| {
        assert!(!DocumentPrice::<Test>::exists());
        assert_eq!(DocumentPrice::<Test>::get(), 2_500);
    });
}

/// `set_price` from a plain signed account is rejected with `DispatchError::BadOrigin`, and
/// writes nothing to `DocumentPrice`'s storage key.
#[test]
fn set_price_from_foreign_account_is_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Documents::set_price(RuntimeOrigin::signed(1), 9_000),
            DispatchError::BadOrigin
        );
        assert!(!DocumentPrice::<Test>::exists());
    });
}

/// A council motion carrying exactly three quarters of the vote (three of four) clears
/// `AdminOrigin`, and `DocumentPrice` carries the new value after the call returns.
#[test]
fn set_price_from_council_at_three_quarters_changes_stored_price() {
    new_test_ext().execute_with(|| {
        let origin: RuntimeOrigin =
            pallet_collective::RawOrigin::<AccountId, CouncilCollective>::Members(3, 4).into();
        assert_ok!(Documents::set_price(origin, 9_000));
        assert_eq!(DocumentPrice::<Test>::get(), 9_000);
    });
}

/// `set_price` from root also clears `AdminOrigin`.
#[test]
fn set_price_from_root_changes_stored_price() {
    new_test_ext().execute_with(|| {
        assert_ok!(Documents::set_price(RuntimeOrigin::root(), 9_000));
        assert_eq!(DocumentPrice::<Test>::get(), 9_000);
    });
}

/// A written price of zero overrides the pallet's own nonzero default, rather than the default
/// reasserting itself: the price is first moved to a nonzero value so the final assertion of
/// zero cannot pass merely because storage started unwritten.
#[test]
fn set_price_accepts_zero_price_and_overrides_default() {
    new_test_ext().execute_with(|| {
        assert_ok!(Documents::set_price(RuntimeOrigin::root(), 500));
        assert_eq!(DocumentPrice::<Test>::get(), 500);

        assert_ok!(Documents::set_price(RuntimeOrigin::root(), 0));
        assert_eq!(DocumentPrice::<Test>::get(), 0);
    });
}

/// The mock's own `TransactionByteFee` equals the same literal `runtime/src/configs/mod.rs`
/// computes as `10 * MICRO_UNIT`, named as a direct number rather than compared against an
/// import from the runtime crate, which this pallet cannot depend on without a cycle.
#[test]
fn mock_transaction_byte_fee_matches_runtime_literal() {
    assert_eq!(TransactionByteFee::get(), 10);
}

/// A `FileFingerprint` is exactly thirty-two bytes — sanity check on the type alias this
/// pallet's storage key relies on.
#[test]
fn file_fingerprint_is_thirty_two_bytes() {
    assert_eq!(core::mem::size_of::<FileFingerprint>(), 32);
}
