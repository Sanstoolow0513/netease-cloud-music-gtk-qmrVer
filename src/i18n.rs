//
// i18n.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use gettextrs::gettext;
use gtk::gio::{self, prelude::SettingsExt};

/// Ordered language ids; index matches Preferences ComboRow.
pub const LANGUAGE_IDS: &[&str] = &["system", "zh_CN", "en"];

pub fn language_label(id: &str) -> String {
    match id {
        "system" => gettext("System default"),
        "zh_CN" => "简体中文".to_string(),
        "en" => "English".to_string(),
        _ => id.to_string(),
    }
}

/// Apply display language via the `LANGUAGE` environment variable.
///
/// Must run before `bindtextdomain` / `textdomain`, on the main thread, before
/// GTK worker threads exist (same env-var safety window as `platform::initialize_runtime`).
///
/// On Unix we also call `setlocale(LC_ALL, "")` so libc and gettext agree.
/// On Windows, gettext-rs `setlocale` can abort (MSVC CRT vs libintl); GNU
/// gettext still honors `LANGUAGE` without it.
pub fn apply_ui_language(id: &str) {
    match id {
        "zh_CN" => {
            // SAFETY: single-threaded startup, before gettext/GTK init.
            unsafe {
                std::env::set_var("LANGUAGE", "zh_CN");
            }
        }
        "en" => {
            // SAFETY: single-threaded startup, before gettext/GTK init.
            unsafe {
                std::env::set_var("LANGUAGE", "en");
            }
        }
        // "system" and unknown: leave LANGUAGE alone so the OS locale is used.
        _ => {}
    }

    #[cfg(not(windows))]
    {
        use gettextrs::{LocaleCategory, setlocale};
        setlocale(LocaleCategory::LcAll, "");
    }
}

/// Read `ui-language` from GSettings and apply it.
pub fn apply_from_settings() {
    let settings = gio::Settings::new(crate::APP_ID);
    let language = settings.string("ui-language");
    apply_ui_language(language.as_str());
}
