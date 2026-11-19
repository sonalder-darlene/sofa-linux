//! What the user typed in the address field, turned into something to load.
//!
//! Same rules as Sofa on Android: an address with a scheme loads as it is, a bare domain loads over HTTPS, an IPv4
//! address or a `host:port` loads over plain HTTP (home devices rarely have certificates), and anything else is
//! a search.

use std::fmt::Write as _;

/// A search engine: a name and an address template where `%s` stands for the query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchEngine {
    /// Stable id, as stored in settings (the same ids as on Android).
    pub id: &'static str,
    /// Name shown to the user.
    pub name: &'static str,
    /// Address template; `%s` is replaced with the percent-encoded query.
    pub template: &'static str,
    /// Works only with a paid account or key.
    pub paid: bool,
}

const fn engine_entry(id: &'static str, name: &'static str, template: &'static str, paid: bool) -> SearchEngine {
    SearchEngine { id, name, template, paid }
}

/// The first-start choices, A to Z, as on Android: templates from each engine's OpenSearch description, minus
/// traffic-source markers. Showing the list contacts no one.
pub const ENGINES: &[SearchEngine] = &[
    engine_entry("brave", "Brave Search", "https://search.brave.com/search?q=%s", false),
    SearchEngine::DUCKDUCKGO,
    engine_entry("ecosia", "Ecosia", "https://www.ecosia.org/search?q=%s", false),
    engine_entry("google", "Google", "https://www.google.com/search?q=%s", false),
    engine_entry("kagi", "Kagi", "https://kagi.com/search?q=%s", true),
    engine_entry("metager", "MetaGer", "https://metager.org/meta/meta.ger3?eingabe=%s", true),
    engine_entry("mojeek", "Mojeek", "https://www.mojeek.com/search?q=%s", false),
    engine_entry("murena", "Murena Find", "https://murena.qwant.com/?q=%s", false),
    engine_entry("qwant", "Qwant", "https://www.qwant.com/?q=%s", false),
    engine_entry("startpage", "Startpage", "https://www.startpage.com/sp/search?query=%s", false),
    engine_entry("swisscows", "Swisscows", "https://swisscows.com/web?query=%s", false),
    engine_entry("uruky", "Uruky", "https://uruky.com/search?q=%s", true),
];

/// The engine stored as `id`.
#[must_use]
pub fn engine(id: &str) -> Option<&'static SearchEngine> {
    ENGINES.iter().find(|e| e.id == id)
}

impl SearchEngine {
    /// DuckDuckGo, the default on Android too (used until the user picks one).
    pub const DUCKDUCKGO: Self = engine_entry("duckduckgo", "DuckDuckGo", "https://duckduckgo.com/?q=%s", false);

    /// The address that searches for `query`.
    #[must_use]
    pub fn search_url(&self, query: &str) -> String {
        self.template.replace("%s", &percent_encode(query))
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::DUCKDUCKGO
    }
}

/// Turns what the user typed into an address to load.
///
/// Returns `None` when there is nothing but white space.
///
/// # Examples
///
/// ```
/// use sofa_core::address::{resolve, SearchEngine};
///
/// let engine = SearchEngine::default();
/// assert_eq!(resolve("example.com", &engine).as_deref(), Some("https://example.com"));
/// assert_eq!(resolve("192.168.1.1", &engine).as_deref(), Some("http://192.168.1.1"));
/// assert_eq!(resolve("cat videos", &engine).as_deref(), Some("https://duckduckgo.com/?q=cat%20videos"));
/// ```
#[must_use]
pub fn resolve(text: &str, engine: &SearchEngine) -> Option<String> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    if !t.contains(char::is_whitespace) {
        if let Some(address) = as_address(t) {
            return Some(address);
        }
    }
    Some(engine.search_url(t))
}

/// `t` as an address, or `None` when it should be searched for. `t` is trimmed and has no white space.
fn as_address(t: &str) -> Option<String> {
    if starts_with_ignore_case(t, "http://") || starts_with_ignore_case(t, "https://") {
        return Some(t.to_owned());
    }
    if starts_with_ignore_case(t, "about:") || starts_with_ignore_case(t, "view-source:") {
        return Some(t.to_owned());
    }
    if is_host_port(t) || starts_with_ignore_case(t, "localhost") {
        return Some(format!("http://{t}"));
    }
    let before_colon = t.split(':').next().unwrap_or(t);
    if !has_scheme(t) || before_colon.contains('.') {
        let host = t.split('/').next().unwrap_or(t);
        let host = host.split('?').next().unwrap_or(host);
        if is_ipv4(host) {
            return Some(format!("http://{t}"));
        }
        if is_domain_name(host) && host.contains('.') && !host.ends_with('.') {
            return Some(format!("https://{t}"));
        }
    }
    None
}

