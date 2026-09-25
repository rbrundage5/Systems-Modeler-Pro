"""Real local Git fixtures; no network, credentials or user configuration."""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from ensure_repository_origin import MARKERS, ORIGIN, SetupError, ensure_origin


class RepositoryOriginTests(unittest.TestCase):
    def setUp(self):
        self.environment = patch.dict(os.environ, {
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GIT_TERMINAL_PROMPT": "0",
        })
        self.environment.start()
        self.addCleanup(self.environment.stop)
        temporary = tempfile.TemporaryDirectory(
            prefix="origin-test-", dir=Path(__file__).resolve().parents[1]
        )
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.git("init", "--quiet")
        for name in MARKERS:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("fixture\n")
        self.git("add", "--", *MARKERS)

    def git(self, *args):
        return subprocess.run(
            ["git", "-C", str(self.root), *args],
            capture_output=True, text=True, check=True,
        ).stdout.strip()

    def config_bytes(self):
        return (self.root / ".git/config").read_bytes()

    def assert_rejected_without_change(self):
        before = self.config_bytes()
        with self.assertRaises(SetupError) as error:
            ensure_origin(self.root)
        self.assertEqual(before, self.config_bytes())
        self.assertNotIn("secret", str(error.exception))

    def test_missing_origin_added_once_without_claiming_authentication(self):
        result = ensure_origin(self.root)
        self.assertEqual(result["origin"], "ADDED")
        self.assertEqual(result["publication_authentication"], "UNVERIFIED")
        self.assertEqual(result["worker_qualification"], "NOT_EVALUATED")
        self.assertEqual(self.git("remote", "get-url", "origin"), ORIGIN)
        before = self.config_bytes()
        self.assertEqual(ensure_origin(self.root)["origin"], "PRESENT")
        self.assertEqual(before, self.config_bytes())

    def test_valid_origin_and_existing_options_are_preserved(self):
        self.git("remote", "add", "origin", ORIGIN.removesuffix(".git"))
        self.git("config", "remote.origin.tagOpt", "--no-tags")
        before = self.config_bytes()
        ensure_origin(self.root)
        self.assertEqual(before, self.config_bytes())

    def test_foreign_fetch_url_rejected_without_exposing_credentials(self):
        self.git("remote", "add", "origin", "https://secret@example.invalid/other.git")
        self.assert_rejected_without_change()

    def test_foreign_push_url_rejected(self):
        self.git("remote", "add", "origin", ORIGIN)
        self.git("remote", "set-url", "--push", "origin", "https://example.invalid/other.git")
        self.assert_rejected_without_change()

    def test_extra_fetch_url_rejected(self):
        self.git("remote", "add", "origin", ORIGIN)
        self.git("config", "--add", "remote.origin.url", "https://example.invalid/other.git")
        self.assert_rejected_without_change()

    def test_rewritten_fetch_and_push_urls_reject_before_origin_is_added(self):
        for setting in ("insteadOf", "pushInsteadOf"):
            with self.subTest(setting=setting):
                key = f"url.https://example.invalid/.{setting}"
                self.git("config", key, "https://github.com/")
                before = self.config_bytes()
                with self.assertRaisesRegex(SetupError, "Matching URL rewrite"):
                    ensure_origin(self.root)
                self.assertEqual(before, self.config_bytes())
                self.assertNotIn("origin", self.git("remote").splitlines())
                self.git("config", "--unset", key)

    def test_untracked_marker_rejected(self):
        self.git("rm", "--cached", "--quiet", "Cargo.lock")
        self.assert_rejected_without_change()
        self.assertEqual(self.git("remote"), "")

    def test_nested_path_rejected_without_changing_repository(self):
        before = self.config_bytes()
        with self.assertRaises(SetupError):
            ensure_origin(self.root / "crates")
        self.assertEqual(before, self.config_bytes())

    def test_non_repository_rejected_without_creating_git_metadata(self):
        (self.root / ".git").rename(self.root / "saved-git-metadata")
        with self.assertRaises(SetupError):
            ensure_origin(self.root)
        self.assertFalse((self.root / ".git").exists())


if __name__ == "__main__":
    unittest.main()
