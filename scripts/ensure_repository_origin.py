"""Configure only the approved origin in an authorized hosted checkout.

Local Git metadata only: no networking, credential setup or worker enablement.
"""

import json
from pathlib import Path
import subprocess
import sys


REPOSITORY = "rbrundage5/Systems-Modeler-Pro"
ORIGIN = f"https://github.com/{REPOSITORY}.git"
ALLOWED_URLS = {ORIGIN, ORIGIN.removesuffix(".git")}
MARKERS = (
    "Cargo.toml",
    "Cargo.lock",
    "crates/model-core/Cargo.toml",
    "apps/desktop/src-tauri/Cargo.toml",
)


class SetupError(RuntimeError):
    pass


def git(root, *args, allow_missing=False):
    try:
        result = subprocess.run(
            ["git", "-C", str(root), *args],
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise SetupError("Local Git check could not complete.") from error
    if allow_missing and result.returncode == 1:
        return ""
    if result.returncode:
        # Git errors can contain credential-bearing URLs. Do not echo them.
        raise SetupError("Local Git check failed; no publication was attempted.")
    return result.stdout.strip()


def validate_urls(root):
    for options in [("--all",), ("--push", "--all")]:
        urls = git(root, "remote", "get-url", *options, "origin")
        if not urls or any(url not in ALLOWED_URLS for url in urls.splitlines()):
            raise SetupError("Unexpected origin fetch/push destination; configuration unchanged.")


def ensure_origin(root):
    root = Path(root).resolve()
    if Path(git(root, "rev-parse", "--show-toplevel")).resolve() != root:
        raise SetupError("Run only from the authorized repository root.")
    git(root, "ls-files", "--error-unmatch", "--", *MARKERS)
    if "origin" in git(root, "remote").splitlines():
        validate_urls(root)
        origin_state = "PRESENT"
    else:
        # A command-line remote definition is not a configured remote for
        # get-url. Conservatively refuse matching rewrites before any mutation.
        rewrites = git(
            root, "config", "--null", "--get-regexp",
            r"^url\..*\.(insteadof|pushinsteadof)$", allow_missing=True,
        )
        for record in rewrites.split("\0"):
            if not record:
                continue
            _, separator, prefix = record.partition("\n")
            if not separator or ORIGIN.startswith(prefix):
                raise SetupError("Matching URL rewrite prevents origin setup; configuration unchanged.")
        git(root, "remote", "add", "origin", ORIGIN)
        origin_state = "ADDED"
    return {
        "repository": REPOSITORY,
        "origin": origin_state,
        "publication_authentication": "UNVERIFIED",
        "worker_qualification": "NOT_EVALUATED",
    }


def main():
    try:
        result = ensure_origin(Path(__file__).resolve().parents[1])
    except SetupError as error:
        print(f"BLOCKED: {error}", file=sys.stderr)
        return 2
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
