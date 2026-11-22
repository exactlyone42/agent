# 變異測試既有債務

變異測試會故意改壞程式一小處，看測試能不能發現；改壞了卻沒被發現的叫「漏抓」。本檔記錄 M3 開工前就存在的漏抓，不在 M3 修；每項都要有人認領或證明等價後才能刪。等價改動的說明在 `mutation-equivalents.md`。

## 怎麼確認是既有的

2026-11-18 用乾淨的 HEAD（6258dfc）工作樹跑同樣的變異測試：38 個漏抓在 HEAD 上全部同樣漏抓（其中 `structure_violations.rs` 在 HEAD 是第 48 行，M3 之後是第 51 行，同一處）。M3 新增與修改的檔案另外單獨跑變異（75 個，57 抓到、15 不可編譯、3 漏抓都在 `Capability::id`，已補測試，重跑 19 個全抓到），沒有新增漏抓。證據在 `.runtime/logs/m3-baseline-mutation.log`、`m3-baseline4-mutation.log`、`m3-new-mutation.log`、`m3-ids-mutation.log`。

## 三類共 38 項

| 類別                                 | 數量 | 位置                                                                                                                                                                               | 目前判斷                                                                                      |
| ------------------------------------ | ---- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| 只在 Windows 編譯的程式              | 13   | `file_check/input/windows.rs`（12 項）、`file_check/single_link/windows.rs`（1 項）                                                                                                | Linux 永遠碰不到，必在 Windows 實機驗；現行排除規則用行號，行號已偏移，規則失效               |
| 已有等價說明但行號偏移，或未重新證明 | 14   | `store/task_store.rs`（9 項）、`store/approval_limits.rs`（2 項，說明裡沒有）、`store/approvals.rs`（2 項）、`approval/gate.rs`（1 項）                                            | 行號偏移讓 `.cargo/mutants.toml` 的排除不再對到；是否真的等價沒有重新逐項證明，不得當作已解決 |
| 沒有排除、是真的測試缺口             | 11   | `cli/agent_platform.rs`（事件指令分支）、三個 CLI 文字檔（6 項，替換成空字串沒被測試發現）、`xtask` 的 `read_tracked.rs`、`run_structure.rs`、`structure_violations.rs`、`task.rs` | 需補測試                                                                                      |

## 排除設定的問題

- `.cargo/mutants.toml` 與 `crates/xtask/src/run/tool_arguments.rs` 的 `AGENT_EXCLUDE_PATTERN` 都用「行號」比對；程式一動行號就偏，排除就悄悄失效，也可能誤排新程式碼。
- 修法（另開票，不在 M3）：改用函式名稱比對，再逐項重新證明等價。

## M3 驗收怎麼寫

M3 的驗收看「相對 HEAD 有沒有新退化」，不看漏抓總數。目前結果：沒有新退化，但 `cargo xtask all` 的變異步驟仍因上述既有債務失敗，所以不得寫成 `cargo xtask all` 通過。
