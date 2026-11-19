//! Which requests a page may make.
//!
//! Every request the web engine is about to send comes through [`RequestPolicy::should_block`]. Today the policy
//! knows a plain list of hosts; ad blocking with filter lists and site filters build on this same seam.

use std::collections::HashSet;

use url::Url;

/// The rules every request is checked against.
#[derive(Debug, Default, Clone)]
pub struct RequestPolicy {
    blocked_hosts: HashSet<Box<str>>,
}

impl RequestPolicy {
    /// A policy that blocks `hosts` and every host under them (`example.com` also blocks `ads.example.com`).
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
        Self { blocked_hosts }
    }

    /// Whether the request for `url` must not be sent. An address that doesn't parse, or has no host, is let
    /// through: the engine itself refuses what it can't load.
    #[must_use]
    pub fn should_block(&self, url: &str) -> bool {
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

    #[test]
    fn an_empty_policy_lets_everything_through() {
        let policy = RequestPolicy::default();
        assert!(!policy.should_block("https://ads.example.com/a.js"));
    }

    #[test]
    fn a_blocked_host_blocks_itself_and_its_subdomains() {
        let policy = RequestPolicy::with_blocked_hosts(["example.com"]);
        assert!(policy.should_block("https://example.com/"));
        assert!(policy.should_block("https://ads.example.com/a.js"));
        assert!(policy.should_block("http://a.b.EXAMPLE.com./x"));
        assert!(policy.should_block("wss://example.com:8443/socket"));
    }

    #[test]
    fn similar_looking_hosts_are_not_blocked() {
        let policy = RequestPolicy::with_blocked_hosts(["example.com"]);
        assert!(!policy.should_block("https://notexample.com/"));
        assert!(!policy.should_block("https://example.com.evil.test/"));
        assert!(!policy.should_block("https://other.test/?u=example.com"));
    }

    #[test]
    fn entries_are_cleaned_up() {
        let policy = RequestPolicy::with_blocked_hosts([" .Tracker.Test. ", "", "  "]);
        assert!(policy.should_block("https://cdn.tracker.test/t.gif"));
        assert_eq!(policy.blocked_hosts.len(), 1);
    }

    #[test]
    fn addresses_without_a_host_pass() {
        let policy = RequestPolicy::with_blocked_hosts(["example.com"]);
        assert!(!policy.should_block("data:text/plain,example.com"));
        assert!(!policy.should_block("not a url"));
        assert!(!policy.should_block("about:blank"));
    }
}
