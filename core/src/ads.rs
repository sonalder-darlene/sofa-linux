//! Ad and tracker blocking with filter lists, through Brave's adblock-rust engine.
//!
//! Sofa carries two domain lists of its own (StevenBlack's hosts and HaGeZi's pop-up ad domains, the same two as on
//! Android, in `data/filters/`). Each line is one domain, and it blocks that domain and every domain under it. The
//! lists are compiled into one engine on a worker thread when Sofa starts (about a tenth of a second). Until then
//! nothing is blocked, so starting never waits for them.
//!
//! The engine only decides about the parts of a page: images, scripts, frames and the like. Where a page itself may
//! go is the pop-up rules' job, not this one. On a site where the user switched ad blocking off ([`AdAllowlist`]),
//! nothing its pages ask for is blocked.

use std::fmt;
use std::io::{self, Read};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{OnceLock, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use adblock::Engine;
use adblock::lists::{FilterFormat, FilterSet, ParseOptions};
use flate2::read::GzDecoder;

use crate::ad_allowlist::AdAllowlist;
use crate::requests::{Request, ResourceKind};

/// A filter list built into Sofa.
#[derive(Debug, Clone, Copy)]
pub struct BundledList {
    /// The id the list has on Android too.
    pub id: &'static str,
    /// What the list is called where its name is shown.
    pub name: &'static str,
    gzip: &'static [u8],
}

impl BundledList {
    /// The list's text: one domain per line, `#` comments.
    ///
    /// # Errors
    ///
    /// The built-in data doesn't unpack, which would be a broken build.
    pub fn text(&self) -> io::Result<String> {
        let mut text = String::with_capacity(self.gzip.len() * 6);
        GzDecoder::new(self.gzip).read_to_string(&mut text)?;
        Ok(text)
    }
}

/// The lists every copy of Sofa has, refreshed with `data/filters/update.sh` before a release.
pub const BUNDLED: &[BundledList] = &[
    BundledList {
        id: "stevenblack",
        name: "StevenBlack hosts",
        gzip: include_bytes!("../../data/filters/stevenblack-hosts.txt.gz"),
    },
    BundledList {
        id: "hagezi-popups",
        name: "HaGeZi Pop-Up Ads",
        gzip: include_bytes!("../../data/filters/hagezi-popupads.txt.gz"),
    },
];

/// The ad blocker: one engine for every profile (normal and private) and every thread, and the on/off switch.
pub struct AdBlocker {
    enabled: AtomicBool,
    engine: OnceLock<Engine>,
    allowlist: RwLock<AdAllowlist>,
}

impl fmt::Debug for AdBlocker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AdBlocker")
            .field("enabled", &self.is_enabled())
            .field("ready", &self.is_ready())
            .finish_non_exhaustive()
    }
}

