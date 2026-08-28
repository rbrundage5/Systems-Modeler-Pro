from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def write(path: str, text: str) -> None:
    (ROOT / path).write_text(text, encoding="utf-8")


def replace_between(text: str, start: str, end: str, replacement: str) -> str:
    left = text.find(start)
    if left < 0:
        raise SystemExit(f"missing start marker: {start}")
    right = text.find(end, left)
    if right < 0:
        raise SystemExit(f"missing end marker: {end}")
    return text[:left] + replacement + text[right:]


# ---------------------------------------------------------------------------
# Shared frontend gesture authority
# ---------------------------------------------------------------------------
path = "apps/desktop/frontend/diagram-interaction.js"
text = read(path)
text = text.replace(
    "    const scale = options.scale || { x: 1, y: 1 };\n",
    "    const scale = options.scale || surfaceScale(owner);\n",
    1,
)
text = text.replace(
    "    const handle = event.target.closest?.('.smp-resize-handle');\n    if (!handle && event.target.closest?.('.constraint-parameter, input, select, textarea, a, [contenteditable=\"true\"]')) return;\n",
    "    const handle = event.target.closest?.('.smp-resize-handle');\n    // Fork/Join uses the same gesture lifecycle but a notation-specific thickness\n    // adapter in state-bar-resize.js. Let that adapter receive its resize handle.\n    if (handle && node.matches?.('.state-fork, .state-join')) return;\n    if (!handle && event.target.closest?.('.constraint-parameter, input, select, textarea, a, [contenteditable=\"true\"]')) return;\n",
    1,
)

activity_impl = r'''function installActivity() {
    const diagram = state.activitySnapshot?.diagrams?.find((item) => String(item.id) === String(state.selectedActivityDiagramId));
    if (!diagram) return;
    const svg = document.querySelector('#canvas .activity-svg');
    if (!svg) return;
    const viewBox = svg.viewBox.baseVal;
    const diagramScale = () => {
      const rect = svg.getBoundingClientRect();
      return {
        x: Math.max(rect.width / Math.max(1, viewBox.width), 0.0001),
        y: Math.max(rect.height / Math.max(1, viewBox.height), 0.0001),
      };
    };
    svg.querySelectorAll('.activity-node').forEach((group) => {
      const semanticId = group.dataset.activityNodeId;
      const presentation = diagram.nodes?.find((item) => String(item.activity_node_id) === String(semanticId));
      if (!presentation) return;
      group.dataset.smpGeometryBound = '1';
      group.classList.add('smp-interactive-presentation');
      let handle = group.querySelector('.smp-svg-resize-handle');
      if (!handle) {
        handle = document.createElementNS('http://www.w3.org/2000/svg', 'rect');
        handle.classList.add('smp-svg-resize-handle');
        handle.setAttribute('width', '12');
        handle.setAttribute('height', '12');
        group.appendChild(handle);
      }
      handle.setAttribute('x', presentation.x + presentation.width - 6);
      handle.setAttribute('y', presentation.y + presentation.height - 6);
      handle.onclick = (event) => {
        event.preventDefault();
        event.stopPropagation();
      };

      group.onpointerdown = (event) => {
        if (event.button !== 0 || event.target === handle) return;
        event.preventDefault();
        event.stopPropagation();
        state.selectedActivityNodeId = semanticId;
        const original = { ...presentation };
        let next = { ...original };
        beginPointerGesture(event, {
          owner: group,
          scale: diagramScale(),
          prepare: cancelTransientAuthoring,
          disabled: () => !!state.activityPendingFlow || !!state.activityTool,
          onStart: () => group.classList.add('smp-dragging'),
          onMove: (dx, dy) => {
            next.x = Math.max(0, original.x + dx);
            next.y = Math.max(42, original.y + dy);
            group.setAttribute('transform', `translate(${next.x - original.x} ${next.y - original.y})`);
          },
          onCancel: () => {
            group.classList.remove('smp-dragging');
            group.removeAttribute('transform');
          },
          onCommit: async () => {
            group.classList.remove('smp-dragging');
            group.removeAttribute('transform');
            await commit('update_activity_presentation_geometry', {
              diagramId: diagram.id,
              presentationId: presentation.id,
              x: next.x, y: next.y, width: next.width, height: next.height,
            });
          },
        });
      };

      handle.onpointerdown = (event) => {
        if (event.button !== 0) return;
        event.preventDefault();
        event.stopPropagation();
        const original = { ...presentation };
        let next = { ...original };
        beginPointerGesture(event, {
          owner: handle,
          scale: diagramScale(),
          prepare: cancelTransientAuthoring,
          disabled: () => !!state.activityPendingFlow || !!state.activityTool,
          onMove: (dx, dy) => {
            next.width = Math.max(MIN.Activity.width, original.width + dx);
            const barLike = group.classList.contains('activity-fork') || group.classList.contains('activity-join');
            next.height = barLike
              ? Math.max(20, Math.min(24, original.height + dy))
              : Math.max(MIN.Activity.height, original.height + dy);
            applyActivityShapeGeometry(group, next);
            handle.setAttribute('x', original.x + next.width - 6);
            handle.setAttribute('y', original.y + next.height - 6);
          },
          onCancel: () => render(),
          onCommit: async () => {
            await commit('update_activity_presentation_geometry', {
              diagramId: diagram.id,
              presentationId: presentation.id,
              x: original.x, y: original.y, width: next.width, height: next.height,
            });
          },
        });
      };
    });
  }

  '''
