"""Validate the PR22 cross-diagram standard editing and authoring boundary."""

from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def require(source: str, tokens: list[str], label: str) -> None:
    missing = [token for token in tokens if token not in source]
    if missing:
        raise SystemExit(f"{label} is missing: {', '.join(missing)}")


standard = read("apps/desktop/src-tauri/src/workspace/standard_editing.rs")
bridge = read("apps/desktop/src-tauri/src/workspace/standard_editing_bridge.rs")
ui = read("apps/desktop/frontend/standard-editing-ui.js")
marquee = read("apps/desktop/frontend/marquee-selection.js")
index = read("apps/desktop/frontend/index.html")
shared = read("apps/desktop/frontend/shared-workspace.js")
app = read("apps/desktop/frontend/app.js")
workspace_ux = read("apps/desktop/frontend/workspace-ux.js")
ibd_ui = read("apps/desktop/frontend/ibd-ui.js")
use_case_ui = read("apps/desktop/frontend/use-case-ui.js")
parametric_ui = read("apps/desktop/frontend/parametric-ui.js")
behavior_completion_ui = read("apps/desktop/frontend/behavior-completion-ui.js")
behavior_sequence_input = read("apps/desktop/frontend/behavior-sequence-input.js")
activity_ui = read("apps/desktop/frontend/activity-ui.js")
activity_rich_ui = read("apps/desktop/frontend/activity-rich-ui.js")
main = read("apps/desktop/src-tauri/src/main.rs")
behavior = read("apps/desktop/src-tauri/src/workspace/behavior_workspace.rs")
repository = read("apps/desktop/src-tauri/src/workspace/repository_editing.rs")
workflow = read(".github/workflows/ci.yml")

require(
    standard,
    [
        "enum EditingFamily",
        "Bdd,",
        "Package,",
        "Requirement,",
        "UseCase,",
        "Ibd,",
        "StateMachine,",
        "Sequence,",
        "Activity,",
        "fn collect_clipboard(",
        "fn paste_clipboard(",
        "fn duplicate_selection_items(",
        "fn remove_presentations(",
        "fn move_selection_items(",
        "history::checkpoint_states",
        "pub fn copy_selection(",
        "pub fn paste_selection(",
        "pub fn duplicate_selection(",
        "pub fn delete_active_selection(",
        "pub fn move_active_selection(",
    ],
    "Rust standard editing engine",
)

for command in (
    "copy_selection",
    "paste_selection",
    "duplicate_selection",
    "delete_active_selection",
    "move_active_selection",
):
    if f"#[tauri::command]\npub fn {command}(" in standard:
        raise SystemExit(
            f"standard_editing.rs: {command} must remain an internal Rust function; "
            "the shared-selection bridge is the sole Tauri command boundary"
        )
    if f"#[tauri::command]\npub fn {command}(" not in bridge:
        raise SystemExit(
            f"standard_editing_bridge.rs: missing Tauri bridge for {command}"
        )

require(
    bridge,
    [
        "workspace_interaction_snapshot",
        "active_selections",
        "from_model: Option<bool>",
        "Delete from Model requires exactly one selected relationship",
        "delete_selected_relationship_from_model",
    ],
    "Rust shared-selection bridge",
)

require(
    ui,
    [
        "copy_selection",
        "paste_selection",
        "duplicate_selection",
        "delete_active_selection",
        "Remove from Diagram",
        "Delete from Model",
        "event.ctrlKey || event.metaKey || event.shiftKey",
        "window.smpRendererHost?.publishInteraction?.()",
        "window.smpStandardEditing",
        "Deleting Behavior item from model",
        "Deleting Activity item from model",
        "itemKind: edge ? 'edge' : 'node'",
        "The selected Activity presentation does not resolve to a semantic node or edge.",
    ],
    "cross-diagram editing UI",
)

require(
    marquee,
    [
        "smp-marquee-selection",
        "[data-smp-presentation-id]",
        "window.smpStandardEditing?.selections?.()",
        "window.smpStandardEditing?.setSelections?.([...existing, ...hits])",
        "smp:selection-changed",
        "event.ctrlKey",
        "event.metaKey",
        "canvas.classList.contains('pan-active')",
        "canvas.classList.contains('is-panning')",
    ],
    "shared marquee selection",
)
for forbidden in (
    "addEventListener('keydown'",
    'addEventListener("keydown"',
    "addEventListener('keyup'",
    'addEventListener("keyup"',
):
    if forbidden in marquee:
        raise SystemExit(
            "marquee-selection.js must not own a keyboard controller; "
            "Space/Ctrl/Meta panning belongs to shared-workspace.js"
        )

# A simple palette click on an empty renderer surface must remain a renderer click.
# Capturing the pointer during pointerdown can retarget the synthetic click to the
# shared canvas and silently disable click-to-place in every family that authors by
# clicking its SVG/frame. Marquee may capture only after the drag threshold is met.
marquee_pointerdown = marquee.split("canvas.addEventListener('pointerdown'", 1)[1].split(
    "canvas.addEventListener('pointermove'", 1
)[0]
if "setPointerCapture" in marquee_pointerdown:
    raise SystemExit(
        "marquee-selection.js captures on pointerdown and can steal palette click-to-place from diagram renderers"
    )
