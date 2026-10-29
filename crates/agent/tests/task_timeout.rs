use agent_platform_agent::{
    store::ApprovalMode,
    task::{MANUAL_TASK_TIMEOUT, TASK_TIMEOUT},
    task_timeout::task_timeout,
};

#[test]
fn manual_mode_waits_longer_than_preapproved() {
    assert_eq!(task_timeout(ApprovalMode::Manual), MANUAL_TASK_TIMEOUT);
    assert_eq!(task_timeout(ApprovalMode::Preapproved), TASK_TIMEOUT);
}
