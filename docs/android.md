# LightCraft on Android

`apps/lightcraft-android` runs LightCraft on **Android 9 and later (API 28+)**, built for **ARMv7**
(`armv7-linux-androideabi`, ABI `armeabi-v7a`). It is the desktop UI (`lightcraft-ui-egui`) in a
`NativeActivity`, still pure Rust: no Java or Kotlin of our own.

> **Status: builds for the target, not yet run on a device.** `cargo check`, clippy and the host-side
> tests pass for `armv7-linux-androideabi`; the APK packaging (`cargo apk`) and everything that needs a
> phone (renderer, storage permission, memory on 32-bit) are untested. Treat the first run as a
> test run and read `adb logcat -s LightCraft`.

## Build

```sh
export ANDROID_HOME=~/Android/Sdk                 # platform 28 and build-tools installed
export ANDROID_NDK_ROOT=$ANDROID_HOME/ndk/26.3.11579264
packaging/android/build.sh debug armv7            # APK signed with the debug key
packaging/android/build.sh release arm64          # release; sign it, see below
adb install -r target/lightcraft-armv7-linux-androideabi-debug.apk
```

The architectures are `armv7` (armeabi-v7a), `arm64` (arm64-v8a), `x86_64`, `x86` and `all`.
A release APK is signed with the key named by `CARGO_APK_RELEASE_KEYSTORE` and
`CARGO_APK_RELEASE_KEYSTORE_PASSWORD` (or `[package.metadata.android.signing.release]` in your own
checkout, never in the repository).

### GitHub Actions

**Actions ▸ Android APK ▸ Run workflow** (`.github/workflows/android.yml`) builds from a menu:

| Input | Choices |
|---|---|
| Architecture | `armv7`, `arm64`, `x86_64`, `x86`, `all` (one job and one APK per architecture) |
| Build type | `release`, `debug` |
| Release tag | optional: also publishes the APKs as a pre-release with this tag |

The APKs are the run's artifacts. A release build is signed with the key in the repository secrets
`ANDROID_KEYSTORE_BASE64` (the keystore, base64) and `ANDROID_KEYSTORE_PASSWORD`; without them it uses a
throwaway key made for that run (fine for testing, but an update signed with another key does not
install over it). The run's summary says which one signed each APK.

## The phone layout

The app opens in a Lightroom-Mobile-style layout (`lightcraft_ui_egui::panels::mobile`, switched on by
`LightcraftApp::mobile`), made of the desktop's own panels:

- **Library:** a top bar (folders menu, title and count, filter, add), the photos in three columns, and
  a bottom bar (Library, Edit, Settings). A tap opens a photo. The menu opens a drawer with the desktop's
  folders, albums and **Local**, which is how photos come in on a phone.
- **Edit:** back, the file name, undo and redo on top; the photo; a sheet with the open tool's controls;
  and a scrolling strip of tools under it (Presets, Light, Color, Effects, Detail, Optics, Crop, Masking,
  Remove, Info). Tap the open tool again, or the chevron on the sheet, to close it.

`lightcraft-cli snapshot --demo --mobile --size 390x844 --scale 2 -o phone.png` renders it without a
phone. Not done yet: long-press to select several photos, swipe between photos, pinch to zoom, and a
layout for landscape and tablets.

## How it differs from the desktop app

| | Desktop | Android |
|---|---|---|
| Folders | `~/Pictures/LightCraft Library`, `~/.config/lightcraft` | settings and logs in the app's private folder; the library in `Android/data/<package>/files/LightCraft Library` (private folder if there is none). `lightcraft_engine::config::set_data_root` |
| Photos | system file dialog | none: **Local** browses Camera (`DCIM`), Pictures and Downloads on shared storage |
| Window | wgpu (Vulkan/Metal/DX12), glow as the fallback | Vulkan when an adapter that is not a software one answers, else glow (OpenGL ES); a marker file in the config folder sends a renderer that crashed to the end of the line |
| GPU compute | on | **off** on the first start; Settings ▸ GPU turns it on |
| AI masks (SAM 3) | optional | not built in: the model does not fit a 32-bit address space |
| Settings file | `ui.json`, written when the app quits and on a timer | `ui.json`, written every few seconds when it changed (Android can end the process without notice) |

## Storage permission

The manifest asks for `READ_EXTERNAL_STORAGE` and `WRITE_EXTERNAL_STORAGE` and targets API 28, so on
Android 10 and later the classic storage model applies once the permission is granted. The app does
**not** yet show the system permission prompt (that needs a JNI call): grant *Storage* in Settings ▸
Apps ▸ LightCraft ▸ Permissions. Until then the app says so in a notice. Android 14 and 15 warn that an
app targeting API 28 was built for an older version; it still installs and runs. The Play Store does not
accept such an app, so this build is for sideloading.

## Known limits and next steps

- **32-bit memory.** A process on ARMv7 gets 2–3 GB of address space at most, and phones kill apps well
  before that. Large RAWs and panorama or HDR merges may not fit. The engine's memory budget reads
  `/proc/meminfo`; it has not been tuned for Android.
- **No runtime permission prompt**, no system file/folder picker (`ACTION_OPEN_DOCUMENT`), no "Open in"
  or share intents, no clipboard and no screen-reader support (arboard and accesskit have no Android
  backend). Each needs JNI or Java glue.
- **Touch.** The UI is the desktop layout; it takes touch input through egui but is not adapted to a
  phone-sized screen.
- **Packaging unverified.** The `[package.metadata.android]` keys follow `cargo-apk`'s documentation and
  have not been run through `cargo apk build` yet.
