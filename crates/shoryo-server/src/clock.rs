//! The server's clock: the time stamps and sent rounds are recorded with
//! (`docs/spec/server.md`, "状態データ").

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use shoryo_core::Timestamp;

/// The server's clock now, in UTC.
pub fn now() -> Timestamp {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    Timestamp::new(rfc3339_utc(since_epoch))
}

/// `since_epoch` as an RFC 3339 time in UTC with milliseconds, such as
/// `2026-10-08T04:01:13.123Z`. The milliseconds are cut, not rounded, so the time never lies
/// after the moment it was read.
fn rfc3339_utc(since_epoch: Duration) -> String {
    let seconds = since_epoch.as_secs();
    let days = seconds / 86_400;
    let of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60,
        since_epoch.subsec_millis()
    )
}

/// The proleptic Gregorian date `days` after 1970-01-01 (Howard Hinnant's `civil_from_days`).
// Written out because the standard library has no calendar, and these few lines are all the
// server needs of a date-time crate.
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let shifted = days + 719_468;
    let era = shifted / 146_097;
    let day_of_era = shifted % 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_the_first_of_january_1970() {
        assert_eq!(rfc3339_utc(Duration::ZERO), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn time_is_written_in_utc_with_milliseconds() {
        let since_epoch = Duration::from_millis(1_791_432_073_123);

        assert_eq!(rfc3339_utc(since_epoch), "2026-10-08T04:01:13.123Z");
    }

    #[test]
    fn leap_day_and_year_end_are_dated_correctly() {
        assert_eq!(
            rfc3339_utc(Duration::from_secs(951_782_400)),
            "2000-02-29T00:00:00.000Z"
        );
        assert_eq!(
            rfc3339_utc(Duration::from_secs(1_798_761_599)),
            "2026-12-31T23:59:59.000Z"
        );
    }

    #[test]
    fn milliseconds_are_cut_not_rounded() {
        let since_epoch = Duration::from_nanos(1_999_999_999);

        assert_eq!(rfc3339_utc(since_epoch), "1970-01-01T00:00:01.999Z");
    }
}
