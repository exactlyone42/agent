# P05a 知識檢索（常用格式）

母檔：../agent-platform.loop.md。依賴 P04。方案見 docs/design/knowledge-plan.md；門檻見 docs/research/evaluation.md「RAG 階段映射與可執行門檻」。

## 目標

背景發現檔案、解析、切片、全文加向量搜尋、重排、引用；Agent 呼叫知識檢索，結果回到 Harness 決定下一步。三塊 RAG 一塊都不能少。

## 子任務

| id | 做什麼 | 驗收 | 狀態 |
| --- | --- | --- | --- |
| K0 | 先定「哪些資料是知識、訊息、歷史記錄」的分類標準與各類處理（knowledge-plan.md:85,91 待改） | 文件定案；系統檔、程式、快取不切片 | PENDING |
| K1 | Task／Knowledge 匯入與查詢契約（版本、request ID、scope、deadline、取消、型別化錯誤） | 第二個 CLI Client 能獨立使用 | PENDING |
| K2 | 發現與索引排程：事件、定期核對、游標、前景優先、資源限額 | 漏事件補掃、續跑、2 CPU／4 GiB 限制 | PENDING |
| K3 | 解析與切片（Docling＋LlamaIndex 首次試接；Swiftide、langchain-rust 同題比較） | 對照原文位置；毀損、改檔競爭 | PENDING |
| K4 | 向量、全文、重排；中文、數字、負面條件 | 精確對照近似；找回率 ≥0.95 | PENDING |
| K5 | 權限與有效版本：撤權、改檔、刪除後舊答案失效 | 未發布或撤權片段零返回 | PENDING |
| K6 | Agentic RAG：多步、矛盾、無答案、預算用盡 | 跨文件 10 題 ≥9 正確；最多 3 次檢索 | PENDING |
| K7 | 60 題調整集加 60 題保留集評測；AI 評分與真人標註校準（≥12 題） | 達 evaluation.md 的各項門檻 | PENDING |
| K8 | 備份還原與一致性 manifest | 恢復後重新核權 | PENDING |

## 完成條件

K0–K8 完成。門檻不改低；做不到就修實作或明確修訂範圍。
