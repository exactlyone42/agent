# 記事格式與紀錄

用途：規定這個專案的工作紀錄怎麼寫、寫在哪，並放每一輪的記事。使用者 2026-11-16 要求「凡事都有記錄、夠白話」，並盡量用 PAR、AAR、ORID、PDCA、YWT、GRAI 六種格式（六種各是什麼，見「六種格式」一節的表）。

## 寫在哪

| 內容                             | 位置                                                                          | 為什麼                             |
| -------------------------------- | ----------------------------------------------------------------------------- | ---------------------------------- |
| 步驟進度（誰、何時、做到哪）     | `loops/agent-platform.checkpoints.jsonl`，只能用 checkpoint（檢查點）工具寫入 | 有格式檢查，不能手改               |
| 人讀的記事（本文件下方）         | `docs/records.md`                                                             | 一個專案一份，不另開資料夾         |
| 命令的完整輸出、失敗時的錯誤訊息 | `.runtime/`（不進 Git）                                                       | 只看終端畫面，視窗一關就沒了       |
| 對外的結果與風險                 | PR 說明                                                                       | 審查的人從這裡看，用 PAR 或 AAR 寫 |

新記事加在「記事」一節最下面，最新的在最後；一則記事只用一種格式，不重複寫同一件事。

## 六種格式

名詞：回顧是做完事後回頭看；里程碑是整個專案中幾個重要的完成節點。

| 格式 | 全名與欄位                                                                          | 什麼時候用                         |
| ---- | ----------------------------------------------------------------------------------- | ---------------------------------- |
| PDCA | Plan 計畫、Do 做了什麼、Check 檢查結果、Act 下一步或修正                            | 每一個步驟，開工前寫 P，做完補 DCA |
| AAR  | 行動後回顧：預期發生什麼？實際發生什麼？差在哪、為什麼？下次保留什麼、改進什麼？    | 每個步驟做完，和 PDCA 的 C 對照    |
| PAR  | Problem 問題、Action 做了什麼、Result 結果                                          | 一個問題、一次修復、PR 說明        |
| ORID | Objective 事實、Reflective 反應與感受、Interpretive 意義與影響、Decisional 決定     | 有取捨的決定，特別是和使用者討論後 |
| YWT  | Y 做了什麼、W 學到或發現什麼、T 下一步                                              | 每輪對話收尾                       |
| GRAI | Goal 原本目標、Result 實際結果、Analysis 差在哪與原因、Insight 學到什麼與具體下一步 | 階段或里程碑結束                   |

定義來源與邊界：

