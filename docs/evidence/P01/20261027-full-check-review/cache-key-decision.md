# 秘密掃描疑似誤報的核對

23 筆警示都只命中 prompt_cache_key，原件中的值全為 UUID；逐檔雜湊與欄位核對見 cache-key-analysis.json。OpenAI 官方 API 定義此欄位用來改善快取命中，而非 API 驗證憑證：https://developers.openai.com/api/docs/guides/prompt-caching 。這項判斷限於本次已核對的請求原件，不把任意帶 key 的欄位一律忽略。

候選設定 secrets-candidate.toml 只針對 generic-api-key、指定 P01 請求原件路徑、完整欄位名稱與 UUID 形狀同時符合時作例外。其餘預設規則保留。設定語法依已安裝版本 Gitleaks 8.30.1：https://github.com/gitleaks/gitleaks/blob/v8.30.1/README.md#configuration 。

隔離測試五案通過：合法快取值放行；相同 UUID 放其他欄位、相同欄位放其他路徑、快取欄位內置入 token、同檔另置 token 都拒絕，輸出未出現合成 token 原文。原 23 筆命中的檔案重播也通過。結果見 secrets-candidate-tests.json 與 secrets-replay.json。

候選設定尚未接入執行中的 all。既有失敗結果保留，不改寫為成功；需等待變異程序終態、確認來源恢復後才修改 scripts，並以新一輪秘密掃描驗證。
