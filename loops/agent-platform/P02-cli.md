# P02 可用 CLI，連真 Gateway

母檔：../agent-platform.loop.md。依賴 P01。對應：CLI、任務執行、Gateway 連線 → Gateway。契約見 docs/cli.md。

## 目標

使用者選工作資料夾，經真 Gateway 完成「修復 Markdown 連結」任務；拒絕與取消零寫入；偽造成功文字被驗證器擋下。

人類使用介面：啟動一次後以 `/` 短指令操作，不要求每次重打程式名稱、任務編號或版本；批准時在同一介面顯示動作與內容，由使用者直接批准或拒絕。現有命令列保留供非互動用途；互動介面開發中，尚待完整驗收。

## 子任務

| id | 做什麼 | 驗收 | 狀態 |
| --- | --- | --- | --- |
| C1 | `version`：回報契約版本與能力 | 測試；輸出含版本 | PENDING |
| C2 | `list`：列任務、狀態、工作區 | 分頁、空清單、壞檔測試 | PENDING |
| C3 | `result`：改了哪些檔、差異、驗證結果 | 差異與實際檔案一致 | PENDING |
| C4 | `events --follow`：JSON 一行一筆，可從序號接續 | 斷線續讀不漏不重 | PENDING |
| C5 | 互動授權與非互動執行（缺授權即停） | 兩種模式正反測試 | RUNNING（`/help`、`/run`、`/status`、`/events`、`/approvals`、`/approve`、`/deny`、`/exit` 已有程式與 2 項測試，快檢通過；缺授權即停的非互動負向測試與真人驗收未做） |
| C6 | Gateway 連線用虛擬金鑰連真 Gateway（原 Client 實送一次） | 真請求成功；金鑰不進日誌 | RUNNING（2026-11-04：本機 Gateway 127.0.0.1:18088 已發 agent 專用金鑰，存 `~/.config/agent-platform/gateway-key`（權限 600，30 天效期）；`/v1/models` 回 200 共 20 個模型；`/v1/chat/completions` 三次皆失敗：`ollama/qwen3.8:27b` 502 上游失敗、`local-general` 503 無可用供應商、`claude-haiku-5-5` 400 請求選項不支援。尚無真請求成功） |
| C7 | 連結修復任務 + 獨立驗證器 | 偽造成功被擋；故意錯結果必失敗 | PENDING |
| C8 | P08 最小評測資料與入口 | 一鍵跑，正常與故障分開報 | PENDING |
| C9 | 真人驗收：不看內部日誌能看懂改動並批准或拒絕 | 具名驗收人紀錄 | WAITING |

## 完成條件

C1–C9 完成；CLI 初版交付點。未接真 Gateway 不得標完成。

真 Gateway 已架在本機（使用者 2026-11-04 告知），目前只允許本機 IP 的限制不擋 C6；遠端 Gateway 與 TLS 留待之後。C6 須先經 Offora 向 Gateway 取得網址、虛擬金鑰取得方式與模型名稱。P02 其餘子任務等 P01 整合驗收與階段 1B 後再開工。
