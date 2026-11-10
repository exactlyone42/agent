# 外部 RAG 接口（依 rag 設計文件推得，未經對方同意）

這份說明 Agent 將如何呼叫獨立 repo `rag` 的知識庫函式庫。內容全部取自 rag 的設計文件，不是雙方已同意的接口。rag 目前沒有可執行程式，也沒有 GitHub issue，23 票全是 PENDING（2026-11-10 讀 `loops/tickets.json`）。對方逐條回覆前，P05a 的 K2 到 K4 不開工。

來源（都在 `/home/cash/work_space/private/rag`）：`docs/schema.md` 第一、四節、`docs/modules/contracts-and-policy.md`、`docs/modules/query-orchestration.md`、`docs/modules/evidence-assembly.md`、`loops/tickets.json` 的 C1、Q2、Q3、X1。

## rag 自己已經定下的

- **接入方式**：同一個程序內呼叫 Rust 函式庫，公開函式全是 `async fn`。不內建 HTTP；MCP 轉接另開 crate，需要才做。
- **工具清單**：`ToolSet` 的 `descriptors()` 回手寫的 JSON Schema，`call(name, args, ctx)` 執行。固定四個工具：`rag_query`、`rag_search`、`rag_read`、`rag_answer`。Agent 自己的工具轉接寫在 Agent repo，rag 不依賴任何 Agent。
- **身分**：`QueryContext` 含 `tenant_id`、`subject_id`、`device_id`、`kb_ids`、`egress_policy`、`deadline_ms`、`budget`、`trace_id`，由可信的呼叫端建立，工具參數不收模型自報的身分。`deadline_ms` 是絕對時間（Unix 毫秒）。
- **回覆分流**：有答案、部分、衝突、查無，以及被拒絕（`FORBIDDEN`、`STALE_REVISION`）都是 `Ok(信封)`。只有組不出信封（`STORE_UNAVAILABLE`、`CONFIG_INVALID`、`INTERNAL`，以及 context 本身不合格）才是 `Err`。
- **信封欄位**：`schema_version`（"主.次"）、`trace_id`、`status`（`Answered`、`Partial`、`Conflict`、`Abstained`、`Error`）、`error`（`code`、`retryable`、`summary`）、`gaps`、`usage`；答案與證據包另有 `stop_reason`、`warnings`、`snapshot_refs`。
- **證據欄位**：`evidence_id`、`kb_id`、`kind`（文件或即時）、`revision`（`revision_id` 加選填 `chunk_id`）、`text`、`text_sha256`、`observed_at`、選填 `expires_at`。
- **錯誤碼**：16 個，各標可重試與否（例：`TIMEOUT`、`UPSTREAM_FAILED`、`STALE_REVISION` 可重試，`FORBIDDEN` 不可）。rag 自己不自動重試，是否重送由呼叫端決定。
- **取消**：丟掉 Future 或到期後，不再發新請求（Q2 的 Q-cancel）。
- **撤權與失效**：`read` 遇撤權回 `FORBIDDEN`，版本到期回 `STALE_REVISION`。授權只在讀取當下判定，沒有主動通知。
- **外送**：原文交給外部 Agent 後 rag 追不回，Agent 延後使用或轉送時必須重新核授權。

## Agent 端的做法（本 repo 決定，對方不受影響）

- 身分從 E01 的身分政策建立，每次呼叫帶入；模型只提查詢文字。
- 工具註冊走批准閘門，當唯讀工具。先只接 `rag_search` 與 `rag_read`；`rag_query` 與 `rag_answer` 會呼叫模型並外送資料，等對方回覆後再決定。
- `Err`、`Error` 狀態、缺 `revision` 或 `text_sha256` 的文件證據，一律當失敗，不當成「查無資料」或成功。
- `FORBIDDEN`、`STALE_REVISION` 不擴權、不重送舊引用，任務紀錄標明原因。
- 任務紀錄只存引用（`evidence_id`、`revision`、雜湊、位置），不存全文與完整查詢結果。
- 取消時丟掉呼叫；取消後不寫任何結果。成功只由驗證器判定，模型文字不算。

## 對原 8 點提案要改的地方

- 狀態集合改用 rag 的 `AnswerStatus` 加 `ErrorCode`，不另造 `ok`、`denied`、`stale` 這組名稱。
- 回覆欄位改用 rag 的 `evidence_id`、`revision`、`text_sha256`、`observed_at`、`expires_at`，不用提案自擬的 `source_uri`、`content_hash`、`retrieved_at`。
- 撤回 `idempotency_key`：rag 查詢的去重是內部 `AttemptKey`，我讀到的範圍內沒有對外的查詢冪等鍵。
- 身分不進請求、取消方式、Schema 版本三項與 rag 設計一致，不改。

## 仍是缺口（要問對方）

- 入口 crate 名稱 `rag` 見於 schema.md，但 repo 還沒有程式；版本號、最低 Rust 版本、授權：未查證。
- Q3 還沒做，`ToolDescriptor`、`ToolResult` 的欄位和 JSON Schema 放哪裡：未定。
- 執行時需要 PostgreSQL（pgvector）和一個 SQLite 檔，且同一時間只有一個擁有者程序能寫。Agent 以唯讀開庫，還是另有擁有者程序，沒決定。
- `max_cost_micro_usd` 預設 0；Agent 是否固定傳 0，待確認。
- 撤權後 Agent 已持有的片段怎麼處理：rag 只在讀取時核權，Agent 每次使用引用前要再 `rag_read` 一次，這點待對方確認。
- 實際查詢品質與延遲：rag 沒有程式，無數據。

## 下一步

對方逐條回覆後，把上面「推得」的項目改成「已同意」並記入 P05a 的 K1；沒回覆的不當成同意。
