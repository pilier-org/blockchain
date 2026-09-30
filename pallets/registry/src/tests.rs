use crate::{
    Error, Event, Projects, RegistryAccess, RegistryEntries, Schemas, StorageEndpoints, mock::*,
};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::{DispatchError, traits::Hash};

/// A real GS1 Digital Link carrying a product number, a batch/lot and a serial
/// number must fit inside the configured 128-byte bound.
#[test]
fn gs1_digital_link_with_batch_and_serial_fits_configured_bound() {
    new_test_ext().execute_with(|| {
        let gs1_digital_link =
            b"https://id.gs1.org/01/09506000134352/10/LOT-2026-04-XY/21/00019283746501".to_vec();
        let _bounded: crate::Gs1Id<Test> = gs1_digital_link.try_into().expect(
            "a real GS1 Digital Link with batch and serial number must fit the configured bound",
        );
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

/// A writer list longer than the configured 32-account bound is rejected, and the
/// project's stored writer list is left untouched.
#[test]
fn writer_list_exceeding_configured_bound_is_rejected() {
    new_test_ext().execute_with(|| {
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));

        let too_many_writers: Vec<u64> = (0..33).collect();
        assert_noop!(
            Registry::set_project_writers(RuntimeOrigin::root(), 0, too_many_writers),
            Error::<Test>::TooManyWriters
        );

        let exactly_the_bound: Vec<u64> = (0..32).collect();
        assert_ok!(Registry::set_project_writers(
            RuntimeOrigin::root(),
            0,
            exactly_the_bound
        ));
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
        assert_eq!(Registry::writer_project(&1, b"552100554"), Some(0));
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

/// `RegistryAccess::is_project_member` answers "yes" for a project's owner and for an account
/// added to its writer list, "no" for an outsider who is neither, and "no" for a project
/// identifier that does not exist.
#[test]
fn is_project_member_answers_owner_writer_outsider_and_unknown_project() {
    new_test_ext().execute_with(|| {
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        assert_ok!(Registry::set_project_writers(
            RuntimeOrigin::root(),
            0,
            vec![2],
        ));

        assert!(<Registry as RegistryAccess<u64>>::is_project_member(0, &1));
        assert!(<Registry as RegistryAccess<u64>>::is_project_member(0, &2));
        assert!(!<Registry as RegistryAccess<u64>>::is_project_member(0, &3));
        assert!(!<Registry as RegistryAccess<u64>>::is_project_member(
            999, &1
        ));
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
            0,
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
        let description =
            br#"{"category":"textile","version":1,"fields":["gtin","batch","serial"]}"#.to_vec();

        assert_ok!(Registry::register_schema(
            RuntimeOrigin::root(),
            0,
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

        let new_address =
            b"https://storage.pilier.net/dpp/evidence/fr/siren/552100554/v2/".to_vec();
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
        let project_writers: crate::ProjectWriters<Test> = vec![2].try_into().unwrap();
        System::assert_last_event(
            Event::ProjectWritersUpdated {
                project_id: 0,
                writers: project_writers,
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

        // 8. register_schema -> SchemaRegistered { schema_id, category, version, fingerprint, project }
        let description = b"{\"fields\":[]}".to_vec();
        assert_ok!(Registry::register_schema(
            RuntimeOrigin::root(),
            0,
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
                project: 0,
            }
            .into(),
        );

        // 9. create_storage_endpoint -> StorageEndpointCreated { endpoint_id, company_registration_number, address }
        let address_bytes = b"https://storage.pilier.net/dpp/evidence/fr/siren/552100554/".to_vec();
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

/// `ensure_owner_or_admin` admits the admin origin (root, in this mock) and the named project's
/// own owner: the two callers allowed to create a project's registries and schemas.
#[test]
fn ensure_owner_or_admin_admits_root_and_owner() {
    new_test_ext().execute_with(|| {
        // Project 0 owned by account 1.
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));

        assert_ok!(Registry::ensure_owner_or_admin(RuntimeOrigin::root(), 0));
        assert_ok!(Registry::ensure_owner_or_admin(RuntimeOrigin::signed(1), 0));
    });
}

/// `ensure_owner_or_admin` rejects a project's writer, an unrelated account, and a signed origin
/// naming a project that does not exist: a writer is not the owner, so it may not create the
/// project's registries or schemas.
#[test]
fn ensure_owner_or_admin_rejects_writer_outsider_and_unknown_project() {
    new_test_ext().execute_with(|| {
        // Project 0 owned by account 1, with account 2 added as a writer.
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        assert_ok!(Registry::set_project_writers(
            RuntimeOrigin::root(),
            0,
            vec![2]
        ));

        // A writer is not the owner.
        assert_noop!(
            Registry::ensure_owner_or_admin(RuntimeOrigin::signed(2), 0),
            Error::<Test>::NotProjectOwner
        );
        // An unrelated account is not the owner.
        assert_noop!(
            Registry::ensure_owner_or_admin(RuntimeOrigin::signed(3), 0),
            Error::<Test>::NotProjectOwner
        );
        // A signed caller naming a project that does not exist.
        assert_noop!(
            Registry::ensure_owner_or_admin(RuntimeOrigin::signed(1), 99),
            Error::<Test>::ProjectNotFound
        );
    });
}

/// A project's own owner creates a code registry naming their own project as its writer.
#[test]
fn project_owner_creates_registry_for_own_project() {
    new_test_ext().execute_with(|| {
        // Project 0 owned by account 1.
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));

        assert_ok!(Registry::create_registry_type(
            RuntimeOrigin::signed(1),
            b"Textile fibres per Regulation 1007/2011".to_vec(),
            0
        ));
    });
}

/// A project's owner cannot create a code registry that names a different project as its writer.
#[test]
fn project_owner_cannot_create_registry_for_another_project() {
    new_test_ext().execute_with(|| {
        // Project 0 owned by account 1, project 1 owned by account 2.
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 2));

        // Account 1 owns project 0, not project 1.
        assert_noop!(
            Registry::create_registry_type(RuntimeOrigin::signed(1), b"reg".to_vec(), 1),
            Error::<Test>::NotProjectOwner
        );
    });
}

/// A project's writer, who is not its owner, cannot create a code registry for the project.
#[test]
fn project_writer_cannot_create_registry() {
    new_test_ext().execute_with(|| {
        // Project 0 owned by account 1, with account 2 added as a writer.
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        assert_ok!(Registry::set_project_writers(
            RuntimeOrigin::root(),
            0,
            vec![2]
        ));

        assert_noop!(
            Registry::create_registry_type(RuntimeOrigin::signed(2), b"reg".to_vec(), 0),
            Error::<Test>::NotProjectOwner
        );
    });
}

/// The admin origin (root) still creates a code registry for any project.
#[test]
fn root_creates_registry_for_any_project() {
    new_test_ext().execute_with(|| {
        // Project 0 owned by account 1.
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));

        assert_ok!(Registry::create_registry_type(
            RuntimeOrigin::root(),
            b"reg".to_vec(),
            0
        ));
    });
}

/// The project owner and the admin origin (root) may mark an entry of that project's registry
/// deprecated; a different project's owner and a writer of the project may not.
#[test]
fn deprecate_registry_entry_owner_and_root_pass_writer_and_foreign_owner_rejected() {
    new_test_ext().execute_with(|| {
        // Project 0 owned by account 1, project 1 owned by account 2, account 3 a writer on
        // project 0.
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 2));
        assert_ok!(Registry::set_project_writers(
            RuntimeOrigin::root(),
            0,
            vec![3]
        ));

        // Registry 0 is writable by project 0; add three entries as its owner.
        assert_ok!(Registry::create_registry_type(
            RuntimeOrigin::signed(1),
            b"reg".to_vec(),
            0
        ));
        assert_ok!(Registry::add_registry_entry(
            RuntimeOrigin::signed(1),
            0,
            b"a".to_vec()
        ));
        assert_ok!(Registry::add_registry_entry(
            RuntimeOrigin::signed(1),
            0,
            b"b".to_vec()
        ));

        // A different project's owner cannot deprecate an entry of project 0's registry.
        assert_noop!(
            Registry::deprecate_registry_entry(RuntimeOrigin::signed(2), 0, 0),
            Error::<Test>::NotProjectOwner
        );
        // A writer of project 0 (account 3) is not its owner.
        assert_noop!(
            Registry::deprecate_registry_entry(RuntimeOrigin::signed(3), 0, 0),
            Error::<Test>::NotProjectOwner
        );
        // The project's owner may.
        assert_ok!(Registry::deprecate_registry_entry(
            RuntimeOrigin::signed(1),
            0,
            0
        ));
        // Root may, for any project's registry.
        assert_ok!(Registry::deprecate_registry_entry(
            RuntimeOrigin::root(),
            0,
            1
        ));
    });
}

/// A project's owner registers a schema for their own project; the stored schema and the emitted
/// event both carry that project's identifier.
#[test]
fn project_owner_registers_schema_for_own_project() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Project 0 owned by account 1.
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));

        assert_ok!(Registry::register_schema(
            RuntimeOrigin::signed(1),
            0,
            7,
            1,
            b"{}".to_vec()
        ));

        let schema = Schemas::<Test>::get(0).expect("schema must be stored");
        assert_eq!(schema.project, Some(0));

        let fingerprint = <Test as frame_system::Config>::Hashing::hash(b"{}");
        System::assert_last_event(
            Event::SchemaRegistered {
                schema_id: 0,
                category: 7,
                version: 1,
                fingerprint,
                project: 0,
            }
            .into(),
        );
    });
}