/// A short form of `url` for showing: no scheme, no `www.`, no trailing slash. Other schemes stay as they are.
///
/// # Examples
///
/// ```
/// assert_eq!(sofa_core::address::display("https://www.example.com/"), "example.com");
/// assert_eq!(sofa_core::address::display("about:blank"), "about:blank");
/// ```
#[must_use]
pub fn display(url: &str) -> &str {
    let rest = strip_prefix_ignore_case(url, "https://").or_else(|| strip_prefix_ignore_case(url, "http://"));
    let Some(rest) = rest else {
        return url;
    };
    let rest = rest.strip_prefix("www.").unwrap_or(rest);
    rest.strip_suffix('/').unwrap_or(rest)
}

fn starts_with_ignore_case(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

fn strip_prefix_ignore_case<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    // The prefixes are ASCII, so a match ends on a char boundary.
    starts_with_ignore_case(s, prefix).then(|| &s[prefix.len()..])
}

/// `^[a-zA-Z][a-zA-Z0-9+.-]*:` anywhere at the start.
fn has_scheme(t: &str) -> bool {
    let mut chars = t.chars();
    if !chars.next().is_some_and(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    for c in chars {
        match c {
            ':' => return true,
            c if c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-') => {}
            _ => return false,
        }
    }
    false
}

/// `^[^\s/]+:\d+(/.*)?$`: something, a colon, a port, then nothing or a path.
fn is_host_port(t: &str) -> bool {
    let authority = t.split('/').next().unwrap_or(t);
    let Some((host, port)) = authority.rsplit_once(':') else {
        return false;
    };
    !host.is_empty() && !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit())
}

/// Four dot-separated numbers from 0 to 255, without leading zeros beyond what Android's pattern allows.
fn is_ipv4(host: &str) -> bool {
    let mut parts = 0;
    for part in host.split('.') {
        parts += 1;
        if parts > 4 || part.is_empty() || part.len() > 3 || !part.bytes().all(|b| b.is_ascii_digit()) {
            return false;
        }
        if part.parse::<u16>().map_or(true, |n| n > 255) {
            return false;
        }
    }
    parts == 4
}

/// A domain name as Android's `Patterns.DOMAIN_NAME` sees it: labels of letters, digits, hyphens and non-ASCII
/// letters (not starting or ending with a hyphen, at most 63 long), and a top-level label of at least two
/// letters, or a punycode one (`xn--…`).
fn is_domain_name(host: &str) -> bool {
    let labels: Vec<&str> = host.split('.').collect();
    let Some((tld, rest)) = labels.split_last() else {
        return false;
    };
    if rest.is_empty() {
        return false;
    }
    let label_ok = |label: &str| {
        let n = label.chars().count();
        (1..=63).contains(&n)
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || !c.is_ascii())
    };
    let tld_ok = if starts_with_ignore_case(tld, "xn--") {
        label_ok(tld)
    } else {
        let n = tld.chars().count();
        (2..=63).contains(&n) && tld.chars().all(|c| c.is_ascii_alphabetic() || (!c.is_ascii() && c.is_alphabetic()))
    };
    tld_ok && rest.iter().all(|l| label_ok(l))
}

