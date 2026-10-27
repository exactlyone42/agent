import argparse
import hashlib
import json
import os
from pathlib import Path
import selectors
import shutil
import subprocess
import tempfile
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


def run_case(binary, output, case):
    output.mkdir()
    records, requests = [], []
    reads = writes = 0
    checks = {}

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def do_POST(self):
            length = int(self.headers['Content-Length'])
            if length > 4 * 1024 * 1024 or len(requests) >= 3:
                self.send_error(413)
                return
            requests.append(json.loads(self.rfile.read(length)))
            index = len(requests)
            if index == 1:
                name, args = 'read_fixture', {'path': 'README.md'}
            elif index == 2 and case != 'missing':
                name, args = 'write_fixture', {'content': 'incorrect content' if case == 'wrong' else 'ORIGINAL FIXTURE'}
            else:
                name, args = None, None
            item = ({'type': 'function_call', 'id': f'fc_{index}', 'name': name,
                     'arguments': json.dumps(args), 'call_id': f'call_{index}'} if name else
                    {'type': 'message', 'id': f'msg_{index}', 'role': 'assistant',
                     'content': [{'type': 'output_text', 'text': 'done', 'annotations': []}]})
            response = {'id': f'resp_{index}', 'object': 'response', 'status': 'completed',
                        'output': [item], 'usage': {'input_tokens': 1, 'output_tokens': 1, 'total_tokens': 2}}
            events = [
                {'type': 'response.created', 'response': {**response, 'status': 'in_progress', 'output': []}},
                {'type': 'response.output_item.added', 'output_index': 0, 'item': item},
                {'type': 'response.output_item.done', 'output_index': 0, 'item': item},
                {'type': 'response.completed', 'response': response},
            ]
            payload = ''.join(f'event: {event["type"]}\ndata: {json.dumps(event)}\n\n' for event in events).encode()
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.send_header('Content-Length', str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

    with tempfile.TemporaryDirectory(prefix='agent-file-contract-') as directory:
        workspace = Path(directory)
        source, artifact = workspace / 'README.md', workspace / 'result.txt'
        source.write_text('original fixture')
        server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        worker = threading.Thread(target=server.serve_forever)
        worker.start()
        (workspace / 'config.toml').write_text('\n'.join([
            'model = "fixture-model"', 'model_provider = "fixture"',
            '[model_providers.fixture]', 'name = "offline fixture"',
            f'base_url = "http://127.0.0.1:{server.server_port}/v1"',
            'wire_api = "responses"', 'requires_openai_auth = false',
            'request_max_retries = 0', 'stream_max_retries = 0',
        ]))
        proc = None
        selector = selectors.DefaultSelector()
        buffer = b''
        try:
            with (output / 'stderr.txt').open('w') as errors:
                proc = subprocess.Popen([str(binary), 'app-server'], cwd=workspace,
                                        env={'PATH': '/usr/bin:/bin', 'CODEX_HOME': directory},
                                        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=errors)
                selector.register(proc.stdout, selectors.EVENT_READ)

                def send(message):
                    records.append({'sent': message})
                    proc.stdin.write(json.dumps(message).encode() + b'\n')
                    proc.stdin.flush()

                def receive(predicate):
                    nonlocal buffer
                    deadline = time.monotonic() + 25
                    while time.monotonic() < deadline:
                        if b'\n' not in buffer:
                            if not selector.select(max(0, deadline - time.monotonic())):
                                break
                            chunk = os.read(proc.stdout.fileno(), 65536)
                            if not chunk:
                                raise RuntimeError('app-server 提早關閉輸出')
                            buffer += chunk
                        while b'\n' in buffer:
                            line, buffer = buffer.split(b'\n', 1)
                            message = json.loads(line)
                            records.append({'received': message})
                            if predicate(message):
                                return message
                    raise TimeoutError('等候 app-server 事件逾時')

                def request(number, method, params):
                    send({'id': number, 'method': method, 'params': params})
                    response = receive(lambda message: message.get('id') == number)
                    if 'error' in response:
                        raise RuntimeError(response['error'])
                    return response['result']

                request(1, 'initialize', {'clientInfo': {'name': 'file_contract', 'version': '0.1.0'},
                                           'capabilities': {'experimentalApi': True}})
                send({'method': 'initialized', 'params': {}})
                tools = [{'type': 'function', 'name': name, 'description': 'Synthetic file task',
                          'inputSchema': {'type': 'object', 'properties': {field: {'type': 'string'}},
                                          'required': [field], 'additionalProperties': False}}
                         for name, field in [('read_fixture', 'path'), ('write_fixture', 'content')]]
                thread = request(2, 'thread/start', {'cwd': directory, 'model': 'fixture-model',
                    'modelProvider': 'fixture', 'approvalPolicy': 'untrusted', 'sandbox': 'read-only',
                    'ephemeral': True, 'dynamicTools': tools})
                request(3, 'turn/start', {'threadId': thread['thread']['id'],
                    'input': [{'type': 'text', 'text': '讀取 README.md，將大寫內容寫入成果檔'}]})
                for _ in range(4):
                    event = receive(lambda message: message.get('method') in ['item/tool/call', 'turn/completed'])
                    if event['method'] == 'turn/completed':
                        checks['harness_completed'] = event['params']['turn']['status'] == 'completed'
                        break
                    params = event['params']
                    name, args = params['tool'], params['arguments']
                    if case == 'denied':
                        value, success = '使用者拒絕工具操作', False
                    elif name == 'read_fixture' and args == {'path': 'README.md'}:
                        reads += 1
                        value, success = source.read_text(), True
                    elif name == 'write_fixture' and set(args) == {'content'} and isinstance(args['content'], str):
                        with artifact.open('x') as file:
                            file.write(args['content'])
                        writes += 1
                        value, success = '寫入完成，仍須獨立讀回驗證', True
                    else:
                        value, success = '工具或參數不在授權範圍', False
                    send({'id': event['id'], 'result': {'contentItems': [{'type': 'inputText', 'text': value}],
                                                      'success': success}})
                else:
                    raise RuntimeError('事件次數超出測試範圍')
                result = artifact.read_bytes() if artifact.exists() else None
                checks.update({
                    'expected_read_count': reads == (0 if case == 'denied' else 1),
                    'expected_write_count': writes == (1 if case in ['correct', 'wrong'] else 0),
                    'artifact_verdict': (result == b'ORIGINAL FIXTURE') == (case == 'correct'),
                    'source_unchanged': source.read_bytes() == b'original fixture',
                    'expected_request_count': len(requests) == (2 if case == 'missing' else 3),
                })
                (output / 'artifact.json').write_text(json.dumps({'content': result.decode() if result is not None else None,
                                                                'verified': result == b'ORIGINAL FIXTURE'}))
                proc.stdin.close()
                checks['clean_exit'] = proc.wait(timeout=10) == 0
        finally:
            if proc is not None and proc.poll() is None:
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
            (output / 'transcript.json').write_text(json.dumps(records, indent=2, ensure_ascii=False))
            (output / 'requests.json').write_text(json.dumps(requests, indent=2, ensure_ascii=False))
            (output / 'checks.json').write_text(json.dumps(checks, indent=2))
    if len(checks) != 7 or not all(checks.values()):
        raise RuntimeError(f'{case} 檢查失敗：{checks}')
    return checks


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    executable = shutil.which('codex')
    if executable is None:
        raise RuntimeError('找不到 Codex 執行檔')
    binary = Path(executable).resolve()
    results = {case: run_case(binary, args.output / case, case)
               for case in ['correct', 'wrong', 'missing', 'denied']}
    manifest = {'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                'version': subprocess.check_output([str(binary), '--version'], text=True).strip(),
                'script_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                'results': results, 'model': 'offline fixed Responses SSE',
                'not_proven': ['model reasoning', 'production CLI', 'OS sandbox', 'Windows/macOS']}
    (args.output / 'manifest.json').write_text(json.dumps(manifest, indent=2))
    print(json.dumps(results, indent=2))


if __name__ == '__main__':
    main()
