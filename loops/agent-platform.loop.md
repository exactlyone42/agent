# Agent Platform LOOP

Protocol version: `1.2.0`（母檔只列大綱；細節在 `agent-platform/` 子檔，每輪只讀母檔加一個子檔）

## 目標

- 做出企業可交付的 Agent：本機 CLI（核心、工具、批准、驗證、任務紀錄檔）→ 公司 Gateway → 模型；桌面 ADE 在 agent-ade repo。實機驗證只做 Linux；macOS、Windows 只做相容設計（使用者 2026-10-31 定）。
- 依據：`docs/agent-target-architecture.pdf`、`docs/architecture.md`、`PLAN.md`。子任務全部完成並拼起來，就是最終目標。
- 不做：完整 IDE、純 Web、模型各家格式轉換（Gateway 負責）。

## 整體驗收

- [ ] A1 CLI 在 Linux 完成真實任務（連真 Gateway），成功只由驗證器判定；macOS、Windows 只做相容設計，不做實機驗證
- [ ] A2 拒絕、取消、崩潰重開都不擅自重做副作用，取消後零寫入
- [ ] A3 知識檢索、記憶、電腦操作各過正反驗收
- [ ] A4 桌面 ADE 讀同一份事件契約並接續同任務
- [ ] A5 企業發布門檻與整體評測全過，未跑項目明列

## 計畫（子檔在 loops/agent-platform/）

| step | 大綱 | 子檔 | 依賴 | 狀態 |
| --- | --- | --- | --- | --- |
| S00 | 品質底座與設計規則落地 | S00-quality.md | 無 | DONE |
| P01 | 核心迴圈與批准紀錄 | P01-core.md | S00 | RUNNING |
| P02 | 可用 CLI，連真 Gateway | P02-cli.md | P01 | PENDING |
| E01 | 身分政策、隔離、稽核、秘密 | E01-enterprise.md | P01 | PENDING |
| P03a | Linux 安裝（內部預覽） | P03a-install.md | P02 | PENDING |
| P04 | 接續、跨程序取消、單一擁有者 | P04-resume.md | P03a | PENDING |
| P05a | 知識檢索常用格式 | P05a-knowledge.md | P04 | PENDING |
| P03b | 完整安裝整合 | P03b-install-full.md | P05a | PENDING |
| P05b | 知識檢索大檔與專業格式 | P05b-knowledge-wide.md | P05a | PENDING |
| P06 | 長期記憶 | P06-memory.md | P04 | PENDING |
| P07 | 電腦操作 | P07-computer.md | P04 | PENDING |
| P09 | 桌面 ADE（agent-ade repo） | P09-ade.md | P04 | PENDING |
| P08 | 整體評測（最小版在 P02 建） | P08-eval.md | P02 | PENDING |
| P10 | 分次交接 | P10-handoff.md | 全部 | PENDING |

## 交付點

- 內部步驟：S00。階段交付：P02（CLI 初版）、P03b（完整安裝）、P09（桌面）、P10。
- 無部署目標前不建 delivery.json；有發布包時再補，部署前須過 P08 與企業門檻。

## 目前狀態

- task／step：agent-platform／P01（子檔 P01-core.md）
- 已證明（2026-10-31）：Q7 完成，`xtask render-plan` 產出 PLAN.pdf 38 頁、白底檢查通過，gate 通過。（2026-10-30 讀 git 紀錄與 PR #2）：`cargo xtask all` 通過（391 個變異，323 抓到、68 無法編譯、0 漏、0 逾時）；PR #2 已合 develop
- 未證明：真 Gateway（Windows／macOS 不做實機驗證，只做相容設計）
- 阻塞：無（先前「cargo 被擋」已查證為誤判）
- 下一步：階段 1A 的停止原因、取消逐筆結算及批准遭拒已實作，完整檢查通過；完成快檢與提交後，繼續 P01 尚未完成的整合驗收。CLI 取消歷史持久化待階段 1B。
- checkpoint JSONL：loops/agent-platform.checkpoints.jsonl（只經 ~/tools/cash-rule-checkpoint 寫）
- owner／排程：單一寫入者 feature/plan-tooling（feature/agent-plan 已合併並刪除）；/loop 動態續跑；COMPLETE、BLOCKED、需決策時停
