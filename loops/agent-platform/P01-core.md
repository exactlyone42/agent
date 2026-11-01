# P01 核心迴圈與批准紀錄

母檔：../agent-platform.loop.md。對應圖上框：agent-harness、批准閘門、批准轉接、任務紀錄檔。細節見 docs/harness-design.md、docs/agent-method.md。

## 目標

自寫 agent-harness 的行為完整且可證明：工具壞參數、取消、重啟都有確定結果；批准與回執只寫進任務紀錄檔。

## 子任務

| id | 做什麼 | 驗收 | 狀態 |
| --- | --- | --- | --- |
| H1 | 使用者確認 docs/harness-design.md 階段 0 取捨表（並修表內「任務資料庫」舊字） | 使用者回覆；取捨寫入文件 | DONE（使用者 2026-11-01 確認新版與兩項決策） |
| H2 | 壞工具參數改為回報模型，不中止任務 | 失敗測試先紅後綠；openai `read_reply` 同步 | DONE（gate 通過；新增 2 測試；舊行為是 read_reply 回錯誤而中止） |
| H3 | 呼叫與結果配對；停止原因列舉 | 缺結果、重複結果、錯配都有測試 | RUNNING（階段 1A 完整檢查通過；重複編號在接受前拒絕整批；CLI 取消歷史持久化待階段 1B） |
| H4 | 取消後延遲工具不能再寫入 | 取消競爭測試；取消後無寫入 | DONE（gate 通過；工具執行中收到取消即丟棄；邊界：已進入同步阻塞寫入的工具無法中斷） |
| H5 | 兩個程序搶同一任務、崩潰於工具前後的恢復 | 雙程序測試；崩潰注入；未知副作用不盲重送 | 部分完成（3 測試只用執行緒與關閉連線模擬；真雙程序與殺程序待 P04；見 docs/aar-h5-simulation.md） |
| H6 | 同題比較 Codex 與自寫 harness，列版本、授權、選用原因 | 先驗假模型及全部工具攔截；不符則記淘汰與證據 | DONE（假模型接通；互動程序後續輸入不經掛勾或再次批准，淘汰本次原版整合，見 docs/p01-codex-evaluation.md） |
| H7 | Windows 路徑與程序取消缺口（記錄在 mutation-equivalents.md） | 改成只做相容設計並明列不實測 | DONE（docs/mutation-equivalents.md 已改寫；未改程式） |
| H8 | 事件加 `schema_version`、固定錯誤代碼 | 契約文件與測試 | DONE（gate 通過；失敗最後一行為含 error_code 的 JSON；事件原本已有 schema_version；5 個代碼有測試，其餘未逐一測） |

## 完成條件

固定假模型回應的允許、拒絕、錯參數、串流、取消、重啟都有結果；任一安全條件不過則不算完成。
