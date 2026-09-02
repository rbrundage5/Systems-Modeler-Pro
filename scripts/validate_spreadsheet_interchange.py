from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
RUST = (ROOT / "apps/desktop/src-tauri/src/workspace/spreadsheet_interchange.rs").read_text()
MAIN = (ROOT / "apps/desktop/src-tauri/src/main.rs").read_text()
SHELL = (ROOT / "apps/desktop/frontend/ui-shell.js").read_text()
UI = SHELL


COMMANDS = {
    "stage_spreadsheet_upload",
    "discard_staged_spreadsheet",
    "preview_spreadsheet_workbook_import",
    "apply_spreadsheet_workbook_import",
    "export_spreadsheet_workbook",
}

for command in COMMANDS:
    assert f"fn {command}" in RUST, f"missing Rust spreadsheet command: {command}"
    assert command in MAIN, f"spreadsheet command is not registered: {command}"

for contract in (
    "SpreadsheetExportProfile",
    "CatiaSemantic",
    "SystemsModeler",
    "SpreadsheetSynchronizationPolicy",
    "AuthoritativeMappedScope",
    "SpreadsheetInterchangeAction::Remove",
    "SystemsModelerState",
    "DiagramPresentations",
    "DiagramRelationships",
):
    assert contract in RUST, f"missing spreadsheet interchange contract: {contract}"

for workflow_text in (
    "Import Spreadsheet",
    "Mapped XLSX / CSV",
    "Systems-Modeler workbook",
    "Spreadsheet import preview",
    "CREATE",
    "UPDATE",
    "NO_CHANGE",
    "REMOVE",
    "BLOCKED",
):
    assert workflow_text in UI, f"desktop workflow is missing: {workflow_text}"

assert "data-action=\"import-spreadsheet\"" in SHELL
assert "data-action=\"export-spreadsheet\"" in SHELL
assert "smpSpreadsheetInterchange" in SHELL

print("Spreadsheet interchange desktop integration contract is complete.")
