"""Direct hook contract tests; these do not prove client hook loading."""
import json
from pathlib import Path
import subprocess
import sys
import unittest

SCRIPT = Path(__file__).with_name("agent_hook_probe.py")
MARKER = "S7_02B_PROBE_7F4C8D"


class HookProbeTests(unittest.TestCase):
    def invoke(self, raw):
        return subprocess.run(
            [sys.executable, str(SCRIPT)], input=raw,
            capture_output=True, text=True, timeout=5,
        )

    def test_marker_denied_without_execution(self):
        for tool, key in (("Bash", "command"), ("exec_command", "cmd")):
            with self.subTest(tool=tool):
                result = self.invoke(json.dumps({
                    "hook_event_name": "PreToolUse", "tool_name": tool,
                    "tool_input": {key: "printf " + MARKER},
                }))
                self.assertEqual(result.returncode, 2)
                self.assertIn("S7_02B_HOOK_OBSERVED:", result.stderr)
                self.assertEqual(result.stdout, "")

    def test_normal_command_allowed_without_execution(self):
        result = self.invoke(json.dumps({
            "hook_event_name": "PreToolUse", "tool_name": "Bash",
            "tool_input": {"command": "exit 47"},
        }))
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout + result.stderr, "")

    def test_invalid_input_is_not_success_evidence(self):
        for raw in ("{", "[]", "null", "{}", json.dumps({
            "hook_event_name": "PreToolUse", "tool_name": "Bash",
            "tool_input": {"command": []},
        })):
            with self.subTest(raw=raw):
                result = self.invoke(raw)
                self.assertEqual(result.returncode, 2)
                self.assertIn("S7_02B_INVALID_INPUT:", result.stderr)
                self.assertNotIn("S7_02B_HOOK_OBSERVED:", result.stderr)


if __name__ == "__main__":
    unittest.main()
