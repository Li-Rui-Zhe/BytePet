#!/usr/bin/env bash
set -euo pipefail
if [[ "$(uname -s)" != Darwin ]]; then
  echo 'Run this bundle verifier on macOS, not Windows/Linux.' >&2
  exit 1
fi
target="${1:?target triple required}"
arch="${2:?arm64 or x86_64 required}"
case "$target:$arch" in
  aarch64-apple-darwin:arm64|x86_64-apple-darwin:x86_64) ;;
  *) echo 'Target and CPU architecture do not match.' >&2; exit 1 ;;
esac
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
app="$root/src-tauri/target/$target/release/bundle/macos/BytePet.app"
info="$app/Contents/Info.plist"
test -d "$app"
plutil -lint "$info"
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$info")" == com.bytepet.app ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :LSUIElement' "$info")" == true ]]
[[ "$(/usr/libexec/PlistBuddy -c 'Print :LSMinimumSystemVersion' "$info")" == 13.3 ]]
exe="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$info")"
[[ "$exe" != */* && -n "$exe" ]]
binary="$app/Contents/MacOS/$exe"
test -x "$binary"
lipo "$binary" -verify_arch "$arch"
codesign --verify --deep --strict --verbose=2 "$app"
version="$(node -p 'JSON.parse(require("fs").readFileSync("package.json","utf8")).version')"
[[ "$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$info")" == "$version" ]]
out="$root/artifacts/macos-$arch"
mkdir -p "$out"
pid=''
cleanup() { if [[ -n "$pid" ]]; then kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; fi; }
trap cleanup EXIT
# Launch the packaged executable, not a source-tree `tauri dev` instance.
# CI uses fresh local data and no API keys. No network chat/test calls are made.
"$binary" >"$out/startup.log" 2>&1 &
pid=$!
sleep 8
if ! kill -0 "$pid" 2>/dev/null; then
  echo 'Packaged app exited during the native startup smoke test.' >&2
  cat "$out/startup.log"
  exit 1
fi
cleanup
pid=''
shopt -s nullglob
dmgs=("$root/src-tauri/target/$target/release/bundle/dmg/"*.dmg)
[[ ${#dmgs[@]} -eq 1 ]]
hdiutil verify "${dmgs[0]}"
cp "${dmgs[0]}" "$out/BytePet_${version}_${arch}.dmg"
# upload-artifact does not preserve Unix permissions for loose .app directories.
ditto -c -k --sequesterRsrc --keepParent "$app" "$out/BytePet_${version}_${arch}.app.zip"
(
  cd "$out"
  shasum -a 256 ./*.dmg ./*.zip > SHA256SUMS.txt
)
if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
  {
    echo "### BytePet $version — $arch"
    echo '- Build, architecture, ad-hoc signature, native startup and DMG checks passed.'
    echo '- Download the DMG or permission-preserving .app.zip from this run’s Artifacts.'
    echo '- Requires macOS 13.3+. These are ad-hoc test packages, not Apple-notarized releases.'
  } >> "$GITHUB_STEP_SUMMARY"
fi
