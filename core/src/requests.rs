//! Which requests a page may make.
//!
//! Every request the web engine is about to send comes through [`RequestPolicy::should_block`]: first a plain list
//! of hosts (a development aid), then the ad blocker ([`crate::ads`]). Site filters build on this same seam.

use std::collections::HashSet;
use std::sync::Arc;

use url::Url;

use crate::ads::AdBlocker;

/// What a request is for, as the web engine reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    /// A page in the main frame.
    Page,
    /// A page in a frame inside another page.
    Frame,
    /// A style sheet.
    Stylesheet,
    /// A script, a worker's script included.
    Script,
    /// A picture, a site icon included.
    Image,
    /// A font.
    Font,
    /// Sound or video.
    Media,
    /// A plug-in's content.
    Object,
    /// A script's own fetch (`fetch`, `XMLHttpRequest`).
    Xhr,
    /// A ping or a beacon: a message sent as the user leaves or clicks.
    Ping,
    /// A WebSocket.
    WebSocket,
    /// A report of a Content Security Policy breach.
    CspReport,
    /// Anything else (prefetches and the like).
    Other,
}

impl ResourceKind {
    /// The name filter lists use for this kind (`$script`, `$image`…).
    #[must_use]
    pub const fn as_filter_type(self) -> &'static str {
        match self {
            Self::Page => "document",
            Self::Frame => "subdocument",
            Self::Stylesheet => "stylesheet",
            Self::Script => "script",
            Self::Image => "image",
            Self::Font => "font",
            Self::Media => "media",
            Self::Object => "object",
            Self::Xhr => "xmlhttprequest",
            Self::Ping => "ping",
            Self::WebSocket => "websocket",
            Self::CspReport => "csp_report",
            Self::Other => "other",
        }
    }
}

/// A request the web engine is about to send.
#[derive(Debug, Clone, Copy)]
pub struct Request<'a> {
    /// The address asked for.
    pub url: &'a str,
    /// The page in the main frame that the request is made for ("" when there is none).
    pub first_party: &'a str,
    /// What the request is for.
    pub kind: ResourceKind,
}

/// The rules every request is checked against.
#[derive(Debug, Default, Clone)]
pub struct RequestPolicy {
    blocked_hosts: HashSet<Box<str>>,
    ads: Option<Arc<AdBlocker>>,
}

impl RequestPolicy {
    /// A policy that blocks `hosts` and every host under them (`example.com` also blocks `ads.example.com`), pages
    /// included.
    ///
    /// Host names are compared in lower case; empty entries and surrounding dots are ignored.
    #[must_use]
    pub fn with_blocked_hosts<I, S>(hosts: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let blocked_hosts = hosts
            .into_iter()
            .map(|h| h.as_ref().trim().trim_matches('.').to_ascii_lowercase())
            .filter(|h| !h.is_empty())
            .map(String::into_boxed_str)
            .collect();
        Self { blocked_hosts, ads: None }
    }

    /// This policy, with `ads` deciding about ads and trackers as well.
    #[must_use]
    pub fn with_ad_blocker(mut self, ads: Arc<AdBlocker>) -> Self {
        self.ads = Some(ads);
        self
    }

    /// Whether `request` must not be sent. An address that doesn't parse, or has no host, is let through: the
    /// engine itself refuses what it can't load.
    #[must_use]
    pub fn should_block(&self, request: &Request<'_>) -> bool {
        self.blocks_host(request.url) || self.ads.as_ref().is_some_and(|ads| ads.blocks(request))
    }

    fn blocks_host(&self, url: &str) -> bool {
        if self.blocked_hosts.is_empty() {
            return false;
        }
        let Ok(parsed) = Url::parse(url) else {
            return false;
        };
        let Some(host) = parsed.host_str() else {
            return false;
        };
        let host = host.trim_end_matches('.');
        host_and_parents(host).any(|h| self.blocked_hosts.contains(h))
    }
}

/// `a.b.example.com`, `b.example.com`, `example.com`, `com`.
fn host_and_parents(host: &str) -> impl Iterator<Item = &str> {
    std::iter::successors(Some(host), |h| h.split_once('.').map(|(_, parent)| parent))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(url: &str) -> Request<'_> {
        Request { url, first_party: "", kind: ResourceKind::Other }
    }

    #[test]
    fn an_empty_policy_lets_everything_through() {
        let policy = RequestPolicy::default();
        assert!(!policy.should_block(&get("https://ads.example.com/a.js")));
    }

    #[test]
    fn a_blocked_host_blocks_itself_and_its_subdomains() {
        let policy = RequestPolicy::with_blocked_hosts(["example.com"]);
        assert!(policy.should_block(&get("https://example.com/")));
        assert!(policy.should_block(&get("https://ads.example.com/a.js")));
        assert!(policy.should_block(&get("http://a.b.EXAMPLE.com./x")));
        assert!(policy.should_block(&get("wss://example.com:8443/socket")));
    }

    #[test]
    fn similar_looking_hosts_are_not_blocked() {
        let policy = RequestPolicy::with_blocked_hosts(["example.com"]);
        assert!(!policy.should_block(&get("https://notexample.com/")));
        assert!(!policy.should_block(&get("https://example.com.evil.test/")));
        assert!(!policy.should_block(&get("https://other.test/?u=example.com")));
    }

    #[test]
    fn entries_are_cleaned_up() {
        let policy = RequestPolicy::with_blocked_hosts([" .Tracker.Test. ", "", "  "]);
        assert!(policy.should_block(&get("https://cdn.tracker.test/t.gif")));
        assert_eq!(policy.blocked_hosts.len(), 1);
    }

    #[test]
    fn the_ad_blocker_is_asked_after_the_host_list() {
        let ads = Arc::new(AdBlocker::new(true));
        ads.load(["ads.example\n"]);
        let policy = RequestPolicy::with_blocked_hosts(["blocked.example"]).with_ad_blocker(Arc::clone(&ads));
        let script = |url| Request { url, first_party: "https://news.example/", kind: ResourceKind::Script };
        assert!(policy.should_block(&script("https://ads.example/a.js")));
        assert!(policy.should_block(&script("https://blocked.example/a.js")));
        assert!(!policy.should_block(&script("https://news.example/a.js")));
        ads.set_enabled(false);
        assert!(!policy.should_block(&script("https://ads.example/a.js")));
    }

    #[test]
    fn addresses_without_a_host_pass() {
        let policy = RequestPolicy::with_blocked_hosts(["example.com"]);
        assert!(!policy.should_block(&get("data:text/plain,example.com")));
        assert!(!policy.should_block(&get("not a url")));
        assert!(!policy.should_block(&get("about:blank")));
    }
}
