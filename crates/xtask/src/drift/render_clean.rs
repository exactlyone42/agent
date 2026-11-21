use crate::drift::{drift_text, file_result::FileResult, ruling::Ruling};

pub(crate) fn render_clean(results: &[FileResult]) -> String {
    let mut section = format!("{}\n\n", drift_text::CLEAN_TITLE);
    let clean: Vec<&FileResult> = results
        .iter()
        .filter(|result| result.ruling == Ruling::NoIssue)
        .collect();
    if clean.is_empty() {
        section.push_str(drift_text::NONE_LABEL);
        section.push_str("\n\n");
        return section;
    }
    for result in clean {
        section.push_str(&format!("- `{}`\n", result.path));
    }
    section.push('\n');
    section
}
