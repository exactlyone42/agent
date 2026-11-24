use crate::prelude::*;
use crate::{
    run::tool_arguments::ALL_FLAG, run::tool_arguments::ALL_TARGETS_FLAG,
    run::tool_arguments::BINS_FLAG, run::tool_arguments::CHECK_FLAG, run::tool_arguments::CLIPPY,
    run::tool_arguments::CLIPPY_CONFIG_VARIABLE, run::tool_arguments::CLIPPY_LINT_PREFIX,
    run::tool_arguments::DENY_FLAG, run::tool_arguments::FMT_COMMAND,
    run::tool_arguments::LIB_FLAG, run::tool_arguments::LINTS, run::tool_arguments::TEST_COMMAND,
    run::tool_arguments::TOOL_ARGUMENTS, run::tool_arguments::WARNINGS,
};

pub fn gate_steps(root: &Path) -> Vec<Step> {
    let lint_flags: Vec<String> = LINTS
        .iter()
        .flat_map(|lint| [DENY_FLAG.to_string(), format!("{CLIPPY_LINT_PREFIX}{lint}")])
        .collect();
    let mut strict = vec![
        CLIPPY,
        WORKSPACE_FLAG,
        LIB_FLAG,
        BINS_FLAG,
        ALL_FEATURES_FLAG,
        LOCKED_FLAG,
        TOOL_ARGUMENTS,
        DENY_FLAG,
        WARNINGS,
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<_>>();
    strict.extend(lint_flags);
    let strict_refs: Vec<&str> = strict.iter().map(String::as_str).collect();
    vec![
        Step::new(CARGO, &[FMT_COMMAND, ALL_FLAG, CHECK_FLAG]),
        Step::new(CARGO, &[MACHETE]),
        Step::new(
            CARGO,
            &[
                CLIPPY,
                WORKSPACE_FLAG,
                ALL_TARGETS_FLAG,
                ALL_FEATURES_FLAG,
                LOCKED_FLAG,
                TOOL_ARGUMENTS,
                DENY_FLAG,
                WARNINGS,
            ],
        ),
        Step::new(CARGO, &strict_refs)
            .with_environment(CLIPPY_CONFIG_VARIABLE, &root.join(POLICY_DIRECTORY)),
        Step::new(
            CARGO,
            &[TEST_COMMAND, WORKSPACE_FLAG, ALL_FEATURES_FLAG, LOCKED_FLAG],
        ),
    ]
}
