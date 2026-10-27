# Agent Platform 工作交接

盤點時間：2026-10-27 13:45（Asia/Taipei），範圍是 `/home/cash/work_space/private/agent-platform`，分支 `feature/agent-plan`。本次只讀現場並改寫本檔，沒有提交、推送、部署、刪除，也沒有停掉任何程序。上午版交接已存到 `~/.local/state/recall/reports/agent-platform-20261027/HANDOFF.previous.md`。

## 目標與現況

目標是企業可交付的 Agent：先做命令列（CLI），再做 Tauri 桌面程式，共用 Rust 核心，Windows、macOS、Linux 都要交付。

現況：P01 進行中（`docs/plan-state.json`），P02 到 P10 未開始。產品還不能交付；目前只有 Linux 上的 CLI 雛形與局部驗證。

## 已完成與可打開的成果

- 主計畫：`PLAN.md`（唯一現行），圖形在 `docs/PLAN.pdf`、`docs/PLAN.html`。
- CLI：`cargo build --locked` 後執行 `target/debug/agent-platform run task.json`，經本機模型端點讀寫文字檔，並由獨立核對確認成果；沒有成果時模型說完成也不算成功。說明見 `docs/cli.md`。
- 任務紀錄：SQLite 保存任務開始與結果，CLI 可用 status／events 查詢。批准流程另有證據在 `docs/evidence/P01/20261027-durable-approval/`，內含工具完成後資料庫忙碌的有界重試（兩秒窗口，不重送工具），故障注入三案例通過。
- 證據：`docs/evidence/P01/` 有 20 多個資料夾，逐項內容我沒有全部讀回。只讀了 `20261027-durable-approval/receipt-busy-validation.md` 與 `docs/progress.md` 的描述，其餘視為未查證。
- 限制：以上只在 Linux 驗證；Windows、macOS、桌面程式、真人驗收都沒做。本次沒有重跑測試（見下一節的原因），上午曾記錄 `cargo test --locked` 通過，現在的程式碼以後續的檢查結果為準。

## 遺留與停點

- 執行中：另一個工作者正在本目錄跑完整品質檢查（見「現場整理」）。它是單一寫入者；未結束前不要再開第二個寫入者或同時跑編譯，否則會互搶鎖。解除：檢查程序結束，輸出在 `.runtime/checks/20261027T051033Z-889b3491/`。
- P01 缺口（`docs/progress.md`、LOOP）：完整任務規格與批准、工具意圖同交易保存；可靠恢復（先辨識已生效動作，不盲重送）；資料目錄隔離與網路磁碟拒絕；Windows ACL；容量與斷電測試；新增程式的整檔變異與三平台門檻。
- 上一輪變異檢查 68 項：61 抓到、2 漏測、5 無法編譯；兩個漏測已補測並重驗，但完整同版重測尚待結果。
- Windows、macOS 實機資訊已詢問使用者，尚待回覆，不要重問；先做不依賴的工作。
- 全部修改都沒有提交（27 項：11 個修改檔，其餘為未追蹤）。換機或清理會遺失。提交須品質門檻全過，且沿用既有授權；門檻沒過不要提交。

## 從這裡接手

1. `cd /home/cash/work_space/private/agent-platform`，先查寫入者：用 `pgrep cargo-mutants` 看變異檢查是否還在跑，並讀 `.runtime/checks/active.json`（目前為 pid 1194044，啟動 2026-10-27T05:10Z）。還在跑就等它結束，不重複啟動。
2. 讀唯一 LOOP 的「目前狀態」與「唯一下一步」：`/home/cash/work_space/private/agent-pool-gateway-completion/loops/agent-platform-decisions.loop.md`。原 `.cache` 路徑已不存在，不要重建。
3. 檢查結束後讀結果，再依 LOOP 的下一步：把版本化任務規格、批准與工具意圖放進同一個 SQLite 交易，用真正 CLI 驗批准、拒絕、中斷。成功判準：`python3 scripts/test_cli.py` 的批准案例通過，且有陰性對照。
4. 每步證據放 `docs/evidence/P01/`，並更新 `docs/progress.md`。

## 現場整理

- 已整理：改寫本檔；`README.md` 已有入口；LOOP 路徑已確認為 gateway-completion 那份（存在，最後修改 12:41），上午版「路徑不一致」已解。
- 刻意保留：全部未提交修改、`.runtime/`（已被 Git 忽略）、`target/`（13G）、`node_modules`、`output/pdf`，都屬本任務或建置產物，未動。
- 程序：本目錄有 `cargo-mutants`（pid 1205021，已跑約 33 分鐘）及其測試用 cargo，父程序是 python3 pid 1194044（品質檢查入口）。其他 codex 程序在 Helios、JobResearch、line-ai-sandbox、Fluke、家目錄，與本任務無關，未動。我無法確認這些程序由哪個對話持有，只知道它們存在。
- 待授權：提交與推送；其餘無。

## 驗證方式

- 已做：讀回 Git 狀態、HEAD、遠端、LOOP、`docs/progress.md`、`plan-state.json`、批准證據之一、程序與工作目錄。
- 沒做：任何編譯或測試（避免干擾執行中的檢查）、真實模型端到端、Windows 與 macOS、桌面程式、真人驗收。

## 已決定與不要重做

- 主計畫只維護 `PLAN.md`，不另存日期版。
- 圖形交付用 HTML／CSS 轉 PDF，每頁不透明白底。
- Rig 固定在 `55cd481a2421fd5da468b5bd6afc6ac876883599`。
- 不縮成 MVP；局部測試或模型說完成都不算任務成功。
- 已完成並有證據的項目（如 safe-input、task-store）不重跑，除非程式碼動到。

## 查證資料

- Git：HEAD `4d6e54e`，與遠端 `feature/agent-plan` 相同；本地 27 項未提交。PR：無。CI：未查。
- 部署：無，正式環境未改。
- 真人驗收：無。
