import os
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

import runtime_sweep as rs

DAY = rs.DAY
NOW = 10_000_000.0


def entry(name="x", age_days=10.0, **kwargs):
    return rs.Entry(f"/r/.runtime/{name}", name, NOW - age_days * DAY, 5, **kwargs)


def decide_one(e, in_use=()):
    [d] = rs.decide([e], set(in_use), NOW, 7 * DAY)
    return d.action, d.reason


class DecideTest(unittest.TestCase):
    def test_unused_old_entry_is_deleted(self):
        self.assertEqual(decide_one(entry(age_days=8)), ("delete", "unused"))

    def test_fresh_entry_is_kept(self):
        self.assertEqual(decide_one(entry(age_days=3)), ("keep", "fresh"))

    def test_entry_under_a_day_is_kept_even_if_merged(self):
        self.assertEqual(decide_one(entry(age_days=0.5, merged_commit=True)), ("keep", "recent"))

    def test_merged_commit_is_deleted_after_a_day(self):
        self.assertEqual(decide_one(entry(age_days=2, merged_commit=True)), ("delete", "merged_commit"))

    def test_old_date_in_name_is_deleted_before_idle_limit(self):
        self.assertEqual(decide_one(entry(age_days=2, dated=NOW - 8 * DAY)), ("delete", "old_date"))

    def test_recent_date_in_name_is_kept(self):
        self.assertEqual(decide_one(entry(age_days=2, dated=NOW - 3 * DAY)), ("keep", "fresh"))

    def test_open_file_inside_keeps_old_merged_entry(self):
        e = entry(age_days=30, merged_commit=True)
        self.assertEqual(decide_one(e, in_use=[e.path + "/a.log"]), ("keep", "in_use"))

    def test_sibling_prefix_is_not_in_use(self):
        e = entry("ab", age_days=30)
        self.assertEqual(decide_one(e, in_use=["/r/.runtime/abc/file"]), ("delete", "unused"))

    def test_listed_entry_is_kept(self):
        self.assertEqual(decide_one(entry(age_days=30, listed=True)), ("keep", "listed"))

    def test_pinned_evidence_is_kept(self):
        self.assertEqual(decide_one(entry(age_days=30, pinned=True)), ("keep", "latest_evidence"))


class NameTest(unittest.TestCase):
    def test_date_in_name(self):
        self.assertGreater(rs.date_in_name("gate-20261118.log"), 0)
        self.assertEqual(rs.date_in_name("gate-20261318.log"), 0.0)
        self.assertEqual(rs.date_in_name("review-8468b77"), 0.0)


class GitTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.repo = Path(self.tmp.name)
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.email", "t@example.com")
        self.git("config", "user.name", "t")
        (self.repo / ".gitignore").write_text("/.runtime/\n")
        self.git("add", ".gitignore")
        self.git("commit", "-q", "-m", "a")
        self.merged = self.git("rev-parse", "--short=7", "HEAD").strip()
        self.git("update-ref", "refs/remotes/origin/main", "HEAD")
        self.git("checkout", "-q", "-b", "side")
        (self.repo / "f").write_text("x")
        self.git("add", "f")
        self.git("commit", "-q", "-m", "b")
        self.unmerged = self.git("rev-parse", "--short=7", "HEAD").strip()
        self.runtime = self.repo / ".runtime"
        self.runtime.mkdir()

    def tearDown(self):
        self.tmp.cleanup()

    def git(self, *args):
        return subprocess.run(["git", "-C", str(self.repo), *args], capture_output=True, text=True, check=True).stdout

    def make(self, name, age_days):
        path = self.runtime / name
        path.mkdir(parents=True, exist_ok=True)
        (path / "a.log").write_text("x")
        old = time.time() - age_days * DAY
        os.utime(path / "a.log", (old, old))
        os.utime(path, (old, old))
        return path

    def test_merged_commit_name_is_detected_and_unmerged_is_not(self):
        refs = rs.integration_refs(self.repo)
        self.assertTrue(rs.is_merged_commit(self.repo, f"review-{self.merged}", refs))
        self.assertFalse(rs.is_merged_commit(self.repo, f"review-{self.unmerged}", refs))
        self.assertFalse(rs.is_merged_commit(self.repo, "target-rev-20261119", refs))

    def test_sweep_dry_run_deletes_nothing(self):
        stale = self.make(f"review-{self.merged}", 2)
        summary = rs.sweep(self.repo, False, 7, 200)
        self.assertEqual([d["reason"] for d in summary["deleted"]], ["merged_commit"])
        self.assertTrue(stale.exists())

    def test_sweep_apply_removes_stale_and_keeps_the_rest(self):
        stale = self.make(f"review-{self.merged}", 2)
        idle = self.make("idle", 30)
        unmerged = self.make(f"review-{self.unmerged}", 2)
        fresh = self.make("fresh", 1.5)
        listed = self.make("pinned-by-hand", 30)
        (self.runtime / ".keep").write_text("# note\npinned-by-hand\n")
        summary = rs.sweep(self.repo, True, 7, 200)
        self.assertEqual(summary["failed"], [])
        self.assertFalse(stale.exists())
        self.assertFalse(idle.exists())
        self.assertTrue(unmerged.exists())
        self.assertTrue(fresh.exists())
        self.assertTrue(listed.exists())

    def test_checks_keeps_newest_three_rounds(self):
        for i in range(5):
            self.make(f"checks/{i}-round", 30 - i)
        rs.sweep(self.repo, True, 7, 200)
        left = sorted(p.name for p in (self.runtime / "checks").iterdir())
        self.assertEqual(left, ["2-round", "3-round", "4-round"])

    def test_max_deletes_bounds_the_run(self):
        for i in range(5):
            self.make(f"idle{i}", 30)
        summary = rs.sweep(self.repo, True, 7, 2)
        self.assertEqual(len(summary["deleted"]), 2)
        self.assertEqual(len(list(self.runtime.iterdir())), 3)

    def test_runtime_not_ignored_is_refused(self):
        (self.repo / ".gitignore").write_text("")
        self.make("idle", 30)
        summary = rs.sweep(self.repo, True, 7, 200)
        self.assertEqual(summary["error"], "runtime_missing_or_not_ignored")
        self.assertTrue((self.runtime / "idle").exists())

    def test_symlinked_runtime_is_refused(self):
        other = tempfile.mkdtemp()
        os.rmdir(self.runtime)
        os.symlink(other, self.runtime)
        summary = rs.sweep(self.repo, True, 7, 200)
        self.assertEqual(summary["error"], "runtime_missing_or_not_ignored")

    def test_open_file_keeps_old_entry(self):
        busy = self.make("busy", 30)
        with open(busy / "a.log") as held:
            self.assertIn(str(busy / "a.log"), rs.scan_in_use(self.runtime))
            summary = rs.sweep(self.repo, True, 7, 200)
            self.assertTrue(held.readable())
        self.assertTrue(busy.exists())
        self.assertEqual(summary["kept"].get("in_use"), 1)


if __name__ == "__main__":
    unittest.main()
