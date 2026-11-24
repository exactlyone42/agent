# Agent Platform

這是企業用的 Agent（能自己規劃步驟、呼叫工具完成工作的 AI 程式），Windows、macOS、Linux 三個平台都必須交付。先完成 CLI（在終端機輸入命令的操作方式）；桌面工作台 ADE 放在另一個 repo（agent-ade），只透過本 repo 的事件契約（雙方約定好的事件訊息格式）連接。功能包含任務執行、三塊 RAG（先查資料再回答：向量搜尋找原文、主動查證、持續更新的知識庫）、記憶及電腦操作。唯一現行計畫是 [PLAN.md](PLAN.md)，首頁直接說明產品、介面、安裝及正式發布門檻。

目前還沒有通過整合驗收（把各部件接起來跑完整流程的檢查）的正式產品；內部試接不能當成企業交付。

| 文件                                                                                                              | 何時讀                                                                                                                |
| ----------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| [全案運作方法](docs/design/agent-method.md)                                                                       | 研究整個 Agent 的分工、來源證據與驗收契約                                                                             |
| [實作計畫](PLAN.md)及[圖文版 PDF](docs/diagrams/PLAN.pdf)                                                         | 接手工作、確認目標、依賴、交付與驗收                                                                                  |
| [知識計畫](docs/design/knowledge-plan.md)                                                                         | 做知識發現、解析、檢索、更新與維護之前                                                                                |
| [技術與架構](docs/design/architecture.md)                                                                         | 確認程式、API、資料與桌面 Client 的邊界；現況各 crate 分工見[現況類別圖 PDF](docs/diagrams/current-class-diagram.pdf) |
| [mortal agent](docs/design/mortal-agent.md)及[架構圖 PDF](docs/diagrams/mortal-agent-architecture.pdf)            | 改核心、能力、排程，或看各家 harness（執行核心）取捨之前                                                              |
| 目標圖：[目標架構圖](docs/diagrams/target-architecture.pdf)、[目標類別圖](docs/diagrams/target-class-diagram.pdf) | 開工前看要做成什麼樣子；這是設計，不是現況；現況見上方兩列                                                            |
| [記事](docs/standards/records.md)                                                                                 | 查工作紀錄格式、每輪記事                                                                                              |
| [來源研究](docs/research/sources.md)                                                                              | 選擇現成 Harness（執行核心）、框架、Rust 移植參考與授權                                                               |
| [評測與錯誤注入](docs/research/evaluation.md)                                                                     | 設計案例、判定成功、測試故障及比較改版                                                                                |
| [任務切分與交接](docs/research/task-splitting.md)                                                                 | 改 PLE harness 的規劃、切分、上下文處理或交接筆記之前                                                                 |
| [PLE 跑題記錄](docs/run-history/ple-runs.md)                                                                      | 每次跑 PLEBench 題目後記分數、沒過的檢查、模型與耗時；要看過去結果之前                                                |
| [品質要求](docs/standards/engineering-standards.md)                                                               | 寫程式、安裝依賴、提交或推送之前                                                                                      |
| [目錄結構](docs/standards/directory-structure.md)                                                                 | 新增檔案、資料夾或 crate（Rust 的獨立編譯套件）之前，確認該放哪裡                                                     |
| [過時檢查](docs/standards/drift-review.md)                                                                        | 定期找文件、圖與程式對不上、或可刪的過時文件；要跑、排程或調整門檻之前                                                |
| [變異測試例外](docs/standards/mutation-equivalents.md)                                                            | 新增或調整變異測試（故意改壞程式、看測試能否抓到）排除項目之前                                                        |
| [變異測試既有債務](docs/standards/mutation-debt.md)                                                               | 要判斷變異測試漏抓是不是既有問題、或要補這些測試之前                                                                  |
| [CLI 開發入口](docs/design/cli.md)                                                                                | 執行目前的檔案任務、判讀結果與已知限制                                                                                |
| [共用 Git 規範](DEVELOPMENT.md)                                                                                   | 開分支、提交、整合與安裝 hooks（提交與推送時自動執行的檢查）                                                          |

一個 repo 可放多個 crate 與服務，但不必一起部署。RAG、Memory（記憶）、Computer Use（電腦操作）、Evaluation（評測）各自保留獨立的使用入口；有具體的外部接手者時，可連同契約、測試與歷史拆出去。
