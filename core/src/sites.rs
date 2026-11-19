//! Small rules about web addresses that several screens share: which addresses are web pages, a page's site, and a
//! readable name for a site that never gave itself one.

use url::Url;

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
    fn pretty_hosts_read_like_names() {
        assert_eq!(pretty_host("news.example.co.uk"), "News Example");
        assert_eq!(pretty_host("example.org"), "Example");
        assert_eq!(pretty_host("localhost"), "localhost");
        assert_eq!(pretty_host("[::1]"), "[::1]");
        assert_eq!(pretty_host("a.b.c.example.com"), "A B C Example");
    }
}
