//! Sites where the user switched ad blocking off, from Sofa's menu ("Ad block: off here").
//!
//! A site is a host without its `www.` (`news.example` and `www.news.example` are one site; `example.com` and
//! `news.example.com` are two), the same rule as Sofa on Android. The list is kept in a small text file, one host per
//! line, so a settings backup can carry it.
//!
//! A switch made in private browsing, either way, is kept in memory only and counts over the stored one until
//! private browsing ends: nothing of a private session is written down.

use std::collections::{BTreeSet, HashMap};
use std::io;
use std::path::PathBuf;

use url::Url;

use crate::settings::write_replacing;
use crate::sites;

/// The sites where ad blocking is off.
#[derive(Debug, Default, Clone)]
pub struct AdAllowlist {
    path: Option<PathBuf>,
    stored: BTreeSet<String>,
    /// Private browsing's switches: host → ad blocking off.
    private: HashMap<String, bool>,
}

impl AdAllowlist {
    /// The list kept at `path` (`None`: memory only). A missing or unreadable file gives an empty list; lines that
    /// aren't a host name are dropped.
    #[must_use]
    pub fn load(path: Option<PathBuf>) -> Self {
        let text = path.as_deref().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
        let stored = text.lines().map(str::trim).filter(|l| !l.starts_with('#')).filter_map(site_of_host).collect();
        Self { path, stored, private: HashMap::new() }
    }

    /// Whether ad blocking is off on the page at `url` (any address on its site).
    #[must_use]
    pub fn allows(&self, url: &str) -> bool {
        sites::host_of(url).is_some_and(|host| self.allows_host(&host))
    }

    fn allows_host(&self, host: &str) -> bool {
        self.private.get(host).copied().unwrap_or_else(|| self.stored.contains(host))
    }

    /// Switches ad blocking off (`allowed`) or back on for the site of the page at `url`. In `private` browsing the
    /// switch lasts until [`forget_private`](Self::forget_private); otherwise it is written to the file. Returns
    /// false for an address without a host (nothing changes then).
    ///
    /// # Errors
    ///
    /// The file can't be written. The switch still holds until Sofa ends.
    pub fn set(&mut self, url: &str, allowed: bool, private: bool) -> io::Result<bool> {
        let Some(host) = sites::host_of(url) else { return Ok(false) };
        if private {
            self.private.insert(host, allowed);
            return Ok(true);
        }
        let changed = if allowed { self.stored.insert(host) } else { self.stored.remove(&host) };
        if changed {
            self.save()?;
        }
        Ok(true)
    }

    /// Private browsing began or ended: its switches go.
    pub fn forget_private(&mut self) {
        self.private.clear();
    }

    /// The stored sites, A to Z; private browsing's are never among them.
    pub fn stored(&self) -> impl Iterator<Item = &str> {
        self.stored.iter().map(String::as_str)
    }

    fn save(&self) -> io::Result<()> {
        let Some(path) = &self.path else { return Ok(()) };
        let mut text = String::from("# Sites where ad blocking is off, one per line. Written by Sofa.\n");
        for host in &self.stored {
            text.push_str(host);
            text.push('\n');
        }
        write_replacing(path, text.as_bytes())
    }
}

/// A stored line as the site it names, or `None` when it isn't a plain host name (no scheme, path or spaces).
fn site_of_host(line: &str) -> Option<String> {
    if line.is_empty() || line.contains(['/', '@', '?', '#', ' ']) {
        return None;
    }
    let parsed = Url::parse(&format!("http://{line}/")).ok()?;
    // A port or anything the parser had to fix up means the line was more than a host.
    if parsed.port().is_some() || !parsed.host_str()?.eq_ignore_ascii_case(line.trim_end_matches('.')) {
        return None;
    }
    sites::host_of(parsed.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clean_up(file: &std::path::Path) {
        if let Some(dir) = file.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sofa-ad-allowlist-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("ad-blocking-off.txt")
    }

    #[test]
    fn a_site_switched_off_covers_its_pages_and_www_but_not_other_hosts() -> io::Result<()> {
        let mut list = AdAllowlist::default();
        assert!(list.set("https://www.News.example/a?b", true, false)?);
        assert!(list.allows("https://news.example/other"));
        assert!(list.allows("http://www.news.example:8080/"));
        assert!(!list.allows("https://sport.news.example/"));
        assert!(!list.allows("https://example.org/"));
        assert!(list.set("https://news.example/", false, false)?);
        assert!(!list.allows("https://news.example/"));
        Ok(())
    }

    #[test]
    fn addresses_without_a_host_change_nothing() -> io::Result<()> {
        let mut list = AdAllowlist::default();
        assert!(!list.set("about:blank", true, false)?);
        assert!(!list.set("data:text/html,hi", true, true)?);
        assert_eq!(list.stored().count(), 0);
        assert!(!list.allows("about:blank"));
        Ok(())
    }

    #[test]
    fn the_list_is_kept_in_its_file_and_read_back() -> io::Result<()> {
        let path = temp_file("roundtrip");
        let mut list = AdAllowlist::load(Some(path.clone()));
        list.set("https://b.example/", true, false)?;
        list.set("https://a.example/", true, false)?;
        list.set("https://c.example/", true, false)?;
        list.set("https://c.example/", false, false)?;
        let again = AdAllowlist::load(Some(path.clone()));
        assert_eq!(again.stored().collect::<Vec<_>>(), ["a.example", "b.example"]);
        assert!(again.allows("https://www.a.example/x"));
        clean_up(&path);
        Ok(())
    }

    #[test]
    fn private_switches_count_over_stored_ones_and_are_never_written() -> io::Result<()> {
        let path = temp_file("private");
        let mut list = AdAllowlist::load(Some(path.clone()));
        // Off in private browsing: no file at all.
        list.set("https://private.example/", true, true)?;
        assert!(list.allows("https://private.example/"));
        assert!(!path.exists());
        // A site off in normal browsing, switched back on in private browsing: on for the session, file unchanged.
        list.set("https://kept.example/", true, false)?;
        let before = std::fs::read_to_string(&path)?;
        list.set("https://kept.example/", false, true)?;
        assert!(!list.allows("https://kept.example/"));
        assert_eq!(std::fs::read_to_string(&path)?, before);
        // Private browsing ends: back to what is stored.
        list.forget_private();
        assert!(!list.allows("https://private.example/"));
        assert!(list.allows("https://kept.example/"));
        assert_eq!(list.stored().collect::<Vec<_>>(), ["kept.example"]);
        clean_up(&path);
        Ok(())
    }

    #[test]
    fn odd_lines_in_the_file_are_dropped() -> io::Result<()> {
        let path = temp_file("odd");
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let lines = "# comment\n  WWW.Good.example  \nhttps://scheme.example/\nport.example:8080\n\
                     path.example/x\nuser@at.example\ntwo words.example\n\n192.168.1.1\n";
        std::fs::write(&path, lines)?;
        let list = AdAllowlist::load(Some(path.clone()));
        assert_eq!(list.stored().collect::<Vec<_>>(), ["192.168.1.1", "good.example"]);
        clean_up(&path);
        Ok(())
    }
}
