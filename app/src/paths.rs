//! Where Sofa keeps its files, by the XDG base directory rules (inside a Flatpak these point into the app's own
//! folder): the library in the data folder, settings in the config folder, the session in the state folder.

use std::path::PathBuf;

use crate::APP_ID;

fn base(var: &str, fallback: &str) -> Option<PathBuf> {
    std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(fallback)))
        .map(|dir| dir.join(APP_ID))
}

/// `~/.local/share/<app ID>`: the library database (and QtWebEngine's profile, which Qt puts there itself).
pub fn data_dir() -> Option<PathBuf> {
    base("XDG_DATA_HOME", ".local/share")
}

/// `~/.config/<app ID>`: settings.
pub fn config_dir() -> Option<PathBuf> {
    base("XDG_CONFIG_HOME", ".config")
}

/// `~/.local/state/<app ID>`: the session (open tabs, the page left off).
pub fn state_dir() -> Option<PathBuf> {
    base("XDG_STATE_HOME", ".local/state")
}

/// The user's Downloads folder (`XDG_DOWNLOAD_DIR` in `user-dirs.dirs`, else `~/Downloads`).
pub fn downloads_dir() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let config = std::env::var_os("XDG_CONFIG_HOME").map_or_else(|| home.join(".config"), PathBuf::from);
    let from_file = std::fs::read_to_string(config.join("user-dirs.dirs")).ok().and_then(|text| {
        text.lines().find_map(|line| {
            let value = line.trim().strip_prefix("XDG_DOWNLOAD_DIR=")?.trim().trim_matches('"');
            let path = match value.strip_prefix("$HOME") {
                Some(rest) => home.join(rest.trim_start_matches('/')),
                None => PathBuf::from(value),
            };
            path.is_absolute().then_some(path)
        })
    });
    Some(from_file.unwrap_or_else(|| home.join("Downloads")))
}

/// Whether Sofa runs inside a Flatpak.
pub fn is_flatpak() -> bool {
    std::path::Path::new("/.flatpak-info").exists()
}
