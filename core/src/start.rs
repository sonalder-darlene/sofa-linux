//! Settings → Open at start: what Sofa shows when it starts without an address to open. Home always opens the start
//! page, whatever is chosen here, so the start page can't be lost.

/// The three choices. The ids are stored in settings (and in a settings backup): they never change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Choice {
    /// The start page (the default).
    #[default]
    StartPage,
    /// A web page the user typed.
    WebPage,
    /// Where the user left off: the tab that was in sight, if a page was.
    LastPage,
}

impl Choice {
    /// The stored id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::StartPage => "home",
            Self::WebPage => "page",
            Self::LastPage => "last",
        }
    }

    /// The choice stored as `id`; the start page for anything else.
    #[must_use]
    pub fn from_id(id: &str) -> Self {
        match id {
            "page" => Self::WebPage,
            "last" => Self::LastPage,
            _ => Self::StartPage,
        }
    }
}

/// The address to open at start, or `None` for the start page.
#[must_use]
pub fn address(choice: Choice, web_page: &str, last_page: &str) -> Option<String> {
    match choice {
        Choice::StartPage => None,
        Choice::WebPage => web_address(web_page),
        Choice::LastPage => web_address(last_page),
    }
}

/// What "Where I left off" keeps when Sofa closes: the page in sight (`None` while the start page shows). Nothing when
/// another choice is made, or in private browsing.
#[must_use]
pub fn to_remember(choice: Choice, page_in_sight: Option<&str>, private: bool) -> String {
    if choice != Choice::LastPage || private {
        return String::new();
    }
    page_in_sight.and_then(web_address).unwrap_or_default()
}

/// A typed address as Sofa keeps it for "A web page": `example.com` and `example.com/tv` mean https, as does a full
/// address typed after the `https://` the field starts with. Anything that isn't a web page (`file:`, `javascript:`,
/// `data:`, `about:`, no site at all, spaces) gives `None`.
///
/// # Examples
///
/// ```
/// use sofa_core::start::web_address;
///
/// assert_eq!(web_address("example.com/tv").as_deref(), Some("https://example.com/tv"));
/// assert_eq!(web_address("about:blank"), None);
/// ```
#[must_use]
pub fn web_address(typed: &str) -> Option<String> {
    let mut t = typed.trim();
    if starts_with_ignore_case(t, "https://") && t[8..].contains("://") {
        t = &t[8..];
    }
    if t.is_empty() || t.chars().any(char::is_whitespace) {
        return None;
    }
    let owned;
    if !t.contains("://") {
        // "javascript:…" or "about:blank" name a scheme; "localhost:8080" names a port.
        if names_a_scheme(t) {
            return None;
        }
        owned = format!("https://{t}");
        t = &owned;
    }
    let (scheme, rest) = t.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, tail) = rest.split_at(end);
    if authority.is_empty() || !is_host_port(authority.rsplit('@').next().unwrap_or(authority)) {
        return None;
    }
    Some(format!("{}://{authority}{tail}", scheme.to_ascii_lowercase()))
}

/// How Settings names the page: its host, with `http://` in front when the page isn't https.
#[must_use]
pub fn label(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_owned();
    };
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = rest[..end].rsplit('@').next().unwrap_or("");
    let host = host.strip_prefix("www.").unwrap_or(host);
    if scheme.eq_ignore_ascii_case("http") { format!("http://{host}") } else { host.to_owned() }
}

fn starts_with_ignore_case(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

/// `^[a-zA-Z][a-zA-Z0-9+.-]*:` not followed by a digit (a port).
fn names_a_scheme(t: &str) -> bool {
    let b = t.as_bytes();
    if !b.first().is_some_and(u8::is_ascii_alphabetic) {
        return false;
    }
    for (i, c) in b.iter().enumerate().skip(1) {
        match c {
            b':' => return !b.get(i + 1).is_some_and(u8::is_ascii_digit),
            c if c.is_ascii_alphanumeric() || matches!(c, b'+' | b'.' | b'-') => {}
            _ => return false,
        }
    }
    false
}

/// A name (any script, for IDN) or an IPv4 or `[IPv6]` address, then an optional port of up to five digits.
fn is_host_port(s: &str) -> bool {
    const BAD: &[char] = &['<', '>', '"', '\\', '{', '}', '|', '^', '`', '%', '@'];
    let (host, port) = if let Some(rest) = s.strip_prefix('[') {
        let Some((inside, after)) = rest.split_once(']') else { return false };
        if inside.is_empty() || !inside.chars().all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.') {
            return false;
        }
        let port = match after.strip_prefix(':') {
            Some(p) => Some(p),
            None if after.is_empty() => None,
            None => return false,
        };
        (None, port)
    } else {
        let (h, p) = match s.split_once(':') {
            Some((h, p)) => (h, Some(p)),
            None => (s, None),
        };
        (Some(h), p)
    };
    if let Some(h) = host {
        if h.is_empty()
            || h.starts_with('.')
            || h.chars().any(|c| c.is_whitespace() || c == '[' || c == ']' || BAD.contains(&c))
        {
            return false;
        }
    }
    port.is_none_or(|p| (1..=5).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_never_change() {
        for c in [Choice::StartPage, Choice::WebPage, Choice::LastPage] {
            assert_eq!(Choice::from_id(c.id()), c);
        }
        assert_eq!([Choice::StartPage.id(), Choice::WebPage.id(), Choice::LastPage.id()], ["home", "page", "last"]);
        assert_eq!(Choice::from_id("bogus"), Choice::StartPage);
    }

    #[test]
    fn typed_addresses_become_web_pages() {
        assert_eq!(web_address("example.com").as_deref(), Some("https://example.com"));
        assert_eq!(web_address("  HTTP://Example.com/Path ").as_deref(), Some("http://Example.com/Path"));
        assert_eq!(web_address("https://https://example.org").as_deref(), Some("https://example.org"));
        assert_eq!(web_address("localhost:8080").as_deref(), Some("https://localhost:8080"));
        assert_eq!(web_address("http://192.168.1.1:80/x").as_deref(), Some("http://192.168.1.1:80/x"));
        assert_eq!(web_address("http://[::1]:8080/").as_deref(), Some("http://[::1]:8080/"));
    }

    #[test]
    fn anything_else_is_refused() {
        for bad in [
            "",
            "  ",
            "about:blank",
            "javascript:alert(1)",
            "file:///etc",
            "ftp://x.example",
            "a b.example",
            "https://",
            "https://example.orgabout:blank",
            "https://ex%ample.com",
            "https://.example",
            "http://x:123456",
        ] {
            assert_eq!(web_address(bad), None, "{bad}");
        }
    }

    #[test]
    fn labels_show_the_host() {
        assert_eq!(label("https://www.example.com/tv"), "example.com");
        assert_eq!(label("http://192.168.1.1/"), "http://192.168.1.1");
    }

    #[test]
    fn where_i_left_off_remembers_only_what_it_should() {
        assert_eq!(to_remember(Choice::LastPage, Some("https://example.com/a"), false), "https://example.com/a");
        assert_eq!(to_remember(Choice::LastPage, Some("https://example.com/a"), true), "");
        assert_eq!(to_remember(Choice::LastPage, None, false), "");
        assert_eq!(to_remember(Choice::WebPage, Some("https://example.com/a"), false), "");
        assert_eq!(address(Choice::LastPage, "", "https://example.com/a").as_deref(), Some("https://example.com/a"));
        assert_eq!(address(Choice::WebPage, "example.org", "").as_deref(), Some("https://example.org"));
        assert_eq!(address(Choice::StartPage, "example.org", "https://x.example"), None);
    }
}
