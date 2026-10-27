# 第一次程序試接

使用獨立暫存 HOME 與 CODEX_HOME，啟動本機 Codex 0.163.0 的 app-server --stdio，逐筆送出 requests.json；每次按請求 id 等回覆，最後關閉標準輸入並等待退出。未送出模型任務，也未接正式資料庫。

responses.json 是原始回覆。初始化成功；整數 threadId 與未知方法都回 -32600；程序正常退出。四項結果見 validation.json。

初次驗證器把非同步通知誤當第二個請求回覆，manifest.json 保留此失敗判讀；重新依 id 判讀同一原件，四項均通過。後續接法必須允許通知與回覆交錯。

尚未驗證工具授權、模型串流、取消恢復、Rig 或桌面；P01 未完成。本輪只有試接證據，沒有產品程式，未跑 Rust/TS 全套或部署。
