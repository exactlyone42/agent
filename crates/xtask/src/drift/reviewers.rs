use crate::prelude::*;
use serde_json::Value;

pub type ReviewCall<'a> = dyn FnMut(&str, &str) -> Result<Value, XtaskError> + 'a;

pub struct Reviewers<'a> {
    pub claude: Box<ReviewCall<'a>>,
    pub codex: Box<ReviewCall<'a>>,
}
