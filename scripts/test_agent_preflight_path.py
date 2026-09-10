"""Execute the configured command in synthetic Git roots; not native hook qualification."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]


class PreflightPathTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="smp-preflight-path-")
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        # Keep fixture Git discovery and configuration independent of the caller.
        self.env = {key: value for key, value in os.environ.items()
                    if not key.startswith("GIT_")}
        self.env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull,
                        GIT_CEILING_DIRECTORIES=str(self.base))
        config = tomllib.loads((ROOT / ".codex/config.toml").read_text())
        self.assertIs(config["agents"]["enabled"], False)
        hook = config["hooks"]["PreToolUse"][0]
        self.assertEqual(hook["matcher"], "^(spawn_agent|Agent)$")
        self.command = hook["hooks"][0]["command"]

    def checkout(self, path, with_guard=True):
        repo = self.base / path
        repo.mkdir(parents=True)
        subprocess.run(["git", "init", "--quiet", str(repo)], env=self.env,
                       check=True, capture_output=True, timeout=10)
        if with_guard:
            (repo / "scripts").mkdir()
            shutil.copyfile(ROOT / "scripts/agent_preflight.py",
                            repo / "scripts/agent_preflight.py")
        return repo

    def run_hook_command(self, cwd):
        return subprocess.run(["sh", "-c", self.command], cwd=cwd, env=self.env,
                              capture_output=True, text=True, timeout=10)

    def assert_refused(self, cwd):
        result = self.run_hook_command(cwd)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertIn("BLOCKED: repository-only worker isolation has not been qualified.",
                      result.stderr)
        self.assertIn("No agent was launched.", result.stderr)

    def test_both_hosted_root_layouts_and_nested_working_directory(self):
        for layout in ("workspace/Systems-Modeler-Pro",
                       "workspaces/Systems-Modeler-Pro"):
            with self.subTest(layout=layout):
                repo = self.checkout(layout)
                nested = repo / "apps/desktop"
                nested.mkdir(parents=True)
                self.assert_refused(repo)
                self.assert_refused(nested)

    def test_checkout_path_with_spaces(self):
        self.assert_refused(self.checkout("hosted/Systems Modeler Pro"))

    def test_no_git_repository_does_not_start_python(self):
        outside = self.base / "outside"
        outside.mkdir()
        # A fake python executable records any attempt after failed Git discovery.
        bin_dir = self.base / "bin"
        bin_dir.mkdir()
        marker = self.base / "python-started"
        fake = bin_dir / "python3"
        fake.write_text('#!/bin/sh\nprintf started > "$SMP_TEST_MARKER"\n')
        fake.chmod(0o755)
        self.env["PATH"] = str(bin_dir) + os.pathsep + self.env["PATH"]
        self.env["SMP_TEST_MARKER"] = str(marker)
        result = self.run_hook_command(outside)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(marker.exists())
        self.assertEqual(result.stdout, "")

    def test_missing_guard_fails_without_claiming_refusal_executed(self):
        repo = self.checkout("missing-guard", with_guard=False)
        result = self.run_hook_command(repo)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stdout, "")
        self.assertNotIn("BLOCKED:", result.stderr)


if __name__ == "__main__":
    unittest.main()
