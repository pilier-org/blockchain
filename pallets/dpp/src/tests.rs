use crate::{
    Call, Error, EventCounts, EventRecord, Events, Files, PublicationPrice,
    migrations::InitializePublicationPrice, mock::*,
};
use frame_support::{
    assert_noop, assert_ok,
    dispatch::Pays,
    traits::{OnRuntimeUpgrade, StorageVersion},
};
use sp_runtime::{DispatchError, TokenError, traits::Hash};

/// Registering a fingerprint that already carries the same storage endpoint, path
/// and content type succeeds and leaves the existing file table entry untouched.
#[test]
fn registering_same_fingerprint_twice_with_matching_data_is_noop() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let endpoint_id = create_storage_endpoint(1, b"552100554");
        let fingerprint = [7u8; 32];

        assert_ok!(Dpp::register_file(
            RuntimeOrigin::signed(1),
            fingerprint,
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        ));
        let first = Files::<Test>::get(fingerprint).expect("file must be registered");

        assert_ok!(Dpp::register_file(
            RuntimeOrigin::signed(1),
            fingerprint,
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        ));
        let second = Files::<Test>::get(fingerprint).expect("file must still be registered");

        assert_eq!(first, second);
    });
}

/// Registering a fingerprint that already exists with a different path is rejected
/// with `Error::FileDataMismatch`, and the existing entry is not overwritten.
#[test]
fn registering_existing_fingerprint_with_different_data_is_rejected() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let endpoint_id = create_storage_endpoint(1, b"552100554");
        let fingerprint = [7u8; 32];

        assert_ok!(Dpp::register_file(
            RuntimeOrigin::signed(1),
            fingerprint,
            endpoint_id,
            b"certificates/iso-14001.pdf".to_vec(),
            b"application/pdf".to_vec(),
        ));

        assert_noop!(
            Dpp::register_file(
                RuntimeOrigin::signed(1),
                fingerprint,
                endpoint_id,
                b"certificates/different-file.pdf".to_vec(),
                b"application/pdf".to_vec(),
            ),
            Error::<Test>::FileDataMismatch
        );

        let stored = Files::<Test>::get(fingerprint).expect("original entry must remain");
        assert_eq!(&stored.path[..], b"certificates/iso-14001.pdf");
    });
}

/// Registering a file against a storage endpoint that does not exist is rejected,
/// exercising `RegistryAccess::storage_endpoint_exists`.
#[test]
fn registering_file_against_unknown_storage_endpoint_is_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dpp::register_file(
                RuntimeOrigin::signed(1),
                [1u8; 32],
                999,
                b"certificates/iso-14001.pdf".to_vec(),
                b"application/pdf".to_vec(),
            ),
            Error::<Test>::StorageEndpointNotFound
        );
    });
}

/// `publish_head` from an account whose project was never granted the given company
/// registration number is rejected with `Error::NoPermissionForRegistrationNumber`.
#[test]
fn publish_head_under_foreign_registration_number_is_rejected() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();

        assert_noop!(
            Dpp::publish_head(
                RuntimeOrigin::signed(2),
                b"552100554".to_vec(),
                b"https://id.gs1.org/01/09506000134352".to_vec(),
                schema_id,
                b"passport body".to_vec(),
                Vec::new(),
            ),
            Error::<Test>::NoPermissionForRegistrationNumber
        );
    });
}

/// `publish_head` referencing a file fingerprint that is not in the file table is
/// rejected with `Error::FileNotRegistered`.
#[test]
fn publish_head_referencing_unregistered_fingerprint_is_rejected() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();

        assert_noop!(
            Dpp::publish_head(
                RuntimeOrigin::signed(1),
                b"552100554".to_vec(),
                b"https://id.gs1.org/01/09506000134352".to_vec(),
                schema_id,
                b"passport body".to_vec(),
                vec![[9u8; 32]],
            ),
            Error::<Test>::FileNotRegistered
        );
    });
}

/// A second `publish_head` on an already-occupied key is rejected with
/// `Error::RecordAlreadyExists`, naming `republish_head`, and leaves the stored body unchanged.
#[test]
fn repeated_publish_head_on_occupied_key_is_rejected_and_body_unchanged() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();

        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"first body".to_vec(),
            Vec::new(),
        ));

        assert_noop!(
            Dpp::publish_head(
                RuntimeOrigin::signed(1),
                b"552100554".to_vec(),
                gs1_id.clone(),
                schema_id,
                b"a worker's unacknowledged retry".to_vec(),
                Vec::new(),
            ),
            Error::<Test>::RecordAlreadyExists
        );

        let stored = get_head(b"552100554", &gs1_id).expect("head record must exist");
        assert_eq!(&stored.body[..], b"first body");
        assert_eq!(stored.version, 1);
    });
}

