//
// typography.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use gettextrs::gettext;
use gtk::{
    CssProvider, STYLE_PROVIDER_PRIORITY_USER, gdk::Display, gio::Settings, glib,
    prelude::SettingsExt, style_context_add_provider_for_display,
};
use once_cell::sync::OnceCell;
use std::cell::RefCell;

const BASE_PT: f64 = 11.0;
const LYRICS_BASE_PT: f64 = 13.0;
const TITLE_RATIO: f64 = 0.95;
const META_RATIO: f64 = 0.8;
const DURATION_RATIO: f64 = 0.75;

/// Ordered preset ids; index matches Preferences ComboRow.
pub const FONT_PRESET_IDS: &[&str] = &[
    "system",
    "sans",
    "serif",
    "mono",
    "cjk-sans",
    "source-han",
];

const FONT_SCALE_KEYS: &[&str] = &[
    "ui-font-preset",
    "ui-font-scale",
    "list-title-font-scale",
    "list-meta-font-scale",
    "player-font-scale",
    "lyrics-font-scale",
];

thread_local! {
    static PROVIDER: RefCell<Option<CssProvider>> = const { RefCell::new(None) };
}

static LISTENER: OnceCell<()> = OnceCell::new();

pub fn preset_label(id: &str) -> String {
    match id {
        "system" => gettext("System default"),
        "sans" => gettext("Sans"),
        "serif" => gettext("Serif"),
        "mono" => gettext("Monospace"),
        "cjk-sans" => gettext("CJK Sans"),
        "source-han" => gettext("Source Han Sans"),
        _ => id.to_string(),
    }
}

fn preset_css_family(id: &str) -> Option<&'static str> {
    match id {
        "system" => None,
        "sans" => Some("sans-serif"),
        "serif" => Some("serif"),
        "mono" => Some("monospace"),
        "cjk-sans" => Some(
            "\"Microsoft YaHei UI\", \"Microsoft YaHei\", \"PingFang SC\", \"Noto Sans CJK SC\", \"Source Han Sans SC\", sans-serif",
        ),
        "source-han" => Some(
            "\"Source Han Sans SC\", \"Noto Sans CJK SC\", \"Microsoft YaHei\", sans-serif",
        ),
        _ => None,
    }
}

fn clamp_scale(value: f64) -> f64 {
    value.clamp(0.8, 1.5)
}

fn fmt_pt(pt: f64) -> String {
    // Snap to 0.5pt so glyphs land nearer device-pixel baselines (less blur).
    let snapped = (pt * 2.0).round() / 2.0;
    format!("{:.1}pt", snapped)
}

fn pt_to_px(pt: f64) -> i32 {
    // CSS px at 96dpi; integer px keeps multiline baselines aligned.
    ((pt * 2.0).round() / 2.0 * 96.0 / 72.0)
        .round()
        .max(1.0) as i32
}

fn fmt_px(pt: f64) -> String {
    format!("{}px", pt_to_px(pt))
}

fn fmt_line_height_px(pt: f64) -> String {
    let font_px = pt_to_px(pt) as f64;
    let line_px = (font_px * 1.35).round().max(font_px + 2.0) as i32;
    format!("{}px", line_px)
}

pub fn lyrics_font_pt(settings: &Settings) -> f64 {
    LYRICS_BASE_PT * clamp_scale(settings.double("lyrics-font-scale"))
}

pub fn lyrics_highlight_pango_size(settings: &Settings) -> i32 {
    (lyrics_font_pt(settings) * f64::from(gtk::pango::SCALE)).round() as i32
}

pub fn lyrics_font_family_for_tag(settings: &Settings) -> Option<&'static str> {
    match settings.string("ui-font-preset").as_str() {
        "sans" => Some("sans-serif"),
        "serif" => Some("serif"),
        "mono" => Some("monospace"),
        "cjk-sans" => Some("Microsoft YaHei UI"),
        "source-han" => Some("Source Han Sans SC"),
        _ => None,
    }
}

fn build_css(settings: &Settings) -> String {
    let preset = settings.string("ui-font-preset");
    let ui = clamp_scale(settings.double("ui-font-scale"));
    let list_title = clamp_scale(settings.double("list-title-font-scale"));
    let list_meta = clamp_scale(settings.double("list-meta-font-scale"));
    let player = clamp_scale(settings.double("player-font-scale"));
    let lyrics = clamp_scale(settings.double("lyrics-font-scale"));

    let base = BASE_PT * ui;
    let title_pt = base * list_title * TITLE_RATIO;
    let meta_pt = base * list_meta * META_RATIO;
    let duration_pt = base * list_meta * DURATION_RATIO;
    let player_pt = base * player;
    let lyrics_pt = LYRICS_BASE_PT * lyrics;

    let mut css = String::new();
    css.push_str("window {\n");
    if let Some(family) = preset_css_family(preset.as_str()) {
        css.push_str(&format!("  font-family: {};\n", family));
    }
    css.push_str(&format!("  font-size: {};\n", fmt_pt(base)));
    css.push_str("}\n");

    css.push_str(&format!(
        ".song_row.activatable .song-title,\n.title-3 {{\n  font-size: {};\n}}\n",
        fmt_pt(title_pt)
    ));
    // Discover/grid card titles wrap to 2 lines; integer px + line-height
    // avoids half-pixel second baselines (common at 125% Windows DPI).
    css.push_str(&format!(
        ".songlist-card-title {{\n  font-size: {};\n  line-height: {};\n}}\n",
        fmt_px(title_pt),
        fmt_line_height_px(title_pt)
    ));
    css.push_str(&format!(
        ".song_row.activatable .song-meta,\n.label-album-grid-artist {{\n  font-size: {};\n}}\n",
        fmt_pt(meta_pt)
    ));
    css.push_str(&format!(
        ".song_row.activatable .song-duration {{\n  font-size: {};\n}}\n",
        fmt_pt(duration_pt)
    ));
    css.push_str(&format!(
        ".ncm-player-controls {{\n  font-size: {};\n}}\n",
        fmt_pt(player_pt)
    ));

    css.push_str(".ncm-lyrics {\n");
    if let Some(family) = preset_css_family(preset.as_str()) {
        css.push_str(&format!("  font-family: {};\n", family));
    }
    css.push_str(&format!("  font-size: {};\n", fmt_pt(lyrics_pt)));
    css.push_str("}\n");

    css
}

pub fn apply(settings: &Settings) {
    let css = build_css(settings);
    PROVIDER.with(|cell| {
        if let Some(provider) = cell.borrow().as_ref() {
            provider.load_from_data(&css);
        }
    });
}

/// Install USER-priority CssProvider once, apply current settings, and listen for changes.
pub fn init_and_apply(settings: &Settings) {
    PROVIDER.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            let provider = CssProvider::new();
            if let Some(display) = Display::default() {
                style_context_add_provider_for_display(
                    &display,
                    &provider,
                    STYLE_PROVIDER_PRIORITY_USER,
                );
            }
            *slot = Some(provider);
        }
    });

    apply(settings);

    if LISTENER.set(()).is_ok() {
        for key in FONT_SCALE_KEYS {
            settings.connect_changed(
                Some(*key),
                glib::clone!(
                    #[strong]
                    settings,
                    move |_, _| {
                        apply(&settings);
                    }
                ),
            );
        }
    }
}