text = replace_between(text, "function installActivity() {", "function install() {", activity_impl)
write(path, text)


# ---------------------------------------------------------------------------
# State Machine Fork/Join retains notation-specific thickness, but consumes the
# same shared pointer lifecycle as the other presentations.
# ---------------------------------------------------------------------------
path = "apps/desktop/frontend/state-bar-resize.js"
text = read(path)
state_bar_impl = r'''function bindStateBars() {
    const diagram = activeStateDiagram();
    if (!diagram || diagram.kind !== 'StateMachine') return;

    document.querySelectorAll('#canvas .state-fork, #canvas .state-join').forEach((node) => {
      const vertexId = node.dataset.vertexId;
      const presentation = diagram.state_nodes?.find(
        (item) => String(item.vertex_id) === String(vertexId),
      );
      const bar = node.querySelector('.fork-bar');
      const handle = node.querySelector('.smp-resize-handle');
      if (!presentation || !bar || !handle) return;

      const renderedThickness = displayThickness(presentation);
      bar.style.width = '100%';
      bar.style.height = `${renderedThickness}px`;

      handle.onpointerdown = (event) => {
        if (event.button !== 0 || state.behaviorPending || state.behaviorTool) return;
        event.preventDefault();
        event.stopPropagation();
        const begin = window.smpBeginPresentationGesture;
        if (typeof begin !== 'function') return;
        const original = { ...presentation };
        const originalThickness = displayThickness(original);
        let next = { ...original };
        let nextThickness = originalThickness;
        begin(event, {
          owner: handle,
          disabled: () => !!state.behaviorPending || !!state.behaviorTool,
          onMove: (dx, dy) => {
            next.width = Math.max(24, original.width + dx);
            nextThickness = clampThickness(originalThickness + dy);
            next.height = storedHeightForThickness(nextThickness);
            node.style.width = `${next.width}px`;
            node.style.height = `${next.height}px`;
            bar.style.width = '100%';
            bar.style.height = `${nextThickness}px`;
            updateIncidentTransitions(diagram, vertexId, next);
          },
          onCancel: () => render(),
          onCommit: async () => {
            await window.smpCommitPresentationGeometry('update_state_presentation_geometry', {
              diagramId: diagram.id,
              stateVertexId: String(vertexId),
              x: next.x,
              y: next.y,
              width: next.width,
              height: next.height,
            });
          },
        });
      };
    });
  }

  '''
text = replace_between(text, "function bindStateBars() {", "const baseRender = render;", state_bar_impl)
write(path, text)