/// `republish_head` from an account whose project was never granted the given
/// company registration number is rejected with `Error::NoPermissionForRegistrationNumber`.
#[test]
fn republish_head_under_foreign_registration_number_is_rejected() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();

        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"first body".to_vec(),
            Vec::new(),
        ));

        assert_noop!(
            Dpp::republish_head(
                RuntimeOrigin::signed(2),
                b"552100554".to_vec(),
                gs1_id,
                schema_id,
                b"forged replacement".to_vec(),
                Vec::new(),
            ),
            Error::<Test>::NoPermissionForRegistrationNumber
        );
    });
}

/// After `republish_head`, the stored body is the new one, the version is two, and
/// the stored previous-body fingerprint matches the hash of the body that was replaced.
#[test]
fn republish_head_stores_new_body_version_two_and_previous_fingerprint() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();

        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"first body".to_vec(),
            Vec::new(),
        ));
        let expected_previous_fingerprint =
            <Test as frame_system::Config>::Hashing::hash(b"first body");

        assert_ok!(Dpp::republish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"corrected body".to_vec(),
            Vec::new(),
        ));

        let stored = get_head(b"552100554", &gs1_id).expect("head record must exist");
        assert_eq!(&stored.body[..], b"corrected body");
        assert_eq!(stored.version, 2);
        assert_eq!(
            stored.previous_body_fingerprint,
            Some(expected_previous_fingerprint)
        );
    });
}

/// A second `republish_head` raises the version to three.
#[test]
fn second_republish_head_raises_version_to_three() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();

        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"first body".to_vec(),
            Vec::new(),
        ));
        assert_ok!(Dpp::republish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"second body".to_vec(),
            Vec::new(),
        ));
        assert_ok!(Dpp::republish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"third body".to_vec(),
            Vec::new(),
        ));

        let stored = get_head(b"552100554", &gs1_id).expect("head record must exist");
        assert_eq!(stored.version, 3);
    });
}

/// `republish_head` on a key with no existing head record is rejected with
/// `Error::RecordNotFound`, naming `publish_head`, and creates nothing.
#[test]
fn republish_head_on_free_key_is_rejected_and_creates_nothing() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();

        assert_noop!(
            Dpp::republish_head(
                RuntimeOrigin::signed(1),
                b"552100554".to_vec(),
                gs1_id.clone(),
                schema_id,
                b"first body".to_vec(),
                Vec::new(),
            ),
            Error::<Test>::RecordNotFound
        );

        assert!(get_head(b"552100554", &gs1_id).is_none());
    });
}

/// Appended events are assigned consecutive indices starting at zero, without gaps.
#[test]
fn appended_events_go_consecutively_without_gaps() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();
        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"first body".to_vec(),
            Vec::new(),
        ));

        assert_ok!(Dpp::append_event(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            b"shipped".to_vec(),
        ));
        assert_ok!(Dpp::append_event(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            b"received".to_vec(),
        ));
        assert_ok!(Dpp::append_event(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            b"sold".to_vec(),
        ));

        let company: crate::CompanyRegistrationNumber<Test> =
            b"552100554".to_vec().try_into().unwrap();
        let gs1: crate::Gs1Id<Test> = gs1_id.try_into().unwrap();
        assert_eq!(crate::EventCounts::<Test>::get(&company, &gs1), 3);
        assert_eq!(
            &crate::Events::<Test>::get((&company, &gs1, 0))
                .unwrap()
                .body[..],
            b"shipped"
        );
        assert_eq!(
            &crate::Events::<Test>::get((&company, &gs1, 1))
                .unwrap()
                .body[..],
            b"received"
        );
        assert_eq!(
            &crate::Events::<Test>::get((&company, &gs1, 2))
                .unwrap()
                .body[..],
            b"sold"
        );
    });
}

/// An event appended on a nonzero block carries that block's own number, not zero.
/// Runs on block seven, not block zero: `new_test_ext` never advances the block number on its
/// own, so a check against zero would pass equally whether the number came from the system
/// pallet, was hard-coded to zero, or was left at a type's default.
#[test]
fn appended_event_carries_the_block_number_it_was_recorded_in() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();
        set_block_number(7);
        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"first body".to_vec(),
            Vec::new(),
        ));

        assert_ok!(Dpp::append_event(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            b"shipped".to_vec(),
        ));

        let company: crate::CompanyRegistrationNumber<Test> =
            b"552100554".to_vec().try_into().unwrap();
        let gs1: crate::Gs1Id<Test> = gs1_id.try_into().unwrap();
        let stored: EventRecord<Test> = crate::Events::<Test>::get((&company, &gs1, 0)).unwrap();
        assert_eq!(stored.recorded_at, 7);
    });
}

