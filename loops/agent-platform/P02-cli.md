# P02 可用 CLI，連真 Gateway

母檔：../agent-platform.loop.md。依賴 P01。對應：CLI、任務執行、Gateway 連線 → Gateway。契約見 docs/cli.md。

## 目標

2026-11-05 重排：全案第二優先成果；價值是使用者能交代並驗收真實工作。先完成 P01，不以模型往返或 models 清單成功代替完整任務。

使用者選工作資料夾，經真 Gateway 完成「修復 Markdown 連結」任務；拒絕與取消零寫入；偽造成功文字被驗證器擋下。

人類使用介面：啟動一次後以 `/` 短指令操作，不要求每次重打程式名稱、任務編號或版本；批准時在同一介面顯示動作與內容，由使用者直接批准或拒絕。現有命令列保留供非互動用途；互動介面開發中，尚待完整驗收。

## 子任務

| id  | 做什麼                                                   | 驗收                           | 狀態                                                                                                                                                                                                                                                                                                                                       |
| --- | -------------------------------------------------------- | ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| C1  | `version`：回報契約版本與能力                            | 測試；輸出含版本               | DONE（版本與能力回報、雙入口正反測試及gate過；未發布） |
| C2  | `list`：列任務、狀態、工作區                             | 分頁、空清單、壞檔測試         | DONE（分頁與舊庫正反測試、gate、獨立審查及完整all過；未發布） |
| C3  | `result`：改了哪些檔、差異、驗證結果                     | 差異與實際檔案一致             | DONE（雙入口及正反測試、gate、簡化審查、完整all過：771變異656抓到115不可編譯零漏抓；未提交）                                                                                                                                                                                                                                                                                                                                  |
| C4  | `events --follow`：JSON 一行一筆，可從序號接續           | 斷線續讀不漏不重               | DONE（原CLI跨頁、即時輸出、中斷續讀及正反測試過；gate 198過零失敗略過；獨立審查逾時後自查；未提交發布） |
| C5 | 互動授權與非互動執行（缺授權即停） | 兩種模式正反測試 | VALIDATING（真拒絕零派工；新動作另批後成功，模型等待取消零成果；gate211過，必要all待跑，真人未驗） |
| C6 | Gateway 連線用虛擬金鑰連真 Gateway | 真請求成功；金鑰不進日誌 | RUNNING（隔離同版真Gateway與免費模型整合已過；正式路由與正式身分政策未驗通，不代替虛擬金鑰驗收） |
| C7 | 連結修復任務 + 獨立驗證器 | 偽造成功被擋；故意錯結果必失敗 | VALIDATING（原CLI真工作精確修連結並重新讀回；少換行錯成果被拒，必要all待跑） |
| C8  | P08 最小評測資料與入口                                   | 一鍵跑，正常與故障分開報       | PENDING                                                                                                                                                                                                                                                                                                                                    |
| C9  | 真人驗收：不看內部日誌能看懂改動並批准或拒絕             | 具名驗收人紀錄                 | WAITING                                                                                                                                                                                                                                                                                                                                    |

## 整條任務驗收

2026-11-05 隨 P01 完整檢查補驗互動入口：真 CLI 的空輸入、無任務查詢、狀態／事件、明示批准、重複通知、執行中禁止離開及離開後不再執行輸入；另以隔離子程序核對七種保存狀態與成果顯示。定向測試已通過，完整變異待新版結果；不代表真 Gateway 或真人 C9 已通過。

完成 P01 階段 1B 與 H5 後，以原 CLI 經真 Gateway 執行修復 Markdown 連結：顯示動作與差異、批准、寫入、獨立讀回並檢查連結。模型說成功但無成果、成果錯誤必須失敗；拒絕與取消後不得新增寫入。請求成功與整條任務通過分開記錄，秘密不得進日誌。供應商格式與路由仍由 Gateway 負責。C8 保存正常與故障的可重跑證據；MCP 另由 P02m 負責，不以內建檔案工具驗收替代。

## 完成條件

C1–C9 完成；CLI 初版交付點。未接真 Gateway 不得標完成。

真 Gateway 已架在本機（使用者 2026-11-04 告知），目前只允許本機 IP 的限制不擋 C6；遠端 Gateway 與 TLS 留待之後。C6 沿用已取得的網址、金鑰位置與模型資訊，先查三次失敗的原因與現場狀態；設定失效才另行取得，不盲重送。P02 其餘子任務等 P01 整合驗收與階段 1B 後再開工。

