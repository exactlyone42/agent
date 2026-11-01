import json, os, pathlib, re, subprocess, sys, tempfile, threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
home = pathlib.Path(tempfile.mkdtemp(prefix='h6-cli-'))
requests = []
tool = sys.argv[1]
class Handler(BaseHTTPRequestHandler):
 def log_message(self, *args): pass
 def do_POST(self):
  body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
  requests.append(body)
  if len(requests) == 1:
   item = {'type':'function_call','id':'fc_1','name':'exec_command','arguments':json.dumps({'cmd':'cat','yield_time_ms':1000,'tty':True}),'call_id':'call_1'}
  elif len(requests) == 2:
   outputs = [x for x in body.get('input',[]) if x.get('type') == 'function_call_output']
   match = re.search(r'Process running with session ID (\d+)', outputs[-1]['output'])
   if match is None:
    print('NO SESSION', outputs, flush=True)
    item = {'type':'message','id':'msg_1','role':'assistant','content':[{'type':'output_text','text':'no session','annotations':[]}]}
   else:
    item = {'type':'function_call','id':'fc_2','name':'write_stdin','arguments':json.dumps({'session_id':int(match.group(1)),'chars':'h6-input\n\u0004','yield_time_ms':1000}),'call_id':'call_2'}
  else:
   item = {'type':'message','id':'msg_1','role':'assistant','content':[{'type':'output_text','text':'fixture complete','annotations':[]}]}
  response = {'id':f'resp_{len(requests)}','object':'response','status':'completed','output':[item],'usage':{'input_tokens':1,'output_tokens':1,'total_tokens':2}}
  events = [{'type':'response.created','response':{**response,'status':'in_progress','output':[]}}, {'type':'response.output_item.added','output_index':0,'item':item},{'type':'response.output_item.done','output_index':0,'item':item},{'type':'response.completed','response':response}]
  payload = ''.join('event: '+e['type']+'\ndata: '+json.dumps(e)+'\n\n' for e in events).encode()
  self.send_response(200); self.send_header('Content-Type','text/event-stream'); self.send_header('Content-Length',str(len(payload))); self.end_headers(); self.wfile.write(payload)
server = ThreadingHTTPServer(('127.0.0.1',0),Handler)
worker = threading.Thread(target=server.serve_forever); worker.start()
(home/'config.toml').write_text(f'''model = "fixture-model"\nmodel_provider = "fixture"\napproval_policy = "never"\n[features]\nhooks = true\n[model_providers.fixture]\nname = "offline fixture"\nbase_url = "http://127.0.0.1:{server.server_port}/v1"\nwire_api = "responses"\nrequires_openai_auth = false\nrequest_max_retries = 0\nstream_max_retries = 0\n''')
hook = home/'deny.py'
hook.write_text('import json, pathlib, sys\ndata=json.load(sys.stdin)\nwith pathlib.Path(sys.argv[1]).open("a") as f: f.write(json.dumps(data)+"\\n")\nif data.get("tool_name") == "write_stdin":\n print("h6 denied", file=sys.stderr)\n sys.exit(2)\n')
(home/'hooks.json').write_text(json.dumps({'hooks':{'PreToolUse':[{'hooks':[{'type':'command','command':f'/usr/bin/python3 {hook} {home}/hook-input','timeout':5}]}]}}))
env={'PATH':'/usr/bin:/bin','HOME':str(home),'CODEX_HOME':str(home)}
try:
 p=subprocess.run(['/home/cash/.local/bin/codex','exec','--ephemeral','--skip-git-repo-check','--dangerously-bypass-hook-trust','--json','test'],cwd=home,env=env,capture_output=True,text=True,timeout=30)
finally:
 server.shutdown();server.server_close();worker.join()
(home/'requests.json').write_text(json.dumps(requests,indent=2))
(home/'stdout.jsonl').write_text(p.stdout)
(home/'stderr.txt').write_text(p.stderr)
print(json.dumps({'home':str(home),'tool':tool,'exit':p.returncode,'hook':(home/'hook-input').exists(),'requests':len(requests),'outputs':[x for req in requests for x in req.get('input',[]) if x.get('type')=='function_call_output']},indent=2))

outputs = [x for req in requests for x in req.get('input', []) if x.get('type') == 'function_call_output']
hook_inputs = [json.loads(line) for line in (home/'hook-input').read_text().splitlines()] if (home/'hook-input').exists() else []
passed = p.returncode == 0 and len(requests) == 3 and [x.get('tool_name') for x in hook_inputs] == ['Bash'] and any(x.get('call_id') == 'call_2' and 'h6-input' in x.get('output', '') for x in outputs)
raise SystemExit(0 if passed else 1)
