//! Whether the web engine is old enough to say so: Sofa can't update the engine (the system or the Flatpak runtime
//! brings it), only tell the user once in a while. The same rule as Sofa on Android, where it judges the TV's WebView.
//!
//! The version judged is the one Chromium's **security fixes** reach, not the Chromium the engine was built from:
//! Qt WebEngine keeps an older Chromium base and carries the newer fixes over to it, so the base alone would always
//! look old.

/// Engines whose security fixes stop below this Chromium major version get the notice: about a year of Chrome
/// releases behind the stable one (157 in November 2026). Raise it with each release, as on Android.
pub const MINIMUM: u32 = 144;

/// How long a notice that was shown stays quiet while the engine doesn't change: about three months.
pub const REMIND_AFTER_SECS: u64 = 90 * 24 * 3600;

/// The major version at the start of a version such as `157.0.8103.50`; `None` when there is none.
#[must_use]
pub fn major(version: &str) -> Option<u32> {
    let (first, rest) = version.trim().split_once('.')?;
    // A bare number with no dot is not a Chromium version; neither is an empty or signed first part.
    if rest.is_empty() || first.is_empty() || !first.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    first.parse().ok()
}

/// Whether to show the notice now for the engine at `version` (the security-fix version), given the version it was
/// last shown for and when (seconds since the Unix epoch; 0 for never). Shown once per engine version, and again after
/// [`REMIND_AFTER_SECS`].
#[must_use]
pub fn should_warn(version: &str, warned_version: &str, warned_at: u64, now: u64) -> bool {
    let Some(major) = major(version) else { return false };
    major < MINIMUM && (version.trim() != warned_version || now.saturating_sub(warned_at) > REMIND_AFTER_SECS)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: u64 = 24 * 3600;

    #[test]
    fn major_reads_the_first_number_of_a_dotted_version() {
        assert_eq!(major("157.0.8103.50"), Some(157));
        assert_eq!(major(" 140.0.7339.264 "), Some(140));
        assert_eq!(major("157"), None);
        assert_eq!(major("157."), None);
        assert_eq!(major(""), None);
        assert_eq!(major("-1.0"), None);
        assert_eq!(major("v157.0"), None);
        assert_eq!(major("99999999999.0"), None);
    }

    #[test]
    fn a_current_engine_is_never_mentioned() {
        assert!(!should_warn("157.0.8103.50", "", 0, 1000 * DAY));
        assert!(!should_warn(&format!("{MINIMUM}.0.1.2"), "", 0, 1000 * DAY));
    }

    #[test]
    fn an_old_engine_is_mentioned_once_per_version_then_after_three_months() {
        let old = "132.0.6834.210";
        let now = 1000 * DAY;
        assert!(should_warn(old, "", 0, now));
        assert!(!should_warn(old, old, now, now + DAY));
        assert!(!should_warn(old, old, now, now + 90 * DAY));
        assert!(should_warn(old, old, now, now + 91 * DAY));
        // A different (still old) engine is new news.
        assert!(should_warn("138.0.7204.100", old, now, now + DAY));
    }

    #[test]
    fn an_unreadable_version_or_a_clock_gone_back_says_nothing_wrong() {
        assert!(!should_warn("unknown", "", 0, 1000 * DAY));
        let old = "132.0.6834.210";
        assert!(!should_warn(old, old, 1000 * DAY, 10 * DAY));
    }
}
