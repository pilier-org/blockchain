//! # Pilier Documents Pallet
//!
//! Holds the chain-wide, deduplicated table of evidence-file fingerprints a digital product
//! passport cites as proof: where a file lives, what it is, and which council-approved project
//! registered it. The table is deliberately not owned by a project or scoped to it — the same
//! certificate can back passports from several projects, so it is stored once and referenced by
//! its content fingerprint, exactly as the passport pallet's own head-record table deduplicates
//! by fingerprint rather than by publisher.
//!
//! This pallet asks `pallet-pilier-registry`, through the [`RegistryAccess`] trait, for every
//! permission and existence check a write depends on: whether the calling account is a member of
//! the named project, and whether a storage endpoint exists. It never reads that pallet's storage
//! directly. The passport pallet, in turn, asks this pallet through [`DocumentsAccess`] whether a
//! fingerprint is registered; it never reads this pallet's storage directly either.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;

// Re-export pallet items so they can be accessed from the crate namespace.
pub use pallet::*;
pub use pallet_pilier_registry::{ProjectId, RegistryAccess, StorageEndpointId};

/// A file's content fingerprint: thirty-two bytes, computed by the caller from the file's
/// content and never by this pallet. The file table is keyed by this value, so registering the
/// same content twice — from the same or a different project — deduplicates to one entry.
pub type FileFingerprint = [u8; 32];

/// Consulted by the passport pallet on every publication, so it never reads this pallet's
/// storage directly. It answers exactly the one question a passport write depends on: is this
/// fingerprint registered.
pub trait DocumentsAccess {
    /// Returns whether a file with `fingerprint` is registered.
    fn file_exists(fingerprint: FileFingerprint) -> bool;
}

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
    /// Weight for [`Pallet::set_price`].
    fn set_price() -> frame_support::pallet_prelude::Weight;
}

impl WeightInfo for () {
    fn register_file() -> frame_support::pallet_prelude::Weight {
        frame_support::pallet_prelude::Weight::from_parts(10_000, 0) // TEMPORARY WEIGHT
    }
    fn set_price() -> frame_support::pallet_prelude::Weight {
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
        DocumentsAccess, FileFingerprint, ProjectId, RegistryAccess, StorageEndpointId, Vec,
        WeightInfo,
    };
    use frame_support::{
        pallet_prelude::*,
        traits::{
            Imbalance, fungible,
            fungible::Balanced,
            tokens::{Fortitude, Precision, Preservation},
        },
    };
    use frame_system::pallet_prelude::*;

