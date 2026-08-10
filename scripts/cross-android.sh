#!/usr/bin/env bash
# Cross-compile axon-server for Android (aarch64-linux-android, arm64-v8a).
# Prerequisites: Android NDK + cargo-ndk installed.
#   rustup target add aarch64-linux-android
#   cargo install cargo-ndk
set -euo pipefail

TARGET=aarch64-linux-android
PLATFORM=${1:-21}
BIN_NAME=axon
OUT_DIR="$(dirname "$0")/../android/apk/src/main/assets"

echo "[cross-android] target=$TARGET platform=$PLATFORM"

if ! command -v cargo-ndk >/dev/null 2>&1; then
  echo "[cross-android] cargo-ndk not found; installing..."
  cargo install cargo-ndk
fi

rustup target add "$TARGET" 2>/dev/null || true

cargo ndk --target "$TARGET" --platform "$PLATFORM" -- build --release -p axon-server

mkdir -p "$OUT_DIR"
cp "target/$TARGET/release/$BIN_NAME" "$OUT_DIR/axon"
chmod +x "$OUT_DIR/axon"

echo "[cross-android] done: $OUT_DIR/axon"
ls -lh "$OUT_DIR/axon"
