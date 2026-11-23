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

/// `~/.cache/<app ID>`: QtWebEngine's cache (Qt puts it there itself).
pub fn cache_dir() -> Option<PathBuf> {
    base("XDG_CACHE_HOME", ".cache")
}

/// Makes Sofa's own folders (data, config, state, cache) readable by the user alone: made that way when missing,
/// and set so at each start when they exist. Before the web engine starts, so its profile lands inside them.
pub fn make_own_folders_private() {
    use std::os::unix::fs::PermissionsExt;
    for dir in [data_dir(), config_dir(), state_dir(), cache_dir()].into_iter().flatten() {
        let done = if dir.is_dir() {
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
        } else {
            sofa_core::settings::create_private_dir(&dir)
        };
        if let Err(error) = done {
            eprintln!("sofa-tv-browser: {} could not be made private: {error}", dir.display());
        }
    }
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
