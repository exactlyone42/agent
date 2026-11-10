# CLI 開發入口

目前可在明確授權後，讓模型讀取一個文字來源、建立一個新成果，再由程式核對成果，並用 Ctrl+C 要求停止。已保存檔案任務規格、工具動作、批准與執行結果，可逐次批准或拒絕，關閉 CLI 後可再查詢。這是實作中的入口，尚非正式企業產品；同終端互動介面已實作基本指令，尚待完整驗收；正式安裝包、完整管理者政策、可靠接續、知識與電腦操作仍未交付。

正式產品的模型請求只送往 Gateway，由 Gateway 連接及管理模型；Agent 不負責啟動模型服務。下方直連本機模型的方式僅用於開發測試，不是正式產品接法。真 Gateway 尚未完成端到端驗收，當前程式也只允許本機 IP，遠端 Gateway 尚不能使用。

## 互動短指令（開發中）

不帶參數啟動 `agent-platform` 後，輸入 `/help` 可查看短指令。`/version` 回報程式、錯誤、事件與資料庫版本，以及目前能力；不讀取任務資料庫，也不呼叫模型。`/list` 查看既有任務、最後保存的狀態與工作區；`/list next` 翻下一頁，沒有下一頁會明示。以 `/run task.json` 啟動任務後，`/status` 查看目前任務狀態，`/events` 看前 100 筆紀錄；逐次批准模式會顯示待批准動作與內容，也可輸入 `/approvals` 再看一次；確認後輸入 `/approve` 或 `/deny`，不必輸入編號和版本。`/exit` 離開；任務仍在執行時不能離開。現階段仍須準備任務 JSON，沒有持續對話。這是開發中的介面，尚未完成真人驗收。

互動模式輸入 `/cancel` 可要求停止本畫面啟動的任務，不必輸入編號。另一個互動 CLI 先輸入 `/list`，再用 `/cancel 1` 停止清單第一個任務；此畫面已有任務時先停止目前任務。要求停止後不能再批准該任務；等畫面顯示「任務已取消」，再以 `/status` 讀回「已取消」，才算保存完成。關閉互動輸入，或互動 CLI 意外結束，也會要求子任務停止。已建立的成果會保留，請用 `/result` 核對；同步檔案操作卡住時仍不保證立即停止。非互動 `run` 沿用 Ctrl+C。

新任務的啟動通知無法送出時，程式會先保存取消結果再以錯誤退出；連錯誤訊息也無法送出則回報退出碼 74。重新開啟 CLI 後可用 `/list` 查看保存狀態。

## 開發測試：接本機 Qwen 模型

這份教學使用 Linux、已安裝的 Rust 和 Ollama，以及已下載的 `qwen3.8:27b`。目前沒有安裝包，先從原始碼編譯；它還不是互動聊天工具，只能執行指定來源與成果檔案的任務。

1. 在終端確認 Ollama 已啟動，且模型清單中有 `qwen3.8:27b`：

   ```sh
   ollama list
   ```

   如果無法連線，先啟動本機 Ollama 服務，再重新查詢。不要把範例模型名稱當成已安裝；未安裝時需先自行準備模型。

2. 在 Agent 專案目錄編譯，將產生 `target/debug/agent-platform`：

   ```sh
   cd /home/cash/work_space/private/agent-platform
   cargo build --locked -p agent-platform
   ```

   這個路徑是本機範例；專案放在其他位置時，請改成實際路徑。

3. 建立新的測試資料夾與來源檔，避免碰到既有成果。以下指令在同一個終端依序執行：

   ```sh
   test_dir=$(mktemp -d)
   cd "$test_dir"
   printf 'hello' > input.txt
   cat > task.json <<'EOF'
   {
     "endpoint": "http://127.0.0.1:11434/v1",
     "model": "qwen3.8:27b",
     "task": {
       "prompt": "讀取來源，把 hello 改成 HELLO，寫入成果。不要加換行。",
       "input": "input.txt",
       "output": "output.txt",
       "expected": "HELLO",
       "allow_read": true,
       "allow_write": true
     }
   }
   EOF
   ```

   `endpoint` 是模型服務網址，`model` 是模型名稱；要換模型時修改這兩個欄位。`allow_read` 和 `allow_write` 明確授權這次讀取及建立成果；`expected` 讓程式核對成果內容。

