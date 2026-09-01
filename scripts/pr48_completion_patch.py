from pathlib import Path

path = Path("apps/desktop/src-tauri/src/workspace/bulk_model/pr48_tests.rs")
if path.exists():
    text = path.read_text()
    text = text.replace(
        "use systems_modeler_core::{ActivityEdgeKind, ActivityNodeKind, ActivitySemanticId, ElementKind};",
        "use systems_modeler_core::{ActivityEdgeKind, ActivitySemanticId, ElementKind};",
    )
    path.write_text(text)
