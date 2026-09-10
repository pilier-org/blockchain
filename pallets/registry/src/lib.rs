//! # Pilier Registry Pallet
//!
//! Holds the on-chain registry a digital product passport is read against: projects and their
//! write rights, code registries (a registry of registries), schemas, and storage endpoints. A
//! passport pallet consults this pallet through the [`RegistryAccess`] trait on every
//! publication; it never reads this pallet's storage directly.
//!
//! This pallet is deliberately separate from the passport pallet it serves: a passport is
//! written thousands of times a day by a project, while a project, a code registry, a schema or
//! a storage endpoint changes on the order of months and is administered by the validators'
//! council. Mixing the two behind one set of storages would lock the rare administrative change
//! behind the same logic as the high-volume publication path.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

// Re-export pallet items so they can be accessed from the crate namespace.
pub use pallet::*;

/// A project's on-chain identifier. Fixed-width, assigned in order by the pallet when a project
/// is created; never reused.
pub type ProjectId = u32;

/// A code registry's on-chain identifier ("a registry of registries" — the pallet does not know
/// in advance which registries will exist, only how to create and address one). Fixed-width,
/// assigned in order by the pallet when a registry type is created.
pub type RegistryTypeId = u32;

/// The identifier of one entry inside a code registry. Fixed-width, assigned in order by the
/// pallet, starting at zero for each registry type independently.
pub type RegistryEntryId = u32;

/// A schema's on-chain identifier. Fixed-width, assigned in order by the pallet when a schema is
/// registered.
pub type SchemaId = u32;

/// A storage endpoint's on-chain identifier. Fixed-width, assigned in order by the pallet when a
/// storage endpoint is created.
pub type StorageEndpointId = u32;

/// Consulted by the passport pallet on every publication, so it never reads this pallet's
/// storage directly. It answers exactly the three questions a passport write depends on: may
/// this account write under this company registration number, does this schema exist, does this
/// storage endpoint exist.
pub trait RegistryAccess<AccountId> {
    /// Returns the project that grants `who` the right to write under
    /// `company_registration_number` — its owner or one of its writers — or `None` if no
    /// project grants `who` that right.
    fn writer_project(who: &AccountId, company_registration_number: &[u8]) -> Option<ProjectId>;

    /// Returns whether a schema with `schema_id` is registered.
    fn schema_exists(schema_id: SchemaId) -> bool;

    /// Returns whether a storage endpoint with `endpoint_id` exists.
    fn storage_endpoint_exists(endpoint_id: StorageEndpointId) -> bool;
}

/// Weight functions needed for this pallet's dispatchables.
///
/// Every function here is a hand-written placeholder, not a generated (benchmarked) weight —
/// each is marked `TEMPORARY WEIGHT` in its own line so the weight-measurement work that
/// eventually replaces them can find every one by that string. `()` is a valid implementation,
/// used in this pallet's own unit tests; a runtime that includes this pallet supplies its own
/// generated implementation once `runtime-benchmarks` support is added for it.
pub trait WeightInfo {
    /// Weight for [`Pallet::create_project`].
    fn create_project() -> frame_support::pallet_prelude::Weight;
    /// Weight for [`Pallet::set_project_writers`].
    fn set_project_writers() -> frame_support::pallet_prelude::Weight;
    /// Weight for [`Pallet::grant_registration_number`].
    fn grant_registration_number() -> frame_support::pallet_prelude::Weight;
    /// Weight for [`Pallet::revoke_registration_number`].
    fn revoke_registration_number() -> frame_support::pallet_prelude::Weight;
    /// Weight for [`Pallet::create_registry_type`].
    fn create_registry_type() -> frame_support::pallet_prelude::Weight;
    /// Weight for [`Pallet::add_registry_entry`].
    fn add_registry_entry() -> frame_support::pallet_prelude::Weight;
    /// Weight for [`Pallet::deprecate_registry_entry`].
    fn deprecate_registry_entry() -> frame_support::pallet_prelude::Weight;
    /// Weight for [`Pallet::register_schema`].
    fn register_schema() -> frame_support::pallet_prelude::Weight;
    /// Weight for [`Pallet::create_storage_endpoint`].
    fn create_storage_endpoint() -> frame_support::pallet_prelude::Weight;
    /// Weight for [`Pallet::update_storage_endpoint_address`].
    fn update_storage_endpoint_address() -> frame_support::pallet_prelude::Weight;
}