4. 執行任務並讀回成果：

   ```sh
   /home/cash/work_space/private/agent-platform/target/debug/agent-platform run task.json
   cat output.txt
   ```

   預期結果是 `HELLO`，程式回報 `artifact: verified`。執行時會顯示任務編號，留著可用下方指令查詢。成果檔必須事先不存在；重跑前先核對現有檔案，另建測試資料夾，不要直接覆寫。

2026-11-02 已在 Linux 以此模型實際執行相同任務，讀回來源 `hello` 與成果 `HELLO`。這只證明該次檔案任務，不能推論複雜任務、崩潰恢復或其他平台已驗收。

## 常用指令

以下用 `agent-platform` 代表上面編譯出的完整路徑；若未加入 `PATH`，請替換成 `/home/cash/work_space/private/agent-platform/target/debug/agent-platform`。

```sh
agent-platform --help
agent-platform version
agent-platform list
agent-platform list <上一頁的next_cursor>
agent-platform run task.json
agent-platform resume <任務編號>
agent-platform cancel <任務編號>
agent-platform instruct <任務編號> <訊息編號> <文字>
agent-platform status <任務編號>
agent-platform events <任務編號> 0
agent-platform approvals <任務編號>
agent-platform approve <批准編號> <任務版本>
agent-platform deny <批准編號> <任務版本>
```

`list` 每頁最多 100 筆，依任務編號字串升冪排序，不代表建立時間；輸出含 `schema_version:1`、`tasks` 及 `next_cursor`。下一頁沿用回傳位置，空值表示已列完；查詢過程有新任務插入時，可重新從第一頁查看。舊任務沒有保存工作區時回傳 null，互動介面顯示「未保存工作區」，不猜路徑。規格損壞會拒絕整頁，不輸出部分成功；讀取舊版資料不自動升級，找不到資料庫時明示錯誤、不建立空庫。

`instruct` 對執行中或等待批准的任務補一句指令：原文（1–8192 位元組）連同訊息編號（1–128 字元）存進任務庫並依序編號。同一訊息編號重送相同文字回傳原序號與 `duplicate:true`，不同文字回 `task_conflict`；任務不在執行中或不存在則分別回 `task_conflict`、`task_not_found`。指令在下一個模型回合開始前（上一輪工具都已結算）以「補充指令 N：原文」加進對話，與對話保存在同一筆交易內改為 `applied`；先存對話再改狀態的中斷不會重複加入，因為接續時會比對對話裡是否已有同文。已要求取消時不再加入，指令留在 `received`；任務若在最後一個回合收到指令，也留在 `received`。每個任務最多同時 16 筆未被模型讀取的指令，超過回 `instruction_backlog`，重送已收下的訊息編號不受限；`status` 輸出的 `pending_instructions` 是尚未被模型讀取的指令數（舊版任務庫固定為 0）。

`status` 查最後保存的狀態，`events` 從序號 0 開始讀紀錄；`approvals`、`approve`、`deny` 用於下方的逐次批准模式。任務執行中按 Ctrl+C 可要求停止；停止後仍須檢查成果是否已建立。

`cancel` 先按目前執行次數保存 `cancel_requested`（已要求停止），立即擋住新工具申請、領用批准及晚到的批准。原執行者接收要求後保存歷史及結果；命令必須取得同一任務鎖並讀回 `cancelled`，才回傳結束碼 0 與 `schema_version:1`、`confirmed:true`；`confirmation_scope:local_task` 明示只確認本機任務，`external_stop_confirmed:false` 表示尚未證明外部服務已停止。重送同一次要求不新增事件；舊次要求不能取消新次。

