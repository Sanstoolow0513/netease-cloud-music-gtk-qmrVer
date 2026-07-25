//
// i18n.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use crate::config::GETTEXT_PACKAGE;
use gettextrs::{bind_textdomain_codeset, bindtextdomain, gettext, pgettext};
use gtk::gio::{self, prelude::*};
use gtk::{glib, prelude::*};
use once_cell::sync::{Lazy, OnceCell};
use std::cell::Cell;
use std::collections::HashMap;
use std::path::PathBuf;

/// Ordered language ids; index matches Preferences ComboRow.
pub const LANGUAGE_IDS: &[&str] = &["system", "zh_CN", "en"];

/// String properties holding translated, user-visible text.
///
/// `text` is deliberately absent: it carries user input (search entry, captcha)
/// and must never be rewritten.
const TEXT_PROPERTIES: &[&str] = &[
    "label",
    "tooltip-text",
    "title",
    "subtitle",
    "description",
    "placeholder-text",
];

static LOCALE_DIR: OnceCell<PathBuf> = OnceCell::new();
static ORIGINAL_LANGUAGE: OnceCell<Option<String>> = OnceCell::new();

thread_local! {
    /// `bindtextdomain` only invalidates cached catalogs when the directory
    /// string differs, so two spellings of the same directory are alternated.
    static ALTERNATE_BINDING: Cell<bool> = const { Cell::new(false) };
}

pub fn language_label(id: &str) -> String {
    match id {
        "system" => gettext("System default"),
        "zh_CN" => "简体中文".to_string(),
        "en" => "English".to_string(),
        _ => id.to_string(),
    }
}

/// Remember the locale directory and the inherited `LANGUAGE` value.
///
/// Must run before [`apply_from_settings`], on the main thread, before GTK
/// worker threads exist.
pub fn init(locale_dir: impl Into<PathBuf>) {
    let _ = LOCALE_DIR.set(locale_dir.into());
    let _ = ORIGINAL_LANGUAGE.set(std::env::var("LANGUAGE").ok());
}

