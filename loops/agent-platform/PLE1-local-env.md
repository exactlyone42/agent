# PLE1 比賽本機測試環境：能連上、能跑分

母檔：../agent-platform.loop.md。依賴：無（不等 S00、P01）。對應 PLAN.md 比賽版步驟 C1 的前半與 C3 的「能量到分數」。官方原文存在 `.runtime/ple-docs/`（2026-11-19 下載；開工先核對是否仍是最新）。

## 固定主任務（不可改寫）

在這台 Linux 機器上，讓我們自己寫的 harness（Agent 執行核心，比賽要求它是一個 WebSocket 伺服器，由評分端連進來）通過官方檢查工具 `kit check` 的連線、範例題與 MCP 題，並用官方 `eval/run.py` 對至少一題開發集題目跑出官方分數。

這張票只要「量得到分數」，不要「分數高」。分數多少都算完成；提高分數是 PLAN.md 的 C2–C7，不在這張票。

L1–L7 全過只代表本機環境可用，不代表可以報名。報名另外要三件事，都不在這張票：harness 宣告使用官方八個 `google-cloud/<id>` 模型之一；29 題開發集的成果由同一個 harness、同一個模型經 `eval/run.py` 產生（官方 rules.md）；推送成 GitHub 上的一個 commit。本機要跑這些模型，官方給兩條路：`kit/model_proxy`（向 GCP 專案計費），或任何提供相同模型名稱的相容代理（官方 models.md）。公司 Gateway 能不能當第二條路未查證。

Kaggle 比賽頁面的五條規則（2026-11-19 使用者貼上原文；整理在 docs/design/mortal-agent.md 第五節「Kaggle 比賽頁面的規則」）。前四條這張票就要顧到，第五條只記不做：

1. 本機可以用自己的模型產生成果，交件前再把 `model_api` 改成官方模型；所以 A3 的公司 Gateway 通路可以沿用到交件。
2. 交件的 repo 截止時必須公開，授權是 Apache-2.0 或 MIT。參賽資料夾從第一天就當成會公開的來寫：不放公司內部網址、金鑰、內部文件；從本 repo 複製程式前，先列出要複製哪些檔案並回報，等使用者同意公開範圍。
3. 根目錄要有 `serve.py`／`harness.py`。核心若用 Rust，放一層很薄的 Python 啟動檔去呼叫它；這兩個檔名是否強制未查證，L4 動手前先在官方 clone 的說明與範例裡查，查到的寫進 L4 證據欄。
4. 正式環境：安裝限 30 分鐘、`/opt/harness` 壓縮後 2 GiB、每題 2 核心與 4 GB 記憶體。L4 要記下安裝步驟實際花的時間與產物大小；超過就是阻塞。
5. 得獎看分數也看成本，說明文要兩個模型的 29 題結果。這屬於 PLAN.md 的 C3–C7，記到「延後事項」，不在這張票做。

## 完成條件（進度帳；只有附證據路徑才能打勾）

