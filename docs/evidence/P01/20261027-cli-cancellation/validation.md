# 本版實際驗證

完整原件：`.runtime/checks/20261027T041611Z-22f99988/`。快檢、秘密掃描及覆蓋率命令成功，source_unchanged=true；完整門檻未過，原因與原始變異結果保留於 assessment.json、mutation/mutants.out/outcomes.json。

之前 `.runtime/checks/20261027T040935Z-f5252074/` 因測試等待已退出程序而產生真實 90 秒逾時，已針對唯一 cargo-mutants 程序送 SIGINT，正常結束且來源雜湊恢復。沒有因觀察時間用完而重開。修正後 `/bin/true` 故障替身在 0.3 秒內被拒；CLI 整合仍保留全部斷言，從約 15 秒降至 3.3 秒，見 negative-early-exit.json 與 fast-gate.txt。完整重跑已無逾時。

目前 11 個 Windows 專屬變異尚未在 Windows 執行，另 1 個 Linux OR/XOR 旗標變異等價；均未改寫成已抓到。Windows/macOS 實機、真人簽收、持久任務與批准紀錄、正式發布仍未完成。未部署。
