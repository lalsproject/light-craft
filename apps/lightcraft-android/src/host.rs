//! Starting LightCraft inside the activity: folders, settings, the window's renderer, the library
//! and the platform services. Mirrors `apps/lightcraft/src/main.rs` for what an Android phone has.

use crate::{paths, prefs};
use eframe::egui_wgpu::{WgpuConfiguration, WgpuSetup};
use lightcraft_engine::Session;
use lightcraft_engine::gpu::backend::{self, Backends, WindowRenderer};
use lightcraft_ui_egui::panels::library_problem::LibraryProblem;
use lightcraft_ui_egui::{LightcraftApp, Services, UiState};
use winit::platform::android::activity::AndroidApp;

/// How long quitting waits for renders that are running (the desktop app's value).
const EXIT_WAIT: std::time::Duration = std::time::Duration::from_secs(2);

/// Runs until the activity ends. Failures are logged (`logcat -s LightCraft`): there is no console.
pub fn run(app: AndroidApp) {
    android_logger::init_once(android_logger::Config::default().with_tag("LightCraft").with_max_level(log::LevelFilter::Info));
    if let Err(e) = start(app) {
        log::error!("LightCraft stopped: {e}");
    }
}

fn start(android: AndroidApp) -> Result<(), String> {
    let internal = android.internal_data_path().ok_or("Android gave the app no private data folder")?;
    let external = android.external_data_path();
    let dirs = paths::dirs(&internal, external.as_deref());
    // before anything asks the engine for the config folder or the default library
    lightcraft_engine::config::set_data_root(internal);
    for d in [&dirs.config, &dirs.logs] {
        if let Err(e) = std::fs::create_dir_all(d) {
            log::warn!("can't create {}: {e}", d.display());
        }
    }
    lightcraft_engine::guard::install_hook(dirs.logs.join("panic.log"));
    log::info!("LightCraft {}, config {}, library {}", env!("CARGO_PKG_VERSION"), dirs.config.display(), dirs.library.display());

    let (saved, prefs_warning, keep_prefs_file) = prefs::load(&dirs.prefs);
    if let Some(ui) = &saved {
        lightcraft_ui_egui::i18n::set_language(ui.language);
    }
    // GPU compute stays off until Settings turns it on: phone drivers differ a lot, and a crash in
    // one kills the app with nothing to show for it
    let first_run = saved.is_none();
    let gpu_on = saved.as_ref().is_some_and(|u| u.settings.gpu);
    lightcraft_engine::gpu::set_enabled(gpu_on);

    let marker = dirs.config.join("window-renderer-armed");
    let (renderer, wgpu_options, which) = choose_renderer(&marker);

    let places: Vec<(String, String)> = paths::shared_places(&paths::shared_root(std::env::var_os("EXTERNAL_STORAGE")), |p| p.is_dir())
        .into_iter()
        .map(|(name, p)| (lightcraft_ui_egui::i18n::tr(name).to_string(), p.to_string_lossy().into_owned()))
        .collect();
    let log_file = dirs.logs.join("panic.log");
    let prefs_path = dirs.prefs.clone();
    let library = dirs.library.clone();
    let shared_ok = places.first().is_none_or(|(_, p)| std::fs::read_dir(p).is_ok());

    let options = eframe::NativeOptions {
        android_app: Some(android),
        renderer,
        wgpu_options,
        viewport: egui::ViewportBuilder::default().with_title("LightCraft"),
        ..Default::default()
    };
    let started = eframe::run_native(
        "LightCraft",
        options,
        Box::new(move |cc| {
            // the window's renderer is up
            let _ = std::fs::remove_file(&marker);
            let (mut session, problem) = open_session(&library);
            let mut app = LightcraftApp::new(session_with_defaults(&mut session), services(places, Some(log_file)));
            if let Some(ui) = saved {
                app.ui = ui;
            }
            lightcraft_ui_egui::i18n::set_language(app.ui.language);
            if first_run {
                app.ui.settings.gpu = false;
            }
            app.notices.extend(prefs_warning);
            if !shared_ok {
                app.notices.push(
                    lightcraft_ui_egui::i18n::tr(
                        "LightCraft can't read your photos yet. Allow Storage in Android Settings ▸ Apps ▸ LightCraft ▸ Permissions, then reopen the app.",
                    )
                    .to_string(),
                );
            }
            app.gpu_notice = false;
            app.disk_space_notice = true;
            if problem.is_none() {
                lightcraft_ui_egui::panels::empty_library::open_on_empty_library(&mut app);
            }
            app.library_problem = problem;
            let writer = prefs::Writer::new(prefs_path, &app.ui, keep_prefs_file);
            let _ = &cc.egui_ctx;
            Ok(Box::new(App { app, prefs: writer }))
        }),
    );
    started.map_err(|e| format!("the window ({}) ended with an error: {e}", which.name()))
}

