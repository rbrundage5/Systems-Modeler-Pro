"""PR26B Package Diagram semantic, palette, editing, and rendering contract."""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


index = read("apps/desktop/frontend/index.html")
app = read("apps/desktop/frontend/app.js")
shell = read("apps/desktop/frontend/ui-shell.js")
workspace = read("apps/desktop/frontend/workspace-ux.js")
shared_frontend = read("apps/desktop/frontend/shared-workspace.js")
repository = read("apps/desktop/frontend/repository-tree-ui.js")
bdd_completion = read("apps/desktop/frontend/bdd-completion-ui.js")
bdd_commands = read("apps/desktop/src-tauri/src/workspace/bdd_elements.rs")
ibd_ui = read("apps/desktop/frontend/ibd-ui.js")
use_case_ui = read("apps/desktop/frontend/use-case-ui.js")
parametric_ui = read("apps/desktop/frontend/parametric-ui.js")
commands = read("apps/desktop/src-tauri/src/workspace/package_diagrams.rs")
shared_workspace = read("apps/desktop/src-tauri/src/workspace/shared_workspace.rs")
standard_editing = read("apps/desktop/src-tauri/src/workspace/standard_editing.rs")
family_registry = read("crates/model-core/src/diagram_family.rs")
presentation_theme = read("apps/desktop/src-tauri/src/workspace/presentation_theme.rs")
model = read("crates/model-core/src/model.rs")
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

for semantic_kind in (
    "ModelLibrary",
    "PackageImport",
    "ElementImport",
    "VisibilityKind",
):
    assert semantic_kind in model
assert "pub fn create_package_import" in model
assert "pub fn create_element_import" in model
assert "DuplicatePackageRelationship" in model
assert "InvalidElementImportAlias" in model
assert '"package",\n            "Package Diagram",\n            ("pkg", "Package")' in family_registry
assert "C::Relationships" in family_registry
assert '"ModelLibrary", FRAME' in presentation_theme
assert '"Package",' in presentation_theme

for command in (
    "create_package_element",
    "create_package_relationship",
    "reconnect_package_relationship",
    "delete_package_relationship",
    "update_package_element",
    "update_package_relationship",
):
    assert f"pub fn {command}" in commands
    assert command in main
assert "project.validate().map_err" in commands
assert "validate_package_diagram(&project, diagram)" in commands
assert "history::checkpoint_states" in commands
assert "reroute(&mut diagrams[index])" in commands

palette_map_source = app.split("const PALETTE_TYPE_BY_FAMILY = Object.freeze({", 1)[1].split("});", 1)[0]
palette_types = {(quoted or bare): palette_type for quoted, bare, palette_type in re.findall(
    r"(?:'([^']+)'|([a-z]+)):\s*'([^']+)'", palette_map_source
)}
assert palette_types == {
    "bdd": "BDD",
    "requirement": "Requirement",
    "use-case": "UseCase",
    "parametric": "Parametric",
    "package": "Package",
}
assert "return 'IBD'" in app
assert "const diagramType = resolvePaletteDiagramType(state.snapshot, diagramId)" in app
assert "requireInvoke()('diagram_palette', { diagramType })" in app
assert "request !== paletteLoadRequest || state.selectedDiagramId !== diagramId" in app
assert "request !== diagramSelectionRequest || state.selectedDiagramId !== diagramId" in app
for adapter in (ibd_ui, use_case_ui, parametric_ui):
    assert "loadPalette =" not in adapter

palette_switch = ["BDD", "Package", "Parametric", "Package", "IBD"]
family_switch = ["bdd", "package", "parametric", "package"]
assert [palette_types[family] for family in family_switch] + ["IBD"] == palette_switch

package_palette = main.split('"Package" => Ok(vec![', 1)[1].split(']),\n        "UseCase"', 1)[0]
for palette_item in (
    'element_item("package", "Package", "Package")',
    'element_item("model-library", "Model Library", "ModelLibrary")',
    'element_item("block", "Block", "Block")',
    'element_item("association-block", "Association Block", "AssociationBlock")',
    'element_item("interface-block", "Interface Block", "InterfaceBlock")',
    'element_item("constraint-block", "Constraint Block", "ConstraintBlock")',
    'element_item("value-type", "Value Type", "ValueType")',
    'element_item("data-type", "Data Type", "DataType")',
    'element_item("primitive-type", "Primitive Type", "PrimitiveType")',
    'element_item("enumeration", "Enumeration", "Enumeration")',
    'element_item("signal", "Signal", "Signal")',
    'element_item("unit", "Unit", "Unit")',
    'element_item("quantity-kind", "Quantity Kind", "QuantityKind")',
    'element_item("requirement", "Requirement", "Requirement")',
    'element_item("test-case", "Test Case", "TestCase")',
    'element_item("actor", "Actor", "Actor")',
    'element_item("use-case", "Use Case", "UseCase")',
    'element_item("comment", "Comment", "Comment")',
    'relationship_item("package-import", "Package Import", "PackageImport")',
    'relationship_item("element-import", "Element Import", "ElementImport")',
    'relationship_item("dependency", "Dependency", "Dependency")',
):
    assert palette_item in package_palette
