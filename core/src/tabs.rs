//! The tabs of one side (normal or private browsing): their order in the menu's strip, the one in sight, and the
//! order they were in sight in.
//!
//! That order decides where Sofa goes when a tab closes (the tab in sight before it) and which tabs keep their page
//! loaded (the most recent, and none for long). The rules are those of Sofa on Android; times are milliseconds on a
//! clock that stops while the computer sleeps, passed in by the caller so tests control them.

use std::collections::HashMap;
use std::hash::Hash;

use crate::sites;

/// At most this many tabs per side: the cap keeps the strip readable (an unloaded tab costs almost nothing).
pub const MAX_TABS: usize = 10;

/// A tab whose address is longer than this (in bytes) isn't saved between runs: [`MAX_TABS`] such addresses stay under
/// the 64 KiB the session file takes for the list, so one very long address never stops the whole list being saved.
pub const MAX_SAVED_ADDRESS: usize = 6 * 1024;

/// A tab out of sight this long is unloaded: its scripts stop and its memory goes back, its address and history stay.
pub const REST_MS: i64 = 10 * 60 * 1000;

/// One side's tabs. `T` is the caller's id for a tab.
#[derive(Debug, Clone)]
pub struct TabList<T> {
    max: usize,
    order: Vec<T>,
    /// Most recently in sight first; a tab added and never selected is the least recent.
    recent: Vec<T>,
    /// When each tab out of sight went out of sight.
    left_at: HashMap<T, i64>,
}

impl<T: Copy + Eq + Hash> TabList<T> {
    /// An empty list that holds at most `max` tabs.
    #[must_use]
    pub fn new(max: usize) -> Self {
        Self { max, order: Vec::new(), recent: Vec::new(), left_at: HashMap::new() }
    }

    /// The tabs in strip order.
    #[must_use]
    pub fn items(&self) -> &[T] {
        &self.order
    }

    /// How many tabs there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Whether no other tab fits.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.order.len() >= self.max
    }

    /// The tab in sight (or behind the start page); `None` only while the list is empty.
    #[must_use]
    pub fn current(&self) -> Option<T> {
        self.recent.first().copied()
    }

    /// Adds `tab` right after `after`, or at the end when `after` is `None` or not here. False when full or there.
    pub fn add(&mut self, tab: T, after: Option<T>, now: i64) -> bool {
        if self.is_full() || self.order.contains(&tab) {
            return false;
        }
        match after.and_then(|a| self.order.iter().position(|t| *t == a)) {
            Some(i) => self.order.insert(i + 1, tab),
            None => self.order.push(tab),
        }
        if !self.recent.is_empty() {
            self.left_at.insert(tab, now);
        }
        self.recent.push(tab);
        true
    }

    /// `tab` comes into sight.
    pub fn select(&mut self, tab: T, now: i64) {
        if !self.order.contains(&tab) {
            return;
        }
        let was = self.current();
        self.recent.retain(|t| *t != tab);
        self.recent.insert(0, tab);
        if let Some(was) = was.filter(|w| *w != tab) {
            self.left_at.insert(was, now);
        }
        self.left_at.remove(&tab);
    }

    /// Takes `tab` out; returns the tab now in sight: the one in sight before it, or `None` when none is left.
    pub fn remove(&mut self, tab: T) -> Option<T> {
        self.order.retain(|t| *t != tab);
        self.recent.retain(|t| *t != tab);
        self.left_at.remove(&tab);
        if let Some(c) = self.current() {
            self.left_at.remove(&c);
        }
        self.current()
    }

    /// The tabs out of sight, the most recently seen first.
    #[must_use]
    pub fn background(&self) -> &[T] {
        self.recent.get(1..).unwrap_or(&[])
    }

    /// Of the tabs out of sight whose page is loaded (`live`), the ones past the `keep` most recently seen: the pages
    /// to unload.
    #[must_use]
    pub fn beyond(&self, keep: usize, live: impl Fn(T) -> bool) -> Vec<T> {
        self.background().iter().copied().filter(|t| live(*t)).skip(keep).collect()
    }

    /// Of the tabs out of sight whose page is loaded, the ones out of sight for `rest_ms` or longer: to unload.
    #[must_use]
    pub fn rested(&self, rest_ms: i64, now: i64, live: impl Fn(T) -> bool) -> Vec<T> {
        self.background()
            .iter()
            .copied()
            .filter(|t| live(*t) && now - self.left_at.get(t).copied().unwrap_or(now) >= rest_ms)
            .collect()
    }

    /// How long until the next loaded tab out of sight has rested `rest_ms`; `None` when none is loaded.
    #[must_use]
    pub fn next_rest(&self, rest_ms: i64, now: i64, live: impl Fn(T) -> bool) -> Option<i64> {
        self.background()
            .iter()
            .copied()
            .filter(|t| live(*t))
            .map(|t| (self.left_at.get(&t).copied().unwrap_or(now) + rest_ms - now).max(0))
            .min()
    }

    /// The tab `steps` places from the one in sight in strip order (negative: to the left), going round at the ends:
    /// where the keyboard's next and previous tab go. `None` only while the list is empty.
    #[must_use]
    pub fn neighbour(&self, steps: isize) -> Option<T> {
        let len = isize::try_from(self.order.len()).ok().filter(|n| *n > 0)?;
        let here = self.current().and_then(|c| self.order.iter().position(|t| *t == c))?;
        let here = isize::try_from(here).ok()?;
        let at = usize::try_from((here + steps).rem_euclid(len)).ok()?;
        self.order.get(at).copied()
    }
}

