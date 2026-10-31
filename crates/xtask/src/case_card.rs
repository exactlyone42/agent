use crate::error::XtaskError;
use crate::html_escape::html_escape;
use std::collections::HashMap;

const FIELDS: [&str; 6] = ["前置", "操作", "通過", "失敗對照", "證據", "AI／真人"];

pub struct CaseCard {
    pub id: String,
    title: String,
    fields: HashMap<String, String>,
}

impl CaseCard {
    pub fn parse_all(evaluation: &str) -> Result<Vec<Self>, XtaskError> {
        let mut cards: Vec<Self> = Vec::new();
        let mut open = false;
        for line in evaluation.lines() {
            open = Self::feed(&mut cards, open, line);
        }
        if cards.is_empty() {
            return Err(XtaskError::Report("評測文件沒有驗收卡"));
        }
        Ok(cards)
    }

    fn feed(cards: &mut Vec<Self>, open: bool, line: &str) -> bool {
        if line.starts_with("## ") {
            return false;
        }
        if let Some(card) = Self::header(line) {
            cards.push(card);
            return true;
        }
        let Some(card) = cards.last_mut().filter(|_| open) else {
            return open;
        };
        card.add_field(line);
        open
    }

    fn header(line: &str) -> Option<Self> {
        let (id, title) = line.strip_prefix("### ")?.split_once('｜')?;
        let digits = id.strip_prefix('A')?;
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some(Self {
            id: id.to_string(),
            title: title.to_string(),
            fields: HashMap::new(),
        })
    }

    fn add_field(&mut self, line: &str) {
        let Some((key, value)) = line.strip_prefix("- ").and_then(|l| l.split_once('：')) else {
            return;
        };
        self.fields.insert(key.to_string(), value.to_string());
    }

    fn field(&self, key: &'static str) -> Result<String, XtaskError> {
        self.fields
            .get(key)
            .map(|value| html_escape(value))
            .ok_or(XtaskError::Report("驗收卡缺少必要欄位"))
    }

    pub fn html(&self) -> Result<String, XtaskError> {
        let mut body = format!(
            "<h3 id=\"{id}\">{id}｜{title}</h3><div class=\"state\">{stage}｜待執行</div>",
            id = self.id,
            title = html_escape(&self.title),
            stage = self.field("階段")?,
        );
        for key in FIELDS {
            body.push_str(&format!("<p><b>{key}：</b>{}</p>", self.field(key)?));
        }
        Ok(format!("<article class=\"card\">{body}</article>"))
    }
}
