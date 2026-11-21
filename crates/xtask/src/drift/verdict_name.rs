use crate::drift::{drift_text, verdict::Verdict};

pub(crate) fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Ok => drift_text::OK_NAME,
        Verdict::Update => drift_text::UPDATE_NAME,
        Verdict::Delete => drift_text::DELETE_NAME,
    }
}