/// Percent-encodes everything except the characters Android's `Uri.encode` leaves alone
/// (`A–Z a–z 0–9 _ - ! . ~ ' ( ) *`).
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'!' | b'.' | b'~' | b'\'' | b'(' | b')' | b'*') {
            out.push(char::from(b));
        } else {
            // Writing to a String can't fail.
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn go(text: &str) -> Option<String> {
        resolve(text, &SearchEngine::default())
    }

    #[test]
    fn empty_or_blank_input_gives_nothing() {
        assert_eq!(go(""), None);
        assert_eq!(go("   \t"), None);
    }

    #[test]
    fn full_addresses_load_as_typed() {
        assert_eq!(go("https://example.com/a?b=c").as_deref(), Some("https://example.com/a?b=c"));
        assert_eq!(go("HTTP://Example.com").as_deref(), Some("HTTP://Example.com"));
        assert_eq!(go("  https://example.com  ").as_deref(), Some("https://example.com"));
        assert_eq!(go("about:blank").as_deref(), Some("about:blank"));
        assert_eq!(go("view-source:https://example.com").as_deref(), Some("view-source:https://example.com"));
    }

    #[test]
    fn bare_domains_load_over_https() {
        assert_eq!(go("example.com").as_deref(), Some("https://example.com"));
        assert_eq!(go("www.example.co.uk/path?x=1").as_deref(), Some("https://www.example.co.uk/path?x=1"));
        assert_eq!(go("bücher.de").as_deref(), Some("https://bücher.de"));
        assert_eq!(go("xn--bcher-kva.xn--p1ai").as_deref(), Some("https://xn--bcher-kva.xn--p1ai"));
    }

    #[test]
    fn home_devices_load_over_http() {
        assert_eq!(go("192.168.1.1").as_deref(), Some("http://192.168.1.1"));
        assert_eq!(go("192.168.1.1/admin").as_deref(), Some("http://192.168.1.1/admin"));
        assert_eq!(go("tv.local:8080").as_deref(), Some("http://tv.local:8080"));
        assert_eq!(go("router:80/status").as_deref(), Some("http://router:80/status"));
        assert_eq!(go("localhost").as_deref(), Some("http://localhost"));
        assert_eq!(go("localhost:3000").as_deref(), Some("http://localhost:3000"));
    }

    #[test]
    fn everything_else_is_a_search() {
        assert_eq!(go("cat videos").as_deref(), Some("https://duckduckgo.com/?q=cat%20videos"));
        assert_eq!(go("weather").as_deref(), Some("https://duckduckgo.com/?q=weather"));
        assert_eq!(go("example.com is down").as_deref(), Some("https://duckduckgo.com/?q=example.com%20is%20down"));
        assert_eq!(go("example.").as_deref(), Some("https://duckduckgo.com/?q=example."));
        assert_eq!(go("a.1").as_deref(), Some("https://duckduckgo.com/?q=a.1"));
        assert_eq!(go("-bad-.com").as_deref(), Some("https://duckduckgo.com/?q=-bad-.com"));
        assert_eq!(go("999.1.1.1").as_deref(), Some("https://duckduckgo.com/?q=999.1.1.1"));
        assert_eq!(go("mailto:someone").as_deref(), Some("https://duckduckgo.com/?q=mailto%3Asomeone"));
        assert_eq!(go("javascript:alert(1)").as_deref(), Some("https://duckduckgo.com/?q=javascript%3Aalert(1)"));
        assert_eq!(go("file:///etc/passwd").as_deref(), Some("https://duckduckgo.com/?q=file%3A%2F%2F%2Fetc%2Fpasswd"));
    }

    #[test]
    fn search_queries_are_encoded_like_android() {
        let engine = SearchEngine::default();
        assert_eq!(engine.search_url("a&b=c#d"), "https://duckduckgo.com/?q=a%26b%3Dc%23d");
        assert_eq!(engine.search_url("crème brûlée"), "https://duckduckgo.com/?q=cr%C3%A8me%20br%C3%BBl%C3%A9e");
        assert_eq!(engine.search_url("it's (ok)!*~"), "https://duckduckgo.com/?q=it's%20(ok)!*~");
    }

    #[test]
    fn every_engine_has_a_query_slot_and_a_unique_id() {
        let mut ids: Vec<&str> = ENGINES.iter().map(|e| e.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), ENGINES.len());
        assert!(ENGINES.iter().all(|e| e.template.starts_with("https://") && e.template.contains("%s")));
        assert_eq!(engine("kagi").map(|e| e.paid), Some(true));
        assert!(engine("nope").is_none());
    }

    #[test]
    fn display_drops_scheme_www_and_trailing_slash() {
        assert_eq!(display("https://www.example.com/"), "example.com");
        assert_eq!(display("http://example.com/a/b/"), "example.com/a/b");
        assert_eq!(display("HTTPS://example.com"), "example.com");
        assert_eq!(display("about:blank"), "about:blank");
        assert_eq!(display(""), "");
    }
}
