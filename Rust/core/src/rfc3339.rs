//! Shared timestamp parsing and civil-date math for the hand-rolled RFC 3339 parsers.
//!
//! The crate parses RFC 3339 timestamps in ~16 places with genuinely different
//! semantics per call site: fraction precision (truncated vs millisecond vs
//! nanosecond), leap-second handling (reject vs clamp vs full second),
//! timezone offsets (`Z`-only vs numeric `±HH:MM`), and overflow behavior
//! (checked vs wrapping). This module is the single home for the parsing core
//! and the proleptic Gregorian helpers so the copies cannot drift apart again.
//!
//! Each call site keeps its exact semantics by selecting one of the
//! [`Rfc3339MsOptions`] presets; nothing user-visible changes. The presets are
//! the only sanctioned combinations — do not hand-roll new ones.

/// Days since the Unix epoch for a proleptic Gregorian civil date.
///
/// Howard Hinnant's `days_from_civil` algorithm. `month` and `day` are
/// 1-based; callers must validate ranges (see [`days_in_month`]).
pub fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_prime = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// True for Gregorian leap years.
pub fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Days in `month` (1-based) of `year`. Returns 0 for an invalid month.
pub fn days_in_month(year: i64, month: u32) -> u32 {
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

/// How a parser treats the fractional-seconds component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FractionMode {
    /// Drop the fraction entirely; whole seconds only.
    Truncate,
    /// Milliseconds from the leading digit run (up to 3 digits, right-padded
    /// with zeros). A non-digit fraction contributes 0 instead of rejecting.
    /// The fraction is split off the whole time component before segmenting
    /// on ':' (historical quirk; only malformed inputs notice).
    MillisLenient,
    /// Milliseconds from up to 3 digits, right-padded with zeros. Any
    /// non-digit character rejects the timestamp.
    MillisStrict,
}

/// Which timezone designators a parser accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetPolicy {
    /// Only a trailing `Z`; anything else is rejected.
    ZuluOnly,
    /// `Z` or a numeric `±HH:MM` offset, subtracted from the result.
    NumericOffset,
}

/// How a parser treats a leap second (`second == 60`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeapSecondPolicy {
    /// Reject `second == 60`.
    Reject,
    /// Accept `second == 60` but report it as second 59.
    ClampTo59,
    /// Accept `second == 60` as a full extra second.
    AllowFullSecond,
}

/// The semantic axes along which the crate's timestamp parsers differ.
/// Use one of the presets below; do not hand-roll new combinations.
#[derive(Debug, Clone, Copy)]
pub struct Rfc3339MsOptions {
    /// Strip surrounding ASCII whitespace before parsing.
    pub trim: bool,
    /// Which timezone designators are accepted.
    pub offset: OffsetPolicy,
    /// How the fractional-seconds component is interpreted.
    pub fraction: FractionMode,
    /// How `second == 60` is treated.
    pub leap_second: LeapSecondPolicy,
    /// Reject on arithmetic overflow instead of wrapping.
    pub checked: bool,
    /// Accept years outside the `i32` range (capture import's historical behavior).
    pub wide_year: bool,
}

/// `bridge` / `metrics`: `Z`-only, fraction truncated, leap seconds rejected.
pub const RFC3339_MS_TRUNCATE: Rfc3339MsOptions = Rfc3339MsOptions {
    trim: false,
    offset: OffsetPolicy::ZuluOnly,
    fraction: FractionMode::Truncate,
    leap_second: LeapSecondPolicy::Reject,
    checked: false,
    wide_year: false,
};

/// Rollup/validation/step parsers: trimmed, lenient millis, leap second clamped to 59.
pub const RFC3339_MS_LENIENT: Rfc3339MsOptions = Rfc3339MsOptions {
    trim: true,
    offset: OffsetPolicy::ZuluOnly,
    fraction: FractionMode::MillisLenient,
    leap_second: LeapSecondPolicy::ClampTo59,
    checked: false,
    wide_year: false,
};

/// `export` / `historical_sync` / `sleep_validation` / `metric_features` /
/// bridge capture-arrival: `Z`-only, strict millis, leap seconds rejected,
/// overflow is an error.
pub const RFC3339_MS_STRICT: Rfc3339MsOptions = Rfc3339MsOptions {
    trim: false,
    offset: OffsetPolicy::ZuluOnly,
    fraction: FractionMode::MillisStrict,
    leap_second: LeapSecondPolicy::Reject,
    checked: true,
    wide_year: false,
};

