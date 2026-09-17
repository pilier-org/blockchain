# Changelog

All notable changes to the Pilier runtime, keyed by runtime `spec_version`. This file follows the
spirit of [Keep a Changelog](https://keepachangelog.com). It records what changed and why for
node operators and integrators; it deliberately omits internal implementation and decision detail.

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
