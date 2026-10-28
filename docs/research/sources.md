# 現成專案與 Harness 研究

查證日：2026-10-24。以下固定 Git commit 與已讀檔案，不以星數當證據。原始碼已讀，尚未在本機建置或跑完整任務；採用建議須由 P01 實測確認。

## 大公司開源 Harness

OpenAI Codex 本身已是 Rust，優先試接 app-server；Google Gemini CLI、Microsoft Agent Framework、ByteDance DeerFlow 參考串流、恢復與長任務。Anthropic SDK 與文章可參考，但不是完整 Claude Code 核心原始碼。

產品 Harness 管模型、工具、授權與接續；評測 Harness 建測試環境並獨立判結果。P01 選前者，P02 開始建後者，不互相代替。

## 0xPlaygrounds/rig

Rust / MIT。直接使用組件候選；engine.rs處理模型、工具步驟與取消，不照搬其mod.rs命名。

固定版本：`55cd481a2421fd5da468b5bd6afc6ac876883599`。

- [已讀 LICENSE](https://github.com/0xPlaygrounds/rig/blob/55cd481a2421fd5da468b5bd6afc6ac876883599/LICENSE)
- [已讀 crates/rig-agent/src/agent/engine.rs](https://github.com/0xPlaygrounds/rig/blob/55cd481a2421fd5da468b5bd6afc6ac876883599/crates/rig-agent/src/agent/engine.rs)

## modelcontextprotocol/rust-sdk

Rust SDK候選。根LICENSE為MIT與Apache-2.0轉換期，文件CC-BY-4.0；不是無授權。client.rs含放棄請求的取消通知；P01須再核對選定crate的實際授權。

固定版本：`8aec3891dd07fc537b21c2eb2596235640de9025`。

- [已讀 LICENSE](https://github.com/modelcontextprotocol/rust-sdk/blob/8aec3891dd07fc537b21c2eb2596235640de9025/LICENSE)
- [已讀 crates/rmcp/Cargo.toml](https://github.com/modelcontextprotocol/rust-sdk/blob/8aec3891dd07fc537b21c2eb2596235640de9025/crates/rmcp/Cargo.toml)
- [已讀 crates/rmcp/src/service/client.rs](https://github.com/modelcontextprotocol/rust-sdk/blob/8aec3891dd07fc537b21c2eb2596235640de9025/crates/rmcp/src/service/client.rs)

## aaif-goose/goose

Rust / Apache-2.0。agent.rs接入PermissionManager與CancellationToken，參考授權與取消，不搬整套桌面。

固定版本：`c3bfc5f7b0e230180ba3164ec1ee84fd951eaf0d`。

- [已讀 LICENSE](https://github.com/aaif-goose/goose/blob/c3bfc5f7b0e230180ba3164ec1ee84fd951eaf0d/LICENSE)
- [已讀 crates/goose/src/agents/agent.rs](https://github.com/aaif-goose/goose/blob/c3bfc5f7b0e230180ba3164ec1ee84fd951eaf0d/crates/goose/src/agents/agent.rs)

## infiniflow/ragflow

Apache-2.0。本次讀到Go retrieval.go的不同檢索服務契約；不能沿用舊印象稱全是Python。只移植來源、權限及檢索邊界。

固定版本：`a55dccee4c211046b4e4d216a538bc60fae78dfd`。

- [已讀 LICENSE](https://github.com/infiniflow/ragflow/blob/a55dccee4c211046b4e4d216a538bc60fae78dfd/LICENSE)
- [已讀 internal/agent/runtime/retrieval.go](https://github.com/infiniflow/ragflow/blob/a55dccee4c211046b4e4d216a538bc60fae78dfd/internal/agent/runtime/retrieval.go)

## mem0ai/mem0

Python / Apache-2.0。main.py提供add/search及scope，抽取失敗拋錯。Rust版保留同意、來源、隔離，不搬全部儲存依賴。

固定版本：`2c81af3bbe27d9bccb09c772d583df9e9be72ee5`。

- [已讀 LICENSE](https://github.com/mem0ai/mem0/blob/2c81af3bbe27d9bccb09c772d583df9e9be72ee5/LICENSE)
- [已讀 mem0/memory/main.py](https://github.com/mem0ai/mem0/blob/2c81af3bbe27d9bccb09c772d583df9e9be72ee5/mem0/memory/main.py)

## promptfoo/promptfoo

TypeScript / MIT。assertions/index.ts處理型別化斷言及未知類型錯誤，作為Rust評測器參考。

固定版本：`c0b88a324175c60019cc80de1b2e32b4c849d4c3`。

- [已讀 LICENSE](https://github.com/promptfoo/promptfoo/blob/c0b88a324175c60019cc80de1b2e32b4c849d4c3/LICENSE)
- [已讀 src/assertions/index.ts](https://github.com/promptfoo/promptfoo/blob/c0b88a324175c60019cc80de1b2e32b4c849d4c3/src/assertions/index.ts)

## Shopify/toxiproxy

Go / MIT。timeout.go可斷開或持續阻塞連線。直接用測試容器，由Rust控制API，不重寫網路代理。

固定版本：`49299d8838525126e09c85ebe0ba89940a3261e5`。

- [已讀 LICENSE](https://github.com/Shopify/toxiproxy/blob/49299d8838525126e09c85ebe0ba89940a3261e5/LICENSE)
- [已讀 toxics/timeout.go](https://github.com/Shopify/toxiproxy/blob/49299d8838525126e09c85ebe0ba89940a3261e5/toxics/timeout.go)

## trycua/cua

根MIT，子套件另查。讀到cua-driver的macOS權限與程序要求；未實測Linux，不直接選定。作為現成Computer Use驅動候選。

固定版本：`f06f203291c6c5a12e7c9134ec6b1b918acaf5d1`。

- [已讀 LICENSE.md](https://github.com/trycua/cua/blob/f06f203291c6c5a12e7c9134ec6b1b918acaf5d1/LICENSE.md)
- [已讀 libs/cua-driver/README.md](https://github.com/trycua/cua/blob/f06f203291c6c5a12e7c9134ec6b1b918acaf5d1/libs/cua-driver/README.md)

## openai/codex

Rust / Apache-2.0。主要Harness試接候選；app-server供Client接會話與授權，turn.rs傳遞CancellationToken。優先程序介面，不依賴內部crate。

固定版本：`970111f6cb62cc02fe36a82ea7d71b37a2889c97`。

- [已讀 LICENSE](https://github.com/openai/codex/blob/970111f6cb62cc02fe36a82ea7d71b37a2889c97/LICENSE)
- [已讀 codex-rs/app-server/README.md](https://github.com/openai/codex/blob/970111f6cb62cc02fe36a82ea7d71b37a2889c97/codex-rs/app-server/README.md)
- [已讀 codex-rs/core/src/session/turn.rs](https://github.com/openai/codex/blob/970111f6cb62cc02fe36a82ea7d71b37a2889c97/codex-rs/core/src/session/turn.rs)

## google-gemini/gemini-cli

TypeScript / Apache-2.0。client.ts的sendMessageStream傳遞AbortSignal，參考串流及中止，不搬供應商綁定。

固定版本：`cd1b45c2db0a5ea7b7fc3b67c1ba5879601d6005`。

- [已讀 LICENSE](https://github.com/google-gemini/gemini-cli/blob/cd1b45c2db0a5ea7b7fc3b67c1ba5879601d6005/LICENSE)
- [已讀 packages/core/src/core/client.ts](https://github.com/google-gemini/gemini-cli/blob/cd1b45c2db0a5ea7b7fc3b67c1ba5879601d6005/packages/core/src/core/client.ts)

## microsoft/agent-framework

Python與.NET / MIT。_checkpoint.py保存工作流與graph_signature_hash，參考恢復時核對流程版本。

固定版本：`4982e4c61ec81cc4f9ba0cbdb5594ed530dbbcfe`。

- [已讀 LICENSE](https://github.com/microsoft/agent-framework/blob/4982e4c61ec81cc4f9ba0cbdb5594ed530dbbcfe/LICENSE)
- [已讀 README.md](https://github.com/microsoft/agent-framework/blob/4982e4c61ec81cc4f9ba0cbdb5594ed530dbbcfe/README.md)
- [已讀 python/packages/core/agent_framework/_workflows/_checkpoint.py](https://github.com/microsoft/agent-framework/blob/4982e4c61ec81cc4f9ba0cbdb5594ed530dbbcfe/python/packages/core/agent_framework/_workflows/_checkpoint.py)

## anthropics/claude-agent-sdk-python

Python SDK / MIT。subprocess_cli.py啟動CLI子程序。SDK開源不代表Claude Code核心可複製改成Rust；服務條款及模型成本另算。

固定版本：`0e89b298e771259378a6cd3734e00d323dff5703`。

- [已讀 LICENSE](https://github.com/anthropics/claude-agent-sdk-python/blob/0e89b298e771259378a6cd3734e00d323dff5703/LICENSE)
- [已讀 src/claude_agent_sdk/_internal/transport/subprocess_cli.py](https://github.com/anthropics/claude-agent-sdk-python/blob/0e89b298e771259378a6cd3734e00d323dff5703/src/claude_agent_sdk/_internal/transport/subprocess_cli.py)

## bytedance/deer-flow

Python / MIT。lead_agent/agent.py以create_agent組裝middleware及checkpoint模式，參考長任務與上下文，不移植多agent全套。

固定版本：`9a11f9aec71b754ffe392b47a4950bfbce669454`。

- [已讀 LICENSE](https://github.com/bytedance/deer-flow/blob/9a11f9aec71b754ffe392b47a4950bfbce669454/LICENSE)
- [已讀 backend/packages/harness/deerflow/agents/lead_agent/agent.py](https://github.com/bytedance/deer-flow/blob/9a11f9aec71b754ffe392b47a4950bfbce669454/backend/packages/harness/deerflow/agents/lead_agent/agent.py)

## google/adk-python

Python / Apache-2.0。agent_evaluator.py讀資料集並透過eval_service評量，參考評測接法，環境結果與工具軌跡分開。

固定版本：`25d191710985371dfb93a131f2fa797734850952`。

- [已讀 LICENSE](https://github.com/google/adk-python/blob/25d191710985371dfb93a131f2fa797734850952/LICENSE)
- [已讀 src/google/adk/evaluation/agent_evaluator.py](https://github.com/google/adk-python/blob/25d191710985371dfb93a131f2fa797734850952/src/google/adk/evaluation/agent_evaluator.py)

## 移植與授權

本次未複製第三方產品程式，只有來源與分析。翻成 Rust 仍需保留來源 commit、檔案、原授權及必要 NOTICE，註明修改；換語言不免除授權。子套件、資料集、模型權重與雲端 API 必須分別核對；付費託管版不因程式開源而免費。

## 一手資料

- [Cargo workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)：多 crate 共用 lock 與 target。
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)：桌面環境依賴。
- [Anthropic 長任務 Harness](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents)：跨上下文保存進度及逐段驗收；文章不是完整核心程式。
- [OSWorld](https://arxiv.org/abs/2404.07972)：真環境結果驗證。
- [τ-bench](https://arxiv.org/abs/2406.12045)：工具結果與重複可靠性。

## 同題試接決策

P01 兩候選都跑：假模型入口、讀寫工具、拒絕授權、串流中止、重開接續、Gateway 協定及離線評測。每格記 pass/fail/not_run 與證據，任何安全要求失敗都不可選。優先 Codex 現成程序；若多模型/工具/授權契約不可行，再用自寫 agent-harness 補最小核心（2026-10-28 起 Rig 已移除）。只維護一套主迴圈，不因大公司開源就免驗收。
