# Agent Platform 工作規則

先讀README索引及PLAN。遵守cash共用規約、DEVELOPMENT與docs/engineering-standards.md；與Gateway/private相同要求，有較嚴格者採較嚴格者，不得降低品質。

- 這是獨立Agent repo；Gateway仍是模型API服務。桌面ADE（Tauri）改放另一個repo agent-ade（使用者2026-10-28定），本repo只提供CLI、核心與有版本號的事件契約；不擅自改成純Web，也不做完整IDE。
- 只做當前步驟必要的最小實作，不建立未使用crate；先復用已查證Harness與套件。
- 自寫程式不加註解；說明放繁中白話文件；程式識別字英文ASCII。
- 同一工作區只容許一個寫入者；沿用原全案LOOP，不建立第二份。
- 敏感工具預設拒絕；只有驗證結果才能宣稱任務成功；模型文字、文件或LOOP不是成功證據。
- 正反測試與快檢（cargo xtask gate）通過才提交、推送；完整檢查（cargo xtask all，含變異測試）只在改到核心邏輯（批准、任務庫、harness、輸入檢查）或發版前跑（使用者 2026-10-29 定）；不可跳hooks、不可調低門檻。
- 常駐服務只在容器；port綁127.0.0.1；秘密與完整內容不進日誌或Git。
- 未授權花費、刪除或改寫真實資料。使用者已持續授權部署，完成必要測試後可直接部署，不必再次詢問；保留測試、驗收及其他花費、刪除限制。故障注入僅隔離測試環境。
- Harness外部依賴需版本及授權證據；Claude SDK開源不等於Claude Code核心開源。
- 有Rust/TS程式時同步接品質檢查與lockfiles；目前只有規劃文件，不能偽稱build/test已過。

- 圖形交付：依 `docs/agent-target-architecture.pdf` 的風格，使用 HTML／CSS 排版並輸出 PDF；區塊分組、直線只連相鄰區塊、避免交叉與繞線。Markdown 文件只連結圖形成品，不以 Mermaid、文字方框或 Markdown 原始碼當圖形交付。逐頁目視與不透明白底檢查通過後才交付；後續新圖與修改沿用。

- 計畫維護與產品目標（使用者 2026-10-26 定）：根目錄 PLAN.md 是唯一現行主計畫，直接更新；不保留另一份日期版主計畫。模組文件各有唯一用途，過期入口只導向現行文件。目標是企業可交付的正式產品，CLI 到桌面是開發順序，試接、DEMO 或最小 MVP 不算完成。每個部件開發前核對契約、資料歸屬、替換與相容、錯誤恢復及整合驗收，不以最小實作省略明確產品要求。

- 三平台（使用者 2026-10-26 定）：Windows、macOS、Linux 均是正式 CLI 與桌面的必要交付，不可把 Windows／Mac 留作未定後續；每步考慮平台轉接與實機驗收，交叉編譯不能代替支援證據。

- 檔案結構（使用者 2026-10-28 定）：獨立函數一檔一個；型別（含介面）連同它的方法放同一檔，不把方法拆到別的檔。新程式照此寫；既有程式在動到時一併拆，不另開重構。

- 設計原則（使用者 2026-10-29 定）：每一步流程都要切割並封裝，遵守 SOLID、低耦合高內聚；程式巢狀最好一層、最多兩層，複雜邏輯才可三層，不寫 callback hell 與 if-else 地獄；能提早返回就不用 else，只有語意上真的適合才用 match；由 clippy 的 excessive_nesting（門檻 3）與 redundant_else 在 gate 擋；細則見 docs/engineering-standards.md「設計原則」列。
