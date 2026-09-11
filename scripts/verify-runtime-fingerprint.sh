#!/usr/bin/env bash
# Verifies that a runtime WebAssembly file about to be rolled out matches the fingerprint the
# continuous-integration run published for the same commit.
#
# The expected fingerprint can be supplied directly, so this script can be exercised without any
# network access. It reaches for the continuous-integration run only when neither form of the
# expected fingerprint is supplied.
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
Usage: verify-runtime-fingerprint.sh <runtime-wasm-path> [options]

Options:
  --expected <sha256>     the expected fingerprint, given directly (no network access)
  --expected-file <path>  a file whose first token is the expected fingerprint (no network access)
  --commit <sha>          the commit the check run was for (default: the current HEAD)
  --repo <owner/repo>     the GitHub repository to query (default: the origin remote)
  --workflow <file>       the workflow file whose run published the artifact (default: ci.yml)
  --artifact <name>       the artifact name to download (default: runtime-wasm)

Exactly one of --expected or --expected-file may be given. When neither is given, the expected
fingerprint is fetched from the most recent successful run of the named workflow for the named
commit, using the GitHub CLI (gh), which must already be authenticated.
USAGE
}

wasm_path=""
expected=""
expected_file=""
commit=""
repo=""
workflow="ci.yml"
artifact="runtime-wasm"

while [ $# -gt 0 ]; do
  case "$1" in
    --expected)
      expected="$2"
      shift 2
      ;;
    --expected-file)
      expected_file="$2"
      shift 2
      ;;
    --commit)
      commit="$2"
      shift 2
      ;;
    --repo)
      repo="$2"
      shift 2
      ;;
    --workflow)
      workflow="$2"
      shift 2
      ;;
    --artifact)
      artifact="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    -*)
      echo "unknown option: $1" >&2
      usage
      exit 2
      ;;
    *)
      if [ -z "$wasm_path" ]; then
        wasm_path="$1"
        shift
      else
        echo "unexpected extra argument: $1" >&2
        usage
        exit 2
      fi
      ;;
  esac
done

if [ -z "$wasm_path" ]; then
  echo "the runtime wasm path is required" >&2
  usage
  exit 2
fi

if [ ! -f "$wasm_path" ]; then
  echo "no such file: $wasm_path" >&2
  exit 2
fi

if [ -n "$expected" ] && [ -n "$expected_file" ]; then
  echo "give only one of --expected or --expected-file" >&2
  exit 2
fi

if [ -n "$expected_file" ]; then
  if [ ! -f "$expected_file" ]; then
    echo "no such file: $expected_file" >&2
    exit 2
  fi
  expected="$(awk '{print $1; exit}' "$expected_file")"
fi

if [ -z "$expected" ]; then
  # Reach for the check run only when no expected fingerprint was supplied directly.
  if ! command -v gh >/dev/null 2>&1; then
    echo "the GitHub CLI (gh) is required to look up the check run's own fingerprint" >&2
    exit 2
  fi
  if [ -z "$commit" ]; then
    commit="$(git rev-parse HEAD)"
  fi
  if [ -z "$repo" ]; then
    repo="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
  fi
  run_id="$(gh run list --repo "$repo" --workflow "$workflow" --commit "$commit" \
    --json databaseId,conclusion \
    --jq '[.[] | select(.conclusion == "success")][0].databaseId')"
  if [ -z "$run_id" ] || [ "$run_id" = "null" ]; then
    echo "no successful $workflow run found for commit $commit in $repo" >&2
    exit 1
  fi
  download_dir="$(mktemp -d)"
  gh run download "$run_id" --repo "$repo" --name "$artifact" --dir "$download_dir"
  fingerprint_file="$(find "$download_dir" -name '*.sha256' | head -n1)"
  if [ -z "$fingerprint_file" ]; then
    echo "no fingerprint file found in the $artifact artifact of run $run_id" >&2
    exit 1
  fi
  expected="$(awk '{print $1; exit}' "$fingerprint_file")"
fi

# GNU coreutils ships sha256sum; macOS ships shasum instead and has no sha256sum at all, and the
# operator who runs this before a rollout may be on either.
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "$wasm_path" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  actual="$(shasum -a 256 "$wasm_path" | awk '{print $1}')"
else
  echo "neither sha256sum nor shasum is available to compute the fingerprint" >&2
  exit 2
fi

if [ "$actual" = "$expected" ]; then
  echo "OK: $wasm_path matches the published fingerprint ($actual)"
  exit 0
else
  echo "MISMATCH: $wasm_path has fingerprint $actual, expected $expected" >&2
  exit 1
fi
