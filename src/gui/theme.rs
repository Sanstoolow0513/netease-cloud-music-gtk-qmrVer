//
// theme.rs
// Copyright (C) 2026 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

//! 可切换配色主题：`ui-theme` 键选择预设主题，运行时以
//! `STYLE_PROVIDER_PRIORITY_APPLICATION + 1` 叠加对应 CSS
//! （仅含 `@define-color` 覆盖），高于 modern.css 的 APPLICATION
//! 优先级使其生效；切回 `default` 时移除 provider 完全还原。
//!
//! 两层主题：自适应主题（netease-red/ocean/forest）只覆盖 accent
//! 三件套、跟随明/暗变体；完整皮肤（midnight/paper/abyss/matcha）
//! 覆盖整套语义色板、效果自我包含，盖过 style-variant 明/暗设置。

use gettextrs::gettext;
use gtk::{
    CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION, gdk::Display, gio::Settings, glib,
    prelude::SettingsExt, style_context_add_provider_for_display,
    style_context_remove_provider_for_display,
};
use once_cell::sync::OnceCell;
use std::cell::RefCell;

/// Ordered theme ids; index matches Preferences ComboRow.
/// 两层机制：default/netease-red/ocean/forest 为自适应主题（仅覆盖
/// accent，跟随明/暗变体）；midnight/paper/abyss/matcha 为完整皮肤
/// （整套色板，自带明暗，盖过 style-variant 的视觉效果）。
pub const THEME_IDS: &[&str] = &[
    "default",
    "netease-red",
    "ocean",
    "forest",
    "midnight",
    "paper",
    "abyss",
    "matcha",
];

thread_local! {
    static PROVIDER: RefCell<Option<CssProvider>> = const { RefCell::new(None) };
}

static LISTENER: OnceCell<()> = OnceCell::new();

pub fn theme_label(id: &str) -> String {
    match id {
        "default" => gettext("Default"),
        "netease-red" => gettext("NetEase Red"),
        "ocean" => gettext("Ocean"),
        "forest" => gettext("Forest"),
        "midnight" => gettext("Midnight"),
        "paper" => gettext("Paper"),
        "abyss" => gettext("Abyss"),
        "matcha" => gettext("Matcha"),
        _ => id.to_string(),
    }
}

fn theme_resource(id: &str) -> Option<String> {
    match id {
        "default" => None,
        other => Some(format!(
            "/com/gitee/gmg137/NeteaseCloudMusicGtk4/themes/theme-{other}.css"
        )),
    }
}

fn remove_provider() {
    PROVIDER.with(|cell| {
        if let Some(provider) = cell.borrow_mut().take()
            && let Some(display) = Display::default()
        {
            style_context_remove_provider_for_display(&display, &provider);
        }
    });
}

pub fn apply(settings: &Settings) {
    remove_provider();
    let Some(resource) = theme_resource(settings.string("ui-theme").as_str()) else {
        return;
    };
    let provider = CssProvider::new();
    provider.load_from_resource(resource.as_str());
    if let Some(display) = Display::default() {
        style_context_add_provider_for_display(
            &display,
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
        );
        PROVIDER.with(|cell| *cell.borrow_mut() = Some(provider));
    }
}

/// Apply the current ui-theme once and listen for changes (hot switch).
pub fn init_and_apply(settings: &Settings) {
    apply(settings);

    if LISTENER.set(()).is_ok() {
        settings.connect_changed(
            Some("ui-theme"),
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
