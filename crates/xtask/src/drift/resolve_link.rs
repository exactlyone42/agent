const ANCHOR: char = '#';
const URL_MARK: &str = "://";
const SEPARATOR: char = '/';
const PARENT: &str = "..";
const CURRENT: &str = ".";

pub(crate) fn resolve_link(from: &str, target: &str) -> Option<String> {
    let first = target.split_whitespace().next()?;
    let without_anchor = first.split(ANCHOR).next()?;
    let outside = without_anchor.contains(URL_MARK) || without_anchor.starts_with(SEPARATOR);
    if without_anchor.is_empty() || outside {
        return None;
    }
    let mut parts: Vec<&str> = from.split(SEPARATOR).collect();
    parts.pop();
    for part in without_anchor.split(SEPARATOR) {
        match part {
            CURRENT | "" => {}
            PARENT => {
                parts.pop()?;
            }
            name => parts.push(name),
        }
    }
    (!parts.is_empty()).then(|| parts.join(&SEPARATOR.to_string()))
}
