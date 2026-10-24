#!/usr/bin/env bash
# Build the Kestrl macOS app: kestrl-ffi as an XCFramework, the Xcode project, then the app.
# Prints the path of the Debug .app when it is done.
set -euo pipefail

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
step "Building Kestrl (Debug)"
xcodebuild -project "$ROOT/app/Kestrl.xcodeproj" -scheme Kestrl -configuration Debug \
    -derivedDataPath "$DERIVED" -quiet build

APP="$DERIVED/Build/Products/Debug/Kestrl.app"
step "Built"
echo "$APP"
