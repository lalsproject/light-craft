#!/usr/bin/env bash
# Builds the LightCraft APK for Android 9+ (API 28) on ARMv7 (armeabi-v7a).
#   packaging/android/build.sh            # release APK (signed with the key named in the environment, see docs/android.md)
#   packaging/android/build.sh debug      # debug APK, signed with the debug key
# Needs: Rust (rustup), the Android SDK (platform 28+, build-tools) and NDK r26+.
# Set ANDROID_HOME and ANDROID_NDK_ROOT (or ANDROID_NDK_HOME) before running.
set -euo pipefail
cd "$(dirname "$0")/../.."

mode="${1:-release}"
target="${ANDROID_TARGET:-armv7-linux-androideabi}"

[[ -n "${ANDROID_HOME:-}" ]] || { echo "ANDROID_HOME is not set (the Android SDK folder)" >&2; exit 1; }
if [[ -z "${ANDROID_NDK_ROOT:-}" && -z "${ANDROID_NDK_HOME:-}" ]]; then
  echo "ANDROID_NDK_ROOT is not set (the NDK folder, r26 or newer)" >&2
  exit 1
fi
export ANDROID_NDK_ROOT="${ANDROID_NDK_ROOT:-$ANDROID_NDK_HOME}"
export ANDROID_NDK_HOME="$ANDROID_NDK_ROOT"

rustup target add "$target"
command -v cargo-apk >/dev/null || cargo install cargo-apk --locked

args=(apk build -p lightcraft-android --target "$target")
[[ "$mode" == "release" ]] && args+=(--release)
cargo "${args[@]}"

find target -name 'lightcraft-android*.apk' -newer packaging/android/build.sh -print
