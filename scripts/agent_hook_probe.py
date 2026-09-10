"""S7.02B diagnostic hook. No sandbox, worker launcher, or qualification grant."""
import json
import sys

MARKER = "S7_02B_PROBE_7F4C8D"
DENIAL = "S7_02B_HOOK_OBSERVED: synthetic command denied before execution"


def main():
    try:
        payload = json.load(sys.stdin)
        if not isinstance(payload, dict):
            raise ValueError("expected object")
        if payload.get("hook_event_name") != "PreToolUse":
            raise ValueError("unexpected event")
        if payload.get("tool_name") not in ("Bash", "exec_command", "shell", "shell_command"):
            raise ValueError("unexpected shell tool")
        tool_input = payload.get("tool_input")
        if not isinstance(tool_input, dict):
            raise ValueError("expected tool_input object")
        command = tool_input.get("command", tool_input.get("cmd"))
        if not isinstance(command, str):
            raise ValueError("expected string command")
    except (ValueError, TypeError):
        print("S7_02B_INVALID_INPUT: diagnostic hook input was not understood", file=sys.stderr)
        return 2

    if MARKER in command:
        print(DENIAL, file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