/// `capture_import`: trimmed, numeric `±HH:MM` offsets, strict millis, leap
/// second counts as a full second, `i64` years accepted.
pub const RFC3339_MS_OFFSET: Rfc3339MsOptions = Rfc3339MsOptions {
    trim: true,
    offset: OffsetPolicy::NumericOffset,
    fraction: FractionMode::MillisStrict,
    leap_second: LeapSecondPolicy::AllowFullSecond,
    checked: false,
    wide_year: true,
};

/// Parse an RFC 3339 timestamp to Unix milliseconds under `opts`.
///
/// Day-of-month is validated against [`days_in_month`] for every preset, so
/// impossible dates like `2026-02-30` are rejected everywhere.
pub fn parse_rfc3339_utc_unix_ms(value: &str, opts: Rfc3339MsOptions) -> Option<i64> {
    let value = if opts.trim { value.trim() } else { value };
    let (date, time, offset_seconds) = match opts.offset {
        OffsetPolicy::ZuluOnly => {
            let date_time = value.strip_suffix('Z')?;
            let (date, time) = date_time.split_once('T')?;
            (date, time, 0_i64)
        }
        OffsetPolicy::NumericOffset => {
            let (date, time_and_zone) = value.split_once('T')?;
            let (time, offset_seconds) = split_numeric_offset(time_and_zone)?;
            (date, time, offset_seconds)
        }
    };

    let mut date_parts = date.split('-');
    let year: i64 = date_parts.next()?.parse().ok()?;
    let month: u32 = date_parts.next()?.parse().ok()?;
    let day: u32 = date_parts.next()?.parse().ok()?;
    if date_parts.next().is_some() {
        return None;
    }

    // The lenient dialect splits the fraction off the whole time component
    // before segmenting on ':'; the others split it off the seconds segment.
    // Only malformed inputs (a '.' outside the seconds) can tell the difference.
    let (time_main, pre_fraction) = match opts.fraction {
        FractionMode::MillisLenient => time.split_once('.').unwrap_or((time, "")),
        _ => (time, ""),
    };
    let mut time_parts = time_main.split(':');
    let hour: i64 = time_parts.next()?.parse().ok()?;
    let minute: i64 = time_parts.next()?.parse().ok()?;
    let seconds_part = time_parts.next()?;
    if time_parts.next().is_some() {
        return None;
    }
    let (second_text, post_fraction) = seconds_part
        .split_once('.')
        .map_or((seconds_part, ""), |(second, fraction)| (second, fraction));
    let fraction_text = match opts.fraction {
        FractionMode::MillisLenient => pre_fraction,
        _ => post_fraction,
    };
    let second: i64 = second_text.parse().ok()?;
    let millis: i64 = match opts.fraction {
        FractionMode::Truncate => 0,
        FractionMode::MillisLenient => {
            let digits: String = fraction_text
                .chars()
                .take_while(|character| character.is_ascii_digit())
                .take(3)
                .collect();
            if digits.is_empty() {
                0
            } else {
                format!("{digits:0<3}").parse().ok()?
            }
        }
        FractionMode::MillisStrict => i64::from(parse_millis_fraction(fraction_text)?),
    };

    let second = match opts.leap_second {
        LeapSecondPolicy::Reject => {
            if second > 59 {
                return None;
            }
            second
        }
        LeapSecondPolicy::ClampTo59 => {
            if second > 60 {
                return None;
            }
            second.min(59)
        }
        LeapSecondPolicy::AllowFullSecond => {
            if second > 60 {
                return None;
            }
            second
        }
    };

    if !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
    {
        return None;
    }

    // Non-wide presets keep the historical i32 year boundary: larger years
    // fail to parse instead of reaching date math.
    let days = if opts.wide_year {
        days_from_civil(year, month, day)
    } else {
        days_from_civil(i64::from(i32::try_from(year).ok()?), month, day)
    };

    if opts.checked {
        days.checked_mul(86_400_000)?
            .checked_add(hour * 3_600_000)?
            .checked_add(minute * 60_000)?
            .checked_add(second * 1_000)?
            .checked_add(millis)
    } else {
        Some(
            (days * 86_400 + hour * 3_600 + minute * 60 + second) * 1_000 + millis
                - offset_seconds * 1_000,
        )
    }
}