for owned_feature_kind in (
    '"PartProperty"',
    '"ReferenceProperty"',
    '"ValueProperty"',
    '"FlowProperty"',
    '"ConstraintProperty"',
    '"ProxyPort"',
    '"FullPort"',
    '"Operation"',
    '"Parameter"',
    '"EnumerationLiteral"',
    '"Slot"',
):
    assert owned_feature_kind not in package_palette
assert 'PackageMerge' not in package_palette
assert 'package_relationship_semantic_kind("PackageMerge").is_err()' in commands
assert "create_package_element" in workspace
assert "place_on_package_diagram" in workspace
assert "element.packageable" in workspace
assert "Boolean(element.namespace)" in workspace
assert "Boolean(element.packageable)" in workspace
assert "pub namespace: bool" in bdd_commands
assert "namespace: element.is_namespace()" in bdd_commands
assert "create_package_relationship" in workspace
package_selection = workspace.split("async function selectPackageNode", 1)[1].split(
    "function renderPackageCanvas", 1
)[0]
assert "create_package_relationship" in package_selection
assert "create_bdd_relationship" not in package_selection
assert "create_bdd_relationship_complete" not in package_selection
assert "is not a valid Package relationship endpoint" not in package_selection
assert "registerRenderer('package'" not in shared_frontend
assert "registerSelectionRenderer?.(\n    'package'" in workspace
assert "{ renderCanvas: renderPackageCanvas }" in workspace
assert "update_package_element" in workspace
assert "update_package_relationship" in workspace
assert "sourceElementId: sourceId" in workspace
assert "targetElementId: targetId" in workspace
assert "delete_package_relationship" in workspace
assert "«modelLibrary»" in workspace
assert "package-node-tab" in workspace
assert "package-node-body" in workspace
assert "['Model', 'Package', 'ModelLibrary'].includes(element.kind)" in workspace
assert "package-contents" in workspace
assert "window.smpRequirementPresentationMarkup(element)" in workspace
assert "window.smpUseCasePresentation.actorMarkup(element, node)" in workspace
assert "window.smpUseCasePresentation.useCaseMarkup(element)" in workspace
assert "window.smpRequirementPresentationMarkup = requirementPresentationMarkup" in app
assert "window.smpUseCasePresentation = Object.freeze({ actorMarkup, useCaseMarkup })" in use_case_ui
assert "candidate.owner_id === element.id" in workspace
assert "pkg [package]" in workspace
assert "node.dataset.smpInternalDrag = 'true'" in workspace
assert "createPaletteElementAt = async function createPackagePaletteElementAt" in workspace
assert "move_repository_element" in workspace

assert '"bdd" | "requirement" | "use-case" | "package"' in shared_workspace
assert "super::route_bdd_with_bounds" in shared_workspace
assert "super::layout_bdd_with_bounds" in shared_workspace
assert '"bdd" | "requirement" | "package" => {' in shared_workspace
assert "rename_owner_owned_diagram(&workspace, &diagram_id, model_element_name, diagram_name)" in shared_workspace
assert "package_header_apply_renames_the_rust_owner_and_diagram" in shared_workspace
assert "await renderer()?.refresh?.()" in read("apps/desktop/frontend/shared-workspace.js")
assert "EditingFamily::Package" in standard_editing
assert "duplicate.owner_id = duplicate" in standard_editing
assert "relationship.visibility === 'private' ? 'access' : 'import'" in app
assert "edge.label_anchor" in app
assert "['Dependency', 'PackageImport', 'ElementImport'" in app
assert "PackageMerge" not in workspace
assert "Importing Namespace" in workspace
assert "Imported Package" in workspace
assert "kind === 'Dependency') return Boolean(element.packageable)" in workspace
assert "requirementId: definition.values.requirementId || null" in workspace
assert "pkg-display-contents" in workspace
assert '>Public</option>' in workspace
assert '>Private</option>' in workspace
assert "await refresh();" in workspace
assert "!source.is_classifier() || !target.is_classifier()" in bdd_commands
assert "requires classifier endpoints on a BDD" in bdd_commands

print("PR26B Package Diagram semantic and workspace integration contract passed")
