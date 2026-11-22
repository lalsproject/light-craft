//! The one `unsafe` item of the Android app. `android-activity` finds the app by looking up the
//! symbol `android_main` in the library, so it must keep its name (`#[unsafe(no_mangle)]`);
//! everything it does is in safe code (`host`).
#![allow(unsafe_code)]

use winit::platform::android::activity::AndroidApp;

// SAFETY: `android_main` is the symbol `android-activity` loads the app through; nothing else in the library
// is named like it, and the function itself does only safe things.
#[unsafe(no_mangle)]
fn android_main(app: AndroidApp) {
    crate::host::run(app);
}