- ORID 的四層問題來自 ICA（文化事務協會）的對話法，來源見 [Umbrex](https://umbrex.com/resources/frameworks/facilitation-frameworks/orid-focused-conversation-method/)，2026-11-16 查到。
- GRAI 沒有權威或學術來源；只查到一份開源技能與一張流程圖說它是 Goal、Result、Analysis、Insight 四步回顧（[技能說明](https://skills.sh/hexbee/hello-skills/retrospective-master)、[ProcessOn](https://www.processon.io/view/grai-review-method/67405408e9d3aa4b3b67df7a)）。本專案採這個說法；使用者若指的是別的意思，要改本表。
- PAR 找不到工作紀錄用的標準來源；本專案採「問題、做法、結果」的字面意思。
- YWT（做了什麼、學到什麼、下一步）據說是日本常見的回顧格式；本輪沒有查到來源，欄位依常見用法。
- AAR 與 GRAI 內容有重疊。分工：AAR 用在單一步驟，GRAI 用在階段或里程碑，不對同一件事寫兩次。
- PDCA 即計畫、執行、檢查、行動的循環，是通用管理方法。

## 記事

### 2026-11-16 mortal agent 文件重整（步驟 M00）

M00 是 mortal agent 實作步驟的第一步，只做文件與圖；名稱與編號的說明見 [mortal agent](mortal-agent.md)。

**ORID：新架構要不要有名字、怎麼稱呼、拆幾個 crate**

- O 事實：使用者提了 12 點要求；原本的 harness（Agent 的核心套件）把迴圈、工具呼叫、回合上限綁在一個 Agent 型別（`agent.rs`），沒有排程。
- R 反應：使用者嫌文件太多，要求新舊文件整併，且所有判斷由程式與算法做、先寫成文件。
- I 意義：能力要能單獨載入或不載入，排程是「多件工作之間」的事，不是「一件工作之內」的事，兩者責任不同。
- D 決定：名稱 mortal agent；迴圈、工具呼叫、批准、排程統一叫「能力」；新增 `capabilities` 與 `scheduler` 兩個 crate（Rust 程式套件），在要放程式的那一步才建；專案與 repo 不改名。

**PAR：記事工具缺套件**

- P：checkpoint 寫入工具需要 jsonschema（檢查資料格式的 Python 套件），系統環境沒有，用 `uv run` 執行也沒自動裝上。
- A：在暫存區（scratchpad）建一個獨立的 Python 虛擬環境（venv），裝 jsonschema、rfc3339-validator、markdown，用它執行工具。
- R：M00 兩筆 checkpoint 寫入成功；沒有動專案依賴。

**PAR：架構圖第 3 頁分組標籤被截斷**

- P：「人（harness crate，不依賴任何能力）」太長，換行後蓋住框線。
- A：縮成「人（harness crate）」，不依賴任何能力的規則寫在第 2 頁。
- R：重新把 PDF 轉成圖片，逐頁用眼睛檢查，標籤完整。

**PAR：重出 PLAN.pdf 時產生器壞了**

- P：產生 PLAN.pdf 的程式 `scripts/render_plan.py` 找每張驗收卡的「狀態」欄，36 張卡都沒有這一欄，執行就中止。這不是本輪改壞的：存放驗收卡的 `docs/evaluation.md` 本輪沒被動過，所以早就壞了。
- A：只刪掉讀「狀態」的那一處，卡片上仍顯示「階段」；重出 PDF 後做不透明白底檢查（確認每頁底層是實心白色，40 頁全過）與陰性對照（故意拿掉底色，確認檢查會報錯：抽 3 頁全部抓到透明）。
- R：`docs/PLAN.pdf` 由 37 頁變 40 頁，附錄含新的 M1–M7 表。產生器改成 Rust，屬於原計畫「第三刀」（PLAN.md 把 Python 測試與檢查工具改成 Rust 的第三個步驟），這次沒做。

**PAR：刪舊文件前先留備份**

- P：`capability-design`、`ple-competition` 兩份文件與兩個 PDF 從沒提交（commit）進 Git，刪了就拿不回來。
- A：先複製到這次工作的暫存區，再刪；`harness-design.md` 與舊架構圖已提交過，用 `git rm`，Git 歷史保有。
- R：`docs/` 剩 13 個檔案；`harness-design.md` 的「做的順序」與現況補進 `mortal-agent.md` 第四節，沒有丟決定。

**PDCA：步驟 M00（只動文件，不動程式）**

- P：12 點要求拆成：名稱與術語、現況與目標圖、實作步驟、舊文件整併、算法與取捨文件、記事格式。
- D：新增 `mortal-agent.md`、4 頁架構圖 PDF、`records.md`；改 PLAN、README、architecture、directory-structure、AGENTS、agent-method；刪 6 個舊檔。
- C：圖 PDF 4 頁全部不透明白底；`npm run -s format:check`（檢查文件排版）過；`cargo xtask gate` 退出碼 0（成功），沒有失敗測試。沒跑：`cargo xtask all`（完整檢查；因為沒動程式，所以沒跑）、真模型、Windows 與 macOS。
- A：下一步 M1。排程用的「急、重要」由誰填、任務庫 v4 何時做，等使用者決定。

**AAR：步驟 M00**

- 預期：只改文件，一輪做完。
- 實際：多做了兩件預料外的事：補 checkpoint 工具的環境、修壞掉的 PLAN.pdf 產生器。
- 差在哪、為什麼：沒有先試跑產生器，直到要重出 PDF 才發現壞了；工具環境缺套件是因為全機沒有共用的環境。
- 保留：圖先在暫存區做、檢查完才放進 repo；刪檔前備份。改進：開工先試跑一次要用到的產生器與檢查工具。

**YWT：2026-11-16**

- Y 做了：mortal agent 的文件、圖與記事，舊文件整併。
- W 學到：排程算法一律先寫取捨再寫程式，能讓「用什麼算法」變成可以檢查的決定；產生器久沒跑會悄悄壞掉。
- T 下一步：使用者確認 GRAI 的意思與 PAR 的定義；確認後開工 M1。
