# Agent instructions for Pilier

Compact orientation for OpenCode sessions. For the full project story, read `CLAUDE.md` first; for the reasoning behind specific decisions, read `ai/decisions/*.md`.

## Do not trust the root README

- `README.md` is still the stock `polkadot-sdk-solochain-template` text. It refers to a non-existent binary (`solochain-template-node`) and generic template commands.
- The real binary is **`pilier-node`**. Trust this file, `CLAUDE.md`, and the actual source over `README.md`.

## What this repo is

- Standalone (solo) Proof-of-Authority blockchain built with the Polkadot SDK `stable2512` branch and FRAME. It is *not* a parachain.
- Native token: PIL, 6 decimals, SS58 prefix 42.

## Workspace layout

- `node/` — operator node (`pilier-node`); networking, RPC, Aura/GRANDPA consensus, CLI.
- `runtime/` — on-chain state transition function (`pilier-runtime`), compiled to native and WebAssembly.
- `pallets/validator-set/` — **live** pallet. Owns the mutable validator set and feeds it to `pallet-session` via `SessionManager`.
- `pallets/template/` — scaffold only. It compiles as a workspace member but is **not wired into the runtime**. Treat it as example code.

## Runtime pallets and a critical ordering rule

Live pallets and their fixed indices in `runtime/src/lib.rs`:

| Index | Pallet |
|-------|--------|
| 0 | System |
| 1 | Timestamp |
| 2 | Aura |
| 3 | Grandpa |
| 4 | Balances |
| 5 | TransactionPayment |
| 6 | Sudo |
| 7 | **ValidatorSet** |
| 8 | **Session** |
| 9 | Council (`pallet-collective` Instance1) |

**Ordering constraint:** `ValidatorSet` must have a lower pallet index than `Session`. FRAME builds genesis by ascending index, and `Session` reads `ValidatorSet::Validators` during its genesis build. If `Session` is indexed lower, genesis authorities become empty and the node panics on startup (`genesis authorities is non-empty`). See `ai/decisions/genesis-build-order.md`.

**Never renumber an existing pallet; only append new indices.** Pallet indices are part of the chain's wire format.

## Developer commands

Build the release node (expected for any non-trivial run; debug builds are extremely slow):

```sh
cargo build --release
```

Run a throwaway single-node dev chain:

```sh
./target/release/pilier-node --dev
```

Purge dev chain state:

```sh
./target/release/pilier-node purge-chain --dev
```

Explore subcommands and flags:

```sh
./target/release/pilier-node --help
```

## Tests and verification

Run all workspace tests:

```sh
cargo test --workspace
```

Run tests for one pallet:

```sh
cargo test -p pallet-validator-set
cargo test -p pallet-template
```

Run a single test by name:

```sh
cargo test -p pallet-validator-set -- <test_name>
```

Format and lint:

```sh
cargo fmt --all
cargo clippy --workspace
```

## Benchmarks

Benchmarking requires the `runtime-benchmarks` feature. Build with it before invoking any benchmark subcommand, or the node exits with an explicit error:

```sh
cargo build --release --features runtime-benchmarks
./target/release/pilier-node benchmark pallet --chain dev --pallet <name> --extrinsic '*' ...
```

## Toolchain

- Rust edition 2024, stable channel. The `wasm32-unknown-unknown` target is required.
- The pinned toolchain and components are in `env-setup/rust-toolchain.toml`. The Nix flake in `env-setup/` provides a reproducible dev shell.

## Chains and genesis

Built-in chain identifiers, resolved in `node/src/command.rs`:

- `dev` — single-node, Alice only, non-persistent.
- `local` — default when no `--chain` is given; two-node local testnet with Alice and Bob.
- `pilier_testnet` — the real testnet; genesis comes from the preset in `runtime/src/genesis_config_presets.rs`.

Any other `--chain` value is treated as a path to a chain-spec JSON file.

If genesis changes, regenerate the raw chain specs at the repo root so nodes agree on the genesis hash:

```sh
./target/release/pilier-node build-spec --chain pilier_testnet --raw --disable-default-bootnode > pilier-testnet-raw.json
```

## Runtime versioning

- `spec_version` lives in `runtime/src/lib.rs` inside the `VERSION` block. Bump it on any runtime change.
- It is append-only / monotonically increasing. Never decrease it, even when rebuilding genesis from scratch.
- The changelog of version numbers is in `ai/decisions/spec-version-log.md`.

## Deployment

- Operators run Docker. `deployment/Dockerfile` builds `pilier-node`; `deployment/docker-compose.template.yml` runs a validator against `--chain=pilier_testnet`.
- `deployment/setup-validator.sh` interactively writes a `.env` file for the compose template.
- `ops/testnet/Dockerfile` is a separate operations image.

Exposed ports: `30333` (p2p), `9944` (RPC), `9615` (Prometheus).

## Authoritative context

- `/Users/laptop/Dev/pilier/devops` — the shared project knowledge base (Obsidian vault), including plans, decisions, and operational context for the broader Pilier project.
- `CLAUDE.md` — full project orientation, command reference, and conventions.
- `ai/decisions/genesis-build-order.md` — why `ValidatorSet` must precede `Session`.
- `ai/decisions/spec-version-log.md` — the meaning of each `spec_version` bump.
- `ai/plans/runtime-mutable-validator-set.md` — the plan that introduced `ValidatorSet`, `Session`, and the validator council.
