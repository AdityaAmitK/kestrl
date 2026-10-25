#!/bin/sh
# Render Homebrew formula and cask files from an existing GitHub release.
set -eu

if [ "$#" -ne 1 ]; then
    echo "Usage: $0 <version>" >&2
    exit 2
fi

version=${1#v}
repo_root=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d "${TMPDIR:-/tmp}/kestrl-homebrew.XXXXXX")
trap 'rm -rf "$tmp"' EXIT HUP INT TERM

gh release download "v$version" --repo AdityaAmitK/kestrl \
    --pattern "kestrl-$version-macos-universal.tar.gz.sha256" \
    --pattern "Kestrl-$version.dmg.sha256" --dir "$tmp"

cli_sha=$(awk 'NR == 1 { print $1 }' "$tmp/kestrl-$version-macos-universal.tar.gz.sha256")
app_sha=$(awk 'NR == 1 { print $1 }' "$tmp/Kestrl-$version.dmg.sha256")
[ -n "$cli_sha" ] && [ -n "$app_sha" ] || { echo "Could not read release checksums" >&2; exit 1; }

mkdir -p "$repo_root/packaging/homebrew/rendered/Formula" "$repo_root/packaging/homebrew/rendered/Casks"
sed -e "s/VERSION/$version/g" -e "s/SHA256/$cli_sha/g" \
    "$repo_root/packaging/homebrew/Formula/kestrl.rb" \
    > "$repo_root/packaging/homebrew/rendered/Formula/kestrl.rb"
sed -e "s/VERSION/$version/g" -e "s/SHA256/$app_sha/g" \
    "$repo_root/packaging/homebrew/Casks/kestrl.rb" \
    > "$repo_root/packaging/homebrew/rendered/Casks/kestrl.rb"

printf 'Copy these files into a checkout of AdityaAmitK/homebrew-tap:\n'
printf '  cp %s/packaging/homebrew/rendered/Formula/kestrl.rb <tap>/Formula/kestrl.rb\n' "$repo_root"
printf '  cp %s/packaging/homebrew/rendered/Casks/kestrl.rb <tap>/Casks/kestrl.rb\n' "$repo_root"
