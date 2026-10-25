#!/usr/bin/env bash
set -euo pipefail
[[ "${CI:-}" == true && -n "${GH_TOKEN:-}" && -n "${GH_REPO:-}" ]]
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
version="$(node -p 'JSON.parse(require("fs").readFileSync("package.json","utf8")).version')"
tag="${RELEASE_TAG:?release tag required}"
[[ "$tag" == "v$version" ]]
notes="docs/releases/$tag.md"
test -s "$notes"
assets=(artifacts/release/*)
[[ ${#assets[@]} -eq 4 ]]

# Upload all packages to a draft first; a partial upload must not become public.
if existing="$(gh release view "$tag" --json isDraft --jq '.isDraft' 2>/dev/null)"; then
  if [[ "$existing" != true ]]; then
    echo "Release $tag is already published; leaving it unchanged."
    exit 0
  fi
  gh release edit "$tag" --title "BytePet $tag" --notes-file "$notes"
  gh release upload "$tag" "${assets[@]}" --clobber
else
  gh release create "$tag" "${assets[@]}" --verify-tag --draft --title "BytePet $tag" --notes-file "$notes"
fi
gh release edit "$tag" --draft=false --prerelease=false --latest
if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
  echo "Published BytePet $tag: https://github.com/$GH_REPO/releases/tag/$tag" >> "$GITHUB_STEP_SUMMARY"
fi
