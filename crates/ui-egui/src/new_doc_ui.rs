//! File › New: Photoshop's New Document dialog. Category tabs with blank-document presets on the
//! left, Preset Details on the right. Values live in the dialog fields (`width`/`height` in pixels,
//! `resolution` in ppi, `mode`, `depth`, `background`, `name`), so `ui.dialog.set` drives it and
//! Create runs `file.new` with them.

use egui::{Align2, Rect, RichText, Sense, Stroke, StrokeKind, pos2, vec2};
use serde_json::{Map, Value, json};

use crate::theme::Tokens;
use crate::{icons, widgets};

/// A blank-document preset: (name, width px, height px, ppi).
pub type Preset = (&'static str, u32, u32, f32);

/// Photoshop's New Document categories and their blank-document presets.
pub const CATEGORIES: &[(&str, &[Preset])] = &[
    ("Recent", &[("Default Photoshop Size", 2100, 1500, 300.0), ("HDTV 1080p", 1920, 1080, 72.0)]),
    (
        "Photo",
        &[
            ("Landscape, 6 x 4", 1800, 1200, 300.0),
            ("Landscape, 7 x 5", 2100, 1500, 300.0),
            ("Landscape, 10 x 8", 3000, 2400, 300.0),
            ("Portrait, 4 x 6", 1200, 1800, 300.0),
            ("Portrait, 5 x 7", 1500, 2100, 300.0),
            ("Square, 5 x 5", 1500, 1500, 300.0),
        ],
    ),
    (
        "Print",
        &[
            ("Letter", 2550, 3300, 300.0),
            ("Legal", 2550, 4200, 300.0),
            ("Tabloid", 3300, 5100, 300.0),
            ("A4", 2480, 3508, 300.0),
            ("A3", 3508, 4961, 300.0),
            ("A5", 1748, 2480, 300.0),
        ],
    ),
    (
        "Art & Illustration",
        &[("Poster", 5400, 7200, 300.0), ("Postcard", 1800, 1200, 300.0), ("Comic Book", 1988, 3075, 300.0), ("Square, 12 x 12", 3600, 3600, 300.0)],
    ),
    (
        "Web",
        &[
            ("Web Most Common", 1366, 768, 72.0),
            ("Web Minimum", 1024, 768, 72.0),
            ("Web Large", 1920, 1080, 72.0),
            ("MacBook Pro 16\"", 3456, 2234, 72.0),
            ("iMac 24\"", 4480, 2520, 72.0),
        ],
    ),
    (
        "Mobile",
        &[
            ("iPhone 16", 1179, 2556, 72.0),
            ("iPhone 16 Pro Max", 1320, 2868, 72.0),
            ("iPad Pro 13\"", 2064, 2752, 72.0),
            ("Android 1080p", 1080, 1920, 72.0),
            ("Apple Watch 45mm", 396, 484, 72.0),
        ],
    ),
    (
        "Film & Video",
        &[
            ("HDTV 1080p", 1920, 1080, 72.0),
            ("HDTV 720p", 1280, 720, 72.0),
            ("UHD 4K", 3840, 2160, 72.0),
            ("DCI 4K", 4096, 2160, 72.0),
            ("UHD 8K", 7680, 4320, 72.0),
        ],
    ),
];

const DEPTH_OPTIONS: &[(u64, &str, &str)] = &[(8, "8 bit", "Integer"), (16, "16 bit", "Integer"), (32, "32 bit (float)", "Floating point")];

/// Width/Height units: (key, label, units per inch; 0 = pixels).
pub const UNITS: &[(&str, &str, f32)] =
    &[("px", "Pixels", 0.0), ("in", "Inches", 1.0), ("cm", "Centimeters", 2.54), ("mm", "Millimeters", 25.4), ("pt", "Points", 72.0), ("pica", "Picas", 6.0)];

/// Pixels to the display unit at `ppi`.
pub fn to_unit(px: f32, unit: &str, ppi: f32) -> f32 {
    match UNITS.iter().find(|u| u.0 == unit) {
        Some((_, _, per_in)) if *per_in > 0.0 => px / ppi.max(1.0) * per_in,
        _ => px,
    }
}

/// Display unit back to pixels at `ppi`.
pub fn from_unit(v: f32, unit: &str, ppi: f32) -> f32 {
    match UNITS.iter().find(|u| u.0 == unit) {
        Some((_, _, per_in)) if *per_in > 0.0 => (v / per_in * ppi.max(1.0)).round(),
        _ => v.round(),
    }
}

/// A typed size as the whole pixel count `file.new` takes (#254: a float like `512.0` isn't one, so
/// the command fell back to its 1920 x 1080 default). Clamped to the command's 1–300000 range.
pub fn px_value(px: f32) -> Value {
    json!(if px.is_finite() { px.round().clamp(1.0, 300_000.0) as u32 } else { 1 })
}

/// Name of the preset that takes the clipboard image's size.
pub const CLIPBOARD: &str = "Clipboard";

/// Offer the Clipboard preset (`w` × `h` px, at 72 ppi) first under the Recent presets, and
/// select it.
pub fn set_clipboard(f: &mut Map<String, Value>, w: u32, h: u32) {
    if w == 0 || h == 0 {
        return;
    }
    f.insert("__clipboard".into(), json!([w, h]));
    apply_preset(f, &(CLIPBOARD, w, h, 72.0));
}

/// The clipboard image's size, when the dialog offers the Clipboard preset.
fn clipboard_preset(f: &Map<String, Value>) -> Option<Preset> {
    let size = f.get("__clipboard")?.as_array()?;
    let dim = |i: usize| size.get(i)?.as_u64().and_then(|v| u32::try_from(v).ok()).filter(|v| *v > 0);
    Some((CLIPBOARD, dim(0)?, dim(1)?, 72.0))
}

/// Set Width or Height (`key`) from a value typed in `unit`: whole pixels for `file.new`, plus the
/// typed value so the field keeps showing it (see [`shown_size`]).
pub fn set_size(f: &mut Map<String, Value>, key: &str, v: f32, unit: &str, ppi: f32) {
    f.insert(key.into(), px_value(from_unit(v, unit, ppi)));
    f.insert(typed_key(key), json!({"value": v, "unit": unit}));
    f.remove("__preset");
    f.remove("__savedPreset");
}

/// What Width or Height (`key`, `d` pixels when unset) shows in `unit`: the value the user typed
/// while it still gives the field's pixel count, else the pixels converted. Converting the rounded
/// pixels back on every keystroke turned a typed `5` mm into 14 px and the text into `4.9` (#1147).
/// A size over the 300000 px limit shows the limit, not the typed value the document won't have.
pub fn shown_size(f: &Map<String, Value>, key: &str, d: f32, unit: &str, ppi: f32) -> f32 {
    let px = get_f(f, key, d);
    let typed = f.get(&typed_key(key)).and_then(Value::as_object).filter(|t| unit != "px" && t.get("unit").and_then(Value::as_str) == Some(unit));
    match typed.and_then(|t| t.get("value")).and_then(Value::as_f64) {
        Some(v) if from_unit(v as f32, unit, ppi).max(1.0) == px => v as f32,
        _ => to_unit(px, unit, ppi),
    }
}

fn typed_key(key: &str) -> String {
    format!("__{key}Typed")
}

/// Swap Width and Height (the Orientation buttons), with the values typed for them, so a typed
/// 841 x 1189 mm turns into 1189 x 841 mm, not 1188.9 x 841.
pub fn swap_size(f: &mut Map<String, Value>) {
    f.remove("__savedPreset");
    let (w, h) = (get_f(f, "width", 1920.0), get_f(f, "height", 1080.0));
    f.insert("width".into(), px_value(h));
    f.insert("height".into(), px_value(w));
    let (typed_w, typed_h) = (f.remove(&typed_key("width")), f.remove(&typed_key("height")));
    if let Some(t) = typed_h {
        f.insert(typed_key("width"), t);
    }
    if let Some(t) = typed_w {
        f.insert(typed_key("height"), t);
    }
}

/// Apply a preset to the dialog fields.
pub fn apply_preset(f: &mut Map<String, Value>, p: &Preset) {
    Card::preset(p).apply(f);
}

/// Fields `file.new` takes (drops the dialog's `__` UI keys).
pub fn command_params(f: &Map<String, Value>) -> Value {
    Value::Object(f.iter().filter(|(k, _)| !k.starts_with("__")).map(|(k, v)| (k.clone(), v.clone())).collect())
}

/// Set the resolution (pixels/inch) the way Photoshop's New Document does (#758): with Width/Height
/// in a physical unit the physical size is kept and the pixel count changes; in pixels the pixels
/// are kept.
pub fn set_resolution(f: &mut Map<String, Value>, new_ppi: f32) {
    let old_ppi = get_f(f, "resolution", 72.0);
    f.insert("resolution".into(), json!(new_ppi));
    if old_ppi != new_ppi {
        f.remove("__savedPreset");
    }
    if get_s(f, "__unit", "px") == "px" || !(old_ppi > 0.0 && new_ppi > 0.0) || old_ppi == new_ppi {
        return;
    }
    let scale = new_ppi / old_ppi;
    let (w, h) = (get_f(f, "width", 1920.0), get_f(f, "height", 1080.0));
    f.insert("width".into(), px_value(w * scale));
    f.insert("height".into(), px_value(h * scale));
    f.remove("__preset");
    f.remove("__savedPreset");
}

fn get_f(f: &Map<String, Value>, k: &str, d: f32) -> f32 {
    f.get(k).and_then(Value::as_f64).map_or(d, |v| v as f32)
}

fn get_s(f: &Map<String, Value>, k: &str, d: &str) -> String {
    f.get(k).and_then(Value::as_str).unwrap_or(d).to_string()
}

fn small_label(ui: &mut egui::Ui, s: &str) {
    let t = Tokens::get(ui.ctx());
    ui.label(RichText::new(s).size(12.0).color(t.text_dim));
}

/// The dialog's content width: the four-column grid and Preset Details.
pub const DIALOG_W: f32 = 1040.0;

/// Preset card size and the gap between cards.
pub const CARD: egui::Vec2 = egui::Vec2::new(164.0, 150.0);
pub const CARD_GAP: f32 = 10.0;
/// Cards per row.
const COLUMNS: usize = 4;
/// Width of the Preset Details column.
const DETAILS_W: f32 = 290.0;
/// Height of the preset grid.
const GRID_H: f32 = 470.0;
/// Width of a Width / Height / Resolution field.
const FIELD_W: f32 = 118.0;

/// Most recent sizes kept for the Recent tab.
pub const RECENT_MAX: usize = 20;

/// Name of a recent size that didn't come from a preset.
pub const CUSTOM: &str = "Custom";

/// One card in the preset grid: a built-in preset, the clipboard size or a recent size.
#[derive(Clone, Debug, PartialEq)]
struct Card {
    name: String,
    w: u32,
    h: u32,
    ppi: f32,
    unit: String,
    /// A recent or clipboard size rather than a named page: drawn with crop marks.
    custom: bool,
}

impl Card {
    fn preset(p: &Preset) -> Self {
        // Print and photo presets are specified in inches, screen presets in pixels.
        let unit = if p.3 >= 300.0 { "in" } else { "px" };
        Self { name: p.0.to_string(), w: p.1, h: p.2, ppi: p.3, unit: unit.into(), custom: p.0 == CLIPBOARD }
    }

    /// "8.5 x 11 in @ 300 ppi" or "1920 x 1080 px @ 72 ppi".
    fn size_text(&self) -> String {
        match UNITS.iter().find(|u| u.0 == self.unit && u.2 > 0.0) {
            Some((key, _, _)) => format!(
                "{} x {} {key} @ {} ppi",
                widgets::fmt_num(to_unit(self.w as f32, key, self.ppi) as f64),
                widgets::fmt_num(to_unit(self.h as f32, key, self.ppi) as f64),
                self.ppi
            ),
            None => format!("{} x {} px @ {} ppi", self.w, self.h, self.ppi),
        }
    }

    fn apply(&self, f: &mut Map<String, Value>) {
        f.remove("__savedPreset");
        f.insert("width".into(), json!(self.w));
        f.insert("height".into(), json!(self.h));
        f.insert("resolution".into(), json!(self.ppi));
        f.insert("__preset".into(), json!(self.name));
        f.insert("__unit".into(), json!(self.unit));
        f.remove(&typed_key("width"));
        f.remove(&typed_key("height"));
    }

    /// The card is the dialog's current size.
    fn chosen(&self, f: &Map<String, Value>) -> bool {
        get_s(f, "__preset", "") == self.name
            && get_f(f, "width", 0.0) == self.w as f32
            && get_f(f, "height", 0.0) == self.h as f32
            && get_f(f, "resolution", 72.0) == self.ppi
    }
}

/// The recent sizes the dialog was opened with (`__recent`, from the preferences).
fn recent_cards(f: &Map<String, Value>) -> Vec<Card> {
    let Some(list) = f.get("__recent").and_then(Value::as_array) else { return Vec::new() };
    list.iter()
        .filter_map(|v| serde_json::from_value::<photocraft_engine::prefs::RecentDocumentSize>(v.clone()).ok())
        .filter(|r| r.width > 0 && r.height > 0 && r.resolution.is_finite() && r.resolution > 0.0)
        .take(RECENT_MAX)
        .map(|r| Card { name: r.name, w: r.width, h: r.height, ppi: r.resolution, unit: r.unit, custom: true })
        .collect()
}

/// Offer `recent` (newest first) under the Recent tab.
pub fn set_recent(f: &mut Map<String, Value>, recent: &[photocraft_engine::prefs::RecentDocumentSize]) {
    if !recent.is_empty() {
        f.insert("__recent".into(), serde_json::to_value(recent).unwrap_or_default());
    }
}

/// Show `background` (the toolbar's background colour, 0–1 RGB) in the Background Contents swatch.
pub fn set_background_color(f: &mut Map<String, Value>, background: [f32; 4]) {
    f.insert("__bgColor".into(), json!([background[0], background[1], background[2]]));
}

/// The size a confirmed dialog created, for the Recent tab: its preset's name, or "Custom".
pub fn recent_entry(f: &Map<String, Value>) -> photocraft_engine::prefs::RecentDocumentSize {
    let name = get_s(f, "__preset", "");
    let width = f.get("width").and_then(Value::as_u64).and_then(|v| u32::try_from(v).ok()).unwrap_or(1920);
    let height = f.get("height").and_then(Value::as_u64).and_then(|v| u32::try_from(v).ok()).unwrap_or(1080);
    let resolution = get_f(f, "resolution", 72.0);
    photocraft_engine::prefs::RecentDocumentSize {
        name: if name.is_empty() || name == CLIPBOARD { CUSTOM.into() } else { name },
        width,
        height,
        resolution: if resolution.is_finite() && resolution > 0.0 { resolution } else { 72.0 },
        unit: get_s(f, "__unit", "px"),
    }
}

/// Put `entry` first in `list`, dropping an older copy of the same size, and keep at most
/// [`RECENT_MAX`].
pub fn push_recent(list: &mut Vec<photocraft_engine::prefs::RecentDocumentSize>, entry: photocraft_engine::prefs::RecentDocumentSize) {
    list.retain(|r| !(r.width == entry.width && r.height == entry.height && r.resolution == entry.resolution));
    list.insert(0, entry);
    list.truncate(RECENT_MAX);
}

/// Lay out a preset card's title centred in `width`: wrapped onto at most two lines, the rest
/// elided, so long translations stay inside the card.
fn card_title(painter: &egui::Painter, title: &str, width: f32, color: egui::Color32) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple(title.to_owned(), egui::FontId::proportional(13.0), color, width);
    job.wrap.max_rows = 2;
    job.halign = egui::Align::Center;
    painter.layout_job(job)
}

