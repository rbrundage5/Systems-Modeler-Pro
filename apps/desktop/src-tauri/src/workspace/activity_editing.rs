use super::*;
use systems_modeler_core::{
    Action, ActionKind, ActivityNode, ActivityNodeId, ActivityNodeKind, ActivityParameterNode,
    ActivityPartition, ActivityPartitionId, ElementKind, ParameterDirection, Pin,
    StructuredActivityNode, StructuredActivityNodeKind, StructuredNodeId,
};

fn activity_for_diagram<'a>(
    repository: &'a mut systems_modeler_core::ActivityRepository,
    diagram: &activity_workspace::ActivityDiagram,
) -> Result<&'a mut systems_modeler_core::Activity, String> {
    let activity_id = activity_workspace::parse_activity_id(&diagram.activity_id)?;
    repository
        .activities
        .get_mut(&activity_id)
        .ok_or_else(|| "Activity not found".to_string())
}

fn operation_pins(project: &Project, operation_id: ElementId) -> Result<Vec<Pin>, String> {
    let operation = project
        .element(operation_id)
        .map_err(|error| error.to_string())?;
    if operation.kind != ElementKind::Operation {
        return Err("CallOperationAction requires an Operation stable ID".into());
    }
    let mut parameters: Vec<_> = project
        .elements
        .values()
        .filter(|element| {
            element.kind == ElementKind::Parameter && element.owner_id == Some(operation_id)
        })
        .collect();
    parameters.sort_by(|a, b| a.name.cmp(&b.name));
    let mut pins = Vec::with_capacity(parameters.len());
    for parameter in parameters {
        let mut pin = match parameter.parameter_direction {
            Some(ParameterDirection::Out | ParameterDirection::Return) => {
                Pin::output(parameter.name.clone(), parameter.type_id)
            }
            Some(ParameterDirection::In | ParameterDirection::InOut) | None => {
                Pin::input(parameter.name.clone(), parameter.type_id)
            }
        };
        pin.parameter_id = Some(parameter.id);
        pin.multiplicity = parameter.multiplicity.unwrap_or(Multiplicity::ONE);
        pins.push(pin);
    }
    Ok(pins)
}

