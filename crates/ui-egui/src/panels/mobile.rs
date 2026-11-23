//! The phone layout (Android): a Lightroom-Mobile-style shell around the same panels the desktop
//! docks. Two screens, chosen by the view mode:
//!
//! - **Library**: a top bar (menu, title, filter, add), the photo grid in three columns, and a bottom
//!   navigation bar. The menu opens a side drawer with the desktop's folders, albums and Local browser.
//! - **Edit**: a top bar (back, name, undo, redo), the photo, a sheet with the open tool's sliders, and a
//!   horizontally scrolling tool strip under it (Presets, Light, Color, Effects, Detail, Optics, Crop,
//!   Masking, Remove, Info).
//!
//! Everything is the desktop's own panel code (`grid`, `detail`, `right`, `presets`, `left`), so a
//! feature added there shows up here; only the arrangement and the touch-sized chrome are new.

use egui::{Align2, Color32, Rect, Sense, Stroke, pos2, vec2};
use serde_json::json;

use crate::LightcraftApp;
use crate::icons::{Icon, paint};
use crate::state::{RightPanel, ViewMode};
use crate::theme::Tokens;

/// The smallest touch target (points).
const TOUCH: f32 = 44.0;
const TOP_H: f32 = 56.0;
const NAV_H: f32 = 60.0;
const TOOLS_H: f32 = 68.0;
/// Photos per row in the Library grid.
const COLUMNS: f32 = 3.0;

/// A tool on the Edit screen's strip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Presets,
    Light,
    Color,
    Effects,
    Detail,
    Optics,
    Crop,
    Masking,
    Remove,
    Info,
}

impl Tool {
    const ALL: [Self; 10] =
        [Self::Presets, Self::Light, Self::Color, Self::Effects, Self::Detail, Self::Optics, Self::Crop, Self::Masking, Self::Remove, Self::Info];

    /// The tool called `name` (its English label, any case): `light`, `crop`…
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.label().eq_ignore_ascii_case(name.trim()))
    }

    fn label(self) -> &'static str {
        match self {
            Self::Presets => "Presets",
            Self::Light => "Light",
            Self::Color => "Color",
            Self::Effects => "Effects",
            Self::Detail => "Detail",
            Self::Optics => "Optics",
            Self::Crop => "Crop",
            Self::Masking => "Masking",
            Self::Remove => "Remove",
            Self::Info => "Info",
        }
    }

    fn icon(self) -> Icon {
        match self {
            Self::Presets => Icon::Presets,
            Self::Light | Self::Color | Self::Effects | Self::Detail | Self::Optics => Icon::Sliders,
            Self::Crop => Icon::Crop,
            Self::Masking => Icon::Mask,
            Self::Remove => Icon::Eraser,
            Self::Info => Icon::Info,
        }
    }

    /// The Edit panel section this tool opens, when it is one.
    pub(crate) fn section(self) -> Option<&'static str> {
        match self {
            Self::Light => Some("light"),
            Self::Color => Some("color"),
            Self::Effects => Some("effects"),
            Self::Detail => Some("detail"),
            Self::Optics => Some("optics"),
            _ => None,
        }
    }
}

/// Whether the Edit panel is a phone's sheet for one of its sections (Light, Color…): the panel then
/// leaves out what sits above the sliders.
pub(crate) fn compact(app: &LightcraftApp) -> bool {
    app.mobile && app.mobile_tool.is_some_and(|t| t.section().is_some())
}

/// Draws the phone window's panels. Called where the desktop calls its top bar and docks.
pub fn show(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    // the phone has the grid and the single photo: Compare, Survey, Reference and People are desktop views
    if matches!(app.ui.view, ViewMode::Compare | ViewMode::Survey | ViewMode::Reference | ViewMode::People) {
        let _ = app.run("view.photoGrid", json!({}));
    }
    if app.ui.view == ViewMode::Detail && app.session.active().is_some() {
        edit_screen(app, ui);
    } else {
        if app.ui.view == ViewMode::Detail {
            let _ = app.run("view.photoGrid", json!({}));
        }
        library_screen(app, ui);
    }
    drawer(app, ui.ctx());
}

