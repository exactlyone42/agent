use crate::prelude::*;

pub fn zh_tw_task_text(error: &TaskError) -> &'static str {
    match error {
        TaskError::ApprovalDenied => "工具未獲批准或批准已失效；請查詢批准紀錄",
        TaskError::Cancelled { .. } => "任務已停止；若已建立成果，請核對後再接續",
        TaskError::InvalidTask => "任務內容不符合長度限制",
        TaskError::InvalidInput => "來源必須是大小在限制內的一般檔案",
        TaskError::Input(_) => "無法開啟獲准的來源檔案",
        TaskError::Harness(_) => "模型執行失敗；可能已有部分成果，請核對後再接續",
        TaskError::Timeout => "任務逾時；可能已有部分成果，請核對後再接續",
        TaskError::State => "工具狀態異常，不能確認成果",
        TaskError::MissingArtifact => "模型已結束，但沒有可驗證的成果檔案",
        TaskError::IncompleteArtifact => "成果未完整寫入或保存，請核對現有檔案",
        TaskError::Verification(_) => "成果驗證未通過",
    }
}
