from pathlib import Path
import html
import re
import markdown

root = Path(__file__).resolve().parents[1]
docs = root / 'docs'
evaluation = (docs / 'evaluation.md').read_text()
css = '''
@page{size:A3 landscape;margin:0;background:white}
@page appendix{size:A3 landscape;margin:12mm 0;background:white}
*{box-sizing:border-box}html,body{margin:0;background:#fff;color:#20383c;font-family:"Noto Sans CJK TC",sans-serif;-webkit-print-color-adjust:exact;print-color-adjust:exact}
.page{width:420mm;height:297mm;padding:42px 60px 56px;background:#fff;break-after:page;position:relative}
h1{font-size:30px;color:#12404a;margin:0 0 10px}h2{font-size:21px;margin:24px 0 12px;color:#12404a}h3{font-size:19px;margin:0 0 8px}p{line-height:1.65;margin:8px 0}.sub{font-size:14px;color:#586d71}.status{background:#fff2da;border-left:5px solid #b77d25;padding:12px 18px;font-size:16px;margin:18px 0}
.box{border:2px solid #6b8a8f;border-radius:12px;background:#ebf4f2;padding:16px;text-align:center;min-height:100px;display:flex;flex-direction:column;justify-content:center}.box b{font-size:21px}.box span{font-size:16px;line-height:1.6;margin-top:7px}.blue{background:#e6f0fb}.amber{background:#fff2da}.purple{background:#efe7fb}.group{border:2px dashed #6b8a8f;border-radius:14px;padding:16px}.group .box+.box{margin-top:12px}.label{font-size:16px;font-weight:700;text-align:center;margin-bottom:14px;color:#46676e}.flow{display:grid;grid-template-columns:1fr 100px 1fr 100px 1fr;align-items:center;margin:20px 0 28px}.flow.four{grid-template-columns:1fr 64px 1fr 64px 1fr 64px 1fr}.arrow{height:2px;background:#1f7a70;position:relative;margin:0 4px}.arrow:after{content:"";position:absolute;right:-1px;top:-7px;border:8px solid transparent;border-right:0;border-left:14px solid #1f7a70}.arrow.both:before{content:"";position:absolute;left:-1px;top:-7px;border:8px solid transparent;border-left:0;border-right:14px solid #1f7a70}.arrow i{position:absolute;left:0;right:0;top:-26px;text-align:center;font-size:13px;font-style:normal;white-space:nowrap}.columns{display:grid;grid-template-columns:repeat(3,1fr);gap:24px}.columns.four{grid-template-columns:repeat(4,1fr)}.v{width:2px;height:54px;margin:auto;background:#1f7a70;position:relative}.v:after{content:"";position:absolute;bottom:0;left:-7px;border:8px solid transparent;border-bottom:0;border-top:14px solid #1f7a70}.note{font-size:16px;line-height:1.7;margin:20px 0}.foot{position:absolute;left:60px;right:60px;bottom:20px;font-size:13px;color:#61777c;border-top:1px solid #d9e2e3;padding-top:8px}.cards{display:grid;grid-template-columns:1fr 1fr;gap:18px}.card{border:2px solid #6b8a8f;background:#f3f7f7;border-radius:12px;padding:15px 19px;font-size:15px;line-height:1.6}.card p{margin:5px 0}.card .state{font-size:13px;color:#806022}.card b{color:#12404a}.card h3{font-size:20px}.contract{font-size:16px;line-height:1.65}.contract td,.contract th{padding:14px}table{width:100%;border-collapse:collapse;table-layout:fixed;font-size:16px;line-height:1.65}td,th{border:1px solid #b9cacc;padding:9px 12px;vertical-align:top;overflow-wrap:anywhere}th{background:#e7f0ef;text-align:left}tr{break-inside:avoid}a{color:#126c77}nav{position:sticky;top:0;background:#143f47;color:white;padding:12px 30px;z-index:9;display:flex;gap:20px}nav a{color:white}code{font-size:.9em;overflow-wrap:anywhere}.appendix{background:#fff;width:420mm;padding:38px 60px;font-size:16px;line-height:1.65;break-before:page}.appendix h1,.appendix h2,.appendix h3{break-after:avoid}.appendix h2{margin-top:24px}.appendix table{font-size:14px}.appendix img{max-width:100%}.appendix pre{white-space:pre-wrap;overflow-wrap:anywhere}.links{columns:2;font-size:18px;line-height:2.1}.metrics td{padding:12px}.tag{color:#806022;font-weight:700}button{padding:5px 14px;background:white;border:0;border-radius:5px;color:#143f47}
#deploy .box{min-height:80px;padding:12px}#faults th:first-child{width:5%}#faults th:last-child{width:12%}#faults table{font-size:15px}#faults td,#faults th{padding:8px}
@media screen{body{background:#dce5e6}.page,.appendix{margin:24px auto;box-shadow:0 3px 20px #183e4920}.appendix{margin-bottom:24px}section{scroll-margin-top:70px}}
@media print{.appendix{page:appendix;padding:0 60px;width:auto}nav{display:none}.page,.appendix{box-shadow:none;margin:0}a{color:inherit;text-decoration:none}}
'''