impl WeightInfo for () {
    fn create_project() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0) // TEMPORARY WEIGHT
    }
    fn set_project_writers() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0) // TEMPORARY WEIGHT
    }
    fn grant_registration_number() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0) // TEMPORARY WEIGHT
    }
    fn revoke_registration_number() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0) // TEMPORARY WEIGHT
    }
    fn create_registry_type() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0) // TEMPORARY WEIGHT
    }
    fn add_registry_entry() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0) // TEMPORARY WEIGHT
    }
    fn deprecate_registry_entry() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0) // TEMPORARY WEIGHT
    }
    fn register_schema() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0) // TEMPORARY WEIGHT
    }
    fn create_storage_endpoint() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0) // TEMPORARY WEIGHT
    }
    fn update_storage_endpoint_address() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0) // TEMPORARY WEIGHT
    }
}

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use super::{
        ProjectId, RegistryEntryId, RegistryTypeId, SchemaId, StorageEndpointId, Vec, WeightInfo,
    };
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Hash;

    /// The pallet's placeholder struct, used to implement traits, methods and dispatchables.
    ///
    /// `without_storage_info` is required because [`ProjectInfo::writers`] stores a plain,
    /// unbounded `Vec` rather than a `BoundedVec` — the same trade-off `pallet-validator-set`
    /// already makes in this codebase for its own admin/council-controlled account list, and
    /// for the same reason: there is no hard cap on how many accounts a project designates as
    /// writers, and this pallet is not used with storage-info-driven tooling.
    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// The pallet's configuration trait.
    ///
    /// The six length bounds below are named as parameters, taken from measured figures
    /// rather than guessed, precisely so the limit is not a number an implementer happens to pick: a chain
    /// running with a bound too small would reject real data on a live chain, where the type
    /// cannot be changed without a runtime upgrade.
    #[pallet::config]
    pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
        /// The origin allowed to call this pallet's administrative extrinsics. In the runtime
        /// this pallet is eventually wired into, this will be "a council supermajority, or root
        /// as an emergency lever" — the same design `pallet-validator-set` already uses in this
        /// codebase for the mutable validator set. This pallet's own `Config` only requires some
        /// origin to be checked; wiring it to the council is a runtime-integration concern
        /// outside this pallet's own scope.
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

        /// Upper bound, in bytes, on a company registration number (for France, a SIREN or
        /// SIRET).
        #[pallet::constant]
        type MaxCompanyRegistrationNumberLen: Get<u32>;

        /// Upper bound, in bytes, on a GS1 identifier written as a full GS1 Digital Link (a
        /// product number under application identifier 01, a batch/lot under 10, a serial number
        /// under 21 — as many parts as the passport's own level of detail needs). No storage in
        /// this pallet holds a value of this type yet; it is declared here as a configuration
        /// parameter, alongside the other five, so that the passport pallet this registry serves
        /// inherits the same deliberately chosen limit instead of picking its own.
        #[pallet::constant]
        type MaxGs1IdLen: Get<u32>;

        /// Upper bound, in bytes, on a code registry's name.
        #[pallet::constant]
        type MaxRegistryTypeNameLen: Get<u32>;

        /// Upper bound, in bytes, on one code registry entry's value.
        #[pallet::constant]
        type MaxRegistryEntryValueLen: Get<u32>;

        /// Upper bound, in bytes, on a storage endpoint's common address part.
        #[pallet::constant]
        type MaxStorageEndpointAddressLen: Get<u32>;

        /// Upper bound, in bytes, on a schema's description, stored on chain in full.
        #[pallet::constant]
        type MaxSchemaDescriptionLen: Get<u32>;

        /// Weight information for this pallet's dispatchables.
        type WeightInfo: super::WeightInfo;
    }

    /// A company registration number (for France, a SIREN or SIRET), bounded by
    /// `T::MaxCompanyRegistrationNumberLen`.
    pub type CompanyRegistrationNumber<T> =
        BoundedVec<u8, <T as Config>::MaxCompanyRegistrationNumberLen>;

    /// A GS1 identifier written as a full GS1 Digital Link, bounded by `T::MaxGs1IdLen`. See
    /// that constant's own documentation for why this type is declared even though no storage in
    /// this pallet uses it yet.
    pub type Gs1Id<T> = BoundedVec<u8, <T as Config>::MaxGs1IdLen>;

    /// A code registry's name, bounded by `T::MaxRegistryTypeNameLen`.
    pub type RegistryTypeName<T> = BoundedVec<u8, <T as Config>::MaxRegistryTypeNameLen>;

    /// One code registry entry's value, bounded by `T::MaxRegistryEntryValueLen`.
    pub type RegistryEntryValue<T> = BoundedVec<u8, <T as Config>::MaxRegistryEntryValueLen>;

    /// A storage endpoint's common address part, bounded by `T::MaxStorageEndpointAddressLen`.
    pub type StorageEndpointAddress<T> =
        BoundedVec<u8, <T as Config>::MaxStorageEndpointAddressLen>;

    /// A schema's description, stored on chain in full, bounded by `T::MaxSchemaDescriptionLen`.
    pub type SchemaDescription<T> = BoundedVec<u8, <T as Config>::MaxSchemaDescriptionLen>;

    /// A project: an owner account and the accounts allowed to write on the project's behalf
    /// (add code registry entries, create and update storage endpoints). The owner is always
    /// implicitly a writer; `writers` holds the accounts added on top of the owner.
    #[derive(
        CloneNoBound, PartialEqNoBound, EqNoBound, Encode, Decode, TypeInfo, RuntimeDebugNoBound,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct ProjectInfo<T: Config> {
        /// The account that owns the project.
        pub owner: T::AccountId,
        /// Accounts, besides the owner, allowed to write on the project's behalf.
        pub writers: Vec<T::AccountId>,
    }

    /// The next `ProjectId` to assign. Starts at zero and is incremented on every
    /// [`Pallet::create_project`] call; identifiers are never reused.
    #[pallet::storage]
    pub type NextProjectId<T: Config> = StorageValue<_, ProjectId, ValueQuery>;

    /// Every project the pallet knows about, keyed by its identifier.
    #[pallet::storage]
    pub type Projects<T: Config> = StorageMap<_, Blake2_128Concat, ProjectId, ProjectInfo<T>>;

    /// Which project, if any, currently holds the right to write under a given company
    /// registration number. A registration number maps to at most one project at a time:
    /// granting it to a project is exclusive, and revoking it removes the entry entirely, so the
    /// permission check this pallet exposes through [`super::RegistryAccess`] stops applying the
    /// instant a revocation lands.
    #[pallet::storage]
    pub type CompanyPermissions<T: Config> =
        StorageMap<_, Blake2_128Concat, CompanyRegistrationNumber<T>, ProjectId>;

    /// A code registry: a short-number-to-string lookup table, plus which project is allowed to
    /// add entries to it and how many numbers it has issued so far.
    #[derive(
        CloneNoBound, PartialEqNoBound, EqNoBound, Encode, Decode, TypeInfo, RuntimeDebugNoBound,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct RegistryTypeInfo<T: Config> {
        /// The registry's name (for example, "ISO 3166-1 country codes").
        pub name: RegistryTypeName<T>,
        /// The project allowed to add entries to this registry.
        pub writer_project: ProjectId,
        /// How many entry numbers this registry has issued so far; the next
        /// [`Pallet::add_registry_entry`] call is assigned this number.
        pub next_entry_id: RegistryEntryId,
    }

    /// One entry inside a code registry: the string a short number stands for, and whether it
    /// has been marked deprecated. An entry is never deleted — see [`Pallet::deprecate_registry_entry`].
    #[derive(
        CloneNoBound, PartialEqNoBound, EqNoBound, Encode, Decode, TypeInfo, RuntimeDebugNoBound,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct RegistryEntryInfo<T: Config> {
        /// The string this entry's short number stands for.
        pub value: RegistryEntryValue<T>,
        /// Whether this entry has been marked deprecated. A deprecated entry remains readable —
        /// it is never removed — so a passport that already carries its number still resolves.
        pub deprecated: bool,
    }

    /// Every code registry the pallet knows about, keyed by its identifier.
    #[pallet::storage]
    pub type RegistryTypes<T: Config> =
        StorageMap<_, Blake2_128Concat, RegistryTypeId, RegistryTypeInfo<T>>;

    /// The next `RegistryTypeId` to assign. Starts at zero and is incremented on every
    /// [`Pallet::create_registry_type`] call; identifiers are never reused.
    #[pallet::storage]
    pub type NextRegistryTypeId<T: Config> = StorageValue<_, RegistryTypeId, ValueQuery>;

    /// Every code registry entry the pallet knows about, keyed by its registry's identifier and
    /// its own entry number.
    #[pallet::storage]
    pub type RegistryEntries<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        RegistryTypeId,
        Blake2_128Concat,
        RegistryEntryId,
        RegistryEntryInfo<T>,
    >;

    /// A schema: the on-chain shape a passport's data is parsed against, kept in full so a
    /// passport is readable without the company that wrote it still existing.
    /// `fingerprint` is only an integrity check — it lets someone who already has the
    /// description confirm it was not altered — never the sole record of the schema.
    #[derive(
        CloneNoBound, PartialEqNoBound, EqNoBound, Encode, Decode, TypeInfo, RuntimeDebugNoBound,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct SchemaInfo<T: Config> {
        /// The product category this schema describes (an application-defined classifier
        /// number; this pallet does not interpret it further).
        pub category: u32,
        /// The schema's version number.
        pub version: u32,
        /// The schema's description, stored on chain in full.
        pub description: SchemaDescription<T>,
        /// The hash of `description`, computed with the chain's own hashing algorithm at
        /// registration time.
        pub fingerprint: T::Hash,
    }

    /// Every schema the pallet knows about, keyed by its identifier.
    #[pallet::storage]
    pub type Schemas<T: Config> = StorageMap<_, Blake2_128Concat, SchemaId, SchemaInfo<T>>;

    /// The next `SchemaId` to assign. Starts at zero and is incremented on every
    /// [`Pallet::register_schema`] call; identifiers are never reused.
    #[pallet::storage]
    pub type NextSchemaId<T: Config> = StorageValue<_, SchemaId, ValueQuery>;

    /// A storage endpoint: the common part of the address where a company's evidence files are
    /// found, addressed by the company registration number that currently holds the right to
    /// write under it. Only the latest address is kept here — a passport's proof is the file's
    /// own fingerprint, not this address, so this pallet accepts the trade-off that a company
    /// whose registration number's grant has since moved on leaves its past addresses only in
    /// this pallet's event log, which a node with pruned history does not keep.
    #[derive(
        CloneNoBound, PartialEqNoBound, EqNoBound, Encode, Decode, TypeInfo, RuntimeDebugNoBound,
    )]
    #[scale_info(skip_type_params(T))]
    pub struct StorageEndpointInfo<T: Config> {
        /// The company registration number this endpoint was created under.
        pub company_registration_number: CompanyRegistrationNumber<T>,
        /// The endpoint's common address part.
        pub address: StorageEndpointAddress<T>,
    }

    /// Every storage endpoint the pallet knows about, keyed by its identifier.
    #[pallet::storage]
    pub type StorageEndpoints<T: Config> =
        StorageMap<_, Blake2_128Concat, StorageEndpointId, StorageEndpointInfo<T>>;

    /// The next `StorageEndpointId` to assign. Starts at zero and is incremented on every
    /// [`Pallet::create_storage_endpoint`] call; identifiers are never reused.
    #[pallet::storage]
    pub type NextStorageEndpointId<T: Config> = StorageValue<_, StorageEndpointId, ValueQuery>;

    /// Events that functions in this pallet can emit. Every event names the identifier of the
    /// entity it changed together with the new value of the field that changed, so an observer
    /// can reconstruct the change without reading storage.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A project was created.
        ProjectCreated {
            project_id: ProjectId,
            owner: T::AccountId,
        },
        /// A project's writer list was replaced.
        ProjectWritersUpdated {
            project_id: ProjectId,
            writers: Vec<T::AccountId>,
        },
        /// A project was granted the right to write under a company registration number.
        RegistrationNumberGranted {
            company_registration_number: CompanyRegistrationNumber<T>,
            project_id: ProjectId,
        },
        /// A company registration number's grant was revoked.
        RegistrationNumberRevoked {
            company_registration_number: CompanyRegistrationNumber<T>,
        },
        /// A code registry was created.
        RegistryTypeCreated {
            registry_type_id: RegistryTypeId,
            name: RegistryTypeName<T>,
            writer_project: ProjectId,
        },
        /// An entry was added to a code registry.
        RegistryEntryAdded {
            registry_type_id: RegistryTypeId,
            entry_id: RegistryEntryId,
            value: RegistryEntryValue<T>,
        },
        /// A code registry entry was marked deprecated.
        RegistryEntryDeprecated {
            registry_type_id: RegistryTypeId,
            entry_id: RegistryEntryId,
            deprecated: bool,
        },
        /// A schema was registered.
        SchemaRegistered {
            schema_id: SchemaId,
            category: u32,
            version: u32,
            fingerprint: T::Hash,
        },
        /// A storage endpoint was created.
        StorageEndpointCreated {
            endpoint_id: StorageEndpointId,
            company_registration_number: CompanyRegistrationNumber<T>,
            address: StorageEndpointAddress<T>,
        },
        /// A storage endpoint's address was replaced. Only the new address is kept in storage;
        /// see [`StorageEndpointInfo`]'s own documentation for the accepted trade-off.
        StorageEndpointAddressUpdated {
            endpoint_id: StorageEndpointId,
            address: StorageEndpointAddress<T>,
        },
    }

    /// Errors that can be returned by this pallet.
    #[pallet::error]
    pub enum Error<T> {
        /// No project exists with the given identifier.
        ProjectNotFound,
        /// The company registration number does not fit `T::MaxCompanyRegistrationNumberLen`.
        RegistrationNumberTooLong,
        /// The company registration number is already granted to a project.
        RegistrationNumberAlreadyGranted,
        /// The company registration number is not currently granted to any project.
        RegistrationNumberNotGranted,
        /// The registry name does not fit `T::MaxRegistryTypeNameLen`.
        RegistryTypeNameTooLong,
        /// No code registry exists with the given identifier.
        RegistryTypeNotFound,
        /// The calling account is not a member of the project allowed to write to this registry.
        NotRegistryTypeWriter,
        /// The registry entry value does not fit `T::MaxRegistryEntryValueLen`.
        RegistryEntryValueTooLong,
        /// No entry exists with the given registry and entry identifier.
        RegistryEntryNotFound,
        /// The schema description does not fit `T::MaxSchemaDescriptionLen`.
        SchemaDescriptionTooLong,
        /// The storage endpoint address does not fit `T::MaxStorageEndpointAddressLen`.
        StorageEndpointAddressTooLong,
        /// The calling account's project does not currently hold the right to write under this
        /// company registration number.
        NoPermissionForRegistrationNumber,
        /// No storage endpoint exists with the given identifier.
        StorageEndpointNotFound,
    }

    /// The pallet's dispatchable functions ("calls").
    ///
    /// The weights below are temporary hand-written placeholders, not the pallet's final
    /// `WeightInfo`. They are replaced with measured benchmarks once every call this pallet
    /// needs exists.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Create a new project with `owner` as its owner and no writers beyond the owner.
        ///
        /// Must be called by `T::AdminOrigin`.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::create_project())]
        pub fn create_project(origin: OriginFor<T>, owner: T::AccountId) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            let project_id = NextProjectId::<T>::get();
            NextProjectId::<T>::put(project_id.saturating_add(1));
            Projects::<T>::insert(
                project_id,
                ProjectInfo {
                    owner: owner.clone(),
                    writers: Vec::new(),
                },
            );

            Self::deposit_event(Event::ProjectCreated { project_id, owner });
            Ok(())
        }

        /// Replace a project's writer list with `writers`. The owner remains an implicit writer
        /// regardless of what this list contains.
        ///
        /// Must be called by `T::AdminOrigin`. Fails with [`Error::ProjectNotFound`] if
        /// `project_id` does not name an existing project.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::set_project_writers())]
        pub fn set_project_writers(
            origin: OriginFor<T>,
            project_id: ProjectId,
            writers: Vec<T::AccountId>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            Projects::<T>::try_mutate(project_id, |maybe_project| -> DispatchResult {
                let project = maybe_project.as_mut().ok_or(Error::<T>::ProjectNotFound)?;
                project.writers = writers.clone();
                Ok(())
            })?;

            Self::deposit_event(Event::ProjectWritersUpdated {
                project_id,
                writers,
            });
            Ok(())
        }

        /// Grant `project_id` the right to write under `company_registration_number`.
        ///
        /// Must be called by `T::AdminOrigin`. Fails with [`Error::ProjectNotFound`] if
        /// `project_id` does not name an existing project, and with
        /// [`Error::RegistrationNumberAlreadyGranted`] if the registration number is already
        /// granted to a project (revoke it first to move it to a different project).
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::grant_registration_number())]
        pub fn grant_registration_number(
            origin: OriginFor<T>,
            project_id: ProjectId,
            company_registration_number: Vec<u8>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            ensure!(
                Projects::<T>::contains_key(project_id),
                Error::<T>::ProjectNotFound
            );

            let company_registration_number: CompanyRegistrationNumber<T> =
                company_registration_number
                    .try_into()
                    .map_err(|_| Error::<T>::RegistrationNumberTooLong)?;
            ensure!(
                !CompanyPermissions::<T>::contains_key(&company_registration_number),
                Error::<T>::RegistrationNumberAlreadyGranted
            );

            CompanyPermissions::<T>::insert(&company_registration_number, project_id);
            Self::deposit_event(Event::RegistrationNumberGranted {
                company_registration_number,
                project_id,
            });
            Ok(())
        }

        /// Revoke whichever project currently holds the right to write under
        /// `company_registration_number`. The permission check this pallet exposes through
        /// [`super::RegistryAccess`] stops applying to it immediately: this call does not mark
        /// the grant as revoked, it removes it.
        ///
        /// Must be called by `T::AdminOrigin`. Fails with
        /// [`Error::RegistrationNumberNotGranted`] if the registration number is not currently
        /// granted to any project.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::revoke_registration_number())]
        pub fn revoke_registration_number(
            origin: OriginFor<T>,
            company_registration_number: Vec<u8>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            let company_registration_number: CompanyRegistrationNumber<T> =
                company_registration_number
                    .try_into()
                    .map_err(|_| Error::<T>::RegistrationNumberTooLong)?;
            ensure!(
                CompanyPermissions::<T>::contains_key(&company_registration_number),
                Error::<T>::RegistrationNumberNotGranted
            );

            CompanyPermissions::<T>::remove(&company_registration_number);
            Self::deposit_event(Event::RegistrationNumberRevoked {
                company_registration_number,
            });
            Ok(())
        }

        /// Create a new code registry named `name`, writable by `writer_project`.
        ///
        /// Must be called by `T::AdminOrigin`. Fails with [`Error::ProjectNotFound`] if
        /// `writer_project` does not name an existing project.
        #[pallet::call_index(4)]
        #[pallet::weight(T::WeightInfo::create_registry_type())]
        pub fn create_registry_type(
            origin: OriginFor<T>,
            name: Vec<u8>,
            writer_project: ProjectId,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            ensure!(
                Projects::<T>::contains_key(writer_project),
                Error::<T>::ProjectNotFound
            );

            let name: RegistryTypeName<T> = name
                .try_into()
                .map_err(|_| Error::<T>::RegistryTypeNameTooLong)?;

            let registry_type_id = NextRegistryTypeId::<T>::get();
            NextRegistryTypeId::<T>::put(registry_type_id.saturating_add(1));
            RegistryTypes::<T>::insert(
                registry_type_id,
                RegistryTypeInfo {
                    name: name.clone(),
                    writer_project,
                    next_entry_id: 0,
                },
            );

            Self::deposit_event(Event::RegistryTypeCreated {
                registry_type_id,
                name,
                writer_project,
            });
            Ok(())
        }

        /// Add an entry carrying `value` to registry `registry_type_id`. The entry's number is
        /// issued by the pallet in order, starting at zero for each registry independently.
        ///
        /// The caller must be a member — owner or writer — of the project that registry names as
        /// its writer. Fails with [`Error::RegistryTypeNotFound`] if `registry_type_id` does not
        /// name an existing registry, and with [`Error::NotRegistryTypeWriter`] if the caller is
        /// not a member of the project allowed to write to it.
        #[pallet::call_index(5)]
        #[pallet::weight(T::WeightInfo::add_registry_entry())]
        pub fn add_registry_entry(
            origin: OriginFor<T>,
            registry_type_id: RegistryTypeId,
            value: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let registry_type = RegistryTypes::<T>::get(registry_type_id)
                .ok_or(Error::<T>::RegistryTypeNotFound)?;
            ensure!(
                Self::is_project_member(registry_type.writer_project, &who),
                Error::<T>::NotRegistryTypeWriter
            );

            let value: RegistryEntryValue<T> = value
                .try_into()
                .map_err(|_| Error::<T>::RegistryEntryValueTooLong)?;

            let entry_id = registry_type.next_entry_id;
            RegistryTypes::<T>::try_mutate(
                registry_type_id,
                |maybe_registry_type| -> DispatchResult {
                    let registry_type = maybe_registry_type
                        .as_mut()
                        .ok_or(Error::<T>::RegistryTypeNotFound)?;
                    registry_type.next_entry_id = entry_id.saturating_add(1);
                    Ok(())
                },
            )?;
            RegistryEntries::<T>::insert(
                registry_type_id,
                entry_id,
                RegistryEntryInfo {
                    value: value.clone(),
                    deprecated: false,
                },
            );

            Self::deposit_event(Event::RegistryEntryAdded {
                registry_type_id,
                entry_id,
                value,
            });
            Ok(())
        }

        /// Mark a code registry entry deprecated, without removing it: a passport that already
        /// carries its number still resolves.
        ///
        /// Must be called by `T::AdminOrigin`. Fails with [`Error::RegistryEntryNotFound`] if no
        /// entry exists with the given registry and entry identifier.
        #[pallet::call_index(6)]
        #[pallet::weight(T::WeightInfo::deprecate_registry_entry())]
        pub fn deprecate_registry_entry(
            origin: OriginFor<T>,
            registry_type_id: RegistryTypeId,
            entry_id: RegistryEntryId,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            RegistryEntries::<T>::try_mutate(
                registry_type_id,
                entry_id,
                |maybe_entry| -> DispatchResult {
                    let entry = maybe_entry
                        .as_mut()
                        .ok_or(Error::<T>::RegistryEntryNotFound)?;
                    entry.deprecated = true;
                    Ok(())
                },
            )?;

            Self::deposit_event(Event::RegistryEntryDeprecated {
                registry_type_id,
                entry_id,
                deprecated: true,
            });
            Ok(())
        }

        /// Register a schema for product `category`, version `version`, with `description`
        /// stored on chain in full. `fingerprint` is computed by the pallet from `description`
        /// at registration time, so it can never disagree with what is stored.
        ///
        /// Must be called by `T::AdminOrigin`. Fails with [`Error::SchemaDescriptionTooLong`] if
        /// `description` does not fit `T::MaxSchemaDescriptionLen`.
        #[pallet::call_index(7)]
        #[pallet::weight(T::WeightInfo::register_schema())]
        pub fn register_schema(
            origin: OriginFor<T>,
            category: u32,
            version: u32,
            description: Vec<u8>,
        ) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;

            let description: SchemaDescription<T> = description
                .try_into()
                .map_err(|_| Error::<T>::SchemaDescriptionTooLong)?;
            let fingerprint = T::Hashing::hash(&description);

            let schema_id = NextSchemaId::<T>::get();
            NextSchemaId::<T>::put(schema_id.saturating_add(1));
            Schemas::<T>::insert(
                schema_id,
                SchemaInfo {
                    category,
                    version,
                    description,
                    fingerprint,
                },
            );

            Self::deposit_event(Event::SchemaRegistered {
                schema_id,
                category,
                version,
                fingerprint,
            });
            Ok(())
        }

        /// Create a storage endpoint at `address` under `company_registration_number`.
        ///
        /// The caller must be a member of the project that currently holds the right to write
        /// under `company_registration_number`. Fails with
        /// [`Error::NoPermissionForRegistrationNumber`] if no project holds that right, or the
        /// caller is not one of its members.
        #[pallet::call_index(8)]
        #[pallet::weight(T::WeightInfo::create_storage_endpoint())]
        pub fn create_storage_endpoint(
            origin: OriginFor<T>,
            company_registration_number: Vec<u8>,
            address: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let company_registration_number: CompanyRegistrationNumber<T> =
                company_registration_number
                    .try_into()
                    .map_err(|_| Error::<T>::RegistrationNumberTooLong)?;
            let project_id = CompanyPermissions::<T>::get(&company_registration_number)
                .ok_or(Error::<T>::NoPermissionForRegistrationNumber)?;
            ensure!(
                Self::is_project_member(project_id, &who),
                Error::<T>::NoPermissionForRegistrationNumber
            );

            let address: StorageEndpointAddress<T> = address
                .try_into()
                .map_err(|_| Error::<T>::StorageEndpointAddressTooLong)?;

            let endpoint_id = NextStorageEndpointId::<T>::get();
            NextStorageEndpointId::<T>::put(endpoint_id.saturating_add(1));
            StorageEndpoints::<T>::insert(
                endpoint_id,
                StorageEndpointInfo {
                    company_registration_number: company_registration_number.clone(),
                    address: address.clone(),
                },
            );

            Self::deposit_event(Event::StorageEndpointCreated {
                endpoint_id,
                company_registration_number,
                address,
            });
            Ok(())
        }

        /// Replace storage endpoint `endpoint_id`'s address with `address`. The endpoint's own
        /// identifier never changes.
        ///
        /// The caller must be a member of the project that currently holds the right to write
        /// under the endpoint's company registration number. Fails with
        /// [`Error::StorageEndpointNotFound`] if `endpoint_id` does not name an existing
        /// endpoint, and with [`Error::NoPermissionForRegistrationNumber`] if no project
        /// currently holds that right, or the caller is not one of its members.
        #[pallet::call_index(9)]
        #[pallet::weight(T::WeightInfo::update_storage_endpoint_address())]
        pub fn update_storage_endpoint_address(
            origin: OriginFor<T>,
            endpoint_id: StorageEndpointId,
            address: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let endpoint = StorageEndpoints::<T>::get(endpoint_id)
                .ok_or(Error::<T>::StorageEndpointNotFound)?;
            let project_id = CompanyPermissions::<T>::get(&endpoint.company_registration_number)
                .ok_or(Error::<T>::NoPermissionForRegistrationNumber)?;
            ensure!(
                Self::is_project_member(project_id, &who),
                Error::<T>::NoPermissionForRegistrationNumber
            );

            let address: StorageEndpointAddress<T> = address
                .try_into()
                .map_err(|_| Error::<T>::StorageEndpointAddressTooLong)?;

            StorageEndpoints::<T>::try_mutate(endpoint_id, |maybe_endpoint| -> DispatchResult {
                let endpoint = maybe_endpoint
                    .as_mut()
                    .ok_or(Error::<T>::StorageEndpointNotFound)?;
                endpoint.address = address.clone();
                Ok(())
            })?;

            Self::deposit_event(Event::StorageEndpointAddressUpdated {
                endpoint_id,
                address,
            });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Returns whether `who` is a member of `project_id` — its owner or one of its writers.
        /// Returns `false` if no project exists with that identifier.
        pub fn is_project_member(project_id: ProjectId, who: &T::AccountId) -> bool {
            Projects::<T>::get(project_id)
                .is_some_and(|project| &project.owner == who || project.writers.contains(who))
        }
    }

    impl<T: Config> super::RegistryAccess<T::AccountId> for Pallet<T> {
        fn writer_project(
            who: &T::AccountId,
            company_registration_number: &[u8],
        ) -> Option<ProjectId> {
            let company_registration_number: CompanyRegistrationNumber<T> =
                company_registration_number.to_vec().try_into().ok()?;
            let project_id = CompanyPermissions::<T>::get(&company_registration_number)?;
            Self::is_project_member(project_id, who).then_some(project_id)
        }

        fn schema_exists(schema_id: SchemaId) -> bool {
            Schemas::<T>::contains_key(schema_id)
        }

        fn storage_endpoint_exists(endpoint_id: StorageEndpointId) -> bool {
            StorageEndpoints::<T>::contains_key(endpoint_id)
        }
    }
}
