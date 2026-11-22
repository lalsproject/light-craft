//! LightCraft for Android 9 and later (API 28+), built for `armv7-linux-androideabi` (32-bit ARM)
//! and the other Android ABIs.
//!
//! The app is the desktop UI (`lightcraft-ui-egui`) in a `NativeActivity`: `android_main` (the only
//! `unsafe` item, in `entry`) starts eframe. What differs from the desktop app:
//!
//! - **Folders.** Android has no home folder. Settings and logs live in the app's private data
//!   folder, the library in its folder on shared storage when there is one (see [`paths`]).
//! - **Photos.** There is no system file dialog: Local in the sidebar browses Camera, Pictures and
//!   Downloads on shared storage (the permission is granted in Settings ▸ Apps ▸ LightCraft).
//! - **Window.** Vulkan when the phone has a usable one, else glow (OpenGL ES): winit cannot create
//!   its event loop twice, so the renderer is chosen before the window opens.
//! - **GPU compute** starts off (Settings ▸ GPU turns it on) and AI masks (SAM 3, too big for a
//!   32-bit address space) are not offered.
//!
//! See `docs/android.md` for building and the known limits.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod paths;
pub mod prefs;

#[cfg(target_os = "android")]
mod entry;
#[cfg(target_os = "android")]
mod host;