/// Two events of the same passport, appended in different blocks, carry different
/// block numbers matching the block each was recorded in, while their indices still run
/// consecutively. Both blocks used are nonzero, for the same reason as the test above.
#[test]
fn events_from_different_blocks_carry_different_block_numbers_and_consecutive_indices() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();
        set_block_number(3);
        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"first body".to_vec(),
            Vec::new(),
        ));

        assert_ok!(Dpp::append_event(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            b"shipped".to_vec(),
        ));

        set_block_number(9);
        assert_ok!(Dpp::append_event(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            b"received".to_vec(),
        ));

        let company: crate::CompanyRegistrationNumber<Test> =
            b"552100554".to_vec().try_into().unwrap();
        let gs1: crate::Gs1Id<Test> = gs1_id.try_into().unwrap();
        let first: EventRecord<Test> = crate::Events::<Test>::get((&company, &gs1, 0)).unwrap();
        let second: EventRecord<Test> = crate::Events::<Test>::get((&company, &gs1, 1)).unwrap();
        assert_eq!(first.recorded_at, 3);
        assert_eq!(second.recorded_at, 9);
        assert_ne!(first.recorded_at, second.recorded_at);
    });
}

/// Reading back an appended event returns its body exactly as it was passed into
/// `append_event`, byte for byte, alongside the block number.
#[test]
fn reading_an_appended_event_returns_the_body_exactly_as_given() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();
        set_block_number(4);
        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"first body".to_vec(),
            Vec::new(),
        ));

        let body = b"EPCIS ObjectEvent: shipped from Lyon warehouse".to_vec();
        assert_ok!(Dpp::append_event(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            body.clone(),
        ));

        let company: crate::CompanyRegistrationNumber<Test> =
            b"552100554".to_vec().try_into().unwrap();
        let gs1: crate::Gs1Id<Test> = gs1_id.try_into().unwrap();
        let stored: EventRecord<Test> = crate::Events::<Test>::get((&company, &gs1, 0)).unwrap();
        assert_eq!(&stored.body[..], &body[..]);
    });
}

/// Appending an event to a passport with no head record is rejected with
/// `Error::RecordNotFound`.
#[test]
fn appending_event_to_nonexistent_passport_is_rejected() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");

        assert_noop!(
            Dpp::append_event(
                RuntimeOrigin::signed(1),
                b"552100554".to_vec(),
                b"https://id.gs1.org/01/09506000134352".to_vec(),
                b"shipped".to_vec(),
            ),
            Error::<Test>::RecordNotFound
        );
    });
}

/// An event's index is never reused: the counter keeps climbing across many calls.
#[test]
fn event_index_is_never_reused() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();
        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"first body".to_vec(),
            Vec::new(),
        ));

        for _ in 0..5 {
            assert_ok!(Dpp::append_event(
                RuntimeOrigin::signed(1),
                b"552100554".to_vec(),
                gs1_id.clone(),
                b"event".to_vec(),
            ));
        }

        let company: crate::CompanyRegistrationNumber<Test> =
            b"552100554".to_vec().try_into().unwrap();
        let gs1: crate::Gs1Id<Test> = gs1_id.try_into().unwrap();
        assert_eq!(crate::EventCounts::<Test>::get(&company, &gs1), 5);
    });
}

/// An event body of exactly the configured ceiling (one hundred twenty-eight bytes
/// in this mock) is accepted; one byte longer is rejected with `Error::EventTooLong`.
#[test]
fn event_at_exact_ceiling_passes_one_byte_over_is_rejected() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();
        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"first body".to_vec(),
            Vec::new(),
        ));

        let at_ceiling = vec![0u8; MaxEventLen::get() as usize];
        assert_ok!(Dpp::append_event(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            at_ceiling,
        ));

        let over_ceiling = vec![0u8; MaxEventLen::get() as usize + 1];
        assert_noop!(
            Dpp::append_event(
                RuntimeOrigin::signed(1),
                b"552100554".to_vec(),
                gs1_id,
                over_ceiling,
            ),
            Error::<Test>::EventTooLong
        );
    });
}

/// Replacing a head record with `republish_head` leaves the event counter and every
/// already-appended event untouched.
#[test]
fn republish_head_does_not_change_event_count_or_events() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();
        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"first body".to_vec(),
            Vec::new(),
        ));
        assert_ok!(Dpp::append_event(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            b"shipped".to_vec(),
        ));

        assert_ok!(Dpp::republish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"corrected body".to_vec(),
            Vec::new(),
        ));

        let company: crate::CompanyRegistrationNumber<Test> =
            b"552100554".to_vec().try_into().unwrap();
        let gs1: crate::Gs1Id<Test> = gs1_id.try_into().unwrap();
        assert_eq!(crate::EventCounts::<Test>::get(&company, &gs1), 1);
        assert_eq!(
            &crate::Events::<Test>::get((&company, &gs1, 0))
                .unwrap()
                .body[..],
            b"shipped"
        );
    });
}

