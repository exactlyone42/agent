# Agent Platform

圖文閱讀：[完整架構與驗收 PDF](docs/PLAN.pdf)。包含架構圖、部件契約與 36 張驗收操作卡；產品實測仍待執行。

Windows、macOS、Linux 三平台必交的企業 Agent：先完成 CLI，再完成共用核心的 Tauri 桌面；含任務執行、三塊 RAG、記憶及電腦操作。唯一現行計畫是 [PLAN.md](PLAN.md)，首頁直接說明產品、介面、安裝及正式發布門檻。

目前尚無通過整合驗收的正式產品；內部試接不能當成企業交付。

| 文件                                            | 何時讀                                                   |
| ----------------------------------------------- | -------------------------------------------------------- |
| [全案運作方法](docs/agent-method.md)            | 研究整個 Agent 的分工、來源證據與驗收契約                |
| [實作計畫](PLAN.md)                             | 接手工作、確認目標、依賴、交付與驗收                     |
| [技術與架構](docs/architecture.md)              | 確認程式、API、資料與桌面 Client 的邊界                  |
| [自寫 harness 設計取捨](docs/harness-design.md) | 動手寫核心、場次、回呼、MCP 之前，看各家做法與我們的取捨 |
| [來源研究](docs/sources.md)                     | 選擇現成 Harness、框架、Rust 移植參考與授權              |
| [評測與錯誤注入](docs/evaluation.md)            | 設計案例、判定成功、測試故障及比較改版                   |
| [品質要求](docs/engineering-standards.md)       | 寫程式、安裝依賴、提交或推送之前                         |
| [變異測試例外](docs/mutation-equivalents.md)    | 新增或調整變異測試排除項目之前                           |
| [CLI 開發入口](docs/cli.md)                     | 執行目前的檔案任務、判讀結果與已知限制                   |
| [共用 Git 規範](DEVELOPMENT.md)                 | 開分支、提交、整合與安裝 hooks                           |

一個 repo 可有多個 Rust crate（可獨立編譯的套件）與服務，不代表必須一起部署。RAG、Memory、Computer Use、Evaluation 保留獨立使用入口；有具體外部接手者時可連同契約、測試與歷史拆出去。
