# Agent Platform LOOP

Protocol version: `1.3.1`（細節見 `agent-platform/` 子檔）

## 目標

- 做出企業可交付的 Agent：本機 CLI（核心、工具、批准、驗證、任務紀錄檔）→ 公司 Gateway → 模型；桌面 ADE 在 agent-ade repo。實機驗證只做 Linux；macOS、Windows 只做相容設計。
- 依據：`docs/agent-target-architecture.pdf`、`docs/architecture.md`、`PLAN.md`。
- 不做：完整 IDE、純 Web、模型各家格式轉換（Gateway 負責）。

## 整體驗收

- [ ] A1 CLI 在 Linux 完成真實任務（連真 Gateway），成功只由驗證器判定；macOS、Windows 只做相容設計，不做實機驗證
- [ ] A2 拒絕、取消、崩潰重開都不擅自重做副作用，取消後零寫入
- [ ] A6 MCP 工具安裝、批准、取消與結果讀回過正反驗收
- [ ] A3 外部 RAG 接口與整合、記憶、電腦操作各過正反驗收
- [ ] A4 桌面 ADE 讀同一份事件契約並接續同任務
- [ ] A5 企業發布門檻與整體評測全過，未跑項目明列

## 計畫（子檔在 loops/agent-platform/）

| step | 大綱 | 子檔 | 依賴 | 狀態 |
| ---- | ---------------------------- | ---------------------- | ---------- | ------- |
| S00 | 品質底座與設計規則落地 | S00-quality.md | 無 | DONE |
| P01 | 核心迴圈與批准紀錄 | P01-core.md | S00 | VALIDATING |
| P02 | 可用 CLI，連真 Gateway | P02-cli.md | P01 | RUNNING |
| E01 | 身分政策、隔離、稽核、秘密 | E01-enterprise.md | P01 | PENDING |
| P02m | MCP 工具接入與自動安裝 | P02m-mcp.md | P02、E01 | PENDING |
| P03a | Linux 安裝（內部預覽） | P03a-install.md | P02m | PENDING |
| P04 | 接續、跨程序取消、單一擁有者 | P04-resume.md | P01核心、P02入口 | RUNNING |
| P05a | 外部 RAG 接口與整合 | P05a-knowledge.md | P04 | PENDING |
| P03b | 完整安裝整合 | P03b-install-full.md | P03a、P05a | PENDING |
| P05b | 專業格式需求移交 RAG | P05b-knowledge-wide.md | 外部 rag | SUPERSEDED |
| P06 | 長期記憶 | P06-memory.md | P04 | PENDING |
| P07 | 電腦操作 | P07-computer.md | P04、E01 | PENDING |
| P09 | 桌面 ADE（agent-ade repo） | P09-ade.md | P04 | PENDING |
| P08 | 整體評測（最小版在 P02 建） | P08-eval.md | P02 | PENDING |
| P10 | 分次交接 | P10-handoff.md | 全部 | PENDING |

## 順序

1. P01：可靠任務核心。完整歷史落盤，真雙程序及強制終止不重做；避免後續所有能力在中斷後失去可信狀態。
2. P02：可驗證的真實任務。原 CLI 經真 Gateway 完成連結修復、逐次批准與獨立讀回；拒絕、取消、假成功各有反例。
3. P05a：只做外部 RAG 接口與整合驗收；RAG 本體在獨立 rag repo 開發。查詢、原文來源與版本、失效／撤權、取消及錯誤須與對口確認，不實作索引或格式處理。

P05a 需 P04 恢復與 E01 權限，按需接 P02m；完整安裝及發布門檻保留。

後續功能共用保存、工具閘門及事件契約。

## 每輪優先檢討（使用者 2026-11-05 定）

每輪結束先列下一輪工作、對應核心成果、必要性、成本／時間與已知缺口，交給 `qwen3.8:27b` 檢查：是否偏離核心？在預算內值得現在做嗎？急需上線時有沒有更重要的事？具體預算未知就明列未知。保存提案、回覆、採納／不採納理由與調整順序，寫 checkpoint 後再執行下一輪。意見不替代實測或降低品質。模型失敗記實際原因，不假裝審過、不盲重試。

## 交付點

- 內部步驟：S00。階段交付：P02（CLI 初版）、P03b（完整安裝）、P09（桌面）、P10。
- 無部署目標前不建 delivery.json；有發布包時再補，部署前須過 P08 與企業門檻。

## 目前狀態

- 唯一writer保留dirty，未提交推送，本輪未動正式；預算交期未知。
- R1 VALIDATING：真Gateway兩次SIGKILL後第4次接續成功，成功只寫一次不重寫；2任務1成功1失敗，兩自有容器已停。原件.runtime/p04-resume-gateway-20261110/acceptance-summary.json。
- 原all退出1：41漏抓，後補5，餘36未複驗；未全過不提交推送發布。
- R2 RUNNING：已接cancel及清單/cancel 1，原CLI等待批准／模型取消後十秒無新寫入，接續仍需新批；缺執行者不假確認。有未明派工保留unknown。DB升7保舊列，事件4／新任務3，舊任務首次新狀態同交易升3。
- 交錯反例已修並抓到錯世代反轉。原件.runtime/p04-cross-cancel-20261110/。
- 卡住執行者：cancel_stalled現過（5秒終止確認、之後10秒無寫入），停用SIGKILL陰性對照101。原件.runtime/p04-stalled-20261110/。真Gateway取消、程序樹未驗，新版all未跑。
- R3切片1–4：instruct收下、去重（DB升8），下回合前交模型並標applied，取消不注入；待處理上限16；status加pending_instructions。具名測試過、改錯7種失敗；切片3快檢231過，切片4快檢2次未過。原件.runtime/p04-pending-limit-20261110/。
- BLOCKED：cancel_stalled單獨跑8次敗3次（EACCES，與切片4無關，all基準同），成因未明。
- 唯一下一步：查EACCES（task_process.rs），修好重跑快檢；真Gateway取消、程序樹、36種漏抓、R4–R6保留。
