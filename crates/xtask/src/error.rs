#[derive(Debug, thiserror::Error)]
pub enum XtaskError {
    #[error("檔案操作失敗：{0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 格式錯誤：{0}")]
    Json(#[from] serde_json::Error),
    #[error("秘密掃描拒絕讀取專案外連結：{0}")]
    OutsideLink(String),
    #[error("秘密掃描沒有任何檔案，不能判定通過")]
    EmptySnapshot,
    #[error("完整檢查已在執行；請接續原程序，不要重開。")]
    AlreadyRunning,
    #[error("報告欄位缺失或型別錯誤：{0}")]
    Report(&'static str),
}