# ---------------------------------------------------------------------------
# Sequence: remove the competing second drag owner and keep only a route preview
# helper that understands the actual polyline rendering.
# ---------------------------------------------------------------------------
path = "apps/desktop/frontend/interaction-runtime-fixes.js"
text = read(path)
old_update_start = "function updateSequenceMessages(interaction, lifelineId, x) {"
old_update_end = "function bindSequenceConnectedDrag() {"
new_update = r'''function updateSequenceMessages(interaction, lifelineId, x) {
    for (const message of interaction?.messages || []) {
      const line = document.querySelector(
        `#canvas .sequence-message[data-message-id="${CSS.escape(String(message.id))}"]`,
      );
      if (!line) continue;
      const raw = (line.getAttribute('points') || '').trim();
      const points = raw ? raw.split(/\s+/).map((pair) => {
        const [px, py] = pair.split(',').map(Number);
        return { x: px, y: py };
      }).filter((point) => Number.isFinite(point.x) && Number.isFinite(point.y)) : [];
      if (points.length < 2) continue;
      if (String(message.send_event?.lifeline_id || '') === String(lifelineId)) {
        const oldX = points[0].x;
        points[0].x = x;
        if (points[1] && Math.abs(points[1].x - oldX) < 0.1) points[1].x = x;
      }
      if (String(message.receive_event?.lifeline_id || '') === String(lifelineId)) {
        const last = points.length - 1;
        const oldX = points[last].x;
        points[last].x = x;
        if (points[last - 1] && Math.abs(points[last - 1].x - oldX) < 0.1) points[last - 1].x = x;
      }
      line.setAttribute('points', points.map((point) => `${point.x},${point.y}`).join(' '));
    }
  }

  window.smpPreviewSequenceLifelineGeometry = updateSequenceMessages;

  '''
text = replace_between(text, old_update_start, old_update_end, new_update)
# Remove the obsolete bindSequenceConnectedDrag function + render wrapper.
text = replace_between(
    text,
    "function bindSequenceConnectedDrag() {",
    "})();\n\n(() => {",
    "",
)
text = text.replace("})();\n\n(() => {", "})();\n\n(() => {", 1)
write(path, text)


# ---------------------------------------------------------------------------
# Authoritative behavior renderer: generic State geometry is handled by the
# central HTML controller; Sequence move/resize consumes the shared lifecycle.
# ---------------------------------------------------------------------------
path = "apps/desktop/frontend/behavior-authoritative-renderer.js"
text = read(path)
# Remove local State pointer lifecycle; central state adapter owns it.
text = replace_between(
    text,
    "      node.onpointerdown = (event) => {",
    "      frame.appendChild(node);",
    "      frame.appendChild(node);",
)

seq_old_start = "      if (!presentation.fallback) {\n        timelineResize.onpointerdown = (event) => {"
seq_old_end = "      frame.appendChild(node);"
seq_new = r'''      if (!presentation.fallback) {
        timelineResize.onpointerdown = (event) => {
          if (event.button !== 0 || state.behaviorPending || state.behaviorTool) return;
          event.preventDefault();
          event.stopPropagation();
          const begin = window.smpBeginPresentationGesture;
          if (typeof begin !== 'function') return;
          const originalEnd = timelineEnd;
          let nextEnd = originalEnd;
          begin(event, {
            owner: timelineResize,
            disabled: () => !!state.behaviorPending || !!state.behaviorTool,
            onMove: (_dx, dy) => {
              nextEnd = Math.max(timelineStart + 80, originalEnd + dy);
              timeline.style.height = `${nextEnd - timelineStart}px`;
              node.style.height = `${Math.max(120, nextEnd - 60)}px`;
              timelineResize.style.top = `${Math.max(50, nextEnd - 66)}px`;
            },
            onCancel: () => render(),
            onCommit: async () => {
              await window.smpCommitPresentationGeometry('resize_sequence_lifeline_timeline', {
                diagramId: diagram.id,
                lifelineIdValue: String(lifeline.id),
                timelineStartY: timelineStart,
                timelineEndY: nextEnd,
              });
            },
          });
        };
        node.onpointerdown = (event) => {
          if (event.button !== 0 || event.target.closest?.('.lifeline-resize-handle')) return;
          if (state.behaviorPending || state.behaviorTool) return;
          event.preventDefault();
          event.stopPropagation();
          const begin = window.smpBeginPresentationGesture;
          if (typeof begin !== 'function') return;
          const original = presentation.x;
          let nextX = original;
          begin(event, {
            owner: node,
            disabled: () => !!state.behaviorPending || !!state.behaviorTool,
            onStart: () => node.classList.add('smp-dragging'),
            onMove: (dx) => {
              nextX = Math.max(70, original + dx);
              node.style.left = `${nextX - 65}px`;
              window.smpPreviewSequenceLifelineGeometry?.(inter, lifeline.id, nextX);
            },
            onCancel: () => {
              node.classList.remove('smp-dragging');
              render();
            },
            onCommit: async () => {
              node.classList.remove('smp-dragging');
              await runCommand('Moving Lifeline…', () => requireInvoke()('move_sequence_lifeline', {
                diagramId: diagram.id,
                lifelineIdValue: String(lifeline.id),
                x: nextX,
              }));
              await refresh();
            },
          });
        };
      }
      frame.appendChild(node);'''
