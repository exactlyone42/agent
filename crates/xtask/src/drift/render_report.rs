use crate::drift::{
    candidate::Candidate, drift_text, render_clean::render_clean, render_diagrams::render_diagrams,
    render_section::render_section, render_summary::render_summary, review_batch::BatchOutcome,
    ruling::Ruling, selection::Selection,
};

pub fn render_report(
    candidates: &[Candidate],
    selection: &Selection,
    batch: &BatchOutcome,
    now: u64,
) -> String {
    let mut report = render_summary(candidates, selection, now);
    if let Some(failure) = &batch.failure {
        report.push_str(&format!(
            "{}\n\n{}{}\n\n",
            drift_text::FAILURE_TITLE,
            drift_text::FAILURE_NOTE,
            failure
        ));
    }
    report.push_str(&render_section(
        drift_text::CONFIRMED_TITLE,
        Ruling::Confirmed,
        &batch.results,
    ));
    report.push_str(&render_section(
        drift_text::DISPUTED_TITLE,
        Ruling::Disputed,
        &batch.results,
    ));
    report.push_str(&render_section(
        drift_text::REJECTED_TITLE,
        Ruling::Rejected,
        &batch.results,
    ));
    report.push_str(&render_clean(&batch.results));
    report.push_str(&render_diagrams(candidates));
    report.push_str(drift_text::FOOTER);
    report.push('\n');
    report
}
