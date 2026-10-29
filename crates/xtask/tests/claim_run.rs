use agent_platform_xtask::{XtaskError, claim_run};

#[test]
fn interrupted_run_cannot_become_a_new_baseline() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("active.json");
    claim_run(&marker, || {
        assert!(marker.exists());
        Ok(())
    })
    .unwrap();
    assert!(!marker.exists());
    let interrupted: Result<(), XtaskError> = claim_run(&marker, || Err(XtaskError::EmptySnapshot));
    assert!(interrupted.is_err());
    assert!(marker.exists());
    let mut ran = false;
    let second = claim_run(&marker, || {
        ran = true;
        Ok(())
    });
    assert!(matches!(second, Err(XtaskError::Io(_))));
    assert!(!ran);
}
