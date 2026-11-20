pub(crate) const CARGO: &str = "cargo";
pub(crate) const GIT: &str = "git";
pub(crate) const GITLEAKS: &str = "gitleaks";
pub(crate) const RUSTC: &str = "rustc";
pub(crate) const CARGO_PKG_NAME_VARIABLE: &str = "CARGO_PKG_NAME";
pub(crate) const MACHETE: &str = "machete";
pub(crate) const VERSION_FLAG: &str = "--version";
pub(crate) const WORKSPACE_FLAG: &str = "--workspace";
pub(crate) const ALL_FEATURES_FLAG: &str = "--all-features";
pub(crate) const LOCKED_FLAG: &str = "--locked";
pub(crate) const POLICY_DIRECTORY: &str = "scripts/policy";
pub(crate) const CLIPPY: &str = "clippy";
pub(crate) const CLIPPY_LINT_PREFIX: &str = "clippy::";
pub(crate) const CLIPPY_CONFIG_VARIABLE: &str = "CLIPPY_CONF_DIR";
pub(crate) const DENY_FLAG: &str = "-D";
pub(crate) const WARNINGS: &str = "warnings";
pub(crate) const TOOL_ARGUMENTS: &str = "--";
pub(crate) const LLVM_COV: &str = "llvm-cov";
pub(crate) const MUTANTS: &str = "mutants";
pub(crate) const EXCLUDE_FLAG: &str = "--exclude-re";
pub(crate) const MUTATION_TIMEOUT_SECONDS: &str = "90";
pub(crate) const TASK_EXCLUDE_PATTERN: &str =
    r"crates/xtask/src/run/task\.rs:\d+:\d+: replace Task::(run|all) -> ";
pub(crate) const AGENT_EXCLUDE_PATTERN: &str =
    r"crates/agent/src/file_check/(input|single_link)/windows\.rs:";
pub(crate) const LIST_FILES: &str = "ls-files";
pub(crate) const NUL_SEPARATED_FLAG: &str = "-z";
pub(crate) const NUL: char = '\0';
pub(crate) const IN_PLACE_FLAG: &str = "--in-place";
pub(crate) const TEST_WORKSPACE_FLAG: &str = "--test-workspace";
pub(crate) const TRUE_VALUE: &str = "true";
pub(crate) const TIMEOUT_FLAG: &str = "--timeout";
pub(crate) const CARGO_LOCKED_ARGUMENT: &str = "--cargo-arg=--locked";
pub(crate) const OUTPUT_FLAG: &str = "--output";
pub(crate) const JSON_FLAG: &str = "--json";
pub(crate) const OUTPUT_PATH_FLAG: &str = "--output-path";
pub(crate) const VERSION_COMMAND: &str = "version";
pub(crate) const LIB_FLAG: &str = "--lib";
pub(crate) const BINS_FLAG: &str = "--bins";
pub(crate) const ALL_TARGETS_FLAG: &str = "--all-targets";
pub(crate) const FMT_COMMAND: &str = "fmt";
pub(crate) const ALL_FLAG: &str = "--all";
pub(crate) const CHECK_FLAG: &str = "--check";
pub(crate) const TEST_COMMAND: &str = "test";
pub(crate) const CACHED_FLAG: &str = "--cached";
pub(crate) const OTHERS_FLAG: &str = "--others";
pub(crate) const EXCLUDE_STANDARD_FLAG: &str = "--exclude-standard";

pub const LINTS: [&str; 10] = [
    "unwrap_used",
    "expect_used",
    "panic",
    "todo",
    "unimplemented",
    "unreachable",
    "disallowed_macros",
    "mod_module_files",
    "excessive_nesting",
    "redundant_else",
];
