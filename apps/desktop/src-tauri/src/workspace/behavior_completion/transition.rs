use super::super::behavior_workspace::BehaviorDiagramKind;
use super::super::{WorkspaceState, parse_element_id};
use systems_modeler_core::Project;
use systems_modeler_core::behavior::{
    Event, Region, RegionId, StateMachineId, Transition, TransitionId, TransitionKind, Trigger,
    VertexId, VertexKind,
};

fn parse_uuid(value: &str) -> Result<uuid::Uuid, String> {
    uuid::Uuid::parse_str(value).map_err(|_| format!("invalid behavior id: {value}"))
}

fn vertex_id(value: &str) -> Result<VertexId, String> {
    parse_uuid(value).map(VertexId)
}

fn project_snapshot(state: &WorkspaceState) -> Result<Project, String> {
    state
        .project
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .clone()
        .ok_or_else(|| "no project open".to_string())
}

fn machine_id(state: &WorkspaceState, diagram_id: &str) -> Result<StateMachineId, String> {
    let diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    if diagram.kind != BehaviorDiagramKind::StateMachine {
        return Err("active behavior diagram is not a State Machine".into());
    }
    parse_uuid(&diagram.semantic_id).map(StateMachineId)
}

fn parse_transition_kind(value: &str) -> Result<TransitionKind, String> {
    match value {
        "External" => Ok(TransitionKind::External),
        "Internal" => Ok(TransitionKind::Internal),
        "Local" => Ok(TransitionKind::Local),
        _ => Err(format!("unsupported transition kind: {value}")),
    }
}

fn trigger_from_input(
    event_kind: Option<String>,
    event_reference_id: Option<String>,
    event_expression: Option<String>,
) -> Result<Option<Trigger>, String> {
    let Some(kind) = event_kind.filter(|value| value != "None") else {
        return Ok(None);
    };
    let event = match kind.as_str() {
        "Signal" => Event::Signal {
            signal_id: parse_element_id(
                event_reference_id
                    .as_deref()
                    .ok_or("Signal trigger requires a Signal")?,
            )?,
        },
        "Call" => Event::Call {
            operation_id: parse_element_id(
                event_reference_id
                    .as_deref()
                    .ok_or("Call trigger requires an Operation")?,
            )?,
        },
        "Time" => Event::Time {
            expression: event_expression.unwrap_or_default(),
            is_relative: true,
        },
        "Change" => Event::Change {
            expression: event_expression.unwrap_or_default(),
        },
        "AnyReceive" => Event::AnyReceive,
        _ => return Err(format!("unsupported trigger event: {kind}")),
    };
    Ok(Some(Trigger { event }))
}

fn containing_region(regions: &[Region], wanted: VertexId) -> Option<RegionId> {
    for region in regions {
        if region.vertices.iter().any(|vertex| vertex.id == wanted) {
            return Some(region.id);
        }
        for vertex in &region.vertices {
            if let VertexKind::State(state) = &vertex.kind
                && let Some(id) = containing_region(&state.regions, wanted)
            {
                return Some(id);
            }
        }
    }
    None
}

fn region_mut(regions: &mut [Region], wanted: RegionId) -> Option<&mut Region> {
    for region in regions {
        if region.id == wanted {
            return Some(region);
        }
        for vertex in &mut region.vertices {
            if let VertexKind::State(state) = &mut vertex.kind
                && let Some(found) = region_mut(&mut state.regions, wanted)
            {
                return Some(found);
            }
        }
    }
    None
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn add_state_transition_complete(
    diagram_id: String,
    source_vertex_id: String,
    target_vertex_id: String,
    kind: String,
    event_kind: Option<String>,
    event_reference_id: Option<String>,
    event_expression: Option<String>,
    guard: Option<String>,
    effect: Option<String>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let project = project_snapshot(&state)?;
    let machine_id = machine_id(&state, &diagram_id)?;
    let source_id = vertex_id(&source_vertex_id)?;
    let target_id = vertex_id(&target_vertex_id)?;
    let transition = Transition {
        id: TransitionId::new(),
        source_id,
        target_id,
        kind: parse_transition_kind(&kind)?,
        trigger: trigger_from_input(event_kind, event_reference_id, event_expression)?,
        guard: guard.filter(|value| !value.trim().is_empty()),
        effect: effect.filter(|value| !value.trim().is_empty()),
    };
    let id = transition.id;

    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let machine = repository
        .state_machines
        .get_mut(&machine_id)
        .ok_or("State Machine not found")?;
    let source_region = containing_region(&machine.regions, source_id)
        .ok_or("Transition source vertex is not owned by this State Machine")?;
    let target_region = containing_region(&machine.regions, target_id)
        .ok_or("Transition target vertex is not owned by this State Machine")?;
    if source_region != target_region {
        return Err("Transition endpoints are in different Regions. Model the boundary crossing through the owning composite State and appropriate Entry/Exit Points instead of creating an illegally owned cross-Region Transition.".into());
    }
    region_mut(&mut machine.regions, source_region)
        .ok_or("Transition owning Region not found")?
        .transitions
        .push(transition);
    if let Err(error) = systems_modeler_core::behavior::validate_state_machine(&project, machine) {
        if let Some(region) = region_mut(&mut machine.regions, source_region) {
            region.transitions.retain(|item| item.id != id);
        }
        return Err(error.to_string());
    }
    Ok(id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use systems_modeler_core::behavior::{State, Vertex};

    #[test]
    fn containing_region_finds_nested_vertex_owner() {
        let child_id = RegionId::new();
        let child_vertex = Vertex {
            id: VertexId::new(),
            name: "Nested".into(),
            kind: VertexKind::State(State::default()),
        };
        let wanted = child_vertex.id;
        let regions = vec![Region {
            id: RegionId::new(),
            name: "root".into(),
            vertices: vec![Vertex {
                id: VertexId::new(),
                name: "Composite".into(),
                kind: VertexKind::State(State {
                    entry: None,
                    do_activity: None,
                    exit: None,
                    regions: vec![Region {
                        id: child_id,
                        name: "child".into(),
                        vertices: vec![child_vertex],
                        transitions: Vec::new(),
                    }],
                }),
            }],
            transitions: Vec::new(),
        }];
        assert_eq!(containing_region(&regions, wanted), Some(child_id));
    }
}
