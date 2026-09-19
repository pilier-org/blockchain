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