/// Paint a page with a folded corner and the card's aspect ratio, centred in `r`; a custom size
/// also gets crop marks at its top-left corner.
fn page_icon(ui: &egui::Ui, r: Rect, w: u32, h: u32, custom: bool, color: egui::Color32) {
    let longest = w.max(h).max(1) as f32;
    let size = vec2((w as f32 / longest * r.width()).max(r.width() * 0.3), (h as f32 / longest * r.height()).max(r.height() * 0.3));
    let page = Rect::from_center_size(r.center(), size);
    let fold = (page.width().min(page.height()) * 0.28).clamp(5.0, 14.0);
    let stroke = Stroke::new(1.5, color);
    let p = ui.painter();
    let outline =
        vec![page.left_top(), page.right_top() - vec2(fold, 0.0), page.right_top() + vec2(0.0, fold), page.right_bottom(), page.left_bottom(), page.left_top()];
    p.add(egui::Shape::line(outline, stroke));
    p.add(egui::Shape::line(vec![page.right_top() - vec2(fold, 0.0), page.right_top() + vec2(-fold, fold), page.right_top() + vec2(0.0, fold)], stroke));
    if custom {
        p.line_segment([page.left_top() - vec2(0.0, 6.0), page.left_top() - vec2(0.0, 16.0)], stroke);
        p.line_segment([page.left_top() - vec2(6.0, 0.0), page.left_top() - vec2(16.0, 0.0)], stroke);
    }
}