## 等待P01期間的唯讀準備（2026-11-06）

`.runtime/p02-gateway-readonly-preflight.json`：現有本機18088入口的/v1/models回200，20個模型，現有金鑰檔權限600。未送推論、未用CLI執行任務，不能更新C6為完成，也不推翻之前三次推論失敗；P01完整檢查期間沒有新建或重啟服務。P01通過後再依當時入口狀態做一次有界真任務，避免盲重送。

## 原CLI真Gateway失敗定位（2026-11-06）

本輪未改產品來源，以隔離資料執行原CLI連結修復（免費ollama/qwen3.8:27b、預先批准指定來源與新成果）。`.runtime/p02-gateway-task-1793905329162144224/result.json`：退出1、沒有成果；來源及任務保存於該目錄，並非C7成功。未做真人或逐次批准驗收。

`.runtime/p02-gateway-network-evidence.json`：Gateway回502，上游連線目標為127.0.0.1:11434；該Gateway使用bridge網路，主機模型服務僅監聽127.0.0.1:11434。容器內loopback不等於主機loopback；先核對既有本機模型轉接，不能盲重送或讓Agent改直連。未更改Gateway配置／正式服務，未發起付費路由。下一輪審查見 `.runtime/qwen-p02-network-next*`。

## C1版本查詢（2026-11-06）

`version`與`/version`已實作：程式版本、回覆契約1、錯誤1、事件3、資料庫4及已實作能力分開回報。不開任務庫、不呼叫模型，額外參數拒絕。`.runtime/p02-version-red.txt`的兩項新測試在舊版因不支援命令失敗；`.runtime/p02-version-green.txt`兩項通過、19項未選入定向執行，後續快檢全跑。測試另建立隔離參考資料庫，核對真實新事件與資料庫版本，並驗證無資料目錄／錯誤資料路徑不受影響、兩入口JSON一致。快檢及獨立審查結果見 `.runtime/p02-version-*`。

Gateway目前三個既有容器皆使用同一loopback上游，工作樹有其他活躍執行者，見 `.runtime/p02-gateway-ownership-audit.json`。無Offora工具可協調時不接管該工作樹或服務，先完成獨立CLI項；C6/C7仍未通過。P01較廣安全要求保留，不冒充整階段交付。

本輪gate退出0：174項通過、零失敗／略過；子程序篩選計數合計10，來自分別執行的渲染自測。獨立審查與裁決見 `.runtime/p02-version-review.txt`、`.runtime/p02-version-review-decision.md`。僅CLI版本輸出與既有DB常數公開，未改核心邏輯，未重跑完整變異；未提交、推送或部署。下一輪提案／回覆／裁決見 `.runtime/qwen-p02-list-next*`。

## C2任務清單（2026-11-06）

`list [清單位置]`與`/list`、`/list next`已實作：100筆有界查詢，依task_id字串排序，回傳狀態與工作區；不顯示prompt、模型或工具參數。v1沒有task_specs時工作區明示未知，唯讀不升級；錯游標、損壞規格、壞DB拒絕，不回傳部分成功。位置是上頁最後一項，非第101項，避免漏筆。

`.runtime/p02-list-red.txt`：舊版3項新測試均因不支援清單而斷言失敗；新版4項CLI整合測試與1項保存邊界測試過，gate過。103筆隨機ID逐項核對無重複／漏項、七種狀態、空頁、v1讀取不升級、規格完整性、128KiB精確邊界與錯JSON均有測試；查詢前後DB原件不變。獨立審查無發現，見 `.runtime/p02-list-independent-review.md`。

巢狀過深由gate擋下，改為TaskSummary自己驗證保存內容、提早返回，未放寬規則。既有六項store.rs變異例外只移動行號，原語句相同，無新增排除，見 `.runtime/p02-list-exclusion-relocation.json`。此輪改到任務庫讀取，必要完整all待完成，C2暫不DONE。未提交、推送、部署；Gateway未改，C6仍受阻。