/// Apply display language via the `LANGUAGE` environment variable.
///
/// Must run before `bindtextdomain` / `textdomain`, on the main thread, before
/// GTK worker threads exist (same env-var safety window as
/// `platform::initialize_runtime`).
///
/// On Unix we also call `setlocale(LC_ALL, "")` so libc and gettext agree.
/// On Windows, gettext-rs `setlocale` can abort (MSVC CRT vs libintl); GNU
/// gettext still honors `LANGUAGE` without it.
pub fn apply_ui_language(id: &str) {
    set_language_env(id);

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

/// Switch the display language without restarting.
///
/// The returned [`Retranslator`] maps every catalog string as rendered by the
/// previous language onto the new one; widgets built earlier keep their old
/// text until they are walked with it.
pub fn switch_ui_language(id: &str) -> Retranslator {
    let entries = catalog_entries();
    let previous: Vec<String> = entries.iter().map(CatalogEntry::translate).collect();

    set_language_env(id);
    reload_catalog();

    let mut map: HashMap<String, String> = HashMap::with_capacity(entries.len());
    for (entry, old) in entries.iter().zip(previous) {
        if old.is_empty() {
            continue;
        }
        let new = entry.translate();
        if new != old {
            // First entry wins so that ambiguous translations stay deterministic.
            map.entry(old).or_insert(new);
        }
    }

    Retranslator { map }
}

/// Rewrites already-built widgets from one language to another.
pub struct Retranslator {
    map: HashMap<String, String>,
}

impl Retranslator {
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn translate(&self, current: &str) -> Option<&str> {
        self.map.get(current).map(String::as_str)
    }

    /// Retranslate `widget` and its whole subtree, including stack page titles
    /// and attached popovers.
    pub fn retranslate_widget(&self, widget: &impl IsA<gtk::Widget>) {
        if self.map.is_empty() {
            return;
        }
        self.walk(widget.as_ref());
    }

    /// Build a copy of `model` with translated item labels.
    ///
    /// Menu models cache the strings they were built with, so the owning
    /// popover has to be rebuilt from the returned copy.
    pub fn retranslate_menu_model(&self, model: &gio::MenuModel) -> gio::Menu {
        let menu = gio::Menu::new();
        for index in 0..model.n_items() {
            let item = gio::MenuItem::new(None, None);

            let attributes = model.iterate_item_attributes(index);
            while let Some((name, value)) = attributes.next() {
                let translated = if name == "label" {
                    value.str().and_then(|label| self.translate(label))
                } else {
                    None
                };
                let value = match translated {
                    Some(label) => label.to_variant(),
                    None => value,
                };
                item.set_attribute_value(&name, Some(&value));
            }

            let links = model.iterate_item_links(index);
            while let Some((name, link)) = links.next() {
                item.set_link(&name, Some(&self.retranslate_menu_model(&link)));
            }

            menu.append_item(&item);
        }
        menu
    }

    fn walk(&self, widget: &gtk::Widget) {
        self.retranslate_object(widget.upcast_ref());

        if let Some(stack) = widget.downcast_ref::<gtk::Stack>() {
            self.retranslate_items(&stack.pages());
        }
        if let Some(stack) = widget.downcast_ref::<adw::ViewStack>() {
            self.retranslate_items(&stack.pages());
        }

        let mut child = widget.first_child();
        while let Some(current) = child {
            self.walk(&current);
            child = current.next_sibling();
        }
    }

    fn retranslate_items(&self, model: &impl IsA<gio::ListModel>) {
        let model = model.as_ref();
        for index in 0..model.n_items() {
            if let Some(item) = model.item(index) {
                self.retranslate_object(&item);
            }
        }
    }

    fn retranslate_object(&self, object: &glib::Object) {
        for name in TEXT_PROPERTIES {
            let Some(pspec) = object.find_property(name) else {
                continue;
            };
            if pspec.value_type() != glib::types::Type::STRING {
                continue;
            }
            let flags = pspec.flags();
            if !flags.contains(glib::ParamFlags::READABLE)
                || !flags.contains(glib::ParamFlags::WRITABLE)
                || flags.contains(glib::ParamFlags::CONSTRUCT_ONLY)
            {
                continue;
            }

            let current = object
                .property_value(name)
                .get::<Option<String>>()
                .ok()
                .flatten();
            if let Some(translated) = current.as_deref().and_then(|text| self.translate(text)) {
                object.set_property(name, translated);
            }
        }
    }
}

fn set_language_env(id: &str) {
    // SAFETY: called from the main thread. At startup no worker thread exists
    // yet; when switching at runtime other threads may read the environment
    // concurrently, which is the unavoidable cost of gettext having no API to
    // select a language programmatically.
    match id {
        "zh_CN" | "en" => unsafe { std::env::set_var("LANGUAGE", id) },
        // "system" and unknown ids restore whatever the environment provided,
        // so a user-set LANGUAGE keeps working.
        _ => match ORIGINAL_LANGUAGE.get().and_then(Option::as_deref) {
            Some(original) => unsafe { std::env::set_var("LANGUAGE", original) },
            None => unsafe { std::env::remove_var("LANGUAGE") },
        },
    }
}

/// Force gettext to re-resolve the catalog for the current `LANGUAGE`.
///
/// GNU gettext (and glibc) cache loaded catalogs and only re-read the
/// environment when their internal generation counter changes. `bindtextdomain`
/// bumps that counter, but only when the bound directory actually differs, so
/// the path alternates between `<dir>` and `<dir>/.` on every switch.
fn reload_catalog() {
    let Some(dir) = LOCALE_DIR.get() else {
        return;
    };

    let alternate = ALTERNATE_BINDING.with(|flag| {
        let next = !flag.get();
        flag.set(next);
        next
    });
    let dir = if alternate { dir.join(".") } else { dir.clone() };

    if bindtextdomain(GETTEXT_PACKAGE, &dir).is_ok() {
        let _ = bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8");
    }
}

/// A `msgctxt` / `msgid` pair from the shipped catalog.
struct CatalogEntry {
    context: Option<String>,
    msgid: String,
}

impl CatalogEntry {
    fn translate(&self) -> String {
        match &self.context {
            Some(context) => pgettext(context, &self.msgid),
            None => gettext(&self.msgid),
        }
    }
}

/// The catalog is the only source of strings that may be retranslated, so the
/// shipped translation is parsed for its msgids.
fn catalog_entries() -> &'static [CatalogEntry] {
    static ENTRIES: Lazy<Vec<CatalogEntry>> =
        Lazy::new(|| parse_po_msgids(include_str!("../po/zh_CN.po")));
    &ENTRIES
}

