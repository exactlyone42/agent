# 外部 RAG 接口（依 rag 程式碼核對）

這份說明 Agent 要怎麼呼叫獨立 repo `rag`（知識庫函式庫）。2026-11-11 讀 `/home/cash/work_space/private/rag` 的 `feature/project-scope` 分支（HEAD `ed8166c`，另有對方未提交的修改）。內容取自程式碼，不是雙方已同意的接口；我沒有編譯或執行 rag，它自己宣稱的測試結果列在「缺口」。

## 程式碼已證實的

- **接入方式**：同程序內呼叫 Rust 函式庫，入口 crate 叫 `rag`（`rag/src/rag.rs` 匯出各模組），共 15 個 `crates/*`。沒有 HTTP 或 MCP 服務。目前只能用路徑依賴。
- **版本**：0.1.0、edition 2024、rust-version 1.98、Apache-2.0，與本 repo 的 Rust 版本相同。
- **工具清單**：`ToolSet::descriptors()` 回 `rag_search`、`rag_query`，每個登記的唯讀具名 SQL 查詢各多一個 `live_<query_name>`，開了網頁功能才多 `web_search`、`web_read`（`rag_query/src/tools.rs:142-180`）。沒有 `rag_read`，也沒有 `rag_answer`；舊版本文件的這兩個名字不存在。
- **參數**：`rag_search` 與 `rag_query` 只收 `question`（1 到 2000 字）和選填 `as_of_ms`（整數），多給任何欄位回 `INVALID_ARGUMENT`（`tools.rs:64-72`、`294`）。`rag_search` 只檢索並回證據；`rag_query` 再呼叫模型產生帶引用的答案。
- **呼叫方式**：`tools.session(ctx, 私有文字清單)` 建一次執行用的 session，再 `session.call(name, &args).await` 回 `ToolReply`（`tools.rs:187`）。範例在 `examples/agent_style/src/agent_style.rs`。
- **身分**：`QueryContext` 由可信的呼叫端建立，含 `tenant_id`、`subject_id`、`device_id`、`kb_ids`（1 到 16 個）、`egress_policy`（`LocalOnly` 或 `AllowedModels`）、`deadline_ms`、`budget`、`trace_id`。工具參數不帶身分。`deadline_ms` 是絕對 Unix 毫秒，必須晚於現在；`trace_id` 必須是 UUID（`rag_contracts/src/query.rs:50-66`）。
- **預算預設**：最多 8 次工具呼叫、6 次模型呼叫、8000 讀取 token、費用上限 0（`query.rs:28-31`）。Agent 是否固定傳預設值，待決定。
- **回覆**：`ToolReply` 有 `content`（JSON 字串）、`is_error`、`stop`（`tools.rs:34`）。JSON 內有 `trace_id`、`stop`、`usage`、`unknown_result` 和 `result`；`result` 有 `source`、`version`（目前 "1.0"）、`observed_at`、`status`（`ok`、`no_data`、`error`）、`egress_limit`、`data`、`error`（`code`、`retryable`、`summary`）。
- **`data` 內容**：`evidence`（證據清單）、`snapshot_refs`、`gaps`、`conflicts`、`degraded`、`unread`、`sufficiency`（`supported`、`partial`、`conflict`、`no_evidence`）、`answer`（每個 claim 帶 `citations`：`evidence_id`、`kb_id`、`revision_id`、`text_sha256`、`start`、`end`）、`answer_status`（`Answered`、`Partial`、`Conflict`、`Abstained`、`Error`）。
- **證據欄位**：文件證據有 `evidence_id`、`kind`、`kb_id`、`revision_id`、`text`、`text_sha256`、`location`、`effective_at_ms`、`expires_at_ms`；即時查詢證據有 `connector`、`query_name`、`args_sha256`、`observed_at_ms`、`expires_at_ms`、`payload`。
- **停止原因**：`Completed`、`BudgetExhausted`、`DeadlineReached`、`NoNewEvidence`、`Forbidden`、`Cancelled`（`rag_contracts/src/refs.rs:72`）。同一個 session 重複同樣的呼叫會回 `NoNewEvidence`（`tools.rs:552`）。
- **錯誤碼**：23 個。只有 `STALE_REVISION`、`RESOURCE_EXHAUSTED`、`TIMEOUT`、`UPSTREAM_FAILED`、`STORE_UNAVAILABLE` 標為可重試。未授權一律回 `FORBIDDEN` 與 "not authorized"（`rag_runtime/src/guard.rs:97`）。
- **取消**：`session.cancel()` 只設旗標，下一次 `call` 開頭才檢查（`tools.rs:441`、`544`）。已經在跑的呼叫要靠丟掉 Future 才會停，丟掉後 rag 把結果標為 `unknown_result`（`guard.rs:85-93`）。逾時取呼叫上限與剩餘期限的較小者，到期回 `TIMEOUT`（`guard.rs:187-194`）。
- **授權檢查順序**：期限、授權（依 subject、device、kb、動作）、外送政策、預算、逾時（`guard.rs:152-189`）。

