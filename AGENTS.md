# Agent Platform 工作規則

先讀README索引及PLAN。遵守cash共用規約、DEVELOPMENT與docs/engineering-standards.md；與Gateway/private相同要求，有較嚴格者採較嚴格者，不得降低品質。

- 這是獨立Agent repo；Gateway仍是模型API服務。Tauri桌面Client，不擅自改成純Web。
- 只做當前步驟必要的最小實作，不建立未使用crate；先復用已查證Harness與套件。
- 自寫程式不加註解；說明放繁中白話文件；程式識別字英文ASCII。
- 同一工作區只容許一個寫入者；沿用原全案LOOP，不建立第二份。
- 敏感工具預設拒絕；只有驗證結果才能宣稱任務成功；模型文字、文件或LOOP不是成功證據。
- 正反測試與全套品質檢查通過才提交、推送；不可跳hooks、不可調低門檻。
- 常駐服務只在容器；port綁127.0.0.1；秘密與完整內容不進日誌或Git。
- 未授權花費、正式部署、刪除或改寫真實資料。故障注入僅隔離測試環境。
- Harness外部依賴需版本及授權證據；Claude SDK開源不等於Claude Code核心開源。
- 有Rust/TS程式時同步接品質檢查與lockfiles；目前只有規劃文件，不能偽稱build/test已過。
