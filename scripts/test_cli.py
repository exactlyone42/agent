import json
import itertools
import os
import signal
import select
import sqlite3
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


ROOT = Path(__file__).resolve().parents[1]
BINARY = Path(os.environ.get('AGENT_CLI_BINARY', str(ROOT / 'target/debug' / ('agent-platform.exe' if os.name == 'nt' else 'agent-platform'))))


def wait_for_boundary(process, event, timeout):
    deadline = time.monotonic() + timeout
    while not event.wait(0.01):
        if process.poll() is not None or time.monotonic() >= deadline:
            return False
    return True


class CliContract(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.fixture_directory = tempfile.TemporaryDirectory(prefix='agent-cli-fixtures-')
        cls.library = Path(cls.fixture_directory.name) / 'fixture.so'
        if sys.platform == 'linux':
            subprocess.run(['cc', '-shared', '-fPIC', '-Wall', '-Wextra', '-Werror',
                str(ROOT / 'scripts/fixtures/open_race.c'), '-o', str(cls.library)], check=True)

    @classmethod
    def tearDownClass(cls):
        cls.fixture_directory.cleanup()

    def setUp(self):
        self.state_directory = tempfile.TemporaryDirectory(prefix='agent-cli-state-')
        self.previous_state = os.environ.get('AGENT_STATE_DIR')
        os.environ['AGENT_STATE_DIR'] = str(Path(self.state_directory.name) / 'private')

    def tearDown(self):
        if self.previous_state is None:
            os.environ.pop('AGENT_STATE_DIR', None)
        else:
            os.environ['AGENT_STATE_DIR'] = self.previous_state
        self.state_directory.cleanup()

    def stored_result(self, task_id):
        result = subprocess.run([str(BINARY), 'status', task_id], capture_output=True,
            text=True, encoding='utf-8', timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr)
        status = json.loads(result.stdout)
        result = subprocess.run([str(BINARY), 'events', task_id, '0'], capture_output=True,
            text=True, encoding='utf-8', timeout=5)
        self.assertEqual(result.returncode, 0, result.stderr)
        events = [json.loads(line) for line in result.stdout.splitlines()]
        self.assertEqual([event['seq'] for event in events], list(range(1, status['last_seq'] + 1)))
        self.assertEqual(events[0]['event_type'], 'started')
        self.assertTrue(all(event['task_id'] == task_id for event in events))
        return status, events

    def test_manual_approval_controls_real_tool_execution(self):
        cases = ['approve', 'deny', 'wrong_revision', 'expired', 'claim_failure', 'busy', 'receipt_failure']
        if sys.platform == 'linux':
            cases.extend(['receipt_busy', 'receipt_busy_timeout', 'receipt_busy_cancel'])
        if os.name != 'nt':
            cases.append('cancel')
        for action in cases:
            with self.subTest(action=action), tempfile.TemporaryDirectory(prefix='agent-approval-cli-') as directory:
                root = Path(directory)
                os.environ['AGENT_STATE_DIR'] = str(root / 'state')
                artifact = root / 'output.txt'
                requests = []

                class Handler(BaseHTTPRequestHandler):
                    def log_message(self, *_args):
                        pass

                    def do_POST(self):
                        requests.append(json.loads(self.rfile.read(int(self.headers['Content-Length']))))
                        if len(requests) == 1:
                            message = {'role': 'assistant', 'content': None, 'tool_calls': [{
                                'id': 'approval-write-1', 'type': 'function', 'function': {
                                    'name': 'write_output', 'arguments': json.dumps({'content': 'APPROVED'})}}]}
                        else:
                            message = {'role': 'assistant', 'content': 'done'}
                        payload = json.dumps({'id': 'approval-test', 'object': 'chat.completion',
                            'created': 1, 'model': 'fixture-model', 'choices': [
                            {'index': 0, 'message': message, 'finish_reason': 'tool_calls' if len(requests) == 1 else 'stop'}]}).encode()
                        self.send_response(200)
                        self.send_header('Content-Type', 'application/json')
                        self.send_header('Content-Length', str(len(payload)))
                        self.end_headers()
                        self.wfile.write(payload)

                server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
                worker = threading.Thread(target=server.serve_forever, kwargs={'poll_interval': 0.01})
                worker.start()
                task = root / 'task.json'
                task.write_text(json.dumps({'endpoint': f'http://127.0.0.1:{server.server_port}/v1',
                    'model': 'fixture-model', 'approval_mode': 'manual', 'task': {
                    'prompt': 'create approved output', 'input': 'unused', 'output': str(artifact),
                    'allow_write': True, 'expected': 'APPROVED'}}))
                descriptors = []
                launch = {}
                if action.startswith('receipt_busy'):
                    notify_read, notify_write = os.pipe()
                    release_read, release_write = os.pipe()
                    descriptors = [notify_read, notify_write, release_read, release_write]
                    launch = {'pass_fds': (notify_write, release_read), 'env': {
                        **os.environ, 'LD_PRELOAD': str(self.library),
                        'AGENT_TEST_BLOCK_SYNC': str(artifact),
                        'AGENT_TEST_SYNC_NOTIFY': str(notify_write),
                        'AGENT_TEST_SYNC_RELEASE': str(release_read)}}
                process = subprocess.Popen([str(BINARY), 'run', str(task)], cwd=root,
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding='utf-8', **launch)
                ready = threading.Event()
                identifiers = []

                def receive_id():
                    identifiers.append(process.stderr.readline())
                    ready.set()

                observer = threading.Thread(target=receive_id)
                observer.start()

                def decide(command, revision=1):
                    return subprocess.run([str(BINARY), command, approval['approval_id'], str(revision)],
                        capture_output=True, text=True, encoding='utf-8', timeout=5)

                try:
                    self.assertTrue(wait_for_boundary(process, ready, 5), 'task identifier missing')
                    observer.join(timeout=5)
                    self.assertTrue(identifiers[0].startswith('任務編號：'), identifiers)
                    task_id = identifiers[0].split('：')[1].strip()
                    pending = []
                    deadline = time.monotonic() + 5
                    while time.monotonic() < deadline and process.poll() is None:
                        query = subprocess.run([str(BINARY), 'approvals', task_id], capture_output=True,
                            text=True, encoding='utf-8', timeout=5)
                        self.assertEqual(query.returncode, 0, query.stderr)
                        pending = json.loads(query.stdout)
                        if pending:
                            break
                        time.sleep(0.02)
                    self.assertEqual(len(pending), 1)
                    approval = pending[0]
                    self.assertEqual(approval['status'], 'pending')
                    self.assertEqual(approval['scope_path'], str(artifact))
                    self.assertEqual(json.loads(approval['args_json']), {'content': 'APPROVED'})
                    self.assertEqual(approval['uses_remaining'], 1)
                    self.assertFalse(artifact.exists())
                    status, _ = self.stored_result(task_id)
                    self.assertEqual(status['recorded_state'], 'waiting_approval')
                    if action == 'wrong_revision':
                        rejected = decide('approve', 2)
                        self.assertEqual(rejected.returncode, 1)
                        self.assertFalse(artifact.exists())
                    if action == 'busy':
                        with sqlite3.connect(root / 'state/tasks.sqlite3') as database:
                            database.execute('BEGIN IMMEDIATE')
                            time.sleep(0.9)
                            self.assertIsNone(process.poll(), 'temporary database lock stopped the task')
                            self.assertFalse(artifact.exists())
                            database.rollback()
                    if action == 'expired':
                        with sqlite3.connect(root / 'state/tasks.sqlite3') as database:
                            database.execute('UPDATE approvals SET expires_at=0')
                        self.assertEqual(decide('approve').returncode, 1)
                    elif action == 'cancel':
                        process.send_signal(signal.SIGINT)
                    else:
                        if action == 'claim_failure':
                            with sqlite3.connect(root / 'state/tasks.sqlite3') as database:
                                database.execute("CREATE TRIGGER reject_dispatch BEFORE INSERT ON events WHEN NEW.event_type='tool_dispatched' BEGIN SELECT RAISE(ABORT,'injected failure'); END")
                        if action == 'receipt_failure':
                            with sqlite3.connect(root / 'state/tasks.sqlite3') as database:
                                database.execute("CREATE TRIGGER reject_tool_receipt BEFORE INSERT ON events WHEN NEW.event_type='tool_succeeded' BEGIN SELECT RAISE(ABORT,'injected receipt failure'); END")
                        decision = decide('deny' if action == 'deny' else 'approve')
                        self.assertEqual(decision.returncode, 0, decision.stderr)
                    if action.startswith('receipt_busy'):
                        self.assertTrue(select.select([notify_read], [], [], 5)[0], 'artifact sync not reached')
                        self.assertEqual(os.read(notify_read, 1), b'1')
                        with sqlite3.connect(root / 'state/tasks.sqlite3') as database:
                            database.execute('BEGIN IMMEDIATE')
                            os.write(release_write, b'1')
                            if action == 'receipt_busy_timeout':
                                process.wait(timeout=5)
                            elif action == 'receipt_busy_cancel':
                                time.sleep(0.55)
                                process.send_signal(signal.SIGINT)
                                time.sleep(0.1)
                            else:
                                time.sleep(0.9)
                                self.assertIsNone(process.poll())
                            database.rollback()
                    stdout, stderr = process.communicate(timeout=10)
                    if action in ['approve', 'wrong_revision', 'busy', 'receipt_busy']:
                        self.assertEqual(process.returncode, 0, stderr)
                        self.assertEqual(artifact.read_text(), 'APPROVED')
                        self.assertEqual(json.loads(stdout)['artifact'], 'verified')
                        self.assertEqual(len(requests), 2)
                    else:
                        self.assertEqual(process.returncode, 130 if action in ['cancel', 'receipt_busy_cancel'] else 1, stderr)
                        if action in ['receipt_failure', 'receipt_busy_timeout', 'receipt_busy_cancel']:
                            self.assertEqual(artifact.read_text(), 'APPROVED')
                            if action == 'receipt_busy_cancel':
                                self.assertEqual(json.loads(stdout)['state'], 'outcome_unknown')
                            else:
                                self.assertEqual(stdout, '')
                        else:
                            self.assertFalse(artifact.exists())
                        self.assertEqual(len(requests), 1)
                        if action in ['deny', 'expired']:
                            self.assertIn('未獲批准', stderr)
                    self.assertEqual(decide('approve').returncode, 1)
                    query = subprocess.run([str(BINARY), 'approvals', task_id], capture_output=True,
                        text=True, encoding='utf-8', timeout=5)
                    saved = json.loads(query.stdout)[0]
                    self.assertEqual(saved['uses_remaining'], 0 if action in ['approve', 'wrong_revision', 'busy', 'receipt_busy', 'receipt_busy_timeout', 'receipt_busy_cancel', 'receipt_failure'] else 1)
                    if action in ['receipt_failure', 'receipt_busy_cancel']:
                        self.assertEqual(saved['intent_status'], 'outcome_unknown')
                        status, events = self.stored_result(task_id)
                        self.assertEqual(status['recorded_state'], 'outcome_unknown')
                        self.assertEqual(events[-1]['event_type'], 'tool_outcome_unknown')
                    if action in ['cancel', 'claim_failure']:
                        self.assertEqual(saved['status'], 'revoked')
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.communicate(timeout=5)
                    observer.join(timeout=5)
                    for descriptor in descriptors:
                        os.close(descriptor)
                    server.shutdown()
                    server.server_close()
                    worker.join(timeout=5)

    def test_queries_refuse_missing_database_and_invalid_arguments(self):
        for command in [['status', 'missing'], ['events', 'missing'], ['events', 'missing', 'x'],
                        ['approve', 'missing', '1'], ['deny', 'missing', '1'], ['approvals', 'missing'],
                        ['events', 'missing', '9223372036854775808'], ['status'], ['events', 'id', '0', 'extra']]:
            with self.subTest(command=command):
                result = subprocess.run([str(BINARY), *command], capture_output=True,
                    text=True, encoding='utf-8', timeout=5)
                self.assertEqual(result.returncode, 1)
                self.assertEqual(result.stdout, '')
                self.assertFalse(Path(os.environ['AGENT_STATE_DIR']).exists())

    def test_invalid_state_directory_prevents_start(self):
        with tempfile.TemporaryDirectory(prefix='agent-state-denied-') as directory:
            root = Path(directory)
            task = root / 'task.json'
            task.write_text(json.dumps({'endpoint': 'http://127.0.0.1:1/v1', 'model': 'fixture-model',
                'task': {'prompt': 'work', 'input': 'unused', 'output': str(root / 'output'), 'allow_write': True}}))
            for value in ['', 'relative/path', str(task)]:
                with self.subTest(value=value):
                    result = subprocess.run([str(BINARY), 'run', str(task)],
                        env={**os.environ, 'AGENT_STATE_DIR': value}, capture_output=True,
                        text=True, encoding='utf-8', timeout=5)
                    self.assertEqual(result.returncode, 1)
                    self.assertNotIn('任務編號：', result.stderr)
                    self.assertFalse((root / 'output').exists())

    @unittest.skipUnless(os.name == 'posix', 'Unix 資料目錄與連結拒絕測試')
    def test_state_directory_and_database_links_are_rejected(self):
        with tempfile.TemporaryDirectory(prefix='agent-state-links-') as directory:
            root = Path(directory)
            state = root / 'state'
            state.mkdir(mode=0o700)
            target = root / 'target'
            target.write_text('preserve this file')
            database = state / 'tasks.sqlite3'
            scenarios = ['public_directory', 'symlink', 'hardlink', 'fifo']
            for scenario in scenarios:
                with self.subTest(scenario=scenario):
                    if scenario == 'public_directory':
                        state.chmod(0o755)
                    elif scenario == 'symlink':
                        database.symlink_to(target)
                    elif scenario == 'hardlink':
                        os.link(target, database)
                    else:
                        os.mkfifo(database)
                    try:
                        result = subprocess.run([str(BINARY), 'status', 'missing'],
                            env={**os.environ, 'AGENT_STATE_DIR': str(state)}, capture_output=True,
                            text=True, encoding='utf-8', timeout=5)
                        self.assertEqual(result.returncode, 1)
                        self.assertEqual(result.stdout, '')
                        self.assertEqual(target.read_text(), 'preserve this file')
                    finally:
                        if scenario == 'public_directory':
                            state.chmod(0o700)
                        else:
                            database.unlink()

    @unittest.skipUnless(sys.platform == 'linux', 'Linux 隔離開檔阻塞注入')
    def test_second_interrupt_exits_even_when_file_open_is_stuck(self):
        with tempfile.TemporaryDirectory(prefix='agent-cancel-block-') as directory:
            root = Path(directory)
            library = root / 'fixture.so'
            subprocess.run(['cc', '-shared', '-fPIC', '-Wall', '-Wextra', '-Werror',
                str(ROOT / 'scripts/fixtures/open_race.c'), '-o', str(library)], check=True)
            source = root / 'source.txt'
            source.write_text('preserve source')
            task = root / 'task.json'
            task.write_text(json.dumps({'endpoint': 'http://127.0.0.1:1/v1', 'model': 'fixture-model',
                'task': {'prompt': 'work', 'input': str(source), 'output': str(root / 'output.txt'),
                         'allow_read': True, 'allow_write': True}}))
            read_fd, write_fd = os.pipe()
            reached = threading.Event()

            def receive():
                if os.read(read_fd, 1) == b'1':
                    reached.set()

            observer = threading.Thread(target=receive)
            observer.start()
            process = subprocess.Popen([str(BINARY), 'run', str(task)], cwd=root,
                env={**os.environ, 'LD_PRELOAD': str(library), 'AGENT_TEST_BLOCK_INPUT': str(source),
                     'AGENT_TEST_BLOCK_NOTIFY_FD': str(write_fd)}, pass_fds=(write_fd,),
                stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding='utf-8')
            os.close(write_fd)
            try:
                self.assertTrue(wait_for_boundary(process, reached, 5), 'file open did not reach blocking boundary')
                process.send_signal(signal.SIGINT)
                with self.assertRaises(subprocess.TimeoutExpired):
                    process.communicate(timeout=0.5)
                process.send_signal(signal.SIGINT)
                stdout, stderr = process.communicate(timeout=5)
                self.assertEqual(process.returncode, 130, stderr)
                self.assertEqual(stdout, '')
                self.assertEqual(source.read_text(), 'preserve source')
                self.assertFalse((root / 'output.txt').exists())
                task_id = stderr.split('任務編號：')[1].splitlines()[0]
                status, events = self.stored_result(task_id)
                self.assertEqual(status['recorded_state'], 'running')
                self.assertEqual(len(events), 1)
            finally:
                if process.poll() is None:
                    process.kill()
                    process.communicate(timeout=5)
                observer.join(timeout=5)
                os.close(read_fd)

    @unittest.skipIf(os.name == 'nt', 'Windows 控制台停止事件須實機驗收')
    def test_interrupt_stops_pending_model_and_preserves_prior_output(self):
        for (already_written, write_failed), stop_signal in itertools.product(
                [(False, False), (True, False), (True, True)], [signal.SIGINT, signal.SIGTERM, signal.SIGHUP, signal.SIGKILL]):
            if write_failed and sys.platform != 'linux':
                continue
            with self.subTest(already_written=already_written, write_failed=write_failed, stop_signal=stop_signal), tempfile.TemporaryDirectory(prefix='agent-cancel-') as directory:
                root = Path(directory)
                os.environ['AGENT_STATE_DIR'] = str(root / 'state')
                artifact = root / 'output.txt'
                ready = threading.Event()
                disconnected = threading.Event()
                requests = []

                class Handler(BaseHTTPRequestHandler):
                    def log_message(self, *_args):
                        pass

                    def do_POST(self):
                        requests.append(json.loads(self.rfile.read(int(self.headers['Content-Length']))))
                        if already_written and len(requests) == 1:
                            message = {'role': 'assistant', 'content': None, 'tool_calls': [{
                                'id': 'write-once', 'type': 'function', 'function': {
                                    'name': 'write_output', 'arguments': json.dumps({'content': 'preserve this'})}}]}
                            payload = json.dumps({'id': 'pending-test', 'object': 'chat.completion',
                                'created': 1, 'model': 'fixture-model', 'choices': [
                                {'index': 0, 'message': message, 'finish_reason': 'tool_calls'}]}).encode()
                            self.send_response(200)
                            self.send_header('Content-Type', 'application/json')
                            self.send_header('Content-Length', str(len(payload)))
                            self.end_headers()
                            self.wfile.write(payload)
                            return
                        self.connection.settimeout(5)
                        ready.set()
                        try:
                            if self.connection.recv(1) == b'':
                                disconnected.set()
                        except ConnectionResetError:
                            disconnected.set()

                server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
                worker = threading.Thread(target=server.serve_forever, kwargs={'poll_interval': 0.01})
                worker.start()
                path = root / 'task.json'
                path.write_text(json.dumps({'endpoint': f'http://127.0.0.1:{server.server_port}/v1',
                    'model': 'fixture-model', 'task': {'prompt': 'produce output', 'input': 'unused.txt',
                    'output': str(artifact), 'allow_write': True, 'expected': 'preserve this'}}))
                command = [str(BINARY), 'run', str(path)]
                environment = dict(os.environ)
                if write_failed:
                    environment.update(LD_PRELOAD=str(self.library), AGENT_TEST_FAIL_WRITE=str(artifact))
                process = subprocess.Popen(command, cwd=root, env=environment,
                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding='utf-8')
                try:
                    self.assertTrue(wait_for_boundary(process, ready, 10), 'model request did not reach cancellation boundary')
                    process.send_signal(stop_signal)
                    stdout, stderr = process.communicate(timeout=5)
                    if stop_signal == signal.SIGKILL:
                        self.assertEqual(process.returncode, -signal.SIGKILL)
                        self.assertEqual(stdout, '')
                        task_id = stderr.split('任務編號：')[1].splitlines()[0]
                        status, events = self.stored_result(task_id)
                        self.assertEqual(status['recorded_state'], 'running')
                        self.assertEqual(len(events), 4 if already_written else 1)
                        self.assertIsNone(status['artifact_created'])
                    else:
                        self.assertEqual(process.returncode, 130, stderr)
                        report = json.loads(stdout)
                        self.assertEqual(report['state'], 'cancelled')
                        self.assertEqual(report['artifact_created'], already_written)
                        status, events = self.stored_result(report['task_id'])
                        self.assertEqual(status['recorded_state'], 'cancelled')
                        self.assertEqual(status['artifact_created'], already_written)
                        self.assertEqual(events[-1]['event_type'], 'cancelled')
                        self.assertIn('已停止本機任務', stderr)
                    self.assertTrue(disconnected.wait(5), 'model connection remained open')
                    self.assertEqual(artifact.exists(), already_written)
                    if already_written:
                        self.assertEqual(artifact.read_text(), '' if write_failed else 'preserve this')
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.communicate(timeout=5)
                    server.shutdown()
                    server.server_close()
                    worker.join(timeout=5)

    @unittest.skipUnless(sys.platform == 'linux', 'Linux isolated file replacement fixture')
    def test_source_replaced_after_precheck_is_rejected(self):
        with tempfile.TemporaryDirectory(prefix='agent-cli-race-') as directory:
            workspace = Path(directory)
            library = workspace / 'race.so'
            subprocess.run(['cc', '-shared', '-fPIC', '-Wall', '-Wextra', '-Werror',
                            str(ROOT / 'scripts/fixtures/open_race.c'), '-o', str(library)],
                           check=True, capture_output=True, timeout=20)
            for kind in ['directory', 'fifo', 'file', 'symlink']:
                with self.subTest(kind=kind):
                    case = workspace / kind
                    case.mkdir()
                    source, backup = case / 'input', case / 'original'
                    source.write_text('unchanged original')
                    replacement = case / 'other'
                    replacement.write_text('not authorized')
                    request = {'endpoint': 'http://127.0.0.1:1/v1', 'model': 'fixture-model',
                               'task': {'input': str(source), 'output': str(case / 'output'),
                                        'prompt': 'work', 'allow_read': True}}
                    path = case / 'task.json'
                    path.write_text(json.dumps(request))
                    result = subprocess.run([str(BINARY), 'run', str(path)], text=True, encoding="utf-8",
                                            capture_output=True, timeout=5,
                                            env={**os.environ, 'LD_PRELOAD': str(library),
                                                 'AGENT_TEST_REPLACE_INPUT': str(source),
                                                 'AGENT_TEST_ORIGINAL_INPUT': str(backup),
                                                 'AGENT_TEST_REPLACEMENT_KIND': kind,
                                                 'AGENT_TEST_REPLACEMENT_FILE': str(replacement)})
                    self.assertEqual(result.returncode, 1, result.stderr)
                    self.assertIn('來源必須是', result.stderr)
                    self.assertEqual(backup.read_text(), 'unchanged original')
                    self.assertEqual(replacement.read_text(), 'not authorized')
                    self.assertNotIn('not authorized', result.stdout + result.stderr)
                    self.assertFalse((case / 'output').exists())

    @unittest.skipUnless(hasattr(os, 'mkfifo'), 'FIFO is a Unix fixture')
    def test_fifo_source_is_rejected_without_waiting_for_a_writer(self):
        with tempfile.TemporaryDirectory(prefix='agent-cli-fifo-') as directory:
            workspace = Path(directory)
            source = workspace / 'source'
            os.mkfifo(source)
            request = {'endpoint': 'http://127.0.0.1:1/v1', 'model': 'fixture-model',
                       'task': {'input': str(source), 'output': str(workspace / 'output'),
                                'prompt': 'work', 'allow_read': True}}
            task = workspace / 'task.json'
            task.write_text(json.dumps(request))
            result = subprocess.run([str(BINARY), 'run', str(task)],
                                    text=True, encoding="utf-8", capture_output=True, timeout=2)
            self.assertEqual(result.returncode, 1)
            self.assertIn('來源必須是', result.stderr)
            self.assertFalse((workspace / 'output').exists())
            config_result = subprocess.run([str(BINARY), 'run', str(source)],
                                           text=True, encoding="utf-8", capture_output=True, timeout=2)
            self.assertEqual(config_result.returncode, 1)
            self.assertIn('無法開啟任務檔', config_result.stderr)

    def test_command_arguments(self):
        for args, expected in [(['--help'], 0), ([], 1), (['run'], 1),
                               (['other', 'missing.json'], 1), (['--help', 'extra'], 1)]:
            with self.subTest(args=args):
                result = subprocess.run([str(BINARY), *args], text=True, encoding="utf-8",
                                        capture_output=True, timeout=5)
                self.assertEqual(result.returncode, expected)
                self.assertIn('用法：', result.stdout if expected == 0 else result.stderr)

    def test_url_policy_and_json_size(self):
        with tempfile.TemporaryDirectory(prefix='agent-cli-policy-') as directory:
            workspace = Path(directory)
            path = workspace / 'task.json'
            request = {'endpoint': 'http://127.0.0.1:1/v1', 'model': 'fixture-model',
                       'task': {'input': 'missing.txt', 'output': 'output.txt',
                                'prompt': 'work', 'allow_read': True}}
            rejected = ['http://localhost:1/v1', 'http://192.0.2.1/v1',
                        'http://[2001:db8::1]/v1', 'ftp://127.0.0.1/v1',
                        'http://user@127.0.0.1/v1', 'http://:secret@127.0.0.1/v1',
                        'http://127.0.0.1/v1?key=value', 'http://127.0.0.1/v1#fragment']
            for endpoint in rejected + ['http://127.0.0.1:1/v1', 'http://[::1]:1/v1']:
                with self.subTest(endpoint=endpoint):
                    request['endpoint'] = endpoint
                    path.write_text(json.dumps(request))
                    result = subprocess.run([str(BINARY), 'run', str(path)], cwd=workspace,
                                            text=True, encoding="utf-8", capture_output=True, timeout=5)
                    self.assertEqual(result.returncode, 1)
                    self.assertIn('目前只接受' if endpoint in rejected else '無法開啟獲准的來源',
                                  result.stderr)
            request['endpoint'] = 'http://127.0.0.1:1/v1'
            payload = json.dumps(request)
            for size in [65535, 65536, 65537]:
                with self.subTest(size=size):
                    path.write_text(payload + ' ' * (size - len(payload)))
                    result = subprocess.run([str(BINARY), 'run', str(path)], cwd=workspace,
                                            text=True, encoding="utf-8", capture_output=True, timeout=5)
                    self.assertEqual(result.returncode, 1)
                    self.assertIn('任務檔超過' if size > 65536 else '無法開啟獲准的來源', result.stderr)

    def test_file_tasks_through_real_cli_and_http(self):
        for case, exit_code in [('correct', 0), ('wrong', 1), ('missing', 1),
                                ('denied', 1), ('existing', 1), ('review', 2),
                                ('malformed', 1), ('corrupted_spec', 1), ('redirect', 1), ('existing_equal', 1),
                                ('read_limit', 0), ('write_limit', 2), ('write_oversize', 1),
                                ('source_grew', 0), ('duplicate_write', 0), ('reused_call_id', 0), ('receipt_failed', 1), ('write_failed', 1)]:
            with self.subTest(case=case), tempfile.TemporaryDirectory(prefix='agent-cli-test-') as directory:
                if case == 'write_failed' and sys.platform != 'linux':
                    self.skipTest('指定成果寫入失敗的故障注入僅適用 Linux')
                workspace = Path(directory)
                os.environ['AGENT_STATE_DIR'] = str(workspace / 'state')
                source, artifact = workspace / 'input.txt', workspace / 'output.txt'
                source_content = 'x' * 1048576 if case in ['read_limit', 'source_grew'] else 'original fixture'
                output_content = ('x' * (1048576 if case == 'write_limit' else 1048577)
                                  if case in ['write_limit', 'write_oversize'] else 'ORIGINAL FIXTURE')
                source.write_text(source_content)
                if case == 'existing':
                    artifact.write_text('keep existing')
                if case == 'existing_equal':
                    artifact.write_text('ORIGINAL FIXTURE')
                requests = []

                class Handler(BaseHTTPRequestHandler):
                    def log_message(self, *_args):
                        pass

                    def do_POST(self):
                        requests.append(json.loads(self.rfile.read(int(self.headers['Content-Length']))))
                        if case == 'redirect':
                            self.send_response(307)
                            self.send_header('Location', f'http://127.0.0.1:{self.server.server_port}/redirected')
                            self.send_header('Content-Length', '0')
                            self.end_headers()
                            return
                        index = len(requests)
                        if index == 1:
                            if case == 'corrupted_spec':
                                with sqlite3.connect(Path(os.environ['AGENT_STATE_DIR']) / 'tasks.sqlite3') as database:
                                    database.execute("UPDATE task_specs SET spec_hash='corrupted'")
                            if case == 'source_grew':
                                with source.open('a') as changed:
                                    changed.write('y')
                            name, args = 'read_input', {}
                        elif index == 2 and case != 'missing':
                            name = 'write_output'
                            args = {'content': 'incorrect content' if case == 'wrong' else output_content}
                            if case == 'malformed':
                                args['path'] = 'unapproved.txt'
                        elif index == 3 and case == 'duplicate_write':
                            name, args = 'write_output', {'content': 'unexpected second write'}
                        else:
                            name, args = None, None
                            if case == 'receipt_failed':
                                with sqlite3.connect(Path(os.environ['AGENT_STATE_DIR']) / 'tasks.sqlite3') as database:
                                    database.execute("CREATE TRIGGER reject_receipt BEFORE INSERT ON events WHEN NEW.event_type='succeeded' BEGIN SELECT RAISE(ABORT, 'injected receipt failure'); END")
                        message = {'role': 'assistant', 'content': 'done'}
                        if name:
                            message = {'role': 'assistant', 'content': None, 'tool_calls': [
                                {'id': 'reused-id' if case == 'reused_call_id' else f'call_{index}', 'type': 'function',
                                 'function': {'name': name, 'arguments': json.dumps(args)}}]}
                        response = {'id': f'chat_{index}', 'object': 'chat.completion', 'created': 1,
                                    'model': 'fixture-model', 'choices': [{'index': 0, 'message': message,
                                    'finish_reason': 'tool_calls' if name else 'stop'}],
                                    'usage': {'prompt_tokens': 1, 'completion_tokens': 1, 'total_tokens': 2}}
                        payload = json.dumps(response).encode()
                        self.send_response(200)
                        self.send_header('Content-Type', 'application/json')
                        self.send_header('Content-Length', str(len(payload)))
                        self.end_headers()
                        self.wfile.write(payload)

                server = ThreadingHTTPServer(('127.0.0.1', 0), Handler)
                worker = threading.Thread(target=server.serve_forever, kwargs={'poll_interval': 0.01})
                worker.start()
                request = {'endpoint': f'http://127.0.0.1:{server.server_port}/v1', 'model': 'fixture-model',
                           'task': {'prompt': '讀來源並寫入大寫成果', 'input': str(source), 'output': str(artifact),
                                    'allow_read': case != 'denied', 'allow_write': case != 'denied',
                                    'expected': None if case in ['review', 'write_limit'] else
                                                '' if case == 'write_failed' else 'ORIGINAL FIXTURE'}}
                task_file = workspace / 'task.json'
                task_file.write_text(json.dumps(request))
                command = [str(BINARY), 'run', str(task_file)]
                environment = {**os.environ, 'AGENT_API_KEY': 'test-secret-canary'}
                if case == 'write_failed':
                    environment.update(LD_PRELOAD=str(self.library), AGENT_TEST_FAIL_WRITE=str(artifact))
                try:
                    result = subprocess.run(command, cwd=workspace,
                                            env=environment,
                                            text=True, encoding="utf-8", capture_output=True, timeout=20)
                finally:
                    server.shutdown()
                    server.server_close()
                    worker.join(timeout=5)
                self.assertEqual(result.returncode, exit_code, result.stderr)
                task_id = result.stderr.split('任務編號：')[1].splitlines()[0]
                status, events = self.stored_result(task_id)
                expected_state = ('running' if case == 'receipt_failed' else
                                  'succeeded' if exit_code == 0 else 'needs_review' if exit_code == 2 else
                                  'outcome_unknown' if case in ['redirect', 'corrupted_spec'] else 'failed')
                self.assertEqual(status['recorded_state'], expected_state)
                self.assertEqual(events[-1]['event_type'], 'tool_succeeded' if case == 'receipt_failed' else expected_state)
                with sqlite3.connect(Path(os.environ['AGENT_STATE_DIR']) / 'tasks.sqlite3') as database:
                    self.assertNotIn('test-secret-canary', '\n'.join(database.iterdump()))
                    if case == 'receipt_failed':
                        database.execute('DROP TRIGGER reject_receipt')
                if exit_code == 1:
                    message = ('成果驗證未通過' if case == 'wrong' else
                               '任務結果未確認' if case == 'receipt_failed' else
                               '成果未完整寫入' if case == 'write_failed' else
                               '模型執行失敗' if case in ['redirect', 'corrupted_spec'] else
                               '沒有可驗證的成果檔案')
                    self.assertIn(message, result.stderr)
                self.assertNotIn('test-secret-canary', result.stdout + result.stderr)
                self.assertEqual(source.read_text(), source_content + ('y' if case == 'source_grew' else ''))
                if case in ['correct', 'review', 'read_limit', 'write_limit', 'source_grew', 'duplicate_write', 'reused_call_id']:
                    report = json.loads(result.stdout)
                    self.assertEqual(report['artifact'], 'verified' if exit_code == 0 else 'awaiting_review')
                    self.assertEqual(artifact.read_text(), output_content)
                    if case == 'duplicate_write':
                        self.assertEqual(len(requests), 4)
                        messages = [m for m in requests[3]['messages'] if m['role'] == 'tool']
                        self.assertIn('成果已寫入，不重複執行', messages[-1]['content'])
                elif case == 'wrong':
                    self.assertEqual(artifact.read_text(), 'incorrect content')
                elif case == 'existing':
                    self.assertEqual(artifact.read_text(), 'keep existing')
                elif case == 'existing_equal':
                    self.assertEqual(artifact.read_text(), 'ORIGINAL FIXTURE')
                elif case == 'write_failed':
                    self.assertEqual(artifact.read_text(), '')
                elif case == 'receipt_failed':
                    self.assertEqual(result.stdout, '')
                    self.assertEqual(artifact.read_text(), output_content)
                else:
                    self.assertFalse(artifact.exists())
                if case in ['redirect', 'corrupted_spec']:
                    self.assertEqual(len(requests), 1)
                else:
                    self.assertGreaterEqual(len(requests), 2)
                    tool_messages = [m for m in requests[1]['messages'] if m['role'] == 'tool']
                    if case == 'denied':
                        self.assertNotIn('original fixture', json.dumps(tool_messages))
                    elif case == 'source_grew':
                        self.assertTrue(any('來源超出大小限制' in m['content'] for m in tool_messages))
                        self.assertFalse(any(source_content in m['content'] for m in tool_messages))
                    else:
                        self.assertTrue(any(m['content'] == source_content for m in tool_messages))
                    definitions = {tool['function']['name']: tool['function'] for tool in requests[0]['tools']}
                    self.assertEqual(definitions['read_input']['description'], '讀取使用者指定且獲准的來源檔案')
                    self.assertEqual(definitions['write_output']['description'], '建立使用者指定且獲准的新成果檔案，不覆寫既有資料')
                    self.assertEqual(definitions['read_input']['parameters'],
                                     {'type': 'object', 'properties': {}, 'additionalProperties': False})
                    self.assertEqual(definitions['write_output']['parameters'],
                                     {'type': 'object', 'properties': {'content': {'type': 'string'}},
                                      'required': ['content'], 'additionalProperties': False})
                self.assertFalse((workspace / 'unapproved.txt').exists())

    @unittest.skipUnless(os.supports_bytes_environ, '作業系統不支援位元組環境變數')
    def test_invalid_key_encoding_fails_before_connection(self):
        with tempfile.TemporaryDirectory(prefix='agent-cli-key-') as directory:
            root = Path(directory)
            path = root / 'task.json'
            path.write_text(json.dumps({'endpoint': 'http://127.0.0.1:1/v1', 'model': 'fixture-model',
                'task': {'prompt': 'work', 'input': 'missing.txt', 'output': 'new.txt'}}))
            result = subprocess.run([str(BINARY), 'run', str(path)], cwd=root,
                env={**os.environb, b'AGENT_API_KEY': b'\xff'}, text=True, encoding='utf-8',
                capture_output=True, timeout=5)
            self.assertEqual(result.returncode, 1)
            self.assertIn('AGENT_API_KEY 格式錯誤', result.stderr)
            self.assertFalse((root / 'new.txt').exists())

    def test_invalid_configuration_does_not_start_task(self):
        with tempfile.TemporaryDirectory(prefix='agent-cli-config-') as directory:
            workspace = Path(directory)
            path = workspace / 'task.json'
            for value in ['{', 'x' * 65537, json.dumps({'endpoint': 'https://example.com/v1',
                    'model': 'fixture-model', 'task': {'input': 'missing.txt', 'output': 'new.txt',
                    'prompt': 'do work', 'allow_read': True, 'allow_write': True}})]:
                with self.subTest(value_length=len(value)):
                    path.write_text(value)
                    result = subprocess.run([str(BINARY), 'run', str(path)], cwd=workspace,
                                            text=True, encoding="utf-8", capture_output=True, timeout=5)
                    self.assertEqual(result.returncode, 1)
                    self.assertFalse((workspace / 'new.txt').exists())
                    self.assertNotIn('無法開啟獲准的來源', result.stderr)


if __name__ == '__main__':
    unittest.main()
