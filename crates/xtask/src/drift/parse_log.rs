use std::collections::BTreeMap;

const TIME_MARK: char = '@';

pub fn parse_log(text: &str) -> BTreeMap<String, u64> {
    let mut times = BTreeMap::new();
    let mut current = 0;
    for line in text.lines().filter(|line| !line.is_empty()) {
        match line.strip_prefix(TIME_MARK) {
            Some(seconds) => current = seconds.parse().unwrap_or(0),
            None => {
                times.entry(line.to_string()).or_insert(current);
            }
        }
    }
    times
}
