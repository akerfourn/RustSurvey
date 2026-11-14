use super::AppError;

/// Format native Unix seconds as UTC; imported wall-clock values stay separate.
pub(super) fn utc_datetime(seconds: i64) -> Result<String, AppError> {
    if !(0..=253_402_300_799).contains(&seconds) {
        return Err(AppError::internal());
    }
    let mut days = seconds / 86_400;
    let mut year = 1970;
    loop {
        let year_days = if leap(year) { 366 } else { 365 };
        if days < year_days {
            break;
        }
        days -= year_days;
        year += 1;
    }
    let mut lengths = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if leap(year) {
        lengths[1] = 29;
    }
    let mut month = 0;
    while days >= lengths[month] {
        days -= lengths[month];
        month += 1;
    }
    let clock = seconds % 86_400;
    Ok(format!(
        "{year:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        month + 1,
        days + 1,
        clock / 3600,
        clock / 60 % 60,
        clock % 60
    ))
}

fn leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utc_datetime_should_preserve_calendar_boundaries_without_host_timezone() {
        for (seconds, expected) in [
            (0, "1970-01-01 00:00:00"),
            (951_782_400, "2000-02-29 00:00:00"),
            (253_402_300_799, "9999-12-31 23:59:59"),
        ] {
            assert_eq!(utc_datetime(seconds).unwrap(), expected);
        }
        assert!(utc_datetime(-1).is_err());
        assert!(utc_datetime(253_402_300_800).is_err());
    }
}
