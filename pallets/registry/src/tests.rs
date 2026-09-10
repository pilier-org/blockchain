use crate::{Error, Event, RegistryAccess, RegistryEntries, Schemas, StorageEndpoints, mock::*};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::{DispatchError, traits::Hash};

/// A real GS1 Digital Link carrying a product number, a batch/lot and a serial
/// number must fit inside the configured 128-byte bound.
#[test]
fn gs1_digital_link_with_batch_and_serial_fits_configured_bound() {
    new_test_ext().execute_with(|| {
        let gs1_digital_link =
            b"https://id.gs1.org/01/09506000134352/10/LOT-2026-04-XY/21/00019283746501".to_vec();
        let _bounded: crate::Gs1Id<Test> = gs1_digital_link
            .try_into()
            .expect("a real GS1 Digital Link with batch and serial number must fit the configured bound");
    });
}

/// A real storage endpoint address must fit inside the configured 256-byte bound.
#[test]
fn storage_endpoint_address_fits_configured_bound() {
    new_test_ext().execute_with(|| {
        let address = b"https://storage.pilier.net/dpp/evidence/fr/siren/552100554/".to_vec();
        let _bounded: crate::StorageEndpointAddress<Test> = address
            .try_into()
            .expect("a real storage endpoint address must fit the configured bound");
    });
}

/// An account belonging to a project that was never granted a given company
/// registration number cannot write under it: `RegistryAccess::writer_project` returns `None`.
#[test]
fn foreign_registration_number_cannot_be_used() {
    new_test_ext().execute_with(|| {
        // Project 0 owned by account 1, project 1 owned by account 2.
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 2));
        assert_ok!(Registry::grant_registration_number(
            RuntimeOrigin::root(),
            0,
            b"552100554".to_vec()
        ));

        // Account 1 (project 0's owner) may write under it...
        assert_eq!(
            Registry::writer_project(&1, b"552100554"),
            Some(0)
        );
        // ...but account 2 (a different project's owner) may not.
        assert_eq!(Registry::writer_project(&2, b"552100554"), None);
    });
}

/// Revoking a registration number's grant stops the permission immediately.
#[test]
fn revoked_permission_stops_immediately() {
    new_test_ext().execute_with(|| {
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        assert_ok!(Registry::grant_registration_number(
            RuntimeOrigin::root(),
            0,
            b"552100554".to_vec()
        ));
        assert_eq!(Registry::writer_project(&1, b"552100554"), Some(0));

        assert_ok!(Registry::revoke_registration_number(
            RuntimeOrigin::root(),
            b"552100554".to_vec()
        ));
        assert_eq!(Registry::writer_project(&1, b"552100554"), None);
    });
}

/// An administrative call from an account that is not `T::AdminOrigin` is rejected.
#[test]
fn admin_call_from_outsider_is_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Registry::create_project(RuntimeOrigin::signed(1), 1),
            DispatchError::BadOrigin
        );
        assert_noop!(
            Registry::set_project_writers(RuntimeOrigin::signed(1), 0, vec![2]),
            DispatchError::BadOrigin
        );
        assert_noop!(
            Registry::grant_registration_number(RuntimeOrigin::signed(1), 0, b"552100554".to_vec()),
            DispatchError::BadOrigin
        );
        assert_noop!(
            Registry::revoke_registration_number(RuntimeOrigin::signed(1), b"552100554".to_vec()),
            DispatchError::BadOrigin
        );
    });
}

/// An administrative call from the root origin succeeds.
#[test]
fn admin_call_from_root_origin_succeeds() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        System::assert_last_event(
            Event::ProjectCreated {
                project_id: 0,
                owner: 1,
            }
            .into(),
        );
    });
}

/// A project that does not exist cannot be granted a registration number or have
/// its writers changed.
#[test]
fn unknown_project_is_rejected() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            Registry::set_project_writers(RuntimeOrigin::root(), 0, vec![1]),
            Error::<Test>::ProjectNotFound
        );
        assert_noop!(
            Registry::grant_registration_number(RuntimeOrigin::root(), 0, b"552100554".to_vec()),
            Error::<Test>::ProjectNotFound
        );
    });
}

/// Adding an entry to a registry from an account that is not a member of the
/// project that registry names as its writer is rejected.
#[test]
fn registry_entry_rejected_when_caller_not_writer() {
    new_test_ext().execute_with(|| {
        // Project 0 (owner 1) is the registry's writer; project 1 (owner 2) is not.
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 2));
        assert_ok!(Registry::create_registry_type(
            RuntimeOrigin::root(),
            b"ISO 3166-1 country codes".to_vec(),
            0
        ));

        assert_noop!(
            Registry::add_registry_entry(RuntimeOrigin::signed(2), 0, b"FR".to_vec()),
            Error::<Test>::NotRegistryTypeWriter
        );
        // The registry's own writer succeeds.
        assert_ok!(Registry::add_registry_entry(
            RuntimeOrigin::signed(1),
            0,
            b"FR".to_vec()
        ));
    });
}

