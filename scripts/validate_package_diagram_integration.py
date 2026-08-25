"""PR26A Package Diagram creation and discoverability integration contract."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


index = read("apps/desktop/frontend/index.html")
app = read("apps/desktop/frontend/app.js")
shell = read("apps/desktop/frontend/ui-shell.js")
workspace = read("apps/desktop/frontend/workspace-ux.js")
repository = read("apps/desktop/frontend/repository-tree-ui.js")
bdd_completion = read("apps/desktop/frontend/bdd-completion-ui.js")
commands = read("apps/desktop/src-tauri/src/workspace/package_diagrams.rs")
main = read("apps/desktop/src-tauri/src/main.rs")

assert 'id="new-package-diagram"' in index
assert '<span class="command-icon">pkg</span><span>Package Diagram</span>' in index
assert shell.count('data-forward="new-package-diagram"') == 2
assert shell.count('<span class="command-icon">pkg</span>') == 2
assert '<span>Package<br>Diagram</span>' in shell
assert '<span>New Package<br>Diagram</span>' in shell
assert "'new-package-diagram'" in shell
assert 'data-action="new-package-diagram"' not in shell

assert "$('new-package-diagram').onclick = createPackageDiagram" in app
assert "window.smpCreatePackageDiagram = createPackageDiagram" in app
assert "const ownerId = state.selectedPackageId || project.root_id" in app
assert "['Model', 'Package'].includes(owner.kind)" in app
assert "requireInvoke()('create_package_diagram'" in app
assert "await refresh();\n  await selectDiagram(diagramId);" in app
assert "window.smpCreatePackageDiagram" not in workspace

for frontend in (app, bdd_completion, repository):
    assert "diagram.family === 'package'" in frontend
    assert "'PKG'" in frontend
assert "renderPackageDiagramTabs" not in workspace
assert "renderPackageRepository" not in workspace

assert "pub fn create_package_diagram" in commands
assert "ElementKind::Model | ElementKind::Package" in commands
assert 'family: "package".into()' in commands
assert "create_package_diagram" in main
assert "place_on_package_diagram" in main

print("PR26A Package Diagram creation and discoverability integration contract passed")