/// The names of the strip's chips, in order, for tabs of these `titles` and `addresses`: the title, else the short
/// address, else `new_tab`. Tabs that would get the same name though their pages differ (two pages of one site often
/// share a title) show their short address instead.
#[must_use]
pub fn names(titles: &[Option<String>], addresses: &[Option<String>], new_tab: &str) -> Vec<String> {
    let short = |a: &str| crate::address::display(a).to_owned();
    let plain: Vec<String> = titles
        .iter()
        .zip(addresses)
        .map(|(t, a)| {
            t.clone()
                .filter(|t| !t.trim().is_empty())
                .or_else(|| a.as_deref().map(short).filter(|s| !s.is_empty()))
                .unwrap_or_else(|| new_tab.to_owned())
        })
        .collect();
    plain
        .iter()
        .enumerate()
        .map(|(i, name)| match addresses.get(i).cloned().flatten() {
            Some(address)
                if plain.iter().enumerate().any(|(j, other)| {
                    j != i && other == name && addresses.get(j).cloned().flatten().as_deref() != Some(address.as_str())
                }) =>
            {
                short(&address)
            }
            _ => name.clone(),
        })
        .collect()
}

/// The tab list as Sofa keeps it between runs: the index of the tab in sight on the first line, then one web address
/// per line. Addresses only: never a page's content, its history or a private tab.
///
/// The tab in sight is kept by its place, not its address, since two tabs may show the same page. When it isn't kept
/// itself (an empty tab, a page not on the web, an address longer than [`MAX_SAVED_ADDRESS`]), the kept tab next to it
/// takes its place: the one before, else after. At most [`MAX_TABS`] are kept, so the list always fits the session
/// file's limit.
#[must_use]
pub fn encode(addresses: &[String], current: usize) -> String {
    let mut count = 0;
    let keep: Vec<bool> = addresses
        .iter()
        .map(|a| {
            let ok =
                count < MAX_TABS && a.len() <= MAX_SAVED_ADDRESS && sites::is_web_url(a) && !a.contains(['\n', '\r']);
            count += usize::from(ok);
            ok
        })
        .collect();
    let kept: Vec<&str> = addresses.iter().zip(&keep).filter(|(_, k)| **k).map(|(a, _)| a.as_str()).collect();
    if kept.is_empty() {
        return String::new();
    }
    let c = current.min(addresses.len() - 1);
    let before = keep[..c].iter().filter(|k| **k).count();
    let at = if keep[c] || before == 0 { before } else { before - 1 };
    std::iter::once(at.to_string()).chain(kept.into_iter().map(str::to_owned)).collect::<Vec<_>>().join("\n")
}

