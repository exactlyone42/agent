# Client 拒絕工具的往返試接

Codex 0.163.0 把假模型的寫入工具要求交給測試 Client；Client 回覆拒絕與 success=false，下一次模型請求收到 permission denied。7 項執行檢查通過，合成檔案內容保持 fixed-value，程序正常退出。

這證明 Client 能把拒絕結果送回模型，並在這次拒絕路徑維持檔案內容。尚未實作產品的授權判斷或寫入工具，不能宣稱 Codex 內建權限已阻擋寫入；也不能把回合 completed 當成使用者任務 succeeded。此案例正好顯示兩者必須分開。

失敗對照在 negative-write：故意在回覆拒絕前改寫臨時合成檔案，實跑退出碼 1，fixture_unchanged=false，其餘六項通過。獨立檢查器另拒絕改錯回傳、缺少呼叫與改動檔案三種反例；連同原始往返共四項通過。

原件：manifest.json、transcript.json、requests.json、fixture-after.txt；失敗原件在 negative-write。checks.json 保存退出碼、語法檢查與程式雜湊。所有模型回應固定在本機，無推論費用、正式資料庫或部署。

重跑前複製整個目錄至新證據目錄，再依序執行其中 probe.py、validate.py、negative-write/probe.py；最後一個命令預期退出碼 1。不要覆寫本輪原件。

已驗 Python 語法與差異空白；尚未跑整檔變異、覆蓋率、Rust/TS 全套、Rig、取消恢復或 Tauri 實機。未提交或推送，P01 仍未完成。下一步驗內建敏感工具拒絕及取消，再按同一契約比較 Rig。
