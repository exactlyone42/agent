import json
from pathlib import Path
import secrets
import string
import sys
import tempfile
import unittest
import uuid

from check_all import claim_run, copy_snapshot, run_step, secret_command, validate_reports


class CheckRunnerContract(unittest.TestCase):
    def test_interrupted_run_cannot_become_a_new_baseline(self):
        with tempfile.TemporaryDirectory() as directory:
            marker = Path(directory) / 'active.json'
            with claim_run(marker):
                self.assertTrue(marker.exists())
            self.assertFalse(marker.exists())
            with self.assertRaises(RuntimeError):
                with claim_run(marker):
                    raise RuntimeError('synthetic interruption')
            self.assertTrue(marker.exists())
            with self.assertRaises(FileExistsError):
                with claim_run(marker):
                    self.fail('interrupted run was ignored')

    def test_cache_exception_does_not_hide_credentials(self):
        value = str(uuid.uuid4())
        token = 'ghp_' + ''.join(secrets.choice(string.ascii_letters + string.digits) for _ in range(36))
        cases = [('cache', {'prompt_cache_key': value}, True, 0),
                 ('other-field', {'api_key': value}, True, 1),
                 ('outside-path', {'prompt_cache_key': value}, False, 1),
                 ('token-in-cache', {'prompt_cache_key': token}, True, 1),
                 ('same-file-token', {'prompt_cache_key': value, 'api_key': token}, True, 1)]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name, payload, matching, expected in cases:
                with self.subTest(name=name):
                    snapshot = root / name / 'snapshot'
                    evidence = root / name / 'evidence'
                    evidence.mkdir(parents=True)
                    file = snapshot / ('docs/evidence/P01/sample/requests.json' if matching else 'other/requests.json')
                    file.parent.mkdir(parents=True)
                    file.write_text(json.dumps(payload, indent=2))
                    self.assertEqual(run_step('secrets', secret_command(snapshot, evidence), snapshot, evidence), expected)
                    self.assertNotIn(token, (evidence / 'secrets.txt').read_text())
                    self.assertNotIn(token, (evidence / 'secrets-report.json').read_text())

    def test_empty_or_incomplete_reports_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            mutants = root / 'mutation/mutants.out'
            mutants.mkdir(parents=True)
            coverage = {'data': [{'totals': {'lines': {'count': 2, 'covered': 1}}}]}
            mutation = {'total_mutants': 2, 'caught': 1, 'unviable': 1, 'missed': 0, 'timeout': 0}
            (root / 'coverage-report.json').write_text(json.dumps(coverage))
            (mutants / 'outcomes.json').write_text(json.dumps(mutation))
            self.assertTrue(validate_reports(root))
            self.assertEqual(run_step('coverage', [sys.executable, '-c', 'pass'], root, root), 0)
            self.assertTrue(validate_reports(root))
            self.assertEqual(json.loads((root / 'coverage-report.json').read_text()), coverage)
            for field, value in [('total_mutants', 0), ('caught', 0), ('missed', 1), ('timeout', 1)]:
                (mutants / 'outcomes.json').write_text(json.dumps({**mutation, field: value}))
                self.assertFalse(validate_reports(root))
            (mutants / 'outcomes.json').write_text(json.dumps(mutation))
            coverage['data'][0]['totals']['lines']['covered'] = 0
            (root / 'coverage-report.json').write_text(json.dumps(coverage))
            self.assertFalse(validate_reports(root))

    def test_snapshot_scans_internal_links_and_rejects_external_links(self):
        with tempfile.TemporaryDirectory() as directory:
            base = Path(directory)
            root = base / 'repo'
            root.mkdir()
            snapshot = base / 'snapshot'
            snapshot.mkdir()
            with self.assertRaises(ValueError):
                copy_snapshot(root, snapshot, ['missing.txt'])
            (root / 'source.txt').write_text('scan this content')
            (root / 'internal.txt').symlink_to('source.txt')
            copy_snapshot(root, snapshot, ['internal.txt', 'deleted.txt'])
            self.assertEqual((snapshot / 'internal.txt').read_text(), 'scan this content')
            (base / 'external.txt').write_text('private outside repo')
            (root / 'external.txt').symlink_to(base / 'external.txt')
            with self.assertRaises(ValueError):
                copy_snapshot(root, snapshot, ['external.txt'])
            self.assertFalse((snapshot / 'external.txt').exists())

    def test_step_records_failure_and_success(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for code in [0, 7]:
                name = f'exit-{code}'
                result = run_step(name, [sys.executable, '-c', f'raise SystemExit({code})'], root, root)
                self.assertEqual(result, code)
                self.assertEqual(json.loads((root / f'{name}.json').read_text())['exit_code'], code)
            self.assertEqual(run_step('missing', [str(root / 'missing-tool')], root, root), 127)

    def test_secret_scanner_detects_canary_and_redacts_it(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            snapshot = root / 'snapshot'
            snapshot.mkdir()
            for present in [False, True]:
                evidence = root / str(present)
                evidence.mkdir()
                canary = 'ghp_' + ''.join(secrets.choice(string.ascii_letters + string.digits)
                                         for _ in range(36))
                (snapshot / 'sample.txt').write_text('token = ' + canary if present else 'ordinary document')
                result = run_step('secrets', secret_command(snapshot, evidence), snapshot, evidence)
                self.assertEqual(result, 1 if present else 0)
                self.assertNotIn(canary, (evidence / 'secrets.txt').read_text())
                report = (evidence / 'secrets-report.json').read_text()
                self.assertNotIn(canary, report)
                self.assertEqual(bool(json.loads(report)), present)


if __name__ == '__main__':
    unittest.main()
