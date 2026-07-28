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
use regex::Regex;
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
/// GNU gettext only honors `LANGUAGE` once localization is enabled, i.e. the
/// locale is not "C". On Unix the desktop session enables it; on Windows the
/// CRT starts in the "C" locale, so without `setlocale(LC_ALL, "")` every
/// string silently stays English.
pub fn apply_ui_language(id: &str) {
    set_language_env(id);
    enable_locale();
}

/// Enable a non-"C" locale so GNU gettext honors `LANGUAGE`.
///
/// On Windows this goes through the C library directly: gettext-rs
/// `setlocale` resolves to libintl's own override, which aborts the process
/// (MSVC CRT vs libintl).
fn enable_locale() {
    #[cfg(windows)]
    unsafe {
        libc::setlocale(libc::LC_ALL, c"".as_ptr());
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
    let mut patterns: Vec<PlaceholderPattern> = Vec::new();
    for (entry, old) in entries.iter().zip(previous) {
        if old.is_empty() {
            continue;
        }
        let new = entry.translate();
        if new == old {
            continue;
        }
        match PlaceholderPattern::new(&old, &new) {
            // Rendered placeholder text (`12 首歌曲`) never equals the catalog
            // translation (`{num} 首歌曲`), so it goes to the pattern list.
            Some(pattern) => patterns.push(pattern),
            // First entry wins so that ambiguous translations stay deterministic.
            None => {
                map.entry(old).or_insert(new);
            }
        }
    }

    Retranslator { map, patterns }
}

/// Rewrites already-built widgets from one language to another.
pub struct Retranslator {
    map: HashMap<String, String>,
    patterns: Vec<PlaceholderPattern>,
}

impl Retranslator {
    pub fn is_empty(&self) -> bool {
        self.map.is_empty() && self.patterns.is_empty()
    }

    pub fn translate(&self, current: &str) -> Option<String> {
        if let Some(exact) = self.map.get(current) {
            return Some(exact.clone());
        }
        // A label may combine several rendered placeholders (`12 首歌曲, 34
        // 收藏`), so every pattern gets a pass over the running text.
        let mut text = current.to_string();
        let mut matched = false;
        for pattern in &self.patterns {
            if let Some(out) = pattern.translate(&text) {
                text = out;
                matched = true;
            }
        }
        matched.then_some(text)
    }

    /// Retranslate `widget` and its whole subtree, including stack page titles
    /// and attached popovers.
    pub fn retranslate_widget(&self, widget: &impl IsA<gtk::Widget>) {
        if self.is_empty() {
            return;
        }
        self.walk(widget.as_ref());
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

/// A translation carrying `{name}` placeholders (see `utils::gettext_f`).
///
/// Widgets show the rendered text (`12 首歌曲`), which never equals the
/// catalog translation (`{num} 首歌曲`), so the exact map cannot reach them.
struct PlaceholderPattern {
    regex: Regex,
    /// Capture group `v{i}` holds the rendered value of placeholder `vars[i]`.
    vars: Vec<String>,
    template: String,
}

impl PlaceholderPattern {
    /// Compile `old` into a matcher: `{name}` runs become non-greedy capture
    /// groups, everything else stays literal. `None` when there is no
    /// placeholder to capture.
    fn new(old: &str, new: &str) -> Option<Self> {
        let mut pattern = String::new();
        let mut vars = Vec::new();
        let mut rest = old;
        while let Some(start) = rest.find('{') {
            let Some(end) = rest[start..].find('}').map(|offset| start + offset) else {
                break;
            };
            let name = &rest[start + 1..end];
            let is_placeholder = !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_');
            if is_placeholder {
                pattern.push_str(&regex::escape(&rest[..start]));
                pattern.push_str(&format!("(?P<v{}>.*?)", vars.len()));
                vars.push(name.to_string());
            } else {
                // Literal braces, kept as plain text.
                pattern.push_str(&regex::escape(&rest[..end + 1]));
            }
            rest = &rest[end + 1..];
        }
        if vars.is_empty() {
            return None;
        }
        pattern.push_str(&regex::escape(rest));
        Some(Self {
            regex: Regex::new(&pattern).ok()?,
            vars,
            template: new.to_string(),
        })
    }

    /// Render every match in `text` from `template`, substituting the captured
    /// placeholder values. `None` when nothing matched.
    fn translate(&self, text: &str) -> Option<String> {
        let mut matched = false;
        let out = self.regex.replace_all(text, |captures: &regex::Captures| {
            matched = true;
            let mut rendered = self.template.clone();
            for (index, name) in self.vars.iter().enumerate() {
                let value = captures
                    .name(&format!("v{index}"))
                    .map(|capture| capture.as_str())
                    .unwrap_or_default();
                rendered = rendered.replace(&format!("{{{name}}}"), value);
            }
            rendered
        });
        matched.then(|| out.into_owned())
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
    sync_crt_language_env();
}

/// Mirror the Win32 `LANGUAGE` value into the CRT environment.
///
/// Rust's `std::env::set_var` only updates the Win32 environment block, but
/// libintl resolves `LANGUAGE` through the CRT's own copy, so without this
/// the override is invisible to gettext on Windows.
#[cfg(windows)]
fn sync_crt_language_env() {
    match std::env::var("LANGUAGE") {
        Ok(value) => unsafe {
            let value = std::ffi::CString::new(value).unwrap_or_default();
            libc::putenv_s(c"LANGUAGE".as_ptr(), value.as_ptr());
        },
        Err(_) => unsafe {
            libc::putenv_s(c"LANGUAGE".as_ptr(), c"".as_ptr());
        },
    }
}

#[cfg(not(windows))]
fn sync_crt_language_env() {}

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

    fn retranslator_for(old: &str, new: &str) -> Retranslator {
        Retranslator {
            map: HashMap::new(),
            patterns: vec![PlaceholderPattern::new(old, new).unwrap()],
        }
    }

    #[test]
    fn placeholder_pattern_renders_zh_to_en() {
        let retranslator = retranslator_for("{num} 首歌曲", "{num} songs");
        assert_eq!(
            retranslator.translate("12 首歌曲").as_deref(),
            Some("12 songs")
        );
    }

    #[test]
    fn placeholder_pattern_renders_en_to_zh() {
        let retranslator = retranslator_for("{num} songs", "{num} 首歌曲");
        assert_eq!(
            retranslator.translate("12 songs").as_deref(),
            Some("12 首歌曲")
        );
    }

    #[test]
    fn placeholder_patterns_cover_combined_labels() {
        let retranslator = Retranslator {
            map: HashMap::new(),
            patterns: vec![
                PlaceholderPattern::new("{num} 首歌曲", "{num} songs").unwrap(),
                PlaceholderPattern::new("{num} 收藏", "{num} favs").unwrap(),
            ],
        };
        assert_eq!(
            retranslator.translate("12 首歌曲, 34 收藏").as_deref(),
            Some("12 songs, 34 favs")
        );
    }

    #[test]
    fn literal_braces_are_not_placeholders() {
        assert!(PlaceholderPattern::new("use {} here", "benutze {} hier").is_none());
    }

    #[test]
    fn non_matching_text_is_untouched() {
        let retranslator = retranslator_for("{num} 首歌曲", "{num} songs");
        assert_eq!(retranslator.translate("随机播放"), None);
    }

    #[test]
    fn exact_match_wins_over_patterns() {
        let mut retranslator = retranslator_for("{num} 首歌曲", "{num} songs");
        retranslator
            .map
            .insert("12 首歌曲".to_string(), "twelve songs".to_string());
        assert_eq!(
            retranslator.translate("12 首歌曲").as_deref(),
            Some("twelve songs")
        );
    }
}
