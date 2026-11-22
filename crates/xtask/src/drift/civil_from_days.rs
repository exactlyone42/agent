const DAYS_BEFORE_EPOCH_ERA: i64 = 719_468;
const DAYS_PER_ERA: i64 = 146_097;
const YEARS_PER_ERA: i64 = 400;
const FIRST_SHIFTED_MONTH_OF_NEXT_YEAR: i64 = 10;

pub(crate) fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + DAYS_BEFORE_EPOCH_ERA;
    let era = shifted.div_euclid(DAYS_PER_ERA);
    let day_of_era = shifted.rem_euclid(DAYS_PER_ERA);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < FIRST_SHIFTED_MONTH_OF_NEXT_YEAR {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * YEARS_PER_ERA + i64::from(month <= 2);
    (year, month, day)
}