/// The Background Contents swatch: the colour the new document's background gets.
fn background_swatch(ui: &mut egui::Ui, f: &Map<String, Value>, t: &Tokens) {
    let (r, _) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::hover());
    let rgb =
        |c: [f32; 3]| egui::Color32::from_rgb((c[0].clamp(0.0, 1.0) * 255.0) as u8, (c[1].clamp(0.0, 1.0) * 255.0) as u8, (c[2].clamp(0.0, 1.0) * 255.0) as u8);
    let bg_color = || {
        let c = f.get("backgroundColor").or_else(|| f.get("__bgColor")).and_then(Value::as_array)?;
        let ch = |i: usize| c.get(i).and_then(Value::as_f64).map(|v| v as f32);
        Some(rgb([ch(0)?, ch(1)?, ch(2)?]))
    };
    let p = ui.painter();
    let fill = match get_s(f, "background", "white").as_str() {
        "white" => Some(egui::Color32::WHITE),
        "black" => Some(egui::Color32::BLACK),
        "backgroundColor" => Some(bg_color().unwrap_or(egui::Color32::WHITE)),
        "transparent" => None,
        hex => photocraft_engine::prefs::parse_hex(hex).map(|c| egui::Color32::from_rgb(c[0], c[1], c[2])),
    };
    match fill {
        Some(c) => {
            p.rect_filled(r, 2.0, c);
        }
        None => {
            // Transparent: a checkerboard.
            let n = 4;
            let cell = r.width() / n as f32;
            for i in 0..n {
                for j in 0..n {
                    let c = if (i + j) % 2 == 0 { egui::Color32::from_gray(255) } else { egui::Color32::from_gray(204) };
                    p.rect_filled(Rect::from_min_size(r.min + vec2(i as f32 * cell, j as f32 * cell), vec2(cell, cell)), 0.0, c);
                }
            }
        }
    }
    p.rect_stroke(r, 2.0, Stroke::new(1.0, t.field_border), StrokeKind::Inside);
}

/// Restore a snapshot without replacing the new document's name or its clipboard offer.
pub fn apply_saved_preset(f: &mut Map<String, Value>, preset: &photocraft_engine::document_preset_cmds::DocumentPreset) {
    f.remove("backgroundColor");
    if let Some(params) = preset.settings.command_params().as_object() {
        f.extend(params.clone());
    }
    f.insert("__unit".into(), json!(preset.settings.unit));
    f.insert("__resUnit".into(), json!(preset.settings.resolution_unit));
    f.insert("__savedPreset".into(), json!(preset.name));
    f.remove("__preset");
    // Typed Width/Height belong to the previous values (`shown_size`).
    f.remove(&typed_key("width"));
    f.remove(&typed_key("height"));
}

fn saved_presets(app: &mut crate::PhotocraftApp, ui: &mut egui::Ui, f: &mut Map<String, Value>) {
    let presets = app.session.presets.documents.clone();
    if presets.is_empty() {
        ui.label(tl!("No saved presets yet."));
        return;
    }
    egui::ScrollArea::vertical().id_salt("saved-document-presets").max_height(320.0).show(ui, |ui| {
        for preset in &presets {
            ui.push_id(&preset.name, |ui| {
                ui.horizontal(|ui| {
                    let selected = get_s(f, "__savedPreset", "") == preset.name;
                    let label = egui::Button::new(&preset.name).selected(selected).truncate();
                    if ui.add_sized(vec2(422.0, 32.0), label).on_hover_text(&preset.name).clicked() {
                        apply_saved_preset(f, preset);
                    }
                    if widgets::secondary_button(ui, tl!("Delete"), 76.0).clicked() {
                        match app.run("document.presets.delete", json!({"name":preset.name})) {
                            Ok(_) => {
                                f.remove("__presetError");
                                if selected {
                                    f.remove("__savedPreset");
                                }
                            }
                            Err(e) => {
                                f.insert("__presetError".into(), json!(e.to_string()));
                            }
                        }
                    }
                });
                let s = &preset.settings;
                small_label(ui, &format!("{} × {} px @ {} ppi · {}/{}", s.width, s.height, s.resolution, s.mode.to_uppercase(), s.depth));
                ui.add_space(8.0);
            });
        }
    });
}

