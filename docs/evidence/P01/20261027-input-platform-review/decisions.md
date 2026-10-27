# 審查處理

原始審查只讀程式，未實跑；不能當三平台驗收證據。

1. 接受身分比對缺少反例。已新增 Windows 單元測試：內容相同的不同檔案，身分必須不同；同一檔案的硬連結，身分必須相同。另保留開啟期間禁止改寫及刪除的 Windows 測試。兩者已通過 Windows 目標編譯與 Clippy，尚未執行，不能宣稱測試通過。
2. 接受作業系統 InvalidInput 與本案驗證結果混淆。新增明確的 InputError，區分檔案檢查未過與系統讀取／核對失敗，不再依 io 錯誤種類猜測。Linux gate 通過；三種外部目標核心編譯與 Clippy 通過。
3. CLI 測試已明確使用 UTF-8 解碼，避免依 Windows 預設編碼判讀 Rust 輸出。
4. source_grew 已同時斷言工具回報超量、沒有回傳被截短的來源；原件見目前 scripts/test_cli.py，不以審查者推論代替測試。最新 Linux gate 實跑通過。

新錯誤型別及 Windows 測試尚需實機與完整同版變異驗證。Windows 的 128 位身分查詢使用 windows-sys 0.61.2（本機套件宣告 MIT OR Apache-2.0）；不回退為較短編號、不支援查詢時拒絕。
