# 現成專案與 Harness 研究

查證日：2026-10-24。以下每個專案都固定在某個 Git commit（程式碼的某一版），並列出實際讀過的檔案；不拿 GitHub 星數當證據。原始碼只讀過，尚未在本機建置或跑完整任務；採用建議須由 P01（PLAN.md 的核心迴圈階段）實測確認。各節開頭的「語言 / 授權」，如「Rust / MIT」，指程式語言與開源授權。

## 大公司開源 Harness

Harness 是包在語言模型外面的程式，讓模型能呼叫工具、取得授權並接續中斷的工作。OpenAI Codex 本身已是 Rust，優先試接它的 app-server（供外部程式連線的服務）；Google Gemini CLI、Microsoft Agent Framework、ByteDance DeerFlow 參考串流（邊產生邊回傳）、恢復與長任務。Anthropic SDK 與文章可參考，但不是完整 Claude Code 核心原始碼。

Harness 分兩種：產品 Harness 管模型、工具、授權與接續；評測 Harness 建測試環境並獨立判結果。P01 選前者，P02 開始建後者，不互相代替。

## 0xPlaygrounds/rig

Rust / MIT。2026-10-28 起已從本案移除，核心改用自寫的 `crates/harness`；下列只保留當時的調查證據，不是現行候選。

固定版本：`55cd481a2421fd5da468b5bd6afc6ac876883599`。

