//! Small rules about web addresses that several screens share: which addresses are web pages, a page's site, and a
//! readable name for a site that never gave itself one.

use url::{Host, Url};

/// Whether `url` is a web page Sofa keeps (in history, tabs, bookmarks): `http` or `https` with a host.
///
/// # Examples
///
/// ```
/// use sofa_core::sites::is_web_url;
///
/// assert!(is_web_url("https://example.com/a"));
/// assert!(!is_web_url("about:blank"));
/// assert!(!is_web_url("file:///tmp/a.html"));
/// ```
#[must_use]
pub fn is_web_url(url: &str) -> bool {
    Url::parse(url).is_ok_and(|u| matches!(u.scheme(), "http" | "https") && u.host_str().is_some_and(|h| !h.is_empty()))
}

/// The host of `url` in lower case, without a leading `www.`; `None` for an address without one.
#[must_use]
pub fn host_of(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let host = parsed.host_str()?.trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() {
        return None;
    }
    Some(host.strip_prefix("www.").map(str::to_owned).unwrap_or(host))
}

/// Whether two addresses are on the same site (the same host, `www.` or not), as "Open at start" uses it to find a
/// tab already open on the chosen page's site.
#[must_use]
pub fn same_site(a: &str, b: &str) -> bool {
    matches!((host_of(a), host_of(b)), (Some(x), Some(y)) if x == y)
}

/// Whether two addresses belong to one site as browsers draw the line between sites: the same registrable domain
/// under the Public Suffix List, so `news.example.co.uk` and `video.example.co.uk` do, and `a.github.io` and
/// `b.github.io` don't. An IP address, or a host with no registrable domain (`localhost`), is a site of its own.
///
/// # Examples
///
/// ```
/// use sofa_core::sites::same_registrable_domain;
///
/// assert!(same_registrable_domain("https://news.example.co.uk/", "https://video.example.co.uk/x"));
/// assert!(!same_registrable_domain("https://example.com/", "https://example.org/"));
/// ```
#[must_use]
pub fn same_registrable_domain(a: &str, b: &str) -> bool {
    matches!((registrable_domain(a), registrable_domain(b)), (Some(x), Some(y)) if x == y)
}

/// The registrable domain of `url`'s host in lower case (see [`same_registrable_domain`]); `None` without a host.
#[must_use]
pub fn registrable_domain(url: &str) -> Option<String> {
    let parsed = Url::parse(url).ok()?;
    let host = match parsed.host()? {
        Host::Domain(d) => d.trim_end_matches('.').to_ascii_lowercase(),
        // Written as the parser writes it, so `[::1]` and `[0:0::1]` are one site.
        Host::Ipv4(ip) => return Some(ip.to_string()),
        Host::Ipv6(ip) => return Some(ip.to_string()),
    };
    if host.is_empty() {
        return None;
    }
    Some(psl::domain_str(&host).map_or_else(|| host.clone(), str::to_owned))
}

/// A readable name for a site that never gave its home page a title: `news.example.co.uk` → `News Example`.
/// Addresses (IPv4, IPv6, `localhost`) are already as readable as they get.
///
/// # Examples
///
/// ```
/// assert_eq!(sofa_core::sites::pretty_host("www.example.com"), "Example");
/// assert_eq!(sofa_core::sites::pretty_host("192.168.1.1"), "192.168.1.1");
/// ```
#[must_use]
pub fn pretty_host(host: &str) -> String {
    if host.chars().all(|c| c.is_ascii_digit() || c == '.') || host.contains(':') || !host.contains('.') {
        return host.to_owned();
    }
    let host = host.strip_prefix("www.").unwrap_or(host);
    let parts: Vec<&str> = host.split('.').collect();
    // A two-letter-or-three country second level (co.uk, com.au) goes with the top level.
    let keep = if parts.len() > 2 && parts[parts.len() - 2].len() <= 3 { parts.len() - 2 } else { parts.len() - 1 };
    let core = if keep == 0 { &parts[..] } else { &parts[..keep] };
    core.iter().map(|p| capitalise(p)).collect::<Vec<_>>().join(" ")
}

fn capitalise(word: &str) -> String {
    let mut chars = word.chars();
    chars.next().map(|first| first.to_uppercase().chain(chars).collect()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_urls_need_http_and_a_host() {
        assert!(is_web_url("http://192.168.1.1:8080/x"));
        assert!(is_web_url("HTTPS://Example.com"));
        assert!(!is_web_url("https://"));
        assert!(!is_web_url("javascript:alert(1)"));
        assert!(!is_web_url("data:text/html,hi"));
        assert!(!is_web_url(""));
    }

    #[test]
    fn hosts_drop_www_and_case() {
        assert_eq!(host_of("https://WWW.Example.com./a").as_deref(), Some("example.com"));
        assert_eq!(host_of("https://sub.example.com").as_deref(), Some("sub.example.com"));
        assert_eq!(host_of("about:blank"), None);
    }

    #[test]
    fn same_site_ignores_www_and_paths() {
        assert!(same_site("https://www.example.com/a", "http://example.com/b?c"));
        assert!(!same_site("https://example.com", "https://example.org"));
        assert!(!same_site("about:blank", "about:blank"));
    }

    #[test]
    fn registrable_domains_follow_the_public_suffix_list() {
        assert_eq!(registrable_domain("https://www.News.Example.co.uk./a").as_deref(), Some("example.co.uk"));
        assert_eq!(registrable_domain("https://a.github.io/").as_deref(), Some("a.github.io"));
        assert_eq!(registrable_domain("http://localhost:8080/").as_deref(), Some("localhost"));
        assert_eq!(registrable_domain("http://127.0.0.1:8765/x").as_deref(), Some("127.0.0.1"));
        assert_eq!(registrable_domain("about:blank"), None);
        assert!(same_registrable_domain("https://cdn.example.com/", "https://example.com/"));
        assert!(!same_registrable_domain("https://a.github.io/", "https://b.github.io/"));
        assert!(!same_registrable_domain("http://127.0.0.1/", "http://127.0.0.2/"));
        assert!(same_registrable_domain("http://[::1]:1/", "http://[0:0::1]:2/"));
    }

    #[test]
    fn pretty_hosts_read_like_names() {
        assert_eq!(pretty_host("news.example.co.uk"), "News Example");
        assert_eq!(pretty_host("example.org"), "Example");
        assert_eq!(pretty_host("localhost"), "localhost");
        assert_eq!(pretty_host("[::1]"), "[::1]");
        assert_eq!(pretty_host("a.b.c.example.com"), "A B C Example");
    }
}
