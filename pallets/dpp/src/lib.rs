//! # Pilier Digital Product Passport Pallet
//!
//! Holds the digital product passports themselves: a deduplicated table of evidence files, a
//! head record per passport that can be replaced by a later version, and an append-only table
//! of lifecycle events. A passport's body is stored as an opaque byte string — this pallet
//! never parses it. What gives the body meaning is the schema number carried alongside it,
//! registered in `pallet-pilier-registry` and read there, not reimplemented here.
//!
//! This pallet asks `pallet-pilier-registry`, through the [`RegistryAccess`] trait, for every
//! permission and existence check a write depends on: who may write under a company
//! registration number, whether a schema exists, whether a storage endpoint exists. It never
//! reads that pallet's storage directly.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

// Re-export pallet items so they can be accessed from the crate namespace.
pub use pallet::*;
pub use pallet_pilier_registry::{ProjectId, RegistryAccess, SchemaId, StorageEndpointId};

/// A file's content fingerprint: thirty-two bytes, computed by the caller from the file's
/// content and never by this pallet. The file table is keyed by this value, so registering the
/// same content twice — from the same or a different passport — deduplicates to one entry.
pub type FileFingerprint = [u8; 32];

/// The position of one lifecycle event inside one passport's append-only event table. Assigned
/// by the pallet in order, starting at zero, and never reused or reassigned.
pub type EventIndex = u32;

/// Weight functions needed for this pallet's dispatchables.
///
/// Every function here is a hand-written placeholder, not a generated (benchmarked) weight —
/// each is marked `TEMPORARY WEIGHT` in its own line so the weight-measurement work that
/// eventually replaces them can find every one by that string. `()` is a valid implementation,
/// used in this pallet's own unit tests; a runtime that includes this pallet supplies its own
/// generated implementation once `runtime-benchmarks` support is added for it.
pub trait WeightInfo {
    /// Weight for [`Pallet::register_file`].
    fn register_file() -> frame_support::pallet_prelude::Weight;
    /// Weight for [`Pallet::publish_head`].
    fn publish_head() -> frame_support::pallet_prelude::Weight;
    /// Weight for [`Pallet::republish_head`].
    fn republish_head() -> frame_support::pallet_prelude::Weight;
    /// Weight for [`Pallet::append_event`].
    fn append_event() -> frame_support::pallet_prelude::Weight;
}

