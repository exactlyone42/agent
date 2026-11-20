use crate::prelude::*;
use crate::{
    run::step_names::COVERAGE_STEP, run::step_names::MUTATION_STEP,
    run::step_names::VERSIONS_COVERAGE_STEP, run::step_names::VERSIONS_DEPENDENCIES_STEP,
    run::step_names::VERSIONS_MUTATION_STEP, run::step_names::VERSIONS_RUST_STEP,
    run::step_names::VERSIONS_SECRETS_STEP, run::tool_arguments::AGENT_EXCLUDE_PATTERN,
    run::tool_arguments::CARGO_LOCKED_ARGUMENT, run::tool_arguments::EXCLUDE_FLAG,
    run::tool_arguments::IN_PLACE_FLAG, run::tool_arguments::JSON_FLAG,
    run::tool_arguments::LLVM_COV, run::tool_arguments::MUTANTS,
    run::tool_arguments::MUTATION_TIMEOUT_SECONDS, run::tool_arguments::OUTPUT_FLAG,
    run::tool_arguments::OUTPUT_PATH_FLAG, run::tool_arguments::RUSTC,
    run::tool_arguments::TASK_EXCLUDE_PATTERN, run::tool_arguments::TEST_WORKSPACE_FLAG,
    run::tool_arguments::TIMEOUT_FLAG, run::tool_arguments::TRUE_VALUE,
    run::tool_arguments::VERSION_COMMAND, run::tool_arguments::VERSION_FLAG, strings::strings,
};
use std::path::Path;

pub type NamedCommand = (String, Vec<String>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub preflight: Vec<NamedCommand>,
    pub main: Vec<NamedCommand>,
}

impl Plan {
    fn entry(name: &str, parts: &[&str]) -> NamedCommand {
        (name.to_string(), strings(parts))
    }

    pub fn real(executable: &str, evidence: &Path, windows: bool) -> Self {
        let report = text(&evidence.join(COVERAGE_REPORT));
        let mut mutation = strings(&[
            CARGO,
            MUTANTS,
            IN_PLACE_FLAG,
            WORKSPACE_FLAG,
            TEST_WORKSPACE_FLAG,
            TRUE_VALUE,
            ALL_FEATURES_FLAG,
            TIMEOUT_FLAG,
            MUTATION_TIMEOUT_SECONDS,
            CARGO_LOCKED_ARGUMENT,
            EXCLUDE_FLAG,
            TASK_EXCLUDE_PATTERN,
        ]);
        if !windows {
            mutation.extend(strings(&[EXCLUDE_FLAG, AGENT_EXCLUDE_PATTERN]));
        }
        mutation.extend(strings(&[
            OUTPUT_FLAG,
            &text(&evidence.join(MUTATION_OUTPUT_DIRECTORY)),
        ]));
        let mut coverage = strings(&[
            CARGO,
            LLVM_COV,
            WORKSPACE_FLAG,
            ALL_FEATURES_FLAG,
            LOCKED_FLAG,
            JSON_FLAG,
            OUTPUT_PATH_FLAG,
        ]);
        coverage.push(report);
        Self {
            preflight: vec![
                Self::entry(GATE_TASK, &[executable, GATE_TASK]),
                Self::entry(VERSIONS_RUST_STEP, &[RUSTC, VERSION_FLAG]),
                Self::entry(VERSIONS_COVERAGE_STEP, &[CARGO, LLVM_COV, VERSION_FLAG]),
                Self::entry(VERSIONS_MUTATION_STEP, &[CARGO, MUTANTS, VERSION_FLAG]),
                Self::entry(VERSIONS_SECRETS_STEP, &[GITLEAKS, VERSION_COMMAND]),
                Self::entry(VERSIONS_DEPENDENCIES_STEP, &[CARGO, MACHETE, VERSION_FLAG]),
            ],
            main: vec![
                (COVERAGE_STEP.to_string(), coverage),
                (MUTATION_STEP.to_string(), mutation),
            ],
        }
    }
}
