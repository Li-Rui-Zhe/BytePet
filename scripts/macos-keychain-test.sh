#!/usr/bin/env bash
set -euo pipefail
if [[ "$(uname -s)" != Darwin || "${CI:-}" != true ]]; then
  echo 'This isolated keychain test is intended only for a macOS CI runner.' >&2
  exit 1
fi
keychain="${RUNNER_TEMP:?}/bytepet-test-$(uuidgen).keychain-db"
password="$(uuidgen)"
echo "::add-mask::$password"
previous="$(security default-keychain -d user | sed 's/^[[:space:]]*"//;s/"[[:space:]]*$//')"
cleanup() {
  security default-keychain -d user -s "$previous" || true
  security delete-keychain "$keychain" || true
}
trap cleanup EXIT
security create-keychain -p "$password" "$keychain"
security unlock-keychain -p "$password" "$keychain"
security set-keychain-settings -lut 3600 "$keychain"
security default-keychain -d user -s "$keychain"
cargo test --locked --manifest-path src-tauri/Cargo.toml --target "${TARGET:?}" native_credential_round_trip -- --ignored
