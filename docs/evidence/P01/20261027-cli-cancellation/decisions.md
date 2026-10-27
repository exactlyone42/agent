# 取消整合審查與裁決

初審與複審原件為 review-result.json、followup-result.json。成果寫入中途失敗誤報已用 negative-partial-artifact.json 重現；直接把記錄移回寫入後的建議未採納，因為會重現該錯誤。改為分開保存「已建立檔案」與「完整寫入且 sync_all 成功」；未完整保存一律 IncompleteArtifact，不能只因殘檔碰巧符合 expected 就成功。negative-incomplete-verified.json 已證明舊判定會誤報 exit 0。

舊 Tokio Ctrl+C 接法在同步開檔卡住時，第二次停止仍無效，負對照見 negative-blocked-interrupt.json。改用 ctrlc 3.5.2 獨立訊號執行緒；第一次取消共用 token，第二次 exit 130 強制結束，沒有 JSON 收據即結果未確認。termination 功能同時接 SIGTERM／SIGHUP，依相同契約處理。只註冊一次，使用 try_set_handler，不覆蓋其他既有處理器。

核對固定 Rig rev 55cd481 的 crates/rig-agent/src/agent/engine.rs：drive_tool_calls 使用 stream::iter 與 bounded buffer_unordered，run_single_tool 直接 await dispatch_tool_call；目前這兩個檔案工具沒有另 spawn 執行緒。工具開始前也核對取消 token，成果驗證前後再核對一次。取消與完成競爭以最後一次核對為本次完成決定點，完成後收到訊號不回溯修改結果；日後持久任務須將此決定放進事件交易。

可中止等待中的模型傳輸，不表示遠端運算已撤回。已開始的同步檔案操作不保證即時取消；第一次要求後仍等其返回，第二次可強制結束程序。Linux 測試不代表 Windows/macOS 控制台實測、外部工具程序樹、崩潰接續或持久授權已完成。

套件版本與授權另存 final-dependencies.json。加入 ctrlc 時離線解析曾降版部分非相關依賴，已按輸出逐項還原原版本 combine 4.6.9、schannel 0.1.30、wasm-bindgen 0.2.129 及其原配套；未藉此升級其他核心。
