use super::super::behavior_workspace::{BehaviorDiagramKind, StateNodePresentation};
use super::super::{WorkspaceState, parse_element_id};
use systems_modeler_core::Project;
use systems_modeler_core::behavior::{
    Event, Region, RegionId, State, StateMachineId, TransitionId, TransitionKind, Trigger, Vertex,
    VertexId, VertexKind,
};

fn parse_uuid(value: &str) -> Result<uuid::Uuid, String> {
    uuid::Uuid::parse_str(value).map_err(|_| format!("invalid behavior id: {value}"))
}

fn region_id(value: &str) -> Result<RegionId, String> {
    parse_uuid(value).map(RegionId)
}

fn state_machine_id(value: &str) -> Result<StateMachineId, String> {
    parse_uuid(value).map(StateMachineId)
}

fn transition_id(value: &str) -> Result<TransitionId, String> {
    parse_uuid(value).map(TransitionId)
}

fn project_snapshot(state: &WorkspaceState) -> Result<Project, String> {
    state
        .project
        .lock()
        .map_err(|_| "project lock poisoned".to_string())?
        .clone()
        .ok_or_else(|| "no project open".to_string())
}

fn behavior_semantic_id(state: &WorkspaceState, diagram_id: &str) -> Result<String, String> {
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
    Ok(diagram.semantic_id.clone())
}

fn parse_transition_kind(value: &str) -> Result<TransitionKind, String> {
    match value {
        "External" => Ok(TransitionKind::External),
        "Internal" => Ok(TransitionKind::Internal),
        "Local" => Ok(TransitionKind::Local),
        _ => Err(format!("unsupported transition kind: {value}")),
    }
}

fn required_expression(value: Option<String>, event_kind: &str) -> Result<String, String> {
    let expression = value.unwrap_or_default();
    if expression.trim().is_empty() {
        return Err(format!(
            "{event_kind} trigger requires a non-empty expression before the Transition can be updated"
        ));
    }
    Ok(expression)
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
            expression: required_expression(event_expression, "Time")?,
            is_relative: true,
        },
        "Change" => Event::Change {
            expression: required_expression(event_expression, "Change")?,
        },
        "AnyReceive" => Event::AnyReceive,
        _ => return Err(format!("unsupported trigger event: {kind}")),
    };
    Ok(Some(Trigger { event }))
}

fn find_region_mut(regions: &mut [Region], wanted: RegionId) -> Option<&mut Region> {
    for region in regions {
        if region.id == wanted {
            return Some(region);
        }
        for vertex in &mut region.vertices {
            if let VertexKind::State(state) = &mut vertex.kind
                && let Some(found) = find_region_mut(&mut state.regions, wanted)
            {
                return Some(found);
            }
        }
    }
    None
}

fn find_transition_mut(
    regions: &mut [Region],
    wanted: TransitionId,
) -> Option<&mut systems_modeler_core::behavior::Transition> {
    for region in regions {
        if let Some(transition) = region.transitions.iter_mut().find(|item| item.id == wanted) {
            return Some(transition);
        }
        for vertex in &mut region.vertices {
            if let VertexKind::State(state) = &mut vertex.kind
                && let Some(found) = find_transition_mut(&mut state.regions, wanted)
            {
                return Some(found);
            }
        }
    }
    None
}