新版完整all執行中：`.runtime/checks/1793906218-4023961/`，exec session 96107，日誌 `.runtime/p02-list-all.txt`。啟動快照包含新CLI測試，當時171個來源hash無差異，見 `.runtime/p02-list-all-start.json`；不修改來源、不重啟同一檢查。下一輪先收同一程序終局；真Gateway入口待協作者提供，同時保留之後C3成果查詢的獨立工作。下一輪三問原件與裁決見 `.runtime/qwen-p02-result-next*`。

## C2完整檢查終局

原exec96107退出0；`.runtime/checks/1793906218-4023961/summary.json`的passed及source_unchanged均為true。688個變異：584抓到、104無法編譯，零漏抓、零逾時；baseline另計。`.runtime/p02-list-all-final.json`再次核對171個來源，前後與目前hash完全相同。C2完成；P02整階段、真Gateway及真人仍未完成。沿用已保存的`.runtime/qwen-p02-result-next*`三問與裁決，Gateway無新入口時續C3獨立成果讀回；預算未知。未提交、推送、部署。

## C3讀回方案核對

已核對TaskSession會取writer鎖，不可用於成果查詢；歷史Message含工具回傳與寫入參數，但不能代替成果檔。成果查詢需唯讀快照、配對成功回執及generation，並獨立開啟成果比對expected；差異使用執行時讀取內容，缺失須明示。Qwen回覆與採納／反駁理由見`.runtime/p02-result-design.response.txt`、`.runtime/p02-result-design.decision.md`。預算及時間未知；尚未實作或測試，不宣稱C3完成。

C3進度：新增TaskStore.result_snapshot唯讀交易入口，核對spec hash及歷史generation/revision、有界讀取、舊schema缺失明示。1項整測過；刻意移除revision驗證後具名斷言失敗，恢復後過，見`.runtime/p02-result-snapshot-{green,negative}.txt`。初始red僅編譯缺API，不作陰性證據。尚未接成果檔／差異，gate與all未跑。下一輪三問及裁決見`.runtime/qwen-p02-result-snapshot-next*`。

C3成果讀回初步實作：成功回執同generation、參數hash與長度核對後配對工具歷史；實際成果再驗expected及保存寫入內容。3定向測試過，含無回執不列成功、改參數拒絕、錯成果拒絕、單獨檔案符合不算任務成功。見`.runtime/p02-result-snapshot-green.txt`。尚未CLI入口、完整差異與路徑安全反例；gate/all未跑。下一輪三問及裁決見`.runtime/qwen-p02-result-inspect-next*`。

C3雙入口已接：`result <任務編號>`及本次任務`/result`。隔離CLI新測試1過、25未選入；保存原文不受現在來源改動影響、錯成果拒絕、DB原件不變，見`.runtime/p02-result-cli-green.txt`。巢狀被gate擋後拆解析方法，未放寬門檻；gate曾過，新增CLI測試後未重跑。互動/result、路徑安全及缺歷史完整反例仍待補，必要all未跑。下一輪三問與裁決`.runtime/qwen-p02-result-cli-next*`。未提交部署。

C3路徑與互動：新增成果開檔時拒絕最末檔名符號連結，原來源讀取語義不變；移除防護的反例具名斷言失敗、恢復過，見`.runtime/p02-result-link-{green,negative}.txt`。缺歷史不verified、互動顯示實際成果並明示無法還原差異均過。新版gate退出0；必要all現執行中exec66309，原件`.runtime/checks/1793915515-1290077/`，日誌`.runtime/p02-result-all.txt`。固定來源不改不重啟；不宣稱父路徑全面防護或C3完成。Qwen三問與裁決`.runtime/qwen-p02-result-safety-next*`。

## C4 持續事件讀取（2026-11-09）

events末尾加--follow，逐頁讀完再每200毫秒查新紀錄；每筆JSON立即送出。Ctrl+C只停讀不取消任務，以最後完整收到的seq重接。不帶旗標仍最多100筆。原CLI測試跨205筆歷史及兩筆新增事件，中斷後續讀沒有漏讀或重複；有效DB錯游標、不存在任務及Linux輸出失敗均拒絕。舊版同驗收失敗，原件p02-events-red.txt。

最終證據.runtime/p02-events-validation.json：gate退出0，198過、零失敗／略過，四個本次程式檔前後hash相同。未改核心，完整all未重跑，舊all不當新版證據。獨立審查90秒逾時，依不可用例外自查，未宣稱獨立通過；裁決.runtime/p02-events-review-decision.md。