marquee_pointermove = marquee.split("canvas.addEventListener('pointermove'", 1)[1].split(
    "canvas.addEventListener('pointerup'", 1
)[0]
require(
    marquee_pointermove,
    [
        "Math.hypot(drag.lastX - drag.startX, drag.lastY - drag.startY) >= 4",
        "drag.moved = true;",
        "canvas.setPointerCapture?.(event.pointerId);",
    ],
    "marquee drag-threshold pointer capture",
)

if '<script src="standard-editing-ui.js"></script>\n  <script src="marquee-selection.js"></script>' not in index:
    raise SystemExit("marquee-selection.js must load after standard-editing-ui.js")

if "itemType: edge ? 'Edge' : 'Node'" in ui:
    raise SystemExit(
        "standard-editing-ui.js: Activity model deletion must use the qualified "
        "delete_activity_item itemKind contract with lowercase edge/node values"
    )

# Cross-family authoring preservation. These assertions intentionally validate the
# established family adapters rather than inventing a new generic authoring system.
# PR31 execution must coexist with the same palette/canvas/repository paths that
# were already qualified before execution was introduced.
require(
    app,
    [
        "button.onclick = () => activatePaletteItem(item);",
        "frame.onclick = async (event) => {",
        "await createPaletteElementAt(state.paletteTool, point.x, point.y);",
        "frame.ondrop = async (event) => {",
        "await placeExistingElementAt(elementId, point.x, point.y);",
        "place_element_on_bdd",
        "place_on_requirement_diagram",
    ],
    "BDD/Requirement palette and repository authoring",
)
require(
    workspace_ux,
    [
        "if (distance < 5) return;",
        "await createPaletteElementAt(payload.item, point.x, point.y);",
        "await placeExistingElementAt(payload.elementId, point.x, point.y);",
        "create_package_element",
        "place_on_package_diagram",
    ],
    "shared drag/drop and Package Diagram authoring",
)
require(
    use_case_ui,
    [
        "frame.onclick = async (event) => {",
        "await createPaletteElementAt(state.paletteTool, point.x, point.y);",
        "frame.ondrop = async (event) => {",
        "create_use_case_element",
        "place_on_use_case_diagram",
    ],
    "Use Case palette and repository authoring",
)
require(
    parametric_ui,
    [
        "frame.onclick = async (event) => {",
        "await createPaletteElementAt(state.paletteTool, point.x, point.y);",
        "frame.ondrop = async (event) => {",
        "create_parametric_constraint_property",
        "create_parametric_value_property",
        "place_on_parametric_diagram",
    ],
    "Parametric palette and repository authoring",
)
require(
    ibd_ui,
    [
        "populate_ibd_from_context",
        "add_nested_port_to_ibd",
        "create_ibd_connector",
    ],
    "IBD qualified context/population authoring",
)
require(
    behavior_completion_ui,
    [
        "button.onclick = () => activateBehaviorTool(item.id);",
        "frame.addEventListener('click', async (event) => {",
        "await createStateToolAt(frame, diagram, event);",
        "add_state_vertex",
        "add_composite_state",
    ],
    "State Machine palette click-to-place authoring",
)
require(
    behavior_sequence_input,
    [
        "event.target.closest?.('.sequence-frame')",
        "state.behaviorTool !== 'Lifeline'",
        "behavior_lifeline_candidates",
        "add_sequence_lifeline",
    ],
    "Sequence Lifeline click-to-place authoring",
)
require(
    activity_ui,
    [
        "button.onclick = () => {",
        "svg.onclick = async (event) => {",
        "if (!state.activityTool) return;",
        "add_activity_node",
        "add_activity_edge",
    ],
    "Activity base palette click-to-place authoring",
)
require(
    activity_rich_ui,
    [
        "RICH_NODE_TOOLS",
        "STRUCTURED_TOOLS",
        "document.addEventListener('click', async (event) => {",
        "add_activity_action",
        "add_activity_parameter_node",
        "add_structured_activity_node",
    ],
    "Activity rich palette click-to-place authoring",
)

require(
    shared,
    [
        "publishInteraction",
        "set_workspace_interaction",
        "workspace_interaction_snapshot",
        "await publishInteraction();",
        "state.space",
        "pan-active",
        "is-panning",
        "smp:selection-changed",
    ],
    "shared workspace interaction authority",
)

require(
    main,
    [
        "StandardEditingState",
        ".manage(StandardEditingState::default())",
        "copy_selection,",
        "paste_selection,",
        "duplicate_selection,",
        "delete_active_selection,",
        "move_active_selection,",
    ],
    "desktop command registration",
)

require(
    behavior,
    [
        "hidden_semantic_ids: Vec<String>",
        "presentation_copies: Vec<BehaviorPresentationCopy>",
        "BEHAVIOR_DIAGRAM_METADATA_KEY",
    ],
    "behavior presentation persistence",
)

require(
    repository,
    [
        "pub fn delete_model_element(",
        "pub fn delete_repository_diagram(",
        "history::checkpoint_states",
    ],
    "model-vs-presentation deletion governance",
)

require(
    workflow,
    [
        "apps/desktop/frontend/standard-editing-ui.js",
        "apps/desktop/frontend/marquee-selection.js",
    ],
    "frontend syntax qualification",
)

print(
    "PR22/PR24/PR25/PR26B/PR31 standard editing and authoring integration contract passed: all nine "
    "diagram families retain their qualified creation/placement paths plus Rust-owned clipboard/remove/move "
    "authority, shared click/marquee selection synchronization, shared pan ownership, presentation persistence, "
    "model-vs-diagram deletion separation, and qualified Behavior/Activity model-deletion history wiring"
)
