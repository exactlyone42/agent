# 已完成與停點

2026-10-27：P01 進行中，P02–P10 尚未完成。目標仍是先正式 CLI、後 Tauri Desktop，Windows、macOS、Linux 全部交付。目前只有 Linux 開發與局部驗證證據，尚無正式產品。

## 已實作與實測

CLI 經本機 HTTP 模型介面執行明確授權的文字讀寫，拒絕覆寫既有檔案並獨立核對成果；未提供核對條件時回報待驗收。使用方式及限制見 [CLI 開發入口](cli.md)。早期曾用 Codex 與 Rig 做合成檔案比較，Rig 已於 2026-10-28 移除、改自寫 agent-harness，同題比較須用新核心重做；模型宣稱完成仍可能沒有正確成果，不能直接算任務成功。

已接 SQLite 的任務開始與結果紀錄，CLI 可用 status／events 在結束後查詢。Linux 實際 CLI 測試涵蓋正常完成、取消、SIGKILL、結果保存失敗；資料層測試涵蓋交易回滾、競爭寫入、過時序號拒絕及唯讀查詢。原件與未完成邊界見 [任務紀錄審查](evidence/P01/20261027-task-store/decisions.md)。這不是完整 TaskSpec、持久授權或自動接續，不能將 running 當作程序存活證明。

格式、靜態檢查與 workspace 測試曾通過。新增測試涵蓋核心長度、來源大小、命名管線拒絕、來源執行途中變大及 Linux 開檔時換成目錄；最後一項只驗隔離臨時資料，不代表完整路徑隔離。Claude Code 審查與處理見 [審查紀錄](evidence/P01/20261026-cli-review/decisions.md)。

## 品質證據及缺口

完整第二輪故意改壞程式的檢查共有 68 項：61 項被攔住、2 項漏測、5 項無法編譯，見 [原始結果](evidence/P01/20261026-cli-core-boundaries/mutants.out/outcomes.json)。兩個漏測已補案例：來源變大重驗 2 項全抓到，路徑替換重驗 1 項抓到；正常版本全 workspace 測試亦通過。原件見 evidence/P01/20261027-cli-source-growth 與 20261027-cli-path-replacement；完整同版重測仍待執行，不將原始失敗改成通過。先前局部函式的滿分與行覆蓋，不代表整個產品品質。

完整品質入口已實作並實跑，但新增程式尚未通過全部整檔變異與跨平台門檻。仍缺完整任務與授權保存、可靠接續、工具隔離、安裝、三塊 RAG、記憶、電腦操作、Tauri、三平台實機及真人簽收。上述均未完成企業驗收；局部本機模型與取消測試不替代整體驗收。

## 接續入口

唯一任務沿用 `/home/cash/work_space/private/agent-pool-gateway-completion/loops/agent-platform-decisions.loop.md`，見 [長任務接手](longrun.md)。原 `.cache` 路徑已失效，不新建第二份。原生 goal 接續；舊 Orca 排程先前已讀回停用，接手仍須核對目前程序及鎖。

下一步沿用同一 SQLite 資料庫，接完整任務規格、批准與工具意圖的原子保存，再做可靠恢復；資料目錄隔離與網路磁碟拒絕亦須補齊。未提交、推送或部署，正式環境未改。提交仍須通過品質門檻，不把未完成項目改成產品完成。
