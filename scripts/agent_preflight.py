"""Fail-closed startup placeholder. No worker execution is authorized by this file.

Run in hosted repository infrastructure only. Replace only through a separately
reviewed supervisor integration after the environment gate is implemented.
This is not a filesystem/network sandbox and is not automatically a Git hook.
"""
import sys

def main():
    print(
        "BLOCKED: repository-only worker isolation has not been qualified. "
        "See docs/AGENT_ENVIRONMENT_GATE.md. No agent was launched.",
        file=sys.stderr,
    )
    return 2

if __name__ == "__main__":
    raise SystemExit(main())
