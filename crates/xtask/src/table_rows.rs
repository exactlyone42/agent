fn is_separator(line: &str) -> bool {
    line.trim_start_matches('|')
        .trim_start()
        .starts_with(['-', ':'])
}

pub fn table_rows(section: &str) -> Vec<Vec<String>> {
    section
        .lines()
        .filter(|line| line.starts_with('|') && !is_separator(line))
        .map(|line| {
            line.trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect()
}
