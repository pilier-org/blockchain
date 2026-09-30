# Changelog

All notable changes to the Pilier runtime, keyed by runtime `spec_version`. This file follows the
spirit of [Keep a Changelog](https://keepachangelog.com). It records what changed and why for
node operators and integrators; it deliberately omits internal implementation and decision detail.

## Runtime rollout

Before any runtime upgrade is deployed, the WebAssembly file being rolled out is checked against
the fingerprint the continuous-integration run published for the same commit, using
`scripts/verify-runtime-fingerprint.sh`.

That fingerprint is reproducible because the compiler is pinned to an exact version in
`env-setup/rust-toolchain.toml`, and both the continuous-integration workflow and the deployment
image build with that version. Building the runtime with a newer compiler produces a different
file, or fails to link at all.

## [runtime 106]

### Changed
- **A project manages its own registry types, schemas, writer keys and ownership.** Until this
  release, adding a registry type or registering a reference schema required the supermajority
  council vote (or the root key) that governs every administrative call in this runtime. From
  this release a project's own owner performs these directly: it creates the project's registry
  types, registers its schemas, adds and removes the writer keys allowed to record entries, and
  transfers the project's ownership. The council and root key keep the powers they already had;
  what changes is that routine project data management no longer needs either of them.
- **`register_schema` now names the project.** The call takes the project identifier as its
  first argument, so a schema belongs to the project that registered it rather than to the chain
  at large.

### Added
- **Project ownership handover and writer management.** New registry calls let a project's
  current owner propose a new owner, let the proposed account accept ownership, and let the owner
  add or remove writer keys. Accepting ownership does not by itself make the previous owner a
  writer, so an owner that must keep writing registers itself as a writer before handing the
  project over.

### Storage migration
- **Every schema record gains a `project` field.** A one-time migration rewrites each schema
  already in storage with `project` set to none, and raises the registry pallet's on-chain
  storage version to 1. Because this changes the format of storage that already holds live data,
  a runtime carrying the previous format can no longer read it: reverting this upgrade by
  submitting the older runtime is not possible, and the only supported rollback is restoring
  nodes from a state snapshot taken before the upgrade.

## [runtime 105]

### Added
- **Digital product passport pallet.** The pallet that publishes passport records and events
  against a product, planned in runtime 104 but deferred out of it, now joins the runtime. A
  passport is published under a company registration number and a GS1 product identifier, and
  every evidence file it cites by fingerprint must already be registered in the documents
  pallet runtime 104 added — a passport can no longer name a file nobody has registered.
  Administrative calls are governed the same way every other pallet in this runtime already is:
  a supermajority (at least 75%) vote of the validators' council, with the root key retained as
  an emergency override.

## [runtime 104]

### Added
- **Product registry and evidence-file document pallets.** Two new pallets join the runtime: a
  registry that projects use to record their company registration number, GS1 product
  identifiers, reference schemas and storage endpoints, and a documents pallet that holds a
  chain-wide, deduplicated table of evidence-file fingerprints. Registering a file names the
  project it is registered under, and only that project's owner or one of its writers — a
  council-approved project, in other words — may register a file under it; every other account
  is rejected. Administrative calls on both pallets — granting registry permissions and changing
  the file registration price — are governed the same way validator changes already are: a
  supermajority (at least 75%) vote of the validators' council, with the root key retained as an
  emergency override. The registration price itself starts at 0.0025 PIL, a value the runtime
  carries directly rather than one set by a migration, and can be changed later by the same
  council vote, without a further runtime upgrade.

  The digital product passport pallet that publishes passport records and events against these
  registered products is not part of this runtime version; it is planned for a later runtime
  version.

  This is the first upgrade the validators' council authorises by vote, using the mechanism
  runtime 103 added.

## [runtime 103]

### Added
- **Council vote for runtime upgrades.** A runtime upgrade can now be authorized by a
  supermajority (at least 75%) vote of the validators' council, with the root key retained as an
  emergency override. Previously only the root key could authorize an upgrade. This follows the
  same governance shape already used for changing the validator set.

## [runtime 102] — 2026-07-19

### Added
- **Mutable validator set.** Validators can now be added to or removed from the active set by a
  supermajority (at least 75%) vote of the validators' council, with the root key retained as an
  emergency override. Previously the validator set was fixed at genesis and could not change
  without a new chain.

### Changed
- **Transaction fees are paid to the block author.** The full transaction fee now goes to the
  validator that produced the block, rather than being burned.

## [runtime 101]

### Changed
- **Transaction cost adjustment.** Reworked the transaction fee configuration.
