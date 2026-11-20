//! The pop-up and redirect rules: which windows a page may open, and where a page may send itself.
//!
//! Pop-under ad networks open their window on the first press anywhere on a page: through a link laid invisibly over
//! the page, or riding on a press on a real link, often a picture link. Sofa opens an allowed window in the tab that
//! asked for it (only the user opens tabs), so such a window would replace the page with an ad.
//!
//! The web engine says whether a window or a navigation comes from a press, but not which press, nor what was
//! pressed. [`PopupGuard`] counts the presses that reach the page and keeps the link each one landed on, which a small
//! script reports from every frame, in a world of its own that the page's scripts can't reach ([`PressTag`]).
//!
//! The rules, the same as Sofa's on Android:
//!
//! 1. A window without a press is never opened, and one press opens at most one window.
//! 2. A window to a host on the filter lists is blocked.
//! 3. A window to another site opens only from a press on a link to it. A press whose link isn't known explains
//!    nothing: pop-under scripts ride on presses on picture links, and their window goes elsewhere.
//! 4. One press, one destination: a press that already sent the page somewhere gets no window.
//! 5. A press that asked for a window can't also send the page to another site without a link to it, for 5 s (the
//!    "tab-under": the page opens itself in the window and sends the tab to an ad).
//! 6. A press not on a link can't send the page to a listed host, nor can a page on its own. A redirect belongs to the
//!    navigation that started it: one the user started passes every hop (click-tracking goes through listed hosts).
//!
//! Rules 2–6 hold only where ad blocking is on for the page. A blocked address can be opened once from the menu
//! ("Allow once") for [`ALLOW_ONCE_MS`] after it was blocked.

use std::hash::{BuildHasher, RandomState};

use crate::ads::AdBlocker;
use crate::sites::{host_of, registrable_domain, same_registrable_domain};

/// How long "Allow once" stays offered after a block, in milliseconds.
pub const ALLOW_ONCE_MS: u64 = 30_000;

/// How long a press counts as the reason for a navigation, in milliseconds, when the engine doesn't say: the time
/// a press lets a page open things in Chromium.
pub const GESTURE_MS: u64 = 5_000;

/// What happens to a window or a navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum Verdict {
    /// It goes ahead.
    Allow,
    /// It is stopped.
    Block,
}

/// What the rules need from the filter lists.
pub trait Lists {
    /// Whether ad blocking applies on the page at `page`.
    fn is_on_for(&self, page: &str) -> bool;
    /// Whether the lists name `url`'s host as an ad or a tracker.
    fn lists(&self, url: &str) -> bool;
}

impl Lists for AdBlocker {
    fn is_on_for(&self, page: &str) -> bool {
        AdBlocker::is_on_for(self, page)
    }

    fn lists(&self, url: &str) -> bool {
        self.lists_host(url)
    }
}

/// A window a page asked for, with a press behind it.
#[derive(Debug, Clone, Copy)]
pub struct Window<'a> {
    /// The window's first address.
    pub url: &'a str,
    /// The page that opened it.
    pub opener: &'a str,
    /// The link the press was on when the window was asked for ([`PopupGuard::link`]): the page may move things
    /// between the request and the window's first address.
    pub link: Option<&'a str>,
}

/// A main-frame navigation the page started, or one of its redirect hops. Sofa's own loads (an address typed or
/// chosen, Back, Reload) are [`PopupGuard::user_navigation`] instead.
#[derive(Debug, Clone, Copy)]
pub struct Navigation<'a> {
    /// Where it goes.
    pub url: &'a str,
    /// The page it leaves (for a redirect hop, the page in the tab when it came).
    pub page: &'a str,
    /// Whether the engine says a press started it; `None` when the engine doesn't say (a press within
    /// [`GESTURE_MS`] counts then).
    pub gesture: Option<bool>,
    /// A server redirect hop of the navigation under way.
    pub redirect: bool,
}

/// The pop-up and redirect rules for one tab. Times are milliseconds on any clock that only goes forward.
#[derive(Debug, Default)]
pub struct PopupGuard {
    /// Presses so far; the engine gives no identity to the press behind a window, so presses are told apart by count.
    press: u64,
    last_press_at: Option<u64>,
    /// The link under the latest press, as the page's press script reported it.
    link: Option<String>,
    press_with_window: Option<u64>,
    press_asking_window: Option<u64>,
    press_with_navigation: Option<u64>,
    /// Whether the user started the navigation under way (its redirect hops then pass).
    user_chain: bool,
    /// The page the navigation under way left: its site's switch decides for every hop.
    chain_page: String,
    pending: Option<(String, u64)>,
}