/// A `publish_head` body of exactly the configured ceiling (four kibibytes in this
/// mock) is accepted; one byte longer is rejected with `Error::RecordBodyTooLong`.
#[test]
fn publish_head_body_at_exact_ceiling_passes_one_byte_over_is_rejected() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();

        let at_ceiling = vec![0u8; MaxRecordBodyLen::get() as usize];
        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            b"https://id.gs1.org/01/09506000134352".to_vec(),
            schema_id,
            at_ceiling,
            Vec::new(),
        ));

        let over_ceiling = vec![0u8; MaxRecordBodyLen::get() as usize + 1];
        assert_noop!(
            Dpp::publish_head(
                RuntimeOrigin::signed(1),
                b"552100554".to_vec(),
                b"https://id.gs1.org/01/09506000134400".to_vec(),
                schema_id,
                over_ceiling,
                Vec::new(),
            ),
            Error::<Test>::RecordBodyTooLong
        );
    });
}

/// A `republish_head` body of exactly the configured ceiling is accepted; one byte
/// longer is rejected with `Error::RecordBodyTooLong`.
#[test]
fn republish_head_body_at_exact_ceiling_passes_one_byte_over_is_rejected() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();
        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"first body".to_vec(),
            Vec::new(),
        ));

        let at_ceiling = vec![0u8; MaxRecordBodyLen::get() as usize];
        assert_ok!(Dpp::republish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            at_ceiling,
            Vec::new(),
        ));

        let over_ceiling = vec![0u8; MaxRecordBodyLen::get() as usize + 1];
        assert_noop!(
            Dpp::republish_head(
                RuntimeOrigin::signed(1),
                b"552100554".to_vec(),
                gs1_id,
                schema_id,
                over_ceiling,
                Vec::new(),
            ),
            Error::<Test>::RecordBodyTooLong
        );
    });
}

/// A realistic passport record body, built from the schema-v1 field composition
/// (product count, label-grade fibre composition, its computed/hand-entered source flag, and
/// the fingerprints of its evidence-file certificates — the passport's GTIN/batch/serial,
/// company registration number and schema number are not repeated here because they are
/// already carried by the storage key and by the head record's own `schema_id` field),
/// SCALE-encoded exactly as this pallet stores it, measures at most one kibibyte — well inside
/// the four-kibibyte ceiling. A realistic lifecycle event — an event type code, a
/// millisecond timestamp, and the thirty-two-byte canonical GS1 event hash the app already
/// computes off-chain (see `dpp_demo/app/internal/epcis/epcis.go`'s `CanonicalEventHash`) —
/// measures at most sixty-four bytes, well inside the one-hundred-twenty-eight-byte ceiling.
#[test]
fn realistic_passport_body_and_event_fit_declared_budgets() {
    use codec::Encode;

    #[derive(Encode)]
    struct CompositionLine {
        fibre: Vec<u8>,
        // Basis points (hundredths of a percent) — label-grade precision, summing to 10 000.
        percentage_bps: u16,
    }

    #[derive(Encode)]
    struct PassportRecordV1 {
        count: u32,
        composition: Vec<CompositionLine>,
        // 0 = computed from consumed batches, 1 = hand-entered (free-tier fallback).
        composition_source: u8,
        certificate_fingerprints: Vec<[u8; 32]>,
    }

    let record = PassportRecordV1 {
        count: 240,
        composition: vec![
            CompositionLine {
                fibre: b"cotton".to_vec(),
                percentage_bps: 5500,
            },
            CompositionLine {
                fibre: b"polyester".to_vec(),
                percentage_bps: 3000,
            },
            CompositionLine {
                fibre: b"elastane".to_vec(),
                percentage_bps: 500,
            },
            CompositionLine {
                fibre: b"viscose".to_vec(),
                percentage_bps: 1000,
            },
        ],
        composition_source: 0,
        certificate_fingerprints: vec![[0x11u8; 32], [0x22u8; 32]],
    };
    let record_bytes = record.encode();
    println!(
        "realistic passport record body measures {} bytes",
        record_bytes.len()
    );
    assert!(
        record_bytes.len() <= 1024,
        "a typical record must fit one kibibyte, measured {} bytes",
        record_bytes.len()
    );
    assert!(record_bytes.len() as u32 <= MaxRecordBodyLen::get());

    #[derive(Encode)]
    struct LifecycleEventV1 {
        // 0 = created, 1 = shipped, 2 = received, 3 = sold, 4 = end-of-life.
        event_type: u8,
        occurred_at_unix_ms: u64,
        gs1_event_hash: [u8; 32],
    }

    let event = LifecycleEventV1 {
        event_type: 1,
        occurred_at_unix_ms: 1_789_000_000_000,
        gs1_event_hash: [0x33u8; 32],
    };
    let event_bytes = event.encode();
    println!(
        "typical lifecycle event measures {} bytes",
        event_bytes.len()
    );
    assert!(
        event_bytes.len() <= 64,
        "a typical event must fit sixty-four bytes, measured {} bytes",
        event_bytes.len()
    );
    assert!(event_bytes.len() as u32 <= MaxEventLen::get());
}

/// `set_price` from a plain signed account — neither root nor any council backing at all — is
/// rejected with `DispatchError::BadOrigin`, and `PublicationPrice` is left untouched.
#[test]
fn set_price_from_foreign_account_is_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Dpp::set_price(RuntimeOrigin::signed(1), 9_000),
            DispatchError::BadOrigin
        );
        assert_eq!(PublicationPrice::<Test>::get(), 0);
    });
}