fn push_presented_node(
    diagram: &mut activity_workspace::ActivityDiagram,
    node: &ActivityNode,
    x: f64,
    y: f64,
) {
    let (width, height) = activity_workspace::activity_node_size(&node.kind);
    diagram.nodes.push(activity_workspace::ActivityDiagramNode {
        id: uuid::Uuid::new_v4().to_string(),
        activity_node_id: node.id.to_string(),
        x,
        y,
        width,
        height,
    });
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn add_activity_action(
    diagram_id: String,
    kind: String,
    name: String,
    reference_id: Option<String>,
    expression: Option<String>,
    x: f64,
    y: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
) -> Result<String, String> {
    let project_guard = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;
    let mut diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("Activity diagram not found")?;
    let mut repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;

    let action = match kind.as_str() {
        "CallBehaviorAction" => {
            let reference =
                reference_id.ok_or("CallBehaviorAction requires an Activity stable ID")?;
            Action {
                kind: ActionKind::CallBehavior {
                    activity_id: activity_workspace::parse_activity_id(&reference)?,
                },
                pins: Vec::new(),
            }
        }
        "CallOperationAction" => {
            let reference =
                reference_id.ok_or("CallOperationAction requires an Operation stable ID")?;
            let operation_id = parse_element_id(&reference)?;
            Action {
                kind: ActionKind::CallOperation { operation_id },
                pins: operation_pins(project, operation_id)?,
            }
        }
        "SendSignalAction" => {
            let signal_id = parse_element_id(
                &reference_id.ok_or("SendSignalAction requires a Signal stable ID")?,
            )?;
            Action {
                kind: ActionKind::SendSignal { signal_id },
                pins: Vec::new(),
            }
        }
        "AcceptEventAction" => Action {
            kind: ActionKind::AcceptEvent {
                signal_id: reference_id.as_deref().map(parse_element_id).transpose()?,
            },
            pins: Vec::new(),
        },
        "AcceptTimeEventAction" => Action {
            kind: ActionKind::AcceptTimeEvent {
                expression: expression.unwrap_or_default(),
            },
            pins: Vec::new(),
        },
        _ => return Err(format!("unsupported Activity action kind: {kind}")),
    };

    let node = ActivityNode {
        id: ActivityNodeId::new(),
        name,
        kind: ActivityNodeKind::Action(action),
        partition_id: None,
        structured_node_id: None,
    };
    let id = node.id;
    activity_for_diagram(&mut repository, diagram)?
        .nodes
        .push(node.clone());
    repository
        .validate(project)
        .map_err(|error| error.to_string())?;
    push_presented_node(diagram, &node, x, y);
    Ok(id.to_string())
}

#[tauri::command]
pub fn add_activity_parameter_node(
    diagram_id: String,
    parameter_id: String,
    x: f64,
    y: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
) -> Result<String, String> {
    let parameter_id = parse_element_id(&parameter_id)?;
    let project_guard = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;
    if project
        .element(parameter_id)
        .map_err(|error| error.to_string())?
        .kind
        != ElementKind::Parameter
    {
        return Err("ActivityParameterNode requires a Parameter stable ID".into());
    }
    let mut diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("Activity diagram not found")?;
    let mut repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let node = ActivityNode {
        id: ActivityNodeId::new(),
        name: project
            .element(parameter_id)
            .map_err(|error| error.to_string())?
            .name
            .clone(),
        kind: ActivityNodeKind::ActivityParameter(ActivityParameterNode { parameter_id }),
        partition_id: None,
        structured_node_id: None,
    };
    let id = node.id;
    activity_for_diagram(&mut repository, diagram)?
        .nodes
        .push(node.clone());
    repository
        .validate(project)
        .map_err(|error| error.to_string())?;
    push_presented_node(diagram, &node, x, y);
    Ok(id.to_string())
}

#[tauri::command]
pub fn add_activity_partition(
    diagram_id: String,
    name: String,
    represented_element_id: Option<String>,
    is_dimension: bool,
    is_external: bool,
    activity_state: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
) -> Result<String, String> {
    let represented_element_id = represented_element_id
        .as_deref()
        .map(parse_element_id)
        .transpose()?;
    let diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("Activity diagram not found")?;
    let mut repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let partition = ActivityPartition {
        id: ActivityPartitionId::new(),
        name,
        represented_element_id,
        is_dimension,
        is_external,
    };
    let id = partition.id;
    activity_for_diagram(&mut repository, diagram)?
        .partitions
        .push(partition);
    Ok(id.to_string())
}

#[tauri::command]
pub fn assign_activity_node_partition(
    diagram_id: String,
    activity_node_id: String,
    partition_id: Option<String>,
    activity_state: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
) -> Result<(), String> {
    let node_id = activity_workspace::parse_activity_node_id(&activity_node_id)?;
    let partition_id = partition_id
        .as_deref()
        .map(|value| {
            uuid::Uuid::parse_str(value)
                .map(ActivityPartitionId)
                .map_err(|_| format!("invalid Activity partition id: {value}"))
        })
        .transpose()?;
    let diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("Activity diagram not found")?;
    let mut repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let activity = activity_for_diagram(&mut repository, diagram)?;
    if partition_id.is_some_and(|id| {
        !activity
            .partitions
            .iter()
            .any(|partition| partition.id == id)
    }) {
        return Err("Activity partition is not owned by this Activity".into());
    }
    let node = activity
        .nodes
        .iter_mut()
        .find(|node| node.id == node_id)
        .ok_or("Activity node not found")?;
    node.partition_id = partition_id;
    Ok(())
}

#[tauri::command]
pub fn add_structured_activity_node(
    diagram_id: String,
    kind: String,
    name: String,
    parent_id: Option<String>,
    activity_state: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
) -> Result<String, String> {
    let kind = match kind.as_str() {
        "StructuredActivityNode" => StructuredActivityNodeKind::Structured,
        "ConditionalNode" => StructuredActivityNodeKind::Conditional,
        "LoopNode" => StructuredActivityNodeKind::Loop,
        "SequenceNode" => StructuredActivityNodeKind::Sequence,
        "ExpansionRegion" => StructuredActivityNodeKind::ExpansionRegion,
        "InterruptibleActivityRegion" => StructuredActivityNodeKind::InterruptibleRegion,
        _ => return Err(format!("unsupported structured Activity node kind: {kind}")),
    };
    let parent_id = parent_id
        .as_deref()
        .map(|value| {
            uuid::Uuid::parse_str(value)
                .map(StructuredNodeId)
                .map_err(|_| format!("invalid structured Activity node id: {value}"))
        })
        .transpose()?;
    let diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("Activity diagram not found")?;
    let mut repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let structured = StructuredActivityNode {
        id: StructuredNodeId::new(),
        name,
        kind,
        parent_id,
    };
    let id = structured.id;
    activity_for_diagram(&mut repository, diagram)?
        .structured_nodes
        .push(structured);
    Ok(id.to_string())
}

#[tauri::command]
pub fn assign_activity_node_structured_parent(
    diagram_id: String,
    activity_node_id: String,
    structured_node_id: Option<String>,
    activity_state: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
) -> Result<(), String> {
    let node_id = activity_workspace::parse_activity_node_id(&activity_node_id)?;
    let structured_node_id = structured_node_id
        .as_deref()
        .map(|value| {
            uuid::Uuid::parse_str(value)
                .map(StructuredNodeId)
                .map_err(|_| format!("invalid structured Activity node id: {value}"))
        })
        .transpose()?;
    let diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("Activity diagram not found")?;
    let mut repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let activity = activity_for_diagram(&mut repository, diagram)?;
    if structured_node_id
        .is_some_and(|id| !activity.structured_nodes.iter().any(|node| node.id == id))
    {
        return Err("structured Activity node is not owned by this Activity".into());
    }
    let node = activity
        .nodes
        .iter_mut()
        .find(|node| node.id == node_id)
        .ok_or("Activity node not found")?;
    node.structured_node_id = structured_node_id;
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_activity_node_semantics(
    diagram_id: String,
    activity_node_id: String,
    name: Option<String>,
    opaque_body: Option<String>,
    decision_input: Option<String>,
    join_specification: Option<String>,
    time_expression: Option<String>,
    action_reference_id: Option<String>,
    update_action_reference: bool,
    workspace: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
) -> Result<(), String> {
    let node_id = activity_workspace::parse_activity_node_id(&activity_node_id)?;
    let project_guard = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;
    let diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("Activity diagram not found")?;
    let activity_id = activity_workspace::parse_activity_id(&diagram.activity_id)?;
    let mut repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let original = repository
        .activities
        .get(&activity_id)
        .cloned()
        .ok_or("Activity not found")?;

    let mutation = (|| -> Result<(), String> {
        let activity = repository
            .activities
            .get_mut(&activity_id)
            .ok_or("Activity not found")?;
        let node = activity
            .nodes
            .iter_mut()
            .find(|node| node.id == node_id)
            .ok_or("Activity node not found")?;
        if let Some(name) = name {
            node.name = name;
        }

        if update_action_reference {
            let action = match &mut node.kind {
                ActivityNodeKind::Action(action) => action,
                _ => return Err("only Activity actions can update an action reference".into()),
            };
            match &mut action.kind {
                ActionKind::CallBehavior { activity_id } => {
                    let reference = action_reference_id
                        .as_deref()
                        .ok_or("CallBehaviorAction requires a referenced Activity")?;
                    *activity_id = activity_workspace::parse_activity_id(reference)?;
                }
                ActionKind::CallOperation { operation_id } => {
                    let reference = action_reference_id
                        .as_deref()
                        .ok_or("CallOperationAction requires a referenced Operation")?;
                    let next_operation_id = parse_element_id(reference)?;
                    let next_pins = operation_pins(project, next_operation_id)?;
                    *operation_id = next_operation_id;
                    action.pins = next_pins;
                }
                ActionKind::SendSignal { signal_id } => {
                    let reference = action_reference_id
                        .as_deref()
                        .ok_or("SendSignalAction requires a Signal")?;
                    *signal_id = parse_element_id(reference)?;
                }
                ActionKind::AcceptEvent { signal_id } => {
                    *signal_id = action_reference_id
                        .as_deref()
                        .map(parse_element_id)
                        .transpose()?;
                }
                ActionKind::Opaque { .. } | ActionKind::AcceptTimeEvent { .. } => {
                    return Err("this Activity action does not use a referenced model element".into());
                }
            }
        }

        match &mut node.kind {
            ActivityNodeKind::Action(Action {
                kind: ActionKind::Opaque { body },
                ..
            }) => {
                if let Some(value) = opaque_body {
                    *body = value;
                }
            }
            ActivityNodeKind::Action(Action {
                kind: ActionKind::AcceptTimeEvent { expression },
                ..
            }) => {
                if let Some(value) = time_expression {
                    *expression = value;
                }
            }
            ActivityNodeKind::Decision {
                decision_input: value,
            } if decision_input.is_some() => {
                *value = decision_input.filter(|text| !text.is_empty());
            }
            ActivityNodeKind::Join {
                join_specification: value,
            } if join_specification.is_some() => {
                *value = join_specification.filter(|text| !text.is_empty());
            }
            _ => {}
        }
        Ok(())
    })();

    if let Err(error) = mutation {
        repository.activities.insert(activity_id, original);
        return Err(error);
    }
    if let Err(error) = repository.validate(project) {
        repository.activities.insert(activity_id, original);
        return Err(error.to_string());
    }
    Ok(())
}