下一輪三問已用qwen3.8:27b實跑；因誤解Gateway金鑰及本機DB事件，不採用其建議，原件與裁決.runtime/qwen-p02-events-next*。唯一下一步核現行Gateway入口與免費路由，推C6/C7原CLI真任務；若仍受阻續C5。真人、跨平台訊號、滿管線阻塞時停止未驗；未提交、推送或部署。

## 2026-11-09 C6現場與C5缺授權即停

Gateway現行18105、Agent專用key可通過models驗證；EXTRA_PROVIDER_PROFILES_JSON仍指容器自己的127.0.0.1:11434。容器內curl退出7，主機只聽loopback；證據.runtime/p02-gateway-current-{preflight,network}.json。本輪推論0、未改Gateway服務，不把models成功當C6通過。

C5找到真缺口：非manual拒絕原走Skip再叫模型，改為deny_and_stop立即停止；manual拒絕保留。原版兩項具名失敗，新版批准17項與原CLI同一整測19情境通過；讀／寫缺權限分別只1／2次模型請求，無成果且來源不變。陰性與綠件.runtime/p02-noninteractive-*。

批准核心改動已啟動完整all exec52967，PID457200，.runtime/checks/1794224985-457200/；gate及coverage過，779變異進行中，尚未DONE，不修改來源或另啟。獨立審查90秒逾時後自查，原件與下一輪Qwen裁決.runtime/p02-noninteractive-review-decision.md。唯一下一步收同一all終局。未提交、推送、部署；真Gateway、同終端取消、可靠接續與真人仍未過。

同一all等待核對：exec52967、PID457200與子程序472975仍活躍；已定位回執保存的精確到期變異（<改<=）漏抓，見.runtime/p02-noninteractive-live-observation.json。未把它判等價或排除，也未改來源／重啟。唯一下一步先收全部779項終局，再依具體反例補必要邊界驗證；Qwen提案與裁決.runtime/qwen-p02-deadline-next*。

等待期間只在.runtime/p02-deadline-candidate/保存候選：沿用Fake與暫停的時間，驗回執時限恰好到期時遇BUSY必停、不得再消耗下一次成功；只加dev test-util功能。候選rustfmt及git apply --check過，來源hash未變，未套用、未編譯、正反執行皆未跑。原完整all仍持同一exec52967；終局後核候選manifest來源hash，再決定套用與有界正反驗收。不能將草案稱為已修好。

## 2026-11-09 已知漏抓修補後的最新狀態

此前all執行中與候選未套用的紀錄已被本節取代。原all因已知漏抓送SIGINT中止，實際終局退出1，269/779（208抓到、59不可編譯、2漏抓、510未跑），179個原始來源hash全還原；原件保留，不宣稱完整通過。核孤兒測試程序的已刪暫存工作區後停止，未碰正式程序或資料。

精確到期測試已套用；事件讀取改用既有Tokio等待以驗三種等待情況，無新增時鐘抽象或依賴版本。兩項都正常版過、破壞版具名斷言失敗（101）、恢復原碼再過。gate200過、零略過；獨立審查逾時，依例外自查，未獨立通過。來源雜湊與原件.runtime/p02-boundaries-validation.json；裁決.runtime/p02-boundaries-review-decision.md。

唯一下一步C5原互動CLI可操作取消；同一核心切片穩定後仍須完整all才可提交，不以定向測試冒充。Qwen三問及不採理由.runtime/qwen-p02-cancel-next*。未提交、推送、部署；真Gateway工作、可靠接續及真人仍未過。

## 2026-11-09 原互動CLI取消接通

/cancel只關閉自己子程序控制管線，沿用共用取消；EOF及互動主程序結束亦會停止。要求停止與保存cancelled分開；停止後本畫面不能再批准。批准等待、模型等待、已有成果後等待三階段乘/cancel、EOF、父程序結束三動作，共9原CLI受控情境過；來源不變、未批准零派工、已寫成果保留、模型連線關閉、狀態及事件獨立讀回。假模型不是C6真Gateway或C9真人。

舊CLI因不識/cancel失敗：p02-cancel-red.txt。初件p02-cancel-green.txt誤以cancelled必為最後事件；核實之後有approval_revoked，修正測試而未改程式符合錯斷言。獨立審查抓到父程序啟動早退造成stderr panic、DB留running；以spawn前已關讀端的UnixStream具名紅綠重現並修正。現在先取消及finish，再回報通知錯誤；錯誤通道也失聯時退出74。固定來源、原件及複審p02-cancel-independent-review.md；gate203過零略過，摘要p02-cancel-validation.json。

