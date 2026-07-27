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
//! 主题自带明暗属性：`default` 跟随系统，其余均为完整皮肤
//! （整套语义色板、前景背景全部显式指定），切换时同步设置
//! `AdwStyleManager` 的 color-scheme（深肤 FORCE_DARK、亮肤
//! FORCE_LIGHT），保证皮肤未覆盖的 Libadwaita 内部色（选中态、
//! 滚动条等）与 modern.css token 派生基准保持一致。

use gettextrs::gettext;
use gtk::{
    CssProvider, STYLE_PROVIDER_PRIORITY_APPLICATION, gdk::Display, gio::Settings, glib,
    prelude::SettingsExt, style_context_add_provider_for_display,
    style_context_remove_provider_for_display,
};
use once_cell::sync::OnceCell;
use std::cell::RefCell;

/// 主题明暗属性：决定切换时同步到 StyleManager 的 color-scheme。
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThemeScheme {
    /// 跟随系统（仅 default）
    System,
    Light,
    Dark,
}

/// 主题元数据；`THEMES` 顺序即首选项 ComboRow 顺序。
pub struct ThemeMeta {
    pub id: &'static str,
    pub scheme: ThemeScheme,
    /// 预览色块（bg, accent）：供首选项下拉绘制主题色小色块。
    /// default 不用（画半明半暗分割），仍给一组占位值。
    pub preview: (&'static str, &'static str),
}

pub const THEMES: &[ThemeMeta] = &[
    ThemeMeta {
        id: "default",
        scheme: ThemeScheme::System,
        preview: ("#ffffff", "#3584e4"),
    },
    ThemeMeta {
        id: "midnight",
        scheme: ThemeScheme::Dark,
        preview: ("#1a1a1e", "#ec4141"),
    },
    ThemeMeta {
        id: "abyss",
        scheme: ThemeScheme::Dark,
        preview: ("#111a24", "#3a9bdc"),
    },
    ThemeMeta {
        id: "one-dark",
        scheme: ThemeScheme::Dark,
        preview: ("#282c34", "#61afef"),
    },
    ThemeMeta {
        id: "nord",
        scheme: ThemeScheme::Dark,
        preview: ("#2e3440", "#88c0d0"),
    },
    ThemeMeta {
        id: "solarized-dark",
        scheme: ThemeScheme::Dark,
        preview: ("#002b36", "#268bd2"),
    },
    ThemeMeta {
        id: "paper",
        scheme: ThemeScheme::Light,
        preview: ("#f7f5f2", "#ec4141"),
    },
    ThemeMeta {
        id: "matcha",
        scheme: ThemeScheme::Light,
        preview: ("#f2f5ee", "#6f9e57"),
    },
    ThemeMeta {
        id: "github-light",
        scheme: ThemeScheme::Light,
        preview: ("#f6f8fa", "#0969da"),
    },
    ThemeMeta {
        id: "solarized-light",
        scheme: ThemeScheme::Light,
        preview: ("#eee8d5", "#268bd2"),
    },
];

thread_local! {
    static PROVIDER: RefCell<Option<CssProvider>> = const { RefCell::new(None) };
}

static LISTENER: OnceCell<()> = OnceCell::new();

pub fn theme_label(id: &str) -> String {
    match id {
        "default" => gettext("Default"),
        "midnight" => gettext("Midnight"),
        "abyss" => gettext("Abyss"),
        "one-dark" => gettext("One Dark"),
        "nord" => gettext("Nord"),
        "solarized-dark" => gettext("Solarized Dark"),
        "paper" => gettext("Paper"),
        "matcha" => gettext("Matcha"),
        "github-light" => gettext("GitHub Light"),
        "solarized-light" => gettext("Solarized Light"),
        _ => id.to_string(),
    }
}

/// 预览色块用色（bg, accent）；default 返回 None，由调用方画半明半暗。
pub fn theme_preview(id: &str) -> Option<(&'static str, &'static str)> {
    let meta = THEMES.iter().find(|m| m.id == id)?;
    (meta.scheme != ThemeScheme::System).then_some(meta.preview)
}

fn theme_resource(id: &str) -> Option<String> {
    match id {
        "default" => None,
        // 仅对登记过的主题生成资源路径：旧版本已删除的主题 id
        // （如 netease-red）残留于配置时按 default 处理，避免
        // load_from_resource 因资源不存在而 panic。
        other if THEMES.iter().any(|m| m.id == other) => Some(format!(
            "/com/gitee/gmg137/NeteaseCloudMusicGtk4/themes/theme-{other}.css"
        )),
        _ => None,
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

/// 同步 color-scheme：default 跟随系统，完整皮肤强制自身明暗，
/// 保证皮肤未覆盖的 Libadwaita 内部色与皮肤基调一致。
fn apply_color_scheme(id: &str) {
    let scheme = match THEMES.iter().find(|m| m.id == id).map(|m| m.scheme) {
        Some(ThemeScheme::Light) => adw::ColorScheme::ForceLight,
        Some(ThemeScheme::Dark) => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    };
    adw::StyleManager::default().set_color_scheme(scheme);
}

pub fn apply(settings: &Settings) {
    remove_provider();
    let id = settings.string("ui-theme");
    apply_color_scheme(id.as_str());
    let Some(resource) = theme_resource(id.as_str()) else {
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