def box(title, body='', cls=''):
    return f'<div class="box {cls}"><b>{title}</b><span>{body}</span></div>'

def arrow(label='', both=False):
    return f'<div class="arrow {"both" if both else ""}"><i>{label}</i></div>'

def flow(items, four=False):
    return f'<div class="flow {"four" if four else ""}">'+''.join(items)+'</div>'

def table(headers, rows, cls=''):
    return f'<table class="{cls}"><thead><tr>'+''.join(f'<th>{h}</th>' for h in headers)+'</tr></thead><tbody>'+''.join('<tr>'+''.join(f'<td>{c}</td>' for c in row)+'</tr>' for row in rows)+'</tbody></table>'

pages=[]

def page(ident,title,body):
    number=len(pages)+1
    pages.append(f'<section class="page" id="{ident}"><h1>{title}</h1><p class="sub">Agent Platform｜現行計畫圖文版｜三平台均為必要交付；所有產品驗收待執行。</p>{body}<div class="foot">{number}｜實線箭頭：標示的步驟、控制或資料傳遞；雙箭頭：請求／回應；虛線框：部署或責任群組。本文不是測試通過報告。</div></section>')

page('start','產品全貌｜企業 Agent，先 CLI，再 Desktop',
'<div class="status">同一份現行計畫：本 PDF／HTML 是 PLAN.md、架構契約與驗收規格的閱讀成品。沒有另立產品版本。</div>'+flow([box('正式 CLI','Windows／macOS／Linux<br>任務、知識、工具、記憶、電腦操作','blue'),arrow('交付順序'),box('正式 Desktop','三平台 Tauri 工作台<br>看進度、核准、差異與成果','blue'),arrow('發布驗收'),box('企業交付','安全、可靠恢復、安裝升級<br>容量、備份、稽核、真人驗收','amber')])+ '<h2>從這裡看，不必逐份翻 Markdown</h2><div class="links"><a href="#deploy">02　系統部署與責任邊界</a><br><a href="#lifecycle">03　任務、授權、取消與接續</a><br><a href="#data">04　資料歸屬與一致性</a><br><a href="#rag">05　向量化 RAG 的資料流</a><br><a href="#knowledge">06　主動查證與知識維護</a><br><a href="#platform">07　三平台的安裝與轉接</a><br><a href="#replace">08　替換、更新與恢復</a><br><a href="#accept">09　程式、AI、真人如何驗收</a><br><a href="#coverage">10　部件覆蓋與發布門檻</a><br><a href="#contracts-1">11 起　逐部件契約</a><br><a href="#cases-1">驗收卡　A01–A36 操作與失敗對照</a><br><a href="#canonical">附錄　唯一主計畫全文</a></div><h2>交付狀態必須分開</h2>'+table(['已有文件','尚待產品證據','不能拿來代替'],[['架構、部件契約、操作卡與量化目標','三平台實機、功能、效能、故障、安全及真人簽收','模型說成功、服務健康、局部編譯、這份 PDF']])+'<p class="note">每張驗收卡列前置、操作、通過條件、失敗對照、證據與 AI／真人責任。共用案例按每個必交 OS／CPU 與適用入口重跑。</p>')

