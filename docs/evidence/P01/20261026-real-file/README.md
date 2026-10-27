# P01 實檔任務契約

這是 Rig Harness 比較測試，並非正式 CLI、沙箱或產品驗收。

測試從合成實檔讀內容，再經工具寫入新成果；由測試本身讀回，固定預期為 ORIGINAL FIXTURE。正常成果通過，漏寫、錯寫、拒絕不得判成功。模型在漏寫或錯寫時仍說 done，證明不能把模型文字或 Harness 正常返回當作成功。

其他案例核對允許／拒絕、錯參數的具體取消原因和模型請求次數、未授權邏輯檔名及缺檔的工具錯誤標記。固定邏輯檔名只用來限制測試能力，未驗證一般路徑解析、符號連結或作業系統沙箱。

negative.txt 是上一版改回固定回覆後失敗的原件；tests.txt 是當時 3 項正常測試。complete-flow-tests.txt 與 complete-flow-manifest.json 對應目前 5 項測試、9 個情境。這些不是 20 題正式任務評測。

Claude 初審原件為 review-prompt.txt／review-result.json；後續原件為 followup-prompt.txt／followup-result.json。初審的假成功問題已加入獨立成果讀回；取消原因及錯誤標記已收緊。測試失敗可能留下合成暫存檔，尚未補自動清理。

格式、靜態及測試通過；cargo-mutants 對這個整合測試檔列出零候選，不能算 100% 通過。正式品質腳本、覆蓋與變異要求、同題 Codex 流程、串流／取消／重啟及三平台仍未完成。

後續：測試已改呼叫 crates/agent 的實際成果驗證元件，工具交付同一個已開啟檔案供讀回。最新同版證據改見 ../20261026-verifier/manifest.json；早期完整流程檔只保留當時狀態，不覆寫原件。
