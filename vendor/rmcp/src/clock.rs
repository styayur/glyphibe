// SPDX-License-Identifier: Apache-2.0
//! Glyphibe maintenance backport: internal clocks without a mandatory chrono runtime.
//! Typed chrono convenience APIs remain available through the default `chrono` feature.

use std::time::Duration;
use time::OffsetDateTime;

pub(crate) fn current_timestamp() -> String {
    timestamp(OffsetDateTime::now_utc())
}

fn timestamp(value: OffsetDateTime) -> String {
    let value = value.to_offset(time::UtcOffset::UTC);
    let year = value.year();
    let year = if year < 0 {
        format!("-{:04}", year.unsigned_abs())
    } else {
        format!("{year:04}")
    };
    let mut result = format!(
        "{year}-{:02}-{:02}T{:02}:{:02}:{:02}",
        value.month() as u8,
        value.day(),
        value.hour(),
        value.minute(),
        value.second()
    );
    // Match chrono's AutoSi representation, including grouped 3/6/9 digits
    // and the explicit UTC offset, rather than silently changing wire strings.
    let nanos = value.nanosecond();
    if nanos != 0 {
        let fraction = if nanos % 1_000_000 == 0 {
            format!(".{:03}", nanos / 1_000_000)
        } else if nanos % 1_000 == 0 {
            format!(".{:06}", nanos / 1_000)
        } else {
            format!(".{nanos:09}")
        };
        result.push_str(&fraction);
    }
    result.push_str("+00:00");
    result
}

#[cfg(any(feature = "request-state", test))]
pub(crate) fn current_timestamp_millis() -> i64 {
    timestamp_millis(OffsetDateTime::now_utc())
}

#[cfg(any(feature = "request-state", test))]
fn timestamp_millis(value: OffsetDateTime) -> i64 {
    // Euclidean division preserves chrono's floor behavior before the epoch.
    // time's supported calendar range is safely inside i64 milliseconds.
    value.unix_timestamp_nanos().div_euclid(1_000_000) as i64
}

#[cfg(any(feature = "client", feature = "server"))]
pub(crate) struct TimeoutDisplay(pub(crate) Duration);

#[cfg(any(feature = "client", feature = "server"))]
impl std::fmt::Display for TimeoutDisplay {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Preserve the previous from_std(...).unwrap_or_default() overflow
        // behavior as well as its ISO-8601 display, without a chrono dependency.
        let duration = if self.0 > Duration::from_millis(i64::MAX as u64) {
            Duration::ZERO
        } else {
            self.0
        };
        if duration.is_zero() {
            return formatter.write_str("P0D");
        }
        write!(formatter, "PT{}", duration.as_secs())?;
        if duration.subsec_nanos() != 0 {
            let digits = format!("{:09}", duration.subsec_nanos());
            write!(formatter, ".{}", digits.trim_end_matches('0'))?;
        }
        formatter.write_str("S")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_wire_strings_preserve_fractional_groups_and_epoch_boundaries() {
        for (nanos, expected, milliseconds) in [
            (0, "1970-01-01T00:00:00+00:00", 0),
            (1, "1970-01-01T00:00:00.000000001+00:00", 0),
            (1_000, "1970-01-01T00:00:00.000001+00:00", 0),
            (1_000_000, "1970-01-01T00:00:00.001+00:00", 1),
            (120_000_000, "1970-01-01T00:00:00.120+00:00", 120),
            (-1, "1969-12-31T23:59:59.999999999+00:00", -1),
        ] {
            let value = OffsetDateTime::from_unix_timestamp_nanos(nanos).expect("valid fixture");
            assert_eq!(timestamp(value), expected);
            assert_eq!(timestamp_millis(value), milliseconds);
        }
    }

    #[test]
    #[cfg(any(feature = "client", feature = "server"))]
    fn timeout_display_preserves_iso8601_and_overflow_behavior() {
        for (duration, expected) in [
            (Duration::ZERO, "P0D"),
            (Duration::from_nanos(1), "PT0.000000001S"),
            (Duration::from_nanos(120), "PT0.00000012S"),
            (Duration::new(42, 120_000_000), "PT42.12S"),
            (Duration::from_secs(3_600), "PT3600S"),
            (
                Duration::from_millis(i64::MAX as u64),
                "PT9223372036854775.807S",
            ),
            (
                Duration::from_millis(i64::MAX as u64) + Duration::from_nanos(1),
                "P0D",
            ),
            (Duration::from_secs(u64::MAX), "P0D"),
        ] {
            assert_eq!(TimeoutDisplay(duration).to_string(), expected);
        }
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn compatibility_feature_matches_original_chrono_formatter() {
        use chrono::{DateTime, Utc};
        for seconds in [
            -2_208_988_800,
            -1,
            0,
            951_782_400,
            1_700_000_000,
            253_402_300_799,
        ] {
            for nanos in [
                0,
                1,
                120,
                1_000,
                120_000,
                1_000_000,
                120_000_000,
                999_999_999,
            ] {
                let original =
                    DateTime::<Utc>::from_timestamp(seconds, nanos).expect("valid fixture");
                let maintained = OffsetDateTime::from_unix_timestamp_nanos(
                    i128::from(seconds) * 1_000_000_000 + i128::from(nanos),
                )
                .expect("valid fixture");
                assert_eq!(timestamp(maintained), original.to_rfc3339());
                assert_eq!(timestamp_millis(maintained), original.timestamp_millis());
            }
        }
        for duration in [
            Duration::ZERO,
            Duration::from_nanos(1),
            Duration::new(42, 120),
            Duration::from_millis(i64::MAX as u64),
            Duration::from_secs(u64::MAX),
        ] {
            assert_eq!(
                TimeoutDisplay(duration).to_string(),
                chrono::Duration::from_std(duration)
                    .unwrap_or_default()
                    .to_string()
            );
        }
    }
}
