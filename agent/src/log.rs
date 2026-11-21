//! Timestamped log lines. launchd writes stdout/stderr to files verbatim, so
//! every daemon message carries its own UTC time.

use std::time::{SystemTime, UNIX_EPOCH};

/// `secs` since the Unix epoch as `YYYY-MM-DDTHH:MM:SSZ`.
pub fn format_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        rem / 60 % 60,
        rem % 60
    )
}

pub fn now() -> String {
    format_utc(SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs()))
}

/// `println!` prefixed with the current UTC time.
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => { println!("{} {}", $crate::log::now(), format_args!($($arg)*)) };
}

/// `eprintln!` prefixed with the current UTC time.
#[macro_export]
macro_rules! elog {
    ($($arg:tt)*) => { eprintln!("{} {}", $crate::log::now(), format_args!($($arg)*)) };
}

#[cfg(test)]
mod tests {
    use super::format_utc;

    #[test]
    fn formats_utc() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(format_utc(1_795_274_396), "2026-11-21T15:19:56Z");
    }
}
