# 接手說明

盤點時間：2026-11-10 15:00（Asia/Taipei）。接手目錄：`/home/cash/work_space/private/agent`，分支 `feature/plan-tooling`。這裡是唯一有未提交工作的現場；`/home/cash/orca/workspaces/agent-platform/flounder`（分支 `DnaBoss/chore-recall-handoff`）是乾淨的 develop 副本，沒有任何本次工作，不要在那裡接手。桌面 ADE 在另一個 repo `agent-ade`，不在此盤點。

## 目標與現況

目標：企業可交付的 Agent，先 CLI、再桌面，只在 Linux 實機驗證；Windows、macOS 只做相容設計（使用者 2026-10-31）。現在 CLI 已有 `run`、`status`、`events`、`approvals`、`approve`、`deny`、`version`、`list`、`result`、`events --follow`、`resume`、`cancel` 的程式與測試，真 Gateway 上跑過一次中斷後接續的真任務。整體仍沒有通過整合驗收，不能當產品交付（`README.md`）。

## 整體工作量與進度

總範圍 = 本 repo 負責的 S00、P01–P08、E01、P02m、P10；不含 P09（桌面 ADE，在 `agent-ade`）。估算日期 2026-11-10，依「實作＋整合＋驗證」相對工作量估算，不是工時；不確定性高（±5 個百分點以上）。已實作但完整檢查未過、或只有真人／正式環境才能驗的算剩餘。

重估說明：2026-10-30 前次估計是完成 12%、剩餘 88%。母檔 1.3.1 版改了範圍：新增 P02m（MCP 工具接入），P05b 因 RAG 本體移到獨立 rag repo 而作廢，P05a 只剩接口整合。因此權重改動如下（合計仍 100%）：P05a 7→4、P05b 4→0、新增 P02m 7、其餘權重不變。完成量之所以從 12% 升到 24%，是 P01、P02 前四項、P04 前段與 S00 收尾真的做完，不是縮小分母。

| 項目 | 權重 | 已完成 | 剩餘 | 證據與阻擋點 |
| --- | --- | --- | --- | --- |
| S00 品質底座 | 5% | 5% | 0% | 母檔標 DONE；`xtask render-plan` 取代 Python 已完成 |
| P01 核心迴圈與批准紀錄 | 12% | 10%：H1–H9 全 DONE（壞參數回報模型、呼叫結果配對、取消後零寫入、雙程序與強制終止） | 2%：最新版完整檢查未過 | 完整檢查 58847 退出 1，41 種改法漏抓，已補 5，餘 36 未補（`.runtime/p04-resume-core/full-all-final.json`） |
| P02 CLI 連真 Gateway | 14% | 6%：C1 version、C2 list、C3 result、C4 events --follow 皆 DONE | 8%：C5、C7 待完整檢查；C6 正式路由與虛擬金鑰未驗；C8 評測入口未做；C9 真人驗收 | 隔離真 Gateway 已跑通，不等於正式 Gateway |
| E01 身分、隔離、稽核、秘密 | 10% | 0% | 10% | 未開始 |
| P02m MCP 工具接入 | 7% | 0% | 7% | 未開始，依賴 P02、E01 |
| P03a Linux 安裝（預覽） | 5% | 0% | 5% | 未開始 |
| P03b 完整安裝整合 | 6% | 0% | 6% | 依賴 P05a |
| P04 接續與跨程序取消 | 9% | 3%：R1 接續在真 Gateway 通過（兩次 SIGKILL 後第 4 次接續，成果寫一次） | 6%：R2 卡住執行者的安全終止與確認後 10 秒零寫入；R3 instruct、R4 單一擁有者、R5 失效處理、R6 試用 | `.runtime/p04-resume-gateway-20261110/acceptance-summary.json` |
| P05a 外部 RAG 接口整合 | 4% | 0% | 4% | 依賴 P04；RAG 本體在 rag repo |
| P06 長期記憶 | 6% | 0% | 6% | 未開始 |
| P07 電腦操作 | 8% | 0% | 8% | 未開始 |
| P08 整體評測 | 9% | 0% | 9% | 未開始 |
| P10 分次交接 | 5% | 0% | 5% | 等全部完成 |
| 合計 | 100% | 24% | 76% | |

24% 是工作量估算，不是功能覆蓋率或成功機率。P01、P02 的 DONE 沿用 LOOP 紀錄，本次未重跑；它們的完整檢查是在較舊版本通過，最新版沒有通過。

## 已完成與可打開的成果

- 入口：`cargo xtask gate`（快檢）與 `cargo xtask all`（含變異測試）；CLI 見 `docs/cli.md`。
- 實測（來源：`.runtime/` 內原件，本次未重跑）：真 Gateway 上 CLI 任務經兩次 SIGKILL 後接續成功；`cancel` 在等待批准與等待模型兩種情況能確認停止，確認後 10 秒無新寫入；`.runtime/p04-owner-stop-20261110/` 內卡住的執行者（SIGSTOP）現在 5 秒內被終止並確認，4 個取消測試與 1 個檔案同步中被殺的測試通過；快檢紀錄 `gate-r2.txt` 有 226 個通過、未見失敗，但沒有留退出碼。
- 限制：只有 Linux；真 Gateway 是隔離的本機副本；沒有部署；以上都在未提交的工作樹。

