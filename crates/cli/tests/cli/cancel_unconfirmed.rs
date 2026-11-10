use crate::case::Case;
use agent_platform_agent::store::TaskStore;
use std::time::{Duration, Instant};

#[test]
fn missing_executor_keeps_request_pending_and_never_claims_confirmed_stop() {
    let case = Case::limited(Duration::from_secs(12));
    std::fs::create_dir(case.state()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(case.state(), std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let mut store = TaskStore::open(&case.database()).unwrap();
    let task = store.start().unwrap().0;
    let began = Instant::now();
    let outcome = case.run(&["cancel", &task]);
    assert_eq!(outcome.code(), Some(1), "{}", outcome.stderr);
    assert!(outcome.stdout.is_empty());
    let error: serde_json::Value =
        serde_json::from_str(outcome.stderr.lines().last().unwrap()).unwrap();
    assert_eq!(error["error_code"], "cancel_unconfirmed");
    assert!(began.elapsed() >= Duration::from_secs(5));
    assert!(began.elapsed() < Duration::from_secs(7));
    assert_eq!(
        store.status(&task).unwrap().recorded_state,
        "cancel_requested"
    );
    assert!(!case.root().join("output").exists());
    let events = store.events(&task, 0).unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].event_type, "cancel_requested");
}