impl PopupGuard {
    /// A guard for a new tab.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A press reached the page at `now`: a click, a tap, or a key that activates what is in focus. What it landed on
    /// comes after, through [`Self::pressed_link`].
    pub fn press(&mut self, now: u64) {
        self.press = self.press.saturating_add(1);
        self.last_press_at = Some(now);
        self.link = None;
    }

    /// The page's press script says the latest press landed on a link to `href` ("": not on a link).
    pub fn pressed_link(&mut self, href: &str) {
        self.link = (!href.is_empty()).then(|| href.to_owned());
    }

    /// The link under the latest press, if it was on one.
    #[must_use]
    pub fn link(&self) -> Option<&str> {
        self.link.as_deref()
    }

    /// The page asked for a window (before its first address is known), on the latest press if `gesture`. False
    /// when the window must not open at all: a page opens no window on its own.
    #[must_use]
    pub fn window_requested(&mut self, gesture: bool) -> bool {
        if gesture {
            self.press_asking_window = Some(self.press);
        }
        gesture
    }

    /// Whether the window `window` (asked for with a press, see [`Self::window_requested`]) may open.
    pub fn window(&mut self, window: &Window<'_>, lists: &impl Lists) -> Verdict {
        if !lists.is_on_for(window.opener) {
            return Verdict::Allow;
        }
        // A blocked window uses up the press as well.
        let extra = self.press_with_window.replace(self.press) == Some(self.press);
        let also_navigated = self.press_with_navigation == Some(self.press);
        let listed = lists.lists(window.url);
        let explained = leads_to(window.link, window.url) || same_registrable_domain(window.url, window.opener);
        if listed || extra || also_navigated || !explained { Verdict::Block } else { Verdict::Allow }
    }

    /// Sofa itself is loading an address (typed or chosen, Back, Forward, Reload, Allow once, an allowed window): it
    /// and its redirects are the user's.
    pub fn user_navigation(&mut self) {
        self.user_chain = true;
    }

    /// Whether the navigation `nav` may go on, at `now`.
    pub fn navigation(&mut self, nav: &Navigation<'_>, now: u64, lists: &impl Lists) -> Verdict {
        let listed = lists.lists(nav.url);
        if !nav.redirect {
            nav.page.clone_into(&mut self.chain_page);
            self.press_with_navigation = Some(self.press);
            let gesture = nav.gesture.unwrap_or_else(|| self.pressed_within(now, GESTURE_MS));
            let from_link = leads_to(self.link.as_deref(), nav.url);
            let cross_site = registrable_domain(nav.page)
                .is_some_and(|page| registrable_domain(nav.url).is_some_and(|target| target != page));
            // Not the engine's gesture: the window has used it up by then, so the navigation comes without one.
            let tab_under = !from_link
                && cross_site
                && self.press_asking_window == Some(self.press)
                && self.pressed_within(now, GESTURE_MS);
            self.user_chain = gesture && (from_link || !listed) && !tab_under;
            if tab_under && lists.is_on_for(nav.page) {
                return Verdict::Block;
            }
        }
        if listed && !self.user_chain && lists.is_on_for(&self.chain_page) { Verdict::Block } else { Verdict::Allow }
    }

    fn pressed_within(&self, now: u64, window_ms: u64) -> bool {
        self.last_press_at.is_some_and(|at| now.saturating_sub(at) <= window_ms)
    }

    /// Remembers `url` as just blocked at `now`, for "Allow once".
    pub fn blocked(&mut self, url: &str, now: u64) {
        self.pending = Some((url.to_owned(), now));
    }

    /// The address "Allow once" would open: the latest one blocked, within [`ALLOW_ONCE_MS`] and until the next page.
    #[must_use]
    pub fn pending(&self, now: u64) -> Option<&str> {
        self.pending.as_ref().filter(|(_, at)| now.saturating_sub(*at) <= ALLOW_ONCE_MS).map(|(url, _)| url.as_str())
    }

    /// Like [`Self::pending`], and forgets it: an address is allowed once.
    pub fn take_pending(&mut self, now: u64) -> Option<String> {
        let url = self.pending(now).map(str::to_owned);
        self.pending = None;
        url
    }