| id  | 成果（不是活動）            | 驗收命令與通過標準                                                                                                                                                                                                                                                                                                           | 負向對照                                                                                                                                                                                                                                                                | 狀態    | 證據                                                                                     |
| --- | --------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------- | ---------------------------------------------------------------------------------------- |
| L1  | 官方工具就位                | `$PLE_KIT_DIR` 是 google-research/ple 的 clone，記下 commit；harbor 0.20.0 已裝；`uv run python -m kit validate` 對官方 `minimal` 範例退出碼 0                                                                                                                                                                               | 把範例的 `serve` 欄位刪掉，validate 必須失敗                                                                                                                                                                                                                            | DONE    | .runtime/ple-env/l1-validate-ok.log、l1-validate-neg.log、l1-commit.txt（clone 8320f04） |
| L2  | 模型通路可用                | 從 Docker 容器內用金鑰呼叫模型端點，回應含工具呼叫；金鑰只從 `$PLE_ENV_FILE` 讀                                                                                                                                                                                                                                              | 不給 MODEL_API_KEY 時 kit 必須在建置前拒絕並點名（使用者 2026-11-19 同意：本機 ollama 不驗金鑰，原「錯金鑰必須失敗」改不要求）                                                                                                                                                       | DONE（依 A3 改路徑，條件已由使用者同意縮減） | 依 A3 改走本機 ollama 後：容器內經出口代理呼叫 gpt-oss:20b 並回工具呼叫（`l4-check.log` 的 smoke 用 write_file、mcp 用 http／read_file／write_file；`l5-run` 有 31 次 exec）。負向對照部分成立：不給 `MODEL_API_KEY` 時 kit 在建置前拒絕並點名（`l6-missing-var.log`）。做不到的：錯金鑰或無金鑰對 ollama 都回 200（`l2-ollama-keys.log`），ollama 不驗金鑰，原條件「錯金鑰必須失敗」在這條路徑上無法驗。Gateway 路徑仍 BLOCKED（Claude 回 400/429、ollama 經 Gateway 回 502，見 `l2-findings.md`；`gateway-prompt.md` 待使用者轉交 Gateway repo） |
| L3  | 環境本身沒問題（對照組）    | 官方 `minimal` 範例原樣跑 `kit check`，退出碼 0；model、listen、smoke 三關通過；mcp 關照實記下結果，通過或警告都算（官方說明：harness 沒用 MCP 時這關只警告；`minimal` 有沒有做 MCP 未查證）                                                                                                                                 | 無（它本身就是 L4 的對照組）                                                                                                                                                                                                                                            | DONE | `.runtime/ple-env/l3-check.log`：對照組（換成 ollama 模型）退出碼 0，model、listen、smoke 過；mcp 為警告（官方說明 harness 沒用 MCP 時只警告） |
| L4  | 我們的 WebSocket 伺服器接上 | 我們的 entry（參賽資料夾）跑 `kit check --cancel-test`，listen、smoke、mcp、cancel 都過，mcp 必須是真的通過、不是警告；協定紀錄檔裡看得到 exec、read_file、write_file、http 四種請求各至少一次與 done。證據分三類分開記：協定單元測試、整合測試、官方檢查；官方檢查沒測到的行為（同時 8 個上限、取消時的排隊）只能引用前兩類 | 不設 `PORT` 啟動必須失敗且訊息說明缺哪個變數；單元測試故意送格式錯的訊息，計數必須增加；單元測試確認還沒收到結果的請求永不超過 8 個，第 9 個在本地等、不送出；任務取消後還在等的請求不得送出                                                                            | DONE | `.runtime/ple-env/l4-check.log`：`kit check --cancel-test` 退出碼 0，build 59 s／bundle 81 MiB（限 30 分／2 GiB）、listen、smoke、mcp（真通過，http 4＋write_file 1＋read_file 1）、cancel（10 s 內 done）皆 PASS；artifacts 警告待 L5。單元＋整合測試 18 條過、clippy 乾淨（`l4-gate.log`）；反向對照 4 項皆明確失敗（`l4-mutations.log`；原先 2 項只是卡住，加逾時後修正，commit edb399e）；`l4-noport.log` 缺 PORT 退出碼 2 並印出 missing environment variable PORT。entry 提交 5d90313（ple-harness，feature/protocol-server，未推送）。未查到 `serve.py` 必要：submission.yaml 的 serve 指令為 Rust 執行檔且檢查通過。exec 不在 smoke／mcp，但在 L5 實跑的協定紀錄出現 31 次（另 read_file 9、done 1，`l5-run/attempt1/jobs/dev/*/agent/ple-harness.txt`）；四種請求合併看都出現過 |
| L5  | 真的跑出官方分數            | `eval/run.py $PLE_ENTRY_DIR --tasks wind-diaphragm-shear --no-export` 的 `results.json` 有該題 0 到 1 的分數與 token 用量                                                                                                                                                                                                    | 在參賽資料夾的隔離副本上拿掉該題交付檔，再讓評分程式重新評一次，分數必須是 0 或標為沒量到。不用 `--regrade`：它只重評評審模型失敗的題目，而且用的是當次存下的交付快照。確切命令開工時依官方 `docs/artifacts.md` 的 `eval/run.py --artifacts` 核對（該檔未下載、未查證） | DONE | `.runtime/ple-env/l5-run/results.json`：wind-diaphragm-shear 分數 0.000（status ok、197 s、input 304215／output 7222 tokens），模型 gpt-oss:20b 經本機 ollama；harness 產出 submission.zip 但內容是占位檔，評分器 valid=0（`test-stdout.txt`）。分數 0 是模型能力，不是環境壞：交付檔存在且被評分。負向對照 `l5-negctl.log`：隔離副本有交付檔→評出 0.000 ok；拿掉交付檔→判為空、被拒、退出碼 1。邊界：只證明分得出「評過得 0」與「沒交付」，沒有拿到非 0 分的例子，評分器能給高分未驗證。Gateway 未涉及（本機 ollama）；Gateway 是否存完整請求未查證 |
| L6  | 別人能重跑                  | entry 的 README 寫明環境變數表與三條命令（檢查、單題跑分、重評）；照著從乾淨終端重跑一次 L4、L5 成功；README 與證據記下四個版本：官方 clone 的 commit、參賽資料夾的 commit、`results.json` 裡的題庫 commit、模型名稱與設定                                                                                                   | 環境檔少一個必要變數時，命令在開始建置前就停下並指出缺哪個                                                                                                                                                                                                              | DONE | `ple-harness` commit 664e5cd 的 README 有環境變數表、三條官方命令、四個版本（官方 clone 8320f04、entry、題庫 e0c21fa558c3、模型 gpt-oss:20b 與預設設定）。乾淨 `git clone` 照 README 重跑：`l6-clean-check.log` 全 PASS 退出碼 0（26 檔），`l6-clean-run.log` 分數 0.000 ok 退出碼 0（255 s）。負向對照 `l6-missing-var.log`：環境檔缺 MODEL_API_KEY，在建置前停下並點名該變數，退出碼 1。邊界：只驗了缺 MODEL_API_KEY，沒逐一驗其他變數；`--regrade` 命令只寫進 README，沒跑過 |
| L7  | 換人接手也判得對            | L4、L5 完成後開一個全新的 Agent，只給這張票、進度帳與證據路徑，不給執行者的對話與自評，請它逐條判定 L1–L6 是否真的完成並寫出依據；它的判定與證據相符；證據不足以確認的條目必須判「無法確認」，不能因為進度帳寫已完成就照收                                                                                                   | 給它一份進度帳副本（放 `.runtime/ple-env/`），其中一條故意標成已完成但證據路徑不存在；它必須指出這一條                                                                                                                                                                  | DONE | `.runtime/ple-env/l7/verdict.md`：全新 Agent 只憑票據副本與證據判定；副本故意植入的假證據（L1 的 `l1-validate-neg-final.log` 不存在）被指出，負向對照成立。它對 L5、L6 判已確認；L2（條件縮減）、L3（無退出碼、對照組已換模型）判未確認或無法確認，這些維持如實記載；L4 兩個缺口已補。邊界：只跑一個驗收者、一次；L7 沒有另外對未植入的原票據再判一次 |

