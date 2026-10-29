#[derive(Clone, Debug)]
pub struct Messages {
    pub cancelled: String,
    pub call_limit: String,
    pub intent_failed: String,
    pub claim_failed: String,
    pub rejection_unrecorded: String,
    pub invalid_call: String,
    pub not_authorized: String,
    pub approval_timeout: String,
    pub approval_denied: String,
    pub receipt_failed: String,
    pub receipt_cancelled: String,
}

impl Default for Messages {
    fn default() -> Self {
        Self {
            cancelled: "the run was asked to stop".into(),
            call_limit: "the tool call limit was reached".into(),
            intent_failed: "the tool call could not be recorded, so it was not run".into(),
            claim_failed: "the approval could not be claimed, so the tool was not run".into(),
            rejection_unrecorded: "the rejection could not be recorded, so the run stopped".into(),
            invalid_call: "the tool name or arguments are not allowed".into(),
            not_authorized: "the tool call was not authorized".into(),
            approval_timeout: "waiting for approval timed out, so the tool was not run".into(),
            approval_denied: "the tool call was not approved or the approval is no longer valid"
                .into(),
            receipt_failed: "the tool result could not be recorded; check what was done".into(),
            receipt_cancelled: "the run was asked to stop before the tool result was recorded"
                .into(),
        }
    }
}
