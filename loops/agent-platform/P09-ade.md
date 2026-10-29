# P09 桌面 ADE

母檔：../agent-platform.loop.md。依賴 P04。程式在另一個 repo agent-ade；本 repo 只提供 CLI、核心與有版本號的事件契約。不做完整 IDE，不改純 Web。

## 子任務

| id | 做什麼 | 驗收 | 狀態 |
| --- | --- | --- | --- |
| D1 | 本 repo：事件契約定版（版本號、相容規則、文件） | 第二個 Client 讀得懂 | PENDING |
| D2 | agent-ade：工作區、任務、事件顯示（子程序呼叫 CLI） | 真視窗 | PENDING |
| D3 | 授權畫面、差異、成果 | 成功、拒絕、停止 | PENDING |
| D4 | CLI 建立任務後桌面接續；雙入口搶同任務被拒 | 兩種入口測試 | PENDING |
| D5 | 事件丟失補讀、關窗、斷線、舊按鈕與核心版本不符 | 反例全處理 | PENDING |
| D6 | 三平台桌面包 | 乾淨機安裝 | PENDING |
| D7 | 依五個試用任務決定維持 ADE、整合編輯器或另做 IDE | 使用者決定 | WAITING |

## 完成條件

D1–D6 完成；D7 由使用者決定，不先開發 IDE。