// ---- Library ----------------------------------------------------------------------------------

fn library_screen(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    app.mobile_tool = None;
    // the phone shows the uniform grid, not the justified rows grouped by day
    if app.ui.view == ViewMode::PhotoGrid {
        app.ui.view = ViewMode::SquareGrid;
    }
    // clean tiles: no file name over each
    app.ui.show_filenames = false;
    nav_bar(app, ui, Nav::Library);
    egui::Panel::top("mobile_top").exact_size(TOP_H).frame(egui::Frame::NONE.fill(t.chrome)).show(ui, |ui| {
        let r = ui.max_rect();
        let mut x = r.left() + 4.0;
        if bar_icon(ui, "mobileMenu", Icon::Sidebar, pos2(x + TOUCH / 2.0, r.center().y), "Folders and albums", false).clicked() {
            app.mobile_drawer = true;
        }
        x += TOUCH + 4.0;
        // (the grid's own header under this bar names the source and counts the photos)
        ui.painter().text(pos2(x, r.center().y), Align2::LEFT_CENTER, "LightCraft", t.semibold(20.0), t.text);
        let filtering = app.session.filter != Default::default() || app.ui.filter_bar;
        let mut right = r.right() - 4.0 - TOUCH / 2.0;
        if bar_icon(ui, "mobileAdd", Icon::Plus, pos2(right, r.center().y), "Add photos", false).clicked() {
            app.mobile_drawer = true;
        }
        right -= TOUCH;
        if bar_icon(ui, "mobileFilter", Icon::Filter, pos2(right, r.center().y), "Filter", filtering).clicked() {
            let _ = app.run("view.filterBar", json!({}));
        }
    });
    super::library_problem::banner(app, ui);
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.grid_bg)).show(ui, |ui| {
        if app.ui.view == ViewMode::SquareGrid {
            // three photos to a row, however wide the screen
            // (the grid's own padding and scroll bar take some of the width, so size from less than all of it)
            let w = ui.available_width() - 20.0;
            app.ui.thumb_size = (w / COLUMNS - 1.0).clamp(crate::state::THUMB_SIZE_MIN, crate::state::THUMB_SIZE_MAX);
        }
        super::grid::show(app, ui);
    });
}

// ---- Edit -------------------------------------------------------------------------------------

fn edit_screen(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    tool_strip(app, ui);
    if let Some(tool) = app.mobile_tool {
        let h = (ui.ctx().content_rect().height() * 0.42).clamp(230.0, 470.0);
        egui::Panel::bottom("mobile_sheet")
            .exact_size(h)
            .frame(egui::Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.divider)))
            .show(ui, |ui| sheet(app, ui, tool));
    }
    egui::Panel::top("mobile_edit_top").exact_size(TOP_H).frame(egui::Frame::NONE.fill(t.chrome)).show(ui, |ui| {
        let r = ui.max_rect();
        if bar_icon(ui, "mobileBack", Icon::ChevronLeft, pos2(r.left() + 4.0 + TOUCH / 2.0, r.center().y), "Back to Library", false).clicked() {
            close_tool(app);
            let _ = app.run("view.photoGrid", json!({}));
        }
        let name = app.session.active().and_then(|id| app.session.catalog.photo(id)).map(|p| p.file_name.clone()).unwrap_or_default();
        let left = r.left() + TOUCH + 12.0;
        let right_edge = r.right() - 2.0 * TOUCH - 12.0;
        // a long name is cut short rather than running under the buttons
        let font = t.semibold(15.0);
        let galley = ui.painter().layout(name, font, t.text, (right_edge - left).max(40.0));
        ui.painter().galley(pos2(left, r.center().y - galley.size().y / 2.0), galley, t.text);
        let mut x = r.right() - 4.0 - TOUCH / 2.0;
        if bar_button(ui, "mobileRedo", pos2(x, r.center().y), "Redo", false, |p, r, c| curved_arrow(p, r, c, true)).clicked() {
            let _ = app.run("edit.redo", json!({}));
        }
        x -= TOUCH;
        if bar_button(ui, "mobileUndo", pos2(x, r.center().y), "Undo", false, |p, r, c| curved_arrow(p, r, c, false)).clicked() {
            let _ = app.run("edit.undo", json!({}));
        }
    });
    super::library_problem::banner(app, ui);
    super::compare::prepare(app);
    // the filmstrip gives way to the sheet (it comes back when the sheet closes)
    let filmstrip = app.ui.filmstrip;
    if app.mobile_tool.is_some() {
        app.ui.filmstrip = false;
    }
    egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.canvas)).show(ui, |ui| super::detail::show(app, ui));
    app.ui.filmstrip = filmstrip;
}

