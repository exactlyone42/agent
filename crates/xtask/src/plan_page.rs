use crate::error::XtaskError;
use std::path::Path;

pub struct PlanPage {
    pub id: String,
    pub title: String,
    pub body: String,
}

impl PlanPage {
    pub fn new(id: &str, title: &str, body: String) -> Self {
        Self {
            id: id.to_string(),
            title: title.to_string(),
            body,
        }
    }

    pub fn read_all(folder: &Path) -> Result<Vec<Self>, XtaskError> {
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(folder)? {
            paths.push(entry?.path());
        }
        paths.retain(|path| path.extension().is_some_and(|ext| ext == "html"));
        paths.sort();
        paths.iter().map(|path| Self::read(path)).collect()
    }

    fn read(path: &Path) -> Result<Self, XtaskError> {
        let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned());
        let id = stem
            .as_deref()
            .and_then(|name| name.split_once('-'))
            .map(|(_, id)| id)
            .ok_or(XtaskError::Report("頁面檔名須為 序號-代號.html"))?;
        let content = std::fs::read_to_string(path)?;
        let (title, body) = content
            .split_once('\n')
            .ok_or(XtaskError::Report("頁面檔缺少標題行"))?;
        Ok(Self::new(id, title, body.to_string()))
    }

    pub fn section(&self, number: usize) -> String {
        format!(
            "<section class=\"page\" id=\"{id}\"><h1>{title}</h1><p class=\"sub\">Agent Platform｜現行計畫圖文版｜Linux 實機驗證，Windows／macOS 只做相容設計；所有產品驗收待執行。</p>{body}<div class=\"foot\">{number}｜實線箭頭：標示的步驟、控制或資料傳遞；雙箭頭：請求／回應；虛線框：部署或責任群組。本文不是測試通過報告。</div></section>",
            id = self.id,
            title = self.title,
            body = self.body,
        )
    }
}
