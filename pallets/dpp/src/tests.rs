use crate::{Error, EventRecord, Files, mock::*};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::Hash;

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