#[tauri::command]
pub fn add_composite_state(
    diagram_id: String,
    region_id_value: Option<String>,
    name: String,
    orthogonal: bool,
    x: f64,
    y: f64,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("Composite State requires a name".into());
    }
    let semantic_id = behavior_semantic_id(&state, &diagram_id)?;
    let machine_id = state_machine_id(&semantic_id)?;
    let id = VertexId::new();
    {
        let mut repository = state
            .behavior
            .lock()
            .map_err(|_| "behavior lock poisoned")?;
        let machine = repository
            .state_machines
            .get_mut(&machine_id)
            .ok_or("State Machine not found")?;
        let target_region = match region_id_value {
            Some(value) => region_id(&value)?,
            None => machine
                .regions
                .first()
                .map(|region| region.id)
                .ok_or("State Machine has no root Region")?,
        };
        let region =
            find_region_mut(&mut machine.regions, target_region).ok_or("Region not found")?;
        let mut regions = vec![Region {
            id: RegionId::new(),
            name: "Region 1".into(),
            vertices: Vec::new(),
            transitions: Vec::new(),
        }];
        if orthogonal {
            regions.push(Region {
                id: RegionId::new(),
                name: "Region 2".into(),
                vertices: Vec::new(),
                transitions: Vec::new(),
            });
        }
        region.vertices.push(Vertex {
            id,
            name,
            kind: VertexKind::State(State {
                regions,
                ..State::default()
            }),
        });
    }
    let mut diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    diagram.state_nodes.push(StateNodePresentation {
        vertex_id: id.to_string(),
        x,
        y,
        width: 280.0,
        height: if orthogonal { 220.0 } else { 170.0 },
    });
    Ok(id.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn add_submachine_state(
    diagram_id: String,
    region_id_value: Option<String>,
    name: String,
    submachine_id_value: String,
    x: f64,
    y: f64,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    if name.trim().is_empty() {
        return Err("Submachine State requires a name".into());
    }
    let project = project_snapshot(&state)?;
    let semantic_id = behavior_semantic_id(&state, &diagram_id)?;
    let machine_id = state_machine_id(&semantic_id)?;
    let submachine_id = state_machine_id(&submachine_id_value)?;
    if machine_id == submachine_id {
        return Err("A State Machine cannot reference itself as a Submachine State".into());
    }
    let id = VertexId::new();
    let target_region;
    {
        let mut repository = state
            .behavior
            .lock()
            .map_err(|_| "behavior lock poisoned")?;
        if !repository.state_machines.contains_key(&submachine_id) {
            return Err("Submachine State must reference an existing State Machine".into());
        }
        target_region = {
            let machine = repository
                .state_machines
                .get(&machine_id)
                .ok_or("State Machine not found")?;
            match region_id_value.as_deref() {
                Some(value) => region_id(value)?,
                None => machine
                    .regions
                    .first()
                    .map(|region| region.id)
                    .ok_or("State Machine has no root Region")?,
            }
        };
        let machine = repository
            .state_machines
            .get_mut(&machine_id)
            .ok_or("State Machine not found")?;
        let region =
            find_region_mut(&mut machine.regions, target_region).ok_or("Region not found")?;
        region.vertices.push(Vertex {
            id,
            name,
            kind: VertexKind::State(State {
                submachine: Some(submachine_id),
                ..State::default()
            }),
        });
        if let Err(error) = repository.validate(&project) {
            let machine = repository
                .state_machines
                .get_mut(&machine_id)
                .ok_or("State Machine not found")?;
            let region =
                find_region_mut(&mut machine.regions, target_region).ok_or("Region not found")?;
            region.vertices.retain(|vertex| vertex.id != id);
            return Err(error.to_string());
        }
    }
    let mut diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    diagram.state_nodes.push(StateNodePresentation {
        vertex_id: id.to_string(),
        x,
        y,
        width: 190.0,
        height: 90.0,
    });
    Ok(id.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_state_transition(
    diagram_id: String,
    transition_id_value: String,
    kind: String,
    event_kind: Option<String>,
    event_reference_id: Option<String>,
    event_expression: Option<String>,
    guard: Option<String>,
    effect: Option<String>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let project = project_snapshot(&state)?;
    let semantic_id = behavior_semantic_id(&state, &diagram_id)?;
    let machine_id = state_machine_id(&semantic_id)?;
    let wanted = transition_id(&transition_id_value)?;
    let parsed_kind = parse_transition_kind(&kind)?;
    let trigger = trigger_from_input(event_kind, event_reference_id, event_expression)?;
    let guard = guard.filter(|value| !value.trim().is_empty());
    let effect = effect.filter(|value| !value.trim().is_empty());
    let mut repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let machine = repository
        .state_machines
        .get_mut(&machine_id)
        .ok_or("State Machine not found")?;
    let transition =
        find_transition_mut(&mut machine.regions, wanted).ok_or("Transition not found")?;
    let original = transition.clone();
    transition.kind = parsed_kind;
    transition.trigger = trigger;
    transition.guard = guard;
    transition.effect = effect;
    if let Err(error) = systems_modeler_core::behavior::validate_state_machine(&project, machine) {
        *find_transition_mut(&mut machine.regions, wanted).ok_or("Transition not found")? =
            original;
        return Err(error.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transition_kind_parser_covers_all_uml_kinds() {
        for value in ["External", "Internal", "Local"] {
            assert!(parse_transition_kind(value).is_ok());
        }
    }

    #[test]
    fn trigger_parser_rejects_unknown_event_kind() {
        assert!(trigger_from_input(Some("Bogus".into()), None, None).is_err());
    }

    #[test]
    fn trigger_parser_rejects_blank_time_and_change_expressions() {
        assert!(trigger_from_input(Some("Time".into()), None, Some(" ".into())).is_err());
        assert!(trigger_from_input(Some("Change".into()), None, None).is_err());
    }
}
