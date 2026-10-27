import datetime
import hashlib
import json
import os
from pathlib import Path
import selectors
import subprocess
import tempfile
import time


out = Path(__file__).resolve().parent
started = datetime.datetime.now(datetime.timezone.utc).isoformat()
records = []
checks = {}
binary = Path('/home/cash/.local/bin/codex').resolve()
with tempfile.TemporaryDirectory(prefix='agent-platform-tools-') as home:
    env = {'PATH': '/usr/bin:/bin', 'HOME': home, 'CODEX_HOME': home}
    config = '\n'.join([
        'model = "fixture-model"',
        'model_provider = "fixture"',
        '[model_providers.fixture]',
        'name = "isolated fixture"',
        'base_url = "http://127.0.0.1:9/v1"',
        'wire_api = "responses"',
        'requires_openai_auth = false',
    ])
    Path(home, 'config.toml').write_text(config)
    with (out / 'stderr.txt').open('w') as errors:
        proc = subprocess.Popen([str(binary), 'app-server'], cwd=home, env=env,
                                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=errors)
        selector = selectors.DefaultSelector()
        selector.register(proc.stdout, selectors.EVENT_READ)
        buffer = b''

        def request(number, method, params):
            global buffer
            message = {'id': number, 'method': method, 'params': params}
            records.append({'sent': message})
            proc.stdin.write(json.dumps(message).encode() + b'\n')
            proc.stdin.flush()
            deadline = time.monotonic() + 20
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
                    if response.get('id') == number:
                        return response
            raise TimeoutError(method)

        try:
            response = request(1, 'initialize', {
                'clientInfo': {'name': 'agent_platform_probe', 'version': '0.1.0'},
                'capabilities': {'experimentalApi': True},
            })
            checks['initialize'] = 'result' in response
            proc.stdin.write(b'{"method":"initialized","params":{}}\n')
            proc.stdin.flush()
            base = {'cwd': home, 'model': 'fixture-model', 'modelProvider': 'fixture',
                    'approvalPolicy': 'untrusted', 'sandbox': 'read-only', 'ephemeral': True}
            tool = {'type': 'function', 'name': 'fixture_read', 'description': 'Read synthetic fixture',
                    'inputSchema': {'type': 'object', 'properties': {}, 'additionalProperties': False}}
            good = request(2, 'thread/start', {**base, 'dynamicTools': [tool]})
            checks['valid_tool_registered'] = 'result' in good
            checks['read_only_retained'] = good.get('result', {}).get('sandbox', {}).get('type') == 'readOnly'
            checks['approval_retained'] = good.get('result', {}).get('approvalPolicy') == 'untrusted'
            bad = request(3, 'thread/start', {**base, 'dynamicTools': [{**tool, 'type': 'invalid'}]})
            checks['invalid_tool_type_rejected'] = bad.get('error', {}).get('code') == -32600
            invalid = request(4, 'thread/start', {**base, 'approvalPolicy': 'allow-everything'})
            checks['invalid_approval_rejected'] = invalid.get('error', {}).get('code') == -32600
            duplicate = request(5, 'thread/start', {**base, 'dynamicTools': [tool, tool]})
            checks['duplicate_tools_rejected'] = 'error' in duplicate
            proc.stdin.close()
            checks['eof_exit'] = proc.wait(timeout=10) == 0
        finally:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    proc.kill()
                    proc.wait(timeout=5)
            selector.close()
            (out / 'transcript.json').write_text(json.dumps(records, indent=2))
            (out / 'manifest.json').write_text(json.dumps({
                'started_at': started,
                'ended_at': datetime.datetime.now(datetime.timezone.utc).isoformat(),
                'commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
                'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
                'version': subprocess.check_output([str(binary), '--version'], text=True).strip(),
                'model': 'not_called', 'prompt': 'not_used', 'fixture': 'inline synthetic tool',
                'authorization': 'temporary home, loopback discard endpoint, no turn/start',
                'checks': checks, 'exit_code': proc.returncode,
                'not_run': ['tool execution', 'approval denial', 'Rig', 'cancel/resume',
                            'Rust/TS gates', 'Tauri desktop', 'deployment'],
            }, indent=2))
print(json.dumps(checks, indent=2))
raise SystemExit(0 if all(checks.values()) else 1)
