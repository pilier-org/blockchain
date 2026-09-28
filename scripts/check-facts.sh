#!/usr/bin/env bash
# Fails when a runtime hint file states the chain's spec_version number or lists its pallets
# with their fixed indices, instead of pointing at runtime/src/lib.rs — the one place either
# fact is actually declared (ANC-11). Both have drifted out of step with the source before.
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
Usage: check-facts.sh [file ...]

Checks each given file for two things that must never be duplicated there: a spec_version
number, and a pallet list carrying fixed indices. Both belong only in runtime/src/lib.rs's own
VERSION constant and #[frame_support::runtime] block.

With no arguments, checks blockchain/CLAUDE.md and blockchain/AGENTS.md relative to the
repository root, plus the "Runtime facts that are easy to get wrong" section of
~/Dev/pilier/AGENTS.md when that file exists on this machine — it lives outside every git
repository, so a continuous-integration checkout never has it, and this script skips it with a
one-line note in that case rather than failing.

Exit codes: 0 none of the checked files or sections duplicate either fact; 1 at least one does,
or a named file does not exist.
USAGE
}

if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
  usage
  exit 0
fi

# version_pattern matches a spec_version number written out in prose, e.g. "spec_version,
# currently 104" or "spec_version: 105" or "VERSION block (currently `104`)".
version_pattern='spec_version[^0-9]{0,60}[0-9]{2,}|currently `?[0-9]{2,}`?[^a-zA-Z]'
# index_pattern matches a pallet-index table or list: a `#[runtime::pallet_index(N)]`
# attribute, a markdown table row starting with a bare number, or prose naming an index range.
index_pattern='pallet_index\(|^\| *[0-9]+ *\||[Ii]ndices? \(?[0-9]+.[0-9]+\)?:|index order [0-9]+ to [0-9]+'

fail=0

check_text() {
  local label="$1" text="$2"
  if [ -z "$text" ]; then
    return
  fi
  if grep -qEi "$version_pattern" <<<"$text"; then
    echo "FAIL: $label states a spec_version number; point at runtime/src/lib.rs's VERSION block instead" >&2
    fail=1
  fi
  if grep -qE "$index_pattern" <<<"$text"; then
    echo "FAIL: $label lists pallets with their fixed indices; point at runtime/src/lib.rs's #[frame_support::runtime] block instead" >&2
    fail=1
  fi
}

if [ "$#" -gt 0 ]; then
  for f in "$@"; do
    if [ ! -f "$f" ]; then
      echo "FAIL: $f does not exist" >&2
      fail=1
      continue
    fi
    check_text "$f" "$(cat "$f")"
  done
  exit "$fail"
fi

repo_root="$(git rev-parse --show-toplevel)"
for f in "$repo_root/CLAUDE.md" "$repo_root/AGENTS.md"; do
  if [ ! -f "$f" ]; then
    echo "FAIL: $f does not exist" >&2
    fail=1
    continue
  fi
  check_text "$f" "$(cat "$f")"
done

# The third hint file lives outside every git repository, on whichever machine reads it, and is
# therefore absent from a continuous-integration checkout by construction; check it only when it
# is actually present, and skip it with a note otherwise.
workspace_agents="$HOME/Dev/pilier/AGENTS.md"
if [ -f "$workspace_agents" ]; then
  section="$(awk '/^### Runtime facts that are easy to get wrong/{flag=1; next} /^### /{flag=0} flag' "$workspace_agents")"
  check_text "$workspace_agents (Runtime facts section)" "$section"
else
  echo "note: $workspace_agents not found on this machine; skipping its own check" >&2
fi

exit "$fail"