/// A different project's owner, a writer of the project, and an unrelated account may not
/// register a schema for the project: only its owner (or the admin origin) may.
#[test]
fn schema_registration_rejected_for_foreign_owner_writer_and_outsider() {
    new_test_ext().execute_with(|| {
        // Project 0 owned by account 1, project 1 owned by account 2, account 3 a writer on
        // project 0.
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 2));
        assert_ok!(Registry::set_project_writers(
            RuntimeOrigin::root(),
            0,
            vec![3]
        ));

        // A different project's owner.
        assert_noop!(
            Registry::register_schema(RuntimeOrigin::signed(2), 0, 1, 1, b"{}".to_vec()),
            Error::<Test>::NotProjectOwner
        );
        // A writer of project 0, who is not its owner.
        assert_noop!(
            Registry::register_schema(RuntimeOrigin::signed(3), 0, 1, 1, b"{}".to_vec()),
            Error::<Test>::NotProjectOwner
        );
        // An unrelated account.
        assert_noop!(
            Registry::register_schema(RuntimeOrigin::signed(9), 0, 1, 1, b"{}".to_vec()),
            Error::<Test>::NotProjectOwner
        );
    });
}

/// The admin origin (root) still registers a schema for any project.
#[test]
fn root_registers_schema_for_any_project() {
    new_test_ext().execute_with(|| {
        // Project 0 owned by account 1.
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));

        assert_ok!(Registry::register_schema(
            RuntimeOrigin::root(),
            0,
            1,
            1,
            b"{}".to_vec()
        ));
        assert_eq!(Schemas::<Test>::get(0).unwrap().project, Some(0));
    });
}