/// What [`encode`] wrote: the addresses (web ones, at most `max`) and the index in sight. Anything else gives none.
#[must_use]
pub fn decode(stored: &str, max: usize) -> (Vec<String>, usize) {
    let mut lines = stored.split('\n');
    let Some(at) = lines.next().and_then(|l| l.trim().parse::<usize>().ok()) else {
        return (Vec::new(), 0);
    };
    let addresses: Vec<String> =
        lines.map(str::trim).filter(|a| sites::is_web_url(a)).take(max).map(str::to_owned).collect();
    let at = if at < addresses.len() { at } else { 0 };
    (addresses, at)
}

/// How many tabs out of sight keep their page loaded, from the computer's memory in bytes. Every loaded page costs
/// the engine's page processes their memory; an unloaded one comes back from its address and history when chosen.
#[must_use]
pub fn live_background(total_memory: u64) -> usize {
    const MB: u64 = 1024 * 1024;
    match total_memory {
        m if m < 1_700 * MB => 0,
        m if m < 2_600 * MB => 1,
        m if m < 3_600 * MB => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(n: u32) -> TabList<u32> {
        let mut l = TabList::new(MAX_TABS);
        for i in 0..n {
            assert!(l.add(i, None, 0));
        }
        l
    }

    #[test]
    fn the_first_tab_added_is_in_sight() {
        let l = list(3);
        assert_eq!(l.current(), Some(0));
        assert_eq!(l.items(), [0, 1, 2]);
    }

    #[test]
    fn a_tab_opens_after_the_one_given() {
        let mut l = list(3);
        l.add(9, Some(0), 0);
        assert_eq!(l.items(), [0, 9, 1, 2]);
    }

    #[test]
    fn next_and_previous_tab_go_round_the_strip() {
        let mut l = list(3);
        l.add(9, Some(0), 0);
        l.select(9, 0);
        assert_eq!(l.neighbour(1), Some(1));
        assert_eq!(l.neighbour(-1), Some(0));
        l.select(2, 0);
        assert_eq!(l.neighbour(1), Some(0), "past the last tab comes the first");
        l.select(0, 0);
        assert_eq!(l.neighbour(-1), Some(2), "before the first tab comes the last");
        assert_eq!(l.neighbour(0), Some(0));
    }

    #[test]
    fn a_lone_tab_is_its_own_neighbour_and_an_empty_list_has_none() {
        assert_eq!(list(1).neighbour(1), Some(0));
        assert_eq!(list(0).neighbour(1), None);
    }

    #[test]
    fn closing_goes_back_to_the_tab_seen_before() {
        let mut l = list(3);
        l.select(2, 1);
        l.select(1, 2);
        assert_eq!(l.remove(1), Some(2));
        assert_eq!(l.remove(2), Some(0));
        assert_eq!(l.remove(0), None);
    }

    #[test]
    fn the_list_is_capped() {
        let mut l = list(10);
        assert!(l.is_full());
        assert!(!l.add(99, None, 0));
        assert!(!l.add(3, None, 0));
    }

    #[test]
    fn only_the_most_recent_pages_stay_loaded() {
        let mut l = list(5);
        for (t, i) in [(4, 1), (3, 2), (2, 3), (1, 4), (0, 5)] {
            l.select(t, i);
        }
        // In sight: 0; seen before: 1, 2, 3, 4.
        assert_eq!(l.beyond(2, |_| true), [3, 4]);
        assert_eq!(l.beyond(2, |t| t != 2), [4]);
    }

    #[test]
    fn tabs_rest_after_their_time_out_of_sight() {
        let mut l = list(3);
        l.select(1, 1_000);
        l.select(2, 5_000);
        // 0 left at 1 000, 1 left at 5 000.
        assert_eq!(l.rested(10_000, 11_000, |_| true), [0]);
        assert_eq!(l.next_rest(10_000, 11_000, |t| t == 1), Some(4_000));
        assert_eq!(l.next_rest(10_000, 11_000, |_| false), None);
    }

    #[test]
    fn clashing_names_show_their_address() {
        let titles = [Some("Video".to_owned()), Some("Video".to_owned()), None, Some("Video".to_owned())];
        let addresses = [
            Some("https://www.example.com/a".to_owned()),
            Some("https://www.example.com/b".to_owned()),
            None,
            Some("https://www.example.com/a".to_owned()),
        ];
        assert_eq!(
            names(&titles, &addresses, "New tab"),
            ["example.com/a", "example.com/b", "New tab", "example.com/a"]
        );
        let alone = names(&[Some("Video".into())], &[Some("https://x.example/".into())], "New tab");
        assert_eq!(alone, ["Video"]);
    }

    #[test]
    fn the_saved_list_keeps_web_addresses_and_the_place_in_sight() {
        let tabs = ["https://a.example/".to_owned(), String::new(), "https://b.example/".to_owned()];
        assert_eq!(encode(&tabs, 2), "1\nhttps://a.example/\nhttps://b.example/");
        // An empty tab in sight: its neighbour before takes its place.
        assert_eq!(encode(&tabs, 1), "0\nhttps://a.example/\nhttps://b.example/");
        let first_empty = [String::new(), "https://b.example/".to_owned()];
        assert_eq!(encode(&first_empty, 0), "0\nhttps://b.example/");
        assert_eq!(encode(&[String::new()], 0), "");
        assert_eq!(encode(&["file:///x".to_owned()], 0), "");
    }

    #[test]
    fn a_very_long_address_is_left_out_and_the_rest_still_fits() {
        let long = format!("https://a.example/?q={}", "x".repeat(MAX_SAVED_ADDRESS));
        let tabs = ["https://b.example/".to_owned(), long];
        assert_eq!(encode(&tabs, 1), "0\nhttps://b.example/");
        let longest = format!("https://a.example/?q={}", "x".repeat(MAX_SAVED_ADDRESS - 21));
        assert_eq!(longest.len(), MAX_SAVED_ADDRESS);
        let full: Vec<String> = (0..MAX_TABS + 3).map(|_| longest.clone()).collect();
        let encoded = encode(&full, 0);
        assert_eq!(encoded.lines().count(), MAX_TABS + 1);
        assert!(encoded.len() <= 64 * 1024);
    }

    #[test]
    fn decoding_refuses_anything_odd() {
        assert_eq!(
            decode("1\nhttps://a.example/\nhttps://b.example/", 10),
            (vec!["https://a.example/".into(), "https://b.example/".into()], 1)
        );
        assert_eq!(decode("7\nhttps://a.example/\njavascript:x", 10), (vec!["https://a.example/".into()], 0));
        assert_eq!(decode("nonsense", 10), (vec![], 0));
        assert_eq!(decode("", 10), (vec![], 0));
        let many: String = std::iter::once("0".to_owned())
            .chain((0..20).map(|i| format!("https://e.example/{i}")))
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(decode(&many, MAX_TABS).0.len(), MAX_TABS);
    }

    #[test]
    fn memory_decides_how_many_pages_stay_loaded() {
        const GB: u64 = 1024 * 1024 * 1024;
        assert_eq!(live_background(GB), 0);
        assert_eq!(live_background(2 * GB), 1);
        assert_eq!(live_background(3 * GB), 2);
        assert_eq!(live_background(16 * GB), 3);
    }
}
