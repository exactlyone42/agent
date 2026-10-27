#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

if [[ "$#" != 1 || ( "${1:-}" != "gate" && "${1:-}" != "all" ) ]]; then
  printf '%s\n' '用法：bash scripts/check.sh gate 或 all' >&2
  exit 2
fi

if [[ "$1" == "all" ]]; then
  exec python3 scripts/check_all.py
fi

cargo fmt --all --check
python3 scripts/test_policy.py
python3 scripts/test_check_all.py
cargo machete
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
CLIPPY_CONF_DIR="$PWD/scripts/policy" cargo clippy --workspace --lib --bins --all-features --locked -- -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::todo -D clippy::unimplemented -D clippy::unreachable -D clippy::disallowed_macros -D clippy::mod_module_files
cargo test --workspace --all-features --locked
