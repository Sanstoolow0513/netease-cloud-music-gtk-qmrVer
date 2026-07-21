//
// platform/mod.rs
// Copyright (C) 2022 gmg137 <gmg137 AT live.com>
// Distributed under terms of the GPL-3.0-or-later license.
//

#[cfg(target_os = "windows")]
use anyhow::Context;
use anyhow::Result;
#[cfg(target_os = "windows")]
use log::warn;
#[cfg(target_os = "windows")]
use std::ffi::OsString;
#[cfg(target_os = "windows")]
use std::path::Path;
use std::path::PathBuf;
#[cfg(target_os = "windows")]
use std::process::Command;

// Stubs keep platform-neutral call paths compilable. Capability flags are
// reserved for user-visible behavior that cannot be represented by a no-op.
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
fn prepend_search_path(path: &Path, existing: Option<OsString>) -> OsString {
    let mut entries = vec![path.to_path_buf()];
    if let Some(existing) = existing {
        entries.extend(std::env::split_paths(&existing));
    }
    std::env::join_paths(entries).unwrap_or_else(|_| path.as_os_str().to_owned())
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
    let path = prepend_search_path(root, std::env::var_os("PATH"));
    let xdg_data_dirs = prepend_search_path(&share_dir, std::env::var_os("XDG_DATA_DIRS"));

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
        std::env::set_var("XDG_DATA_DIRS", xdg_data_dirs);
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
    if !query_loaders.is_file() {
        warn!(
            "GdkPixbuf loader query tool is missing: {}",
            query_loaders.display()
        );
        return None;
    }
    if !module_dir.is_dir() {
        warn!(
            "GdkPixbuf loader directory is missing: {}",
            module_dir.display()
        );
        return None;
    }

    let output = match Command::new(&query_loaders)
        .env("GDK_PIXBUF_MODULEDIR", module_dir)
        .output()
    {
        Ok(output) => output,
        Err(err) => {
            warn!("Unable to run {}: {err}", query_loaders.display());
            return None;
        }
    };
    if !output.status.success() {
        warn!(
            "{} failed with {}: {}",
            query_loaders.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
        return None;
    }

    let cache_dir = gtk::glib::user_cache_dir().join(crate::config::GETTEXT_PACKAGE);
    if let Err(err) = std::fs::create_dir_all(&cache_dir) {
        warn!(
            "Unable to create GdkPixbuf cache directory {}: {err}",
            cache_dir.display()
        );
        return None;
    }
    let cache_file = cache_dir.join("gdk-pixbuf-loaders.cache");
    if let Err(err) = std::fs::write(&cache_file, output.stdout) {
        warn!(
            "Unable to write GdkPixbuf loader cache {}: {err}",
            cache_file.display()
        );
        return None;
    }
    Some(cache_file)
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "windows")]
    #[test]
    fn search_path_prepends_portable_directory() {
        let portable = std::path::PathBuf::from(r"C:\portable\ncm\share");
        let system_paths = [
            std::path::PathBuf::from(r"C:\system\share"),
            std::path::PathBuf::from(r"D:\extra\share"),
        ];
        let existing = std::env::join_paths(&system_paths).unwrap();

        let joined = super::prepend_search_path(&portable, Some(existing));
        let paths = std::env::split_paths(&joined).collect::<Vec<_>>();

        assert_eq!(paths[0], portable);
        assert_eq!(&paths[1..], system_paths.as_slice());
    }

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
