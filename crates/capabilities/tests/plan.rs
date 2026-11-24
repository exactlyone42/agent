use agent_harness_capabilities::{Plan, PlanBoard, PlanError, Step};

fn step(id: &str) -> Step {
    Step::new(id, "goal", "ok")
}

fn two_steps() -> Vec<Step> {
    vec![step("a"), step("b")]
}

#[test]
fn empty_plan_is_refused() {
    assert_eq!(Plan::new(Vec::new()).unwrap_err(), PlanError::Empty);
}

#[test]
fn duplicate_id_is_refused() {
    let result = Plan::new(vec![step("a"), step("a")]);
    assert_eq!(result.unwrap_err(), PlanError::DuplicateId("a".into()));
}

#[test]
fn blank_field_is_refused() {
    let result = Plan::new(vec![Step::new("a", " ", "ok")]);
    assert_eq!(result.unwrap_err(), PlanError::BlankField("goal"));
}

#[test]
fn too_many_steps_is_refused() {
    let steps: Vec<Step> = (0..21).map(|n| step(&format!("s{n}"))).collect();
    assert!(matches!(
        Plan::new(steps).unwrap_err(),
        PlanError::TooManySteps(_)
    ));
}

#[test]
fn steps_must_finish_in_order() {
    let mut plan = Plan::new(two_steps()).unwrap();
    assert!(matches!(
        plan.complete("b").unwrap_err(),
        PlanError::NotCurrent { .. }
    ));
    plan.complete("a").unwrap();
    assert_eq!(plan.current().unwrap().id, "b");
    plan.complete("b").unwrap();
    assert_eq!(plan.unfinished(), 0);
    assert_eq!(plan.complete("b").unwrap_err(), PlanError::NothingToDo);
}

#[test]
fn tool_outside_current_step_is_refused() {
    let steps = vec![
        Step::new("a", "goal", "ok").tools(&["read"]),
        Step::new("b", "goal", "ok").tools(&["write"]),
    ];
    let plan = Plan::new(steps).unwrap();
    assert!(plan.allows("read").is_ok());
    assert!(matches!(
        plan.allows("write").unwrap_err(),
        PlanError::ToolNotInStep { .. }
    ));
}

#[test]
fn step_without_tool_list_allows_every_tool() {
    let plan = Plan::new(vec![step("a")]).unwrap();
    assert!(plan.allows("anything").is_ok());
}

#[test]
fn revision_keeps_done_steps_and_is_limited() {
    let mut plan = Plan::new(two_steps()).unwrap();
    plan.complete("a").unwrap();
    plan.revise(vec![step("c")], 2).unwrap();
    let ids: Vec<&str> = plan.steps().iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, ["a", "c"]);
    assert!(plan.steps()[0].done);
    plan.revise(vec![step("d")], 2).unwrap();
    assert_eq!(
        plan.revise(vec![step("e")], 2).unwrap_err(),
        PlanError::RevisionLimit(2)
    );
}

#[test]
fn revision_cannot_reuse_a_done_id() {
    let mut plan = Plan::new(two_steps()).unwrap();
    plan.complete("a").unwrap();
    assert_eq!(
        plan.revise(vec![step("a")], 2).unwrap_err(),
        PlanError::DuplicateId("a".into())
    );
}

#[test]
fn board_is_shared_between_clones() {
    let board = PlanBoard::new();
    let other = board.clone();
    board.set(two_steps()).unwrap();
    assert_eq!(other.unfinished(), 2);
    other.complete("a").unwrap();
    assert_eq!(board.current().unwrap().id, "b");
}

#[test]
fn board_without_plan_allows_everything() {
    let board = PlanBoard::new();
    assert_eq!(board.unfinished(), 0);
    assert!(board.allows("any").is_ok());
    assert_eq!(board.complete("a").unwrap_err(), PlanError::NothingToDo);
}

#[test]
fn exactly_twenty_steps_are_accepted() {
    let steps: Vec<Step> = (0..20).map(|n| step(&format!("s{n}"))).collect();
    assert!(Plan::new(steps).is_ok());
}