/// A council motion carrying less than three quarters of the vote (two of four) does not clear
/// `AdminOrigin`, so `set_price` is rejected with `DispatchError::BadOrigin` and
/// `PublicationPrice` is left untouched.
#[test]
fn set_price_from_council_below_three_quarters_is_rejected() {
    new_test_ext().execute_with(|| {
        let origin: RuntimeOrigin =
            pallet_collective::RawOrigin::<AccountId, CouncilCollective>::Members(2, 4).into();
        assert_noop!(Dpp::set_price(origin, 9_000), DispatchError::BadOrigin);
        assert_eq!(PublicationPrice::<Test>::get(), 0);
    });
}

/// A council motion carrying exactly three quarters of the vote (three of four) clears
/// `AdminOrigin`, and `PublicationPrice` carries the new value after the call returns.
#[test]
fn set_price_from_council_at_three_quarters_changes_stored_price() {
    new_test_ext().execute_with(|| {
        let origin: RuntimeOrigin =
            pallet_collective::RawOrigin::<AccountId, CouncilCollective>::Members(3, 4).into();
        assert_ok!(Dpp::set_price(origin, 9_000));
        assert_eq!(PublicationPrice::<Test>::get(), 9_000);
    });
}

/// A price of zero is accepted from a council motion at the threshold, and is what
/// `PublicationPrice` carries afterwards — this pallet treats zero as a deliberate choice
/// `set_price` allows, not a value it rejects. The price is first moved away from its default of
/// zero so the assertion cannot pass merely because storage started there.
#[test]
fn set_price_accepts_zero_price_from_council_at_three_quarters() {
    new_test_ext().execute_with(|| {
        let origin: RuntimeOrigin =
            pallet_collective::RawOrigin::<AccountId, CouncilCollective>::Members(3, 4).into();
        assert_ok!(Dpp::set_price(origin, 500));
        assert_eq!(PublicationPrice::<Test>::get(), 500);

        let origin: RuntimeOrigin =
            pallet_collective::RawOrigin::<AccountId, CouncilCollective>::Members(3, 4).into();
        assert_ok!(Dpp::set_price(origin, 0));
        assert_eq!(PublicationPrice::<Test>::get(), 0);
    });
}

/// [`InitializePublicationPrice`] seeds `PublicationPrice` with four thousand of the chain's
/// smallest unit on storage whose on-chain pallet version is still zero, and raises that version
/// to one. Running it again afterwards — simulating a later forkless upgrade that bundles it a
/// second time by mistake — is a no-op: neither the price nor the version changes, even though
/// the price was moved away from its seeded value in between, which is what proves the second
/// run touched nothing rather than merely reseeding the same number.
#[test]
fn migration_seeds_price_once_and_is_noop_on_already_migrated_storage() {
    new_test_ext().execute_with(|| {
        assert_eq!(StorageVersion::get::<Dpp>(), StorageVersion::new(0));
        assert_eq!(PublicationPrice::<Test>::get(), 0);

        InitializePublicationPrice::<Test>::on_runtime_upgrade();

        assert_eq!(PublicationPrice::<Test>::get(), 4_000);
        assert_eq!(StorageVersion::get::<Dpp>(), StorageVersion::new(1));

        PublicationPrice::<Test>::put(9_000u128);
        InitializePublicationPrice::<Test>::on_runtime_upgrade();

        assert_eq!(PublicationPrice::<Test>::get(), 9_000);
        assert_eq!(StorageVersion::get::<Dpp>(), StorageVersion::new(1));
    });
}

/// `publish_head`, when a block author is determined, charges `PublicationPrice` to the caller
/// and credits the exact same amount to the account `pallet_authorship::Pallet::author()` names
/// as the current block's author — the property this pallet's own documentation promises,
/// checked here by account balance rather than by the mechanism that produced it.
#[test]
fn publish_head_credits_publication_price_to_current_block_author() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        set_block_author(2);
        fund_account(1, 10_000);
        PublicationPrice::<Test>::put(4_000);

        let result = Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            b"https://id.gs1.org/01/09506000134352".to_vec(),
            schema_id,
            b"passport body".to_vec(),
            Vec::new(),
        );

        assert_eq!(result.expect("call must succeed").pays_fee, Pays::No);
        assert_eq!(pallet_balances::Pallet::<Test>::free_balance(1), 6_000);
        assert_eq!(
            pallet_balances::Pallet::<Test>::free_balance(2),
            4_000,
            "the current block author (account 2, set by set_block_author) must receive the charged price"
        );
        assert_eq!(pallet_authorship::Pallet::<Test>::author(), Some(2));
    });
}

