use crate::drift::{
    claude_prompt::claude_prompt, codex_prompt::codex_prompt, file_group::FileGroup,
    file_result::FileResult, finding::Finding, reviewers::Reviewers, ruling::Ruling,
    second_opinion::SecondOpinion, verdict::Verdict,
};
use crate::prelude::*;
use std::collections::BTreeMap;

const CLAUDE_REPLY: &str = "claude reply has the wrong shape";
const CODEX_REPLY: &str = "codex reply has the wrong shape";

pub fn review_file(
    group: &FileGroup,
    files: &BTreeMap<String, Vec<u8>>,
    reviewers: &mut Reviewers,
    index: usize,
) -> Result<FileResult, XtaskError> {
    let reply = (reviewers.claude)(&format!("claude-{index}"), &claude_prompt(group))?;
    let finding = Finding::parse(&reply).ok_or(XtaskError::ReviewTool(CLAUDE_REPLY.into()))?;
    let path = group.path.clone();
    if finding.verdict == Verdict::Ok {
        return Ok(FileResult {
            path,
            ruling: Ruling::NoIssue,
            finding,
            opinion: None,
        });
    }
    if !finding.holds(files) {
        return Ok(FileResult {
            path,
            ruling: Ruling::Rejected,
            finding,
            opinion: None,
        });
    }
    let second = (reviewers.codex)(
        &format!("codex-{index}"),
        &codex_prompt(&group.path, &finding),
    )?;
    let opinion =
        SecondOpinion::parse(&second).ok_or(XtaskError::ReviewTool(CODEX_REPLY.into()))?;
    let ruling = if opinion.agree {
        Ruling::Confirmed
    } else {
        Ruling::Disputed
    };
    Ok(FileResult {
        path,
        ruling,
        finding,
        opinion: Some(opinion),
    })
}
