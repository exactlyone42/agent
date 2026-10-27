# 實際全套執行發現的報告覆寫

20261027T033924Z-bff7679f/coverage.json 只有 command、started、exit_code、finished，沒有 LLVM data 欄位。run_step 將執行紀錄命名成 coverage.json，覆寫同名 LLVM 報告。先前「覆蓋率命令成功」僅指退出碼，不能宣稱覆蓋數據已保存。

待原變異程序結束後，把 LLVM 原件改存 coverage-report.json，validate_reports 同步讀新路徑；加執行紀錄與產物共存的回歸測試。修正後利用現有覆蓋率執行檔重新匯出，不重跑沒有變更的產品測試；若工具無法保留資料，才重跑必要覆蓋率。保留本輪失敗，不改寫舊結果。
