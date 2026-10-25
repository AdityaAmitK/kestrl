#!/bin/sh
# Install the standalone Kestrl CLI from the latest GitHub release.
set -eu

REPOSITORY=AdityaAmitK/kestrl
API=https://api.github.com/repos/$REPOSITORY/releases

case "$(uname -s)" in
    Darwin) ;;
    *) echo "kestrl install: macOS is required" >&2; exit 1 ;;
esac

if [ -n "${KESTRL_VERSION:-}" ]; then
    version=${KESTRL_VERSION#v}
else
    release_json=$(curl -fsSL "$API/latest") || {
        echo "kestrl install: could not resolve the latest GitHub release" >&2
        exit 1
    }
    # Extract the version tag without jq; GitHub's tag_name is a JSON string.
    tag=$(printf '%s\n' "$release_json" | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -n 1)
    [ -n "$tag" ] || { echo "kestrl install: latest release response has no tag_name" >&2; exit 1; }
    version=${tag#v}
fi

asset=kestrl-$version-macos-universal.tar.gz
base=https://github.com/$REPOSITORY/releases/download/v$version
tmp=$(mktemp -d "${TMPDIR:-/tmp}/kestrl-install.XXXXXX")
trap 'rm -rf "$tmp"' EXIT HUP INT TERM
curl -fsSL "$base/$asset" -o "$tmp/$asset"
curl -fsSL "$base/$asset.sha256" -o "$tmp/$asset.sha256"

expected=$(awk 'NR == 1 { print $1 }' "$tmp/$asset.sha256")
actual=$(shasum -a 256 "$tmp/$asset" | awk '{ print $1 }')
[ -n "$expected" ] && [ "$expected" = "$actual" ] || {
    echo "kestrl install: SHA-256 verification failed" >&2
    exit 1
}

install_dir=${KESTRL_INSTALL_DIR:-$HOME/.local/bin}
mkdir -p "$install_dir"
tar -xzf "$tmp/$asset" -C "$tmp" kestrl
install -m 755 "$tmp/kestrl" "$install_dir/kestrl"

case ":$PATH:" in
    *:"$install_dir":*) ;;
    *)
        # Single quotes keep $PATH literal so the hint can be pasted as is.
        printf 'Add %s to your PATH, for example:\n  export PATH="%s:$PATH"\n' "$install_dir" "$install_dir"
        ;;
esac
"$install_dir/kestrl" --version
