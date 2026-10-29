use crate::approval_scenario::ApprovalScenario;
use crate::run_each::run_each;

#[test]
fn manual_approval_controls_real_tool_execution() {
    let mut actions = vec![
        "approve",
        "deny",
        "wrong_revision",
        "expired",
        "claim_failure",
        "busy",
        "receipt_failure",
    ];
    if cfg!(target_os = "linux") {
        actions.extend([
            "receipt_busy",
            "receipt_busy_timeout",
            "receipt_busy_cancel",
        ]);
    }
    actions.push("cancel");
    run_each(actions, |action| {
        let mut scenario = ApprovalScenario::start(action);
        scenario.wait_pending();
        scenario.act();
        scenario.finish();
    });
}
