//! Where the Android app keeps and finds things. Pure functions of the folders Android reports, so
//! they run in the desktop tests too.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Folder name of the library, as on the desktop.
pub const LIBRARY_NAME: &str = "LightCraft Library";

/// The app's own folders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    /// Settings, models and presets: `<private>/config` (the engine's data root, see
    /// `lightcraft_engine::config::set_data_root`).
    pub config: PathBuf,
    /// The saved UI state and app settings.
    pub prefs: PathBuf,
    /// Log and panic files.
    pub logs: PathBuf,
    /// The library: in the app's folder on shared storage when Android gave one (more room than the
    /// private folder, and the user can copy it off), else in the private folder.
    pub library: PathBuf,
}

/// `internal` is the app's private data folder, `external` its folder on shared storage
/// (`Android/data/<package>/files`, needs no permission) if there is one.
pub fn dirs(internal: &Path, external: Option<&Path>) -> Dirs {
    let config = internal.join("config");
    Dirs { prefs: config.join("ui.json"), config, logs: internal.join("logs"), library: external.unwrap_or(internal).join(LIBRARY_NAME) }
}

/// The root of the primary shared storage: `$EXTERNAL_STORAGE` when Android sets it, else the
/// usual path.
pub fn shared_root(external_storage: Option<OsString>) -> PathBuf {
    external_storage.filter(|p| !p.is_empty()).map_or_else(|| PathBuf::from("/storage/emulated/0"), PathBuf::from)
}

/// The photo folders of shared storage as (English name, path), those `is_dir` accepts. The names
/// go through the translation system.
pub fn shared_places(root: &Path, is_dir: impl Fn(&Path) -> bool) -> Vec<(&'static str, PathBuf)> {
    [("Camera", "DCIM"), ("Pictures", "Pictures"), ("Downloads", "Download")]
        .into_iter()
        .map(|(name, sub)| (name, root.join(sub)))
        .filter(|(_, p)| is_dir(p))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_goes_to_shared_storage_when_there_is_a_folder_there() {
        let d = dirs(Path::new("/data/user/0/app/files"), Some(Path::new("/storage/emulated/0/Android/data/app/files")));
        assert!(d.library.starts_with("/storage/emulated/0/Android/data/app/files"));
        assert!(d.config.starts_with("/data/user/0/app/files"));
        assert!(d.prefs.starts_with(&d.config));
        let d = dirs(Path::new("/data/user/0/app/files"), None);
        assert!(d.library.starts_with("/data/user/0/app/files"));
    }

    #[test]
    fn shared_root_falls_back_when_unset_or_empty() {
        assert_eq!(shared_root(Some("/sdcard".into())), PathBuf::from("/sdcard"));
        for v in [None, Some(OsString::new())] {
            assert_eq!(shared_root(v), PathBuf::from("/storage/emulated/0"));
        }
    }

    #[test]
    fn only_existing_photo_folders_are_places() {
        let places = shared_places(Path::new("/s"), |p| p.ends_with("DCIM") || p.ends_with("Download"));
        let names: Vec<_> = places.iter().map(|(n, _)| *n).collect();
        assert_eq!(names, ["Camera", "Downloads"]);
    }
}