#[test]
fn revision_counts_kept_steps_toward_the_limit() {
    let mut plan = Plan::new(vec![step("a")]).unwrap();
    plan.complete("a").unwrap();
    let too_many: Vec<Step> = (0..20).map(|n| step(&format!("v{n}"))).collect();
    assert!(matches!(
        plan.revise(too_many, 2).unwrap_err(),
        PlanError::TooManySteps(_)
    ));
    let fits: Vec<Step> = (0..19).map(|n| step(&format!("w{n}"))).collect();
    assert!(plan.revise(fits, 2).is_ok());
}

#[test]
fn board_revision_limit_is_configurable() {
    let none = PlanBoard::new().max_revisions(0);
    none.set(vec![step("a")]).unwrap();
    assert_eq!(
        none.set(vec![step("b")]).unwrap_err(),
        PlanError::RevisionLimit(0)
    );
    let one = PlanBoard::new().max_revisions(1);
    one.set(vec![step("a")]).unwrap();
    one.set(vec![step("b")]).unwrap();
    assert_eq!(
        one.set(vec![step("c")]).unwrap_err(),
        PlanError::RevisionLimit(1)
    );
}

#[test]
fn describe_marks_done_current_and_todo() {
    let board = PlanBoard::new();
    assert_eq!(board.describe(), "there is no plan yet");
    board.set(vec![step("a"), step("b"), step("c")]).unwrap();
    board.complete("a").unwrap();
    assert_eq!(
        board.describe(),
        "[done] a: goal (check: ok)\n[current] b: goal (check: ok)\n[todo] c: goal (check: ok)"
    );
}

#[test]
fn planner_has_a_fixed_id() {
    use agent_harness::Capability;
    use agent_harness_capabilities::Planner;
    assert_eq!(Planner::new(PlanBoard::new()).id(), "planner");
}

#[test]
fn checks_list_registered_names_in_order() {
    use agent_harness_capabilities::{CheckFuture, Checks, StepCheck};
    struct Pass;
    impl StepCheck for Pass {
        fn run(&self) -> CheckFuture<'_> {
            Box::pin(async { Ok(()) })
        }
    }
    let checks = Checks::new().with("b", Pass).with("a", Pass);
    assert_eq!(checks.names(), ["a", "b"]);
    assert!(checks.find("a").is_ok());
    assert!(matches!(
        checks.find("z").err().unwrap(),
        PlanError::UnknownCheck { available, .. } if available == "a, b"
    ));
}

#[test]
fn plan_set_schema_lists_registered_checks() {
    use agent_harness::Tool;
    use agent_harness_capabilities::{CheckFuture, Checks, PlanSetTool, StepCheck};
    use serde_json::json;
    struct Pass;
    impl StepCheck for Pass {
        fn run(&self) -> CheckFuture<'_> {
            Box::pin(async { Ok(()) })
        }
    }
    let checks = Checks::new().with("b", Pass).with("a", Pass);
    let spec = PlanSetTool::new(PlanBoard::new(), checks).spec();
    assert_eq!(spec.name, "plan_set");
    assert_eq!(
        spec.parameters,
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "steps": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "additionalProperties": false,
                        "properties": {
                            "id": {"type": "string"},
                            "goal": {"type": "string"},
                            "check": {"type": "string", "enum": ["a", "b"]},
                            "tools": {"type": "array", "items": {"type": "string"}, "default": []}
                        },
                        "required": ["id", "goal", "check"]
                    }
                }
            },
            "required": ["steps"]
        })
    );
}

#[test]
fn plan_done_schema_requires_the_step() {
    use agent_harness::Tool;
    use agent_harness_capabilities::{Checks, PlanDoneTool};
    use serde_json::json;
    let spec = PlanDoneTool::new(PlanBoard::new(), Checks::new()).spec();
    assert_eq!(
        spec.parameters,
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {"step": {"type": "string"}},
            "required": ["step"]
        })
    );
}