/// The sheet under the photo: the open tool's title and its controls, scrolling.
fn sheet(app: &mut LightcraftApp, ui: &mut egui::Ui, tool: Tool) {
    let t = Tokens::get(ui.ctx());
    let (hr, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
    // the grab handle, and the tool's name with a chevron that closes the sheet
    let handle = Rect::from_center_size(pos2(hr.center().x, hr.top() + 7.0), vec2(36.0, 4.0));
    ui.painter().rect_filled(handle, 2.0, t.divider);
    ui.painter().text(pos2(hr.left() + 20.0, hr.center().y + 4.0), Align2::LEFT_CENTER, crate::i18n::tr(tool.label()), t.semibold(16.0), t.text);
    paint(ui.painter(), Rect::from_center_size(pos2(hr.right() - 28.0, hr.center().y + 4.0), vec2(16.0, 16.0)), Icon::ChevronDown, t.icon);
    if tool.section().is_some() {
        // Auto, as the desktop's Edit header has it
        let auto = Rect::from_center_size(pos2(hr.right() - 84.0, hr.center().y + 4.0), vec2(64.0, 32.0));
        let ar = ui.interact(auto, egui::Id::unique("mobile-auto"), Sense::click());
        ar.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "Auto"));
        ui.painter().rect_stroke(auto, 8.0, Stroke::new(1.0, t.divider), egui::StrokeKind::Inside);
        ui.painter().text(auto.center(), Align2::CENTER_CENTER, crate::i18n::tr("Auto"), t.semibold(13.0), t.text);
        if ar.clicked() {
            let _ = app.run("develop.auto", json!({}));
        }
    }
    if resp.clicked() {
        close_tool(app);
        return;
    }
    egui::Frame::NONE.show(ui, |ui| {
        if tool == Tool::Presets {
            super::presets::show(app, ui);
        } else {
            super::right::show(app, ui);
        }
    });
}

/// The strip of tools under the sheet: icon and name each, side to side.
fn tool_strip(app: &mut LightcraftApp, ui: &mut egui::Ui) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::bottom("mobile_tools").exact_size(TOOLS_H).frame(egui::Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.divider))).show(
        ui,
        |ui| {
            egui::ScrollArea::horizontal().id_salt("mobile-tools").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(
                ui,
                |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        for tool in Tool::ALL {
                            let on = app.mobile_tool == Some(tool);
                            let (r, resp) = ui.allocate_exact_size(vec2(76.0, TOOLS_H - 2.0), Sense::click());
                            crate::widgets::register(ui, format!("mobiletool:{}", tool.label()), r);
                            resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, on, tool.label()));
                            let color = if on { t.accent } else { t.text_dim };
                            paint(ui.painter(), Rect::from_center_size(pos2(r.center().x, r.center().y - 9.0), vec2(24.0, 24.0)), tool.icon(), color);
                            ui.painter().text(
                                pos2(r.center().x, r.center().y + 16.0),
                                Align2::CENTER_CENTER,
                                crate::i18n::tr(tool.label()),
                                t.font(12.0),
                                color,
                            );
                            if on {
                                ui.painter().line_segment(
                                    [pos2(r.left() + 18.0, r.top() + 1.0), pos2(r.right() - 18.0, r.top() + 1.0)],
                                    Stroke::new(2.0, t.accent),
                                );
                            }
                            if resp.clicked() {
                                if on { close_tool(app) } else { open_tool(app, tool) }
                            }
                        }
                    });
                },
            );
        },
    );
}

