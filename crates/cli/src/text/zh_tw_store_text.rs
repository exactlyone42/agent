use crate::prelude::*;

pub fn zh_tw_store_text(error: &StoreError) -> &'static str {
    match error {
        StoreError::Database(_) => "無法保存或讀取任務紀錄",
        StoreError::Incompatible => "任務資料庫版本或設定不相容",
        StoreError::NotFound => "找不到任務",
        StoreError::Conflict => "任務紀錄已變動，不可用舊狀態保存結果或重複執行",
        StoreError::InvalidCursor => "事件位置不可小於零",
        StoreError::InvalidSpec => "任務規格不符合限制，不能繼續執行",
        StoreError::InvalidToolCall => "工具名稱或參數不符合限制",
    }
}