最多等待五秒仍無法確認，或工具結果未明時，結束碼為 1、錯誤代碼 `cancel_unconfirmed`，不回傳確認成功。執行者已意外消失但尚無結束回執，也只保存停止要求；請核對狀態、成果及紀錄，不能把任務鎖已空當成副作用已確認。同步檔案操作卡住時可能超過等待期限，目前尚無逾時後強制終止原執行者的路徑，因此 R2 的五秒停止要求未完成；本機停止不表示外部模型已停止計算。子程序樹、真 Gateway 取消、安裝後與真人驗收仍待完成。

## 中斷後接續（開發中）

重新開啟互動 CLI，先輸入 `/list`，再用 `/resume 1` 選擇該頁第一個任務，不必手輸任務編號或版本；同畫面仍保有目前任務時可直接 `/resume`。非互動入口是 `resume <任務編號>`，沿用保存的工作區、來源、成果、模型及授權範圍，不再讀任務 JSON。

接續前會取得同一任務的程序鎖；原程序仍持鎖時拒絕。原次狀態與原始對話保存在 `task_runs`，新一次執行使用新的 `generation`，事件序號繼續累加。尚未使用的舊批准會撤銷；確定未派工的動作記為「未執行」，由模型重新提出。即使原任務採預先授權，接續的新動作仍須逐次批准；讀寫範圍不能擴大。每次執行最多八回合、十五分鐘，同一任務的工具呼叫總上限仍為六十四次。

只有成功回執、保存的工具結果及目前檔案內容都相符時，才恢復已寫入成果，並沿用唯讀檔案重新驗證；不會再寫一次。`/result` 會配對前次成功回執及來源。結果未知、歷史矛盾、檔案被改或只有檔案而無寫入回執時拒絕接續，不盲重送或覆寫。已成功或等待人工驗收的任務請先讀回成果，不直接接續。

2026-11-10 已用假模型與實際 CLI 子程序驗證：等待批准時終止後，從清單接續並另批成功；寫入成功後終止，重開保留成果與原來源，新動作另行確認；回執失敗後拒絕接續，零模型請求且任務紀錄不變。這些本機測試不能代替真 Gateway、真人、安裝後或完整檢查；R1 尚未交付。

## 如何執行

開發環境先在 repo 根目錄執行 `cargo build --locked -p agent-platform`。Linux／macOS 的程式在 `target/debug/agent-platform`，Windows 在 `target/debug/agent-platform.exe`；目前僅有 Linux 實際執行證據，其他平台未驗收。

先準備已獲授權使用、支援 OpenAI Chat Completions 的本機模型服務。以下網址與模型名稱是範例，必須替換成現有服務；本命令不會啟動服務。

```json
{
  "endpoint": "http://127.0.0.1:8080/v1",
  "model": "your-model",
  "task": {
    "prompt": "讀取來源，把 hello 改成 HELLO，寫入成果。不要加換行。",
    "input": "input.txt",
    "output": "output.txt",
    "expected": "HELLO",
    "allow_read": true,
    "allow_write": true
  }
}
```

將設定存為 `task.json`，執行 `target/debug/agent-platform run task.json`。相對檔案路徑以執行命令時的工作目錄為準。來源可先放入 `hello`；成果必須尚未存在。服務若要求 API 金鑰，透過 `AGENT_API_KEY` 環境變數提供，不寫入任務檔。

## 逐次批准或拒絕

在任務 JSON 最外層加入 `"approval_mode": "manual"`，保留 `allow_read`／`allow_write` 指定可考慮批准的讀寫範圍。false 仍是拒絕，逐次模式不能把它提升為 true。省略模式時沿用先前的明確授權，程式會保存 `task_policy` 政策批准；模型無權批准自己。

