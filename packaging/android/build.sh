#!/usr/bin/env bash
# Builds the LightCraft APK for Android 9+ (API 28).
#   packaging/android/build.sh [release|debug] [armv7|arm64|x86_64|x86|all]
#   packaging/android/build.sh                 # release, armv7 (armeabi-v7a)
#   packaging/android/build.sh debug arm64     # debug APK for 64-bit ARM, signed with the debug key
# The same architectures are the menu of the "Android APK" GitHub workflow.
# Needs: Rust (rustup), the Android SDK (platform 28, build-tools) and NDK r26+.
# Set ANDROID_HOME and ANDROID_NDK_ROOT (or ANDROID_NDK_HOME) before running.
set -euo pipefail
cd "$(dirname "$0")/../.."

mode="${1:-release}"
arch="${2:-${ANDROID_ARCH:-armv7}}"

case "$arch" in
  armv7)  targets=(armv7-linux-androideabi) ;;
  arm64)  targets=(aarch64-linux-android) ;;
  x86_64) targets=(x86_64-linux-android) ;;
  x86)    targets=(i686-linux-android) ;;
  all)    targets=(armv7-linux-androideabi aarch64-linux-android x86_64-linux-android i686-linux-android) ;;
  *) echo "unknown architecture '$arch' (armv7, arm64, x86_64, x86 or all)" >&2; exit 1 ;;
esac
case "$mode" in release|debug) ;; *) echo "unknown mode '$mode' (release or debug)" >&2; exit 1 ;; esac

[[ -n "${ANDROID_HOME:-}" ]] || { echo "ANDROID_HOME is not set (the Android SDK folder)" >&2; exit 1; }
if [[ -z "${ANDROID_NDK_ROOT:-}" && -z "${ANDROID_NDK_HOME:-}" ]]; then
  echo "ANDROID_NDK_ROOT is not set (the NDK folder, r26 or newer)" >&2
  exit 1
fi
export ANDROID_NDK_ROOT="${ANDROID_NDK_ROOT:-$ANDROID_NDK_HOME}"
export ANDROID_NDK_HOME="$ANDROID_NDK_ROOT"

command -v cargo-apk >/dev/null || cargo install cargo-apk --locked

for target in "${targets[@]}"; do
  rustup target add "$target"
  args=(apk build --manifest-path apps/lightcraft-android/Cargo.toml --target "$target")
  [[ "$mode" == "release" ]] && args+=(--release)
  cargo "${args[@]}"
  # cargo-apk writes the same file name for every target: keep one copy per architecture
  apk="$(find "target/$mode/apk" -name '*.apk' | head -n 1)"
  out="target/lightcraft-$target-$mode.apk"
  cp "$apk" "$out"
  echo "$out"
done