text = replace_between(text, seq_old_start, seq_old_end, seq_new)
write(path, text)


# ---------------------------------------------------------------------------
# Parametric constraint parameter movement uses the same gesture lifecycle.
# ---------------------------------------------------------------------------
path = "apps/desktop/frontend/parametric-ui.js"
text = read(path)
param_move = r'''function bindParameterMove(button, diagram, node, parameter) {
    button.onpointerdown = (event) => {
      if (event.button !== 0 || state.pendingRelationship || state.paletteTool) return;
      event.preventDefault();
      event.stopPropagation();
      const begin = window.smpBeginPresentationGesture;
      if (typeof begin !== 'function') return;
      const originalX = parameter.offset_x;
      const originalY = parameter.offset_y;
      let nextX = originalX;
      let nextY = originalY;
      begin(event, {
        owner: button,
        disabled: () => !!state.pendingRelationship || !!state.paletteTool,
        onMove: (dx, dy) => {
          nextX = originalX + dx;
          nextY = originalY + dy;
          button.style.left = `${nextX}px`;
          button.style.top = `${nextY}px`;
        },
        onCancel: () => render(),
        onCommit: async () => {
          await runCommand('Moving constraint parameter…', () => requireInvoke()('update_constraint_parameter_presentation', {
            diagramId: diagram.id,
            presentationId: parameter.id,
            offsetX: nextX,
            offsetY: nextY,
          }));
          await refresh();
        },
      });
    };
  }

  '''
text = replace_between(text, "function bindParameterMove(button, diagram, node, parameter) {", "function constraintMarkup", param_move)
write(path, text)


# ---------------------------------------------------------------------------
# Rust presentation commands: direct manipulation reroutes only incident edges.
# ---------------------------------------------------------------------------
path = "apps/desktop/src-tauri/src/workspace/presentation_interaction.rs"
text = read(path)
text = text.replace(
    "    behavior_workspace::reroute_behavior_presentation(diagram, &repository, None)?;\n",
    "    behavior_workspace::reroute_incident_state_transitions(\n        diagram,\n        &repository,\n        &state_vertex_id,\n    )?;\n",
    1,
)
text = text.replace(
    "    for edge in &mut diagram.edges {\n",
    "    for edge in diagram.edges.iter_mut().filter(|edge| {\n        edge.source_node_id == presentation_id || edge.target_node_id == presentation_id\n    }) {\n",
    1,
)
write(path, text)


