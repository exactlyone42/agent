import argparse
import datetime
import json
import os
import re
import shutil
import stat
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path

DAY = 86400
LOG = Path.home() / ".local/state/runtime-sweep/log.jsonl"
KEEP_FILE = ".keep"
EVIDENCE_DIR = "checks"
EVIDENCE_KEEP = 3
INTEGRATION_REFS = ("origin/develop", "origin/main", "origin/master")
COMMIT_TOKEN = re.compile(r"(?<![0-9A-Za-z])[0-9a-f]{7,40}(?![0-9A-Za-z])")
DATE_TOKEN = re.compile(r"(?<!\d)(20\d{2})[-_]?(\d{2})[-_]?(\d{2})(?!\d)")
WALK_LIMIT = 200_000


@dataclass
class Entry:
    path: str
    name: str
    newest_mtime: float
    size: int
    merged_commit: bool = False
    dated: float = 0.0
    pinned: bool = False
    listed: bool = False


@dataclass
class Decision:
    path: str
    action: str
    reason: str
    size: int = 0


def decide(entries, in_use, now, min_age_s, recent_s=DAY):
    out = []
    for e in entries:
        prefix = e.path.rstrip("/") + "/"
        age = now - e.newest_mtime
        if any(p == e.path or p.startswith(prefix) for p in in_use):
            out.append(Decision(e.path, "keep", "in_use"))
        elif e.listed:
            out.append(Decision(e.path, "keep", "listed"))
        elif e.pinned:
            out.append(Decision(e.path, "keep", "latest_evidence"))
        elif age < recent_s:
            out.append(Decision(e.path, "keep", "recent"))
        elif e.merged_commit:
            out.append(Decision(e.path, "delete", "merged_commit", e.size))
        elif e.dated and now - e.dated >= min_age_s:
            out.append(Decision(e.path, "delete", "old_date", e.size))
        elif age >= min_age_s:
            out.append(Decision(e.path, "delete", "unused", e.size))
        else:
            out.append(Decision(e.path, "keep", "fresh"))
    return out


def scan_in_use(root):
    prefix = str(root).rstrip("/") + "/"
    paths = set()
    for pid in os.listdir("/proc"):
        if not pid.isdigit():
            continue
        base = f"/proc/{pid}"
        for link in ("cwd", "exe"):
            try:
                paths.add(os.readlink(f"{base}/{link}"))
            except OSError:
                pass
        try:
            for fd in os.listdir(f"{base}/fd"):
                try:
                    paths.add(os.readlink(f"{base}/fd/{fd}"))
                except OSError:
                    pass
        except OSError:
            pass
    return {p.removesuffix(" (deleted)") for p in paths if p.startswith(prefix)}


def measure(path):
    st = os.lstat(path)
    if stat.S_ISSOCK(st.st_mode):
        return float("inf"), 0
    newest, size, seen = st.st_mtime, st.st_blocks * 512, 0
    stack = [path] if os.path.isdir(path) and not os.path.islink(path) else []
    while stack:
        try:
            it = os.scandir(stack.pop())
        except OSError:
            continue
        with it:
            for ent in it:
                seen += 1
                if seen > WALK_LIMIT:
                    return float("inf"), size
                try:
                    s = ent.stat(follow_symlinks=False)
                except OSError:
                    continue
                if stat.S_ISSOCK(s.st_mode):
                    return float("inf"), size
                newest = max(newest, s.st_mtime)
                size += s.st_blocks * 512
                if ent.is_dir(follow_symlinks=False):
                    stack.append(ent.path)
    return newest, size


def git_ok(repo, *args):
    result = subprocess.run(["git", "-C", str(repo), *args], capture_output=True)
    return result.returncode == 0


def integration_refs(repo):
    return [r for r in INTEGRATION_REFS if git_ok(repo, "rev-parse", "--verify", "-q", r)]


def is_merged_commit(repo, name, refs):
    for token in COMMIT_TOKEN.findall(name):
        if DATE_TOKEN.fullmatch(token):
            continue
        if not git_ok(repo, "cat-file", "-e", f"{token}^{{commit}}"):
            continue
        if any(git_ok(repo, "merge-base", "--is-ancestor", token, ref) for ref in refs):
            return True
    return False


