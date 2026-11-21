use crate::drift::{candidate::Candidate, candidate_kind::CandidateKind, drift_text};

pub(crate) fn render_diagrams(candidates: &[Candidate]) -> String {
    let mut section = format!(
        "{}\n\n{}\n\n",
        drift_text::DIAGRAM_TITLE,
        drift_text::DIAGRAM_NOTE
    );
    let diagrams: Vec<&Candidate> = candidates
        .iter()
        .filter(|candidate| candidate.kind == CandidateKind::DiagramBehindCode)
        .collect();
    if diagrams.is_empty() {
        section.push_str(drift_text::NONE_LABEL);
        section.push_str("\n\n");
        return section;
    }
    for candidate in diagrams {
        section.push_str(&format!("- `{}`: {}\n", candidate.path, candidate.subject));
    }
    section.push('\n');
    section
}