/// The v1 migration reads a schema stored in the old four-field format and writes it back with
/// `project: None`, leaving its other fields untouched; a second run changes nothing.
#[test]
fn migration_v1_adds_project_none_and_is_idempotent() {
    use crate::migrations::v1::AddSchemaProject;
    use frame_support::traits::OnRuntimeUpgrade;

    new_test_ext().execute_with(|| {
        // Write one schema in the old four-field format (no `project`), the way it sits on a
        // chain upgraded from before this field existed.
        let raw = b"{\"fields\":[\"gtin\"]}".to_vec();
        let description: crate::SchemaDescription<Test> = raw.clone().try_into().unwrap();
        let fingerprint = <Test as frame_system::Config>::Hashing::hash(&raw);
        let old_value: (u32, u32, crate::SchemaDescription<Test>, sp_core::H256) =
            (7, 3, description.clone(), fingerprint);
        let key = Schemas::<Test>::hashed_key_for(0);
        frame_support::storage::unhashed::put(&key, &old_value);

        // The migration turns it into the new format: project == None, other fields unchanged.
        let _ = AddSchemaProject::<Test>::on_runtime_upgrade();
        let migrated = Schemas::<Test>::get(0).expect("schema must remain in storage");
        assert_eq!(migrated.category, 7);
        assert_eq!(migrated.version, 3);
        assert_eq!(migrated.description, description);
        assert_eq!(migrated.fingerprint, fingerprint);
        assert_eq!(migrated.project, None);

        // Running the migration again changes nothing: the storage-version guard returns early.
        let _ = AddSchemaProject::<Test>::on_runtime_upgrade();
        let after = Schemas::<Test>::get(0).expect("schema must remain in storage");
        assert_eq!(after, migrated);
    });
}