/// The library at `dir`, or when it can't be opened an empty in-memory session and the problem to
/// show (never a silent replacement, as on the desktop).
fn open_session(dir: &std::path::Path) -> (Session, Option<LibraryProblem>) {
    let fresh = || Session::new().with_fs().with_default_denoise_models().with_system_clock();
    let mut s = fresh();
    match s.open_library(dir, true) {
        Ok(r) => {
            let replayed = r.replayed;
            log::info!("library {}: {} photos, {replayed} log records replayed", dir.display(), s.catalog.len());
            (s, None)
        }
        Err(e) => {
            log::error!("can't open library {}: {e}", dir.display());
            (fresh(), Some(LibraryProblem::new(dir.to_string_lossy(), e.to_string())))
        }
    }
}

/// Lens profiles and the model folders, as the desktop app sets them (no SAM 3 model: too big for a
/// 32-bit address space).
fn session_with_defaults(session: &mut Session) -> Session {
    let s = std::mem::replace(session, Session::new());
    let mut s = s.with_default_lens_database().with_lens_auto_download();
    s.face_models_dir = lightcraft_engine::config::default_face_models_dir();
    s.slide_templates_dir = lightcraft_engine::config::config_dir().map(|d| d.join(lightcraft_engine::slide_template::DIR_NAME));
    s
}

fn services(places: Vec<(String, String)>, log_file: Option<std::path::PathBuf>) -> Services {
    Services {
        // no system file dialog on Android: Local browses these folders
        local_places: Some(places),
        log_file: log_file.map(|p| p.display().to_string()),
        write_shared: Some(std::sync::Arc::new(lightcraft_engine::export::write_file)),
        write: Some(Box::new(lightcraft_engine::export::write_file)),
        write_new: Some(Box::new(lightcraft_engine::export::write_file_new)),
        png: Some(Box::new(|img: &lightcraft_raster::Rgba8| {
            lightcraft_codecs::encode_png(&lightcraft_codecs::EncodeImage::rgba8(img), &lightcraft_codecs::EncodeMeta::default()).unwrap_or_default()
        })),
        ..Default::default()
    }
}

/// Vulkan when an adapter that is not a software one answers, else glow (OpenGL ES). A marker file
/// names the renderer being started: left behind, the last start died in it and it goes last.
fn choose_renderer(marker: &std::path::Path) -> (eframe::Renderer, WgpuConfiguration, WindowRenderer) {
    let failed = std::fs::read_to_string(marker).ok().and_then(|s| WindowRenderer::parse(s.trim()));
    let forced =
        backend::forced_window_renderer(std::env::var("LIGHTCRAFT_GPU_BACKEND").ok().as_deref(), std::env::var("WGPU_BACKEND").ok().as_deref());
    let order = backend::window_renderer_order(forced, None, failed, "android", eframe::wgpu::Instance::enabled_backend_features(), true);
    let mut which = WindowRenderer::Glow;
    for r in order {
        let usable = match r {
            WindowRenderer::Glow => true,
            WindowRenderer::Wgpu(b) => std::panic::catch_unwind(|| has_hardware_adapter(b)).unwrap_or(false),
        };
        if usable {
            which = r;
            break;
        }
        log::warn!("window: no usable {} adapter, trying the next renderer", r.name());
    }
    log::info!("window: renderer {}", which.name());
    let _ = std::fs::write(marker, which.name());
    let mut wgpu_options = eframe::NativeOptions::default().wgpu_options;
    let renderer = match which {
        WindowRenderer::Glow => eframe::Renderer::Glow,
        WindowRenderer::Wgpu(b) => {
            if let WgpuSetup::CreateNew(n) = &mut wgpu_options.wgpu_setup {
                n.instance_descriptor.backends = b;
            }
            eframe::Renderer::Wgpu
        }
    };
    (renderer, wgpu_options, which)
}

/// Whether `backends` has an adapter that isn't a CPU rasteriser (a phone's Vulkan loader can be
/// there without a driver behind it).
fn has_hardware_adapter(backends: Backends) -> bool {
    let mut desc = eframe::wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = backends;
    let instance = eframe::wgpu::Instance::new(desc);
    pollster::block_on(instance.enumerate_adapters(backends)).iter().any(|a| a.get_info().device_type != eframe::wgpu::DeviceType::Cpu)
}

struct App {
    app: LightcraftApp,
    prefs: prefs::Writer,
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::from_rgb(12, 12, 12).to_normalized_gamma_f32()
    }
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.app.logic(ctx);
        let temporary = self.app.library_problem.as_ref().is_some_and(LibraryProblem::saves_nothing);
        self.prefs.tick(&self.app.ui, temporary);
    }
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw: &mut egui::RawInput) {
        self.app.raw_input_hook(ctx, raw);
    }
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.app.perf.host_frame_ms = frame.info().cpu_usage.map(|s| f64::from(s) * 1000.0);
        self.app.ui(ui);
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        let temporary = self.app.library_problem.as_ref().is_some_and(LibraryProblem::saves_nothing);
        if let Err(e) = self.prefs.save(&self.app.ui, temporary) {
            log::error!("{e}");
        }
        if let Err(e) = self.app.session.close_library() {
            log::error!("saving the library failed: {e}");
        }
        if !self.app.shutdown(EXIT_WAIT) {
            log::warn!("quit: background work was still running after {EXIT_WAIT:?}");
        }
    }
}

// keep the type in the signature of `UiState`-consuming code visible to readers of this file
#[allow(dead_code)]
type Ui = UiState;