page('deploy','部署架構｜主機控制、容器知識、外部模型分開',
flow(['<div class="group"><div class="label">使用者電腦：原生前景程序</div>'+box('CLI／Tauri 桌面','任務、授權、停止、接續')+box('共用核心 Harness','上下文、工具意圖、生命週期','blue')+box('平台政策與獨立驗證','核權、限額、讀回成果','amber')+'</div>',arrow('模型請求',True),'<div class="group"><div class="label">獨立模型服務</div>'+box('Gateway','認證、選路、用量；不管任務授權','purple')+box('獲准模型供應者','本機或遠端模型；外送受政策限制','purple')+'</div>','<div></div>','<div class="group"><div class="label">部署原則</div>'+box('只有一個主執行迴圈','模型不能自批工具或自判完成')+box('常駐應用工作在容器','前景 GUI 驅動留在原生主機')+'</div>'])+'<h2>共用核心透過受限工具存取兩類能力</h2>'+flow([box('共用核心 Harness','提出工具意圖；接收執行回執','blue'),arrow('意圖／回執',True),box('同一工具閘門','身分、參數、範圍、取消、預算','amber'),arrow('核權後派發',True),'<div class="group"><div class="label">兩類受限工具，皆經同一閘門</div>'+box('原生工具與 GUI 驅動','主機檔案／Shell／MCP／畫面操作')+box('容器知識處理服務','發現排程、Python 解析、向量、維護')+box('PostgreSQL／pgvector','有效文件、索引、權限、知識與答案')+'</div>'])+'<p class="note">Gateway 與知識服務沒有直接互相控制的呼叫線。知識服務讀獲准來源；主機先驗來源權限，容器不藉 root 擴權。主機工具與背景容器不能自由跨出任務範圍。對照：A06–A08、A13、A30–A31。</p>')

page('lifecycle','任務生命週期｜先核權，真執行，再獨立驗證',
flow([box('任務規格','原指令、範圍、預算、完成條件'),arrow('必要上下文'),box('AI 提議下一步','規劃／工具／查證','blue'),arrow('待核對'),box('程式決策閘門','參數、授權、政策版本、取消','amber')])+flow([box('已允許的意圖','先持久保存；不可寫就不執行'),arrow('派發'),box('工具執行','有副作用的動作留下回執'),arrow('讀回'),box('獨立驗證','檔案、DB、畫面與原文','blue')])+'<h2>獨立驗證後依結果分流</h2><div class="columns">'+box('通過','證據足夠才完成；有主觀項目仍待真人')+box('未通過','把失敗證據交回規劃；受同一總預算限制','amber')+box('結果未明','先查已發生的副作用；不能盲重送','amber')+'</div><h2>取消與接續另走控制列，不能靠模型自行配合</h2>'+flow([box('取消請求／重啟','保存取消或讀回最後事件'),arrow('先阻擋'),box('停止派發／核對副作用','關閉程序樹；查外部狀態'),arrow('有證據'),box('已停止／可接續／待處理','未確認不能顯示成功')])+flow([box('獨立驗證未通過','保留失敗證據'),arrow('有預算再規劃'),box('AI 提議下一步','回到同一核心；不新增執行者'),arrow('再次核權'),box('程式決策閘門','原授權、取消及限制仍有效')])+'<p class="note">循環以「驗證失敗後重新規劃」表達，不另建第二核心。跨入口只共享一個任務執行者。對照：A05–A12、A28、A30。</p>')

cols=[]
for title,body,store,details in [('平台控制','核准、取消、任務映射、驗收','SQLite 本機資料','短交易、單寫入者；不跨機共享檔'),('核心執行','核心自己的進度與事件','選定 Harness 的紀錄','Codex 沿用其紀錄；Rig 由平台擁有流程'),('來源與知識','文件／切片／向量／答案依賴','PostgreSQL＋受限產物','有效版本、來源權限、失效與查證時間'),('原始資料與記憶','原檔不被建庫改寫；記憶另經同意','原檔／記憶控制及內容映射','原檔與索引分開；記憶同意在 SQLite')]:
 cols.append('<div>'+box(title,body,'blue')+'<div class="v"></div>'+box(store,details)+'</div>')
page('data','資料架構｜誰擁有真相，誰只能讀映射','<div class="columns four">'+''.join(cols)+'</div><h2>沒有跨兩個資料庫的假交易保證</h2>'+flow([box('來源更新／撤權','產生持久事件及新權限世代'),arrow('先失效'),box('文件及衍生資料','摘要、虛擬問題、答案與快取'),arrow('再次核對'),box('每次內容返回前','重驗來源版本與當前授權','amber')])+table(['資料規則','正常路徑','遇到失敗'],[['意圖／回執','意圖先落盤；執行後記真回執','回執缺失先查副作用；不能重播工具'],['發布新索引','解析、切片、向量完成才切有效版本','部分新資料不可見；過期舊版預設不當答案'],['備份與還原','記每庫版本／依賴；還原後重新核權','不把撤權資料復活；不相容資料版本拒絕']])+'<p class="note">憑證放 OS 安全存放處；一般日誌不含完整內容或秘密。對照：A06、A10、A12、A18、A22–A25、A31。</p>')

