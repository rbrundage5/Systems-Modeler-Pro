"""Fail CI when frontend authority or controller debt grows.

This is a ratchet, not a claim that the current frontend is complete.  Existing
debt is recorded so recovery changes can reduce it incrementally without a
large, risky rewrite.  A feature PR may not raise any limit.
"""

from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FRONTEND = ROOT / "apps" / "desktop" / "frontend"


def source_lines(paths: list[Path]) -> int:
    return sum(len(path.read_text(encoding="utf-8").splitlines()) for path in paths)


javascript_files = sorted(FRONTEND.glob("*.js"))
rust_files = sorted((ROOT / "crates").rglob("*.rs")) + sorted(
    (ROOT / "apps" / "desktop" / "src-tauri").rglob("*.rs")
)
javascript = "\n".join(path.read_text(encoding="utf-8") for path in javascript_files)

metrics = {
    "frontend JavaScript files": len(javascript_files),
    "direct frontend state assignments": len(
        re.findall(r"\bstate\.[A-Za-z_$][\w$]*\s*=", javascript)
    ),
    "renderer wrapper assignments": len(
        re.findall(
            r"\b(?:render|renderCanvas|renderProperties)\s*=\s*function", javascript
        )
    ),
    "blocking browser dialogs": len(re.findall(r"\b(?:prompt|alert)\s*\(", javascript)),
    "frontend keydown controllers": len(
        re.findall(r"addEventListener\(\s*['\"]keydown", javascript)
    ),
}

# Controller-debt ceilings retain the PR15 behavior, with one bounded renderer
# adapter per newly registered PR24/PR25 diagram family. Parametric semantics,
# validation, evaluation, routing, layout, history, and persistence remain Rust-owned.
maximums = {
    "frontend JavaScript files": 41,
    "direct frontend state assignments": 333,
    "renderer wrapper assignments": 35,
    "blocking browser dialogs": 73,
    "frontend keydown controllers": 7,
}

failures = []
for name, value in metrics.items():
    maximum = maximums[name]
    if value > maximum:
        failures.append(f"{name} grew from the PR15 ceiling {maximum} to {value}")

rust_lines = source_lines(rust_files)
javascript_lines = source_lines(javascript_files)
ratio = rust_lines / max(1, javascript_lines)
if ratio < 1.9:
    failures.append(
        f"Rust/frontend source ratio fell below 1.90 ({rust_lines}/{javascript_lines} = {ratio:.2f})"
    )

forbidden_authority = {
    "frontend semantic repositories": r"\bnew\s+(?:Activity|Behavior|Model)Repository\b",
    "frontend semantic collection mutation": (
        r"\b(?:activity|behavior|project)\.(?:nodes|edges|elements|relationships|"
        r"partitions|structured_nodes)\.(?:push|splice)\s*\("
    ),
}
for name, pattern in forbidden_authority.items():
    if re.search(pattern, javascript):
        failures.append(f"forbidden {name} detected")

if failures:
    raise SystemExit("Rust-authority gate failed:\n- " + "\n- ".join(failures))

print(
    "Rust-authority gate passed: "
    f"{rust_lines} Rust lines / {javascript_lines} frontend JavaScript lines "
    f"({ratio:.2f}:1); authority-debt ratchets did not increase"
)
