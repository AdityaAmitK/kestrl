#!/usr/bin/env bash
# Build the Kestrl macOS app: kestrl-ffi as an XCFramework, the Xcode project, then the app.
# Pass --release to build the Release .app; defaults to Debug.
set -euo pipefail

CONFIGURATION=Debug
if [[ "${1:-}" == "--release" ]]; then
    CONFIGURATION=Release
    shift
fi
if (( $# != 0 )); then
    echo "Usage: $0 [--release]" >&2
    exit 2
fi

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# rustup and Homebrew tools live outside a login shell's PATH.
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:$PATH"
# Compile Rust and C objects for the app's minimum macOS, so the linker stays quiet.
export MACOSX_DEPLOYMENT_TARGET=15.0

step() { printf '\n==> %s\n' "$*"; }

step "Building the kestrl-ffi XCFramework"
bash "$ROOT/scripts/build-xcframework.sh"

step "Generating the Xcode project"
xcodegen generate --spec "$ROOT/app/project.yml"

DERIVED="$ROOT/target/app"
step "Building Kestrl ($CONFIGURATION)"
xcodebuild -project "$ROOT/app/Kestrl.xcodeproj" -scheme Kestrl -configuration "$CONFIGURATION" \
    -derivedDataPath "$DERIVED" -quiet build

APP="$DERIVED/Build/Products/$CONFIGURATION/Kestrl.app"
step "Built"
echo "$APP"
