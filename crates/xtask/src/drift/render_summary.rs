use crate::drift::{
    candidate::Candidate, civil_date::civil_date, drift_text, selection::Selection,
};

pub(crate) fn render_summary(candidates: &[Candidate], selection: &Selection, now: u64) -> String {
    format!(
        "{title}\n\n{time}{date}\n\n{summary}\n\n{candidates_label}{candidates}\n{chosen_label}{chosen}\n{fresh_label}{fresh}\n{deferred_label}{deferred}\n\n",
        title = drift_text::TITLE,
        time = drift_text::TIME_LABEL,
        date = civil_date(now),
        summary = drift_text::SUMMARY_TITLE,
        candidates_label = drift_text::CANDIDATES_LABEL,
        candidates = candidates.len(),
        chosen_label = drift_text::CHOSEN_LABEL,
        chosen = selection.chosen.len(),
        fresh_label = drift_text::FRESH_LABEL,
        fresh = selection.fresh,
        deferred_label = drift_text::DEFERRED_LABEL,
        deferred = selection.deferred,
    )
}
