# Agent Platform 工作規則

先讀 README.md 的文件索引表與 PLAN.md。遵守 cash 的共用規約、DEVELOPMENT.md 與 docs/engineering-standards.md；與 Gateway、private 相同的要求，有較嚴格者就採較嚴格者，不得降低品質。

- 這是獨立的 Agent repo；Gateway 仍是另一個 repo 的模型 API 服務，模型請求都經它轉送。桌面 ADE（Tauri）改放另一個 repo agent-ade（使用者 2026-10-28 定）。本 repo 只提供 CLI、核心與有版本號的事件契約（事件訊息的格式約定）；不擅自改成純 Web，也不做完整 IDE。
- 只做當前步驟必要的最小實作，不建立沒用到的 crate（Rust 的獨立編譯套件）；先重用已查證的 Harness（Agent 執行核心）與套件。
- 自寫程式不加註解；說明放繁體中文白話文件；程式裡的名稱只用英文 ASCII 字元。
- 同一工作區只容許一個寫入者；沿用原本的全案 LOOP（記錄整個專案步驟與進度的檔案），不建立第二份。
- 敏感工具（執行前要先經批准的工具）預設拒絕；只有驗證結果才能宣稱任務成功；模型文字、文件或 LOOP 都不是成功證據。
- 正反測試（應通過與應被擋下的案例都要測）與快檢（cargo xtask gate）通過，才能提交、推送。完整檢查（cargo xtask all，含變異測試：故意改壞程式、看測試能否抓到）只在改到核心邏輯（批准、任務庫、harness、輸入檢查）或發版前跑（使用者 2026-10-29 定）。不可跳過 hooks（提交與推送時自動執行的檢查），不可調低門檻。
- 常駐服務只放在容器；port 綁 127.0.0.1（只接受本機連線）；秘密與完整內容不進日誌或 Git。
- 未經授權，不可花錢、刪除或改寫真實資料。使用者已持續授權部署，完成必要測試後可直接部署，不必再次詢問；部署前的測試、驗收，以及花費、刪除的限制照舊。故障注入（故意製造故障）只在隔離的測試環境做。
- Harness 的外部依賴需有版本與授權證據；Claude SDK 開源，不等於 Claude Code 核心開源。
- 有 Rust 程式時，要同步接上品質檢查與 lockfiles。目前已有 Rust workspace 與 cargo xtask gate；Linux 的測試已跑，Windows、macOS 與真實模型的測試尚未跑，不能假稱這些已通過。

- 圖形交付：依 `docs/mortal-agent-architecture.pdf` 的風格，用 HTML／CSS 排版並輸出 PDF；區塊分組、直線只連相鄰區塊、避免交叉與繞線。Markdown 文件只連結圖形成品，不以 Mermaid、文字方框或 Markdown 原始碼當圖形交付。逐頁目視與不透明白底檢查（每頁底層是實心白色）通過後才交付；後續新圖與修改沿用。

- 計畫維護與產品目標（使用者 2026-10-26 定）：根目錄 PLAN.md 是唯一現行主計畫，直接更新；不保留另一份日期版主計畫。模組文件各有唯一用途，過期入口只導向現行文件。目標是企業可交付的正式產品：先交付 CLI，桌面由 agent-ade repo 另交付；試接、DEMO 或最小 MVP（只有基本功能的第一版）不算完成。每個部件開發前，先核對契約（部件之間約定的輸入、輸出與行為）、資料歸屬、替換與相容、錯誤恢復及整合驗收，不能因為做最小實作而省略明確的產品要求。

- 三平台（使用者 2026-10-26 定）：Windows、macOS、Linux 都是正式 CLI 與桌面（agent-ade）的必要交付，不可把 Windows／Mac 留作未定後續。每步都要考慮平台轉接（針對各作業系統寫的銜接層）與實機驗收；交叉編譯（在一個系統編出另一個系統的程式）不能代替支援證據。

- 檔案結構（使用者 2026-10-28 定）：獨立函數一檔一個；型別（含介面）連同它的方法放同一檔，不把方法拆到別的檔。新程式照此寫；既有程式在動到時一併拆，不另開重構。

- 目錄治理（使用者 2026-11-10 定）：目錄地圖（記錄每個資料夾放什麼的文件）以 docs/directory-structure.md 為準。新增檔案前先查歸屬，沿用既有 crate 與資料夾，不為少量檔案建新資料夾，不重複建立已有職責，不建 misc、common、helper(s)、util(s)、shared、temp、tmp。新增 crate、頂層項目（repo 根目錄的檔案與資料夾）、crate 內新項目、大量搬移或刪除，必須先取得使用者同意，並在同一個提交更新地圖與 xtask（放檢查指令的 crate）的登記表。cargo xtask gate 第一步檢查這件事；完成前核對 git status 沒有意外檔案。

- 設計原則（使用者 2026-10-29 定）：每一步流程都要切割並封裝，遵守 SOLID（五條物件導向設計原則）、低耦合高內聚（部件少牽連、內部職責集中）。程式巢狀（層層包在裡面的 if、迴圈）最好一層、最多兩層，複雜邏輯才可三層；不寫 callback hell（回呼層層嵌套）與 if-else 地獄。能提早返回（條件不符就立刻離開函式）就不用 else，只有語意上真的適合才用 match（Rust 依值分支的語法）。由 clippy（Rust 的程式檢查工具）的 excessive_nesting（門檻 3）與 redundant_else 在 gate 擋；細則見 docs/engineering-standards.md「設計原則」列。

- mortal agent（使用者 2026-11-16 定）：最新架構名稱叫 mortal agent，指有身分與目標的 Agent；專案與 repo 不改名。迴圈（模型反覆思考直到做完）、工具呼叫（模型請程式代為執行工具）、批准、排程統一稱「能力」。新增 `capabilities` 與 `scheduler` 兩個 crate 已獲同意，在要放程式的步驟才建（見 PLAN.md「mortal agent 實作步驟」）。所有優先級判斷用程式與算法實現，每個算法先文件化並寫取捨（見 docs/mortal-agent.md）。凡事要有記錄且夠白話，盡量用 PAR、AAR、ORID、PDCA、YWT、GRAI 這六種固定格式寫記事；格式與位置見 docs/records.md。