# ---------------------------------------------------------------------------
# Behavior incident routing + undo-aware lifeline/legacy state movement.
# ---------------------------------------------------------------------------
path = "apps/desktop/src-tauri/src/workspace/behavior_workspace.rs"
text = read(path)
helper_marker = '''pub(super) fn reroute_behavior_presentation(
    diagram: &mut BehaviorDiagram,
    repository: &BehaviorRepository,
    bounds: Option<super::routing::RouteRect>,
) -> Result<(), String> {
    // Compute the complete route set before replacing any committed geometry.
    // This is the same transactional behavior used by Route and Clean Layout.
    diagram.edge_routes = routed_behavior_edges(diagram, repository, bounds)?;
    Ok(())
}
'''
helpers = helper_marker + r'''
fn retain_incident_state_transitions(regions: &mut [Region], vertex_id: &str) {
    for region in regions {
        region.transitions.retain(|transition| {
            transition.source_id.to_string() == vertex_id
                || transition.target_id.to_string() == vertex_id
        });
        for vertex in &mut region.vertices {
            if let VertexKind::State(state) = &mut vertex.kind {
                retain_incident_state_transitions(&mut state.regions, vertex_id);
            }
        }
    }
}

pub(super) fn reroute_incident_state_transitions(
    diagram: &mut BehaviorDiagram,
    repository: &BehaviorRepository,
    vertex_id: &str,
) -> Result<(), String> {
    if diagram.kind != BehaviorDiagramKind::StateMachine {
        return Err("active behavior diagram is not a State Machine".into());
    }
    let machine_id = state_machine_id(&diagram.semantic_id)?;
    let mut filtered = repository.clone();
    let machine = filtered
        .state_machines
        .get_mut(&machine_id)
        .ok_or("State Machine not found")?;
    retain_incident_state_transitions(&mut machine.regions, vertex_id);
    let mut endpoints = Vec::new();
    collect_transition_endpoints(&machine.regions, &mut endpoints);
    let wanted: BTreeSet<_> = endpoints.into_iter().map(|(id, _, _, _)| id).collect();
    if wanted.is_empty() {
        return Ok(());
    }
    let routes = state_machine_routes(diagram, &filtered, None)?;
    for route in routes {
        if let Some(existing) = diagram
            .edge_routes
            .iter_mut()
            .find(|existing| existing.semantic_id == route.semantic_id)
        {
            *existing = route;
        } else {
            diagram.edge_routes.push(route);
        }
    }
    Ok(())
}

pub(super) fn reroute_incident_sequence_messages(
    diagram: &mut BehaviorDiagram,
    repository: &BehaviorRepository,
    lifeline_id_value: &str,
) -> Result<(), String> {
    if diagram.kind != BehaviorDiagramKind::Sequence {
        return Err("active behavior diagram is not a Sequence Diagram".into());
    }
    let interaction_id = parse_uuid(&diagram.semantic_id)
        .map(systems_modeler_core::behavior::InteractionId)?;
    let mut filtered = repository.clone();
    let interaction = filtered
        .interactions
        .get_mut(&interaction_id)
        .ok_or("Interaction not found")?;
    interaction.messages.retain(|message| {
        message
            .send_event
            .as_ref()
            .is_some_and(|event| event.lifeline_id.to_string() == lifeline_id_value)
            || message
                .receive_event
                .as_ref()
                .is_some_and(|event| event.lifeline_id.to_string() == lifeline_id_value)
    });
    if interaction.messages.is_empty() {
        return Ok(());
    }
    let routes = sequence_routes(diagram, &filtered, None)?;
    for route in routes {
        if let Some(existing) = diagram
            .edge_routes
            .iter_mut()
            .find(|existing| existing.semantic_id == route.semantic_id)
        {
            *existing = route;
        } else {
            diagram.edge_routes.push(route);
        }
    }
    Ok(())
}
'''
if helper_marker not in text:
    raise SystemExit("behavior reroute helper marker changed")
text = text.replace(helper_marker, helpers, 1)

move_state_start = "#[tauri::command]\npub fn move_state_vertex("
move_state_end = "#[tauri::command]\npub fn behavior_lifeline_candidates("
move_state_impl = r'''#[tauri::command]
pub fn move_state_vertex(
    diagram_id: String,
    state_vertex_id: String,
    x: f64,
    y: f64,
    state: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<(), String> {
    let mut diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    let presentation = diagram
        .state_nodes
        .iter_mut()
        .find(|node| node.vertex_id == state_vertex_id)
        .ok_or("State presentation not found")?;
    presentation.x = x;
    presentation.y = y;
    let repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    reroute_incident_state_transitions(diagram, &repository, &state_vertex_id)?;
    drop(repository);
    history::checkpoint_states(&state, &activity, &history)?;
    *state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")? = diagrams;
    Ok(())
}

'''
text = replace_between(text, move_state_start, move_state_end, move_state_impl)

move_seq_start = "#[tauri::command]\npub fn move_sequence_lifeline("
move_seq_end = "#[tauri::command]\npub fn resize_sequence_lifeline_timeline("
move_seq_impl = r'''#[tauri::command]
pub fn move_sequence_lifeline(
    diagram_id: String,
    lifeline_id_value: String,
    x: f64,
    state: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<(), String> {
    if !x.is_finite() {
        return Err("Lifeline x coordinate must be finite".into());
    }
    let mut diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    let presentation = diagram
        .lifelines
        .iter_mut()
        .find(|item| item.lifeline_id == lifeline_id_value)
        .ok_or("Lifeline presentation not found")?;
    presentation.x = x.max(70.0);
    let repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    reroute_incident_sequence_messages(diagram, &repository, &lifeline_id_value)?;
    drop(repository);
    history::checkpoint_states(&state, &activity, &history)?;
    *state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")? = diagrams;
    Ok(())
}

'''
text = replace_between(text, move_seq_start, move_seq_end, move_seq_impl)
write(path, text)


