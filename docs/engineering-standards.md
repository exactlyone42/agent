# 品質要求：private 與 Gateway 的底線都保留

基準為本機 `mortal-mcp/tools/gitflow-policy/DEVELOPMENT.md`、共用 `~/.agents/RULES.md`，以及 Gateway `docs/engineering-standards.md` 與實際 `scripts/check.sh`。本次讀取日期2026-10-24；來源檔案雜湊在 `docs/research/baseline.json`。規範不是已通過檢查的證據。

## 不能降低的要求

| 項目           | 本repo要求                                                                                               | 落地時間                                                              |
| -------------- | -------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| Git            | develop整合，feature分支工作；Conventional Commits；禁強推、禁跳hooks                                    | 本次建立規範與共用hooks；首次空repo以feature建立seed，之後建立develop |
| 格式/靜態/測試 | 三關過才提交；push/PR前全套                                                                              | 本次僅文件與原樣復用policy；P01接Rust/TS全套                          |
| Rust           | rustfmt、clippy全target/features且-D warnings；測試必用--locked                                          | P01起                                                                 |
| TypeScript     | strict、ESLint、型別、Vitest、build；實際桌面驗收另做                                                    | P01起                                                                 |
| 變異測試       | 修改檔整檔100% mutation score；列工具版本、總mutants及結果，不以覆蓋率取代                               | 有自寫程式起；等價變異須可重現證據，不能默默排除                      |
| 覆蓋率         | 列本次變更未覆蓋行及原因；保留所有既有門檻                                                               | 有自寫程式起                                                          |
| 禁止模式       | 生產路徑無unwrap/expect/panic/todo/unimplemented/unreachable/bare assert；禁止mod.rs、散落模型名稱與設定 | P01 gate                                                              |
| 未用/重複      | Rust模組可達性、依賴檢查；TS knip與重複碼檢查，無用項處理而非壓警告                                      | P01 gate                                                              |
| 錯誤           | thiserror領域錯誤、統一API出口；全部記錄，不吞錯，不回空值冒充成功                                       | P02起                                                                 |
| 記錄與指標     | tracing/OTLP、固定低基數標籤；執行時秘密canary正反測試，禁止token/PII/完整body                           | P02起                                                                 |
| DB             | SQL參數化、隔離測試DB、migration不可改已套用版本；有界批次/交易/續跑                                     | P01/P02起                                                             |
| API/Client     | schema、權限/錯誤/取消契約；舊Client相容性及真正Client完整任務                                           | P02起                                                                 |
| Gate自己       | 每條禁止規則有可通過與故意失敗樣本；檢查腳本也受測                                                       | P01起，不能只說掃描完成                                               |
| 服務與資料     | 常駐只在容器，loopback port；不自動改寫原始資料；秘密不進Git                                             | 全程                                                                  |
| 依賴與授權     | Cargo/npm lock、容器digest、來源license/NOTICE；不複製不明授權                                           | P01鎖版本、每次新增核對                                               |

Gateway 特有的模型路由/租戶/故障契約不硬塞進 Agent；若本次改 Gateway，必須回原repo跑原本全套與原Client，不以Agent檢查替代。Gateway gate已超過文件舊列的檢查數，採實際腳本規則，不能只湊相同測試數。

## 統一檢查入口

2026-10-27 已建立 `bash scripts/check.sh gate`：實際執行格式檢查、全部 target／features 的 Clippy，以及生產函式庫／執行檔的 unwrap、expect、panic、todo、unimplemented、unreachable、assert 系列與 mod.rs 禁用檢查、未用依賴檢查及 workspace 測試。禁止模式以編譯器規則檢查，另用 1 個合法與 13 個違規樣本確認規則確實生效；測試程式仍可使用斷言。未用依賴使用 cargo-machete 0.9.2。`all` 已接快檢、工具版本、Git 列出的現有未忽略檔案秘密掃描、測試覆蓋率與整個 workspace 的變異測試。每輪原件保存在 `.runtime/checks/` 的唯一目錄；任何命令失敗或前後程式雜湊不同均不通過。秘密掃描包含未追蹤檔，排除 Git 忽略的建置產物與本輪原件；不能當成整機秘密掃描。未參與編譯的孤立檔案、模型設定規則與三平台整合驗收仍待接入，所以 P01 品質底座尚未完成。Linux 通過不代表 Windows／macOS 已實測；執行環境需預先安裝上述工具、Clippy 與 Python 3。

P01建立 `bash scripts/check.sh gate`（快檢）與 `bash scripts/check.sh all`（推送前全套）。未建立前命令不得假通過。底層至少執行：

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
npm ci
npm run lint
npm run typecheck
npm test -- --run
npm run build
```

DB測試由隔離容器包住；mutation、coverage、無用程式、秘密、規則自我測試與桌面/外部Client驗收由all納入。精確cargo-mutants/coverage參數在P01依實際crate建立，不能填不存在套件名。API/桌面行為改動需相應整合驗收，只有文件改動不虛構Rust測試。

## 安裝狀態與版本

本機已查到Rust 1.98.0、rustfmt、clippy、cargo-mutants、cargo-llvm-cov、Node22、npm10及Linux Tauri所需GTK/WebKit。這只代表本機有工具，不代表依賴相容或Agent能建置。P01必須記錄實際版本並產生lockfiles；Rust workspace 與 Cargo.lock 已建立；尚無桌面 package manifest，不宣稱桌面已建置。

共用Git policy程式原樣復用，來源雜湊可核對；不重寫它們、不刪上游註解或改寫以規避測試。自寫程式不加註解，必要說明放文件。規劃期只檢查Markdown格式/連結、JSON、共用policy測試、敏感檔案與Git差異；程式品質門檻尚待P01真正接入，不能把本文件當成已強制生效。

## CI與發布

CI的配置在P01寫好並做本機對等驗證；未確認免費額度不啟動可能計費的runner。無部署workflow；push不是部署。首次空repo建立develop不代表發版，不建正式服務。後續合併依已授權規則與真實CI/驗收狀態，不能用管理員繞保護。

完整檢查會暫時修改 Rust 來源來驗證測試能否抓錯，期間禁止其他執行者編輯或編譯；目前由唯一工作流持有專案鎖。每輪輸出包含開始／結束時間、退出碼、工具版本、來源雜湊與平台；`summary.json` 的本機通過不能代替人工簽收。若程序遭強制終止，缺少 summary 即視為未完成，先核對來源再續跑。秘密檢查自身用合成 token 驗證偵測與遮蔽；檢查器的成功、失敗及工具不存在均有測試。

2026-10-27 全套實跑已取得 80 個變異結果，仍有 Windows 未執行路徑與等價變異待正式裁決，完整門檻未過。覆蓋率原件固定存 coverage-report.json，執行紀錄另存 coverage.json，已有覆寫回歸測試。秘密例外僅限已核對的請求快取 UUID，其他憑證仍檢查；空快照拒絕。完整入口保留 active.json 到正常結束；中斷後先核對程序與來源，不能直接刪標記重開。最新分項證據位於 `.runtime/checks/20261027-quality-final/`，不混成原全套的同版通過證明。
