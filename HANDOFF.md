# 接手說明

盤點時間：2026-10-30 16:00（Asia/Taipei）。範圍：本 repo（`/home/cash/work_space/private/agent-platform`）的程式、文件、分支與程序；桌面 ADE 在另一個 repo `agent-ade`，不在此盤點。

## 目標與現況

目標：企業可交付的 Agent，先 CLI、再桌面，只在 Linux 實機驗證；Windows、macOS 只做相容設計（使用者 2026-10-31 改）。現在只有 CLI 與核心的雛形，在 Linux 上通過快檢與變異測試；沒有任何一項通過整合驗收，不能當產品交付（見 `README.md`、`PLAN.md`「指令清單與進度」）。

## 整體工作量與進度

總範圍 = 本 repo 負責的 S00、P01–P08、E01、P10。不含 P09（桌面 ADE，在 `agent-ade`，只算契約介面）。

估算日期 2026-10-30，依「實作＋整合＋驗證」相對工作量估算，不是工時；不確定性高（±5 個百分點以上）。程式寫好但沒驗證的算剩餘。

| 項目 | 權重 | 已完成 | 剩餘 | 證據與阻擋點 |
| --- | --- | --- | --- | --- |
| S00 品質底座 | 5% | 4%：gate／all 入口、巢狀與一檔一函數、變異漏抓、test_cli 改 Rust | 1%：`xtask render-plan` 已完成，剩提交 | `xtask all` 通過（391 變異，323 抓到、68 無法編譯、0 漏、0 逾時）；Q7 未開始 |
| P01 核心迴圈與批准紀錄 | 12% | 5%：批准閘門、持久批准、SQLite 任務紀錄 | 7%：壞工具參數改回報模型、呼叫與結果配對、停止原因、兩程序搶同任務與崩潰恢復的證據 | 程式與單元測試在；`PLAN.md` 寫明兩項證據不足 |
| P02 CLI 連真 Gateway | 14% | 3%：六個指令有程式、Rust 整合測試 | 11%：`version`、`list`、`cancel`、`resume`、`instruct`、`result`、持續事件輸出；連真 Gateway | Gateway 連線只對假伺服器測過 |
| E01 身分、隔離、稽核、秘密 | 10% | 0% | 10% | 未開始 |
| P03a Linux 安裝（預覽） | 5% | 0% | 5% | Windows、macOS 不實測，只做相容設計 |
| P03b 完整安裝整合 | 6% | 0% | 6% | 依賴 P05a |
| P04 接續與跨程序取消 | 9% | 0% | 9% | 未開始 |
| P05a 知識檢索（常用格式） | 7% | 0% | 7% | 未開始 |
| P05b 知識檢索（大檔、專業格式） | 4% | 0% | 4% | 未開始 |
| P06 長期記憶 | 6% | 0% | 6% | 未開始 |
| P07 電腦操作 | 8% | 0% | 8% | 未開始 |
| P08 整體評測 | 9% | 0% | 9% | 未開始 |
| P10 分次交接 | 5% | 0% | 5% | 未開始 |
| 合計 | 100% | 12% | 88% | |

12% 是工作量估算，不是功能覆蓋率或成功機率。

## 已完成與可打開的成果

- 入口：`cargo xtask gate`（快檢）與 `cargo xtask all`（含變異測試）；CLI 見 `docs/cli.md`。
- 實測：`xtask all` 通過（來源：PR #2 紀錄，本次未重跑）。PR #2 已合 `develop`。
- 限制：只有 Linux；Gateway 只用假伺服器；沒有常駐服務，所以沒有部署。

## 遺留與停點

| 項目 | 影響 | 停點與下一步 |
| --- | --- | --- |
| Q7 `render_plan.py` 還是 Python | 違反「測試與檢查工具改 Rust」要求；PLAN.pdf 由它產生 | 已完成：xtask 的 `render-plan` 取代 Python（頁面內容在 `docs/plan-pages/`），PLAN.pdf 重出 38 頁，白底檢查通過；`codex_file_contract.py` 在 `scripts/` 查無 |
| `docs/harness-design.md:31` 寫「已有任務資料庫」 | 詞不精確：SQLite 存任務、事件與批准，不是場次 | 未改。建議改成「已有 SQLite 任務紀錄，場次存取要另做」，第 33 行一併對齊 |
| 階段 0 取捨表待使用者確認 | 擋住階段 1 | 見 `PLAN.md`「目前下一步」 |
| Windows、macOS 不做實機驗證 | CLI 的信號、管線、批准測試只在 Unix 跑 | 使用者 2026-10-31 決定只做相容設計；文件不得宣稱兩平台已支援 |

## 從這裡接手

1. 在 `/home/cash/work_space/private/agent-platform` 讀 `loops/agent-platform.loop.md` 的「目前狀態」，確認分支是 `feature/plan-tooling`（`git status -sb`）。成功：看到該分支與本檔的未提交修改。
2. S00 已全部完成；本輪未提交的修改先提交（分支 feature/plan-tooling），再進 P01。
3. 改 `docs/harness-design.md:31`、`:33` 的措詞。
4. Q7 完成後，S00 全 DONE，依母檔進 P01（`loops/agent-platform/P01-core.md`）。

## 現場整理

- 已整理：舊分支 `feature/agent-plan` 本地與遠端已刪；舊 session 記錄（超過一天沒動的 `.jsonl` 與 `.zst`）、空資料夾與舊附屬資料夾已刪（`~/.claude/projects`、`~/.codex/sessions`，非本 repo）。本次交接更新了 `README.md` 索引、母檔「目前狀態」與 `S00-quality.md` 的 Q5、Q8、Q9。
- 未提交修改：`HANDOFF.md`（新）、`README.md`、`loops/agent-platform.loop.md`、`loops/agent-platform/S00-quality.md`。用途是上述交接；舊版備份在 `~/.local/state/recall/reports/agent-platform-20261030-1600/before/`。
- 背景程序：本次未查到屬於此任務的背景程序（只看了 `loops/` 內的鎖檔存在；未逐一查進程）。
- 待授權：無。

## 驗證方式

- 已跑：本次只做唯讀查證與文件修改；未跑 gate／all，沿用 PR #2 的結果。
- 未跑：真 Gateway、gate（本次修改後）；Windows、macOS 依決定不跑。
- 判準：`cargo xtask gate` 結束碼 0 且警告 0；`cargo xtask all` 的 summary 全過。

## 已決定與不要重做

- 桌面 ADE 放 `agent-ade`，本 repo 只給 CLI、核心與事件契約（使用者 2026-10-28）。
- 完整檢查 `xtask all` 只在改核心邏輯或發版前跑（使用者 2026-10-29）。
- 先前「cargo 被快取掛勾擋」是誤判，實測 cargo 可用，不必再請使用者代跑。
- Q1–Q6、Q8、Q9 已完成，不用重做。

## 查證資料

- Git：`develop` 最新為 `0024b43`（PR #2 合併）；目前分支 `feature/plan-tooling`，與 `develop` 同點。
- PR：#2 已合。CI、部署：未查證／無部署目標。
- 真人驗收：無。
