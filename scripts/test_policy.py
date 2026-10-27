import os
import re
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
LINTS = ['unwrap_used', 'expect_used', 'panic', 'todo', 'unimplemented',
         'unreachable', 'disallowed_macros', 'mod_module_files']


class PolicyContract(unittest.TestCase):
    def test_gate_enables_tested_lints(self):
        script = (ROOT / 'scripts/check.sh').read_text()
        self.assertEqual(set(re.findall(r'-D clippy::([a-z_]+)', script)), set(LINTS))

    def test_compiler_rejects_each_banned_construct(self):
        driver = subprocess.check_output(['rustup', 'which', 'clippy-driver'], cwd=ROOT, text=True).strip()
        fixtures = {
            'valid': 'pub fn value(x: Option<u8>) -> Result<u8, String> { x.ok_or("missing".into()) }',
            'unwrap_used': 'pub fn value(x: Option<u8>) -> u8 { x.unwrap() }',
            'expect_used': 'pub fn value(x: Option<u8>) -> u8 { x.expect("missing") }',
            'panic': 'pub fn value() { panic!("failed") }',
            'todo': 'pub fn value() { todo!() }',
            'unimplemented': 'pub fn value() { unimplemented!() }',
            'unreachable': 'pub fn value() { unreachable!() }',
            'assert': 'pub fn value(x: bool) { assert!(x); }',
            'assert_eq': 'pub fn value(x: u8, y: u8) { assert_eq!(x, y); }',
            'assert_ne': 'pub fn value(x: u8, y: u8) { assert_ne!(x, y); }',
            'debug_assert': 'pub fn value(x: bool) { debug_assert!(x); }',
            'debug_assert_eq': 'pub fn value(x: u8, y: u8) { debug_assert_eq!(x, y); }',
            'debug_assert_ne': 'pub fn value(x: u8, y: u8) { debug_assert_ne!(x, y); }',
            'mod_module_files': 'pub mod nested;',
        }
        with tempfile.TemporaryDirectory(prefix='agent-policy-') as directory:
            workspace = Path(directory)
            nested = workspace / 'nested'
            nested.mkdir()
            (nested / 'mod.rs').write_text('pub fn value() -> u8 { 1 }')
            for name, content in fixtures.items():
                with self.subTest(name=name):
                    source = workspace / f'{name}.rs'
                    source.write_text(content)
                    command = [driver, '--crate-type=lib', '--edition=2024', '--emit=metadata',
                               str(source), '-o', str(workspace / f'{name}.rmeta'), '-Dwarnings']
                    command += [f'-Dclippy::{lint}' for lint in LINTS]
                    result = subprocess.run(command, capture_output=True, text=True, encoding='utf-8',
                                            timeout=20, cwd=workspace, env={**os.environ,
                                            'CLIPPY_CONF_DIR': str(ROOT / 'scripts/policy')})
                    if name == 'valid':
                        self.assertEqual(result.returncode, 0, result.stderr)
                    else:
                        self.assertNotEqual(result.returncode, 0, name)
                        lint = 'disallowed_macros' if 'assert' in name else name
                        self.assertIn(f"clippy::{lint.replace('_', '-')}`", result.stderr)


if __name__ == '__main__':
    unittest.main()