page('rag','向量化 RAG｜先保留原文，再增加查找能力',
flow([box('授權來源','文件／郵件／表格／程式／設計檔'),arrow('分批'),box('發現與排程','路徑、格式、大小、游標；前景優先'),arrow('隔離解析'),box('可定位的結構','頁碼、儲存格、附件、函式行號','blue')])+flow([box('三種切片','結構／語意向量／AI 選切點','blue'),arrow('程式驗邊界'),box('原文片段＋衍生欄位','虛擬提問、背景、父段關係'),arrow('固定模型'),box('全文與向量索引','正規化、維度、版本、去重')])+flow([box('使用者問題','保留型號、否定詞與時間'),arrow('權限過濾'),box('混合候選與重排','關鍵字＋向量；命中問題映射原文'),arrow('補上下文'),box('帶來源的答案','返回前重驗版本及授權','amber')])+ '<div class="status">AI 可以選切點、生成問題與排序；程式必須驗原文覆蓋、合法切點、支援位置及權限。生成問題是搜尋入口，不是答案證據。</div><p class="note">大檔獨立排程，不先把全碟交語言模型才開始工作。結構基準與所有承諾增強方法需有對照；未達品質或成本門檻保留基準，不把未測算成最佳。對照：R01–R08、R12–R15；A13–A20、A24。</p>')

page('knowledge','主動查證與知識庫｜一個執行核心，持續維護依據',
'<h2>Agentic RAG：按證據缺口決定是否再查</h2>'+flow([box('同一 Harness','理解原問題、拆子問題','blue'),arrow('受限查詢'),box('工具與來源','查索引／補讀原檔／交叉核對'),arrow('逐結論核證'),box('回答或停止','足夠才答；衝突、無答案明說','amber')])+'<p class="note">最多 3 次檢索、2 輪重規劃且服從更嚴總預算；取消停止新派發。未足夠時把缺口交回同一核心，不另外起主迴圈。</p><h2>知識發布：AI 提案，規則或真人核對</h2>'+flow([box('候選知識與解答','來源版本、支持段落、適用範圍'),arrow('獨立核對'),box('有效知識','具名確認或可驗規則；記查證時間'),arrow('事件／到期'),box('待查證／衝突／撤回','依據變更先失效，再核對重發布','amber')])+'<h2>維護的對象是依賴，不是每次重算全庫</h2><div class="columns">'+box('來源管理','事件加定期補查；失聯不當刪除')+box('依賴管理','來源 → 切片／問題／摘要 → 答案')+box('使用前再驗','權限或版本不可核對就不回傳')+'</div><p class="note">資料最新與最後成功查證分開顯示；快變外部問題使用時重查。高頻維護受資源與來源限流約束，顯示積壓，不假稱瞬時永遠最新。對照：R09–R13；A21–A24、A31。</p>')

platformrows=[('Windows 11 x64','原生 .exe／.msi 或 NSIS','ACL／原生監看／秘密保存<br>視窗、輸入、程序取消','Podman machine＋WSL2<br>VM 與掛載另量測'),('macOS arm64／x64','原生 CLI／.app＋.dmg<br>簽章、公證','Keychain／檔案與畫面授權<br>兩種 CPU 各驗','Podman machine Linux VM<br>共享路徑不能代主機驗權'),('Linux x64','原生 CLI／.deb／AppImage','UID／GID／隔離工具<br>X11、Wayland 分能力驗','Docker Engine rootless<br>IT 準備系統前置')]
b=''
for name,install,adapter,runtime in platformrows:
 b+=f'<h2>{name}</h2>'+flow([box('正式安裝包',install),arrow('啟動'),box('原生平台轉接',adapter,'blue'),arrow('受限服務'),box('容器與資料',runtime)])
