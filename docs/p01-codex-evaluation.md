# P01：Codex 工具入口驗證

2026-11-01 用本機 `codex-cli 0.164.0`，以本機假模型及合成資料測試，沒有呼叫付費模型。Codex 原始碼授權與固定版本見 [來源研究](sources.md#openaicodex)。

| 檢查                 | 結果與證據                                                                                                                                                                                      |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 接假模型及自訂工具   | 通過：假模型要求讀取固定文字，客戶端收到呼叫並回傳，第二次模型請求收到該文字；[執行結果](evidence/p01-h6/manifest.json)、[往返紀錄](evidence/p01-h6/transcript.json)。                          |
| 命令啟動前批准       | 通過：`cat` 啟動前發出批准請求，測試給予批准；[紀錄](evidence/p01-h6/app-stdin-result.json)。                                                                                                   |
| 已啟動命令的後續輸入 | 未通過：設定工具前掛勾拒絕 `write_stdin`，但掛勾只看到啟動 `cat`，後續文字仍送進程序；[紀錄](evidence/p01-h6/cli-stdin-result.json)。另一組以人工批准模式執行時，後續輸入也沒有第二次批准請求。 |
| 關閉互動執行入口     | `features.unified_exec = false` 後，模型仍看到 `exec_command` 與 `write_stdin`；[工具清單](evidence/p01-h6/disabled-unified-result.json)。                                                      |

因此淘汰本次以原版 Codex CLI／app-server 和上述設定作為平台主迴圈的方案：平台要求逐次攔截工具入口，而已批准的互動程序仍可接收後續模型輸入，不能靠首次批准或工具前掛勾保證逐次審核。這不證明修改 Codex 原始碼或另設模型代理也做不到；這些方案未驗證。本次沒有進行同題功能比較，也未驗取消、重啟與真 Gateway。

[官方掛勾文件](https://developers.openai.com/codex/hooks)亦說明 `write_stdin` 不會再次執行工具前掛勾，託管工具也不走該掛勾。官方文件是補充，以上結論依本機實測。

重跑：`python3 docs/evidence/p01-h6/dynamic-tool-probe.py`、`python3 docs/evidence/p01-h6/cli-stdin-probe.py stdin`、`python3 docs/evidence/p01-h6/app-stdin-probe.py`。測試使用獨立暫存目錄與本機迴圈位址；輸入是無副作用的固定文字。
