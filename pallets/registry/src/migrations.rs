//! Storage migrations for the registry pallet.

/// Migration from storage version 0 to 1: give every schema already in storage the `project`
/// field that [`crate::SchemaInfo`] gained when a project's owner became able to register schemas.
pub mod v1 {
    use crate::{Config, Pallet, SchemaDescription, SchemaInfo, Schemas};
    use core::marker::PhantomData;
    use frame_support::{
        traits::{Get, GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
        weights::Weight,
    };

    #[cfg(feature = "try-runtime")]
    use alloc::vec::Vec;
    #[cfg(feature = "try-runtime")]
    use codec::{Decode, Encode};

    /// The old on-chain value of a schema is a struct of four fields — `category`, `version`,
    /// `description`, `fingerprint` — and SCALE encodes it exactly as a tuple of the same four
    /// types in the same order. Decoding through this tuple avoids declaring a second copy of the
    /// old struct just to read it once. The new [`SchemaInfo`] appends `project` after these four.
    type OldSchema<T> = (
        u32,
        u32,
        SchemaDescription<T>,
        <T as frame_system::Config>::Hash,
    );

    /// Reads every schema in the old four-field format and writes it back with `project: None`,
    /// then raises the pallet's on-chain storage version to 1. Running again is a no-op: the
    /// version guard returns early once it reads 1.
    pub struct AddSchemaProject<T>(PhantomData<T>);

    impl<T: Config> OnRuntimeUpgrade for AddSchemaProject<T> {
        fn on_runtime_upgrade() -> Weight {
            if Pallet::<T>::on_chain_storage_version() >= StorageVersion::new(1) {
                return T::DbWeight::get().reads(1);
            }

            let mut count: u64 = 0;
            Schemas::<T>::translate::<OldSchema<T>, _>(
                |_schema_id, (category, version, description, fingerprint)| {
                    count = count.saturating_add(1);
                    Some(SchemaInfo {
                        category,
                        version,
                        description,
                        fingerprint,
                        project: None,
                    })
                },
            );

            StorageVersion::new(1).put::<Pallet<T>>();
            // One read and one write per migrated schema, plus the storage-version read and write.
            T::DbWeight::get().reads_writes(count.saturating_add(1), count.saturating_add(1))
        }

        #[cfg(feature = "try-runtime")]
        fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
            // Count keys only: the values are still in the old format and would fail to decode as
            // the new `SchemaInfo`.
            let count = Schemas::<T>::iter_keys().count() as u32;
            Ok(count.encode())
        }

        #[cfg(feature = "try-runtime")]
        fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
            let expected = <u32 as Decode>::decode(&mut &state[..])
                .map_err(|_| "AddSchemaProject: could not decode the pre-upgrade schema count")?;

            let mut actual: u32 = 0;
            for (_schema_id, schema) in Schemas::<T>::iter() {
                actual = actual.saturating_add(1);
                frame_support::ensure!(
                    schema.project.is_none(),
                    "AddSchemaProject: a migrated schema must carry project == None"
                );
            }

            frame_support::ensure!(
                expected == actual,
                "AddSchemaProject: the number of schemas changed during migration"
            );
            frame_support::ensure!(
                Pallet::<T>::on_chain_storage_version() == StorageVersion::new(1),
                "AddSchemaProject: on-chain storage version must be 1 after migration"
            );
            Ok(())
        }
    }
}