## Agent 端的做法（本 repo 決定，對方不受影響）

- 身分從 E01 的身分政策建立，每次呼叫帶入；模型只提 `question`。
- 工具註冊走批准閘門，當唯讀工具。先只接 `rag_search`；`rag_query` 會呼叫模型並外送資料，等批准規則定案再接。`live_*` 與 `web_*` 先不接。
- `is_error` 為真、`status` 為 `error`、`unknown_result` 為真、`degraded` 非空，一律當失敗或不完整，不當成「查無資料」或成功。
- 文件證據缺 `revision_id` 或 `text_sha256` 當失敗。
- `Forbidden`、`STALE_REVISION` 不擴權、不重送舊引用，任務紀錄標明原因。
- 任務紀錄只存引用（`evidence_id`、`revision_id`、`text_sha256`、`location`），不存全文與完整結果。
- 取消時丟掉呼叫，取消後不寫任何結果。成功只由驗證器判定，模型文字不算。

## 對原 8 點提案要改的地方

- 欄位改用 rag 的 `evidence_id`、`revision_id`、`text_sha256`、`observed_at_ms`、`expires_at_ms`，不用提案自擬的 `source_uri`、`content_hash`、`retrieved_at`。
- 狀態改用 rag 的 `status` 加 `answer_status` 加 `ErrorCode`，不另造 `ok`、`denied`、`stale` 這組名稱。
- 撤回 `idempotency_key`：工具參數沒有冪等鍵，只有 session 內的重複呼叫偵測。
- 「請求不含身分」與取消方式與 rag 實作一致，不改。

## 仍是缺口

- **執行環境**：範例需要 SQLite 檔、PostgreSQL 加 pgvector（`RAG_PG_URL`）、本機 Ollama（預設 `http://127.0.0.1:11434`）和一個 tokenizer.json。Agent 要不要自帶這些、誰建庫與寫入，沒決定。
- **依賴方式**：入口名稱 `rag` 在 crates.io 已被占用，發佈前要改名（`rag/docs/release-check.md`）；現在只能路徑依賴，改名會動到 `rag::` 的所有用法。
- **對方自己宣稱、我沒重跑**：全庫 458 項通過、4 項預設不跑；`examples/agent_style` 測試曾因真實 4B 模型回「model output invalid」間歇失敗，原因未查出；總費用上限沒有端到端觸發；沒有 OCR、重排、關係補查。
- **撤權與失效**：授權只在每次呼叫的檢查當下判定。Agent 已持有的片段在撤權後怎麼處理，rag 沒有通知機制，Agent 使用引用前要再查一次。回覆裡撤權與 `STALE_REVISION` 長什麼樣，我沒有跑過。
- **對方狀態**：對方 session 仍在改檔，分支未合併；我們在 Offora 的 8 條提案沒有回覆。
- **品質與延遲**：沒有實測數據。

## 下一步

K2 用這份文件的欄位寫接入與保存；K4 需要上面的執行環境與對方同意，之前不當成已整合。