啟動任務後，依顯示的任務編號，在另一個終端執行：

```sh
target/debug/agent-platform approvals <任務編號>
target/debug/agent-platform approve <批准編號> <任務版本>
target/debug/agent-platform deny <批准編號> <任務版本>
```

先看 `tool_name`（read_input 讀來源、write_output 建立成果）、`scope_path` 指定檔案及 `args_json` 完整參數，再選 approve 或 deny；參數中的 content 是將寫入的內容。每筆都有 `approval_id` 與 `spec_revision`，必須原樣帶回，不能拿別的任務或版本的批准代用。此命令回傳 JSON 供人工及程式讀取，目前沒有同終端的差異畫面，不能當成桌面或正式人類易用性驗收完成。

每筆批准有效 120 秒，只可領用一次；逐次模式整個任務上限 15 分鐘，仍包含模型及等待時間。批准前不執行工具；拒絕、過期或取消後不領用批准。批准已被領用時，deny 會拒絕，不能假稱已撤回開始的動作；請用原執行終端停止，或用另一個 CLI 的 cancel 要求停止，再核對成果。每個任務最多 64 次工具呼叫、最多 8 個模型回合。批准查詢最多 100 筆，已涵蓋這個呼叫上限。

逐次模式拒絕一個動作後，模型可繼續判斷；若提出新動作，會重新顯示並等待另一筆批准。拒絕本身不執行工具，也不批准後續動作；要停止整個任務請用 `/cancel`。

工具準備執行前，程式在同一筆交易內領用批准並記為 dispatched（已派發）；結果回來後才記 succeeded 或 failed。沒有確定回執就保留 outcome_unknown，不能把 SQLite 交易視為檔案操作也必定成功。任務正常結束會撤銷未用批准並保存事件；強制殺死程序的遺留狀態仍要由後續恢復程序處理，目前不自動接續或重派工具。

現階段是同一個作業系統使用者的本機入口；`decision_source: local_cli` 表示決定由 CLI 提交，不是企業身分驗證或管理者簽章。任務及政策版本目前從 1 開始，修改版本、角色限制與受管理政策仍待交付。程式不提供任意 SQL 或修改既有批准參數的入口。

## 如何判讀結果

執行開始後，標準錯誤輸出會顯示「任務編號」。成功、待人工核對及正常取消的 JSON 結果也包含 `task_id`。資料庫無法保存開始紀錄時，不呼叫模型；成果已建立但結束紀錄保存失敗時回傳 1，提示結果未確認，不輸出成功收據，也不重跑工具。

| 結束碼 | 意義                                          | 下一步                                               |
| ------ | --------------------------------------------- | ---------------------------------------------------- |
| 0      | 成果內容與 `expected` 的 UTF-8 位元組完全相同 | 只代表這項內容核對通過，不代表其他需求或企業驗收通過 |
| 2      | 已建立成果，但沒有提供 `expected`             | 真人核對成果；不得自動當作成功                       |
| 1      | 參數、模型、讀寫或驗證失敗                    | 核對現有成果後再決定下一步，不能盲目重送             |
| 130    | 使用者要求停止或強制結束                      | 必須再看有無取消結果；只有結束碼不能證明停止範圍     |

`expected` 可省略或設為 `null`。即使模型說工作完成，沒有實際成果仍回報失敗；內容錯誤也不會通過。失敗時可能已留下部分或錯誤成果，程式保留它供核對，不自動刪除或覆寫。

## 停止任務

按一次 Ctrl+C（Unix 的 SIGTERM／SIGHUP 亦同），CLI 通知共用執行層停止；尚未開始的讀寫工具會拒絕執行，等待中的模型請求會被放下。正常停止輸出包含 `task_id`、`state: "cancelled"` 及 `artifact_created`；若本次已建立成果，最後一欄為 `true`，即使後續寫入失敗也不誤報沒有成果。這個欄位不是內容驗證結果，也不表示其他程序未建立或移動同一路徑的檔案。