fn open_tool(app: &mut LightcraftApp, tool: Tool) {
    app.mobile_tool = Some(tool);
    match tool {
        Tool::Presets => {}
        Tool::Light | Tool::Color | Tool::Effects | Tool::Detail | Tool::Optics => {
            let _ = app.run("panel.edit", json!({}));
            app.ui.right = RightPanel::Edit;
            if let Some(id) = tool.section() {
                app.ui.solo_section(id);
            }
        }
        Tool::Crop => {
            let _ = app.run("panel.crop", json!({}));
        }
        Tool::Masking => {
            let _ = app.run("panel.masking", json!({}));
        }
        Tool::Remove => {
            let _ = app.run("panel.remove", json!({}));
        }
        Tool::Info => app.ui.right = RightPanel::Info,
    }
}

/// Closes the sheet and the tool behind it (a crop or mask overlay on the photo goes with it).
fn close_tool(app: &mut LightcraftApp) {
    if matches!(app.ui.right, RightPanel::Crop | RightPanel::Masking | RightPanel::Remove | RightPanel::RedEye) {
        // the tool's own command toggles it off, as its icon does on the desktop
        let name = match app.ui.right {
            RightPanel::Crop => "panel.crop",
            RightPanel::Masking => "panel.masking",
            RightPanel::Remove => "panel.remove",
            _ => "panel.redeye",
        };
        let _ = app.run(name, json!({}));
    }
    app.ui.right = RightPanel::None;
    app.mobile_tool = None;
}

// ---- Bottom navigation and the drawer -----------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Nav {
    Library,
}

/// Library, Edit and Settings along the bottom of the Library screen.
fn nav_bar(app: &mut LightcraftApp, ui: &mut egui::Ui, _on: Nav) {
    let t = Tokens::get(ui.ctx());
    egui::Panel::bottom("mobile_nav").exact_size(NAV_H).frame(egui::Frame::NONE.fill(t.chrome).stroke(Stroke::new(1.0, t.divider))).show(ui, |ui| {
        let r = ui.max_rect();
        let has_photo = app.session.active().is_some();
        let items = [("Library", Icon::Photos, true, true), ("Edit", Icon::Sliders, false, has_photo), ("Settings", Icon::Gear, false, true)];
        let w = r.width() / items.len() as f32;
        for (i, (label, icon, on, enabled)) in items.into_iter().enumerate() {
            let cell = Rect::from_min_size(pos2(r.left() + w * i as f32, r.top()), vec2(w, r.height()));
            let resp = ui.interact(cell, egui::Id::unique(("mobile-nav", label)), if enabled { Sense::click() } else { Sense::hover() });
            crate::widgets::register(ui, format!("mobilenav:{label}"), cell);
            resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, enabled, on, label));
            let color = if on {
                t.accent
            } else if enabled {
                t.text_dim
            } else {
                t.text_disabled
            };
            paint(ui.painter(), Rect::from_center_size(pos2(cell.center().x, cell.center().y - 8.0), vec2(24.0, 24.0)), icon, color);
            ui.painter().text(pos2(cell.center().x, cell.center().y + 17.0), Align2::CENTER_CENTER, crate::i18n::tr(label), t.font(12.0), color);
            if resp.clicked() {
                match label {
                    "Edit" => {
                        let _ = app.run("view.detail", json!({}));
                    }
                    "Settings" => {
                        let _ = app.run("app.settings", json!({}));
                    }
                    _ => {}
                }
            }
        }
    });
}