/// Split `time_and_zone` into the wall-clock time and a numeric `±HH:MM`
/// offset in seconds. `Z` means zero. (capture import's exact semantics:
/// the first `+`/`-` past index 0 starts the offset; hour/minute need not
/// be two digits.)
fn split_numeric_offset(value: &str) -> Option<(&str, i64)> {
    if let Some(time) = value.strip_suffix('Z') {
        return Some((time, 0));
    }
    let sign_index = value
        .char_indices()
        .skip(1)
        .find_map(|(index, character)| (character == '+' || character == '-').then_some(index))?;
    let (time, offset) = value.split_at(sign_index);
    let sign = if offset.starts_with('+') { 1 } else { -1 };
    let mut parts = offset[1..].split(':');
    let hours = parts.next()?.parse::<i64>().ok()?;
    let minutes = parts.next()?.parse::<i64>().ok()?;
    if parts.next().is_some() || hours > 23 || minutes > 59 {
        return None;
    }
    Some((time, sign * (hours * 3600 + minutes * 60)))
}

/// Milliseconds from a fractional-seconds string: up to 3 digits,
/// right-padded with zeros. Empty means 0; any non-digit rejects.
pub fn parse_millis_fraction(value: &str) -> Option<u32> {
    if value.is_empty() {
        return Some(0);
    }
    if !value.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    let mut millis = 0_u32;
    let mut factor = 100_u32;
    for character in value.chars().take(3) {
        millis += character.to_digit(10)? * factor;
        factor /= 10;
    }
    Some(millis)
}

/// Parse a wall-clock time `HH:MM:SS[.fffffffff]` to
/// `(hour, minute, second, nanos)`. Shared by the nanosecond-precision
/// parsers; an empty or over-long fraction rejects.
pub fn parse_time_of_day(value: &str) -> Option<(u32, u32, u32, u32)> {
    let mut parts = value.split(':');
    let hour = parts.next()?.parse::<u32>().ok()?;
    let minute = parts.next()?.parse::<u32>().ok()?;
    let seconds_part = parts.next()?;
    if parts.next().is_some() || hour > 23 || minute > 59 {
        return None;
    }
    let (second_text, fraction_text) = seconds_part
        .split_once('.')
        .map_or((seconds_part, None), |(second, fraction)| {
            (second, Some(fraction))
        });
    let second = second_text.parse::<u32>().ok()?;
    if second > 59 {
        return None;
    }
    let nanos = match fraction_text {
        Some(fraction) if fraction.is_empty() || fraction.len() > 9 => return None,
        Some(fraction) => {
            if !fraction.chars().all(|character| character.is_ascii_digit()) {
                return None;
            }
            let mut padded = fraction.to_string();
            while padded.len() < 9 {
                padded.push('0');
            }
            padded.parse::<u32>().ok()?
        }
        None => 0,
    };
    Some((hour, minute, second, nanos))
}

/// Parse an RFC 3339 timestamp to Unix nanoseconds (`i128`).
/// Shared core of the nanosecond-precision parsers in
/// `activity_identity` and `health_sync`.
pub fn parse_utc_instant_nanos(value: &str) -> Option<i128> {
    let value = value.trim();
    let (date, time_and_offset) = value.split_once('T')?;
    let (year, month, day) = parse_date_ymd(date)?;
    let (time, offset_seconds) = split_strict_offset(time_and_offset)?;
    let (hour, minute, second, nanos) = parse_time_of_day(time)?;
    let days = days_from_civil(i64::from(year), month, day);
    let seconds = i128::from(days) * 86_400
        + i128::from(hour) * 3_600
        + i128::from(minute) * 60
        + i128::from(second)
        - i128::from(offset_seconds);
    Some(seconds * 1_000_000_000 + i128::from(nanos))
}

/// Strict `YYYY-MM-DD` date: exactly 10 bytes, month-aware day check.
fn parse_date_ymd(value: &str) -> Option<(i32, u32, u32)> {
    let mut parts = value.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    let day = parts.next()?.parse::<u32>().ok()?;
    if parts.next().is_some() || value.len() != 10 {
        return None;
    }
    if !(1..=12).contains(&month) {
        return None;
    }
    if !(1..=days_in_month(i64::from(year), month)).contains(&day) {
        return None;
    }
    Some((year, month, day))
}

