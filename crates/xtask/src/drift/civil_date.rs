use crate::drift::civil_from_days::civil_from_days;

const SECONDS_PER_DAY: u64 = 86_400;
const SECONDS_PER_HOUR: u64 = 3_600;
const SECONDS_PER_MINUTE: u64 = 60;

pub fn civil_date(seconds: u64) -> String {
    let days = i64::try_from(seconds / SECONDS_PER_DAY).unwrap_or(0);
    let (year, month, day) = civil_from_days(days);
    let within_day = seconds % SECONDS_PER_DAY;
    let hour = within_day / SECONDS_PER_HOUR;
    let minute = within_day % SECONDS_PER_HOUR / SECONDS_PER_MINUTE;
    let second = within_day % SECONDS_PER_MINUTE;
    format!("{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}")
}