/// The side drawer over the Library: the desktop's left panel (folders, albums, dates, keywords and
/// Local, the way photos come in on a phone), with a scrim that closes it.
fn drawer(app: &mut LightcraftApp, ctx: &egui::Context) {
    if !app.mobile_drawer {
        return;
    }
    let t = Tokens::get(ctx);
    let screen = ctx.content_rect();
    egui::Area::new(egui::Id::unique("mobile-scrim")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
        let (r, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
        ui.painter().rect_filled(r, 0.0, Color32::from_black_alpha(150));
        if resp.clicked() {
            app.mobile_drawer = false;
        }
    });
    let width = (screen.width() * 0.84).min(380.0);
    egui::Area::new(egui::Id::unique("mobile-drawer")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
        let rect = Rect::from_min_size(screen.min, vec2(width, screen.height()));
        ui.painter().rect_filled(rect, 0.0, t.chrome);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(egui::Layout::top_down(egui::Align::Min)));
        child.set_clip_rect(rect);
        let (hr, resp) = child.allocate_exact_size(vec2(width, TOP_H), Sense::click());
        child.painter().text(pos2(hr.left() + 20.0, hr.center().y), Align2::LEFT_CENTER, crate::i18n::tr("Library"), t.semibold(18.0), t.text);
        child.painter().text(pos2(hr.right() - 20.0, hr.center().y), Align2::RIGHT_CENTER, crate::i18n::tr("Done"), t.semibold(15.0), t.accent);
        if resp.clicked() {
            app.mobile_drawer = false;
        }
        super::left::show(app, &mut child);
    });
}

/// A touch button holding one icon, centred on `at`.
fn bar_icon(ui: &mut egui::Ui, id: &str, icon: Icon, at: egui::Pos2, tip: &str, active: bool) -> egui::Response {
    bar_button(ui, id, at, tip, active, |p, r, color| paint(p, r, icon, color))
}

/// A touch button that draws its own glyph (`draw` gets the painter, a 24-point square and the colour).
fn bar_button(
    ui: &mut egui::Ui,
    id: &str,
    at: egui::Pos2,
    tip: &str,
    active: bool,
    draw: impl FnOnce(&egui::Painter, Rect, Color32),
) -> egui::Response {
    let t = Tokens::get(ui.ctx());
    let r = Rect::from_center_size(at, vec2(TOUCH, TOUCH));
    let resp = ui.interact(r, egui::Id::unique(("mobile-icon", id)), Sense::click());
    let tip = crate::i18n::tr(tip);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tip));
    crate::widgets::register(ui, format!("icon:{id}"), r);
    if resp.is_pointer_button_down_on() {
        ui.painter().rect_filled(r.shrink(4.0), 22.0, t.pressed);
    }
    draw(ui.painter(), Rect::from_center_size(at, vec2(24.0, 24.0)), if active { t.accent } else { t.icon });
    resp
}

/// The curved arrow of undo (pointing left) and redo (mirrored): an arc over the top with a head.
fn curved_arrow(p: &egui::Painter, r: Rect, color: Color32, redo: bool) {
    let c = r.center() + vec2(0.0, 2.0);
    let radius = 7.5;
    let sign = if redo { -1.0 } else { 1.0 };
    let point = |deg: f32| {
        let a = deg.to_radians();
        pos2(c.x + sign * radius * a.cos(), c.y - radius * a.sin())
    };
    let stroke = Stroke::new(2.0, color);
    let points: Vec<egui::Pos2> = (0..=18).map(|i| point(15.0 + 170.0 * i as f32 / 18.0)).collect();
    p.add(egui::Shape::line(points, stroke));
    // the head at the end of the arc, pointing along it (down and away)
    let end = point(185.0);
    let a = 185.0f32.to_radians();
    let dir = vec2(-sign * a.sin(), -a.cos()).normalized();
    for turn in [0.6f32, -0.6] {
        let (s, co) = turn.sin_cos();
        let back = vec2(dir.x * co - dir.y * s, dir.x * s + dir.y * co) * -5.5;
        p.line_segment([end, end + back], stroke);
    }
}
