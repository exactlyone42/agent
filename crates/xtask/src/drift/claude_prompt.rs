use crate::drift::file_group::FileGroup;

pub fn claude_prompt(group: &FileGroup) -> String {
    let signals: Vec<String> = group
        .candidates
        .iter()
        .map(|candidate| {
            format!(
                "- line {}: {}: {}",
                candidate.line,
                candidate.kind.describe(),
                candidate.subject
            )
        })
        .collect();
    format!(
        "You are checking one file of the repository in the current directory for being out of date.\n\
         File: {path}\n\
         A program found these signals. They may be false alarms, for example a path that is only planned, or a file that is a dated record:\n\
         {signals}\n\
         Read the real files with Read, Grep and Glob. Then give one verdict:\n\
         - ok: nothing in the file is out of date.\n\
         - update: some statements no longer match the code or the other documents.\n\
         - delete: the whole file is obsolete and nothing needs it.\n\
         For update or delete you must give evidence: the file path, the 1-based line number and an exact quote from that line. Every evidence line is re-read by a program, and a verdict whose evidence does not match is thrown away. If you are not sure, answer ok with evidence as an empty list.\n\
         Write the reason in plain Traditional Chinese, at most 120 characters. Answer with the JSON object only.",
        path = group.path,
        signals = signals.join("\n"),
    )
}