impl WeightInfo for () {
    fn register_file() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0) // TEMPORARY WEIGHT
    }
    fn publish_head() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0) // TEMPORARY WEIGHT
    }
    fn republish_head() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0) // TEMPORARY WEIGHT
    }
    fn append_event() -> frame_support::pallet_prelude::Weight {
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
        EventIndex, FileFingerprint, ProjectId, RegistryAccess, SchemaId, StorageEndpointId, Vec,
        WeightInfo,
    };
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::Hash;

    /// The pallet's placeholder struct, used to implement traits, methods and dispatchables.
    ///
    /// `without_storage_info` is required because the file, head and event bodies below are
    /// each a `BoundedVec` whose bound is a `Config` constant, but the pallet as a whole is not
    /// used with storage-info-driven tooling — the same trade-off `pallet-pilier-registry`
    /// already makes in this codebase.
    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// The pallet's configuration trait.
    ///
    /// Every length bound is named as a parameter, taken from measured figures rather than
    /// guessed, precisely so the limit is not a number an implementer happens to pick: a chain running
    /// with a bound too small would reject real data on a live chain, where the type cannot be
    /// changed without a runtime upgrade.
    #[pallet::config]
    pub trait Config: frame_system::Config<RuntimeEvent: From<Event<Self>>> {
        /// Consulted on every write: who may publish under a company registration number, does
        /// a schema exist, does a storage endpoint exist. Implemented by
        /// `pallet-pilier-registry`; this pallet never reads that pallet's storage directly.
        type Registry: RegistryAccess<Self::AccountId>;

        /// Upper bound, in bytes, on a company registration number (for France, a SIREN or
        /// SIRET) — half of a passport's key.
        #[pallet::constant]
        type MaxCompanyRegistrationNumberLen: Get<u32>;

        /// Upper bound, in bytes, on a GS1 identifier written as a full GS1 Digital Link (a
        /// product number under application identifier 01, a batch/lot under 10, a serial
        /// number under 21 — as many parts as the passport's own level of detail needs) — the
        /// other half of a passport's key.
        #[pallet::constant]
        type MaxGs1IdLen: Get<u32>;

        /// Upper bound, in bytes, on a head record's body. The ceiling here is four
        /// kibibytes; the record this pallet expects to carry measures to well under a
        /// kibibyte, the difference being headroom for European Union Regulation 1007/2011
        /// Article 11, which requires disclosing composition per part once a product has more
        /// than one part with a different composition.
        #[pallet::constant]
        type MaxRecordBodyLen: Get<u32>;

        /// Upper bound, in bytes, on one lifecycle event's body. The ceiling here is one
        /// hundred twenty-eight bytes; a typical event measures to about fifty.
        #[pallet::constant]
        type MaxEventLen: Get<u32>;

        /// Upper bound, in bytes, on an evidence file's path remainder (the part of its address
        /// past the storage endpoint's own common address part).
        #[pallet::constant]
        type MaxFilePathLen: Get<u32>;

        /// Upper bound, in bytes, on an evidence file's content type (for example, a MIME
        /// type).
        #[pallet::constant]
        type MaxFileContentTypeLen: Get<u32>;

        /// Weight information for this pallet's dispatchables.
        type WeightInfo: WeightInfo;
    }

    /// A company registration number (for France, a SIREN or SIRET), bounded by
    /// `T::MaxCompanyRegistrationNumberLen`.
    pub type CompanyRegistrationNumber<T> =
        BoundedVec<u8, <T as Config>::MaxCompanyRegistrationNumberLen>;

    /// A GS1 identifier written as a full GS1 Digital Link, bounded by `T::MaxGs1IdLen`.
    pub type Gs1Id<T> = BoundedVec<u8, <T as Config>::MaxGs1IdLen>;

    /// A head record's body, an opaque byte string this pallet never parses, bounded by
    /// `T::MaxRecordBodyLen`.
    pub type RecordBody<T> = BoundedVec<u8, <T as Config>::MaxRecordBodyLen>;

    /// One lifecycle event's body, an opaque byte string this pallet never parses, bounded by
    /// `T::MaxEventLen`.
    pub type EventBody<T> = BoundedVec<u8, <T as Config>::MaxEventLen>;

    /// An evidence file's path remainder, bounded by `T::MaxFilePathLen`.
    pub type FilePath<T> = BoundedVec<u8, <T as Config>::MaxFilePathLen>;

    /// An evidence file's content type, bounded by `T::MaxFileContentTypeLen`.
    pub type FileContentType<T> = BoundedVec<u8, <T as Config>::MaxFileContentTypeLen>;

    /// One lifecycle event as this pallet stores it: the caller's opaque body plus the block in
    /// which this pallet recorded it. The two carry different dates and neither substitutes for
    /// the other: a date inside `body`, if the schema puts one there, is a claim made by whoever
    /// called [`Pallet::append_event`] about when the event happened in the product's life, and
    /// this pallet never reads it; `recorded_at` is the block number this pallet itself observed
    /// while handling that call, and it is what a reader who does not trust the caller can check
    /// against the chain.
    #[derive(CloneNoBound, PartialEqNoBound, EqNoBound, Encode, Decode, TypeInfo, RuntimeDebugNoBound)]
    #[scale_info(skip_type_params(T))]
    pub struct EventRecord<T: Config> {
        /// The event's body — an opaque byte string this pallet never parses.
        pub body: EventBody<T>,
        /// The number of the block in which this pallet recorded this event.
        pub recorded_at: BlockNumberFor<T>,
    }

    /// One entry in the deduplicated file table: where an evidence file lives and what it is,
    /// keyed in storage by its content fingerprint.
    #[derive(CloneNoBound, PartialEqNoBound, EqNoBound, Encode, Decode, TypeInfo, RuntimeDebugNoBound)]
    #[scale_info(skip_type_params(T))]
    pub struct FileInfo<T: Config> {
        /// The storage endpoint this file's address is relative to.
        pub storage_endpoint_id: StorageEndpointId,
        /// The part of the file's address past the storage endpoint's own common address part.
        pub path: FilePath<T>,
        /// The file's content type (for example, a MIME type).
        pub content_type: FileContentType<T>,
        /// The number of the block in which this fingerprint was first registered.
        pub registered_at: BlockNumberFor<T>,
    }

    /// Every evidence file the pallet knows about, keyed by its content fingerprint. Registering
    /// a fingerprint already present with the same data is a no-op; registering it with
    /// different data is rejected — see [`Pallet::register_file`].
    #[pallet::storage]
    pub type Files<T: Config> = StorageMap<_, Blake2_128Concat, FileFingerprint, FileInfo<T>>;

    /// A passport's head record: its current body plus everything a reader needs to make sense
    /// of it without parsing it — the body is an opaque byte string, and the schema number
    /// alongside it is what gives it meaning. `version` starts at one and is incremented by
    /// every [`Pallet::republish_head`] call; `previous_body_fingerprint` is `None` for a
    /// version-one record and, for any later version, the hash this pallet itself computed from
    /// the body [`Pallet::republish_head`] replaced — that prior body is not kept in this
    /// pallet's storage, only in the block that published it.
    #[derive(CloneNoBound, PartialEqNoBound, EqNoBound, Encode, Decode, TypeInfo, RuntimeDebugNoBound)]
    #[scale_info(skip_type_params(T))]
    pub struct HeadRecord<T: Config> {
        /// The schema this body is written against, registered in `pallet-pilier-registry`.
        pub schema_id: SchemaId,
        /// The project that published this version of the record.
        pub project_id: ProjectId,
        /// The number of the block in which this version was published.
        pub published_at: BlockNumberFor<T>,
        /// This record's version number. One for a passport's first publication, incremented by
        /// one on every replacement.
        pub version: u32,
        /// The hash of the body this version replaced, computed by this pallet at replacement
        /// time. `None` for a version-one record.
        pub previous_body_fingerprint: Option<T::Hash>,
        /// The record's current body — an opaque byte string this pallet never parses.
        pub body: RecordBody<T>,
    }

    /// Every passport's head record, keyed by the pair that identifies it: the company
    /// registration number of the project that published it, and its GS1 identifier.
    #[pallet::storage]
    pub type Heads<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        CompanyRegistrationNumber<T>,
        Blake2_128Concat,
        Gs1Id<T>,
        HeadRecord<T>,
    >;

    /// How many lifecycle events have been appended to each passport so far. The next
    /// [`Pallet::append_event`] call is assigned this number, and it is unaffected by
    /// [`Pallet::republish_head`] — replacing a head record never touches this counter or the
    /// events already appended under it.
    #[pallet::storage]
    pub type EventCounts<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        CompanyRegistrationNumber<T>,
        Blake2_128Concat,
        Gs1Id<T>,
        EventIndex,
        ValueQuery,
    >;

    /// Every lifecycle event ever appended to a passport, keyed by the passport's own key and
    /// the event's position. Each entry is an [`EventRecord`]: the caller's opaque body next to
    /// `recorded_at`, the block number this pallet itself observed while appending it — the
    /// date the chain attests to, not the date, if any, the body claims. No dispatchable in this
    /// pallet changes or removes an entry here once it is written.
    #[pallet::storage]
    pub type Events<T: Config> = StorageNMap<
        _,
        (
            NMapKey<Blake2_128Concat, CompanyRegistrationNumber<T>>,
            NMapKey<Blake2_128Concat, Gs1Id<T>>,
            NMapKey<Blake2_128Concat, EventIndex>,
        ),
        EventRecord<T>,
    >;

    /// Events that functions in this pallet can emit. Every event names the identifier of the
    /// entity it changed together with the new value of the field that changed, so an observer
    /// can reconstruct the change without reading storage.
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new evidence file fingerprint was registered. Not emitted when a fingerprint
        /// already present is registered again with matching data — that call changes nothing.
        FileRegistered {
            fingerprint: FileFingerprint,
            storage_endpoint_id: StorageEndpointId,
            path: FilePath<T>,
            content_type: FileContentType<T>,
        },
        /// A passport's first version was published.
        HeadPublished {
            company_registration_number: CompanyRegistrationNumber<T>,
            gs1_id: Gs1Id<T>,
            project_id: ProjectId,
            schema_id: SchemaId,
        },
        /// A passport's head record was replaced by a new version. `previous_body_fingerprint`
        /// is this pallet's own hash of the body this version replaced; that body itself
        /// remains only in the block that published it, not in this pallet's storage.
        HeadReplaced {
            company_registration_number: CompanyRegistrationNumber<T>,
            gs1_id: Gs1Id<T>,
            project_id: ProjectId,
            schema_id: SchemaId,
            version: u32,
            previous_body_fingerprint: T::Hash,
        },
        /// A lifecycle event was appended to a passport.
        EventAppended {
            company_registration_number: CompanyRegistrationNumber<T>,
            gs1_id: Gs1Id<T>,
            index: EventIndex,
        },
    }

    /// Errors that can be returned by this pallet.
    #[pallet::error]
    pub enum Error<T> {
        /// The company registration number does not fit `T::MaxCompanyRegistrationNumberLen`.
        CompanyRegistrationNumberTooLong,
        /// The GS1 identifier does not fit `T::MaxGs1IdLen`.
        Gs1IdTooLong,
        /// The head record's body does not fit `T::MaxRecordBodyLen`.
        RecordBodyTooLong,
        /// The lifecycle event's body does not fit `T::MaxEventLen`.
        EventTooLong,
        /// The evidence file's path remainder does not fit `T::MaxFilePathLen`.
        FilePathTooLong,
        /// The evidence file's content type does not fit `T::MaxFileContentTypeLen`.
        FileContentTypeTooLong,
        /// No storage endpoint exists with the given identifier.
        StorageEndpointNotFound,
        /// This fingerprint is already registered with different data. A file's fingerprint,
        /// storage endpoint, path and content type are fixed the first time it is registered;
        /// re-registering the same fingerprint with the same data is accepted and changes
        /// nothing, but re-registering it with different data is rejected outright — the
        /// existing entry is never overwritten.
        FileDataMismatch,
        /// No schema exists with the given identifier.
        SchemaNotFound,
        /// One of the referenced file fingerprints is not registered in the file table. Call
        /// [`Pallet::register_file`] for it first.
        FileNotRegistered,
        /// The calling account's project does not currently hold the right to write under this
        /// company registration number.
        NoPermissionForRegistrationNumber,
        /// A head record already exists at this key. `publish_head` only creates a passport's
        /// first version; call `republish_head` to replace an existing one.
        RecordAlreadyExists,
        /// No head record exists at this key. `republish_head` only replaces an existing
        /// passport, and `append_event` only appends to one; call `publish_head` first to
        /// create it.
        RecordNotFound,
    }

    /// The pallet's dispatchable functions ("calls").
    ///
    /// The weights below are temporary hand-written placeholders, marked `TEMPORARY WEIGHT` in
    /// [`WeightInfo`]'s own default implementation, not this pallet's final generated weights.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register an evidence file's content fingerprint, recording where it lives and what
        /// it is. Registering a fingerprint already present with the same
        /// `storage_endpoint_id`, `path` and `content_type` succeeds and changes nothing;
        /// registering it with any different data is rejected with
        /// [`Error::FileDataMismatch`] — the existing entry is never overwritten.
        ///
        /// Fails with [`Error::StorageEndpointNotFound`] if `storage_endpoint_id` does not name
        /// an existing storage endpoint.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_file())]
        pub fn register_file(
            origin: OriginFor<T>,
            fingerprint: FileFingerprint,
            storage_endpoint_id: StorageEndpointId,
            path: Vec<u8>,
            content_type: Vec<u8>,
        ) -> DispatchResult {
            let _who = ensure_signed(origin)?;
            ensure!(
                T::Registry::storage_endpoint_exists(storage_endpoint_id),
                Error::<T>::StorageEndpointNotFound
            );

            let path: FilePath<T> = path.try_into().map_err(|_| Error::<T>::FilePathTooLong)?;
            let content_type: FileContentType<T> = content_type
                .try_into()
                .map_err(|_| Error::<T>::FileContentTypeTooLong)?;

            match Files::<T>::get(fingerprint) {
                Some(existing) => {
                    ensure!(
                        existing.storage_endpoint_id == storage_endpoint_id
                            && existing.path == path
                            && existing.content_type == content_type,
                        Error::<T>::FileDataMismatch
                    );
                    Ok(())
                }
                None => {
                    let registered_at = frame_system::Pallet::<T>::block_number();
                    Files::<T>::insert(
                        fingerprint,
                        FileInfo {
                            storage_endpoint_id,
                            path: path.clone(),
                            content_type: content_type.clone(),
                            registered_at,
                        },
                    );
                    Self::deposit_event(Event::FileRegistered {
                        fingerprint,
                        storage_endpoint_id,
                        path,
                        content_type,
                    });
                    Ok(())
                }
            }
        }

        /// Publish a passport's first version. Fails with
        /// [`Error::NoPermissionForRegistrationNumber`] if the caller's project does not hold
        /// the right to write under `company_registration_number`, with
        /// [`Error::SchemaNotFound`] if `schema_id` does not name an existing schema, with
        /// [`Error::FileNotRegistered`] if any of `file_fingerprints` is not in the file table,
        /// and with [`Error::RecordAlreadyExists`] if a head record already exists at this key
        /// — call [`Pallet::republish_head`] instead. `file_fingerprints` is checked and then
        /// discarded: this pallet stores only `body`, never the list, because the body already
        /// carries whatever references to these files its own schema puts there.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::publish_head())]
        pub fn publish_head(
            origin: OriginFor<T>,
            company_registration_number: Vec<u8>,
            gs1_id: Vec<u8>,
            schema_id: SchemaId,
            body: Vec<u8>,
            file_fingerprints: Vec<FileFingerprint>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let (company_registration_number, gs1_id, project_id) =
                Self::authorise_write(&who, company_registration_number, gs1_id, schema_id, &file_fingerprints)?;
            ensure!(
                !Heads::<T>::contains_key(&company_registration_number, &gs1_id),
                Error::<T>::RecordAlreadyExists
            );

            let body: RecordBody<T> = body.try_into().map_err(|_| Error::<T>::RecordBodyTooLong)?;
            let published_at = frame_system::Pallet::<T>::block_number();
            Heads::<T>::insert(
                &company_registration_number,
                &gs1_id,
                HeadRecord {
                    schema_id,
                    project_id,
                    published_at,
                    version: 1,
                    previous_body_fingerprint: None,
                    body,
                },
            );

            Self::deposit_event(Event::HeadPublished {
                company_registration_number,
                gs1_id,
                project_id,
                schema_id,
            });
            Ok(())
        }

        /// Replace a passport's head record with a new version. This is the only way to change
        /// an already-published record: an ordinary [`Pallet::publish_head`] on an occupied key
        /// is rejected, so a worker's unacknowledged retry can never create a new version
        /// silently. Fails with [`Error::NoPermissionForRegistrationNumber`],
        /// [`Error::SchemaNotFound`] or [`Error::FileNotRegistered`] under the same conditions
        /// as [`Pallet::publish_head`], and with [`Error::RecordNotFound`] if no head record
        /// exists at this key yet — call [`Pallet::publish_head`] first.
        ///
        /// The record's previous body is not kept in this pallet's storage: it remains in the
        /// block that published it, and this call computes and stores its hash — in the record
        /// and in the [`Event::HeadReplaced`] this call emits — so that hash proves what that
        /// body was without this pallet holding a copy of it.
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::republish_head())]
        pub fn republish_head(
            origin: OriginFor<T>,
            company_registration_number: Vec<u8>,
            gs1_id: Vec<u8>,
            schema_id: SchemaId,
            body: Vec<u8>,
            file_fingerprints: Vec<FileFingerprint>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let (company_registration_number, gs1_id, project_id) =
                Self::authorise_write(&who, company_registration_number, gs1_id, schema_id, &file_fingerprints)?;
            let existing = Heads::<T>::get(&company_registration_number, &gs1_id)
                .ok_or(Error::<T>::RecordNotFound)?;

            let body: RecordBody<T> = body.try_into().map_err(|_| Error::<T>::RecordBodyTooLong)?;
            let previous_body_fingerprint = T::Hashing::hash(&existing.body);
            let version = existing.version.saturating_add(1);
            let published_at = frame_system::Pallet::<T>::block_number();
            Heads::<T>::insert(
                &company_registration_number,
                &gs1_id,
                HeadRecord {
                    schema_id,
                    project_id,
                    published_at,
                    version,
                    previous_body_fingerprint: Some(previous_body_fingerprint),
                    body,
                },
            );

            Self::deposit_event(Event::HeadReplaced {
                company_registration_number,
                gs1_id,
                project_id,
                schema_id,
                version,
                previous_body_fingerprint,
            });
            Ok(())
        }

        /// Append a lifecycle event to a passport's event table, at the next free position.
        /// `body` is stored exactly as given, unparsed, alongside the block number this pallet
        /// observes while handling this call — see [`EventRecord`] for why the two dates inside
        /// a stored event, one possibly inside `body` and one the block number the chain itself
        /// attests to, are not the same thing. This pallet declares no call that changes or
        /// removes an event once appended — see [`Events`]'s own documentation.
        ///
        /// Fails with [`Error::NoPermissionForRegistrationNumber`] if the caller's project does
        /// not hold the right to write under `company_registration_number`, and with
        /// [`Error::RecordNotFound`] if no head record exists at this key yet — call
        /// [`Pallet::publish_head`] first.
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::append_event())]
        pub fn append_event(
            origin: OriginFor<T>,
            company_registration_number: Vec<u8>,
            gs1_id: Vec<u8>,
            body: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let company_registration_number: CompanyRegistrationNumber<T> = company_registration_number
                .try_into()
                .map_err(|_| Error::<T>::CompanyRegistrationNumberTooLong)?;
            let gs1_id: Gs1Id<T> = gs1_id.try_into().map_err(|_| Error::<T>::Gs1IdTooLong)?;

            ensure!(
                T::Registry::writer_project(&who, &company_registration_number).is_some(),
                Error::<T>::NoPermissionForRegistrationNumber
            );
            ensure!(
                Heads::<T>::contains_key(&company_registration_number, &gs1_id),
                Error::<T>::RecordNotFound
            );

            let body: EventBody<T> = body.try_into().map_err(|_| Error::<T>::EventTooLong)?;
            let index = EventCounts::<T>::get(&company_registration_number, &gs1_id);
            let recorded_at = frame_system::Pallet::<T>::block_number();
            Events::<T>::insert(
                (&company_registration_number, &gs1_id, index),
                EventRecord { body, recorded_at },
            );
            EventCounts::<T>::insert(
                &company_registration_number,
                &gs1_id,
                index.saturating_add(1),
            );

            Self::deposit_event(Event::EventAppended {
                company_registration_number,
                gs1_id,
                index,
            });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// The checks shared by [`Pallet::publish_head`] and [`Pallet::republish_head`]: bound
        /// the key, resolve `who`'s write permission through [`Config::Registry`], confirm
        /// `schema_id` exists, and confirm every fingerprint in `file_fingerprints` is already
        /// registered in [`Files`]. Returns the bounded key and the resolved project identifier
        /// for the caller to use.
        fn authorise_write(
            who: &T::AccountId,
            company_registration_number: Vec<u8>,
            gs1_id: Vec<u8>,
            schema_id: SchemaId,
            file_fingerprints: &[FileFingerprint],
        ) -> Result<(CompanyRegistrationNumber<T>, Gs1Id<T>, ProjectId), DispatchError> {
            let company_registration_number: CompanyRegistrationNumber<T> = company_registration_number
                .try_into()
                .map_err(|_| Error::<T>::CompanyRegistrationNumberTooLong)?;
            let gs1_id: Gs1Id<T> = gs1_id.try_into().map_err(|_| Error::<T>::Gs1IdTooLong)?;

            let project_id = T::Registry::writer_project(who, &company_registration_number)
                .ok_or(Error::<T>::NoPermissionForRegistrationNumber)?;
            ensure!(T::Registry::schema_exists(schema_id), Error::<T>::SchemaNotFound);
            for fingerprint in file_fingerprints {
                ensure!(Files::<T>::contains_key(fingerprint), Error::<T>::FileNotRegistered);
            }

            Ok((company_registration_number, gs1_id, project_id))
        }
    }
}
