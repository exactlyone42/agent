use crate::drift::{path_shaped::path_shaped, resolve_link::resolve_link};

const BACKTICK: char = '`';
const LINK_OPEN: &str = "](";
const LINK_CLOSE: char = ')';

pub fn referenced_paths(from: &str, text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let spans = line
            .split(BACKTICK)
            .skip(1)
            .step_by(2)
            .filter_map(path_shaped);
        let links = line
            .split(LINK_OPEN)
            .skip(1)
            .filter_map(|rest| rest.split(LINK_CLOSE).next())
            .filter_map(|target| resolve_link(from, target));
        found.extend(spans.chain(links).map(|path| (index + 1, path)));
    }
    found
}
