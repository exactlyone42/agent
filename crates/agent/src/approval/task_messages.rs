use agent_harness_approval::Messages;

pub fn task_messages() -> Messages {
    Messages {
        cancelled: "任務已要求停止".into(),
        call_limit: "工具呼叫已達任務上限，不能繼續執行".into(),
        intent_failed: "無法保存工具意圖，未執行工具".into(),
        claim_failed: "無法領用批准，未執行工具".into(),
        rejection_unrecorded: "無法保存工具拒絕紀錄，任務停止".into(),
        invalid_call: "工具參數或授權範圍不符合限制".into(),
        not_authorized: "使用者未授權此工具操作".into(),
        approval_timeout: "等待批准逾時，未執行工具".into(),
        approval_denied: "工具未獲批准或批准已失效".into(),
        receipt_failed: "無法保存工具結果，請核對已執行的動作".into(),
        receipt_cancelled: "任務已要求停止；工具結果尚未保存".into(),
    }
}