/// The full ownership handover: the owner names a successor, and ownership changes only once the
/// successor accepts. The transfer event names both the old and the new owner.
#[test]
fn project_ownership_transfers_only_after_the_successor_accepts() {
    new_test_ext().execute_with(|| {
        System::set_block_number(1);
        // Project 0 owned by account 1.
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));

        // Naming a successor does not change ownership yet.
        assert_ok!(Registry::propose_project_owner(
            RuntimeOrigin::signed(1),
            0,
            2
        ));
        assert_eq!(Projects::<Test>::get(0).unwrap().owner, 1);

        // The successor accepts; ownership passes to account 2 and the pending record clears.
        assert_ok!(Registry::accept_project_ownership(
            RuntimeOrigin::signed(2),
            0
        ));
        assert_eq!(Projects::<Test>::get(0).unwrap().owner, 2);
        assert!(crate::PendingProjectOwner::<Test>::get(0).is_none());
        System::assert_last_event(
            Event::ProjectOwnershipTransferred {
                project_id: 0,
                old_owner: 1,
                new_owner: 2,
            }
            .into(),
        );
    });
}

/// An account that was not named as the successor cannot accept a project's ownership.
#[test]
fn ownership_acceptance_rejected_for_account_not_named_successor() {
    new_test_ext().execute_with(|| {
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        assert_ok!(Registry::propose_project_owner(
            RuntimeOrigin::signed(1),
            0,
            2
        ));

        // Account 3 was not named.
        assert_noop!(
            Registry::accept_project_ownership(RuntimeOrigin::signed(3), 0),
            Error::<Test>::NotProposedOwner
        );
    });
}

/// Accepting ownership with no proposal outstanding is rejected.
#[test]
fn ownership_acceptance_rejected_when_no_proposal_outstanding() {
    new_test_ext().execute_with(|| {
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));

        assert_noop!(
            Registry::accept_project_ownership(RuntimeOrigin::signed(2), 0),
            Error::<Test>::NoPendingOwnership
        );
    });
}

/// A second proposal replaces the first named successor: only the latest-named account may accept.
#[test]
fn second_ownership_proposal_replaces_the_named_successor() {
    new_test_ext().execute_with(|| {
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        assert_ok!(Registry::propose_project_owner(
            RuntimeOrigin::signed(1),
            0,
            2
        ));
        assert_ok!(Registry::propose_project_owner(
            RuntimeOrigin::signed(1),
            0,
            3
        ));

        // The first-named successor can no longer accept.
        assert_noop!(
            Registry::accept_project_ownership(RuntimeOrigin::signed(2), 0),
            Error::<Test>::NotProposedOwner
        );
        // The latest-named successor can.
        assert_ok!(Registry::accept_project_ownership(
            RuntimeOrigin::signed(3),
            0
        ));
        assert_eq!(Projects::<Test>::get(0).unwrap().owner, 3);
    });
}

/// The admin origin (root) may name a successor on the owner's behalf.
#[test]
fn root_may_propose_a_successor() {
    new_test_ext().execute_with(|| {
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));

        assert_ok!(Registry::propose_project_owner(RuntimeOrigin::root(), 0, 2));
        assert_ok!(Registry::accept_project_ownership(
            RuntimeOrigin::signed(2),
            0
        ));
        assert_eq!(Projects::<Test>::get(0).unwrap().owner, 2);
    });
}

/// After a handover the previous owner, who is not on the writer list, can no longer create the
/// project's registries, while the new owner can.
#[test]
fn after_handover_previous_owner_loses_registry_rights_and_new_owner_gains_them() {
    new_test_ext().execute_with(|| {
        assert_ok!(Registry::create_project(RuntimeOrigin::root(), 1));
        assert_ok!(Registry::propose_project_owner(
            RuntimeOrigin::signed(1),
            0,
            2
        ));
        assert_ok!(Registry::accept_project_ownership(
            RuntimeOrigin::signed(2),
            0
        ));

        // The previous owner (account 1) is not a writer and no longer the owner.
        assert_noop!(
            Registry::create_registry_type(RuntimeOrigin::signed(1), b"reg".to_vec(), 0),
            Error::<Test>::NotProjectOwner
        );
        // The new owner (account 2) can.
        assert_ok!(Registry::create_registry_type(
            RuntimeOrigin::signed(2),
            b"reg".to_vec(),
            0
        ));
    });
}