b+='<p class="note">每列都執行相同核心、RAG、安全與企業驗收，再跑平台專項。Windows 不靠使用者進 WSL 操作 Agent；Mac 不要求停用 Gatekeeper；Linux X11 不代表 Wayland 已驗。對照：A01–A04、A07、A13、A26–A28、A33–A36。</p>'
page('platform','三平台｜共用能力，差異在明確的轉接邊界',b)

page('replace','系統彈性｜更換部件前後都驗整條流程',
flow([box('既有介面契約','版本、能力、權限、取消、錯誤'),arrow('候選替身'),box('新模型／工具／解析器／驅動','先通同題正反契約','blue'),arrow('下游讀回'),box('整合＋故障測試','成果、資料與原入口行為不退步','amber')])+ '<h2>不能靠「有一個 interface」宣稱可替換</h2>'+table(['替換類型','必須保留','不相容時'],[['模型／執行核心','任務、授權與副作用契約；核心進度擁有者唯一','舊任務完成／停止或明確遷移，不能直接塞進新核心'],['解析／切片／向量','原文定位、版本、權限；不同向量空間分開','新索引完成與比較後才切換，保留回退路徑'],['工具／GUI／平台','能力查詢、逾時、取消、結果未知及隔離','缺必要能力拒絕，不能偷偷裸執行'],['儲存／安裝管道','資料版本、有效性、備份與簽章','先驗備份和遷移；還原後重新核權']])+ '<h2>更新與恢復</h2>'+flow([box('版本／依賴與備份','材料清單、原始資料不刪'),arrow('驗簽遷移'),box('新版本受測啟動','活躍任務不得中途換核心'),arrow('讀回證據'),box('接受／還原／待處理','不相容即停；不能只看健康頁')])+'<p class="note">轉接、隔離與共用不等於所有模型／平台組合都支援；正式支援矩陣逐格有證據。對照：A02、A12、A29–A30、A34。</p>')

page('accept','完整驗收｜程式證明行為，AI 找盲點，真人簽收',
flow([box('固定需求與資料','版本、範圍、完成條件、真人負責人'),arrow('執行'),box('程式及獨立驗證','原入口操作；真檔案／DB／畫面讀回','blue'),arrow('帶證據'),box('AI 反方審查','檢查差異、反例、原文與未覆蓋項')])+flow([box('真人驗收','操作者／IT／領域人員各自操作'),arrow('接受／退回'),box('同版證據包','結果、雜湊、版本、未測與限制'),arrow('核對門檻'),box('具名發布決定','CLI／Desktop 各自三平台通過','amber')])+table(['所有案例共同欄位','填寫要求'],[['前置與操作者','OS／CPU、程式、模型、資料、政策與依賴版本；具名執行者、審查者及真人驗收者'],['判定','實際值＋預期門檻；PASS／FAIL／NOT_RUN／UNKNOWN；不以文字宣稱代替行為'],['證據','可重跑步驟、原始退出碼／事件、產物雜湊、畫面或原文、失敗對照、覆蓋與限制'],['故障範圍','只用合成資料與隔離環境；正式環境、真秘密與付款不作故障注入']])+ '<p class="note">當前所有 A01–A36 均待執行。規格完整不等於產品通過；審查模型不能自己改門檻，真人滿意也不能蓋過安全失敗。</p>')

mapping=[('入口、安裝、平台、更新','A01–A04、A28、A34–A36'),('核心、模型、授權、工具、隔離','A05–A08、A29–A31'),('取消、接續、上下文、儲存','A09–A12、A33'),('全機發現、解析、切片、向量與搜尋','A13–A20、A24'),('主動查證、知識發布與高頻維護','A21–A24、A31'),('記憶、電腦操作與真人接手','A25–A28'),('彈性、全流程、安全、稽核及維運','A29–A36')]
page('coverage','覆蓋與門檻｜不能用平均分數掩蓋安全失敗',table(['部件／需求群','對應驗收卡'],mapping)+'<h2>量化門檻與發布範圍</h2>'+table(['領域','必要目標'],[['任務／長任務','檔案20題×3輪，每輪≥18/20；長任務3個×30分鐘，每個2次壓縮及2次重啟，禁止條件全保留'],['知識品質','調整60＋保留60；Recall@5≥27/30、@10≥28/30、nDCG@10≥0.80；定位≥29/30；正確有支持≥27/30；無答案／矛盾／隔離各10/10'],['效能與安全','2並行、冷暖各1000次，查詢＋重排P95≤2秒/P99≤5秒；前景增幅≤20%；安全違規與假成功0；各平台24小時混合負載'],['CLI／Desktop','兩次都是企業正式交付；CLI不要求Tauri畫面，Desktop增加真桌面及跨入口；每次都要三平台／必交CPU完整證據']],'metrics')+'<p class="note">以上是本案待達成目標，不是論文保證或實測結果。更多資源／方法收斂條件沿用工作拆解。A01–A36 覆蓋產品流程，F01–F15 對應可控故障；仍須把每個格式、平台與能力矩陣展開成實際執行紀錄。</p>')