# ---------------------------------------------------------------------------
# Parametric geometry: route only bindings incident to the manipulated node or
# parameter, and do not reject a local presentation edit because another diagram
# has stale metadata.
# ---------------------------------------------------------------------------
path = "apps/desktop/src-tauri/src/workspace/parametrics.rs"
text = read(path)
text = text.replace(
    "use std::collections::HashMap;\n",
    "use std::collections::{HashMap, HashSet};\n",
    1,
)
insert_marker = "pub(super) fn route_parametric_with_bounds("
incident_helper = r'''fn reroute_incident_edges(
    diagram: &mut BddDiagram,
    affected_presentation_ids: &HashSet<String>,
) -> Result<(), String> {
    let mut routing_diagram = diagram.clone();
    routing_diagram.edges.retain(|edge| {
        affected_presentation_ids.contains(&edge.source_node_id)
            || affected_presentation_ids.contains(&edge.target_node_id)
    });
    if routing_diagram.edges.is_empty() {
        return Ok(());
    }
    let routed = routed_edges(&routing_diagram, None)?;
    for routed_edge in routed {
        if let Some(edge) = diagram
            .edges
            .iter_mut()
            .find(|edge| edge.id == routed_edge.id)
        {
            *edge = routed_edge;
        }
    }
    Ok(())
}

'''
if insert_marker not in text:
    raise SystemExit("parametric route marker missing")
text = text.replace(insert_marker, incident_helper + insert_marker, 1)

geom_start = "#[tauri::command]\n#[allow(clippy::too_many_arguments)]\npub fn update_parametric_presentation_geometry("
geom_end = "#[tauri::command]\n#[allow(clippy::too_many_arguments)]\npub fn update_constraint_parameter_presentation("
geom_impl = r'''#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_parametric_presentation_geometry(
    diagram_id: String,
    presentation_id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    if ![x, y, width, height].iter().all(|value| value.is_finite()) || width < 80.0 || height < 50.0
    {
        return Err("Parametric presentation geometry is invalid".into());
    }
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id && diagram.family == "parametric")
        .ok_or("Parametric Diagram not found")?;
    let affected = {
        let node = diagram
            .nodes
            .iter_mut()
            .find(|node| node.id == presentation_id)
            .ok_or("Parametric presentation not found")?;
        node.x = x.max(0.0);
        node.y = y.max(42.0);
        node.width = width;
        node.height = height;
        sync_parameter_presentations(node, &project)?;
        let mut affected = HashSet::from([node.id.clone()]);
        affected.extend(
            node.parameter_presentations
                .iter()
                .map(|parameter| parameter.id.clone()),
        );
        affected
    };
    reroute_incident_edges(diagram, &affected)?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(())
}

'''
text = replace_between(text, geom_start, geom_end, geom_impl)

param_start = "#[tauri::command]\n#[allow(clippy::too_many_arguments)]\npub fn update_constraint_parameter_presentation("
param_end = "#[tauri::command]\npub fn evaluate_parametric_diagram("
param_impl = r'''#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_constraint_parameter_presentation(
    diagram_id: String,
    presentation_id: String,
    offset_x: f64,
    offset_y: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    if !offset_x.is_finite() || !offset_y.is_finite() {
        return Err("parameter position must be finite".into());
    }
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id && diagram.family == "parametric")
        .ok_or("Parametric Diagram not found")?;
    {
        let node = diagram
            .nodes
            .iter_mut()
            .find(|node| {
                node.parameter_presentations
                    .iter()
                    .any(|parameter| parameter.id == presentation_id)
            })
            .ok_or("ConstraintParameter presentation not found")?;
        let parameter = node
            .parameter_presentations
            .iter_mut()
            .find(|parameter| parameter.id == presentation_id)
            .ok_or("ConstraintParameter presentation not found")?;
        let max_x = (node.width - parameter.size).max(0.0);
        let max_y = (node.height - parameter.size).max(0.0);
        let x = offset_x.clamp(0.0, max_x);
        let y = offset_y.clamp(0.0, max_y);
        let distances = [x, max_x - x, y, max_y - y];
        match distances
            .iter()
            .enumerate()
            .min_by(|left, right| left.1.total_cmp(right.1))
            .map(|(index, _)| index)
            .unwrap_or(0)
        {
            0 => {
                parameter.offset_x = 0.0;
                parameter.offset_y = y;
            }
            1 => {
                parameter.offset_x = max_x;
                parameter.offset_y = y;
            }
            2 => {
                parameter.offset_x = x;
                parameter.offset_y = 0.0;
            }
            _ => {
                parameter.offset_x = x;
                parameter.offset_y = max_y;
            }
        }
    }
    reroute_incident_edges(diagram, &HashSet::from([presentation_id]))?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(())
}

'''
text = replace_between(text, param_start, param_end, param_impl)
write(path, text)


