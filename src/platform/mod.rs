//
// platform/mod.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::process::Command;

pub const HAS_MPRIS: bool = cfg!(target_os = "linux");
pub const HAS_SYSTEM_TRAY: bool = cfg!(target_os = "linux");
pub const HAS_DESKTOP_LYRICS: bool = cfg!(target_os = "linux");

#[derive(Debug, Clone)]
pub struct RuntimePaths {
    pub locale_dir: PathBuf,
    pub resource_file: PathBuf,
}

pub fn initialize_runtime() -> Result<RuntimePaths> {
    #[cfg(target_os = "windows")]
    {
        let executable = std::env::current_exe().context("Unable to locate the executable")?;
        let root = executable
            .parent()
            .context("Unable to locate the application directory")?;
        return Ok(initialize_windows_runtime(root));
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(RuntimePaths {
            locale_dir: PathBuf::from(crate::config::LOCALEDIR),
            resource_file: PathBuf::from(crate::config::PKGDATADIR)
                .join("netease-cloud-music-gtk4.gresource"),
        })
    }
}

#[cfg(target_os = "windows")]
fn initialize_windows_runtime(root: &Path) -> RuntimePaths {
    let lib_dir = root.join("lib");
    let share_dir = root.join("share");
    let plugin_dir = lib_dir.join("gstreamer-1.0");
    let plugin_scanner = root
        .join("libexec")
        .join("gstreamer-1.0")
        .join("gst-plugin-scanner.exe");
    let schema_dir = share_dir.join("glib-2.0").join("schemas");
    let gio_modules_dir = lib_dir.join("gio").join("modules");
    let pixbuf_dir = lib_dir
        .join("gdk-pixbuf-2.0")
        .join("2.10.0")
        .join("loaders");
    let path = std::env::var_os("PATH")
        .map(|value| {
            let mut entries = vec![root.to_path_buf()];
            entries.extend(std::env::split_paths(&value));
            std::env::join_paths(entries).unwrap_or(value)
        })
        .unwrap_or_else(|| root.as_os_str().to_owned());

    // SAFETY: this runs at the very beginning of main, before GTK/GStreamer
    // initialization or any worker thread is created.
    unsafe {
        std::env::set_var("PATH", path);
    }

    let pixbuf_cache = prepare_pixbuf_cache(root, &pixbuf_dir);

    // SAFETY: runtime variables are still configured before native
    // initialization or worker thread creation.
    unsafe {
        std::env::set_var("GSETTINGS_SCHEMA_DIR", &schema_dir);
        std::env::set_var("XDG_DATA_DIRS", &share_dir);
        std::env::set_var("GST_PLUGIN_PATH", &plugin_dir);
        std::env::set_var("GST_PLUGIN_PATH_1_0", &plugin_dir);
        std::env::set_var("GST_PLUGIN_SYSTEM_PATH", &plugin_dir);
        std::env::set_var("GST_PLUGIN_SYSTEM_PATH_1_0", &plugin_dir);
        if plugin_scanner.is_file() {
            std::env::set_var("GST_PLUGIN_SCANNER", &plugin_scanner);
            std::env::set_var("GST_PLUGIN_SCANNER_1_0", &plugin_scanner);
        }
        if gio_modules_dir.is_dir() {
            std::env::set_var("GIO_EXTRA_MODULES", &gio_modules_dir);
        }
        if let Some(pixbuf_cache) = pixbuf_cache {
            std::env::set_var("GDK_PIXBUF_MODULEDIR", &pixbuf_dir);
            std::env::set_var("GDK_PIXBUF_MODULE_FILE", &pixbuf_cache);
        }
    }

    windows_runtime_paths(root)
}

#[cfg(target_os = "windows")]
fn windows_runtime_paths(root: &Path) -> RuntimePaths {
    let share_dir = root.join("share");
    RuntimePaths {
        locale_dir: share_dir.join("locale"),
        resource_file: share_dir
            .join("netease-cloud-music-gtk4")
            .join("netease-cloud-music-gtk4.gresource"),
    }
}

#[cfg(target_os = "windows")]
fn prepare_pixbuf_cache(root: &Path, module_dir: &Path) -> Option<PathBuf> {
    let query_loaders = root.join("gdk-pixbuf-query-loaders.exe");
    if !query_loaders.is_file() || !module_dir.is_dir() {
        return None;
    }

    let output = Command::new(query_loaders)
        .env("GDK_PIXBUF_MODULEDIR", module_dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let cache_dir = gtk::glib::user_cache_dir().join(crate::config::GETTEXT_PACKAGE);
    std::fs::create_dir_all(&cache_dir).ok()?;
    let cache_file = cache_dir.join("gdk-pixbuf-loaders.cache");
    std::fs::write(&cache_file, output.stdout).ok()?;
    Some(cache_file)
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "windows")]
    #[test]
    fn windows_runtime_paths_are_relative_to_executable() {
        let root = std::path::Path::new(r"C:\portable\ncm");
        let paths = super::windows_runtime_paths(root);

        assert_eq!(paths.locale_dir, root.join("share").join("locale"));
        assert_eq!(
            paths.resource_file,
            root.join("share")
                .join("netease-cloud-music-gtk4")
                .join("netease-cloud-music-gtk4.gresource")
        );
    }
}
