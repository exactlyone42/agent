use crate::case::{Case, LANGUAGE_ENV};

const FALLBACK_VARIABLES: [&str; 3] = ["LC_ALL", "LC_MESSAGES", "LANG"];

pub fn usage_with(variables: &[(&str, &str)]) -> String {
    let case = Case::new();
    let mut command = case.cli(&[]);
    command.env_remove(LANGUAGE_ENV);
    for name in FALLBACK_VARIABLES {
        command.env_remove(name);
    }
    for (name, value) in variables {
        command.env(name, value);
    }
    case.execute(command).stderr
}