fn save_preset(app: &mut crate::PhotocraftApp, ui: &mut egui::Ui, f: &mut Map<String, Value>) {
    ui.add_space(12.0);
    if f.get("__savingPreset").and_then(Value::as_bool) != Some(true) {
        if widgets::secondary_button(ui, tl!("Save Preset…"), 240.0).clicked() {
            f.insert("__savingPreset".into(), json!(true));
            f.remove("__presetError");
        }
    } else {
        // Consume before TextEdit sees Enter, so naming can never confirm the outer dialog.
        let enter = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        let label = ui.label(tl!("Preset Name"));
        let mut name = get_s(f, "__presetName", "");
        ui.add(egui::TextEdit::singleline(&mut name).id_salt("document-preset-name").desired_width(240.0).char_limit(255)).labelled_by(label.id);
        f.insert("__presetName".into(), json!(name));
        ui.horizontal(|ui| {
            if widgets::secondary_button(ui, tl!("Save"), 110.0).clicked() || enter {
                let mut settings = command_params(f);
                settings["unit"] = json!(get_s(f, "__unit", "px"));
                settings["resolutionUnit"] = json!(get_s(f, "__resUnit", "in"));
                match app.run("document.presets.save", json!({"name":name,"settings":settings})) {
                    Ok(_) => {
                        if let Some(preset) = app.session.presets.documents.last() {
                            apply_saved_preset(f, preset);
                        }
                        f.insert("__category".into(), json!("Saved"));
                        f.remove("__savingPreset");
                        f.remove("__presetName");
                        f.remove("__presetError");
                        ui.ctx().request_repaint();
                    }
                    Err(e) => {
                        f.insert("__presetError".into(), json!(e.to_string()));
                    }
                }
            }
            if widgets::secondary_button(ui, tl!("Cancel"), 110.0).clicked() {
                f.remove("__savingPreset");
                f.remove("__presetError");
            }
        });
    }
    if let Some(error) = f.get("__presetError").and_then(Value::as_str) {
        ui.add(egui::Label::new(error).wrap());
    }
}

