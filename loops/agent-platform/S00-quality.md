# S00 品質底座與設計規則落地

母檔：../agent-platform.loop.md。一輪只做一個子任務；完成要附命令輸出。

## 目標

檢查入口（`cargo xtask gate`、`all`）真的能跑過，現有程式符合新設計規則（巢狀最多兩層、少用 else、一檔一函數），測試與檢查工具不再用 Python。

## 子任務

| id | 做什麼 | 驗收 | 狀態 |
| --- | --- | --- | --- |
| Q1 | 使用者跑 `cargo xtask gate`，確認巢狀 17 處已清、無新警告 | gate 全綠；再跑一次原指令警告數為 0 | DONE（gate 結束碼 0、警告 0、76 測試過 0 失敗；我自己跑，cargo 已可用） |
| Q2 | 修 gate 揭露的編譯或測試錯誤（這次 7 檔改動未編譯過） | 全工作區測試通過 | DONE（gate 無錯誤，無需修） |
| Q3 | 動到的舊檔依「一檔一函數」拆開（agent.rs 的 finish、approvals.rs 自由函式等） | gate 通過、行為測試不變 | DONE（finish、digest、append、close_intents 各一檔；gate 0 警告，76 測試過） |
| Q4 | 補變異漏抓：store/approvals.rs:188,317、approval_gate.rs、openai/agent_harness_openai.rs:41,130、cli main.rs:243、agent/approval.rs:26 | 變異測試修改檔 100%，等價者有可重現證據 | DONE（逾時例外已重驗：approvals.rs 132:38、207:9、261:19 皆被抓到，0 逾時；證據：/tmp/mut6） |
| Q5 | 跑 `cargo xtask all` 到過 | summary.json 全過，列工具版本、總變異數、未跑項 | RUNNING（第一輪 396 個變異：316 抓到、68 無法編譯、10 漏、2 逾時，未過；漏的 10 個已補測試或排除，針對性重跑 0 漏；Q6 已完成，整套 xtask all 於 23:15 重跑中（日誌 /tmp/all.log）；證據：.runtime/checks/1793281037-1039683、/tmp/mut4） |
| Q6 | `test_cli.py`（765 行）改 Rust 整合測試 | 同案例在 Rust 通過；陰性對照失敗；刪舊檔；每案加逾時並重驗 Q4 的兩個逾時變異 | DONE（16 案全移植，舊 Python 與 relative 目錄已刪，案例時限 60 秒、子案例遇錯即停；快檢結束碼 0） |
| Q7 | `render_plan.py` 與 `codex_file_contract.py` 改 Rust 或移除 | PLAN.pdf 重出、白底檢查 65535 | PENDING |
| Q8 | 清殘留：docs/evidence、crates/cli/relative、docs/PLAN.html、scripts/__pycache__（使用者刪）；找出寫 `relative/path` 的測試改用暫存目錄 | git status 乾淨 | PENDING |
| Q9 | prettier、提交 feature/agent-plan、推送、開 PR 到 develop | PR 檢查通過；自己的 PR 可合 | PENDING |

## 完成條件

Q1–Q9 全 DONE，`all` 通過，PR 已合 develop。