跑 L5 之前先查公司 Gateway 會不會把完整請求內容存下來或送到別處（題目會經過它），把查到的結果記在 L5 的證據欄；查不到就寫未查證並回報，不因此停下 L1–L4。

L5 選 `wind-diaphragm-shear` 是因為它由程式檢查評分（checks），不需要另一個模型當評審。若這題的映像建不起來，可換另一題同樣是 checks 的題目，並在證據欄寫明。

## 環境變數（全部從環境讀，不寫死、不進 Git）

| 變數                                                         | 誰設定       | 用途                                                                     |
| ------------------------------------------------------------ | ------------ | ------------------------------------------------------------------------ |
| `PORT`                                                       | 評分端       | 伺服器監聽 `0.0.0.0:$PORT`；沒有就啟動失敗，不給預設值                   |
| `MODEL_BASE_URL`、`MODEL_API_KEY`、`MODEL_NAME`、`MODEL_API` | 評分端       | 模型網址、金鑰、模型名；`OPENAI_BASE_URL`、`OPENAI_API_KEY` 與前兩個相同 |
| `HTTPS_PROXY`、`HTTP_PROXY`、`ALL_PROXY`、`NO_PROXY`         | 評分端       | 對外連線只能走這個代理；HTTP 用戶端必須遵守                              |
| `PLE_KIT_DIR`                                                | 我們（本機） | 官方 repo 的 clone 位置                                                  |
| `PLE_ENTRY_DIR`                                              | 我們（本機） | 我們的參賽資料夾（獨立 Git repo，必須在官方 clone 之外）                 |
| `PLE_ENV_FILE`                                               | 我們（本機） | 放金鑰的環境檔，位置在所有 repo 之外；傳給 `--env-file`                  |
| `HF_TOKEN`                                                   | 使用者       | 下載開發集題目（要先在 Hugging Face 頁面接受條款）                       |
| `MODEL_PROXY_URL`、`MODEL_PROXY_KEY`、`VERTEXAI_PROJECT`     | 使用者       | 只有走 Google Cloud 模型時才用；會向 GCP 專案計費                        |
| `JUDGE_BASE_URL`、`JUDGE_API_KEY`、`JUDGE_MODEL`             | 我們（本機） | 只有跑需要評審模型的題目才用；這張票不需要                               |

