"""PR62 contract: model-script imports expose every native diagram family after Apply."""
from pathlib import Path

root = Path(__file__).resolve().parents[1]
frontend = root / "apps/desktop/frontend"

index = (frontend / "index.html").read_text(encoding="utf-8")
catalog = (frontend / "imported-diagram-catalog.js").read_text(encoding="utf-8")
model_script_ui = (frontend / "model-script-ui.js").read_text(encoding="utf-8")
model_script_rs = (root / "apps/desktop/src-tauri/src/workspace/model_script.rs").read_text(encoding="utf-8")

assert '<script src="imported-diagram-catalog.js"></script>' in index
assert index.index('activity-ui.js') < index.index('imported-diagram-catalog.js')
assert index.index('behavior-ui.js') < index.index('imported-diagram-catalog.js')
assert index.index('repository-tree-ui.js') < index.index('imported-diagram-catalog.js')

for source in (
    "state.snapshot?.diagrams",
    "state.snapshot?.ibd_diagrams",
    "state.behaviorSnapshot?.diagrams",
    "state.activitySnapshot?.diagrams",
):
    assert source in catalog, f"unified catalog missing {source}"

for family, tag in (
    ("package", "PKG"),
    ("requirement", "REQ"),
    ("use-case", "UC"),
    ("bdd", "BDD"),
    ("ibd", "IBD"),
    ("activity", "ACT"),
    ("state-machine", "STM"),
    ("sequence", "SEQ"),
    ("parametric", "PAR"),
):
    assert f"{family}: '{tag}'" in catalog or f"'{family}': '{tag}'" in catalog

for contract in (
    "new Map()",
    "smpSelectActivityDiagram",
    "smpSelectBehaviorDiagram",
    "selectDiagram(diagram.id)",
    "invoke('behavior_snapshot')",
    "invoke('activity_snapshot')",
    "refreshWithImportedDiagramConvergence",
    "renderUnifiedDiagramTabs",
    "renderUnifiedDiagramRepository",
):
    assert contract in catalog, f"missing imported-diagram convergence contract: {contract}"

assert "await refresh();" in model_script_ui, "model-script Apply must refresh the authoritative UI"

# The Rust importer must create specialized diagrams in their authoritative stores,
# not as fake BDD rows.
assert '"activity" => {' in model_script_rs
assert 'activity\n                .diagrams' in model_script_rs
assert '"state-machine" | "sequence" => {' in model_script_rs
assert 'workspace\n                .behavior_diagrams' in model_script_rs
assert '"ibd" => {' in model_script_rs
assert 'workspace\n                .ibd_diagrams' in model_script_rs

print("Imported diagram catalog contract passed")