/// Split `time_and_offset` into wall-clock time and offset seconds.
/// `Z` means zero; otherwise the LAST `+`/`-` starts a strict 2-digit
/// `HH:MM` offset. (The nanosecond parsers' exact semantics; deliberately
/// different from [`split_numeric_offset`].)
fn split_strict_offset(value: &str) -> Option<(&str, i32)> {
    if let Some(time) = value.strip_suffix('Z') {
        return Some((time, 0));
    }
    let split_index = value.rfind(['+', '-'])?;
    if split_index == 0 {
        return None;
    }
    let (time, offset) = value.split_at(split_index);
    let sign = if offset.starts_with('+') { 1 } else { -1 };
    let offset = &offset[1..];
    let (hour, minute) = offset.split_once(':')?;
    if hour.len() != 2 || minute.len() != 2 {
        return None;
    }
    let hour = hour.parse::<i32>().ok()?;
    let minute = minute.parse::<i32>().ok()?;
    if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) {
        return None;
    }
    Some((time, sign * (hour * 3_600 + minute * 60)))
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
        assert_eq!(
            days_from_civil(2024, 3, 1) - days_from_civil(2024, 2, 28),
            2
        );
        // Non-leap century.
        assert_eq!(
            days_from_civil(1900, 3, 1) - days_from_civil(1900, 2, 28),
            1
        );
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
            assert_eq!(civil_from_days(days), (year as i32, month, day));
        }
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
    }

    const DIALECTS: [Rfc3339MsOptions; 4] = [
        RFC3339_MS_TRUNCATE,
        RFC3339_MS_LENIENT,
        RFC3339_MS_STRICT,
        RFC3339_MS_OFFSET,
    ];

    fn parse_all(value: &str) -> [Option<i64>; 4] {
        DIALECTS.map(|opts| parse_rfc3339_utc_unix_ms(value, opts))
    }

    #[test]
    fn epoch_parses_to_zero_everywhere() {
        assert_eq!(
            parse_all("1970-01-01T00:00:00Z"),
            [Some(0), Some(0), Some(0), Some(0)]
        );
    }

    #[test]
    fn impossible_dates_rejected_by_every_dialect() {
        for value in [
            "2026-02-30T00:00:00Z",
            "2026-04-31T12:00:00Z",
            "2025-02-29T00:00:00Z",
            "2026-13-01T00:00:00Z",
            "2026-01-00T00:00:00Z",
            "2026-01-01T24:00:00Z",
            "2026-01-01T00:60:00Z",
        ] {
            assert_eq!(parse_all(value), [None, None, None, None], "{value}");
        }
        // Leap day still parses.
        let leap = parse_all("2024-02-29T12:00:00Z");
        assert_eq!(leap, [Some(1_709_208_000_000); 4]);
    }

    #[test]
    fn truncate_dialect_drops_fraction_and_rejects_leap() {
        let base = parse_rfc3339_utc_unix_ms("2026-05-28T12:34:56Z", RFC3339_MS_TRUNCATE);
        assert_eq!(base, Some(1_779_971_696_000));
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:56.789Z", RFC3339_MS_TRUNCATE),
            base
        );
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:60Z", RFC3339_MS_TRUNCATE),
            None
        );
        // No trimming, Zulu only.
        assert_eq!(
            parse_rfc3339_utc_unix_ms(" 2026-05-28T12:34:56Z", RFC3339_MS_TRUNCATE),
            None
        );
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:56+00:00", RFC3339_MS_TRUNCATE),
            None
        );
    }

    #[test]
    fn lenient_dialect_trims_parses_millis_and_clamps_leap() {
        assert_eq!(
            parse_rfc3339_utc_unix_ms("  2026-05-28T12:34:56Z\n", RFC3339_MS_LENIENT),
            Some(1_779_971_696_000)
        );
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:56.789Z", RFC3339_MS_LENIENT),
            Some(1_779_971_696_789)
        );
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:56.7Z", RFC3339_MS_LENIENT),
            Some(1_779_971_696_700)
        );
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:56.7899Z", RFC3339_MS_LENIENT),
            Some(1_779_971_696_789)
        );
        // Non-digit fraction contributes 0 instead of rejecting.
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:56.abcZ", RFC3339_MS_LENIENT),
            Some(1_779_971_696_000)
        );
        // Leap second 60 is accepted and reported as second 59.
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:60.250Z", RFC3339_MS_LENIENT),
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:59.250Z", RFC3339_MS_LENIENT)
        );
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:61Z", RFC3339_MS_LENIENT),
            None
        );
        // Malformed: fraction split off the whole time component first.
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:56.789:00Z", RFC3339_MS_LENIENT),
            Some(1_779_971_696_789)
        );
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:56.789:00Z", RFC3339_MS_STRICT),
            None
        );
    }

    #[test]
    fn strict_dialect_validates_fraction_and_rejects_overflow() {
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:56.789Z", RFC3339_MS_STRICT),
            Some(1_779_971_696_789)
        );
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:56.78Z", RFC3339_MS_STRICT),
            Some(1_779_971_696_780)
        );
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:56.abcZ", RFC3339_MS_STRICT),
            None
        );
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:60Z", RFC3339_MS_STRICT),
            None
        );
        // i32 year boundary preserved; far-future years overflow to None.
        assert_eq!(
            parse_rfc3339_utc_unix_ms("9999999999-01-01T00:00:00Z", RFC3339_MS_STRICT),
            None
        );
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2147483647-01-01T00:00:00Z", RFC3339_MS_STRICT),
            None
        );
        assert!(parse_rfc3339_utc_unix_ms("9999-12-31T23:59:59.999Z", RFC3339_MS_STRICT).is_some());
    }

    #[test]
    fn offset_dialect_applies_numeric_zone_and_wide_years() {
        let base = parse_rfc3339_utc_unix_ms("2026-05-28T12:34:56Z", RFC3339_MS_OFFSET);
        assert_eq!(base, Some(1_779_971_696_000));
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T14:34:56+02:00", RFC3339_MS_OFFSET),
            base
        );
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T07:04:56-05:30", RFC3339_MS_OFFSET),
            base
        );
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:56.789Z", RFC3339_MS_OFFSET),
            Some(1_779_971_696_789)
        );
        // Leap second counts as a full extra second.
        assert_eq!(
            parse_rfc3339_utc_unix_ms("2026-05-28T12:34:60Z", RFC3339_MS_OFFSET),
            parse_rfc3339_utc_unix_ms("2026-05-28T12:35:00Z", RFC3339_MS_OFFSET)
        );
        // i64 years stay accepted for sane magnitudes (capture import's
        // historical behavior); absurd years overflow debug arithmetic, as before.
        assert!(parse_rfc3339_utc_unix_ms("12026-01-01T00:00:00Z", RFC3339_MS_OFFSET).is_some());
        // Trims like the lenient dialect.
        assert_eq!(
            parse_rfc3339_utc_unix_ms(" 2026-05-28T12:34:56Z ", RFC3339_MS_OFFSET),
            base
        );
    }

    #[test]
    fn millis_fraction_parses_three_digits() {
        assert_eq!(parse_millis_fraction(""), Some(0));
        assert_eq!(parse_millis_fraction("789"), Some(789));
        assert_eq!(parse_millis_fraction("7"), Some(700));
        assert_eq!(parse_millis_fraction("78"), Some(780));
        assert_eq!(parse_millis_fraction("7899"), Some(789));
        assert_eq!(parse_millis_fraction("78a"), None);
    }

    #[test]
    fn time_of_day_parses_nanos() {
        assert_eq!(
            parse_time_of_day("12:34:56.000000001"),
            Some((12, 34, 56, 1))
        );
        assert_eq!(parse_time_of_day("12:34:56"), Some((12, 34, 56, 0)));
        assert_eq!(parse_time_of_day("12:34:60"), None);
        assert_eq!(parse_time_of_day("12:34:56."), None);
        assert_eq!(parse_time_of_day("12:34:56.0000000001"), None);
        assert_eq!(parse_time_of_day("24:00:00"), None);
    }

    #[test]
    fn utc_instant_nanos_matches_known_values() {
        assert_eq!(parse_utc_instant_nanos("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            parse_utc_instant_nanos("1970-01-01T00:00:00.000000001Z"),
            Some(1)
        );
        assert_eq!(
            parse_utc_instant_nanos("1970-01-01T02:00:00+02:00"),
            Some(0)
        );
        assert_eq!(
            parse_utc_instant_nanos("1970-01-01T00:00:01.5Z"),
            Some(1_500_000_000)
        );
        // Strict date shape and month-aware day check.
        assert_eq!(parse_utc_instant_nanos("1970-1-1T00:00:00Z"), None);
        assert_eq!(parse_utc_instant_nanos("2026-02-30T00:00:00Z"), None);
        assert_eq!(parse_utc_instant_nanos("2026-01-01T00:00:00"), None);
    }
}