同步檔案操作已經開始時，第一次停止要求不保證立即打斷它。再次按 Ctrl+C 可強制結束本機程序；此時可能沒有 JSON 結果，必須視為結果未確認，人工核對檔案，不能盲目重送。已寫入或部分寫入的檔案一律保留，不自動回滾。這些行為只涵蓋目前的本機檔案工具；遠端模型是否仍在計算、日後外部工具程序樹與跨程序接續尚未驗收。

## 關閉後查詢任務

```sh
target/debug/agent-platform status <任務編號>
target/debug/agent-platform events <任務編號> 0
```

`status` 回傳最後保存的 `recorded_state`、事件位置 `last_seq`、UTC 更新時間、紀錄版本及執行世代。`artifact_created` 為 true／false／null，null 表示當時不能確認；它不是成果檔目前仍存在的保證。`running` 只代表最後保存的是開始紀錄，不能確認程序仍活著。強制終止、主機斷電或結果保存失敗後，必須先核對原程序與成果；這兩個查詢命令不會接續或重新派工具。

`events` 每行一筆 JSON，依任務內序號遞增，每次最多 100 筆。最後參數是已讀序號，0 從頭讀；下一頁使用上一頁最後一筆的 `seq`，直到空頁。這是逐頁讀取當時已保存的事件，不是跨命令固定快照；查詢過程不修改任務資料。找不到任務、負數或非整數位置均拒絕。

在命令最後加 `--follow` 可持續讀取，例如 `agent-platform events <任務編號> 0 --follow`。讀完目前事件後，每 200 毫秒查詢新紀錄；每筆 JSON 立即送出。按 Ctrl+C 只停止讀取，不取消任務。斷線後以最後完整收到那筆的 `seq` 重新執行，可從下一筆接續；省略已讀序號則從頭開始。事件查詢不代表任務成功，仍須用 `result` 獨立讀回成果。

預設使用作業系統的個人本機資料目錄，由 directories 6.0.0 決定；Linux 通常是 `~/.local/share/agent-platform`，macOS 是 `~/Library/Application Support/agent-platform`，Windows 是個人的 LocalAppData 下 `agent-platform/data`。可用 `AGENT_STATE_DIR` 指定本機資料目錄的絕對路徑。Unix 新目錄權限為 700；既有目錄若允許其他使用者存取便拒絕，最末層目錄／資料庫不接受符號連結，Unix 資料庫也拒絕硬連結與管線。

資料庫名稱為 `tasks.sqlite3`。除了狀態與事件，目前也保存本次檔案任務的需求、工作目錄、模型入口、核對內容、讀寫範圍，以及待批准的工具參數；其中可能含使用者文件內容，應放在個人受限資料目錄。API 金鑰不保存於任務規格，事件本身不放原文；`approvals` 只有在使用者明確查閱時才輸出完整參數。這些紀錄仍未構成完整恢復能力，不提供自動續跑。

狀態與事件使用同一筆交易，過時結果或重複結果拒絕。資料庫每次忙碌等待上限 500 ms；等待批准期間只有 SQLite 明確的 BUSY 才在批准期限內繼續查詢，尚未派工具，不重送任何工具副作用。其他資料錯誤直接停止。舊版資料使用順序遷移，已套用的 SQL 不原位改寫；approve／deny 只開既有資料庫，不會因路徑打錯而建立空庫或升級資料。

使用固定版本 rusqlite 0.41.0 與內附 SQLite，開啟 WAL 日誌、FULL 同步及外鍵，讀回設定確認生效。WAL 要求本機檔案系統，**目前尚未自動辨識所有網路掛載，不可放在 NAS、NFS、SMB 或同步資料夾**。Windows ACL、父層路徑替換、網路掛載拒絕及備份還原仍是未過的企業發布門檻；本機 700 權限檢查不能替代它們。直接取得資料庫檔案寫入權的人仍可改資料；這不是防竄改稽核服務。

