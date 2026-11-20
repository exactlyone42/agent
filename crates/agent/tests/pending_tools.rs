use agent_platform_agent::pending_tools::PendingTools;

fn expecting(ids: &[&str]) -> PendingTools {
    let mut pending = PendingTools::default();
    for id in ids {
        assert!(pending.expect(id));
    }
    pending
}

#[test]
fn it_counts_each_expected_call_once() {
    let pending = expecting(&["a", "b", "c"]);
    assert_eq!(pending.len(), 3);
    assert!(!pending.is_empty());
}

#[test]
fn a_repeated_expectation_is_refused_and_does_not_change_the_count() {
    let mut pending = expecting(&["a"]);
    assert!(!pending.expect("a"));
    assert_eq!(pending.len(), 1);
}

#[test]
fn reports_in_any_order_clear_the_set() {
    let mut pending = expecting(&["a", "b", "c"]);
    assert!(pending.report("c"));
    assert!(pending.report("a"));
    assert_eq!(pending.len(), 1);
    assert!(pending.report("b"));
    assert!(pending.is_empty());
}

#[test]
fn a_repeated_report_is_refused_and_cannot_clear_another_call() {
    let mut pending = expecting(&["a", "b"]);
    assert!(pending.report("a"));
    assert!(!pending.report("a"));
    assert!(!pending.report("a"));
    assert_eq!(pending.len(), 1);
}

#[test]
fn a_report_for_an_unknown_call_is_refused() {
    let mut pending = expecting(&["a"]);
    assert!(!pending.report("zzz"));
    assert_eq!(pending.len(), 1);
}
