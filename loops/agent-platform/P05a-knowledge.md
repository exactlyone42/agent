# P05a 外部 RAG 接口與整合

母檔：../agent-platform.loop.md。使用者 2026-11-06 決定：RAG 在獨立 https://github.com/ifandonlyif42/rag 開發，本 repo 只做接入接口。

## 範圍

Agent 送出查詢並接收可追溯的原文、來源位置與版本；沿用批准、權限範圍、取消及任務紀錄。文件發現、分類、解析、切片、索引、格式支援、來源更新與 RAG 本體評測由 rag repo 負責，不依賴外部 Harness、Tool Use 或 Computer Use 才能開發。

## 子任務

| id | 工作 | 驗收 | 狀態 |
| --- | --- | --- | --- |
| K1 | 透過 Offora 與 RAG 負責 AI 對齊有版本號的請求、回覆及錯誤 | 保存提案、對方回覆與雙方決定；不自行當成已同意 | WAITING |
| K2 | 接入已確認接口，保存查詢與引用來源 | 成功、有界查詢及缺來源反例；未接真服務須明列 | PENDING |
| K3 | 取消、逾時、不可用、撤權及失效回覆處理 | 不擴權、不盲重試、不把失效內容當成功 | PENDING |
| K4 | 與真 RAG 做整合讀回 | 原文／版本可核對；失敗不偽稱成功；模擬不能代替真整合 | PENDING |

## 目前狀態

2026-11-11：使用者說 RAG 已完成，已讀 rag 程式碼（feature/project-scope，ed8166c）。docs/rag-interface.md 已改成程式碼核對的接口：工具為 rag_search、rag_query、live_*、選用 web_*，無 rag_read。Offora 八條提案（message_id 2985616e…）仍無回覆，對方未同意。K1 以程式碼為準；K2 可依該文件寫接入，K4 需 SQLite、PostgreSQL+pgvector、Ollama、tokenizer，且 rag 入口名 rag 已被 crates.io 占用，只能路徑依賴。rag 的測試我沒重跑。原 K0–K8 的 RAG 本體需求以 PLAN 範圍修訂及 docs/knowledge-plan.md 保留作移交參考。
