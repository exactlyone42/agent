# CLI 開發入口

目前可在明確授權後，讓模型讀取一個文字來源、建立一個新成果，再由程式核對成果，並用 Ctrl+C 要求停止。已保存檔案任務規格、工具動作、批准與執行結果，可逐次批准或拒絕，關閉 CLI 後可再查詢。這是實作中的入口，尚非正式企業產品；安裝包、完整管理者政策、同終端互動介面、可靠接續、知識與電腦操作仍未交付。

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

每筆批准有效 120 秒，只可領用一次；逐次模式整個任務上限 15 分鐘，仍包含模型及等待時間。批准前不執行工具；拒絕、過期或取消後不領用批准。批准已被領用時，deny 會拒絕，不能假稱已撤回開始的動作；請用原執行終端停止並核對成果。每個任務最多 64 次工具呼叫、最多 8 個模型回合。批准查詢最多 100 筆，已涵蓋這個呼叫上限。

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

預設使用作業系統的個人本機資料目錄，由 directories 6.0.0 決定；Linux 通常是 `~/.local/share/agent-platform`，macOS 是 `~/Library/Application Support/agent-platform`，Windows 是個人的 LocalAppData 下 `agent-platform/data`。可用 `AGENT_STATE_DIR` 指定本機資料目錄的絕對路徑。Unix 新目錄權限為 700；既有目錄若允許其他使用者存取便拒絕，最末層目錄／資料庫不接受符號連結，Unix 資料庫也拒絕硬連結與管線。

資料庫名稱為 `tasks.sqlite3`。除了狀態與事件，目前也保存本次檔案任務的需求、工作目錄、模型入口、核對內容、讀寫範圍，以及待批准的工具參數；其中可能含使用者文件內容，應放在個人受限資料目錄。API 金鑰不保存於任務規格，事件本身不放原文；`approvals` 只有在使用者明確查閱時才輸出完整參數。這些紀錄仍未構成完整恢復能力，不提供自動續跑。

狀態與事件使用同一筆交易，過時結果或重複結果拒絕。資料庫每次忙碌等待上限 500 ms；等待批准期間只有 SQLite 明確的 BUSY 才在批准期限內繼續查詢，尚未派工具，不重送任何工具副作用。其他資料錯誤直接停止。舊版資料使用順序遷移，已套用的 SQL 不原位改寫；approve／deny 只開既有資料庫，不會因路徑打錯而建立空庫或升級資料。

使用固定版本 rusqlite 0.41.0 與內附 SQLite，開啟 WAL 日誌、FULL 同步及外鍵，讀回設定確認生效。WAL 要求本機檔案系統，**目前尚未自動辨識所有網路掛載，不可放在 NAS、NFS、SMB 或同步資料夾**。Windows ACL、父層路徑替換、網路掛載拒絕及備份還原仍是未過的企業發布門檻；本機 700 權限檢查不能替代它們。直接取得資料庫檔案寫入權的人仍可改資料；這不是防竄改稽核服務。

中斷處理使用 ctrlc 3.5.2 的獨立執行緒，共用取消訊號使用已鎖定的 tokio-util 0.7.20。Linux 已測模型等待中取消、寫入後取消、寫入失敗後取消、開檔卡住後第二次強制結束；原接法的失敗對照已隨舊證據資料夾移除，可從 Git 歷史取回。Windows／macOS 的控制台實機尚未驗收。

## 授權與目前限制

讀寫權限預設皆拒絕；模型只能操作任務預先指定的檔案，不能自行選路徑。成果使用排他建立，既有檔案不覆寫。這不是完整的作業系統隔離；目錄被其他程序更換、檔案系統卡住及程序崩潰的處理仍待後續整合。

任務 JSON 上限 64 KiB，來源與成果各上限 1 MiB。只接受明確的本機 IP 網址，不接受網域名稱、網址夾帶憑證、查詢參數或重新導向；不讀系統代理設定。本機服務是否再將資料送往遠端，仍由服務自身設定決定。

模型最多執行 8 輪，模型請求上限 60 秒、預先授權模式整個任務上限 120 秒，逐次批准模式為 900 秒。來源與任務設定檔共用開檔檢查。Unix 以不等待管線的方式開啟，核對一般檔案、大小及開檔前後的裝置與檔案編號；Linux 已測目錄、管線、其他一般檔案及符號連結替換皆拒絕。Windows 先以只讀取檔案資訊的權限保留控制代碼，再開啟讀取內容的控制代碼，比對兩者的 128 位檔案編號與磁碟編號，並限制開啟期間的改寫與刪除；該實作仍待 Windows 實機驗證，這些時間限制也不能中止卡住的同步檔案讀寫，不代表已具備可靠取消或故障恢復。模型最後文字會出現在結果中；其中內容不是獨立驗證證據。

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

來自任務紀錄：`store_database`、`store_incompatible`、`task_not_found`、`task_conflict`、`invalid_cursor`、`invalid_task_spec`、`invalid_tool_call`。來自任務執行：`approval_denied`、`invalid_task`、`invalid_input`、`input_unreadable`、`model_run_failed`、`task_timeout`、`tool_state_error`、`artifact_missing`、`artifact_incomplete`、`verification_failed`。

事件每筆已帶 `schema_version`（目前是任務資料庫版本 2，不是本節錯誤輸出的版本 1）；兩個版本號分開演進，是否合併成單一契約版本待 `version` 指令定案時決定。尚未涵蓋：以結束碼 130 或 2 結束的情況（取消、待人工核對）不輸出錯誤行；`run` 之外的成功輸出沒有版本欄位。測試：`crates/cli/tests/cli/error_codes.rs` 涵蓋 5 個代碼，其餘代碼未逐一測試。