/// A registry entry marked deprecated is never removed: it remains readable, value
/// unchanged, with the deprecated flag set.
#[test]
fn deprecated_registry_entry_remains_readable() {
    new_test_ext().execute_with(|| {
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        assert_ok!(Registry::create_registry_type(
            RuntimeOrigin::root(),
            b"ISO 3166-1 country codes".to_vec(),
            0
        ));
        assert_ok!(Registry::add_registry_entry(
            RuntimeOrigin::signed(1),
            0,
            b"FR".to_vec()
        ));

        assert_ok!(Registry::deprecate_registry_entry(
            RuntimeOrigin::root(),
            0,
            0
        ));

        let entry = RegistryEntries::<Test>::get(0, 0).expect("entry must remain in storage");
        assert_eq!(entry.value.into_inner(), b"FR".to_vec());
        assert!(entry.deprecated);
    });
}

/// A schema that was never registered does not exist: this is the check a passport
/// pallet would use to reject a write under an unknown schema, and it must answer `false` for a
/// schema identifier nobody registered.
#[test]
fn nonexistent_schema_is_rejected() {
    new_test_ext().execute_with(|| {
        assert!(!Registry::schema_exists(0));

        assert_ok!(Registry::register_schema(
            RuntimeOrigin::root(),
            1,
            1,
            b"{\"fields\":[]}".to_vec()
        ));

        assert!(Registry::schema_exists(0));
        assert!(!Registry::schema_exists(1));
    });
}

/// A schema's description is read back from storage byte for byte identical to what
/// was written, and its fingerprint matches the description actually stored.
#[test]
fn schema_description_round_trips_with_matching_fingerprint() {
    new_test_ext().execute_with(|| {
        let description = br#"{"category":"textile","version":1,"fields":["gtin","batch","serial"]}"#.to_vec();

        assert_ok!(Registry::register_schema(
            RuntimeOrigin::root(),
            7,
            1,
            description.clone()
        ));

        let schema = Schemas::<Test>::get(0).expect("schema must be in storage");
        assert_eq!(schema.description.into_inner(), description);
        assert_eq!(
            schema.fingerprint,
            <Test as frame_system::Config>::Hashing::hash(&description)
        );
    });
}

/// An account outside the project that currently holds the right to write under a
/// storage endpoint's company registration number cannot edit that endpoint's address.
#[test]
fn foreign_storage_endpoint_cannot_be_edited() {
    new_test_ext().execute_with(|| {
        // Project 0 (owner 1) holds the registration number the endpoint was created under;
        // project 1 (owner 2) does not.
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 2));
        assert_ok!(Registry::grant_registration_number(
            RuntimeOrigin::root(),
            0,
            b"552100554".to_vec()
        ));
        assert_ok!(Registry::create_storage_endpoint(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            b"https://storage.pilier.net/dpp/evidence/fr/siren/552100554/".to_vec()
        ));

        assert_noop!(
            Registry::update_storage_endpoint_address(
                RuntimeOrigin::signed(2),
                0,
                b"https://evil.example/redirect/".to_vec()
            ),
            Error::<Test>::NoPermissionForRegistrationNumber
        );
    });
}

/// Replacing a storage endpoint's address does not change its identifier.
#[test]
fn endpoint_id_unchanged_after_address_replacement() {
    new_test_ext().execute_with(|| {
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        assert_ok!(Registry::grant_registration_number(
            RuntimeOrigin::root(),
            0,
            b"552100554".to_vec()
        ));
        assert_ok!(Registry::create_storage_endpoint(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            b"https://storage.pilier.net/dpp/evidence/fr/siren/552100554/".to_vec()
        ));

        let new_address = b"https://storage.pilier.net/dpp/evidence/fr/siren/552100554/v2/".to_vec();
        assert_ok!(Registry::update_storage_endpoint_address(
            RuntimeOrigin::signed(1),
            0,
            new_address.clone()
        ));

        // Same identifier (0), new address; no second endpoint was created.
        let endpoint = StorageEndpoints::<Test>::get(0).expect("endpoint must remain at id 0");
        assert_eq!(endpoint.address.into_inner(), new_address);
        assert!(StorageEndpoints::<Test>::get(1).is_none());
    });
}

