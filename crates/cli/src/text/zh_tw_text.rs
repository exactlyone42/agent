use super::{zh_tw_store_text::zh_tw_store_text, zh_tw_task_text::zh_tw_task_text};
use crate::prelude::*;
use agent_platform_agent::store::{APPROVAL_TTL_SECONDS, EVENT_PAGE_SIZE};

pub fn zh_tw_text(message: &Message) -> String {
    match message {
        Message::Help => format!(
            "用法：{APP_NAME} {COMMAND_RUN} <任務.json>\n      {APP_NAME} {COMMAND_STATUS} <任務編號>\n      {APP_NAME} {COMMAND_EVENTS} <任務編號> [已讀序號]\n      {APP_NAME} {COMMAND_APPROVALS} <任務編號>\n      {APP_NAME} {COMMAND_APPROVE} <批准編號> <任務版本>\n      {APP_NAME} {COMMAND_DENY} <批准編號> <任務版本>\n{COMMAND_EVENTS} 每次最多 {EVENT_PAGE_SIZE} 筆，下一頁使用最後一筆的 seq 作已讀序號。\n任務 JSON 的 approval_mode 設 manual 可逐次確認工具動作；每次批准有效 {APPROVAL_TTL_SECONDS} 秒。\n目前為開發入口，只連本機模型服務；讀寫須在任務中明確授權。尚非企業正式發布。"
        ),
        Message::StateDirEmpty => format!("{STATE_DIR_ENV} 不可為空"),
        Message::NoUserDataDir => format!("找不到使用者資料目錄，請設定 {STATE_DIR_ENV}"),
        Message::StateDirNotAbsolute => format!("{STATE_DIR_ENV} 必須是絕對路徑"),
        Message::StateDirCreateFailed => "無法建立任務資料目錄".into(),
        Message::StateDirMissing => "找不到任務資料目錄".into(),
        Message::StateDirNotPlain => "任務資料目錄必須是一般目錄，不可使用符號連結".into(),
        Message::StateDirTooOpen => format!(
            "任務資料目錄不可讓其他使用者存取，請設定權限為 {}",
            crate::state_dir::MODE_HINT
        ),
        Message::DatabaseNotPlain => "任務資料庫必須是一般檔案，不可使用符號連結".into(),
        Message::DatabaseHardLink => "任務資料庫不可使用硬連結".into(),
        Message::DatabaseUnreadable => "找不到或無法讀取任務資料庫".into(),
        Message::InvalidTaskId => "任務編號格式錯誤".into(),
        Message::InvalidApprovalId => "批准編號格式錯誤".into(),
        Message::InvalidRevision => "必須提供正整數的任務版本".into(),
        Message::InvalidEventPosition => "事件位置格式錯誤".into(),
        Message::OutputFailed => "無法輸出結果".into(),
        Message::TaskFileOpenFailed => "無法開啟任務檔".into(),
        Message::TaskFileReadFailed => "無法讀取任務檔".into(),
        Message::TaskFileTooLarge => format!("任務檔超過 {TASK_FILE_LIMIT_KIB} KiB"),
        Message::TaskFileInvalid => "任務 JSON 格式錯誤或包含未知欄位".into(),
        Message::EndpointRejected => {
            "目前只接受明確的本機 IP 模型入口與非空模型名稱；網址不得夾帶憑證或查詢參數".into()
        }
        Message::ApiKeyInvalid => format!("{API_KEY_ENV} 格式錯誤"),
        Message::ModelClientFailed => "無法建立模型連線".into(),
        Message::InterruptHandlerFailed => "無法接收停止要求，任務未啟動".into(),
        Message::WorkingDirUnavailable => "無法取得工作目錄".into(),
        Message::TaskRecordPoisoned => "任務紀錄狀態異常".into(),
        Message::OutcomeUnknown => "工具結果未確認，請核對紀錄與成果後再接續".into(),
        Message::StatusMayBeStale => {
            "這是最後保存的狀態，不能確認程序仍在執行；請先核對成果，不要直接重跑。".into()
        }
        Message::TaskStarted(task_id) => format!("任務編號：{task_id}"),
        Message::ManualModeHint => format!(
            "逐次確認模式：請以 {COMMAND_APPROVALS} 查閱這個任務，再用 {COMMAND_APPROVE} 或 {COMMAND_DENY} 提交決定及任務版本。"
        ),
        Message::TaskCancelled => "已停止本機任務；已建立的成果會保留，請核對後再接續。".into(),
        Message::Store(error) => zh_tw_store_text(error).into(),
        Message::Task(error) => zh_tw_task_text(error).into(),
        Message::FinishUnconfirmed(error) => {
            format!("{}；任務結果未確認，請勿直接重跑", zh_tw_store_text(error))
        }
    }
}
