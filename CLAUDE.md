# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this project is

Pilier is a standalone (solo) blockchain built with the Polkadot SDK and its FRAME
framework, in the Rust language. "Solochain" means it runs on its own, not as a parachain
attached to the Polkadot relay chain. It is a Proof-of-Authority network: a fixed, named set
of validator nodes produce and finalise blocks, rather than an open set chosen by staking.

The chain was bootstrapped from Parity's `polkadot-sdk-solochain-template` and then customised.
It now tracks the `stable2512` branch of `polkadot-sdk` (the December 2025 stable release). The
native token is **PIL**, with 6 decimal places and an address format (SS58 prefix) of 42.

Two facts about the code are easy to get wrong, so keep them in mind:

- **The `README.md` at the repository root is still the stock Substrate template text.** It
  refers to a binary called `solochain-template-node`, which does not exist here. The real
  binary is `pilier-node`. Trust this file and the actual source over the README for anything
  operational.
- **The `pallets/template` crate is a scaffold that is NOT part of the running chain.** It is a
  member of the Cargo workspace and it compiles, but it is never added to the runtime, so none
  of its logic executes on-chain. The only reference to it in the runtime is a commented-out
  line in the benchmark list. Treat it as example code to copy from when writing a real pallet,
  not as live behaviour.

## Workspace layout

This is a Cargo workspace (see the root `Cargo.toml`) with three kinds of member:

- `node/` — the node application, crate name `pilier-node`. This is the program operators run.
  It handles peer-to-peer networking, the consensus engines, the remote-procedure-call (RPC)
  server, and the command-line interface. It does not contain the chain's business rules.
- `runtime/` — the runtime, crate name `pilier-runtime`. This is the "state transition
  function": the actual on-chain logic that validates blocks and applies their changes. It is
  compiled both to native code and to WebAssembly, and the WebAssembly copy is what the chain
  upgrades when the logic changes.
- `pallets/*` — individual FRAME modules ("pallets"). Currently only `pallets/template`, which,
  as noted above, is not wired in.

### How the runtime is assembled

The runtime is defined across a few files, and understanding the split saves time:

- `runtime/src/lib.rs` declares the chain's core types (account, balance, block, signature),
  the runtime version block (`spec_version`, currently 101 — bump this on any runtime change),
  the block time (6 seconds), the token constants (`UNIT = 1_000_000`, meaning one PIL is a
  million of the smallest unit), and the list of pallets with their fixed indices, inside the
  `#[frame_support::runtime]` block. The pallets included are: System, Timestamp, Aura,
  Grandpa, Balances, TransactionPayment, and Sudo. **Pallet indices are part of the chain's
  wire format — never renumber an existing pallet; only append new ones.**
- `runtime/src/configs/mod.rs` holds the `impl ... Config for Runtime` block for every pallet.
  This is where each pallet is parameterised: block weights and length limits, the existential
  deposit, the fee formula (`WeightToFee`), the consensus authority limits, and so on.
- `runtime/src/apis.rs` implements the runtime API traits the node calls into.
- `runtime/src/genesis_config_presets.rs` builds the named genesis presets. See below.

### Consensus and administration

Block *authoring* uses **Aura** (authorities take turns producing blocks on a schedule); block
*finality* uses **GRANDPA** (authorities vote to finalise). Both authority sets are seeded at
genesis and, in this Proof-of-Authority design, are not opened to public staking. The **Sudo**
pallet gives one privileged "root" key administrative control; the code comments describe this
as intended for Testnet Phase 1, so expect it to be removed or replaced before a production
launch. The wiring of these engines into the node lives in `node/src/service.rs`.

## Chains and genesis

The node knows three chain identifiers, resolved in `node/src/command.rs` (`load_spec`), with
their genesis defined in `node/src/chain_spec.rs`:

- `dev` — single-node development chain, Alice as sole authority and sudo key. Non-persistent.
- `local` (also the default when no `--chain` is given) — two-node local testnet, Alice and Bob.
- `pilier_testnet` — the real testnet. Its genesis is a *preset* named `pilier_testnet` defined
  in `runtime/src/genesis_config_presets.rs`, which hard-codes real SS58 addresses: three
  validator nodes and several endowed pools (faucet, ecosystem, treasury, civic, team, reserve).
  Any other value passed to `--chain` is treated as a path to a chain-spec JSON file.

The pre-built raw chain specifications for the testnet live at the repository root as
`pilier-testnet-raw.json` (the raw form nodes actually load) and `pilier-testnet.json` (the
human-readable form). If you change genesis, these files must be regenerated with `build-spec`
(see commands below), or nodes will disagree on the genesis hash and fail to connect.

## Common commands

Build the node (release profile is effectively required — debug builds of a Substrate node are
extremely slow):

```sh
cargo build --release
# binary lands at ./target/release/pilier-node
```

Run a throwaway single-node dev chain:

```sh
./target/release/pilier-node --dev
# purge its state with:
./target/release/pilier-node purge-chain --dev
```

Run with verbose logging:

```sh
RUST_LOG=debug ./target/release/pilier-node --dev
```

Explore all subcommands and flags:

```sh
./target/release/pilier-node --help
```

Regenerate the testnet raw chain spec after a genesis change:

```sh
./target/release/pilier-node build-spec --chain pilier_testnet --raw --disable-default-bootnode \
  > pilier-testnet-raw.json
```

### Tests, formatting, linting

```sh
cargo test --workspace          # run all tests
cargo test -p pallet-template   # tests for one crate only (mock + tests in that crate)
cargo test -p pallet-template -- <test_name>   # a single test by name
cargo fmt --all                 # format (rustfmt is pinned in env-setup/rust-toolchain.toml)
cargo clippy --workspace        # lint
```

Pallet tests follow the FRAME convention: a mock runtime in `src/mock.rs` and the tests in
`src/tests.rs`.

### Benchmarks

Weight benchmarking is behind a Cargo feature and is off by default. The node prints a clear
error if you invoke a benchmark subcommand without it. Build with the feature to use them:

```sh
cargo build --release --features runtime-benchmarks
./target/release/pilier-node benchmark pallet --chain dev --pallet <name> --extrinsic '*' ...
```

## Toolchain

Rust **edition 2024**, stable channel, with the `wasm32-unknown-unknown` target required for the
WebAssembly runtime build. The exact toolchain and components are pinned in
`env-setup/rust-toolchain.toml`, and `env-setup/` also provides a Nix flake for a reproducible
development shell (`direnv allow`, or `nix develop`). Building from source needs the usual
Substrate system dependencies (clang, protobuf compiler, libssl, etc.); the list is in
`deployment/Dockerfile`.

## Deployment

Validator operators are expected to run the node in Docker, not from a local build:

- `deployment/Dockerfile` — two-stage build (Rust builder on `rust:1.92-bookworm`, then a slim
  Debian runtime) that produces an image running `pilier-node`. Exposes ports 30333 (peer-to-peer),
  9944 (RPC), and 9615 (Prometheus metrics).
- `deployment/docker-compose.template.yml` — runs one validator against `--chain=pilier_testnet`,
  parameterised through a `.env` file.
- `deployment/setup-validator.sh` — interactive helper that writes the `.env` (validator name,
  ports, bootnode address) and prints the follow-up `docker-compose` commands, including how to
  generate validator keys with `pilier-node key generate`.
- `deployment/.env.example` — the template for that `.env`.
- `ops/testnet/Dockerfile` — a separate operations image for the testnet.
