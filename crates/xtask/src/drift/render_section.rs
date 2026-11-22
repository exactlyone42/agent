use crate::drift::{
    drift_text, file_result::FileResult, render_result::render_result, ruling::Ruling,
};

pub(crate) fn render_section(title: &str, ruling: Ruling, results: &[FileResult]) -> String {
    let mut section = format!("{title}\n\n");
    let matching: Vec<&FileResult> = results.iter().filter(|r| r.ruling == ruling).collect();
    if matching.is_empty() {
        section.push_str(drift_text::NONE_LABEL);
        section.push_str("\n\n");
        return section;
    }
    for result in matching {
        section.push_str(&render_result(result));
    }
    section
}