## 遺留與停點

| 項目 | 影響 | 停點與下一步 |
| --- | --- | --- |
| R2 卡住執行者終止 | R2 不能結案 | 07:47 之後的驗證（`p04-owner-stop-20261110/`，08:33 止）尚未寫入 checkpoint；LOOP 母檔還寫「SIGSTOP 反例失敗」。先重跑確認，再補 checkpoint 並更新母檔 |
| 完整檢查 36 種漏抓 | P01、P04 不能算品質通過 | 補測試後重跑 `cargo xtask all`；未全過不提交、推送、發布 |
| 10 個本機提交與 61 個修改檔、約 82 個新檔都沒推送 | 電腦壞了就丟失 | 無遠端分支 `origin/feature/plan-tooling`；推送需使用者同意流程與完整檢查 |
| C6 正式 Gateway 路由與虛擬金鑰 | A1 不能成立 | 原因無紀錄；隔離 Gateway 已跑通 |
| 卡住執行者之外：真 Gateway 取消、程序樹、安裝後驗證、R3–R6 | 全案範圍 | 保留，不縮小 |
| docker 容器 `apg-gateway-1` 反覆重啟 | 歸屬未查證 | 不是本任務明確啟動的，不碰；若要用請先問誰在管 |
| 階段 0 取捨表（`docs/harness-design.md`） | 已由使用者在 2026-11-01 確認，無遺留 | 無 |

## 從這裡接手

1. 在 `/home/cash/work_space/private/agent` 執行 `git status -sb`，應看到 `feature/plan-tooling` 與大量未提交修改；`git log --oneline origin/develop..HEAD` 應列 10 個提交。
2. 讀 `loops/agent-platform.loop.md` 的「目前狀態」，再讀 `loops/agent-platform/P04-resume.md` 最後兩節。
3. 重跑 R2 卡住反例：`cargo test -p agent-platform --test cli cancel_stalled`（命令未實跑）。成功：1 個測試通過。
4. 通過後用 `~/tools/cash-rule-checkpoint` 寫 checkpoint（不要手寫 jsonl），並把母檔「目前狀態」改成實際結果。
5. 再跑 `cargo xtask gate`；通過後補那 36 種漏抓，最後 `cargo xtask all`。

## 現場整理

- 已整理：本次只改 `HANDOFF.md`（重寫過期的 2026-10-30 版，路徑與進度已更新）。README 索引原本就連到 `HANDOFF.md`，沒有再改。
- 刻意保留：全部未提交修改與 `.runtime/` 證據，這是唯一寫入者的工作成果；`docs/framework-comparison.pdf`、`loops/agent-platform/P02m-mcp.md`、`crates/http/` 等未追蹤檔屬於本任務，未歸檔。
- 背景程序：本次查到沒有 cargo、rustc、xtask、mutants 程序在跑；`.runtime/` 最新檔 08:33，LOOP 鎖檔 08:14，之後約 6 小時無活動。有一個 codex 程序（pid 59760）的工作目錄是 `/home/cash/work_space/JobResearch`，與本任務無關。
- 舊版交接備份：`~/.local/state/recall/reports/agent-platform-20261110-1500/prior/`。
- 待授權：提交、推送、發布（使用者尚未要求）。

## 驗證方式

- 已跑：唯讀查證——讀 git 狀態、LOOP、子檔、`.runtime/` 輸出；確認 `task_session.rs:86` 的 clippy 錯誤（`gate.txt`）已不在程式內。
- 未跑：任何 cargo 命令（gate、all、測試）；真 Gateway；Windows、macOS 依決定不跑。
- 判準：`cargo xtask gate` 退出 0 且警告 0；`cargo xtask all` 的 summary 全過、無漏抓。

## 已決定與不要重做

- 桌面 ADE 放 `agent-ade`（使用者 2026-10-28）；完整檢查只在改核心或發版前跑（2026-10-29）；實機只做 Linux（2026-10-31）。
- RAG 本體在獨立 rag repo，本 repo 只做接口（P05b 作廢）。
- Qwen 排序意見只採「可靠停止優先」，不採縮需求改 Beta。
- 失敗的前置（缺測試依賴、基準套件選錯、fixture 設錯）已保留在 `.runtime/`，不當產品陰性證據，也不重做。

## 查證資料

- Git：`origin/develop` 為 `0024b43`；本工作樹 HEAD `e2d8cce`，領先 10 個提交，無遠端分支。flounder 副本在 `0024b43`，乾淨。
- PR：#1、#2 已合；之後無新 PR。CI、部署：未查證／無部署目標。
- 真人驗收：無（C9 在等人）。