/// `append_event`, when a block author is determined, charges `PublicationPrice` to the caller
/// and credits the exact same amount to the account `pallet_authorship::Pallet::author()` names
/// as the current block's author, exactly as `publish_head` does.
#[test]
fn append_event_credits_publication_price_to_current_block_author() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();
        fund_account(1, 10_000);
        // PublicationPrice defaults to zero, so this first call costs nothing and only exists
        // to give append_event a head record to attach to.
        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"first body".to_vec(),
            Vec::new(),
        ));

        set_block_author(3);
        PublicationPrice::<Test>::put(4_000);

        let result = Dpp::append_event(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id,
            b"shipped".to_vec(),
        );

        assert_eq!(result.expect("call must succeed").pays_fee, Pays::No);
        assert_eq!(pallet_balances::Pallet::<Test>::free_balance(1), 6_000);
        assert_eq!(
            pallet_balances::Pallet::<Test>::free_balance(3),
            4_000,
            "the current block author (account 3, set by set_block_author) must receive the charged price"
        );
        assert_eq!(pallet_authorship::Pallet::<Test>::author(), Some(3));
    });
}

/// `publish_head`, when no block author can be determined, still succeeds: it charges
/// `PublicationPrice` to the caller, burns the amount instead of crediting anyone (so total
/// issuance drops by the same amount), and the head record it publishes is created exactly as
/// it would be with a determined author — following the runtime's own standard fee handler
/// (`ToAuthor`), which burns under the same crash case rather than rejecting the call.
#[test]
fn publish_head_burns_price_and_reduces_total_issuance_when_block_author_undetermined() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();
        fund_account(1, 10_000);
        PublicationPrice::<Test>::put(4_000);
        // new_test_ext() already clears the block author; asserted so the burn below is known
        // to exercise the "no author" branch, not merely an unset fixture.
        assert_eq!(pallet_authorship::Pallet::<Test>::author(), None);
        let issuance_before = pallet_balances::Pallet::<Test>::total_issuance();

        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"passport body".to_vec(),
            Vec::new(),
        ));

        assert_eq!(pallet_balances::Pallet::<Test>::free_balance(1), 6_000);
        assert_eq!(
            pallet_balances::Pallet::<Test>::total_issuance(),
            issuance_before - 4_000
        );
        assert!(get_head(b"552100554", &gs1_id).is_some());
    });
}

/// A failed `publish_head` call — here, one referencing an unregistered file fingerprint —
/// returns post-dispatch information without the "no fee" flag, so the standard transaction fee
/// stays charged exactly as it would for any other failed extrinsic; `Pays::No` is reserved for
/// success and never returned alongside an error.
#[test]
fn failed_publish_head_does_not_return_pays_no() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();

        let result = Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            b"https://id.gs1.org/01/09506000134352".to_vec(),
            schema_id,
            b"passport body".to_vec(),
            vec![[9u8; 32]],
        );

        let err = result.expect_err("must fail: referenced fingerprint is not registered");
        assert_eq!(err.error, Error::<Test>::FileNotRegistered.into());
        assert_eq!(err.post_info.pays_fee, Pays::Yes);
    });
}

/// `publish_head`'s `Event::PublicationPriceCharged` names the passport's own key, the exact
/// amount charged, and the account that received it — checked field by field so an observer
/// reading only this event, without touching storage, can reconstruct the charge.
#[test]
fn publish_head_emits_publication_price_charged_event_with_key_amount_and_recipient() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let company: crate::CompanyRegistrationNumber<Test> =
            b"552100554".to_vec().try_into().unwrap();
        let gs1: crate::Gs1Id<Test> = b"https://id.gs1.org/01/09506000134352"
            .to_vec()
            .try_into()
            .unwrap();
        // frame_system never populates Events<T> at block zero ("don't populate events on
        // genesis"), so this must run on a nonzero block to observe anything at all.
        set_block_number(1);
        set_block_author(2);
        fund_account(1, 10_000);
        PublicationPrice::<Test>::put(4_000);

        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            b"https://id.gs1.org/01/09506000134352".to_vec(),
            schema_id,
            b"passport body".to_vec(),
            Vec::new(),
        ));

        let charged = System::events()
            .into_iter()
            .find_map(|record| match record.event {
                RuntimeEvent::Dpp(crate::Event::PublicationPriceCharged {
                    company_registration_number,
                    gs1_id,
                    amount,
                    recipient,
                }) => Some((company_registration_number, gs1_id, amount, recipient)),
                _ => None,
            })
            .expect("PublicationPriceCharged must have been emitted");

        assert_eq!(charged.0, company);
        assert_eq!(charged.1, gs1);
        assert_eq!(charged.2, 4_000);
        assert_eq!(charged.3, Some(2));
    });
}