architecture=(docs/'architecture.md').read_text()
section=architecture.split('## 部件責任與替換驗收')[1].split('## 整體完整性檢查')[0]
rows=[]
for line in section.splitlines():
    if line.startswith('|') and not re.match(r'\|\s*[-:]',line):
        rows.append([c.strip() for c in line.strip('|').split('|')])
headers=rows.pop(0)
for start in range(0,len(rows),7):
    body=table(headers,rows[start:start+7],'contract')+'<p class="note">每個部件另須有：輸入輸出版本、權限、資料擁有者、錯誤與取消、重試／副作用、資源、觀測、遷移／回退、上下游整合。相關驗收卡見前頁需求群對照。</p>'
    page(f'contracts-{start//7+1}',f'部件契約 {start//7+1}｜責任、替換與故障驗收',body)

cases=[]
for match in re.finditer(r'^### (A\d+)｜([^\n]+)\n(.*?)(?=^### A\d+｜|\Z)',evaluation,re.M|re.S):
    ident,title,body=match.groups()
    fields=dict(re.findall(r'^- ([^：]+)：(.+)$',body,re.M))
    cases.append((ident,title,fields))
assert len(cases)==36
for start in range(0,len(cases),4):
    cards=[]
    for ident,title,fields in cases[start:start+4]:
        body=f'<h3 id="{ident}">{ident}｜{html.escape(title)}</h3><div class="state">{html.escape(fields["階段"])}｜{html.escape(fields["狀態"])}</div>'
        for key in ['前置','操作','通過','失敗對照','證據','AI／真人']:
            body+=f'<p><b>{key}：</b>{html.escape(fields[key])}</p>'
        cards.append('<article class="card">'+body+'</article>')
    page(f'cases-{start//4+1}',f'驗收操作卡｜{cases[start][0]}–{cases[min(start+3,len(cases)-1)][0]}','<div class="cards">'+''.join(cards)+'</div><p class="note">共用案例每個必交 OS／CPU 與適用入口分別執行。具名執行者／真人驗收者在實跑前填入；不得以角色名稱或 AI 模擬簽收。安全與假成功反例未過就不能放行。</p>')

faults=evaluation.split('## 故障清單與可觀測的結果')[1].split('## 報表與回歸')[0]
page('faults','故障注入覆蓋｜正常、故障、失敗對照都要有',markdown.markdown(faults,extensions=['tables']))
canonical=(root/'PLAN.md').read_text()
def absolute_links(match):
    label,target=match.groups()
    if '://' in target or target.startswith('#'):
        return match.group(0)
    return f'[{label}]({(root/target.split("#")[0]).resolve()}'+('#'+target.split('#',1)[1] if '#' in target else '')+')'
canonical=re.sub(r'\[([^\]]+)\]\(([^)]+)\)',absolute_links,canonical)
appendix='<section class="appendix" id="canonical"><h1>附錄｜唯一主計畫全文</h1><p>以下由根目錄 PLAN.md 同次產生，與前面圖形及驗收卡共同閱讀。</p>'+markdown.markdown(canonical,extensions=['tables','fenced_code'])+'</section>'
nav='<nav><b>Agent Platform</b><a href="#start">總覽</a><a href="#deploy">架構</a><a href="#rag">RAG</a><a href="#platform">三平台</a><a href="#accept">驗收</a><a href="#cases-1">操作卡</a><a href="#canonical">主計畫全文</a><button onclick="window.print()">列印</button></nav>'
(docs/'PLAN.html').write_text('<!doctype html><html lang="zh-Hant"><head><meta charset="utf-8"><title>Agent Platform｜架構與完整驗收</title><style>'+css+'</style></head><body>'+nav+''.join(pages)+appendix+'</body></html>')
print(f'HTML generated: {len(pages)} designed pages, {len(cases)} acceptance cards, canonical appendix')
