#!/usr/bin/env bash
# Checks the two things that, out of step with each other, break a local build before it even
# starts (ANC-11): the active Rust compiler channel against the one pinned in
# rust-toolchain.toml, and whether the wasm32-unknown-unknown target is installed for it.
set -euo pipefail

if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
  cat >&2 <<'USAGE'
Usage: preflight.sh

Run from anywhere inside the repository, before a build. Checks exactly two things and nothing
else: that the active Rust compiler matches the channel pinned in rust-toolchain.toml, and that
the wasm32-unknown-unknown target is installed for it. Exit codes: 0 both hold; 1 either does
not, or rustup/cargo is not on PATH.
USAGE
  exit 0
fi

repo_root="$(git rev-parse --show-toplevel)"
toolchain_file="$repo_root/rust-toolchain.toml"

if [ ! -f "$toolchain_file" ]; then
  echo "FAIL: $toolchain_file does not exist" >&2
  exit 1
fi

pinned="$(sed -n 's/^channel[[:space:]]*=[[:space:]]*"\(.*\)"/\1/p' "$toolchain_file")"
if [ -z "$pinned" ]; then
  echo "FAIL: $toolchain_file declares no channel" >&2
  exit 1
fi

if ! command -v rustc >/dev/null 2>&1; then
  echo "FAIL: rustc is not on PATH" >&2
  exit 1
fi

active="$(rustc --version | awk '{print $2}')"

fail=0

if [ "$active" != "$pinned" ]; then
  echo "FAIL: active compiler is $active, pinned is $pinned ($toolchain_file)" >&2
  fail=1
else
  echo "OK: active compiler matches the pinned channel ($active)"
fi

if ! command -v rustup >/dev/null 2>&1; then
  echo "FAIL: rustup is not on PATH, cannot check installed targets" >&2
  exit 1
fi

if rustup target list --installed | grep -qx 'wasm32-unknown-unknown'; then
  echo "OK: wasm32-unknown-unknown target is installed"
else
  echo "FAIL: wasm32-unknown-unknown target is not installed for the active toolchain" >&2
  fail=1
fi

exit "$fail"