def date_in_name(name):
    m = DATE_TOKEN.search(name)
    if not m:
        return 0.0
    try:
        day = datetime.date(int(m[1]), int(m[2]), int(m[3]))
    except ValueError:
        return 0.0
    return datetime.datetime.combine(day, datetime.time.max).timestamp()


def listed_names(runtime):
    keep = runtime / KEEP_FILE
    if not keep.is_file():
        return set()
    lines = keep.read_text(encoding="utf-8").splitlines()
    return {line.strip().rstrip("/") for line in lines if line.strip() and not line.startswith("#")}


def make_entry(repo, refs, path, listed):
    newest, size = measure(path)
    name = os.path.basename(path)
    return Entry(path, name, newest, size, is_merged_commit(repo, name, refs), date_in_name(name), listed=name in listed)


def candidates(repo, runtime):
    refs = integration_refs(repo)
    listed = listed_names(runtime)
    out = []
    for ent in sorted(os.scandir(runtime), key=lambda e: e.name):
        if ent.name == KEEP_FILE:
            continue
        if ent.name != EVIDENCE_DIR or not ent.is_dir(follow_symlinks=False):
            out.append(make_entry(repo, refs, ent.path, listed))
            continue
        rounds = [make_entry(repo, refs, c.path, listed) for c in os.scandir(ent.path)]
        rounds.sort(key=lambda e: e.newest_mtime, reverse=True)
        for kept in rounds[:EVIDENCE_KEEP]:
            kept.pinned = True
        out.extend(rounds)
    return out


def remove(path):
    if os.path.isdir(path) and not os.path.islink(path):
        shutil.rmtree(path)
        return
    os.unlink(path)


def runtime_dir(repo):
    runtime = Path(repo) / ".runtime"
    if runtime.is_symlink() or not runtime.is_dir():
        return None
    if not git_ok(repo, "check-ignore", "-q", ".runtime/"):
        return None
    return runtime


def sweep(repo, apply, min_age_days, max_deletes):
    runtime = runtime_dir(repo)
    if runtime is None:
        return {"repo": str(repo), "error": "runtime_missing_or_not_ignored"}
    decisions = decide(candidates(repo, runtime), scan_in_use(runtime), time.time(), min_age_days * DAY)
    doomed = [d for d in decisions if d.action == "delete"][:max_deletes]
    deleted, failed, freed = [], [], 0
    for d in doomed:
        if apply:
            try:
                remove(d.path)
            except OSError as exc:
                failed.append({"path": d.path, "error": type(exc).__name__})
                continue
        deleted.append({"path": os.path.relpath(d.path, repo), "reason": d.reason, "bytes": d.size})
        freed += d.size
    kept = {}
    for d in decisions:
        if d.action == "keep":
            kept[d.reason] = kept.get(d.reason, 0) + 1
    return {
        "repo": str(repo),
        "apply": apply,
        "scanned": len(decisions),
        "deleted": deleted,
        "failed": failed,
        "freed_bytes": freed,
        "kept": kept,
    }


def write_log(summary):
    LOG.parent.mkdir(parents=True, exist_ok=True)
    line = {"at": time.strftime("%Y-%m-%dT%H:%M:%S%z"), **summary}
    with LOG.open("a", encoding="utf-8") as stream:
        stream.write(json.dumps(line, ensure_ascii=False) + "\n")


def main(argv=None):
    ap = argparse.ArgumentParser(description="Remove stale, unused entries under <repo>/.runtime; dry run unless --apply")
    ap.add_argument("--repo", action="append", required=True)
    ap.add_argument("--apply", action="store_true")
    ap.add_argument("--min-age-days", type=float, default=7.0)
    ap.add_argument("--max-deletes", type=int, default=200)
    args = ap.parse_args(argv)
    status = 0
    for repo in args.repo:
        summary = sweep(Path(repo).resolve(), args.apply, args.min_age_days, args.max_deletes)
        write_log({k: v for k, v in summary.items() if k != "deleted"} | {"deleted": len(summary.get("deleted", []))})
        print(json.dumps(summary, ensure_ascii=False, indent=2))
        if "error" in summary or summary["failed"]:
            status = 1
    return status


if __name__ == "__main__":
    sys.exit(main())