- [已讀 LICENSE](https://github.com/0xPlaygrounds/rig/blob/55cd481a2421fd5da468b5bd6afc6ac876883599/LICENSE)
- [已讀 crates/rig-agent/src/agent/engine.rs](https://github.com/0xPlaygrounds/rig/blob/55cd481a2421fd5da468b5bd6afc6ac876883599/crates/rig-agent/src/agent/engine.rs)

## modelcontextprotocol/rust-sdk

MCP（讓外部程式提供工具給 Agent 的標準協定）的 Rust SDK（開發套件）候選。根目錄 LICENSE 為 MIT 與 Apache-2.0 轉換期，文件為 CC-BY-4.0；不是無授權。client.rs 含放棄請求時送出的取消通知；P01 須再核對選定 crate 的實際授權。

固定版本：`8aec3891dd07fc537b21c2eb2596235640de9025`。

- [已讀 LICENSE](https://github.com/modelcontextprotocol/rust-sdk/blob/8aec3891dd07fc537b21c2eb2596235640de9025/LICENSE)
- [已讀 crates/rmcp/Cargo.toml](https://github.com/modelcontextprotocol/rust-sdk/blob/8aec3891dd07fc537b21c2eb2596235640de9025/crates/rmcp/Cargo.toml)
- [已讀 crates/rmcp/src/service/client.rs](https://github.com/modelcontextprotocol/rust-sdk/blob/8aec3891dd07fc537b21c2eb2596235640de9025/crates/rmcp/src/service/client.rs)

## aaif-goose/goose

Rust / Apache-2.0。agent.rs 把授權管理（PermissionManager）與取消訊號（CancellationToken）接進主流程；我們參考它的授權與取消做法，不搬整套桌面程式。

固定版本：`c3bfc5f7b0e230180ba3164ec1ee84fd951eaf0d`。

- [已讀 LICENSE](https://github.com/aaif-goose/goose/blob/c3bfc5f7b0e230180ba3164ec1ee84fd951eaf0d/LICENSE)
- [已讀 crates/goose/src/agents/agent.rs](https://github.com/aaif-goose/goose/blob/c3bfc5f7b0e230180ba3164ec1ee84fd951eaf0d/crates/goose/src/agents/agent.rs)

## infiniflow/ragflow

Apache-2.0。本次讀到 Go 語言寫的 retrieval.go，內容是不同檢索服務（先查來源文件、再讓模型回答）的介面約定；所以不能沿用舊印象，說它全是 Python。我們只移植來源、權限及檢索範圍的做法。

固定版本：`a55dccee4c211046b4e4d216a538bc60fae78dfd`。

- [已讀 LICENSE](https://github.com/infiniflow/ragflow/blob/a55dccee4c211046b4e4d216a538bc60fae78dfd/LICENSE)
- [已讀 internal/agent/runtime/retrieval.go](https://github.com/infiniflow/ragflow/blob/a55dccee4c211046b4e4d216a538bc60fae78dfd/internal/agent/runtime/retrieval.go)

## mem0ai/mem0

Python / Apache-2.0。main.py 提供新增（add）、搜尋（search）及限定範圍（scope），從對話抽取記憶失敗時會丟出錯誤。Rust 版保留使用者同意、來源標示、不同使用者之間的隔離，不搬全部儲存依賴。

固定版本：`2c81af3bbe27d9bccb09c772d583df9e9be72ee5`。

- [已讀 LICENSE](https://github.com/mem0ai/mem0/blob/2c81af3bbe27d9bccb09c772d583df9e9be72ee5/LICENSE)
- [已讀 mem0/memory/main.py](https://github.com/mem0ai/mem0/blob/2c81af3bbe27d9bccb09c772d583df9e9be72ee5/mem0/memory/main.py)

## promptfoo/promptfoo

TypeScript / MIT。assertions/index.ts 依型別處理斷言（對模型輸出的檢查條件），遇到未知類型就報錯；作為 Rust 評測器的參考。

固定版本：`c0b88a324175c60019cc80de1b2e32b4c849d4c3`。

- [已讀 LICENSE](https://github.com/promptfoo/promptfoo/blob/c0b88a324175c60019cc80de1b2e32b4c849d4c3/LICENSE)
- [已讀 src/assertions/index.ts](https://github.com/promptfoo/promptfoo/blob/c0b88a324175c60019cc80de1b2e32b4c849d4c3/src/assertions/index.ts)

## Shopify/toxiproxy

Go / MIT。toxiproxy 是故意製造網路故障的代理程式；timeout.go 能讓連線斷開或一直卡住。我們直接用它的測試容器，由 Rust 呼叫它的控制 API，不重寫網路代理。

固定版本：`49299d8838525126e09c85ebe0ba89940a3261e5`。

- [已讀 LICENSE](https://github.com/Shopify/toxiproxy/blob/49299d8838525126e09c85ebe0ba89940a3261e5/LICENSE)
- [已讀 toxics/timeout.go](https://github.com/Shopify/toxiproxy/blob/49299d8838525126e09c85ebe0ba89940a3261e5/toxics/timeout.go)

## trycua/cua

根目錄授權為 MIT，子套件另查。讀到 cua-driver 對 macOS 權限與程序的要求；Linux 未實測，不直接選定。它是現成 Computer Use（讓 AI 操作電腦畫面）驅動程式的候選。

固定版本：`f06f203291c6c5a12e7c9134ec6b1b918acaf5d1`。

- [已讀 LICENSE.md](https://github.com/trycua/cua/blob/f06f203291c6c5a12e7c9134ec6b1b918acaf5d1/LICENSE.md)
- [已讀 libs/cua-driver/README.md](https://github.com/trycua/cua/blob/f06f203291c6c5a12e7c9134ec6b1b918acaf5d1/libs/cua-driver/README.md)

## openai/codex

Rust / Apache-2.0。主要的 Harness 試接候選。app-server 讓用戶端程式接上對話（會話）與授權，turn.rs 把取消訊號（CancellationToken）一路傳下去。優先用它的程序介面（另開程序、靠訊息溝通），不依賴它的內部 crate。

固定版本：`970111f6cb62cc02fe36a82ea7d71b37a2889c97`。

- [已讀 LICENSE](https://github.com/openai/codex/blob/970111f6cb62cc02fe36a82ea7d71b37a2889c97/LICENSE)
- [已讀 codex-rs/app-server/README.md](https://github.com/openai/codex/blob/970111f6cb62cc02fe36a82ea7d71b37a2889c97/codex-rs/app-server/README.md)
- [已讀 codex-rs/core/src/session/turn.rs](https://github.com/openai/codex/blob/970111f6cb62cc02fe36a82ea7d71b37a2889c97/codex-rs/core/src/session/turn.rs)

## google-gemini/gemini-cli

TypeScript / Apache-2.0。client.ts 的 sendMessageStream（串流送出訊息）把中止訊號（AbortSignal）傳下去；我們參考它的串流及中止做法，不搬它與特定模型供應商綁定的部分。

固定版本：`cd1b45c2db0a5ea7b7fc3b67c1ba5879601d6005`。

- [已讀 LICENSE](https://github.com/google-gemini/gemini-cli/blob/cd1b45c2db0a5ea7b7fc3b67c1ba5879601d6005/LICENSE)
- [已讀 packages/core/src/core/client.ts](https://github.com/google-gemini/gemini-cli/blob/cd1b45c2db0a5ea7b7fc3b67c1ba5879601d6005/packages/core/src/core/client.ts)

## microsoft/agent-framework

Python 與 .NET / MIT。_checkpoint.py 保存工作流（一連串步驟）的進度，並存 graph_signature_hash（流程結構算出的指紋值）；我們參考它在恢復時核對流程版本的做法。

固定版本：`4982e4c61ec81cc4f9ba0cbdb5594ed530dbbcfe`。

- [已讀 LICENSE](https://github.com/microsoft/agent-framework/blob/4982e4c61ec81cc4f9ba0cbdb5594ed530dbbcfe/LICENSE)
- [已讀 README.md](https://github.com/microsoft/agent-framework/blob/4982e4c61ec81cc4f9ba0cbdb5594ed530dbbcfe/README.md)
- [已讀 python/packages/core/agent_framework/_workflows/_checkpoint.py](https://github.com/microsoft/agent-framework/blob/4982e4c61ec81cc4f9ba0cbdb5594ed530dbbcfe/python/packages/core/agent_framework/_workflows/_checkpoint.py)

## anthropics/claude-agent-sdk-python

Python SDK / MIT。subprocess_cli.py 啟動命令列（CLI）子程序。SDK 開源，不代表 Claude Code 核心可以複製後改成 Rust；服務條款及模型費用另算。

固定版本：`0e89b298e771259378a6cd3734e00d323dff5703`。

- [已讀 LICENSE](https://github.com/anthropics/claude-agent-sdk-python/blob/0e89b298e771259378a6cd3734e00d323dff5703/LICENSE)
- [已讀 src/claude_agent_sdk/_internal/transport/subprocess_cli.py](https://github.com/anthropics/claude-agent-sdk-python/blob/0e89b298e771259378a6cd3734e00d323dff5703/src/claude_agent_sdk/_internal/transport/subprocess_cli.py)

## bytedance/deer-flow

Python / MIT。lead_agent/agent.py 用 create_agent 組裝 middleware（夾在模型呼叫前後的處理層）與 checkpoint（存檔點）模式；我們參考它處理長任務與上下文的做法，不移植它的多 Agent 全套。

固定版本：`9a11f9aec71b754ffe392b47a4950bfbce669454`。

- [已讀 LICENSE](https://github.com/bytedance/deer-flow/blob/9a11f9aec71b754ffe392b47a4950bfbce669454/LICENSE)
- [已讀 backend/packages/harness/deerflow/agents/lead_agent/agent.py](https://github.com/bytedance/deer-flow/blob/9a11f9aec71b754ffe392b47a4950bfbce669454/backend/packages/harness/deerflow/agents/lead_agent/agent.py)

## google/adk-python

Python / Apache-2.0。agent_evaluator.py 讀入測試資料集，透過 eval_service 評分；我們參考它的評測接法，它把「環境最後的結果」與「工具呼叫的過程（軌跡）」分開評。

固定版本：`25d191710985371dfb93a131f2fa797734850952`。

- [已讀 LICENSE](https://github.com/google/adk-python/blob/25d191710985371dfb93a131f2fa797734850952/LICENSE)
- [已讀 src/google/adk/evaluation/agent_evaluator.py](https://github.com/google/adk-python/blob/25d191710985371dfb93a131f2fa797734850952/src/google/adk/evaluation/agent_evaluator.py)

## 移植與授權

本次未複製第三方產品程式，只有來源與分析。翻成 Rust 仍需保留來源 commit、檔案、原授權及必要 NOTICE（授權要求隨附的聲明檔），並註明修改；換語言不免除授權。子套件、資料集、模型權重（訓練完成的模型參數檔）與雲端 API 必須分別核對；付費託管版不因程式開源而免費。

## 一手資料

- [Cargo workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)：多個 crate 共用同一份鎖定檔（lock）與編譯輸出資料夾（target）。
- [Anthropic 長任務 Harness](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)：跨上下文保存進度及逐段驗收；文章不是完整核心程式。
- [OSWorld](https://arxiv.org/abs/2404.07972)：在真實電腦環境測 AI 的評測集，用真環境的結果驗證。
- [τ-bench](https://arxiv.org/abs/2406.12045)：測 Agent 使用工具的評測集，看工具結果是否正確，以及同題重複執行是否穩定。
- [Do LLM Agents Execute the Plans They Declare?](https://arxiv.org/abs/2609.38108)（2026-09-29）：只叫模型先寫計畫，執行時只有 22–45% 照計畫的結構走；由程式依計畫類型強制執行後成功率上升（ALFWorld 0.48 到 0.92，SWE-bench Verified 0.36 到 0.44）。只讀了摘要頁；沒測工程設計題。
- [Select-then-Solve](https://arxiv.org/abs/2604.06753)（2026-04-08）：四個模型、十個題庫比較六種推理方式，沒有一種全勝；「先計畫再執行」的平均分在四個模型上都低於想一步做一步（GPT-5 為 52.5 對 57.1）。每題只跑一次；沒測工程設計題。
- [AgentArch](https://arxiv.org/pdf/2509.10769v2)（2026-11-20 搜尋，只讀摘要）：企業流程 18 種配置，不同任務的最佳架構不同；大模型較穩。
- [ReflAct](https://arxiv.org/pdf/2505.15182)（2026-11-20 搜尋，只讀摘要）：強模型的「先計畫再做」與 ReAct 相近或稍好，弱模型較差；目標與狀態反思優於兩者。
- [Building effective agents](https://www.anthropic.com/research/building-effective-agents)（Anthropic，2024-12）：從最簡單做起；評估者加優化者只在有明確評分標準時划算。

## 同題試接決策

P01 要讓兩個候選（Codex app-server 與自寫 agent-harness；Rig 已於 2026-10-28 移除，由自寫取代）都跑同一組項目：假模型入口（用假模型代替真模型來測）、讀寫工具、拒絕授權、串流中止、重開接續、Gateway 協定（與模型閘道的通訊約定）及離線評測。每格（候選乘上項目）記 pass（通過）、fail（失敗）或 not_run（沒跑），並附證據；任何安全要求失敗的候選都不可選。Codex 這一邊的比較仍待做。只維護一套主迴圈（Agent 反覆呼叫模型與工具的核心流程），不因為是大公司開源就免驗收。
