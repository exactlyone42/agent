# Agent Platform

做能實際完成工作、能停止與接續的 Agent，使用 Tauri 桌面 Client；Gateway 獨立提供模型 API。

目前交付的是實作計畫與開發規範，還沒有可用的 Agent。先讀 [PLAN](PLAN.md)，不要把清單、目錄或舊小測試當成產品完成。

| 文件                                      | 何時讀                                      |
| ----------------------------------------- | ------------------------------------------- |
| [實作計畫](PLAN.md)                       | 接手工作、確認目標、依賴、交付與驗收        |
| [技術與架構](docs/architecture.md)        | 確認程式、API、資料與桌面 Client 的邊界     |
| [來源研究](docs/research/sources.md)      | 選擇現成 Harness、框架、Rust 移植參考與授權 |
| [評測與錯誤注入](docs/evaluation.md)      | 設計案例、判定成功、測試故障及比較改版      |
| [品質要求](docs/engineering-standards.md) | 寫程式、安裝依賴、提交或推送之前            |
| [長任務接手](docs/longrun.md)             | 每五分鐘推進、恢復中斷、停止與保存證據      |
| [現況](docs/progress.md)                  | 分清已交付與未完成                          |
| [共用 Git 規範](DEVELOPMENT.md)           | 開分支、提交、整合與安裝 hooks              |

一個 repo 可有多個 Rust crate（可獨立編譯的套件）與服務，不代表必須一起部署。RAG、Memory、Computer Use、Evaluation 保留獨立使用入口；有具體外部接手者時可連同契約、測試與歷史拆出去。
