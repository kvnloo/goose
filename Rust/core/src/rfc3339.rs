//! Shared civil-date math for the hand-rolled RFC 3339 timestamp parsers.
//!
//! Three modules (`capture_import`, `bridge`, `activity_identity`) each
//! vendored their own copy of the days-from-civil algorithm with subtly
//! different validation. This module is the single home for that math so
//! the copies cannot drift apart again.

/// Days since the Unix epoch for a proleptic Gregorian civil date.
///
/// Howard Hinnant's `days_from_civil` algorithm. `month` and `day` are
/// 1-based; callers must validate ranges (see [`days_in_month`]).
pub fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = year as i64 - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_prime = month as i64 + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day as i64 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// True for Gregorian leap years.
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Days in `month` (1-based) of `year`. Returns 0 for an invalid month.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Inverse of [`days_from_civil`]: proleptic Gregorian (year, month, day)
/// with 1-based month and day for days since the Unix epoch.
pub fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    (year as i32, month as u32, day as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_day_zero() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
    }

    #[test]
    fn known_dates_match_calendar() {
        assert_eq!(days_from_civil(1969, 12, 31), -1);
        assert_eq!(days_from_civil(2000, 1, 1), 10957);
        assert_eq!(days_from_civil(2026, 5, 28), 20601);
        // Leap days.
        assert_eq!(days_from_civil(2024, 2, 29), 19782);
        assert_eq!(days_from_civil(2024, 3, 1) - days_from_civil(2024, 2, 28), 2);
        // Non-leap century.
        assert_eq!(days_from_civil(1900, 3, 1) - days_from_civil(1900, 2, 28), 1);
        // Leap century.
        assert_eq!(days_from_civil(2000, 2, 29), 11016);
    }

    #[test]
    fn month_lengths_cover_leap_rules() {
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2025, 2), 28);
        assert_eq!(days_in_month(1900, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(2026, 4), 30);
        assert_eq!(days_in_month(2026, 13), 0);
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2000));
        assert!(is_leap_year(2024));
    }

    #[test]
    fn civil_from_days_round_trips() {
        for (year, month, day) in [
            (1970, 1, 1),
            (1969, 12, 31),
            (2026, 5, 28),
            (2024, 2, 29),
            (1900, 2, 28),
            (2000, 2, 29),
            (2026, 12, 31),
            (1, 1, 1),
        ] {
            let days = days_from_civil(year, month, day);
            assert_eq!(civil_from_days(days), (year, month, day));
        }
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }
}
