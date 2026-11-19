//! Sofa's settings and its session (the tab list, the page left off), each a small `key=value` text file.
//!
//! Only known keys are kept, and each value is checked before it is stored, so a hand-edited or damaged file can't
//! put anything odd in front of the rest of the app. Files are replaced in one step (written beside, then renamed),
//! so a crash never leaves half a file.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use crate::address;
use crate::start::{self, Choice};

/// The keys Settings keeps, with their defaults. The names match Sofa on Android, so a settings backup reads the same.
pub const SETTINGS: &[(&str, &str)] = &[
    // Empty until the user chose one at first start: Sofa doesn't pick a search engine for them.
    ("search_engine", ""),
    ("open_at_start", "home"),
    ("start_page", ""),
];

/// The keys the session file keeps: browsing state, never part of a settings backup.
pub const SESSION: &[(&str, &str)] = &[("tabs", ""), ("last_page", "")];

/// One `key=value` file with a fixed set of keys.
#[derive(Debug, Clone)]
pub struct KeyFile {
    path: Option<PathBuf>,
    known: &'static [(&'static str, &'static str)],
    values: BTreeMap<&'static str, String>,
}

impl KeyFile {
    /// The file at `path` (`None`: memory only) with the keys of `known`. A missing or unreadable file gives the
    /// defaults; unknown keys and values that don't pass their check are dropped.
    #[must_use]
    pub fn load(path: Option<PathBuf>, known: &'static [(&'static str, &'static str)]) -> Self {
        let mut file = Self { path, known, values: BTreeMap::new() };
        let text = file.path.as_deref().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
        for line in text.lines() {
            let Some((key, raw)) = line.split_once('=') else { continue };
            let Some(key) = file.key(key.trim()) else { continue };
            if let Some(value) = check(key, &unescape(raw)) {
                file.values.insert(key, value);
            }
        }
        file
    }

    /// The value of `key`, or its default; "" for a key this file doesn't keep.
    #[must_use]
    pub fn get(&self, key: &str) -> &str {
        let Some(key) = self.key(key) else { return "" };
        self.values.get(key).map_or_else(|| self.default(key), String::as_str)
    }

    /// Sets `key` to `value` after checking it, and writes the file. Returns whether the value was taken.
    ///
    /// # Errors
    ///
    /// The file can't be written (the value is still kept in memory).
    pub fn set(&mut self, key: &str, value: &str) -> io::Result<bool> {
        let Some(key) = self.key(key) else { return Ok(false) };
        let Some(value) = check(key, value) else { return Ok(false) };
        if self.get(key) == value {
            return Ok(true);
        }
        if value == self.default(key) {
            self.values.remove(key);
        } else {
            self.values.insert(key, value);
        }
        self.save()?;
        Ok(true)
    }

    fn key(&self, key: &str) -> Option<&'static str> {
        self.known.iter().find(|(k, _)| *k == key).map(|(k, _)| *k)
    }

    fn default(&self, key: &str) -> &'static str {
        self.known.iter().find(|(k, _)| *k == key).map_or("", |(_, d)| *d)
    }

    fn save(&self) -> io::Result<()> {
        let Some(path) = &self.path else { return Ok(()) };
        let mut text = String::from(
            "# Written by Sofa. Unknown keys and unexpected values are dropped when it reads this file.\n",
        );
        for (key, value) in &self.values {
            text.push_str(key);
            text.push('=');
            text.push_str(&escape(value));
            text.push('\n');
        }
        write_replacing(path, text.as_bytes())
    }
}

/// Writes `bytes` to `path` through a file beside it and a rename, making the folder first.
///
/// # Errors
///
/// The folder or the file can't be written.
pub fn write_replacing(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".new");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// `value` as `key` keeps it, or `None` when it doesn't fit.
fn check(key: &str, value: &str) -> Option<String> {
    let v = value.trim();
    match key {
        "search_engine" => (v.is_empty() || address::engine(v).is_some()).then(|| v.to_owned()),
        "open_at_start" => Some(Choice::from_id(v).id().to_owned()),
        "start_page" | "last_page" => {
            if v.is_empty() {
                Some(String::new())
            } else {
                start::web_address(v)
            }
        }
        // The tab list is checked again when it is read (crate::tabs::decode); here it only has to be text.
        "tabs" => (value.len() <= 64 * 1024).then(|| value.to_owned()),
        _ => None,
    }
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\n', "\\n").replace('\r', "\\r")
}

fn unescape(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("sofa-settings-test-{}-{name}", std::process::id())).join("settings.conf")
    }

    #[test]
    fn defaults_come_back_for_unset_keys() {
        let s = KeyFile::load(None, SETTINGS);
        assert_eq!(s.get("open_at_start"), "home");
        assert_eq!(s.get("search_engine"), "");
        assert_eq!(s.get("nonsense"), "");
    }

    #[test]
    fn values_are_checked_before_they_are_kept() -> io::Result<()> {
        let mut s = KeyFile::load(None, SETTINGS);
        assert!(s.set("search_engine", "duckduckgo")?);
        assert!(!s.set("search_engine", "evil")?);
        assert_eq!(s.get("search_engine"), "duckduckgo");
        assert!(s.set("start_page", "example.com/tv")?);
        assert_eq!(s.get("start_page"), "https://example.com/tv");
        assert!(!s.set("start_page", "javascript:alert(1)")?);
        assert!(!s.set("tabs", "x")?);
        Ok(())
    }

    #[test]
    fn a_file_round_trips_and_drops_what_it_doesnt_know() -> io::Result<()> {
        let path = temp("round");
        let mut s = KeyFile::load(Some(path.clone()), SESSION);
        s.set("tabs", "0\nhttps://a.example/\nhttps://b.example/\\x")?;
        s.set("last_page", "https://a.example/")?;
        let mut text = std::fs::read_to_string(&path)?;
        text.push_str("unknown=1\nlast_page_without_equals\n");
        std::fs::write(&path, text)?;
        let back = KeyFile::load(Some(path.clone()), SESSION);
        assert_eq!(back.get("tabs"), "0\nhttps://a.example/\nhttps://b.example/\\x");
        assert_eq!(back.get("last_page"), "https://a.example/");
        let _ = std::fs::remove_dir_all(path.parent().unwrap_or(&path));
        Ok(())
    }

    #[test]
    fn setting_the_default_removes_the_line() -> io::Result<()> {
        let path = temp("default");
        let mut s = KeyFile::load(Some(path.clone()), SETTINGS);
        s.set("open_at_start", "last")?;
        assert!(std::fs::read_to_string(&path)?.contains("open_at_start=last"));
        s.set("open_at_start", "home")?;
        assert!(!std::fs::read_to_string(&path)?.contains("open_at_start"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap_or(&path));
        Ok(())
    }
}
