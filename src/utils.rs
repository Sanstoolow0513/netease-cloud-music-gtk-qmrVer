use gettextrs::gettext;
use glib::{SourceId, timeout_add_seconds};
use gtk::glib;
use ncm_api::{SongCopyright, SongInfo, SongQualityState};
use std::sync::{Arc, Mutex};

/// Like `gettext`, but replaces named variables with the given dictionary.
///
/// The expected format to replace is `{name}`, where `name` is the first string
/// in the dictionary entry tuple.
/// 使用 xtr 生成 pot 文件时需添加参数: xtr -k gettext_f -k gettext -o NAME.pot src/main.rs
pub fn gettext_f(msgid: &str, args: &[(&str, &str)]) -> String {
    let s = gettext(msgid);
    freplace(s, args)
}

/// Replace variables in the given string with the given dictionary.
///
/// The expected format to replace is `{name}`, where `name` is the first string
/// in the dictionary entry tuple.
pub fn freplace(s: String, args: &[(&str, &str)]) -> String {
    let mut s = s;

    for (k, v) in args {
        s = s.replace(&format!("{{{k}}}"), v);
    }

    s
}

#[derive(Debug)]
pub struct Debounce {
    timer_id: Arc<Mutex<Option<SourceId>>>,
}

impl Debounce {
    pub fn new() -> Self {
        Self {
            timer_id: Arc::new(Mutex::new(None)),
        }
    }
    pub fn debounce<F>(&self, delay: u32, callback: F)
    where
        F: Fn() + 'static + Send,
    {
        let timer_id_clone = self.timer_id.clone();

        if let Some(source_id) = timer_id_clone.lock().unwrap().take() {
            source_id.remove();
        }

        let timer_id_closure = timer_id_clone.clone();
        let new_timer_id = timeout_add_seconds(delay, move || {
            callback();
            timer_id_closure.lock().unwrap().take();
            glib::ControlFlow::Break
        });

        let mut guard = timer_id_clone.lock().unwrap();
        *guard = Some(new_timer_id);
    }
}

impl Default for Debounce {
    fn default() -> Self {
        Self::new()
    }
}

/// Default header page names in UI order.
pub const DEFAULT_HEADER_PAGES: [&str; 3] = ["discover", "toplist", "my"];

/// Sanitize a stored pages-order value into a permutation of the three header pages.
pub fn sanitize_pages_order(order: impl IntoIterator<Item = impl AsRef<str>>) -> Vec<String> {
    let mut result = Vec::with_capacity(DEFAULT_HEADER_PAGES.len());
    for name in order {
        let name = name.as_ref();
        if DEFAULT_HEADER_PAGES.contains(&name) && !result.iter().any(|s| s == name) {
            result.push(name.to_string());
        }
    }
    for &name in &DEFAULT_HEADER_PAGES {
        if !result.iter().any(|s| s == name) {
            result.push(name.to_string());
        }
    }
    result
}

/// Whether a header page should be visible given the show-* settings.
/// "my" is always visible.
pub fn header_page_visible(name: &str, show_discover: bool, show_toplist: bool) -> bool {
    match name {
        "discover" => show_discover,
        "toplist" => show_toplist,
        "my" => true,
        _ => false,
    }
}

/// First visible page name in the given order; falls back to "my".
pub fn first_visible_header_page(
    order: &[String],
    show_discover: bool,
    show_toplist: bool,
) -> String {
    order
        .iter()
        .find(|name| header_page_visible(name, show_discover, show_toplist))
        .cloned()
        .unwrap_or_else(|| "my".to_string())
}

pub fn empty_song_info() -> SongInfo {
    SongInfo {
        id: 0,
        name: String::new(),
        singer: String::new(),
        album: String::new(),
        album_id: 0,
        pic_url: String::new(),
        duration: 0,
        song_url: String::new(),
        quality: SongQualityState::default(),
        copyright: SongCopyright::Unknown,
    }
}
