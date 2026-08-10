#!/usr/bin/env bash
# Build the Android APK via Gradle (assumes axon binary + config.yaml already
# placed in android/apk/src/main/assets/ by cross-android.sh or CI).
set -euo pipefail
cd "$(dirname "$0")/../android/apk"
gradle assembleRelease
echo "APK: build/outputs/apk/release/axon-release-unsigned.apk"
