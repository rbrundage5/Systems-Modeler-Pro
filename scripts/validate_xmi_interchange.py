from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MAIN = (ROOT / "apps/desktop/src-tauri/src/main.rs").read_text(encoding="utf-8")
RUNTIME = (ROOT / "apps/desktop/src-tauri/src/workspace/xmi_runtime.rs").read_text(encoding="utf-8")
ADAPTER = (ROOT / "apps/desktop/src-tauri/src/workspace/xmi_interchange.rs").read_text(encoding="utf-8")
UI = (ROOT / "apps/desktop/frontend/xmi-ui.js").read_text(encoding="utf-8")

for command in (
    "preview_xmi_import",
    "apply_xmi_import",
    "export_xmi",
    "stage_xmi_upload",
    "discard_staged_xmi",
):
    assert command in MAIN, f"missing registered XMI command: {command}"
    assert command in UI, f"missing XMI UI invocation: {command}"

for contract in (
    "XmiSemanticDocument",
    "XmiSynchronizationPolicy",
    "AuthoritativeXmiScope",
    "XmiAction",
    "REFERENCE_PROTECTED_REMOVE",
):
    assert contract in RUNTIME or contract in ADAPTER, f"missing XMI contract: {contract}"

assert "roxmltree" in ADAPTER, "XMI must use the namespace-aware Rust parser"
assert "xmi:Extension" in ADAPTER, "loss-minimized extension preservation is required"
assert "diagram" not in " ".join(line.lower() for line in RUNTIME.splitlines() if "serialize_xmi" in line), "XMI export must remain semantic-only"

for fixture in ("external-uml.xmi", "external-sysml-profile.xmi"):
    assert (ROOT / "examples/xmi" / fixture).is_file(), f"missing external fixture: {fixture}"

print("XMI interchange integration contract validated")
