import datetime
import hashlib
import json
import os
from pathlib import Path
import selectors
import subprocess
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


out = Path(__file__).resolve().parent
records = []
requests = []
checks = {}
started = datetime.datetime.now(datetime.timezone.utc).isoformat()
binary = Path('/home/cash/.local/bin/codex').resolve()


class Handler(BaseHTTPRequestHandler):
    def log_message(self, format, *args):
        records.append({'http': format % args})

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers['Content-Length'])))
        requests.append(body)
        final = len(requests) > 1
        item = ({'type': 'message', 'id': 'msg_1', 'role': 'assistant',
                 'content': [{'type': 'output_text', 'text': 'fixture complete', 'annotations': []}]}
                if final else {'type': 'function_call', 'id': 'fc_1',
                               'name': 'exec_command', 'arguments': json.dumps({'cmd': 'printf changed > fixture.txt', 'login': False, 'sandbox_permissions': 'require_escalated', 'justification': 'Write synthetic test fixture only'}), 'call_id': 'call_1'})
        response = {'id': 'resp_' + str(len(requests)), 'object': 'response',
                    'status': 'completed', 'output': [item],
                    'usage': {'input_tokens': 1, 'output_tokens': 1, 'total_tokens': 2}}
        events = [{'type': 'response.created', 'response': {**response, 'status': 'in_progress', 'output': []}},
                  {'type': 'response.output_item.added', 'output_index': 0, 'item': item},
                  {'type': 'response.output_item.done', 'output_index': 0, 'item': item},
                  {'type': 'response.completed', 'response': response}]
        payload = ''.join('event: ' + event['type'] + '\ndata: ' + json.dumps(event) + '\n\n'
                          for event in events).encode()
        self.send_response(200)
        self.send_header('Content-Type', 'text/event-stream')
        self.send_header('Content-Length', str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)


with tempfile.TemporaryDirectory(prefix='agent-platform-execution-') as home:
    server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
    worker = threading.Thread(target=server.serve_forever)
    worker.start()
    config = '\n'.join(['model = "fixture-model"', 'model_provider = "fixture"',
                        '[model_providers.fixture]', 'name = "offline fixture"',
                        f'base_url = "http://127.0.0.1:{server.server_port}/v1"',
                        'wire_api = "responses"', 'requires_openai_auth = false',
                        'request_max_retries = 0', 'stream_max_retries = 0'])
    Path(home, 'config.toml').write_text(config)
    fixture = Path(home, 'fixture.txt')
    fixture.write_text('fixed-value')
    env = {'PATH': '/usr/bin:/bin', 'HOME': home, 'CODEX_HOME': home}
    with (out / 'stderr.txt').open('w') as errors:
        proc = subprocess.Popen([str(binary), 'app-server'], cwd=home, env=env,
                                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=errors)
        selector = selectors.DefaultSelector()
        selector.register(proc.stdout, selectors.EVENT_READ)
        buffer = b''

        def send(message):
            records.append({'sent': message})
            proc.stdin.write(json.dumps(message).encode() + b'\n')
            proc.stdin.flush()

        def receive(predicate):
            global buffer
            deadline = time.monotonic() + 25
            while time.monotonic() < deadline:
                if b'\n' not in buffer:
                    if not selector.select(max(0, deadline - time.monotonic())):
                        break
                    chunk = os.read(proc.stdout.fileno(), 65536)
                    if not chunk:
                        raise RuntimeError('app-server closed output')
                    buffer += chunk
                while b'\n' in buffer:
                    line, buffer = buffer.split(b'\n', 1)
                    response = json.loads(line)
                    records.append({'received': response})
                    if predicate(response):
                        return response
            raise TimeoutError('expected protocol event not received')

        def request(number, method, params):
            send({'id': number, 'method': method, 'params': params})
            result = receive(lambda message: message.get('id') == number)
            if 'error' in result:
                raise RuntimeError(result['error'])
            return result['result']

        try:
            request(1, 'initialize', {'clientInfo': {'name': 'denial_probe', 'version': '0.1.0'},
                                      'capabilities': {'experimentalApi': True}})
            send({'method': 'initialized', 'params': {}})
            thread = request(2, 'thread/start', {
                'cwd': home, 'model': 'fixture-model', 'modelProvider': 'fixture',
                'approvalPolicy': 'untrusted', 'sandbox': 'read-only', 'ephemeral': True,
                })
            request(3, 'turn/start', {'threadId': thread['thread']['id'],
                                     'input': [{'type': 'text', 'text': 'Run the synthetic fixture command once.'}]})
            call = receive(lambda message: message.get('method') == 'item/commandExecution/requestApproval')
            checks['approval_received'] = call['params']['commandActions'] == [{'type': 'unknown', 'command': 'printf changed > fixture.txt'}]
            checks['fixture_unchanged_before_decision'] = fixture.read_text() == 'fixed-value'
            send({'id': call['id'], 'result': {'decision': 'decline'}})
            completed = receive(lambda message: message.get('method') == 'turn/completed')
            checks['turn_completed'] = completed['params']['turn']['status'] == 'completed'
            checks['two_model_requests'] = len(requests) == 2
            outputs = [item for item in requests[-1].get('input', []) if item.get('type') == 'function_call_output']
            checks['denial_returned_to_model'] = any('reject' in json.dumps(item.get('output')).lower() for item in outputs)
            checks['fixture_unchanged'] = fixture.read_text() == 'fixed-value'
            (out / 'fixture-after.txt').write_text(fixture.read_text())
            proc.stdin.close()
            checks['clean_exit'] = proc.wait(timeout=10) == 0
        finally:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=5)
            selector.close()
            server.shutdown()
            server.server_close()
            worker.join(timeout=5)
            (out / 'transcript.json').write_text(json.dumps(records, indent=2))
            (out / 'requests.json').write_text(json.dumps(requests, indent=2))
            (out / 'manifest.json').write_text(json.dumps({
                'started_at': started, 'ended_at': datetime.datetime.now(datetime.timezone.utc).isoformat(),
                'commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
                'version': subprocess.check_output([str(binary), '--version'], text=True).strip(),
                'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                'checks': checks, 'exit_code': proc.returncode,
                'model': 'fixed local SSE, no inference', 'fixture': 'fixed-value-v1',
                'authorization': 'temporary files and loopback fixture only',
                'not_run': ['Rig comparison', 'cancel/resume', 'Rust/TS gates', 'desktop', 'deployment']
            }, indent=2))
print(json.dumps(checks, indent=2))
raise SystemExit(0 if len(checks) == 7 and all(checks.values()) else 1)