新版完整all exec75842、PID1283613、.runtime/checks/1794228022-1283613/執行中，785變異未終局，不改來源。普通debug副本捕捉與變異重疊，未用於Gateway，保留隔離；改用coverage已通過且時間早於變異的固定原CLI，manifest在.runtime/p02-cli-cancel-coverage-build/，profile只寫副本profiles。唯一下一步刷新Gateway入口、免費路由及既有負責者，需協作先讀技能、不另建writer或擴張花費。Qwen提案及裁決qwen-p02-gateway-after-cancel-next*。未提交、推送、部署；完整產品、真Gateway及可靠接續未完成。

## C6路由刷新與既有負責者協作

本輪models正反只證端點認證；免費上游仍指bridge容器自己127.0.0.1:11434且退出7。證據p02-gateway-refresh-after-cancel.json、p02-gateway-auth-negative-control.json；推論0，未改服務或Agent來源。

核Orca runtime、終端及/proc後，經orchestration向現有Gateway執行者送一次question msg_ecb5dee5deec；只證信箱收件，不證已讀、承接或完成。終端關閉後訊息不持久，無回覆不重發；回執p02-gateway-owner-coordination.json。未建worker或接管工作樹。隔離Markdown任務只備妥未執行，p02-gateway-ready-after-cancel/manifest.json。

唯一下一步收同一問題回覆，核無新增費用路由後，固定原CLI實送、逐次批准與成果獨立讀回。all75842仍活躍，不改來源或另啟編譯；Qwen提案及裁決qwen-p02-gateway-owner-next*。未提交部署；C6/C7與整體產品仍未完成。

## C6隔離真Gateway整合路徑研究

正式路由仍拒連，既有Gateway負責者仍在工作且沒有可核實路由回覆。只讀同版153ac60b58b3原碼及固定映像，確認可使用全新PG、開發bootstrap和唯一Ollama profile；隔離Gateway用host network並只聽回環，無外部API或訂閱憑證。現成正式部署器會切Caddy與正式庫，不適用；不另造正式發布流程。尚未啟動容器、推論0。提案與來源hash.runtime/p02-isolated-gateway-preflight.json。

Qwen已實跑並評分2/3；採隔離整合，不採新增代理或把11434正常持有視為衝突。裁決.runtime/qwen-p02-isolated-gateway-next.decision.md。唯一下一步先核自有到期容器、全新資料庫、唯一免費profile、認證正反與回環監聽，再以固定原CLI驗批准、拒絕、取消及成果讀回。這只補隔離真Gateway整合，正式入口、正式身分政策、真人與可靠接續仍未驗；all75842繼續，不改受測來源。

## C6真Gateway回應格式阻塞

隔離同版Gateway原碼153ac60b58b3、原固定CLI、真免費qwen3.8已實送。PG讀回兩次上游皆completed且attempt_number1：1原CLI工作、1不執行工具的格式診斷；不是兩次工作重送。CLI保存outcome_unknown，批准0、tool_intents0、無成果且所有原檔hash不變。回應有合法read_input及reasoning字串；固定原碼明確拒絕該非空欄位。證據.runtime/p02-isolated-gateway-20261109/approved-first-readback.json、diagnostic-response.body、diagnostic-response.json及gateway-ledger-readback.json。

兩自有容器正常退出0，資料與原件保留；正式服務未改。Qwen評2/3，採保存及原樣重送，不採換模型、忽略欄位或Gateway映射；不是OpenAI通用欄位的宣稱。裁決.runtime/qwen-p02-reasoning-next.decision.md。因新實測阻塞需要核心修改，原785項all未完成，不當新版證據；計畫只SIGINT已核mutator1296030，收清理及逐檔hash還原後才改。完整決策.runtime/p02-reasoning-failure-decision.json。唯一下一步推理文字保存／重送、舊紀錄相容、錯格式及跨格式入口拒絕的紅綠，再gate/all及真實送；正式入口、真人與可靠接續仍未過。

## 推理修補快檢與原CLI六十秒中斷

