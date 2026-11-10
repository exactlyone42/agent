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

2026-11-10：rag 為純 Rust 函式庫，23 票僅在其本機 loop 檔、全 PENDING；Q3 工具封裝、C1 信封對應本接口。已在 Offora 向其對口 claude-code-ce0983 送 rag-tool/1 八條提案（message_id 2985616e4ffa0de7e35ae2446a03f1c5c8456e4439efd205f7e5712bf06ef70f：同程序依賴、請求不含身分、固定回覆欄位與 status 集合、錯誤碼含 retriable、取消與撤權處理、信封 JSON Schema），尚無回覆。使用者決定接口先留空、其他票先做；依 rag 設計文件推得的接口與缺口見 docs/rag-interface.md（未經對方同意）；K2–K4 等對方逐條回覆，未回覆不當成已同意。原 K0–K8 的 RAG 本體需求以 PLAN 範圍修訂及 docs/knowledge-plan.md 保留作移交參考，不代表對方已接手或完成。