pub fn body(app: &mut crate::PhotocraftApp, ui: &mut egui::Ui, f: &mut Map<String, Value>) {
    let t = Tokens::get(ui.ctx());
    let cat = get_s(f, "__category", "Recent");
    // Category tabs; Recent carries a clock, as in Photoshop.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 20.0;
        for name in CATEGORIES.iter().map(|c| c.0).chain(["Saved"]) {
            let on = cat == name;
            let color = if on { t.text } else { t.text_dim };
            let r = ui
                .horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    let icon = (name == CATEGORIES[0].0).then(|| {
                        let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                        icons::paint(ui, r, "clock", 15.0, color);
                        r
                    });
                    let text = if name == "Saved" { tl!("Saved") } else { tl!(name) };
                    let label = ui.add(egui::Label::new(RichText::new(text).size(14.0).color(color)).sense(Sense::click()));
                    (icon, label)
                })
                .inner;
            let (icon, label) = r;
            let under = icon.map_or(label.rect, |i| i.union(label.rect));
            if on {
                ui.painter()
                    .line_segment([pos2(under.left(), label.rect.bottom() + 6.0), pos2(under.right(), label.rect.bottom() + 6.0)], Stroke::new(2.0, t.text));
            }
            if label.clicked() {
                f.insert("__category".into(), json!(name));
            }
        }
    });
    ui.add_space(8.0);
    widgets::hairline(ui);
    let presets = CATEGORIES.iter().find(|c| c.0 == cat).map_or(CATEGORIES[0].1, |c| c.1);
    let recent = if cat == CATEGORIES[0].0 { recent_cards(f) } else { Vec::new() };
    // The clipboard image's size comes first under Recent, then the recent sizes (or, before
    // any, the blank-document presets).
    let clipboard = clipboard_preset(f).filter(|_| cat == CATEGORIES[0].0).map(|p| Card::preset(&p));
    let heading = if recent.is_empty() { tl!("BLANK DOCUMENT PRESETS ({n})") } else { tl!("YOUR RECENT ITEMS ({n})") };
    let cards: Vec<Card> = clipboard.into_iter().chain(if recent.is_empty() { presets.iter().map(Card::preset).collect() } else { recent }).collect();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        // Left: the preset grid on a darker ground.
        let grid_w = COLUMNS as f32 * CARD.x + (COLUMNS - 1) as f32 * CARD_GAP;
        egui::Frame::NONE.fill(t.canvas).inner_margin(egui::Margin::symmetric(18, 16)).show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(grid_w);
                ui.set_height(GRID_H);
                if cat == "Saved" {
                    saved_presets(app, ui, f);
                    return;
                }
                ui.label(RichText::new(crate::i18n::fmt(heading, &[("n", &cards.len().to_string())])).size(12.5).color(t.text_dim));
                ui.add_space(10.0);
                egui::ScrollArea::vertical().id_salt("nd-grid").auto_shrink(false).show(ui, |ui| {
                    for row in cards.chunks(COLUMNS) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = CARD_GAP;
                            for card in row {
                                let (r, resp) = ui.allocate_exact_size(CARD, Sense::click());
                                let on = card.chosen(f);
                                if resp.hovered() && !on {
                                    ui.painter().rect_filled(r, t.radius, t.hover);
                                }
                                if on {
                                    ui.painter().rect_stroke(r, t.radius, Stroke::new(2.0, t.accent), StrokeKind::Inside);
                                }
                                page_icon(
                                    ui,
                                    Rect::from_center_size(pos2(r.center().x, r.top() + 52.0), vec2(58.0, 58.0)),
                                    card.w,
                                    card.h,
                                    card.custom,
                                    t.text_dim,
                                );
                                let name = if card.name == CUSTOM { tl!(CUSTOM).to_string() } else { tl!(&card.name).to_string() };
                                let title_color = if resp.hovered() { t.accent } else { t.text };
                                let title = card_title(ui.painter(), &name, CARD.x - 12.0, title_color);
                                let title_h = title.size().y;
                                ui.painter().galley(pos2(r.center().x, r.top() + 108.0 - title_h / 2.0), title, title_color);
                                let size = card.size_text();
                                ui.painter().text(
                                    pos2(r.center().x, r.top() + 132.0),
                                    Align2::CENTER_CENTER,
                                    &size,
                                    egui::FontId::proportional(12.0),
                                    t.text_dim,
                                );
                                let tip = crate::i18n::fmt(tl!("Start a new {name} document: {size}"), &[("name", &name), ("size", &size)]);
                                if resp.on_hover_text(tip).clicked() {
                                    card.apply(f);
                                }
                            }
                        });
                        ui.add_space(CARD_GAP);
                    }
                });
            })
        });
        // Right: Preset Details.
        egui::Frame::NONE.inner_margin(egui::Margin { left: 20, right: 4, top: 16, bottom: 0 }).show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(DETAILS_W);
                ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
                ui.label(RichText::new(tl!("PRESET DETAILS")).size(12.5).color(t.text_dim));
                ui.add_space(6.0);
                // The name sits on an underline, as in Photoshop.
                let mut name = get_s(f, "name", tl!("Untitled-1"));
                let resp =
                    ui.add(egui::TextEdit::singleline(&mut name).frame(egui::Frame::NONE).desired_width(DETAILS_W).font(egui::FontId::proportional(16.0)));
                if resp.changed() {
                    f.insert("name".into(), json!(name));
                }
                let line = resp.rect.bottom() + 3.0;
                ui.painter().line_segment(
                    [pos2(resp.rect.left(), line), pos2(resp.rect.left() + DETAILS_W, line)],
                    Stroke::new(1.0, if resp.has_focus() { t.accent } else { t.field_border }),
                );
                ui.add_space(12.0);
                let ppi = get_f(f, "resolution", 72.0);
                let mut unit = get_s(f, "__unit", "px");
                small_label(ui, tl!("Width"));
                ui.horizontal(|ui| {
                    let mut w = shown_size(f, "width", 1920.0, &unit, ppi);
                    if widgets::value_field(ui, &mut w, 0.01..=300_000.0, "", FIELD_W).changed() {
                        set_size(f, "width", w, &unit, ppi);
                    }
                    let opts: Vec<(String, &str)> = UNITS.iter().map(|u| (u.0.to_string(), u.1)).collect();
                    if widgets::dropdown(ui, "nd-unit", &mut unit, &opts, DETAILS_W - FIELD_W - 8.0) {
                        f.insert("__unit".into(), json!(unit));
                        f.remove("__savedPreset");
                    }
                });
                ui.add_space(4.0);
                // Height, with Orientation beside it: labels on one row, controls on the next.
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(vec2(FIELD_W, 16.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.set_width(FIELD_W);
                        small_label(ui, tl!("Height"));
                    });
                    small_label(ui, tl!("Orientation"));
                });
                ui.horizontal(|ui| {
                    let mut h = shown_size(f, "height", 1080.0, &unit, ppi);
                    if widgets::value_field(ui, &mut h, 0.01..=300_000.0, "", FIELD_W).changed() {
                        set_size(f, "height", h, &unit, ppi);
                    }
                    let (w, h) = (get_f(f, "width", 1920.0), get_f(f, "height", 1080.0));
                    for (icon, portrait) in [("rectangle-vertical", true), ("rectangle-horizontal", false)] {
                        if icons::button(ui, icon, 30.0, (h > w) == portrait, if portrait { "Portrait" } else { "Landscape" }).clicked() && (h > w) != portrait
                        {
                            swap_size(f);
                        }
                    }
                });
                ui.add_space(4.0);
                small_label(ui, tl!("Resolution"));
                ui.horizontal(|ui| {
                    let per_cm = get_s(f, "__resUnit", "in") == "cm";
                    let mut r = if per_cm { ppi / 2.54 } else { ppi };
                    if widgets::value_field(ui, &mut r, 1.0..=30_000.0, "", FIELD_W).changed() {
                        set_resolution(f, if per_cm { r * 2.54 } else { r });
                    }
                    let mut ru = get_s(f, "__resUnit", "in");
                    let opts = [("in".to_string(), tl!("Pixels/Inch")), ("cm".to_string(), tl!("Pixels/Centimeter"))];
                    if widgets::dropdown(ui, "nd-resunit", &mut ru, &opts, DETAILS_W - FIELD_W - 8.0) {
                        f.insert("__resUnit".into(), json!(ru));
                        f.remove("__savedPreset");
                    }
                });
                ui.add_space(4.0);
                small_label(ui, tl!("Color Mode"));
                ui.horizontal(|ui| {
                    let mut mode = get_s(f, "mode", "rgb");
                    let opts = [
                        ("gray".to_string(), tl!("Grayscale")),
                        ("rgb".to_string(), tl!("RGB Color")),
                        ("cmyk".to_string(), tl!("CMYK Color")),
                        ("lab".to_string(), tl!("Lab Color")),
                    ];
                    if widgets::dropdown(ui, "nd-mode", &mut mode, &opts, DETAILS_W * 0.6) {
                        f.insert("mode".into(), json!(mode));
                        f.remove("__savedPreset");
                    }
                    let mut depth = f.get("depth").and_then(Value::as_u64).unwrap_or(8);
                    let depth_options: Vec<(u64, &str, &str)> = DEPTH_OPTIONS.iter().map(|(bits, label, tooltip)| (*bits, *label, *tooltip)).collect();
                    if widgets::dropdown_with_tooltips(ui, "nd-depth", &mut depth, &depth_options, DETAILS_W * 0.4 - 8.0) {
                        f.insert("depth".into(), json!(depth));
                        f.remove("__savedPreset");
                    }
                });
                ui.add_space(4.0);
                small_label(ui, tl!("Background Contents"));
                ui.horizontal(|ui| {
                    let mut bg = get_s(f, "background", "white");
                    let opts = [
                        ("white".to_string(), tl!("White")),
                        ("black".to_string(), tl!("Black")),
                        ("backgroundColor".to_string(), tl!("Background Color")),
                        ("transparent".to_string(), tl!("Transparent")),
                    ];
                    if widgets::dropdown(ui, "nd-bg", &mut bg, &opts, DETAILS_W - 30.0 - 8.0) {
                        f.insert("background".into(), json!(bg));
                        f.remove("__savedPreset");
                        // A fresh choice of Background Color uses today's toolbox colour. Selecting
                        // a saved preset instead restores the colour captured when it was saved.
                        f.remove("backgroundColor");
                    }
                    background_swatch(ui, f, &t);
                });
                save_preset(app, ui, f);
            })
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_round_trip_through_pixels() {
        assert_eq!(to_unit(2100.0, "in", 300.0), 7.0);
        assert_eq!(from_unit(7.0, "in", 300.0), 2100.0);
        assert_eq!(from_unit(2.54, "cm", 300.0), 300.0);
        assert_eq!(to_unit(640.0, "px", 72.0), 640.0);
    }

    /// #1147: 5 mm at 72 ppi is 14 px, which is 4.94 mm. The field must keep showing the typed 5.
    #[test]
    fn typed_size_in_a_physical_unit_shows_as_typed() {
        let mut f = crate::state::UiState::new_document_fields();
        set_size(&mut f, "height", 5.0, "mm", 72.0);
        assert_eq!(command_params(&f)["height"], json!(14));
        assert_eq!(shown_size(&f, "height", 1080.0, "mm", 72.0), 5.0);
        // Another unit, or pixels changed elsewhere (a preset, the orientation swap): converted.
        assert_eq!(shown_size(&f, "height", 1080.0, "cm", 72.0), to_unit(14.0, "cm", 72.0));
        f.insert("height".into(), json!(1080));
        assert_eq!(shown_size(&f, "height", 1080.0, "mm", 72.0), to_unit(1080.0, "mm", 72.0));
        // Pixels show whole pixels.
        set_size(&mut f, "width", 512.4, "px", 72.0);
        assert_eq!(shown_size(&f, "width", 1920.0, "px", 72.0), 512.0);
        // Over the 300000 px limit the field shows the limit.
        set_size(&mut f, "width", 300_000.0, "mm", 72.0);
        assert_eq!(command_params(&f)["width"], json!(300_000));
        assert_eq!(shown_size(&f, "width", 1920.0, "mm", 72.0), to_unit(300_000.0, "mm", 72.0));
    }

    /// #1147: the Orientation buttons swap the typed values too: 841 x 1189 mm becomes 1189 x 841.
    #[test]
    fn orientation_swap_keeps_the_typed_values() {
        let mut f = crate::state::UiState::new_document_fields();
        set_size(&mut f, "width", 841.0, "mm", 72.0);
        set_size(&mut f, "height", 1189.0, "mm", 72.0);
        swap_size(&mut f);
        let p = command_params(&f);
        assert_eq!((p["width"].clone(), p["height"].clone()), (json!(3370), json!(2384)));
        assert_eq!((shown_size(&f, "width", 1920.0, "mm", 72.0), shown_size(&f, "height", 1080.0, "mm", 72.0)), (1189.0, 841.0));
    }

    #[test]
    fn resolution_keeps_physical_size_in_physical_units_and_pixels_in_px() {
        let a4 = CATEGORIES.iter().find(|c| c.0 == "Print").unwrap().1.iter().find(|p| p.0 == "A4").unwrap();
        let mut f = crate::state::UiState::new_document_fields();
        apply_preset(&mut f, a4);
        f.insert("__unit".into(), json!("in"));
        set_resolution(&mut f, 150.0);
        let p = command_params(&f);
        assert_eq!((p["width"].clone(), p["height"].clone(), p["resolution"].clone()), (json!(1240), json!(1754), json!(150.0)));
        assert!(!f.contains_key("__preset"));

        let mut f = crate::state::UiState::new_document_fields();
        apply_preset(&mut f, a4);
        f.insert("__unit".into(), json!("px"));
        set_resolution(&mut f, 150.0);
        let p = command_params(&f);
        assert_eq!((p["width"].clone(), p["height"].clone(), p["resolution"].clone()), (json!(2480), json!(3508), json!(150.0)));
    }

    #[test]
    fn preset_card_titles_fit_the_card_in_every_language() {
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            for lang in crate::i18n::Lang::all() {
                for p in CATEGORIES.iter().flat_map(|c| c.1.iter()) {
                    let title = crate::i18n::tr(lang, p.0);
                    let g = card_title(ui.painter(), title, 152.0, egui::Color32::WHITE);
                    assert!(g.size().x <= 152.0 && g.rows.len() <= 2 && !g.elided, "{}: {title}", lang.code());
                }
            }
        });
        out.textures_delta.clear();
    }

    #[test]
    fn new_document_depth_labels_and_tooltips_are_translated() {
        for lang in crate::i18n::Lang::all().filter(|lang| lang.code() != "en") {
            for (_, label, tooltip) in DEPTH_OPTIONS {
                assert_ne!(crate::i18n::tr(lang, label), *label, "{}: {label}", lang.code());
                assert_ne!(crate::i18n::tr(lang, tooltip), *tooltip, "{}: {tooltip}", lang.code());
            }
        }
    }

    #[test]
    fn clipboard_preset_comes_first_and_is_selected() {
        // Nothing on the clipboard: the dialog opens as before.
        let mut app = crate::PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
        let mut f = app.new_document_fields();
        assert!(f.remove("__bgColor").is_some(), "the Background Contents swatch gets the background colour");
        assert_eq!(f, crate::state::UiState::new_document_fields());
        assert!(clipboard_preset(&f).is_none());
        // Pixels copied in the app: the Clipboard preset takes their size, at 72 ppi, selected.
        app.run("file.new", json!({"width": 200, "height": 100})).unwrap();
        app.run("select.rect", json!({"x": 10, "y": 20, "width": 123, "height": 45})).unwrap();
        app.run("edit.copy", json!({})).unwrap();
        let f = app.new_document_fields();
        assert_eq!(clipboard_preset(&f), Some((CLIPBOARD, 123, 45, 72.0)));
        assert_eq!((f["width"].as_u64(), f["height"].as_u64(), f["__preset"].as_str()), (Some(123), Some(45), Some(CLIPBOARD)));
        let p = command_params(&f);
        assert!(p.get("__clipboard").is_none(), "file.new never sees the dialog's keys");
        assert_eq!((p["width"].as_u64(), p["height"].as_u64(), p["resolution"].as_f64()), (Some(123), Some(45), Some(72.0)));
    }

    #[test]
    fn preset_sets_size_resolution_and_create_params_drop_ui_keys() {
        let mut f = crate::state::UiState::new_document_fields();
        let a4 = CATEGORIES.iter().find(|c| c.0 == "Print").unwrap().1.iter().find(|p| p.0 == "A4").unwrap();
        apply_preset(&mut f, a4);
        let p = command_params(&f);
        assert_eq!(p["width"], 2480);
        assert_eq!(p["height"], 3508);
        assert_eq!(p["resolution"], 300.0);
        assert!(p.get("__preset").is_none() && p.get("__unit").is_none());
        let mut s = photocraft_engine::Session::new();
        s.execute("file.new", p).unwrap();
        let d = &s.active().unwrap().doc;
        assert_eq!((d.size.width, d.size.height, d.resolution_dpi), (2480, 3508, 300.0));
    }

    #[test]
    fn saved_selection_restores_all_settings_but_keeps_the_document_name() {
        let mut session = photocraft_engine::Session::new();
        session
            .execute(
                "document.presets.save",
                json!({"name":"Print proof","settings":{
                    "width":1600,"height":800,"resolution":254.0,"mode":"cmyk","depth":16,
                    "background":"backgroundColor","backgroundColor":[0.2,0.3,0.4],"unit":"mm","resolutionUnit":"cm"
                }}),
            )
            .unwrap();
        let preset = session.presets.documents[0].clone();
        let mut f = crate::state::UiState::new_document_fields();
        set_clipboard(&mut f, 20, 30);
        f.insert("name".into(), json!("Catalog cover"));
        apply_saved_preset(&mut f, &preset);
        assert_eq!(f["name"], "Catalog cover");
        assert_eq!(f["__unit"], "mm");
        assert_eq!(f["__resUnit"], "cm");
        assert_eq!(f["__savedPreset"], "Print proof");
        assert!(f.get("__preset").is_none());
        assert_eq!(clipboard_preset(&f), Some((CLIPBOARD, 20, 30, 72.0)));
        let mut expected = preset.settings.command_params();
        expected["name"] = json!("Catalog cover");
        assert_eq!(command_params(&f), expected);
        f.insert("width".into(), json!(42));
        assert_eq!(session.presets.documents[0], preset, "editing the form does not edit the snapshot");
        apply_preset(&mut f, &(CLIPBOARD, 20, 30, 72.0));
        assert!(f.get("__savedPreset").is_none());
        assert_eq!((f["width"].as_u64(), f["height"].as_u64()), (Some(20), Some(30)));
    }
    #[test]
    fn recent_sizes_put_the_newest_first_without_duplicates_and_are_capped() {
        let size = |w: u32| photocraft_engine::prefs::RecentDocumentSize { width: w, ..Default::default() };
        let mut list = Vec::new();
        for w in 1..=(RECENT_MAX as u32 + 5) {
            push_recent(&mut list, size(w));
        }
        assert_eq!(list.len(), RECENT_MAX);
        assert_eq!(list[0].width, RECENT_MAX as u32 + 5, "newest first");
        push_recent(&mut list, size(10));
        assert_eq!(list[0].width, 10);
        assert_eq!(list.iter().filter(|r| r.width == 10).count(), 1, "an older copy of the same size is dropped");
        assert_eq!(list.len(), RECENT_MAX);
    }

    #[test]
    fn a_created_size_is_named_after_its_preset_or_custom() {
        let mut f = crate::state::UiState::new_document_fields();
        let letter = CATEGORIES.iter().find(|c| c.0 == "Print").unwrap().1.iter().find(|p| p.0 == "Letter").unwrap();
        apply_preset(&mut f, letter);
        let r = recent_entry(&f);
        assert_eq!((r.name.as_str(), r.width, r.height, r.resolution, r.unit.as_str()), ("Letter", 2550, 3300, 300.0, "in"));
        assert_eq!(Card { name: r.name, w: r.width, h: r.height, ppi: r.resolution, unit: r.unit, custom: true }.size_text(), "8.5 x 11 in @ 300 ppi");
        set_size(&mut f, "width", 1718.0, "px", 72.0);
        assert_eq!(recent_entry(&f).name, CUSTOM, "typing a size makes it custom");
        set_clipboard(&mut f, 640, 360);
        assert_eq!(recent_entry(&f).name, CUSTOM, "the clipboard size is listed as custom");
    }

    #[test]
    fn recent_cards_ignore_bad_entries() {
        let mut f = crate::state::UiState::new_document_fields();
        f.insert(
            "__recent".into(),
            json!([{"name": "Custom", "width": 0, "height": 10}, {"resolution": -1}, "junk", {"name": "Custom", "width": 300, "height": 200, "resolution": 72.0, "unit": "px"}]),
        );
        let cards = recent_cards(&f);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].size_text(), "300 x 200 px @ 72 ppi");
    }

    /// The real dialog (#254): a typed size must reach `file.new`, however it is confirmed.
    mod dialog {
        use super::super::{CARD, CARD_GAP, CATEGORIES, apply_preset};
        use crate::PhotocraftApp;
        use crate::state::{DialogKind, UiState};
        use egui::accesskit::Role;
        use egui_kittest::{Harness, kittest::Queryable};
        use std::sync::{Arc, Mutex};

        /// The real preference services, backed by an in-memory store shared across app restarts.
        fn harness_with_store(store: &Arc<Mutex<Option<String>>>) -> Harness<'static, PhotocraftApp> {
            let (read, write) = (store.clone(), store.clone());
            let services = crate::Services {
                load_prefs: Some(Box::new(move || read.lock().unwrap().clone())),
                save_prefs: Some(Box::new(move |text| {
                    *write.lock().unwrap() = Some(text.to_string());
                    Ok(())
                })),
                ..Default::default()
            };
            let app = PhotocraftApp::new(photocraft_engine::Session::new(), services);
            let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(
                |ui, app| {
                    crate::prefs_ui::tick(app, ui.ctx());
                    crate::dialogs::show(app, ui.ctx());
                },
                app,
            );
            PhotocraftApp::setup_context(&h.ctx, crate::theme::ThemeKind::ALL[0]);
            h.state_mut().ui.open_dialog(DialogKind::NewDocument, UiState::new_document_fields());
            h.run_steps(3);
            h
        }

        fn name_preset(h: &mut Harness<'static, PhotocraftApp>, name: &str) {
            h.get_by_label("Save Preset…").click();
            h.run_steps(2);
            h.get_by_role_and_label(Role::TextInput, "Preset Name").click();
            h.run_steps(1);
            h.event(egui::Event::Text(name.into()));
            h.run_steps(1);
        }

        #[test]
        fn save_restart_select_create_and_delete_through_the_dialog() {
            let store = Arc::new(Mutex::new(None));
            let mut h = harness_with_store(&store);
            type_into(&mut h, 0, "1600");
            type_into(&mut h, 1, "1600");
            let mut f = fields(&h);
            f.insert("background".into(), serde_json::json!("transparent"));
            set_fields(&mut h, f);
            name_preset(&mut h, " Product square ");
            h.get_by_label("Save").click();
            h.run_steps(3);
            assert!(h.state().session.documents().is_empty(), "saving does not create a document");
            assert_eq!(h.state().session.presets.documents[0].name, "Product square");
            type_into(&mut h, 0, "32");
            assert_eq!(h.state().session.presets.documents[0].settings.width, 1600);
            drop(h);

            let mut h = harness_with_store(&store);
            h.get_by_label("Saved").click();
            h.run_steps(2);
            h.get_by_label("Product square").click();
            h.run_steps(2);
            assert!(h.state().session.documents().is_empty(), "selection only populates the form");
            assert_eq!(fields(&h)["name"], "Untitled-1");
            assert_eq!(fields(&h)["background"], "transparent");
            h.get_by_label("Create").click();
            h.run_steps(3);
            assert_eq!(created(&h), (1600, 1600, 72.0));
            let pixel = h.state_mut().session.execute("document.pixel", serde_json::json!({"x":0,"y":0})).unwrap();
            assert_eq!(pixel[3], 0.0);

            h.state_mut().ui.open_dialog(DialogKind::NewDocument, UiState::new_document_fields());
            h.run_steps(2);
            h.get_by_label("Saved").click();
            h.run_steps(2);
            h.get_by_label("Delete").click();
            h.run_steps(3);
            drop(h);
            let mut h = harness_with_store(&store);
            h.get_by_label("Saved").click();
            h.run_steps(2);
            assert!(h.state().session.presets.documents.is_empty());
            assert!(h.query_by_label("No saved presets yet.").is_some());
        }

        #[test]
        fn enter_saves_the_preset_and_bad_names_leave_the_form_open() {
            let mut h = harness();
            name_preset(&mut h, "Enter test");
            enter(&mut h);
            assert_eq!(h.state().session.presets.documents.len(), 1);
            assert!(h.state().session.documents().is_empty(), "Enter must not also press Create");
            assert_eq!(h.state().ui.dialogs.len(), 1);
            name_preset(&mut h, " ENTER TEST ");
            h.get_by_label("Save").click();
            h.run_steps(2);
            assert!(fields(&h).get("__presetError").is_some());
            assert_eq!(h.state().session.presets.documents.len(), 1);
            assert!(h.state().session.documents().is_empty());
            h.get_by_label("Cancel").click();
            h.run_steps(2);
            name_preset(&mut h, "");
            // Cancel keeps the entered name; clear it as an automation edit, then use the button.
            h.state_mut().ui.dialogs[0].fields.insert("__presetName".into(), serde_json::json!("   "));
            h.run_steps(2);
            h.get_by_label("Save").click();
            h.run_steps(2);
            assert!(fields(&h).get("__presetError").is_some());
            assert_eq!(h.state().session.presets.documents.len(), 1);
        }

        fn harness() -> Harness<'static, PhotocraftApp> {
            let app = PhotocraftApp::new(photocraft_engine::Session::new(), crate::Services::default());
            let mut h = Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(|ui, app| crate::dialogs::show(app, ui.ctx()), app);
            PhotocraftApp::setup_context(&h.ctx, crate::theme::ThemeKind::ALL[0]);
            h.state_mut().ui.open_dialog(DialogKind::NewDocument, UiState::new_document_fields());
            h.run_steps(3);
            h
        }

        fn click_at(h: &mut Harness<'static, PhotocraftApp>, at: egui::Pos2) {
            h.hover_at(at);
            h.run_steps(1);
            h.drag_at(at);
            h.run_steps(1);
            h.drop_at(at);
            h.run_steps(2);
        }

        /// The Width (0), Height (1) and Resolution (2) fields.
        fn field(h: &Harness<'static, PhotocraftApp>, i: usize) -> egui::Rect {
            h.query_all_by_role(Role::SpinButton).nth(i).map(|n| n.rect()).expect("a size field")
        }

        /// Click into field `i` (which selects its text) and type `text`, as a user does.
        fn type_into(h: &mut Harness<'static, PhotocraftApp>, i: usize, text: &str) {
            let r = field(h, i);
            click_at(h, r.center());
            for c in text.chars() {
                h.event(egui::Event::Text(c.to_string()));
                h.run_steps(1);
            }
        }

        fn fields(h: &Harness<'static, PhotocraftApp>) -> serde_json::Map<String, serde_json::Value> {
            h.state().ui.dialogs.first().map(|d| d.fields.clone()).expect("the dialog is open")
        }

        fn set_fields(h: &mut Harness<'static, PhotocraftApp>, f: serde_json::Map<String, serde_json::Value>) {
            h.state_mut().ui.dialogs[0].fields = f;
            h.run_steps(2);
        }

        fn enter(h: &mut Harness<'static, PhotocraftApp>) {
            h.key_press(egui::Key::Enter);
            h.run_steps(3);
        }

        fn created(h: &Harness<'static, PhotocraftApp>) -> (u32, u32, f32) {
            assert!(h.state().ui.dialogs.is_empty(), "the dialog closed");
            let d = &h.state().session.active().expect("a new document").doc;
            (d.size.width, d.size.height, d.resolution_dpi)
        }

        /// Created sizes are listed under Recent the next time File › New opens, and survive in the
        /// preferences.
        #[test]
        fn a_created_size_is_listed_under_recent_next_time() {
            let mut h = harness();
            type_into(&mut h, 0, "1718");
            type_into(&mut h, 1, "1071");
            enter(&mut h);
            assert_eq!(created(&h), (1718, 1071, 72.0));
            let recent = h.state().session.prefs().file_handling.recent_new_documents.clone();
            assert_eq!(recent.first().map(|r| (r.name.as_str(), r.width, r.height)), Some(("Custom", 1718, 1071)));
            let f = h.state_mut().new_document_fields();
            h.state_mut().ui.open_dialog(DialogKind::NewDocument, f);
            h.run_steps(3);
            assert!(h.query_by_label_contains("YOUR RECENT ITEMS (1)").is_some());
            // Clicking the card picks its size.
            let heading = h.get_by_label_contains("YOUR RECENT ITEMS").rect();
            type_into(&mut h, 0, "64");
            click_at(&mut h, heading.left_bottom() + egui::vec2(80.0, 60.0));
            enter(&mut h);
            assert_eq!(created(&h), (1718, 1071, 72.0));
        }

        #[test]
        fn typed_size_then_enter_creates_that_size() {
            let mut h = harness();
            type_into(&mut h, 0, "512");
            type_into(&mut h, 1, "512");
            let f = fields(&h);
            assert_eq!((f["width"].as_u64(), f["height"].as_u64()), (Some(512), Some(512)), "whole pixels: {f:?}");
            enter(&mut h);
            assert_eq!(created(&h), (512, 512, 72.0));
        }

        #[test]
        fn typed_size_then_create_without_leaving_the_field_creates_that_size() {
            let mut h = harness();
            type_into(&mut h, 0, "512");
            type_into(&mut h, 1, "300");
            // Still editing Height: click Create straight away.
            let create = h.get_by_label("Create").rect();
            click_at(&mut h, create.center());
            assert_eq!(created(&h), (512, 300, 72.0));
        }

        #[test]
        fn typing_over_a_preset_wins() {
            let mut h = harness();
            let mut f = fields(&h);
            let web = CATEGORIES.iter().find(|c| c.0 == "Web").unwrap().1.iter().find(|p| p.0 == "Web Minimum").unwrap();
            apply_preset(&mut f, web);
            set_fields(&mut h, f);
            type_into(&mut h, 0, "512");
            type_into(&mut h, 1, "512");
            assert!(fields(&h).get("__preset").is_none(), "typing deselects the preset");
            enter(&mut h);
            assert_eq!(created(&h), (512, 512, 72.0));
        }

        #[test]
        fn the_clipboard_card_is_first_and_creates_the_clipboard_size() {
            let mut h = harness();
            let mut f = fields(&h);
            super::super::set_clipboard(&mut f, 640, 360);
            set_fields(&mut h, f);
            // Recent lists the Clipboard card first: three presets instead of two.
            assert!(h.query_by_label_contains("BLANK DOCUMENT PRESETS (3)").is_some());
            let heading = h.get_by_label_contains("BLANK DOCUMENT PRESETS").rect();
            // Pick the second card, then the first (Clipboard) again.
            click_at(&mut h, heading.left_bottom() + egui::vec2(80.0 + CARD.x + CARD_GAP, 60.0));
            assert_eq!(fields(&h).get("__preset").and_then(|v| v.as_str()), Some("Default Photoshop Size"));
            click_at(&mut h, heading.left_bottom() + egui::vec2(80.0, 60.0));
            assert_eq!(fields(&h).get("__preset").and_then(|v| v.as_str()), Some(super::super::CLIPBOARD));
            enter(&mut h);
            assert_eq!(created(&h), (640, 360, 72.0));
        }

        #[test]
        fn clicking_a_preset_card_after_typing_sets_its_size() {
            let mut h = harness();
            type_into(&mut h, 0, "512");
            h.get_by_label("Photo").click();
            h.run_steps(2);
            // The first card ("Landscape, 6 x 4") sits under the presets heading.
            let heading = h.get_by_label_contains("BLANK DOCUMENT PRESETS").rect();
            click_at(&mut h, heading.left_bottom() + egui::vec2(80.0, 60.0));
            assert_eq!(fields(&h).get("__preset").and_then(|v| v.as_str()), Some("Landscape, 6 x 4"));
            enter(&mut h);
            assert_eq!(created(&h), (1800, 1200, 300.0));
        }

        #[test]
        fn typed_size_in_inches_converts_at_the_resolution() {
            let mut h = harness();
            type_into(&mut h, 2, "300");
            let mut f = fields(&h);
            f.insert("__unit".into(), serde_json::json!("in"));
            set_fields(&mut h, f);
            type_into(&mut h, 0, "2");
            type_into(&mut h, 1, "1.5");
            enter(&mut h);
            assert_eq!(created(&h), (600, 450, 300.0));
        }

        /// #1147: typed digit by digit, millimetres used to be rewritten after each keystroke
        /// (`8` became `8.1`), so 841 never arrived.
        #[test]
        fn typed_size_in_millimetres_keeps_every_digit() {
            let mut h = harness();
            let mut f = fields(&h);
            f.insert("__unit".into(), serde_json::json!("mm"));
            set_fields(&mut h, f);
            type_into(&mut h, 0, "841");
            type_into(&mut h, 1, "1189");
            enter(&mut h);
            assert_eq!(created(&h), (2384, 3370, 72.0));
        }

        #[test]
        fn changing_units_keeps_the_typed_pixel_size() {
            let mut h = harness();
            type_into(&mut h, 0, "512");
            type_into(&mut h, 1, "512");
            for unit in ["in", "cm", "mm", "pt", "pica", "px"] {
                let mut f = fields(&h);
                f.insert("__unit".into(), serde_json::json!(unit));
                set_fields(&mut h, f);
            }
            enter(&mut h);
            assert_eq!(created(&h), (512, 512, 72.0));
        }

        #[test]
        fn orientation_swap_keeps_whole_pixels() {
            let mut h = harness();
            type_into(&mut h, 0, "512");
            type_into(&mut h, 1, "256");
            // The Portrait icon button follows the Height field, under the "Orientation" label
            // (icons have tooltips only).
            let height = field(&h, 1);
            let label = h.get_by_label("Orientation").rect();
            click_at(&mut h, egui::pos2(label.left() + 15.0, height.center().y));
            enter(&mut h);
            assert_eq!(created(&h), (256, 512, 72.0));
        }
    }
}