    /// A new page started loading in the tab.
    pub fn page_started(&mut self) {
        self.pending = None;
    }
}

/// Whether a press on the link `link` explains going to `url`: the same address, or the same host.
fn leads_to(link: Option<&str>, url: &str) -> bool {
    let Some(link) = link.filter(|l| !l.is_empty()) else { return false };
    let before_hash = |s: &str| s.split_once('#').map_or(s, |(a, _)| a).to_owned();
    before_hash(link) == before_hash(url) || host_of(link).is_some_and(|h| host_of(url).as_deref() == Some(h.as_str()))
}

/// The secret that marks the press script's reports. The script runs in a world of its own in each frame and reports
/// through the page's console; the page's scripts can write to the console too, but can't read the tag, so they
/// can't speak for a press.
#[derive(Clone, PartialEq, Eq)]
pub struct PressTag(String);

impl std::fmt::Debug for PressTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PressTag(…)")
    }
}

impl PressTag {
    /// A new random tag, for one run of Sofa.
    #[must_use]
    pub fn random() -> Self {
        // Each RandomState is keyed from the system's random source; two give 128 bits.
        let half = || RandomState::new().hash_one(0_u8);
        Self(format!("sofa-press:{:016x}{:016x} ", half(), half()))
    }

    /// The press script, to run at the start of every document in every frame, in a world the page can't reach. On
    /// each real press (a pointer press, or Enter or Space on what is in focus) it reports the link under it.
    #[must_use]
    pub fn script(&self) -> String {
        format!(
            r#"(() => {{
  "use strict";
  const tag = "{tag}";
  const linkOn = path => {{
    for (const node of path) {{
      if ((node instanceof HTMLAnchorElement || node instanceof HTMLAreaElement) && node.href) return node.href;
    }}
    return "";
  }};
  const say = e => {{ if (e.isTrusted) console.log(tag + linkOn(e.composedPath())); }};
  addEventListener("pointerdown", say, true);
  addEventListener("keydown", e => {{ if (!e.repeat && (e.key === "Enter" || e.key === " ")) say(e); }}, true);
}})();"#,
            tag = self.0
        )
    }

    /// The link a console message from the press script reports ("" for a press not on a link), or `None` for any
    /// other message.
    #[must_use]
    pub fn read<'a>(&self, message: &'a str) -> Option<&'a str> {
        message.strip_prefix(self.0.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lists for the tests: `ads.test` is listed; ad blocking is off on `off.test`.
    struct FakeLists;

    impl Lists for FakeLists {
        fn is_on_for(&self, page: &str) -> bool {
            host_of(page).as_deref() != Some("off.test")
        }
        fn lists(&self, url: &str) -> bool {
            host_of(url).is_some_and(|h| h == "ads.test" || h.ends_with(".ads.test"))
        }
    }

    const PAGE: &str = "https://video.test/home";

    fn window<'a>(url: &'a str, link: Option<&'a str>) -> Window<'a> {
        Window { url, opener: PAGE, link }
    }

    fn page_nav(url: &str, gesture: bool) -> Navigation<'_> {
        Navigation { url, page: PAGE, gesture: Some(gesture), redirect: false }
    }

    fn hop(url: &str) -> Navigation<'_> {
        Navigation { url, page: PAGE, gesture: Some(false), redirect: true }
    }

    /// The window `url`, judged with the link under the latest press.
    fn judge(guard: &mut PopupGuard, url: &str) -> Verdict {
        let link = guard.link().map(str::to_owned);
        guard.window(&window(url, link.as_deref()), &FakeLists)
    }

    fn pressed(guard: &mut PopupGuard, link: &str) {
        guard.press(0);
        guard.pressed_link(link);
        assert!(guard.window_requested(true));
    }

    #[test]
    fn a_window_without_a_press_never_opens() {
        let mut guard = PopupGuard::new();
        assert!(!guard.window_requested(false));
    }

    #[test]
    fn a_window_to_a_listed_host_is_blocked() {
        let mut guard = PopupGuard::new();
        pressed(&mut guard, "https://ads.test/x");
        assert_eq!(judge(&mut guard, "https://ads.test/x"), Verdict::Block);
    }

    #[test]
    fn one_press_opens_at_most_one_window_and_a_blocked_one_counts() {
        let mut guard = PopupGuard::new();
        pressed(&mut guard, "");
        assert_eq!(guard.window(&window("https://video.test/a", None), &FakeLists), Verdict::Allow);
        assert_eq!(guard.window(&window("https://video.test/b", None), &FakeLists), Verdict::Block);
        pressed(&mut guard, "");
        assert_eq!(guard.window(&window("https://ads.test/", None), &FakeLists), Verdict::Block);
        assert_eq!(guard.window(&window("https://video.test/c", None), &FakeLists), Verdict::Block);
        pressed(&mut guard, "");
        assert_eq!(guard.window(&window("https://video.test/d", None), &FakeLists), Verdict::Allow);
    }

    #[test]
    fn a_press_anywhere_cant_open_another_sites_window() {
        // The pop-under: a press on a button or on the page opens an ad network no list knows yet.
        let mut guard = PopupGuard::new();
        pressed(&mut guard, "");
        assert_eq!(judge(&mut guard, "https://unknown-ads.example/z"), Verdict::Block);
    }

    #[test]
    fn a_pop_under_riding_on_a_picture_link_is_blocked_and_the_link_still_opens() {
        // The card's link goes to the site itself; the script opens another site on the same press.
        let mut guard = PopupGuard::new();
        pressed(&mut guard, "https://video.test/series/29");
        assert_eq!(judge(&mut guard, "https://unknown-ads.example/z"), Verdict::Block);
        assert_eq!(guard.navigation(&page_nav("https://video.test/series/29", false), 40, &FakeLists), Verdict::Allow);
    }

    #[test]
    fn links_to_other_sites_and_the_sites_own_windows_open() {
        let mut guard = PopupGuard::new();
        pressed(&mut guard, "https://partner.example/show#top");
        assert_eq!(judge(&mut guard, "https://partner.example/show"), Verdict::Allow);
        pressed(&mut guard, "");
        assert_eq!(guard.window(&window("https://cdn.video.test/player", None), &FakeLists), Verdict::Allow);
        // A link to a listed host is still blocked.
        pressed(&mut guard, "https://ads.test/");
        assert_eq!(judge(&mut guard, "https://ads.test/"), Verdict::Block);
    }

    #[test]
    fn a_press_that_followed_a_link_gets_no_window_as_well() {
        let mut guard = PopupGuard::new();
        pressed(&mut guard, "https://partner.example/a");
        assert_eq!(guard.navigation(&page_nav("https://video.test/next", false), 0, &FakeLists), Verdict::Allow);
        assert_eq!(judge(&mut guard, "https://partner.example/a"), Verdict::Block);
        pressed(&mut guard, "https://partner.example/a");
        assert_eq!(judge(&mut guard, "https://partner.example/a"), Verdict::Allow);
    }

    #[test]
    fn a_press_that_opens_a_window_cant_also_send_the_page_to_another_site() {
        // The tab-under, and the hops after it are not the user's either.
        let mut guard = PopupGuard::new();
        pressed(&mut guard, "");
        assert_eq!(guard.navigation(&page_nav("https://landing.example/", true), 0, &FakeLists), Verdict::Block);
        assert_eq!(guard.navigation(&hop("https://ads.test/r"), 0, &FakeLists), Verdict::Block);
    }

    #[test]
    fn a_tab_under_is_blocked_though_the_window_used_up_the_gesture() {
        let mut guard = PopupGuard::new();
        pressed(&mut guard, "");
        assert_eq!(guard.navigation(&page_nav("https://landing.example/", false), 100, &FakeLists), Verdict::Block);
        // Long after the press, a page leaving on its own isn't taken for one.
        assert_eq!(guard.navigation(&page_nav("https://landing.example/", false), 60_000, &FakeLists), Verdict::Allow);
    }

    #[test]
    fn buttons_that_go_to_another_site_still_work() {
        // No window on this press: "Sign in with …" or a form that leaves the site.
        let mut guard = PopupGuard::new();
        guard.press(0);
        assert_eq!(guard.navigation(&page_nav("https://login.example/", true), 0, &FakeLists), Verdict::Allow);
        // A window on an earlier press doesn't count against this one.
        assert!(guard.window_requested(true));
        guard.press(10);
        assert_eq!(guard.navigation(&page_nav("https://login.example/", true), 10, &FakeLists), Verdict::Allow);
    }

    #[test]
    fn only_a_link_press_sends_the_page_to_a_listed_host() {
        let mut guard = PopupGuard::new();
        guard.press(0);
        assert_eq!(guard.navigation(&page_nav("https://ads.test/", true), 0, &FakeLists), Verdict::Block);
        guard.press(0);
        guard.pressed_link("https://ads.test/");
        assert_eq!(guard.navigation(&page_nav("https://ads.test/", true), 0, &FakeLists), Verdict::Allow);
    }

    #[test]
    fn redirect_hops_keep_what_their_navigation_decided() {
        let mut guard = PopupGuard::new();
        // A pressed link passes through listed click-tracking hops.
        guard.press(0);
        guard.pressed_link("https://video.test/out");
        assert_eq!(guard.navigation(&page_nav("https://video.test/out", true), 0, &FakeLists), Verdict::Allow);
        assert_eq!(guard.navigation(&hop("https://ads.test/track"), 0, &FakeLists), Verdict::Allow);
        // A chain the page started on its own stops at its first listed hop.
        assert_eq!(guard.navigation(&page_nav("https://video.test/next", false), 60_000, &FakeLists), Verdict::Allow);
        assert_eq!(guard.navigation(&hop("https://ads.test/track"), 60_000, &FakeLists), Verdict::Block);
    }

    #[test]
    fn sofas_own_loads_start_a_user_chain_until_the_next_page_navigation() {
        let mut guard = PopupGuard::new();
        guard.user_navigation();
        assert_eq!(guard.navigation(&hop("https://ads.test/track"), 0, &FakeLists), Verdict::Allow);
        assert_eq!(guard.navigation(&page_nav("https://ads.test/", false), 0, &FakeLists), Verdict::Block);
    }

    #[test]
    fn without_a_word_from_the_engine_a_recent_press_is_the_gesture() {
        let mut guard = PopupGuard::new();
        let nav = |url| Navigation { url, page: PAGE, gesture: None, redirect: false };
        guard.press(1_000);
        guard.pressed_link("https://ads.test/");
        assert_eq!(guard.navigation(&nav("https://ads.test/"), 1_000 + GESTURE_MS, &FakeLists), Verdict::Allow);
        assert_eq!(guard.navigation(&nav("https://ads.test/"), 2_001 + GESTURE_MS, &FakeLists), Verdict::Block);
    }

    #[test]
    fn a_site_switched_off_gets_every_window_redirect_and_tab_under() {
        let mut guard = PopupGuard::new();
        let off = "https://off.test/";
        pressed(&mut guard, "");
        let w = Window { url: "https://ads.test/", opener: off, link: None };
        assert_eq!(guard.window(&w, &FakeLists), Verdict::Allow);
        assert_eq!(guard.window(&w, &FakeLists), Verdict::Allow);
        let nav = Navigation { url: "https://ads.test/", page: off, gesture: Some(true), redirect: false };
        assert_eq!(guard.navigation(&nav, 0, &FakeLists), Verdict::Allow);
        assert_eq!(guard.navigation(&hop("https://ads.test/2"), 0, &FakeLists), Verdict::Allow);
    }

    #[test]
    fn a_blocked_address_can_be_allowed_once_within_thirty_seconds() {
        let mut guard = PopupGuard::new();
        guard.blocked("https://ads.test/1", 0);
        guard.blocked("https://ads.test/2", 1_000);
        assert_eq!(guard.pending(1_000 + ALLOW_ONCE_MS), Some("https://ads.test/2"));
        assert_eq!(guard.pending(1_001 + ALLOW_ONCE_MS), None);
        assert_eq!(guard.take_pending(2_000).as_deref(), Some("https://ads.test/2"));
        assert_eq!(guard.take_pending(2_000), None);
        guard.blocked("https://ads.test/3", 3_000);
        guard.page_started();
        assert_eq!(guard.pending(3_000), None);
    }

    #[test]
    fn only_reports_with_the_tag_are_presses() {
        let tag = PressTag::random();
        assert_ne!(tag, PressTag::random());
        let script = tag.script();
        let prefix = script.split('"').nth(3).unwrap_or_default();
        assert_eq!(tag.read(&format!("{prefix}https://video.test/a")), Some("https://video.test/a"));
        assert_eq!(tag.read(prefix), Some(""));
        assert_eq!(tag.read("sofa-press:0000000000000000 https://ads.test/"), None);
        assert_eq!(tag.read("hello"), None);
        assert!(!format!("{tag:?}").contains("sofa-press"));
    }
}
