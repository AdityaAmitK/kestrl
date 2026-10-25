#!/usr/bin/env bash
# Deploy site/ to kestrl.adityakinjawadekar.com.
# Stylesheet and script links get a content-hash query (style.css?v=abc123), so browsers
# and Cloudflare never pair new pages with an old cached stylesheet.
# Usage: scripts/deploy-site.sh   (needs SSH access; KESTRL_DEPLOY_KEY overrides the key path)
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HOST="root@187.127.129.8"
KEY="${KESTRL_DEPLOY_KEY:-$HOME/hostinger_vps}"
TARGET="/var/www/kestrl/"

BUILD="$(mktemp -d "${TMPDIR:-/tmp}/kestrl-site.XXXXXX")"
trap 'rm -r "$BUILD"' EXIT

rsync -a --exclude README.md "$ROOT/site/" "$BUILD/"

css_version="$(shasum -a 256 "$BUILD/style.css" | cut -c1-10)"
js_version="$(shasum -a 256 "$BUILD/script.js" | cut -c1-10)"
find "$BUILD" -name '*.html' -print0 | xargs -0 sed -i '' \
    -e "s|style\.css\"|style.css?v=$css_version\"|g" \
    -e "s|script\.js\"|script.js?v=$js_version\"|g"

rsync -az --delete -e "ssh -i $KEY" "$BUILD/" "$HOST:$TARGET"
echo "Deployed site (style.css?v=$css_version, script.js?v=$js_version)"