推理欄位保存、重送、舊紀錄相容及不支援內容拒絕已紅綠；獨立審查核出已知Anthropic區塊會丟引用或把錯文字轉空，六案例中五個舊版錯誤接受，修後通過。不能表示的工具參數也在HTTP前拒絕，舊的空物件改寫預期改為保留及拒絕。複審兩次皆無，最終gate210過零略過；摘要.runtime/p02-reasoning-validation.json。舊原CLI讀新schema5實際拒絕，DB/WAL hash不變。

固定schema5成品sha6401acf7d57f0a898322466ccf8b2e568a16892e83e01c20c90c491972ba10e2，212來源build前後相同。真Gateway修補後工作started14:04:20.962、outcome_unknown14:05:20.990；Gateway記caller離線duration60.012。零工具意圖、無成果、原檔不變，尚未收到可驗reasoning的回應。原件.runtime/p02-isolated-gateway-20261109/approved-r2-readback.json。

原文件本來允許每請求60秒，現在依實測調整為既有task_timeout(mode)，仍保留任務120/900秒及取消，不添加新設定。Qwen評1/3，採優先處理但不採其邏輯矛盾說法；裁決.runtime/qwen-p02-model-timeout-next.decision.md。唯一下一步先受管理65秒假回應控舊／新等待與取消，再真Gateway逐次批准、拒絕、取消及成果讀回，通過才跑all；正式入口、真人與可靠接續仍未過。

## 2026-11-09 真工作、拒絕後再批准與取消

舊手寫樣本漏tool_calls.index，真CLI仍失敗。改以完整去識別HTTP樣本核全部鍵、型別與清單長度；只接受與位置相符的冗餘序號。具名紅綠、四項格式測試、獨立微差異審查無發現，gate211過。固定CLI b032ae76首次完成真連結修復；少末尾換行的舊成果被驗證器拒絕。

真拒絕再現狀態缺口：拒絕後仍waiting_approval，模型下個工具申請Conflict而outcome_unknown。僅在原拒絕事件同交易恢復running；不放寬準備入口、不自動批准。既有測試先具名失敗，修後11批准測試全過；獨立三檔差異審查無發現，gate211過零略過。固定同版CLI0179226f、213來源build前後一致。證據.runtime/p02-denial-state-validation.json。

同版原CLI／真Gateway／真免費qwen3.8：單次拒絕後模型正常結束，零工具派發、無成果且來源不變。另一新工作明確最多再要求一次讀檔；第一次拒絕後新call待批，獨立讀回零授權及派發；再批准讀取與寫入，成果精確修連結，/result及SQLite／檔案讀回皆過。不是resume；真拒絕後重新要求也不能繼承舊批准。

模型等待期間Gateway active_requests1後/cancel，再/approve被拒；29秒後本地cancelled、零意圖／成果、來源未變且Gateway active_requests0。此前b032ae76待批取消18秒後零派工亦過。這不證上游運算停止。

全隔離PG讀回21請求／21嘗試：19完成、2取消，唯一免費provider_key及模型、每次attempt_number1；12個任務紀錄包含原失敗，不能當12次成功。最初查帳比對誤用provider種類，已依設定的provider_key核同一原件，未重送或改資料。兩自有容器退出0、port關閉，volume及全部失敗原件保留；正式環境未動。完整證據.runtime/p02-isolated-gateway-20261109/acceptance-summary.json。

下一步只跑一份必要all，固定來源，不重跑已過真案例；Qwen建議取消門檻不採，裁決.runtime/qwen-p02-all-after-real-next.decision.md。正式路由／虛擬身分政策、真人、可靠接續、最小評測入口及完整PLAN仍未完成，未提交推送或正式部署。

新版完整all已啟動：exec81237、PID3016279、.runtime/checks/1794236841-3016279/，日誌.runtime/p02-all-after-real.txt。期間固定來源、不另編譯；唯一下一步收同一終局。啟動回執.runtime/p02-all-after-real-started.json；不得用舊all或啟動回執當完整通過。

15:14UTC唯讀刷新正式C6：同CID／image153ac60b58b3、18105認證models200；bridge容器profile仍指自己127.0.0.1:11434，實際curl退出7。先前原字串尾斜線差異經URL正規化後確認不是路由變更。未送推論、未動正式服務；證據.runtime/p02-formal-route-after-real-readback.json。原Gateway執行者仍活躍，沒有重送同一協作問題或接管；完整all81237維持原份執行。