#[derive(PartialEq)]
enum PoField {
    None,
    Context,
    Msgid,
}

fn parse_po_msgids(source: &str) -> Vec<CatalogEntry> {
    let mut entries = Vec::new();
    let mut context: Option<String> = None;
    let mut msgid: Option<String> = None;
    let mut field = PoField::None;

    for line in source.lines() {
        let line = line.trim();
        // Comments also terminate a multi-line string; obsolete entries ("#~")
        // never reach the catalog.
        if line.is_empty() || line.starts_with('#') {
            field = PoField::None;
            continue;
        }

        if let Some(rest) = line.strip_prefix("msgctxt ") {
            push_entry(&mut entries, &mut context, &mut msgid);
            context = Some(unquote_po(rest));
            field = PoField::Context;
        } else if line.starts_with("msgid_plural ") {
            field = PoField::None;
        } else if let Some(rest) = line.strip_prefix("msgid ") {
            push_entry(&mut entries, &mut context, &mut msgid);
            msgid = Some(unquote_po(rest));
            field = PoField::Msgid;
        } else if line.starts_with("msgstr") {
            push_entry(&mut entries, &mut context, &mut msgid);
            field = PoField::None;
        } else if line.starts_with('"') {
            let text = unquote_po(line);
            match field {
                PoField::Context => {
                    if let Some(context) = context.as_mut() {
                        context.push_str(&text);
                    }
                }
                PoField::Msgid => {
                    if let Some(msgid) = msgid.as_mut() {
                        msgid.push_str(&text);
                    }
                }
                PoField::None => {}
            }
        }
    }

    push_entry(&mut entries, &mut context, &mut msgid);
    entries
}

/// Finish the entry being parsed, if any. A pending `msgctxt` without a
/// `msgid` belongs to the entry that follows, so it is left in place.
fn push_entry(
    entries: &mut Vec<CatalogEntry>,
    context: &mut Option<String>,
    msgid: &mut Option<String>,
) {
    let Some(msgid) = msgid.take() else {
        return;
    };
    let context = context.take();
    // The header entry has an empty msgid.
    if !msgid.is_empty() {
        entries.push(CatalogEntry { context, msgid });
    }
}

fn unquote_po(text: &str) -> String {
    let text = text.trim();
    let inner = text
        .strip_prefix('"')
        .and_then(|text| text.strip_suffix('"'))
        .unwrap_or(text);

    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(current) = chars.next() {
        if current != '\\' {
            out.push(current);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some(escaped) => out.push(escaped),
            None => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_entries() {
        let entries = parse_po_msgids(
            r#"
msgid ""
msgstr "Project-Id-Version: test\n"

#: data/gtk/window.ui:112
msgid "Discover"
msgstr "发现"
"#,
        );

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].msgid, "Discover");
        assert!(entries[0].context.is_none());
    }

    #[test]
    fn joins_multiline_msgid_and_keeps_context() {
        let entries = parse_po_msgids(
            r#"
msgctxt "shortcut window"
msgid "Fullscreen"
msgstr "全屏"

msgid "song:{name}\n"
"singer:{singer}"
msgstr "歌曲:{name}\n"
"歌手:{singer}"
"#,
        );

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].context.as_deref(), Some("shortcut window"));
        assert_eq!(entries[1].msgid, "song:{name}\nsinger:{singer}");
    }

    #[test]
    fn skips_obsolete_entries() {
        let entries = parse_po_msgids(
            r#"
msgid "Retry"
msgstr "重试"

#~ msgid "{} songs, {} favs"
#~ msgstr "{} 首歌曲，{} 收藏"
"#,
        );

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].msgid, "Retry");
    }

    #[test]
    fn shipped_catalog_is_parsed() {
        let entries = parse_po_msgids(include_str!("../po/zh_CN.po"));
        assert!(entries.iter().any(|entry| entry.msgid == "Discover"));
        assert!(entries.iter().any(|entry| entry.msgid == "_Preferences"));
        assert!(
            entries
                .iter()
                .any(|entry| entry.context.as_deref() == Some("shortcut window"))
        );
    }
}