    /// The pallet's placeholder struct, used to implement traits, methods and dispatchables.
    ///
    /// Every stored value here has a compile-time upper bound: `path` and `content_type` are
    /// each a `BoundedVec` sized by a `Config` constant, so the automatic storage-size
    /// calculation works and is kept on. The map itself carries no bound on its number of
    /// entries — growth is checked by write access (only a council-approved project may write)
    /// and by price (`DocumentPrice`, raisable by the council without a runtime upgrade), not by
    /// a count ceiling; see this pallet's own plan for why a count ceiling is not needed here.
    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// The pallet's configuration trait.
    #[pallet::config]
    pub trait Config:
        frame_system::Config<RuntimeEvent: From<Event<Self>>> + pallet_authorship::Config
    {
        /// Consulted on every write: is the calling account a member of the named project, does
        /// a storage endpoint exist. Implemented by `pallet-pilier-registry`; this pallet never
        /// reads that pallet's storage directly.
        type Registry: RegistryAccess<Self::AccountId>;

        /// The fungible token [`DocumentPrice`] is charged in and paid out from. Bound through
        /// the same `fungible` trait family `pallet_transaction_payment`'s `FungibleAdapter`
        /// uses for the standard fee, so this pallet withdraws and resolves balances the same
        /// way the runtime's own fee handler does.
        type Currency: fungible::Balanced<Self::AccountId>;

        /// Origin allowed to change [`DocumentPrice`] with [`Pallet::set_price`]. This pallet
        /// does not know about any particular collective directly — wiring this to the
        /// validators' council is a runtime-integration concern outside this pallet's own
        /// scope, the same design `pallet-pilier-registry`'s and `pallet-pilier-dpp`'s own
        /// `AdminOrigin` already use.
        type AdminOrigin: EnsureOrigin<Self::RuntimeOrigin>;

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

    /// The balance type of [`Config::Currency`], derived rather than named directly so a
    /// runtime only has to say which currency this pallet uses, not repeat its balance type.
    pub type BalanceOf<T> = <<T as Config>::Currency as fungible::Inspect<
        <T as frame_system::Config>::AccountId,
    >>::Balance;

    /// An evidence file's path remainder, bounded by `T::MaxFilePathLen`.
    pub type FilePath<T> = BoundedVec<u8, <T as Config>::MaxFilePathLen>;

    /// An evidence file's content type, bounded by `T::MaxFileContentTypeLen`.
    pub type FileContentType<T> = BoundedVec<u8, <T as Config>::MaxFileContentTypeLen>;

    /// One entry in the deduplicated file table: where an evidence file lives, what it is, and
    /// which project registered it, keyed in storage by its content fingerprint.
    #[derive(
        CloneNoBound,
        PartialEqNoBound,
        EqNoBound,
        Encode,
        Decode,
        MaxEncodedLen,
        TypeInfo,
        RuntimeDebugNoBound,
    )]
    #[codec(mel_bound(T: Config))]
    #[scale_info(skip_type_params(T))]
    pub struct FileInfo<T: Config> {
        /// The project, checked for membership at registration time, that registered this
        /// fingerprint. Names who is responsible for this entry — it does not make the table
        /// project-owned: the table stays deduplicated chain-wide by fingerprint.
        pub registered_by: ProjectId,
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

    /// The document registration price this cell reads as when its storage key has never been
    /// written: two thousand five hundred of [`Config::Currency`]'s smallest unit — 0.0025 PIL
    /// at the runtime's one-million-unit token, the figure the published tokenomics names for
    /// registering a document. Applies on every chain this pallet is part of, however the
    /// pallet arrived there — a fresh genesis or a forkless runtime upgrade — because both leave
    /// the key unwritten the same way. A written value, including a written zero, always
    /// overrides this default: this default is read only for a key nothing has ever written.
    #[pallet::type_value]
    pub fn DefaultDocumentPrice<T: Config>() -> BalanceOf<T> {
        BalanceOf::<T>::from(2_500u32)
    }

    /// The price charged, in [`Config::Currency`]'s smallest unit, for one call to
    /// [`Pallet::register_file`] that actually registers a new fingerprint — see that call's own
    /// documentation for how it is charged and accounted. Changed only by [`Config::AdminOrigin`]
    /// through [`Pallet::set_price`]. Reads as [`DefaultDocumentPrice`] for a key nothing has
    /// ever written; any write, including a write of zero, overrides that default from then on.
    #[pallet::storage]
    pub type DocumentPrice<T: Config> =
        StorageValue<_, BalanceOf<T>, ValueQuery, DefaultDocumentPrice<T>>;

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
            project_id: ProjectId,
        },
        /// [`DocumentPrice`] was changed by [`Config::AdminOrigin`] through
        /// [`Pallet::set_price`].
        DocumentPriceChanged {
            old_price: BalanceOf<T>,
            new_price: BalanceOf<T>,
        },
    }

    /// Errors that can be returned by this pallet.
    #[pallet::error]
    pub enum Error<T> {
        /// The calling account is not a member — owner or writer — of the named project, or no
        /// project exists with that identifier.
        NotProjectMember,
        /// No storage endpoint exists with the given identifier.
        StorageEndpointNotFound,
        /// The evidence file's path remainder does not fit `T::MaxFilePathLen`.
        FilePathTooLong,
        /// The evidence file's content type does not fit `T::MaxFileContentTypeLen`.
        FileContentTypeTooLong,
        /// This fingerprint is already registered with different data. A file's fingerprint,
        /// storage endpoint, path and content type are fixed the first time it is registered;
        /// re-registering the same fingerprint with the same data is accepted and changes
        /// nothing, but re-registering it with different data is rejected outright — the
        /// existing entry is never overwritten.
        FileDataMismatch,
    }

    /// The pallet's dispatchable functions ("calls").
    ///
    /// The weights below are temporary hand-written placeholders, marked `TEMPORARY WEIGHT` in
    /// [`WeightInfo`]'s own default implementation, not this pallet's final generated weights.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register an evidence file's content fingerprint under `project_id`, recording where
        /// it lives and what it is. The caller must be a member — owner or writer — of
        /// `project_id`; this is checked before anything else in this call, including a repeat
        /// registration of an already-known fingerprint, so an outsider naming a project it does
        /// not belong to is always rejected with [`Error::NotProjectMember`] rather than ever
        /// reaching the "data matches" no-op path.
        ///
        /// Registering a fingerprint already present with the same `storage_endpoint_id`, `path`
        /// and `content_type` succeeds and changes nothing — no charge, no event, the existing
        /// entry (including its `registered_by`) is left exactly as it was, even when called
        /// under a different project. Registering it with any different data is rejected with
        /// [`Error::FileDataMismatch`] — the existing entry is never overwritten.
        ///
        /// Fails with [`Error::StorageEndpointNotFound`] if `storage_endpoint_id` does not name
        /// an existing storage endpoint.
        ///
        /// Charges [`DocumentPrice`] to the caller — see [`Pallet::charge_document_price`] for
        /// how it is charged and accounted — only when a new fingerprint is actually registered,
        /// before writing anything, so a failure to pay leaves this pallet's storage exactly as
        /// it was before the call. On that success, refunds the standard transaction fee (but
        /// not a voluntary tip) by returning `Pays::No`, exactly as `pallet-pilier-dpp`'s
        /// `publish_head` does for its own fixed price. On a no-op or a failure, the standard fee
        /// stays charged as normal.
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::register_file())]
        pub fn register_file(
            origin: OriginFor<T>,
            project_id: ProjectId,
            fingerprint: FileFingerprint,
            storage_endpoint_id: StorageEndpointId,
            path: Vec<u8>,
            content_type: Vec<u8>,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;
            ensure!(
                T::Registry::is_project_member(project_id, &who),
                Error::<T>::NotProjectMember
            );
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
                    Ok(Pays::Yes.into())
                }
                None => {
                    Self::charge_document_price(&who)?;

                    let registered_at = frame_system::Pallet::<T>::block_number();
                    Files::<T>::insert(
                        fingerprint,
                        FileInfo {
                            registered_by: project_id,
                            storage_endpoint_id,
                            path,
                            content_type,
                            registered_at,
                        },
                    );
                    Self::deposit_event(Event::FileRegistered {
                        fingerprint,
                        project_id,
                    });
                    Ok(Pays::No.into())
                }
            }
        }

        /// Change [`DocumentPrice`], the fixed amount [`Pallet::register_file`] charges for a
        /// new registration. Must be called by [`Config::AdminOrigin`]. A price of zero is
        /// accepted and makes new registrations free of this pallet's own charge — the standard
        /// transaction fee still applies as normal.
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::set_price())]
        pub fn set_price(origin: OriginFor<T>, new_price: BalanceOf<T>) -> DispatchResult {
            T::AdminOrigin::ensure_origin(origin)?;
            let old_price = DocumentPrice::<T>::get();
            DocumentPrice::<T>::put(new_price);
            Self::deposit_event(Event::DocumentPriceChanged {
                old_price,
                new_price,
            });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Withdraw [`DocumentPrice`] from `who` and pay it to the current block's author.
        /// Mirrors `pallet-pilier-dpp`'s own `charge_publication_price` exactly: burns instead
        /// of paying out — logging a warning rather than silently dropping — exactly when the
        /// runtime's own standard fee handler (`ToAuthor`) would also burn: no block author
        /// could be determined, or resolving the payment to their account failed (for example
        /// because it would leave them below the existential deposit and their account does not
        /// already exist). Either way the payer has already paid, the call proceeds.
        ///
        /// Fails, without withdrawing anything, if `who` does not hold at least
        /// [`DocumentPrice`]. [`Pallet::register_file`] calls this after every other check has
        /// passed and before writing any storage, so a failure here leaves the pallet's storage
        /// exactly as it was before the call.
        fn charge_document_price(who: &T::AccountId) -> Result<(), DispatchError> {
            let price = DocumentPrice::<T>::get();
            let credit = T::Currency::withdraw(
                who,
                price,
                Precision::Exact,
                Preservation::Preserve,
                Fortitude::Polite,
            )?;

            match pallet_authorship::Pallet::<T>::author() {
                Some(author) => {
                    if let Err(not_resolved) = T::Currency::resolve(&author, credit) {
                        log::warn!(
                            target: "runtime::documents",
                            "document price of {:?} could not be resolved to block author \
                             {:?} (e.g. below the existential deposit) — burned instead of paid",
                            not_resolved.peek(),
                            author,
                        );
                    }
                }
                None => {
                    log::warn!(
                        target: "runtime::documents",
                        "document price of {:?} burned: no block author found",
                        credit.peek(),
                    );
                }
            }

            Ok(())
        }
    }

    impl<T: Config> DocumentsAccess for Pallet<T> {
        fn file_exists(fingerprint: FileFingerprint) -> bool {
            Files::<T>::contains_key(fingerprint)
        }
    }
}
