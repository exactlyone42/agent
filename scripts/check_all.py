import hashlib
import json
import os
import platform
from contextlib import contextmanager
from pathlib import Path
import subprocess
import tempfile
from datetime import datetime, timezone
import uuid


ROOT = Path(__file__).resolve().parents[1]


@contextmanager
def claim_run(marker):
    marker.parent.mkdir(parents=True, exist_ok=True)
    with marker.open('x') as file:
        json.dump({'pid': os.getpid(), 'started': datetime.now(timezone.utc).isoformat()}, file)
    yield
    marker.unlink()


def source_hashes(root):
    files = [root / name for name in ['Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml']]
    for directory in ['crates', 'scripts', '.cargo']:
        files.extend(path for path in (root / directory).rglob('*')
                     if path.is_file() and '__pycache__' not in path.parts)
    return {str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in sorted(files)}


def run_step(name, command, root, evidence):
    print(f'執行 {name}', flush=True)
    result = {'command': command, 'started': datetime.now(timezone.utc).isoformat()}
    with (evidence / f'{name}.txt').open('x', encoding='utf-8') as log:
        try:
            result['exit_code'] = subprocess.run(command, cwd=root, stdout=log,
                                                 stderr=subprocess.STDOUT).returncode
        except OSError as error:
            log.write(f'{type(error).__name__}: {error}\n')
            result['exit_code'] = 127
    result['finished'] = datetime.now(timezone.utc).isoformat()
    (evidence / f'{name}.json').write_text(json.dumps(result, ensure_ascii=False, indent=2) + '\n')
    return result['exit_code']


def secret_command(snapshot, evidence):
    config = evidence / 'secrets-config.toml'
    config.write_bytes((ROOT / 'scripts/policy/gitleaks.toml').read_bytes())
    ignore = evidence / 'secrets-empty.ignore'
    ignore.write_text('')
    return ['gitleaks', 'dir', '.', '--redact', '--no-banner',
            '--config', str(config), '--gitleaks-ignore-path', str(ignore),
            '--ignore-gitleaks-allow', '--report-format', 'json',
            '--report-path', str(evidence / 'secrets-report.json')]


def copy_snapshot(root, snapshot, names):
    copied = 0
    for name in sorted(set(names) - {''}):
        source = root / name
        if not source.is_symlink() and not source.exists():
            continue
        resolved = source.resolve(strict=True)
        if not resolved.is_relative_to(root.resolve()):
            raise ValueError(f'秘密掃描拒絕讀取專案外連結：{name}')
        target = snapshot / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(resolved.read_bytes())
        copied += 1
    if copied == 0:
        raise ValueError('秘密掃描沒有任何檔案，不能判定通過')
    return copied


def validate_reports(evidence):
    coverage = json.loads((evidence / 'coverage-report.json').read_text())
    totals = [item['totals']['lines'] for item in coverage['data']]
    mutation = json.loads((evidence / 'mutation/mutants.out/outcomes.json').read_text())
    return bool(totals) and all(item['count'] > 0 and item['covered'] > 0 for item in totals) and (
        mutation['total_mutants'] > 0 and mutation['caught'] > 0
        and mutation['missed'] == 0 and mutation['timeout'] == 0
        and mutation['caught'] + mutation['unviable'] == mutation['total_mutants'])


def main():
    stamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')
    evidence = ROOT / '.runtime/checks' / f'{stamp}-{uuid.uuid4().hex[:8]}'
    evidence.mkdir(parents=True)
    print(f'本輪原件：{evidence}', flush=True)
    before = source_hashes(ROOT)
    (evidence / 'source-before.json').write_text(json.dumps(before, indent=2) + '\n')
    (evidence / 'platform.json').write_text(json.dumps({
        'system': platform.system(), 'machine': platform.machine(),
        'release': platform.release()}, indent=2) + '\n')
    steps = [('gate', ['bash', 'scripts/check.sh', 'gate']),
             ('versions-rust', ['rustc', '--version']),
             ('versions-coverage', ['cargo', 'llvm-cov', '--version']),
             ('versions-mutation', ['cargo', 'mutants', '--version']),
             ('versions-secrets', ['gitleaks', 'version']),
             ('versions-dependencies', ['cargo', 'machete', '--version'])]
    results = {}
    for name, command in steps:
        results[name] = run_step(name, command, ROOT, evidence)
    if not any(results.values()):
        with tempfile.TemporaryDirectory(prefix='agent-secret-scan-') as directory:
            snapshot = Path(directory)
            names = subprocess.check_output(
                ['git', 'ls-files', '-z', '--cached', '--others', '--exclude-standard'], cwd=ROOT)
            try:
                count = copy_snapshot(ROOT, snapshot, names.decode('utf-8').split('\0'))
                (evidence / 'snapshot.json').write_text(json.dumps({'files': count}) + '\n')
                results['secrets'] = run_step('secrets', secret_command(snapshot, evidence), snapshot, evidence)
            except (OSError, ValueError) as error:
                results['secrets'] = 127
                (evidence / 'snapshot-error.txt').write_text(f'{type(error).__name__}: {error}\n')
        results['coverage'] = run_step('coverage', [
            'cargo', 'llvm-cov', '--workspace', '--all-features', '--locked', '--json',
            '--output-path', str(evidence / 'coverage-report.json')], ROOT, evidence)
        results['mutation'] = run_step('mutation', [
            'cargo', 'mutants', '--in-place', '--workspace', '--test-workspace', 'true',
            '--all-features', '--timeout', '90', '--cargo-arg=--locked',
            '--output', str(evidence / 'mutation')], ROOT, evidence)
        try:
            results['reports'] = 0 if validate_reports(evidence) else 1
        except (OSError, ValueError, KeyError, TypeError) as error:
            results['reports'] = 1
            (evidence / 'report-error.txt').write_text(f'{type(error).__name__}: {error}\n')
    after = source_hashes(ROOT)
    (evidence / 'source-after.json').write_text(json.dumps(after, indent=2) + '\n')
    passed = not any(results.values()) and before == after
    summary = {'passed': passed, 'checks': results, 'source_unchanged': before == after,
               'scope': '本機工程檢查；不代表三平台實機、桌面或企業發布驗收'}
    (evidence / 'summary.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(summary, ensure_ascii=False), flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    with (ROOT / '.git/agent-platform-check.lock').open('a+b') as lock:
        try:
            if platform.system() == 'Windows':
                import msvcrt
                lock.seek(0)
                msvcrt.locking(lock.fileno(), msvcrt.LK_NBLCK, 1)
            else:
                import fcntl
                fcntl.flock(lock.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError:
            raise SystemExit('完整檢查已在執行；請接續原程序，不要重開。')
        try:
            with claim_run(ROOT / '.runtime/checks/active.json'):
                result = main()
        except FileExistsError:
            raise SystemExit('上次完整檢查未正常結束；先核對程序、來源與原件，再解除 active.json，不可直接重開。')
        raise SystemExit(result)