/// `append_event`'s `Event::PublicationPriceCharged` names the passport's own key, the exact
/// amount charged, and the account that received it, exactly as `publish_head`'s own does.
#[test]
fn append_event_emits_publication_price_charged_event_with_key_amount_and_recipient() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id_bytes = b"https://id.gs1.org/01/09506000134352".to_vec();
        let company: crate::CompanyRegistrationNumber<Test> =
            b"552100554".to_vec().try_into().unwrap();
        let gs1: crate::Gs1Id<Test> = gs1_id_bytes.clone().try_into().unwrap();
        // frame_system never populates Events<T> at block zero ("don't populate events on
        // genesis"), so this must run on a nonzero block to observe anything at all.
        set_block_number(1);
        fund_account(1, 10_000);
        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id_bytes.clone(),
            schema_id,
            b"first body".to_vec(),
            Vec::new(),
        ));

        set_block_author(3);
        PublicationPrice::<Test>::put(4_000);

        assert_ok!(Dpp::append_event(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id_bytes,
            b"shipped".to_vec(),
        ));

        let charged = System::events()
            .into_iter()
            .rev()
            .find_map(|record| match record.event {
                RuntimeEvent::Dpp(crate::Event::PublicationPriceCharged {
                    company_registration_number,
                    gs1_id,
                    amount,
                    recipient,
                }) => Some((company_registration_number, gs1_id, amount, recipient)),
                _ => None,
            })
            .expect("PublicationPriceCharged must have been emitted");

        assert_eq!(charged.0, company);
        assert_eq!(charged.1, gs1);
        assert_eq!(charged.2, 4_000);
        assert_eq!(charged.3, Some(3));
    });
}

/// The mock's own `TransactionByteFee` — the length-fee coefficient
/// [`dispatch_through_fee_pipeline`] and every phase-C test below actually run against — equals
/// ten, the same literal `runtime/src/configs/mod.rs` computes as `10 * MICRO_UNIT` (`MICRO_UNIT`
/// is one smallest unit there). Named as a direct number rather than compared against an import
/// from the runtime crate, which this pallet cannot depend on without a cycle: a future edit that
/// changes one side of this constant without the other now fails this test instead of staying
/// invisible.
#[test]
fn mock_transaction_byte_fee_matches_runtime_literal() {
    assert_eq!(TransactionByteFee::get(), 10);
}

/// Publishes a fresh passport with a `body_len`-byte body, priced at `price`, through the real
/// fee pipeline (`tip` zero), and returns how much the publisher's own balance actually dropped
/// by. The call is asserted to succeed inside this helper — a caller only ever wants the amount
/// charged for a call that actually went through.
fn publish_head_net_charge(price: Balance, body_len: usize) -> Balance {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        fund_account(1, 1_000_000);
        PublicationPrice::<Test>::put(price);

        let call = RuntimeCall::Dpp(Call::<Test>::publish_head {
            company_registration_number: b"552100554".to_vec(),
            gs1_id: b"https://id.gs1.org/01/09506000134352".to_vec(),
            schema_id,
            body: vec![7u8; body_len],
            file_fingerprints: Vec::new(),
        });

        let balance_before = pallet_balances::Pallet::<Test>::free_balance(1);
        let (outcome, _standard_fee) = dispatch_through_fee_pipeline(1, 0, call);
        outcome
            .expect("transaction must validate")
            .expect("publish_head must succeed");
        let balance_after = pallet_balances::Pallet::<Test>::free_balance(1);

        balance_before - balance_after
    })
}

/// Through the real fee pipeline, `publish_head`'s total charge to the publisher — the standard
/// transaction fee, refunded in full on success because the call returns `Pays::No`, plus this
/// pallet's own fixed `PublicationPrice` — equals the price alone and does not depend on the
/// record body's length: a one-hundred-byte body and a body at the pallet's own ceiling
/// (`MaxRecordBodyLen`, four kibibytes in this mock) cost exactly the same.
#[test]
fn publish_head_total_charge_equals_price_and_is_independent_of_record_length() {
    let price = 4_000;

    assert_eq!(publish_head_net_charge(price, 100), price);
    assert_eq!(
        publish_head_net_charge(price, MaxRecordBodyLen::get() as usize),
        price
    );
}

/// At a `PublicationPrice` of zero and a transaction carrying no voluntary tip, a successful
/// `publish_head` through the real fee pipeline does not change the publisher's balance at all:
/// this pallet's own charge is zero, and the standard transaction fee is refunded in full because
/// the call returns `Pays::No`.
#[test]
fn publish_head_at_zero_price_and_no_tip_does_not_change_publisher_balance() {
    assert_eq!(publish_head_net_charge(0, 100), 0);
}

