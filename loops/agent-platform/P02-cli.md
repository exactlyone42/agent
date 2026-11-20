# P02 可用 CLI，連真 Gateway

母檔：../agent-platform.loop.md。依賴 P01。對應：CLI、任務執行、Gateway 連線 → Gateway。契約見 docs/design/cli.md。

## 目標

使用者選工作資料夾，經真 Gateway 完成「修復 Markdown 連結」任務；拒絕與取消零寫入；偽造成功文字被驗證器擋下。

## 子任務

| id | 做什麼 | 驗收 | 狀態 |
| --- | --- | --- | --- |
| C1 | `version`：回報契約版本與能力 | 測試；輸出含版本 | PENDING |
| C2 | `list`：列任務、狀態、工作區 | 分頁、空清單、壞檔測試 | PENDING |
| C3 | `result`：改了哪些檔、差異、驗證結果 | 差異與實際檔案一致 | PENDING |
| C4 | `events --follow`：JSON 一行一筆，可從序號接續 | 斷線續讀不漏不重 | PENDING |
| C5 | 互動授權與非互動執行（缺授權即停） | 兩種模式正反測試 | PENDING |
| C6 | Gateway 連線用虛擬金鑰連真 Gateway（原 Client 實送一次） | 真請求成功；金鑰不進日誌 | PENDING |
| C7 | 連結修復任務 + 獨立驗證器 | 偽造成功被擋；故意錯結果必失敗 | PENDING |
| C8 | P08 最小評測資料與入口 | 一鍵跑，正常與故障分開報 | PENDING |
| C9 | 真人驗收：不看內部日誌能看懂改動並批准或拒絕 | 具名驗收人紀錄 | WAITING |

## 完成條件

C1–C9 完成；CLI 初版交付點。未接真 Gateway 不得標完成。
