use crate::drift::{drift_text, file_result::FileResult, verdict_name::verdict_name};

pub(crate) fn render_result(result: &FileResult) -> String {
    let mut block = format!("### {}\n\n", result.path);
    block.push_str(&format!(
        "- {} {}\n- {} {}\n",
        drift_text::VERDICT_LABEL,
        verdict_name(result.finding.verdict),
        drift_text::REASON_LABEL,
        result.finding.reason,
    ));
    if let Some(opinion) = &result.opinion {
        block.push_str(&format!(
            "- {} {}\n",
            drift_text::SECOND_LABEL,
            opinion.reason
        ));
    }
    block.push_str(&format!("- {}\n", drift_text::EVIDENCE_LABEL));
    for proof in &result.finding.evidence {
        block.push_str(&format!(
            "  - `{}:{}` {}\n",
            proof.path, proof.line, proof.quote
        ));
    }
    block.push('\n');
    block
}