時間與大小上限（任務總時間、單次命令上限、輸出上限、寫檔上限）從評分端送來的 task 訊息的 `limits` 讀，不寫死。協定規格裡固定的數字（單則訊息 16 MiB、同時 8 個請求、id 最長 128 字元、格式錯誤上限 20 則）寫成具名常數並註明出處在文件裡。

## 開工前要有的授權（寫在開工 prompt 的「授權」區塊；沒寫到的一律當作沒授權）

| id  | 要決定的事                    | 建議                                                                                                                                                                                                              | 沒授權時                                |
| --- | ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| A1  | 官方 clone 與參賽資料夾建在哪 | 已同意（使用者 2026-11-19）：`~/work_space/private/ple` 與 `~/work_space/private/ple-harness`；參賽資料夾做本機 `git init` 與本機 commit（官方 `files` 關會檢查檔案都已提交），不設遠端、不建 GitHub repo、不推送 | —                                       |
| A2  | 我們的伺服器程式放哪          | 已同意（使用者 2026-11-19）：放參賽資料夾，需要的 crate 用複製帶過去；本 repo 不新增 crate                                                                                                                        | —                                       |
| A3  | 本機跑分用哪個模型            | 已同意（使用者 2026-11-19，同日改）：本機跑分不必走 Gateway；改用本機 ollama（不計費），以官方「自己的模型」方式接（`openai-compatible/<model>` 加 `--model-base-url`）。公司 Gateway 仍可用但非必要。不走 GCP，也不啟用 AWS 計費。這種分數只證明環境通，不能和比賽模型的分數比 | —                                       |
| A4  | 是否同意 GCP 計費             | 不同意（使用者 2026-11-19）                                                                                                                                                                                       | 不啟動 `kit/model_proxy`，不呼叫 Vertex |
| A5  | Hugging Face                  | 環境檔定在 `~/.config/ple/local.env`（權限 600，已建立，內含填寫說明）；使用者已接受條款並填入 `HF_TOKEN`（2026-11-19 實測：帶 token 下載題目檔回 200，不帶回 401）                                               | L5 WAITING；L1–L4 照做                  |