中斷處理使用 ctrlc 3.5.2 的獨立執行緒，共用取消訊號使用已鎖定的 tokio-util 0.7.20。Linux 已測模型等待中取消、寫入後取消、寫入失敗後取消、開檔卡住後第二次強制結束；原接法的失敗對照已隨舊證據資料夾移除，可從 Git 歷史取回。Windows／macOS 的控制台實機尚未驗收。

## 授權與目前限制

讀寫權限預設皆拒絕；模型只能操作任務預先指定的檔案，不能自行選路徑。成果使用排他建立，既有檔案不覆寫。這不是完整的作業系統隔離；目錄被其他程序更換、檔案系統卡住及程序崩潰的處理仍待後續整合。

非互動的預先授權模式若遇到未授權的讀取或寫入，立即停止任務，回報 `approval_denied`，不再向模型發送後續請求。逐次批准模式中的使用者拒絕仍回報模型，讓模型結束或改採其他已獲准的動作；拒絕的工具不執行。

任務 JSON 上限 64 KiB，來源與成果各上限 1 MiB。只接受明確的本機 IP 網址，不接受網域名稱、網址夾帶憑證、查詢參數或重新導向；不讀系統代理設定。本機服務是否再將資料送往遠端，仍由服務自身設定決定。

模型最多執行 8 輪；預先授權模式整個任務上限 120 秒，逐次批准模式為 900 秒。模型請求沿用該模式上限，仍由整個任務時限與取消控制，不另外在 60 秒切斷。來源與任務設定檔共用開檔檢查。Unix 以不等待管線的方式開啟，核對一般檔案、大小及開檔前後的裝置與檔案編號；Linux 已測目錄、管線、其他一般檔案及符號連結替換皆拒絕。Windows 先以只讀取檔案資訊的權限保留控制代碼，再開啟讀取內容的控制代碼，比對兩者的 128 位檔案編號與磁碟編號，並限制開啟期間的改寫與刪除；該實作仍待 Windows 實機驗證，這些時間限制也不能中止卡住的同步檔案讀寫，不代表已具備可靠取消或故障恢復。模型最後文字會出現在結果中；其中內容不是獨立驗證證據。

開發測試是 `crates/cli/tests/cli/` 的 Rust 整合測試（`cargo test -p agent-platform --test cli`）：以標準函式庫啟動短期本機假模型服務，再呼叫實際編譯的 CLI。每個案例各用一個臨時目錄當工作目錄與 `AGENT_STATE_DIR`，只對子程序設定環境，並有 60 秒總時限，逾時即結束子程序並判失敗；同一測試內的子案例遇到第一個失敗就停止，讓變異測試能在時限內看到失敗。Linux 的路徑替換測試另需 C 編譯器 `cc`：只在測試子程序載入攔截程式，將臨時來源改為目錄、管線、其他一般檔案或符號連結，確認開檔後仍拒絕。原內容保存在同一臨時目錄，測試結束自動清理，不改使用者資料或全域環境。信號、管線與權限案例僅 Unix 執行；Windows 尚無對應測試。產品使用者不需要 C 編譯器。

這些測試不證明真實模型品質、效能或 Linux 以外平台的安裝；Linux 路徑替換案例不代表 Windows／macOS 的同類情況已驗證。

## 檔案隔離的後續接法

