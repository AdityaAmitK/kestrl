#!/usr/bin/env bash
# Build kestrl-ffi into app/Frameworks/KestrlFFI.xcframework and generate
# app/Generated/kestrl_ffi.swift. Safe to re-run: outputs are replaced.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

# rustup installs here; make it findable outside a login shell.
export PATH="$HOME/.cargo/bin:$PATH"

LIB=libkestrl_ffi.a
TARGETS=(aarch64-apple-darwin x86_64-apple-darwin)
WORK="$ROOT/target/xcframework"
FRAMEWORK="$ROOT/app/Frameworks/KestrlFFI.xcframework"
SWIFT_OUT="$ROOT/app/Generated/kestrl_ffi.swift"

step() { printf '\n==> %s\n' "$*"; }

step "Installing Rust targets: ${TARGETS[*]}"
rustup target add "${TARGETS[@]}"

step "Building uniffi-bindgen"
cargo build --release --locked -p kestrl-ffi --features bindgen --bin uniffi-bindgen
BINDGEN="$ROOT/target/release/uniffi-bindgen"

for target in "${TARGETS[@]}"; do
    step "Building kestrl-ffi for $target (release)"
    cargo build --release --locked -p kestrl-ffi --target "$target"
done

step "Creating universal static library"
mkdir -p "$WORK/universal"
lipo -create \
    "$ROOT/target/${TARGETS[0]}/release/$LIB" \
    "$ROOT/target/${TARGETS[1]}/release/$LIB" \
    -output "$WORK/universal/$LIB"
echo "wrote $WORK/universal/$LIB"

step "Generating Swift bindings"
GENERATED="$WORK/bindings"
rm -rf "$GENERATED"
"$BINDGEN" generate --library "$ROOT/target/${TARGETS[0]}/release/$LIB" \
    --language swift --out-dir "$GENERATED"

HEADERS="$WORK/headers"
rm -rf "$HEADERS"
mkdir -p "$HEADERS"
cp "$GENERATED/kestrl_ffiFFI.h" "$HEADERS/"
cp "$GENERATED/kestrl_ffiFFI.modulemap" "$HEADERS/module.modulemap"

step "Creating $FRAMEWORK"
rm -rf "$FRAMEWORK"
mkdir -p "$(dirname "$FRAMEWORK")"
xcodebuild -create-xcframework \
    -library "$WORK/universal/$LIB" \
    -headers "$HEADERS" \
    -output "$FRAMEWORK"

step "Copying Swift bindings to $SWIFT_OUT"
mkdir -p "$(dirname "$SWIFT_OUT")"
cp "$GENERATED/kestrl_ffi.swift" "$SWIFT_OUT"

step "Done"
find "$FRAMEWORK" "$SWIFT_OUT" | sed "s|$ROOT/||"