# ---------------------------------------------------------------------------
# Stronger architecture contract: direct presentation manipulation across all
# families must share the gesture lifecycle and incident-only routing policy.
# ---------------------------------------------------------------------------
path = "scripts/validate_presentation_interaction.py"
text = read(path)
text = text.replace(
    'ibd_frontend = read("apps/desktop/frontend/ibd-ui.js")\n',
    'ibd_frontend = read("apps/desktop/frontend/ibd-ui.js")\nparametric_frontend = read("apps/desktop/frontend/parametric-ui.js")\n',
    1,
)
needle = '''assert "resize_sequence_lifeline_timeline" in sequence_frontend
'''
extra = '''assert "resize_sequence_lifeline_timeline" in sequence_frontend
assert "smpBeginPresentationGesture" in sequence_frontend
assert "smpPreviewSequenceLifelineGeometry" in sequence_frontend
assert "bindSequenceConnectedDrag" not in runtime_fixes
assert "smpPreviewSequenceLifelineGeometry" in runtime_fixes
assert "setAttribute('points'" in runtime_fixes
assert "smpBeginPresentationGesture" in state_bar_frontend
assert "smpBeginPresentationGesture" in parametric_frontend
activity_geometry = frontend.split("function installActivity", 1)[1].split("function install()", 1)[0]
assert "beginPointerGesture(event" in activity_geometry
assert ".onpointermove" not in activity_geometry
assert ".onpointerup" not in activity_geometry
'''
if needle not in text:
    raise SystemExit("presentation validator preview marker changed")
text = text.replace(needle, extra, 1)
needle = '''assert "orthogonal_route" in interaction_rs, "Activity rerouting is not integrated"
'''
extra = '''assert "orthogonal_route" in interaction_rs, "Activity rerouting is not integrated"
assert "edge.source_node_id == presentation_id || edge.target_node_id == presentation_id" in interaction_rs
assert "reroute_incident_state_transitions" in interaction_rs
assert "reroute_incident_state_transitions" in behavior_rs
assert "reroute_incident_sequence_messages" in behavior_rs
assert "history::checkpoint_states(&state, &activity, &history)?;" in behavior_rs
'''
text = text.replace(needle, extra, 1)
write(path, text)

# Parametric integration contract: direct geometry must not invoke a whole
# diagram-set validation/reroute after a local pointer gesture.
path = "scripts/validate_parametric_integration.py"
text = read(path)
if 'parametrics_rs = read("apps/desktop/src-tauri/src/workspace/parametrics.rs")' not in text:
    # Most versions use a differently named variable; append a self-contained check.
    text += '''\n# Cross-family direct-manipulation contract\nparametric_geometry_rs = read("apps/desktop/src-tauri/src/workspace/parametrics.rs")\nassert "fn reroute_incident_edges" in parametric_geometry_rs\nparametric_geometry = parametric_geometry_rs.split("pub fn update_parametric_presentation_geometry", 1)[1].split("pub fn update_constraint_parameter_presentation", 1)[0]\nassert "reroute_incident_edges" in parametric_geometry\nassert "validate_loaded_diagrams" not in parametric_geometry\nparameter_geometry = parametric_geometry_rs.split("pub fn update_constraint_parameter_presentation", 1)[1].split("pub fn evaluate_parametric_diagram", 1)[0]\nassert "reroute_incident_edges" in parameter_geometry\nassert "validate_loaded_diagrams" not in parameter_geometry\n'''
write(path, text)

print("PR35 cross-family geometry closure applied")