2026-10-26 查閱官方介面：Rust 的 [Unix 開檔選項](https://doc.rust-lang.org/std/os/unix/fs/trait.OpenOptionsExt.html)可傳入作業系統旗標，[Windows 開檔選項](https://doc.rust-lang.org/std/os/windows/fs/trait.OpenOptionsExt.html)可限制其他程序同時讀寫、刪除或改名；兩者不是相同的安全保證。[cap-std 的已開啟目錄介面](https://docs.rs/cap-std/latest/cap_std/fs/struct.Dir.html)可作為限制工具工作目錄的候選；[rustix 的 openat](https://docs.rs/rustix/latest/rustix/fs/fn.openat.html)提供相對於目錄控制代碼開檔的系統介面。

本案推論：完整工具層應以已授權的目錄控制代碼執行，避免只靠字串路徑與先檢查後開檔。必須補上路徑替換、連結逃逸與成果被移走的反例，再選固定版本接入；目前尚未採用或驗證上述候選，不能把官方介面存在當作產品已隔離。查閱頁面版本與本機 Rust 版本可能不同，接入時以鎖定版本編譯及各平台實測為準。

本次 Unix 開檔旗標使用既有鎖檔中的 libc 0.2.191，已核對本機套件宣告為 MIT OR Apache-2.0。same-file 1.0.6 的官方文件與 Windows 原始碼提示部分檔案系統可能誤判身分，因此未直接採作安全授權的裁判；Windows 穩定版標準庫的檔案編號介面也尚未穩定，須另接系統介面並驗證。

Windows 實作直接使用既有 windows-sys 0.61.2 的系統介面。Microsoft 文件要求以 [128 位檔案編號與磁碟編號](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_id_info)共同辨識檔案；[讀取介面](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-getfileinformationbyhandleex)失敗時必須回報錯誤。本案使用固定大小的初始化結構接收結果；控制代碼由 Rust File 保持有效，只有系統呼叫成功才讀取結果，不將不支援此查詢的檔案系統默認為通過。

### 批准與取消同時發生時

工具執行後若資料庫暫時忙碌，只重試保存結果，不重做工具。重試窗口兩秒，最後一筆資料庫等待最多另占 500 毫秒；持續失敗就停止並要求核對成果。若連收尾都無法寫入，查詢只能顯示最後保存的狀態，不能把它當成程序仍在執行的證據。

模型提供錯誤工具名稱或參數時，該動作會被拒絕並留下事件。若保存的任務規格損壞，則整個任務停止，不再請模型繼續；查詢狀態會保留「結果未確認」，須先核對資料與成果。

任務收尾以執行批次編號核對身分，不以最後一筆事件編號判斷。因此另一個 CLI 剛寫入批准紀錄，不會讓目前執行者無法取消；舊批次仍不能替新批次收尾。收尾交易會撤銷尚未使用的批准。

工具已開始執行、但尚未留下結果時，取消後保留 `outcome_unknown`（結果未確認），不宣稱已安全取消。CLI 的取消回覆採用資料庫實際狀態；仍需核對已建立的成果，不能直接重送。

## 錯誤輸出契約（schema_version 1）

任何失敗（結束碼 1）時，標準錯誤的**最後一行**是一筆 JSON：`{"schema_version":1,"error_code":"...","message":"..."}`。程式與桌面端只看 `error_code`，不解析 `message` 的中文。`message` 給人看，字句可能調整；`error_code` 一經發布不改名，只能新增。

固定代碼：`usage`、`invalid_argument`、`invalid_state_dir`、`state_dir_unavailable`、`invalid_database_file`、`database_unavailable`、`task_file_unreadable`、`task_file_too_large`、`invalid_task_file`、`invalid_model_endpoint`、`invalid_api_key`、`model_client_failed`、`signal_handler_failed`、`workspace_unavailable`、`output_failed`、`store_state_poisoned`、`outcome_unknown`。

來自任務紀錄：`store_database`、`store_incompatible`、`task_not_found`、`task_conflict`、`resume_unsettled`、`session_unavailable`、`invalid_cursor`、`invalid_task_spec`、`invalid_tool_call`。來自任務執行：`approval_denied`、`invalid_task`、`invalid_input`、`input_unreadable`、`model_run_failed`、`task_timeout`、`tool_state_error`、`artifact_missing`、`artifact_incomplete`、`verification_failed`。

事件每筆已帶 `schema_version`；新事件為版本 4，資料庫結構為版本 7，錯誤輸出為版本 1。三者分開演進，`version`／`/version` 分別回報；舊事件保留自己的版本，不能依資料庫版本猜事件格式。版本回覆本身的 `schema_version` 為 1，`program_version` 為執行檔版本。`capabilities` 只列目前已實作的任務操作，不代表真 Gateway 或企業驗收已完成。尚未涵蓋：以結束碼 130 或 2 結束的情況（取消、待人工核對）不輸出錯誤行；`run` 之外的成功輸出沒有版本欄位。測試：`crates/cli/tests/cli/error_codes.rs` 涵蓋 5 個代碼，其餘代碼未逐一測試。

## 成果讀回（開發中）

`result <任務編號>`輸出保存的執行時來源、目前成果與重新驗證結果；互動介面用`/result`查看本次任務。來源與成果是兩個檔案，輸出before／after呈現兩份內容，不表示改寫來源。來源內容取自成功讀取的保存紀錄，不讀現在來源冒充原件；缺紀錄時before為null，互動介面明說無法還原。成果不符expected或已保存的寫入內容時回錯，不宣稱成功。verified須任務成功、歷史已完成且無未知結果、保存寫入與expected兩項皆符合。無expected仍需人工核對。

已做隔離CLI測試；真Gateway、互動入口完整測試、路徑安全反例及新版完整檢查尚待完成，不是正式交付。

模型回應中的 `reasoning` 推理文字會保存在本機任務紀錄，接續時原樣送回相同格式的模型入口。其他未知的非空內容與錯誤型別仍拒絕；不支援這份推理紀錄的入口會在送出前拒絕。推理文字從資料庫版本 5 開始保存；版本 6 另保存接續前的各次原始紀錄，版本 7 加入跨程序停止要求與狀態，舊版程式不能開啟新版庫。既有紀錄在啟動寫入工作時升級，原歷史內容保留。

完整工具回應附帶 `index` 序號時，必須與工具清單位置一致；缺少或為 null 時沿用清單順序，錯型別、負數或錯位即拒絕。任務紀錄與重送保留清單次序、工具編號及參數，不另外保存可由位置還原的重複序號。

指定完整成果內容（`expected`）時，寫入工具會在建立檔案前先比對；不符就把錯誤回給模型，檔案尚未建立。模型可修正後再提出新動作，逐次模式仍須另批；成功寫入後仍從檔案獨立驗證，寫前比對不能代替讀回。這處修補來自真接續時模型少末尾換行的實際反例，本機正反與快檢已過，真模型已在同任務修正成功；正式交付與新版完整檢查仍待驗。

2026-11-10 隔離真Gateway驗收：舊資料庫版本5的原CLI在等待讀取批准時被強制終止，新版升6並保留原始對話；同任務寫入成功後又被強制終止，接續仍沿用同一成果檔。原CLI重新讀回來源79位元組與成果77位元組，僅修正連結，末尾換行保留；成功寫入一次、最後接續沒有再派工具。共兩筆任務，一筆失敗保留、一筆完成；9筆模型請求全部使用同一免費來源，每筆只嘗試一次，不是9次任務成功。兩自有容器已停；正式環境未動。原件.runtime/p04-resume-gateway-20261110/acceptance-summary.json，固定執行檔及各次原件可核對；安裝、正式政策、真人及完整檢查尚未完成。

2026-11-10 跨程序取消本機驗收：等待批准、等待模型兩案已用原CLI操作，確認後十秒無新寫入，重新接續另批；舊資料升級、錯次拒絕、回執未明與取消／派工交錯有正反例。新增暫停執行者反例仍失敗，五秒終止要求尚未完成；223項快檢通過只對加入此反例前的來源，不代表R2或產品通過。原件.runtime/p04-cross-cancel-20261110/validation.json。
