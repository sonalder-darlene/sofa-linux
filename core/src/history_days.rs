//! Settings → Keep history for: how long visited pages stay in History. The choices and their stored values are
//! Sofa on Android's (`history_days`), so a settings backup reads the same on both.

/// Not a number of days: keep everything.
pub const ALWAYS: i32 = -1;
/// Not a number of days: keep nothing, and record nothing.
pub const NEVER: i32 = 0;
/// The choices, in the order Settings lists them.
pub const CHOICES: [i32; 5] = [7, 30, 90, ALWAYS, NEVER];
/// What Sofa keeps until the user chooses.
pub const DEFAULT: i32 = 90;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

/// The stored value as a choice: one of [`CHOICES`], else [`DEFAULT`].
#[must_use]
pub fn from_stored(value: &str) -> i32 {
    value.trim().parse().ok().filter(|days| CHOICES.contains(days)).unwrap_or(DEFAULT)
}

/// Pages visited before this time (milliseconds since 1970) are removed; `None` keeps everything. [`NEVER`] keeps
/// nothing.
#[must_use]
pub fn cutoff(days: i32, now: i64) -> Option<i64> {
    match days {
        ALWAYS => None,
        d if d <= NEVER => Some(now),
        d => Some(now.saturating_sub(i64::from(d) * DAY_MS)),
    }
}

/// Whether new visits are recorded at all.
#[must_use]
pub const fn records(days: i32) -> bool {
    days != NEVER
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000_000;

    #[test]
    fn a_period_keeps_that_many_days() {
        assert_eq!(cutoff(7, NOW), Some(NOW - 7 * DAY_MS));
        assert_eq!(cutoff(90, NOW), Some(NOW - 90 * DAY_MS));
    }

    #[test]
    fn always_keeps_everything_and_never_keeps_and_records_nothing() {
        assert_eq!(cutoff(ALWAYS, NOW), None);
        assert_eq!(cutoff(NEVER, NOW), Some(NOW));
        assert!(records(ALWAYS));
        assert!(records(7));
        assert!(!records(NEVER));
    }

    #[test]
    fn a_stored_value_that_is_not_a_choice_reads_as_the_default() {
        assert_eq!(from_stored("30"), 30);
        assert_eq!(from_stored("-1"), ALWAYS);
        assert_eq!(from_stored("0"), NEVER);
        for odd in ["", "12", "90 days", "-2", "99999999999"] {
            assert_eq!(from_stored(odd), DEFAULT, "{odd:?}");
        }
        assert!(CHOICES.contains(&DEFAULT));
    }
}