完整檢查包含8個套件，共801種改壞程式的案例。核心執行72、Anthropic模型連接37、批准38、HTTP傳輸2、OpenAI模型連接45，共194種已完成：141種被測試抓到，53種編譯不過而未執行測試，零漏抓或逾時。五部件原碼已還原；本輪開始213檔原碼的檔案指紋，與固定CLI0179226f建置前記錄一致。證據.runtime/p02-all-harness-scope-readback.json。這只證明上述五部件；CLI、任務庫及檢查工具仍在同一份完整檢查81237中，尚未整體通過。

同一all在成果快照讀回發現1漏抓：store/result_snapshot.rs第60行的schema_version<4改為<=4，全套仍退出0，會略過第4版原本存在的歷史。原碼正確，新資料庫已是5使原標準案例不再觸及4；既有舊庫測試只驗3。原件.runtime/p02-v4-result-candidate/miss.json、mutant.diff、mutant.log，來源快照均核對固定版本。原程式判斷正確，缺少第4版的讀回測試；不加排除或放寬門檻。先收同一all其餘結果，再只補第3／4版只讀歷史的必要反例，期間不套候選或另編譯。

補測候選只擴充原有舊資料庫測試：第3版仍回報沒有歷史，第4版完整讀回相同歷史，查詢都不升級版本。第5版沒有資料表變更；樣本只有第4版原已支援的使用者訊息，不含新增推理文字。Qwen草案1/3，修正連線被提前移走及不存在的查版本方法後，沿用現有查詢。候選格式與可套用檢查都過，獨立差異審查無發現，原測試檔的檔案指紋未變。證據.runtime/p02-v4-result-candidate/candidate-preflight.json；尚未套用、編譯或執行正反例，不當補測完成。

正式入口重新定位：Docker確認Caddy仍運行並使用host網路，但設定讀回請求http://127.0.0.1:2019/config/得到403，主機保存設定的目錄也無讀取權限，尚未取得當前公開入口指向。不能將15:14UTC的18105後端觀察直接當成現在的公開路由。證據.runtime/p02-formal-entry-readback.json；未送模型請求、未改服務，未重送原協作問題。原all81237繼續，接續及正式路由仍未完成。

後續找到可讀主部署檔/home/cash/services/caddy/Caddyfile，指向18105，原件指紋.runtime/p02-formal-readable-config.json。16:56UTC以正常HTTPS、不帶金鑰查公開入口，以及查本地18105健康頁，兩者均200且回報153ac60b58b3；後端同CID／image及bridge網路，免費profile仍是自身127.0.0.1:11434，原容器curl實際退出7。證據.runtime/p02-formal-entry-health.json。這只核版本及可達性，不是正式CLI真工作通過，也不以檔案設定或健康頁代替。模型請求0、正式變更0，未重送原問題或接管。

原all81237已終局退出1，801全跑：684抓到、116不可編譯、1漏抓、零逾時；213來源前後及現場hash全一致，父／變異程序、target子程序與active標記均清。證據.runtime/p02-all-after-real-readback.json。reports失敗由同一漏抓引起；覆蓋原件4035行、3862已覆蓋、173未覆蓋（95.71%），不稱全覆蓋，診斷.runtime/p02-all-after-real-diagnosis.json。

下一步只套既有舊庫3／4必要案例，正常綠、刻意<=具名紅、還原綠及快檢，未完成前不稱修好。隨後優先真正接續，不在只加測試後立即重跑801項、接著又改核心；必要完整all在核心流程穩定後、提交推送部署前執行，原失敗不改綠。Qwen裁決.runtime/qwen-p02-v4-and-resume-next.decision.md；完整產品及正式CLI路由、真人與原PLAN要求保留。

舊庫回歸已套用並完成：正常版過；只把<4改為<=4，新增schema4歷史斷言具名失敗（101，None對Some）；依原指紋還原後同檔5項全過，快檢211過、零失敗略過。來源比原all只差這一個已審查測試，生產碼全相同；證據.runtime/p02-v4-result-validation.json。原all失敗保留，沒有稱新版完整通過，也未重跑已過真工作。

後續先開發原P04 R1必要核心，原安裝後及全部要求保留；必要all在核心穩定後、提交推送部署前仍必須過。P02的正式路由、最小評測入口及真人、整階段驗收仍未完，不標DONE。
