//! The saved UI state and app settings (`<config>/ui.json`): reading it back and writing it.
//! Android may end the process without a word, so the app saves while it runs, not only at exit.

use lightcraft_ui_egui::UiState;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How often a changed state is written while the app runs.
pub const SAVE_EVERY: Duration = Duration::from_secs(5);

/// The saved state, with a message for the user when the file exists but can't be used. A damaged
/// file is kept as `ui.json.corrupt-<unix time>` so the next save can't destroy it. The returned
/// flag is `true` when nothing may be written this session (the file could not be read or set aside).
pub fn load(path: &Path) -> (Option<UiState>, Option<String>, bool) {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (None, None, false),
        Err(e) => return (None, Some(format!("The app settings ({}) couldn't be read: {e}. Defaults are used.", path.display())), true),
    };
    match decode(&bytes) {
        Ok(ui) => (Some(ui.sanitized()), None, false),
        Err(e) => {
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            let keep = path.with_file_name(format!("ui.json.corrupt-{secs}"));
            let kept = std::fs::rename(path, &keep);
            let what = match &kept {
                Ok(()) => format!("it was kept as {}", keep.display()),
                Err(r) => format!("it couldn't be set aside ({r})"),
            };
            (None, Some(format!("The app settings ({}) are damaged ({e}); {what}. Defaults are used.", path.display())), kept.is_err())
        }
    }
}

/// The desktop app's decoding: an arrangement of panels from an older version starts again from the
/// default, and a file from before the start-up dialog counts as an installation that has been run.
pub fn decode(bytes: &[u8]) -> Result<UiState, String> {
    let mut saved: serde_json::Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let fields = saved.as_object_mut().ok_or("settings must be an object")?;
    lightcraft_ui_egui::docking::drop_outdated(fields);
    if fields.get("docking").is_some_and(|v| serde_json::from_value::<Option<lightcraft_ui_egui::docking::Workspace>>(v.clone()).is_err()) {
        fields.remove("docking");
    }
    lightcraft_ui_egui::startup::migrate_legacy_seen_version(fields);
    serde_json::from_value(saved).map_err(|e| e.to_string())
}

/// Write `bytes` to `path` atomically: a temp file, fsynced, renamed over it.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let tmp = path.with_extension("json.tmp");
    let written = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, path)
}

/// Saves the UI state when it changed, at most every [`SAVE_EVERY`], and on demand.
pub struct Writer {
    path: PathBuf,
    written: Vec<u8>,
    checked: Instant,
    /// Never write (the file couldn't be read at launch).
    keep_file: bool,
    failing: bool,
}

impl Writer {
    /// `current` is what the file holds now, so an unchanged state is not written again.
    pub fn new(path: PathBuf, current: &UiState, keep_file: bool) -> Self {
        Self { path, written: serde_json::to_vec_pretty(current).unwrap_or_default(), checked: Instant::now(), keep_file, failing: false }
    }

    /// Called every frame: writes when [`SAVE_EVERY`] has passed and the state changed.
    pub fn tick(&mut self, ui: &UiState, temporary: bool) {
        if self.checked.elapsed() < SAVE_EVERY {
            return;
        }
        self.checked = Instant::now();
        if let Err(e) = self.save(ui, temporary) {
            // said once until a write works
            if !self.failing {
                log::error!("{e}");
            }
            self.failing = true;
        } else {
            self.failing = false;
        }
    }

    /// Write `ui` now if it changed. `temporary`: the session promises to save nothing (the
    /// library couldn't be opened).
    pub fn save(&mut self, ui: &UiState, temporary: bool) -> Result<(), String> {
        if self.keep_file || temporary {
            return Ok(());
        }
        let bytes = serde_json::to_vec_pretty(ui).map_err(|e| e.to_string())?;
        if bytes == self.written {
            return Ok(());
        }
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("saving the app settings failed: {e}"))?;
        }
        write_atomic(&self.path, &bytes).map_err(|e| format!("saving the app settings failed: {e}"))?;
        self.written = bytes;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("lc-android-prefs-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn what_is_saved_loads_back() {
        let d = dir("roundtrip");
        let path = d.join("ui.json");
        let ui = UiState::default();
        let mut w = Writer::new(path.clone(), &ui, false);
        let mut changed = ui.clone();
        changed.settings.gpu = !changed.settings.gpu;
        w.save(&changed, false).unwrap();
        let (loaded, warning, keep) = load(&path);
        assert!(warning.is_none() && !keep);
        assert_eq!(loaded.unwrap().settings.gpu, changed.settings.gpu);
    }

    #[test]
    fn a_damaged_file_is_set_aside_not_overwritten() {
        let d = dir("damaged");
        let path = d.join("ui.json");
        std::fs::write(&path, b"{ not json").unwrap();
        let (loaded, warning, keep) = load(&path);
        assert!(loaded.is_none() && warning.is_some() && !keep);
        assert!(!path.exists());
        assert!(std::fs::read_dir(&d).unwrap().flatten().any(|e| e.file_name().to_string_lossy().starts_with("ui.json.corrupt-")));
    }

    #[test]
    fn nothing_is_written_when_told_not_to() {
        let d = dir("keep");
        let path = d.join("ui.json");
        let ui = UiState::default();
        let mut changed = ui.clone();
        changed.settings.gpu = !changed.settings.gpu;
        Writer::new(path.clone(), &ui, true).save(&changed, false).unwrap();
        Writer::new(path.clone(), &ui, false).save(&changed, true).unwrap();
        assert!(!path.exists());
    }
}