/// Every mutating call in this pallet deposits an event that carries the identifier
/// of the entity it changed together with the new value of the field that changed, so an
/// observer can reconstruct the change without reading storage. This test walks through all ten
/// mutating calls in a plausible order and checks each one's event immediately after the call
/// that must have deposited it.
#[test]
fn event_composition_for_every_mutating_call() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);

        // 1. create_project -> ProjectCreated { project_id, owner }
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        System::assert_last_event(
            Event::ProjectCreated {
                project_id: 0,
                owner: 1,
            }
            .into(),
        );

        // 2. set_project_writers -> ProjectWritersUpdated { project_id, writers }
        assert_ok!(Registry::set_project_writers(
            RuntimeOrigin::root(),
            0,
            vec![2]
        ));
        System::assert_last_event(
            Event::ProjectWritersUpdated {
                project_id: 0,
                writers: vec![2],
            }
            .into(),
        );

        // 3. grant_registration_number -> RegistrationNumberGranted { company_registration_number, project_id }
        assert_ok!(Registry::grant_registration_number(
            RuntimeOrigin::root(),
            0,
            b"552100554".to_vec()
        ));
        let granted_number: crate::CompanyRegistrationNumber<Test> =
            b"552100554".to_vec().try_into().unwrap();
        System::assert_last_event(
            Event::RegistrationNumberGranted {
                company_registration_number: granted_number.clone(),
                project_id: 0,
            }
            .into(),
        );

        // 4. revoke_registration_number -> RegistrationNumberRevoked { company_registration_number }
        assert_ok!(Registry::revoke_registration_number(
            RuntimeOrigin::root(),
            b"552100554".to_vec()
        ));
        System::assert_last_event(
            Event::RegistrationNumberRevoked {
                company_registration_number: granted_number,
            }
            .into(),
        );
        // Re-grant it so the storage-endpoint calls below have a permission to work with.
        assert_ok!(Registry::grant_registration_number(
            RuntimeOrigin::root(),
            0,
            b"552100554".to_vec()
        ));

        // 5. create_registry_type -> RegistryTypeCreated { registry_type_id, name, writer_project }
        assert_ok!(Registry::create_registry_type(
            RuntimeOrigin::root(),
            b"ISO 3166-1 country codes".to_vec(),
            0
        ));
        let registry_name: crate::RegistryTypeName<Test> =
            b"ISO 3166-1 country codes".to_vec().try_into().unwrap();
        System::assert_last_event(
            Event::RegistryTypeCreated {
                registry_type_id: 0,
                name: registry_name,
                writer_project: 0,
            }
            .into(),
        );

        // 6. add_registry_entry -> RegistryEntryAdded { registry_type_id, entry_id, value }
        assert_ok!(Registry::add_registry_entry(
            RuntimeOrigin::signed(1),
            0,
            b"FR".to_vec()
        ));
        let entry_value: crate::RegistryEntryValue<Test> = b"FR".to_vec().try_into().unwrap();
        System::assert_last_event(
            Event::RegistryEntryAdded {
                registry_type_id: 0,
                entry_id: 0,
                value: entry_value,
            }
            .into(),
        );

        // 7. deprecate_registry_entry -> RegistryEntryDeprecated { registry_type_id, entry_id, deprecated }
        assert_ok!(Registry::deprecate_registry_entry(
            RuntimeOrigin::root(),
            0,
            0
        ));
        System::assert_last_event(
            Event::RegistryEntryDeprecated {
                registry_type_id: 0,
                entry_id: 0,
                deprecated: true,
            }
            .into(),
        );

        // 8. register_schema -> SchemaRegistered { schema_id, category, version, fingerprint }
        let description = b"{\"fields\":[]}".to_vec();
        assert_ok!(Registry::register_schema(
            RuntimeOrigin::root(),
            1,
            1,
            description.clone()
        ));
        let fingerprint = <Test as frame_system::Config>::Hashing::hash(&description);
        System::assert_last_event(
            Event::SchemaRegistered {
                schema_id: 0,
                category: 1,
                version: 1,
                fingerprint,
            }
            .into(),
        );

        // 9. create_storage_endpoint -> StorageEndpointCreated { endpoint_id, company_registration_number, address }
        let address_bytes =
            b"https://storage.pilier.net/dpp/evidence/fr/siren/552100554/".to_vec();
        assert_ok!(Registry::create_storage_endpoint(
            RuntimeOrigin::signed(1),
            b"552100554".to_vec(),
            address_bytes.clone()
        ));
        let company_registration_number: crate::CompanyRegistrationNumber<Test> =
            b"552100554".to_vec().try_into().unwrap();
        let address: crate::StorageEndpointAddress<Test> = address_bytes.try_into().unwrap();
        System::assert_last_event(
            Event::StorageEndpointCreated {
                endpoint_id: 0,
                company_registration_number,
                address,
            }
            .into(),
        );

        // 10. update_storage_endpoint_address -> StorageEndpointAddressUpdated { endpoint_id, address }
        let new_address_bytes =
            b"https://storage.pilier.net/dpp/evidence/fr/siren/552100554/v2/".to_vec();
        assert_ok!(Registry::update_storage_endpoint_address(
            RuntimeOrigin::signed(1),
            0,
            new_address_bytes.clone()
        ));
        let new_address: crate::StorageEndpointAddress<Test> =
            new_address_bytes.try_into().unwrap();
        System::assert_last_event(
            Event::StorageEndpointAddressUpdated {
                endpoint_id: 0,
                address: new_address,
            }
            .into(),
        );
    });
}