/// A `publish_head` call that fails — here, one referencing an unregistered file fingerprint —
/// leaves the standard transaction fee charged in full through the real fee pipeline:
/// `pallet-transaction-payment`'s extension withdraws it up front and, since a failed call's
/// `PostDispatchInfo` keeps `Pays::Yes`, never refunds any of it. The expected amount is the
/// standard fee `pallet-transaction-payment` itself computed for this call
/// (`dispatch_through_fee_pipeline`'s own return value, via `TransactionPayment::compute_fee`),
/// not a number duplicated by this test, so it can never silently drift from what the extension
/// actually charges.
#[test]
fn failed_publish_head_charges_the_standard_fee_in_full() {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        fund_account(1, 1_000_000);

        let call = RuntimeCall::Dpp(Call::<Test>::publish_head {
            company_registration_number: b"552100554".to_vec(),
            gs1_id: b"https://id.gs1.org/01/09506000134352".to_vec(),
            schema_id,
            body: b"passport body".to_vec(),
            file_fingerprints: vec![[9u8; 32]],
        });

        let balance_before = pallet_balances::Pallet::<Test>::free_balance(1);
        let (outcome, standard_fee) = dispatch_through_fee_pipeline(1, 0, call);
        assert!(
            standard_fee > 0,
            "the standard fee must be nonzero for this test to prove anything"
        );

        let dispatch_err = outcome
            .expect("transaction must validate")
            .expect_err("call must fail: referenced fingerprint is not registered");
        assert_eq!(dispatch_err.error, Error::<Test>::FileNotRegistered.into());

        let balance_after = pallet_balances::Pallet::<Test>::free_balance(1);
        assert_eq!(balance_before - balance_after, standard_fee);
    });
}

/// Appends an event of `event_len` bytes to a freshly published passport (published at price
/// zero, so the setup itself is free), then, priced at `price`, appends it through the real fee
/// pipeline (`tip` zero) and returns how much the publisher's own balance actually dropped by.
fn append_event_net_charge(price: Balance, event_len: usize) -> Balance {
    new_test_ext().execute_with(|| {
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();
        fund_account(1, 1_000_000);
        assert_ok!(Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"first body".to_vec(),
            Vec::new(),
        ));

        PublicationPrice::<Test>::put(price);
        let call = RuntimeCall::Dpp(Call::<Test>::append_event {
            company_registration_number: b"552100554".to_vec(),
            gs1_id,
            body: vec![7u8; event_len],
        });

        let balance_before = pallet_balances::Pallet::<Test>::free_balance(1);
        let (outcome, _standard_fee) = dispatch_through_fee_pipeline(1, 0, call);
        outcome
            .expect("transaction must validate")
            .expect("append_event must succeed");
        let balance_after = pallet_balances::Pallet::<Test>::free_balance(1);

        balance_before - balance_after
    })
}

/// Through the real fee pipeline, `append_event`'s total charge to the publisher equals
/// `PublicationPrice` alone, regardless of the event body's length (one byte against the
/// pallet's own ceiling, `MaxEventLen`), and equals exactly what `publish_head` charges for the
/// same price — both calls share the same charging mechanism, `Pallet::charge_publication_price`.
#[test]
fn append_event_total_charge_equals_price_independent_of_event_length_and_matches_publish_head() {
    let price = 4_000;
    let publish_head_charge = publish_head_net_charge(price, 100);

    assert_eq!(append_event_net_charge(price, 1), price);
    assert_eq!(
        append_event_net_charge(price, MaxEventLen::get() as usize),
        price
    );
    assert_eq!(append_event_net_charge(price, 1), publish_head_charge);
}

/// A `publish_head` call from a payer who cannot afford `PublicationPrice` is rejected with a
/// named error — `TokenError::FundsUnavailable`, the same error `fungible::Balanced::withdraw`
/// itself returns under `Precision::Exact` — and leaves every piece of storage this pallet could
/// have written exactly as it was before the call: no head record at the key, no row in the file
/// table, no row in the event table, and no event deposited at all (not `PublicationPriceCharged`
/// or anything else).
#[test]
fn publish_head_rejects_insufficient_funds_for_publication_price_and_leaves_storage_untouched() {
    new_test_ext().execute_with(|| {
        // frame_system never populates Events<T> at block zero, so this must run on a nonzero
        // block — otherwise "no event deposited" would hold trivially even if the pallet wrongly
        // deposited one.
        set_block_number(1);
        setup_project_with_permission(1, b"552100554");
        let schema_id = register_schema();
        let gs1_id = b"https://id.gs1.org/01/09506000134352".to_vec();
        let company: crate::CompanyRegistrationNumber<Test> =
            b"552100554".to_vec().try_into().unwrap();
        let gs1: crate::Gs1Id<Test> = gs1_id.clone().try_into().unwrap();
        PublicationPrice::<Test>::put(4_000);
        fund_account(1, 3_999);

        let events_before = System::events().len();

        let result = Dpp::publish_head(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            gs1_id.clone(),
            schema_id,
            b"passport body".to_vec(),
            Vec::new(),
        );

        let err = result.expect_err("must fail: payer cannot afford the publication price");
        assert_eq!(
            err.error,
            DispatchError::Token(TokenError::FundsUnavailable)
        );

        assert!(get_head(b"552100554", &gs1_id).is_none());
        assert_eq!(Files::<Test>::iter().count(), 0);
        assert_eq!(Events::<Test>::iter().count(), 0);
        assert_eq!(EventCounts::<Test>::get(&company, &gs1), 0);
        assert_eq!(
            System::events().len(),
            events_before,
            "no event, including PublicationPriceCharged, must be deposited on this failure"
        );
    });
}