A3 原案（Gateway）未查證的部分，L2 已實測不通（見 .runtime/ple-env/l2-findings.md）：Gateway 是否提供 OpenAI Chat Completions 格式且串流與非串流都回工具呼叫、能否從 Docker 橋接位址連到。L2 就是查這件事。不通就把 L2 標 BLOCKED 並回報實測結果，不改 Gateway repo，不自行改用會花錢的通路。遇到訂閱額度或速率限制如實回報，不盲目重試。

## 每到邊界先檢查（開始下一個子任務、發現新工作、卡住、想宣告完成時）

1. 我現在要做的事，對應 L1–L7 哪一條？答不出來就不做，記到下面「延後事項」。
2. 不做它，那一條的驗收命令會不會失敗？不會就是改進，不是阻塞，記到「延後事項」。
3. 上一輪有沒有新的有效證據？有效證據是：多過一關、確認了某個失敗原因、排除了某個假設、找到新的可測試解法、或拿到能縮小問題範圍的測試結果。同一個命令重跑得到同樣的失敗不算。沒有就在 checkpoint 寫明原因。

新發現的工作只分三類：阻塞（不做，當前這條過不了）立刻做；缺陷（做錯了，會讓已打勾的條件失效）立刻修；改進（其餘全部）只記錄。

## 允許與禁止

- 允許：在 A1 授權的位置建 clone 與參賽資料夾；裝 harbor 0.20.0；起比賽工具自己的 Docker 容器；讀 `.runtime/ple-docs/` 與官方 repo；把命令輸出存到 `.runtime/ple-env/`。
- 禁止：提高分數的任何工作（提示詞、驗證、多 Agent、換對話、結構化計畫）；重構本 repo 既有 crate；改 Gateway repo；在本 repo 新增 crate、頂層項目或資料夾；推送到 GitHub 或替參賽資料夾設遠端；在本 repo commit（參賽資料夾的本機 commit 不在此限）；啟動會計費的模型通路；把金鑰、token 或題目內容寫進 Git 或日誌；跑完整 29 題；改寫 PLAN.md 的 Decided 文字。
- 本機服務若必須讓容器連到，只綁 Docker 橋接位址，不綁 `0.0.0.0`；我們的伺服器在比賽容器內監聽 `0.0.0.0` 是官方要求，不在此限。
- 題目與評分程式受 Hugging Face 存取條款約束（使用者 2026-11-19 申請時同意）：只留在 `~/.cache/plebench/`，不複製進任何 repo、不公開轉貼、不拿去訓練或微調模型；受測的 Agent 只能經評分端送來的 task 訊息拿到題目，不得把題目、評分程式或評分細則另外放進提示詞、知識庫或它讀得到的檔案。

## 停止條件

- COMPLETE：L1–L7 都有證據路徑且各自的負向對照都跑過。
- WAITING：缺 A1–A5 的授權或 `HF_TOKEN`；先把不依賴它的條目做完再停。
- BLOCKED：同一條連續 3 輪沒有新的有效證據（定義見上面第 3 問），而且手上沒有還沒試過、可測試的解法；或官方工具本身在 L3 就失敗（那是環境問題，回報輸出，不改官方程式）。
- 停下時回報：哪幾條已驗證、卡在哪條的哪一關、試過什麼、需要使用者決定什麼。

## 延後事項（只記不做）

| 發現日期 | 內容 | 為什麼不是阻塞 |
| -------- | ---- | -------------- |
|          |      |                |

## 完成後接哪裡

PLAN.md 比賽版 C1 剩下的部分（確認 Rust 在比賽建置環境能編譯）若 L4 已涵蓋就一併標示；接著是 C2 先交件與 C3 基準量測。Windows、macOS 上的本機跑分環境不在這張票，沒有跑過不得宣稱支援。