impl AdBlocker {
    /// An ad blocker with no lists yet, switched on or off.
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        Self { enabled: AtomicBool::new(enabled), engine: OnceLock::new(), allowlist: RwLock::default() }
    }

    /// Puts `allowlist` in place of the sites switched off so far (Sofa reads its file at start).
    pub fn set_allowlist(&self, allowlist: AdAllowlist) {
        *self.allowlist_mut() = allowlist;
    }

    /// Whether ad blocking is off on the site of the page at `url` (the switch alone, whatever Settings say).
    #[must_use]
    pub fn allows_site(&self, url: &str) -> bool {
        self.allowlist().allows(url)
    }

    /// Switches ad blocking off (`allowed`) or back on for the site of the page at `url`, until private browsing
    /// ends when `private`, else for good. Requests made after this follow it. False for an address without a host.
    ///
    /// # Errors
    ///
    /// The list can't be written to its file; the switch still holds until Sofa ends.
    pub fn allow_site(&self, url: &str, allowed: bool, private: bool) -> io::Result<bool> {
        // Written while the lock is held, so two switches can't reach the file in the wrong order. Only a press of a
        // button gets here, and request checks wait for that write at most.
        self.allowlist_mut().set(url, allowed, private)
    }

    /// Private browsing began or ended: the sites it switched go back to what is stored.
    pub fn forget_private(&self) {
        self.allowlist_mut().forget_private();
    }

    /// The sites stored as switched off, A to Z.
    #[must_use]
    pub fn allowed_sites(&self) -> Vec<String> {
        self.allowlist().stored().map(str::to_owned).collect()
    }

    // A panic elsewhere while the lock was held can't leave the list half changed (each change is one insert or
    // remove), so a poisoned lock is used as it is.
    fn allowlist(&self) -> RwLockReadGuard<'_, AdAllowlist> {
        self.allowlist.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn allowlist_mut(&self) -> RwLockWriteGuard<'_, AdAllowlist> {
        self.allowlist.write().unwrap_or_else(PoisonError::into_inner)
    }

    /// Switches blocking on or off. Requests made after this follow the new state; pages already shown keep what
    /// they loaded.
    pub fn set_enabled(&self, enabled: bool) {
        // Nothing else is published with the flag, so no ordering with other memory is needed.
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Whether blocking is switched on.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Whether the lists are compiled and in use.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.engine.get().is_some()
    }

    /// Compiles `lists` (domain lists, one domain per line) into the engine. Only the first call counts: the
    /// lists in use don't change while Sofa runs.
    pub fn load<I, S>(&self, lists: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut set = FilterSet::new(false);
        let options = ParseOptions { format: FilterFormat::Hosts, ..ParseOptions::default() };
        for text in lists {
            set.add_filter_list(text.into(), options);
        }
        // A second call loses the race and drops its engine; the first one stays.
        let _ = self.engine.set(Engine::new_with_filter_set(set));
    }

    /// Compiles the lists built into Sofa.
    ///
    /// # Errors
    ///
    /// A built-in list doesn't unpack (a broken build). Nothing is loaded then.
    pub fn load_bundled(&self) -> io::Result<()> {
        let texts = BUNDLED.iter().map(BundledList::text).collect::<io::Result<Vec<_>>>()?;
        self.load(texts);
        Ok(())
    }

    /// Whether ad blocking applies on the page at `page`: switched on, and not switched off for its site.
    #[must_use]
    pub fn is_on_for(&self, page: &str) -> bool {
        self.is_enabled() && !self.allows_site(page)
    }

    /// Whether the lists name the host of `url`, for the windows a page opens and the pages a site sends Sofa to
    /// (the pop-up rules, [`crate::popups`]). Whether blocking is on is not asked here.
    ///
    /// Sofa's lists are domain lists, and a domain on them stands for the whole host, whatever is asked of it. The
    /// engine never applies a plain domain rule to a page, so it is asked as for any other part of a page. Lists with
    /// `$popup` or `$document` rules will need a check of their own here.
    #[must_use]
    pub fn lists_host(&self, url: &str) -> bool {
        let Some(engine) = self.engine.get() else { return false };
        adblock::request::Request::new(url, url, ResourceKind::Other.as_filter_type(), "GET")
            .is_ok_and(|r| engine.check_network_request(&r).should_block())
    }

    /// Whether `request` is for an ad or a tracker and must not be sent. Pages themselves (`ResourceKind::Page`)
    /// are never blocked here, nor anything on a site where ad blocking is switched off.
    #[must_use]
    pub fn blocks(&self, request: &Request<'_>) -> bool {
        if request.kind == ResourceKind::Page || !self.is_enabled() {
            return false;
        }
        let Some(engine) = self.engine.get() else { return false };
        // The engine's own parser reads the address; one it can't read (data:, blob:…) has no host to block.
        let listed =
            adblock::request::Request::new(request.url, request.first_party, request.kind.as_filter_type(), "GET")
                .is_ok_and(|r| engine.check_network_request(&r).should_block());
        // The site switch is read only for what the lists would block: a few requests per page, not every one.
        listed && !self.allows_site(request.first_party)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request<'a>(url: &'a str, kind: ResourceKind) -> Request<'a> {
        Request { url, first_party: "https://news.example/", kind }
    }

    fn loaded(lists: &[&str]) -> AdBlocker {
        let ads = AdBlocker::new(true);
        ads.load(lists.iter().copied());
        ads
    }

    #[test]
    fn a_listed_domain_and_its_subdomains_are_blocked() {
        let ads = loaded(&["# comment\nads.example\n"]);
        assert!(ads.blocks(&request("https://ads.example/a.js", ResourceKind::Script)));
        assert!(ads.blocks(&request("https://cdn.ads.example/b.png", ResourceKind::Image)));
        assert!(ads.blocks(&request("https://ads.example/f.html", ResourceKind::Frame)));
    }

    #[test]
    fn other_domains_and_mentions_in_the_address_pass() {
        let ads = loaded(&["ads.example\n"]);
        assert!(!ads.blocks(&request("https://news.example/a.js", ResourceKind::Script)));
        assert!(!ads.blocks(&request("https://notads.example/a.js", ResourceKind::Script)));
        assert!(!ads.blocks(&request("https://other.example/?u=ads.example", ResourceKind::Xhr)));
    }

    #[test]
    fn a_page_is_never_blocked_here() {
        let ads = loaded(&["ads.example\n"]);
        assert!(!ads.blocks(&request("https://ads.example/", ResourceKind::Page)));
    }

    #[test]
    fn nothing_is_blocked_while_switched_off_or_before_the_lists_are_in() {
        let waiting = AdBlocker::new(true);
        assert!(!waiting.blocks(&request("https://ads.example/a.js", ResourceKind::Script)));
        let ads = loaded(&["ads.example\n"]);
        ads.set_enabled(false);
        assert!(!ads.blocks(&request("https://ads.example/a.js", ResourceKind::Script)));
        ads.set_enabled(true);
        assert!(ads.blocks(&request("https://ads.example/a.js", ResourceKind::Script)));
    }

    #[test]
    fn localhost_lines_and_addresses_without_a_host_are_left_alone() {
        let ads = loaded(&["localhost\nads.example\n"]);
        assert!(!ads.blocks(&request("http://localhost:8080/a.js", ResourceKind::Script)));
        assert!(!ads.blocks(&request("data:text/plain,ads.example", ResourceKind::Image)));
        assert!(!ads.blocks(&request("not an address", ResourceKind::Other)));
    }

    #[test]
    fn a_site_switched_off_loads_its_ads_and_others_still_block_them() -> io::Result<()> {
        let ads = loaded(&["ads.example\n"]);
        let on = |page| Request { url: "https://ads.example/a.js", first_party: page, kind: ResourceKind::Script };
        assert!(ads.allow_site("https://www.news.example/watch", true, false)?);
        assert!(!ads.blocks(&on("https://news.example/")));
        assert!(ads.blocks(&on("https://other.example/")));
        // A request made for no page in particular has no site to be switched off.
        assert!(ads.blocks(&on("")));
        assert_eq!(ads.allowed_sites(), ["news.example"]);
        ads.allow_site("https://news.example/", false, false)?;
        assert!(ads.blocks(&on("https://news.example/")));
        Ok(())
    }

    #[test]
    fn private_switches_go_when_private_browsing_ends() -> io::Result<()> {
        let ads = loaded(&["ads.example\n"]);
        let on = |page| Request { url: "https://ads.example/a.js", first_party: page, kind: ResourceKind::Script };
        ads.allow_site("https://news.example/", true, true)?;
        assert!(!ads.blocks(&on("https://news.example/")));
        assert!(ads.allowed_sites().is_empty());
        ads.forget_private();
        assert!(ads.blocks(&on("https://news.example/")));
        Ok(())
    }

    #[test]
    fn listed_hosts_count_for_whole_pages_and_windows() -> io::Result<()> {
        let ads = loaded(&["ads.example\n"]);
        assert!(ads.lists_host("https://ads.example/landing?x=1"));
        assert!(ads.lists_host("https://pop.ads.example/"));
        assert!(!ads.lists_host("https://news.example/"));
        assert!(!ads.lists_host("about:blank"));
        // Listed or not doesn't depend on the switches; whether blocking applies on a page does.
        ads.set_enabled(false);
        assert!(ads.lists_host("https://ads.example/"));
        assert!(!ads.is_on_for("https://news.example/"));
        ads.set_enabled(true);
        assert!(ads.is_on_for("https://news.example/"));
        ads.allow_site("https://news.example/", true, true)?;
        assert!(!ads.is_on_for("https://news.example/a"));
        Ok(())
    }

    #[test]
    fn only_the_first_load_counts() {
        let ads = loaded(&["ads.example\n"]);
        ads.load(["other.example\n"]);
        assert!(ads.blocks(&request("https://ads.example/a.js", ResourceKind::Script)));
        assert!(!ads.blocks(&request("https://other.example/a.js", ResourceKind::Script)));
    }

    #[test]
    fn the_bundled_lists_unpack_and_block_known_ad_hosts() -> io::Result<()> {
        for list in BUNDLED {
            let text = list.text()?;
            assert!(text.lines().filter(|l| !l.starts_with('#')).count() > 10_000, "{} is short", list.id);
        }
        let ads = AdBlocker::new(true);
        ads.load_bundled()?;
        assert!(ads.blocks(&request("https://ad.doubleclick.net/a.png", ResourceKind::Image)));
        assert!(!ads.blocks(&request("https://www.wikipedia.org/static/a.png", ResourceKind::Image)));
        Ok(())
    }
}
