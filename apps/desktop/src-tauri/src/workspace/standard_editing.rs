use super::activity_workspace::{ActivityDiagram, ActivityWorkspaceState};
use super::behavior_workspace::{BehaviorDiagramKind, BehaviorPresentationCopy};
use super::history::{self, HistoryState};
use super::shared_workspace::WorkspaceSelection;
use super::*;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use systems_modeler_core::behavior::{
    ExecutionId, FragmentId, InteractionId, InvariantId, LifelineId, MessageId, OccurrenceId,
    Region, TransitionId, VertexId, VertexKind,
};
use systems_modeler_core::{
    ActivityEdgeId, ActivityEndpoint, ActivityNodeId, ActivityNodeKind, ActivityRepository, ElementId,
    ElementKind, PinId, Project, RelationshipEndId, RelationshipId,
};

const PASTE_OFFSET: f64 = 28.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditingFamily {
    Bdd,
    Requirement,
    Ibd,
    StateMachine,
    Sequence,
    Activity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IbdEndpointLocator {
    element_id: String,
    property_path: Vec<String>,
    port: bool,
    boundary: bool,
}

#[derive(Debug, Clone)]
enum ClipboardItem {
    BddNode {
        node: DiagramNode,
    },
    BddEdge {
        edge: DiagramEdge,
        source_element_id: String,
        target_element_id: String,
    },
    IbdProperty {
        property: ibd::IbdPropertyPresentation,
    },
    IbdPort {
        port: ibd::IbdPortPresentation,
        parent: Option<IbdEndpointLocator>,
    },
    IbdConnector {
        connector: ibd::IbdConnectorPresentation,
        source: IbdEndpointLocator,
        target: IbdEndpointLocator,
    },
    Behavior {
        kind: String,
        semantic_id: String,
    },
    ActivityNode {
        node: activity_workspace::ActivityDiagramNode,
    },
    ActivityEdge {
        edge: activity_workspace::ActivityDiagramEdge,
        source_semantic_id: String,
        target_semantic_id: String,
    },
}

#[derive(Debug, Clone)]
struct ClipboardPayload {
    family: EditingFamily,
    semantic_context_id: Option<String>,
    items: Vec<ClipboardItem>,
}

#[derive(Default)]
pub struct StandardEditingState {
    clipboard: Mutex<Option<ClipboardPayload>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StandardEditingResult {
    pub changed: usize,
    pub selections: Vec<WorkspaceSelection>,
}

#[derive(Clone)]
struct EditingSnapshot {
    project: Project,
    diagrams: Vec<BddDiagram>,
    ibd_diagrams: Vec<ibd::IbdDiagram>,
    behavior: systems_modeler_core::BehaviorRepository,
    behavior_diagrams: Vec<behavior_workspace::BehaviorDiagram>,
    activity: ActivityRepository,
    activity_diagrams: Vec<ActivityDiagram>,
}

impl EditingSnapshot {
    fn capture(workspace: &WorkspaceState, activity: &ActivityWorkspaceState) -> Result<Self, String> {
        Ok(Self {
            project: workspace
                .project
                .lock()
                .map_err(|_| "project lock poisoned")?
                .clone()
                .ok_or("no project open")?,
            diagrams: workspace
                .diagrams
                .lock()
                .map_err(|_| "diagram lock poisoned")?
                .clone(),
            ibd_diagrams: workspace
                .ibd_diagrams
                .lock()
                .map_err(|_| "IBD lock poisoned")?
                .clone(),
            behavior: workspace
                .behavior
                .lock()
                .map_err(|_| "behavior lock poisoned")?
                .clone(),
            behavior_diagrams: workspace
                .behavior_diagrams
                .lock()
                .map_err(|_| "behavior diagram lock poisoned")?
                .clone(),
            activity: activity
                .repository
                .lock()
                .map_err(|_| "Activity repository lock poisoned")?
                .clone(),
            activity_diagrams: activity
                .diagrams
                .lock()
                .map_err(|_| "Activity diagram lock poisoned")?
                .clone(),
        })
    }

    fn validate(&self) -> Result<(), String> {
        self.project.validate().map_err(|error| error.to_string())?;
        validate_loaded_diagrams(&self.project, &self.diagrams)?;
        ibd::validate_ibd_diagrams(&self.project, &self.ibd_diagrams)?;
        behavior_workspace::validate_behavior_workspace(
            &self.project,
            &self.behavior,
            &self.behavior_diagrams,
        )?;
        self.activity
            .validate(&self.project)
            .map_err(|error| error.to_string())?;
        validate_activity_presentations(&self.activity, &self.activity_diagrams)
    }

    fn commit(self, workspace: &WorkspaceState, activity: &ActivityWorkspaceState) -> Result<(), String> {
        *workspace
            .project
            .lock()
            .map_err(|_| "project lock poisoned")? = Some(self.project);
        *workspace
            .diagrams
            .lock()
            .map_err(|_| "diagram lock poisoned")? = self.diagrams;
        *workspace
            .ibd_diagrams
            .lock()
            .map_err(|_| "IBD lock poisoned")? = self.ibd_diagrams;
        *workspace
            .behavior
            .lock()
            .map_err(|_| "behavior lock poisoned")? = self.behavior;
        *workspace
            .behavior_diagrams
            .lock()
            .map_err(|_| "behavior diagram lock poisoned")? = self.behavior_diagrams;
        *activity
            .repository
            .lock()
            .map_err(|_| "Activity repository lock poisoned")? = self.activity;
        *activity
            .diagrams
            .lock()
            .map_err(|_| "Activity diagram lock poisoned")? = self.activity_diagrams;
        Ok(())
    }
}

fn validate_activity_presentations(
    repository: &ActivityRepository,
    diagrams: &[ActivityDiagram],
) -> Result<(), String> {
    for diagram in diagrams {
        let activity_id = activity_workspace::parse_activity_id(&diagram.activity_id)?;
        let activity = repository
            .activities
            .get(&activity_id)
            .ok_or_else(|| format!("Activity diagram references missing activity: {activity_id}"))?;
        for node in &diagram.nodes {
            let semantic_id = activity_workspace::parse_activity_node_id(&node.activity_node_id)?;
            if !activity.nodes.iter().any(|candidate| candidate.id == semantic_id) {
                return Err(format!("Activity presentation references missing node: {semantic_id}"));
            }
        }
        for edge in &diagram.edges {
            let semantic_id = uuid::Uuid::parse_str(&edge.activity_edge_id)
                .map(ActivityEdgeId)
                .map_err(|_| format!("invalid Activity edge id: {}", edge.activity_edge_id))?;
            if !activity.edges.iter().any(|candidate| candidate.id == semantic_id) {
                return Err(format!("Activity presentation references missing edge: {semantic_id}"));
            }
            if !diagram.nodes.iter().any(|node| node.id == edge.source_node_id)
                || !diagram.nodes.iter().any(|node| node.id == edge.target_node_id)
            {
                return Err("Activity edge presentation references a missing node presentation".into());
            }
        }
    }
    Ok(())
}

fn family_for(diagram_id: &str, snapshot: &EditingSnapshot) -> Result<EditingFamily, String> {
    if let Some(diagram) = snapshot.diagrams.iter().find(|diagram| diagram.id == diagram_id) {
        return Ok(if diagram.family == "requirement" {
            EditingFamily::Requirement
        } else {
            EditingFamily::Bdd
        });
    }
    if snapshot.ibd_diagrams.iter().any(|diagram| diagram.id == diagram_id) {
        return Ok(EditingFamily::Ibd);
    }
    if let Some(diagram) = snapshot
        .behavior_diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
    {
        return Ok(match diagram.kind {
            BehaviorDiagramKind::StateMachine => EditingFamily::StateMachine,
            BehaviorDiagramKind::Sequence => EditingFamily::Sequence,
        });
    }
    if snapshot
        .activity_diagrams
        .iter()
        .any(|diagram| diagram.id == diagram_id)
    {
        return Ok(EditingFamily::Activity);
    }
    Err("active diagram not found".into())
}

fn require_selections(selections: &[WorkspaceSelection]) -> Result<(), String> {
    if selections.is_empty() {
        Err("Select at least one diagram presentation first".into())
    } else {
        Ok(())
    }
}

fn kind_is(selection: &WorkspaceSelection, values: &[&str]) -> bool {
    values
        .iter()
        .any(|value| selection.kind.eq_ignore_ascii_case(value))
}

fn split_index(value: &str) -> (&str, usize) {
    let Some((semantic, index)) = value.rsplit_once('#') else {
        return (value, 0);
    };
    index.parse::<usize>().map_or((value, 0), |index| (semantic, index))
}

fn ibd_locator(diagram: &ibd::IbdDiagram, presentation_id: &str) -> Option<IbdEndpointLocator> {
    if let Some(port) = diagram
        .boundary_ports
        .iter()
        .find(|port| port.id == presentation_id)
    {
        return Some(IbdEndpointLocator {
            element_id: port.element_id.clone(),
            property_path: port.property_path.clone(),
            port: true,
            boundary: true,
        });
    }
    for property in &diagram.properties {
        if property.id == presentation_id {
            return Some(IbdEndpointLocator {
                element_id: property.element_id.clone(),
                property_path: property.property_path.clone(),
                port: false,
                boundary: false,
            });
        }
        if let Some(port) = property
            .ports
            .iter()
            .find(|port| port.id == presentation_id)
        {
            return Some(IbdEndpointLocator {
                element_id: port.element_id.clone(),
                property_path: port.property_path.clone(),
                port: true,
                boundary: false,
            });
        }
    }
    None
}

fn find_ibd_presentation(diagram: &ibd::IbdDiagram, locator: &IbdEndpointLocator) -> Option<String> {
    if locator.boundary {
        return diagram
            .boundary_ports
            .iter()
            .find(|port| {
                port.element_id == locator.element_id && port.property_path == locator.property_path
            })
            .map(|port| port.id.clone());
    }
    for property in &diagram.properties {
        if !locator.port
            && property.element_id == locator.element_id
            && property.property_path == locator.property_path
        {
            return Some(property.id.clone());
        }
        if locator.port
            && let Some(port) = property.ports.iter().find(|port| {
                port.element_id == locator.element_id && port.property_path == locator.property_path
            })
        {
            return Some(port.id.clone());
        }
    }
    None
}

fn behavior_kind(selection: &WorkspaceSelection, family: EditingFamily) -> String {
    if kind_is(selection, &["BehaviorCopy"]) {
        return "BehaviorCopy".into();
    }
    match family {
        EditingFamily::StateMachine => {
            if kind_is(selection, &["Transition", "StateTransition", "TransitionPresentation"]) {
                "Transition".into()
            } else {
                "Vertex".into()
            }
        }
        EditingFamily::Sequence => {
            for (names, output) in [
                (&["Message", "MessagePresentation"][..], "Message"),
                (&["Execution", "ExecutionSpecification"][..], "Execution"),
                (&["Fragment", "CombinedFragment"][..], "Fragment"),
                (&["Invariant", "StateInvariant"][..], "Invariant"),
                (&["Lifeline", "LifelinePresentation"][..], "Lifeline"),
            ] {
                if kind_is(selection, names) {
                    return output.into();
                }
            }
            "Lifeline".into()
        }
        _ => selection.kind.clone(),
    }
}

fn collect_clipboard(
    snapshot: &EditingSnapshot,
    diagram_id: &str,
    selections: &[WorkspaceSelection],
) -> Result<ClipboardPayload, String> {
    require_selections(selections)?;
    let family = family_for(diagram_id, snapshot)?;
    let mut items = Vec::new();
    let semantic_context_id = match family {
        EditingFamily::Bdd | EditingFamily::Requirement => None,
        EditingFamily::Ibd => snapshot
            .ibd_diagrams
            .iter()
            .find(|diagram| diagram.id == diagram_id)
            .map(|diagram| diagram.context_block_id.clone()),
        EditingFamily::StateMachine | EditingFamily::Sequence => snapshot
            .behavior_diagrams
            .iter()
            .find(|diagram| diagram.id == diagram_id)
            .map(|diagram| diagram.semantic_id.clone()),
        EditingFamily::Activity => snapshot
            .activity_diagrams
            .iter()
            .find(|diagram| diagram.id == diagram_id)
            .map(|diagram| diagram.activity_id.clone()),
    };

    match family {
        EditingFamily::Bdd | EditingFamily::Requirement => {
            let diagram = snapshot
                .diagrams
                .iter()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("diagram not found")?;
            for selection in selections {
                let relationship_first = kind_is(
                    selection,
                    &["BddEdge", "Relationship", "RelationshipPresentation", "Label"],
                );
                if relationship_first
                    && let Some(edge) = diagram.edges.iter().find(|edge| {
                        edge.id == selection.id || edge.relationship_id == selection.id
                    })
                {
                    let source = diagram
                        .nodes
                        .iter()
                        .find(|node| node.id == edge.source_node_id)
                        .ok_or("relationship source presentation not found")?;
                    let target = diagram
                        .nodes
                        .iter()
                        .find(|node| node.id == edge.target_node_id)
                        .ok_or("relationship target presentation not found")?;
                    items.push(ClipboardItem::BddEdge {
                        edge: edge.clone(),
                        source_element_id: source.element_id.clone(),
                        target_element_id: target.element_id.clone(),
                    });
                    continue;
                }
                if let Some(node) = diagram.nodes.iter().find(|node| {
                    node.id == selection.id || node.element_id == selection.id
                }) {
                    items.push(ClipboardItem::BddNode { node: node.clone() });
                    continue;
                }
                if let Some(edge) = diagram.edges.iter().find(|edge| {
                    edge.id == selection.id || edge.relationship_id == selection.id
                }) {
                    let source = diagram
                        .nodes
                        .iter()
                        .find(|node| node.id == edge.source_node_id)
                        .ok_or("relationship source presentation not found")?;
                    let target = diagram
                        .nodes
                        .iter()
                        .find(|node| node.id == edge.target_node_id)
                        .ok_or("relationship target presentation not found")?;
                    items.push(ClipboardItem::BddEdge {
                        edge: edge.clone(),
                        source_element_id: source.element_id.clone(),
                        target_element_id: target.element_id.clone(),
                    });
                    continue;
                }
                return Err(format!("selected BDD/REQ presentation was not found: {}", selection.id));
            }
        }
        EditingFamily::Ibd => {
            let diagram = snapshot
                .ibd_diagrams
                .iter()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("IBD not found")?;
            for selection in selections {
                if kind_is(selection, &["IbdConnector", "Relationship", "Label"])
                    && let Some(connector) = diagram.connectors.iter().find(|edge| {
                        edge.id == selection.id || edge.relationship_id == selection.id
                    })
                {
                    items.push(ClipboardItem::IbdConnector {
                        source: ibd_locator(diagram, &connector.source_presentation_id)
                            .ok_or("IBD source endpoint presentation not found")?,
                        target: ibd_locator(diagram, &connector.target_presentation_id)
                            .ok_or("IBD target endpoint presentation not found")?,
                        connector: connector.clone(),
                    });
                    continue;
                }
                if let Some(property) = diagram.properties.iter().find(|property| {
                    property.id == selection.id || property.element_id == selection.id
                }) {
                    items.push(ClipboardItem::IbdProperty {
                        property: property.clone(),
                    });
                    continue;
                }
                let mut found_port = None;
                for property in &diagram.properties {
                    if let Some(port) = property.ports.iter().find(|port| {
                        port.id == selection.id || port.element_id == selection.id
                    }) {
                        found_port = Some((port.clone(), ibd_locator(diagram, &property.id)));
                        break;
                    }
                }
                if found_port.is_none()
                    && let Some(port) = diagram.boundary_ports.iter().find(|port| {
                        port.id == selection.id || port.element_id == selection.id
                    })
                {
                    found_port = Some((port.clone(), None));
                }
                if let Some((port, parent)) = found_port {
                    items.push(ClipboardItem::IbdPort { port, parent });
                    continue;
                }
                if let Some(connector) = diagram.connectors.iter().find(|edge| {
                    edge.id == selection.id || edge.relationship_id == selection.id
                }) {
                    items.push(ClipboardItem::IbdConnector {
                        source: ibd_locator(diagram, &connector.source_presentation_id)
                            .ok_or("IBD source endpoint presentation not found")?,
                        target: ibd_locator(diagram, &connector.target_presentation_id)
                            .ok_or("IBD target endpoint presentation not found")?,
                        connector: connector.clone(),
                    });
                    continue;
                }
                return Err(format!("selected IBD presentation was not found: {}", selection.id));
            }
        }
        EditingFamily::StateMachine | EditingFamily::Sequence => {
            let diagram = snapshot
                .behavior_diagrams
                .iter()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("behavior diagram not found")?;
            for selection in selections {
                if kind_is(selection, &["BehaviorCopy"])
                    && let Some(copy) = diagram
                        .presentation_copies
                        .iter()
                        .find(|copy| copy.id == selection.id)
                {
                    items.push(ClipboardItem::Behavior {
                        kind: copy.kind.clone(),
                        semantic_id: copy.semantic_id.clone(),
                    });
                    continue;
                }
                let (semantic_id, _) = split_index(&selection.id);
                items.push(ClipboardItem::Behavior {
                    kind: behavior_kind(selection, family),
                    semantic_id: semantic_id.to_string(),
                });
            }
        }
        EditingFamily::Activity => {
            let diagram = snapshot
                .activity_diagrams
                .iter()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("Activity diagram not found")?;
            for selection in selections {
                let edge_first = kind_is(
                    selection,
                    &["ActivityEdge", "ActivityEdgePresentation", "Relationship", "Label"],
                );
                if edge_first
                    && let Some(edge) = diagram.edges.iter().find(|edge| {
                        edge.id == selection.id || edge.activity_edge_id == selection.id
                    })
                {
                    let source = diagram
                        .nodes
                        .iter()
                        .find(|node| node.id == edge.source_node_id)
                        .ok_or("Activity source presentation not found")?;
                    let target = diagram
                        .nodes
                        .iter()
                        .find(|node| node.id == edge.target_node_id)
                        .ok_or("Activity target presentation not found")?;
                    items.push(ClipboardItem::ActivityEdge {
                        edge: edge.clone(),
                        source_semantic_id: source.activity_node_id.clone(),
                        target_semantic_id: target.activity_node_id.clone(),
                    });
                    continue;
                }
                if let Some(node) = diagram.nodes.iter().find(|node| {
                    node.id == selection.id || node.activity_node_id == selection.id
                }) {
                    items.push(ClipboardItem::ActivityNode { node: node.clone() });
                    continue;
                }
                if let Some(edge) = diagram.edges.iter().find(|edge| {
                    edge.id == selection.id || edge.activity_edge_id == selection.id
                }) {
                    let source = diagram
                        .nodes
                        .iter()
                        .find(|node| node.id == edge.source_node_id)
                        .ok_or("Activity source presentation not found")?;
                    let target = diagram
                        .nodes
                        .iter()
                        .find(|node| node.id == edge.target_node_id)
                        .ok_or("Activity target presentation not found")?;
                    items.push(ClipboardItem::ActivityEdge {
                        edge: edge.clone(),
                        source_semantic_id: source.activity_node_id.clone(),
                        target_semantic_id: target.activity_node_id.clone(),
                    });
                    continue;
                }
                return Err(format!("selected Activity presentation was not found: {}", selection.id));
            }
        }
    }
    Ok(ClipboardPayload {
        family,
        semantic_context_id,
        items,
    })
}

fn add_hidden(diagram: &mut behavior_workspace::BehaviorDiagram, semantic_id: &str) {
    if !diagram
        .hidden_semantic_ids
        .iter()
        .any(|candidate| candidate == semantic_id)
    {
        diagram.hidden_semantic_ids.push(semantic_id.to_string());
    }
}

fn remove_presentations(
    snapshot: &mut EditingSnapshot,
    diagram_id: &str,
    selections: &[WorkspaceSelection],
) -> Result<usize, String> {
    require_selections(selections)?;
    let family = family_for(diagram_id, snapshot)?;
    let mut changed = 0;
    match family {
        EditingFamily::Bdd | EditingFamily::Requirement => {
            let diagram = snapshot
                .diagrams
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("diagram not found")?;
            for selection in selections {
                if let Some(index) = diagram.edges.iter().position(|edge| {
                    edge.id == selection.id || edge.relationship_id == selection.id
                }) {
                    diagram.edges.remove(index);
                    changed += 1;
                    continue;
                }
                if let Some(index) = diagram.nodes.iter().position(|node| {
                    node.id == selection.id || node.element_id == selection.id
                }) {
                    let removed = diagram.nodes.remove(index);
                    diagram.edges.retain(|edge| {
                        edge.source_node_id != removed.id && edge.target_node_id != removed.id
                    });
                    changed += 1;
                }
            }
        }
        EditingFamily::Ibd => {
            let diagram = snapshot
                .ibd_diagrams
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("IBD not found")?;
            for selection in selections {
                if let Some(index) = diagram.connectors.iter().position(|edge| {
                    edge.id == selection.id || edge.relationship_id == selection.id
                }) {
                    diagram.connectors.remove(index);
                    changed += 1;
                    continue;
                }
                if let Some(index) = diagram.properties.iter().position(|property| {
                    property.id == selection.id || property.element_id == selection.id
                }) {
                    let property = diagram.properties.remove(index);
                    let mut removed = vec![property.id];
                    removed.extend(property.ports.into_iter().map(|port| port.id));
                    diagram.connectors.retain(|edge| {
                        !removed.contains(&edge.source_presentation_id)
                            && !removed.contains(&edge.target_presentation_id)
                    });
                    changed += 1;
                    continue;
                }
                if let Some(index) = diagram.boundary_ports.iter().position(|port| {
                    port.id == selection.id || port.element_id == selection.id
                }) {
                    let port = diagram.boundary_ports.remove(index);
                    diagram.connectors.retain(|edge| {
                        edge.source_presentation_id != port.id
                            && edge.target_presentation_id != port.id
                    });
                    changed += 1;
                    continue;
                }
                let mut removed_port = None;
                for property in &mut diagram.properties {
                    if let Some(index) = property.ports.iter().position(|port| {
                        port.id == selection.id || port.element_id == selection.id
                    }) {
                        removed_port = Some(property.ports.remove(index).id);
                        break;
                    }
                }
                if let Some(port_id) = removed_port {
                    diagram.connectors.retain(|edge| {
                        edge.source_presentation_id != port_id
                            && edge.target_presentation_id != port_id
                    });
                    changed += 1;
                }
            }
        }
        EditingFamily::StateMachine | EditingFamily::Sequence => {
            let diagram = snapshot
                .behavior_diagrams
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("behavior diagram not found")?;
            for selection in selections {
                if kind_is(selection, &["BehaviorCopy"])
                    && let Some(index) = diagram
                        .presentation_copies
                        .iter()
                        .position(|copy| copy.id == selection.id)
                {
                    diagram.presentation_copies.remove(index);
                    changed += 1;
                    continue;
                }
                let (semantic_id, _) = split_index(&selection.id);
                let kind = behavior_kind(selection, family);
                match kind.as_str() {
                    "Vertex" => diagram.state_nodes.retain(|node| node.vertex_id != semantic_id),
                    "Lifeline" => diagram
                        .lifelines
                        .retain(|lifeline| lifeline.lifeline_id != semantic_id),
                    "Transition" | "Message" => diagram
                        .edge_routes
                        .retain(|route| route.semantic_id != semantic_id),
                    _ => {}
                }
                add_hidden(diagram, semantic_id);
                changed += 1;
            }
        }
        EditingFamily::Activity => {
            let diagram = snapshot
                .activity_diagrams
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("Activity diagram not found")?;
            for selection in selections {
                if let Some(index) = diagram.edges.iter().position(|edge| {
                    edge.id == selection.id || edge.activity_edge_id == selection.id
                }) {
                    diagram.edges.remove(index);
                    changed += 1;
                    continue;
                }
                if let Some(index) = diagram.nodes.iter().position(|node| {
                    node.id == selection.id || node.activity_node_id == selection.id
                }) {
                    let node = diagram.nodes.remove(index);
                    diagram.edges.retain(|edge| {
                        edge.source_node_id != node.id && edge.target_node_id != node.id
                    });
                    changed += 1;
                }
            }
        }
    }
    if changed == 0 {
        return Err("none of the selected items has a removable presentation on this diagram".into());
    }
    Ok(changed)
}

fn paste_clipboard(
    snapshot: &mut EditingSnapshot,
    target_diagram_id: &str,
    payload: &ClipboardPayload,
) -> Result<Vec<WorkspaceSelection>, String> {
    let target_family = family_for(target_diagram_id, snapshot)?;
    let mut selections = Vec::new();
    match target_family {
        EditingFamily::Bdd | EditingFamily::Requirement => {
            if !matches!(payload.family, EditingFamily::Bdd | EditingFamily::Requirement) {
                return Err("clipboard selection is not compatible with a BDD/Requirement Diagram".into());
            }
            let diagram = snapshot
                .diagrams
                .iter_mut()
                .find(|diagram| diagram.id == target_diagram_id)
                .ok_or("diagram not found")?;
            let mut presentation_map = HashMap::new();
            for item in &payload.items {
                if let ClipboardItem::BddNode { node } = item {
                    snapshot
                        .project
                        .element(parse_element_id(&node.element_id)?)
                        .map_err(|error| error.to_string())?;
                    let mut copy = node.clone();
                    let old_id = copy.id.clone();
                    copy.id = uuid::Uuid::new_v4().to_string();
                    copy.x += PASTE_OFFSET;
                    copy.y += PASTE_OFFSET;
                    presentation_map.insert(old_id, copy.id.clone());
                    selections.push(WorkspaceSelection {
                        kind: "BddNode".into(),
                        id: copy.id.clone(),
                    });
                    diagram.nodes.push(copy);
                }
            }
            for item in &payload.items {
                if let ClipboardItem::BddEdge {
                    edge,
                    source_element_id,
                    target_element_id,
                } = item
                {
                    snapshot
                        .project
                        .relationship(parse_relationship_id(&edge.relationship_id)?)
                        .map_err(|error| error.to_string())?;
                    let source_id = presentation_map
                        .get(&edge.source_node_id)
                        .cloned()
                        .or_else(|| {
                            diagram
                                .nodes
                                .iter()
                                .find(|node| node.element_id == *source_element_id)
                                .map(|node| node.id.clone())
                        })
                        .ok_or("paste requires the relationship source element to be presented")?;
                    let target_id = presentation_map
                        .get(&edge.target_node_id)
                        .cloned()
                        .or_else(|| {
                            diagram
                                .nodes
                                .iter()
                                .find(|node| node.element_id == *target_element_id)
                                .map(|node| node.id.clone())
                        })
                        .ok_or("paste requires the relationship target element to be presented")?;
                    let source = diagram
                        .nodes
                        .iter()
                        .find(|node| node.id == source_id)
                        .cloned()
                        .ok_or("source presentation not found")?;
                    let target = diagram
                        .nodes
                        .iter()
                        .find(|node| node.id == target_id)
                        .cloned()
                        .ok_or("target presentation not found")?;
                    let id = uuid::Uuid::new_v4().to_string();
                    let points = route_relationship(&source, &target, &diagram.nodes);
                    diagram.edges.push(DiagramEdge {
                        id: id.clone(),
                        relationship_id: edge.relationship_id.clone(),
                        source_node_id: source_id,
                        target_node_id: target_id,
                        label_anchor: Some(routing::route_label_anchor(&points)),
                        points,
                    });
                    selections.push(WorkspaceSelection {
                        kind: "BddEdge".into(),
                        id,
                    });
                }
            }
        }
        EditingFamily::Ibd => {
            if payload.family != EditingFamily::Ibd {
                return Err("clipboard selection is not compatible with an IBD".into());
            }
            let target_context = snapshot
                .ibd_diagrams
                .iter()
                .find(|diagram| diagram.id == target_diagram_id)
                .map(|diagram| diagram.context_block_id.clone())
                .ok_or("IBD not found")?;
            if payload.semantic_context_id.as_deref() != Some(target_context.as_str()) {
                return Err("IBD paste is only legal when the copied semantic property paths belong to the same Block context".into());
            }
            let diagram = snapshot
                .ibd_diagrams
                .iter_mut()
                .find(|diagram| diagram.id == target_diagram_id)
                .ok_or("IBD not found")?;
            let mut presentation_map = HashMap::new();
            for item in &payload.items {
                if let ClipboardItem::IbdProperty { property } = item {
                    let mut copy = property.clone();
                    let old_property = copy.id.clone();
                    copy.id = uuid::Uuid::new_v4().to_string();
                    copy.x += PASTE_OFFSET;
                    copy.y += PASTE_OFFSET;
                    presentation_map.insert(old_property, copy.id.clone());
                    for port in &mut copy.ports {
                        let old_port = port.id.clone();
                        port.id = uuid::Uuid::new_v4().to_string();
                        port.x += PASTE_OFFSET;
                        port.y += PASTE_OFFSET;
                        presentation_map.insert(old_port, port.id.clone());
                    }
                    selections.push(WorkspaceSelection {
                        kind: "IbdProperty".into(),
                        id: copy.id.clone(),
                    });
                    diagram.properties.push(copy);
                }
            }
            for item in &payload.items {
                if let ClipboardItem::IbdPort { port, parent } = item {
                    let mut copy = port.clone();
                    let old_id = copy.id.clone();
                    copy.id = uuid::Uuid::new_v4().to_string();
                    copy.x += PASTE_OFFSET;
                    copy.y += PASTE_OFFSET;
                    presentation_map.insert(old_id, copy.id.clone());
                    if let Some(parent) = parent {
                        let parent_id = find_ibd_presentation(diagram, parent)
                            .ok_or("paste requires the nested port's parent property presentation")?;
                        let property = diagram
                            .properties
                            .iter_mut()
                            .find(|property| property.id == parent_id)
                            .ok_or("nested port parent presentation not found")?;
                        property.ports.push(copy.clone());
                    } else {
                        diagram.boundary_ports.push(copy.clone());
                    }
                    selections.push(WorkspaceSelection {
                        kind: "IbdPort".into(),
                        id: copy.id,
                    });
                }
            }
            for item in &payload.items {
                if let ClipboardItem::IbdConnector {
                    connector,
                    source,
                    target,
                } = item
                {
                    let source_id = presentation_map
                        .get(&connector.source_presentation_id)
                        .cloned()
                        .or_else(|| find_ibd_presentation(diagram, source))
                        .ok_or("IBD connector source is not presented on the target diagram")?;
                    let target_id = presentation_map
                        .get(&connector.target_presentation_id)
                        .cloned()
                        .or_else(|| find_ibd_presentation(diagram, target))
                        .ok_or("IBD connector target is not presented on the target diagram")?;
                    let id = uuid::Uuid::new_v4().to_string();
                    let points = ibd::route_ibd_edge(diagram, &source_id, &target_id)?;
                    diagram.connectors.push(ibd::IbdConnectorPresentation {
                        id: id.clone(),
                        relationship_id: connector.relationship_id.clone(),
                        source_presentation_id: source_id,
                        target_presentation_id: target_id,
                        label_anchor: Some(routing::route_label_anchor(&points)),
                        points,
                    });
                    selections.push(WorkspaceSelection {
                        kind: "IbdConnector".into(),
                        id,
                    });
                }
            }
        }
        EditingFamily::StateMachine | EditingFamily::Sequence => {
            if payload.family != target_family {
                return Err("behavior clipboard content must be pasted into the same diagram family".into());
            }
            let diagram = snapshot
                .behavior_diagrams
                .iter_mut()
                .find(|diagram| diagram.id == target_diagram_id)
                .ok_or("behavior diagram not found")?;
            if payload.semantic_context_id.as_deref() != Some(diagram.semantic_id.as_str()) {
                return Err("copy/paste of a behavior presentation requires a diagram of the same StateMachine/Interaction semantic object".into());
            }
            for item in &payload.items {
                let ClipboardItem::Behavior { kind, semantic_id } = item else {
                    continue;
                };
                diagram.hidden_semantic_ids.retain(|id| id != semantic_id);
                let id = uuid::Uuid::new_v4().to_string();
                diagram.presentation_copies.push(BehaviorPresentationCopy {
                    id: id.clone(),
                    semantic_id: semantic_id.clone(),
                    kind: kind.clone(),
                    offset_x: PASTE_OFFSET,
                    offset_y: if matches!(kind.as_str(), "Transition" | "Message") {
                        8.0
                    } else {
                        PASTE_OFFSET
                    },
                });
                selections.push(WorkspaceSelection {
                    kind: "BehaviorCopy".into(),
                    id,
                });
            }
        }
        EditingFamily::Activity => {
            if payload.family != EditingFamily::Activity {
                return Err("clipboard selection is not compatible with an Activity Diagram".into());
            }
            let target_activity = snapshot
                .activity_diagrams
                .iter()
                .find(|diagram| diagram.id == target_diagram_id)
                .map(|diagram| diagram.activity_id.clone())
                .ok_or("Activity diagram not found")?;
            if payload.semantic_context_id.as_deref() != Some(target_activity.as_str()) {
                return Err("Activity paste requires another presentation of the same Activity semantic object".into());
            }
            let diagram = snapshot
                .activity_diagrams
                .iter_mut()
                .find(|diagram| diagram.id == target_diagram_id)
                .ok_or("Activity diagram not found")?;
            let mut presentation_map = HashMap::new();
            for item in &payload.items {
                if let ClipboardItem::ActivityNode { node } = item {
                    let mut copy = node.clone();
                    let old_id = copy.id.clone();
                    copy.id = uuid::Uuid::new_v4().to_string();
                    copy.x += PASTE_OFFSET;
                    copy.y += PASTE_OFFSET;
                    presentation_map.insert(old_id, copy.id.clone());
                    selections.push(WorkspaceSelection {
                        kind: "ActivityNodePresentation".into(),
                        id: copy.id.clone(),
                    });
                    diagram.nodes.push(copy);
                }
            }
            for item in &payload.items {
                if let ClipboardItem::ActivityEdge {
                    edge,
                    source_semantic_id,
                    target_semantic_id,
                } = item
                {
                    let source_id = presentation_map
                        .get(&edge.source_node_id)
                        .cloned()
                        .or_else(|| {
                            diagram
                                .nodes
                                .iter()
                                .find(|node| node.activity_node_id == *source_semantic_id)
                                .map(|node| node.id.clone())
                        })
                        .ok_or("Activity source node is not presented")?;
                    let target_id = presentation_map
                        .get(&edge.target_node_id)
                        .cloned()
                        .or_else(|| {
                            diagram
                                .nodes
                                .iter()
                                .find(|node| node.activity_node_id == *target_semantic_id)
                                .map(|node| node.id.clone())
                        })
                        .ok_or("Activity target node is not presented")?;
                    let source = diagram
                        .nodes
                        .iter()
                        .find(|node| node.id == source_id)
                        .ok_or("Activity source presentation not found")?;
                    let target = diagram
                        .nodes
                        .iter()
                        .find(|node| node.id == target_id)
                        .ok_or("Activity target presentation not found")?;
                    let points = routing::orthogonal_route(routing::RouteRequest {
                        source: routing::RouteRect {
                            x: source.x,
                            y: source.y,
                            width: source.width,
                            height: source.height,
                        },
                        target: routing::RouteRect {
                            x: target.x,
                            y: target.y,
                            width: target.width,
                            height: target.height,
                        },
                        obstacles: &[],
                        lane_index: 0,
                        reserved_routes: &[],
                        allow_shared_departure: false,
                    });
                    let id = uuid::Uuid::new_v4().to_string();
                    diagram.edges.push(activity_workspace::ActivityDiagramEdge {
                        id: id.clone(),
                        activity_edge_id: edge.activity_edge_id.clone(),
                        source_node_id: source_id,
                        target_node_id: target_id,
                        label_anchor: Some(routing::route_label_anchor(&points)),
                        points,
                    });
                    selections.push(WorkspaceSelection {
                        kind: "ActivityEdgePresentation".into(),
                        id,
                    });
                }
            }
        }
    }
    if selections.is_empty() {
        return Err("clipboard contains no presentations compatible with the active diagram".into());
    }
    Ok(selections)
}

fn unique_copy_name(project: &Project, source: &systems_modeler_core::Element) -> String {
    let base = format!("{} Copy", source.name);
    if !project.elements.values().any(|element| element.name == base) {
        return base;
    }
    for index in 2..10_000 {
        let candidate = format!("{} Copy {index}", source.name);
        if !project
            .elements
            .values()
            .any(|element| element.name == candidate)
        {
            return candidate;
        }
    }
    format!("{} Copy {}", source.name, uuid::Uuid::new_v4())
}

fn unique_requirement_id(project: &Project, source: &str) -> String {
    for index in 1..10_000 {
        let candidate = if index == 1 {
            format!("{source}-copy")
        } else {
            format!("{source}-copy-{index}")
        };
        if !project.elements.values().any(|element| {
            element.requirement_id.as_deref() == Some(candidate.as_str())
        }) {
            return candidate;
        }
    }
    format!("{source}-{}", uuid::Uuid::new_v4())
}

fn duplicate_element(project: &mut Project, source_id: ElementId) -> Result<ElementId, String> {
    let source = project
        .element(source_id)
        .map_err(|error| error.to_string())?
        .clone();
    if source.kind == ElementKind::Model {
        return Err("the project Model root cannot be duplicated".into());
    }
    let id = ElementId::new();
    let mut duplicate = source.clone();
    duplicate.id = id;
    duplicate.external_id = format!("EL-{id}");
    duplicate.name = unique_copy_name(project, &source);
    if let Some(requirement_id) = source.requirement_id.as_deref() {
        duplicate.requirement_id = Some(unique_requirement_id(project, requirement_id));
    }
    project.elements.insert(id, duplicate);
    Ok(id)
}

fn duplicate_relationship(
    project: &mut Project,
    source_id: RelationshipId,
    element_map: &HashMap<ElementId, ElementId>,
    relationship_map: &HashMap<RelationshipId, RelationshipId>,
) -> Result<RelationshipId, String> {
    let source = project
        .relationship(source_id)
        .map_err(|error| error.to_string())?
        .clone();
    let id = RelationshipId::new();
    let mut duplicate = source;
    duplicate.id = id;
    duplicate.external_id = format!("REL-{id}");
    duplicate.source_id = element_map
        .get(&duplicate.source_id)
        .copied()
        .unwrap_or(duplicate.source_id);
    duplicate.target_id = element_map
        .get(&duplicate.target_id)
        .copied()
        .unwrap_or(duplicate.target_id);
    for end in &mut duplicate.association_ends {
        end.id = RelationshipEndId::new();
        end.classifier_id = element_map
            .get(&end.classifier_id)
            .copied()
            .unwrap_or(end.classifier_id);
    }
    if let Some(connector) = &mut duplicate.connector {
        connector.context_id = element_map
            .get(&connector.context_id)
            .copied()
            .unwrap_or(connector.context_id);
        for end in [&mut connector.source, &mut connector.target] {
            end.property_path = end
                .property_path
                .iter()
                .map(|id| element_map.get(id).copied().unwrap_or(*id))
                .collect();
            end.role_id = element_map.get(&end.role_id).copied().unwrap_or(end.role_id);
            end.port_id = end
                .port_id
                .map(|port| element_map.get(&port).copied().unwrap_or(port));
        }
    }
    if let Some(flow) = &mut duplicate.item_flow {
        flow.connector_id = relationship_map
            .get(&flow.connector_id)
            .copied()
            .unwrap_or(flow.connector_id);
        for end in [&mut flow.source, &mut flow.target] {
            end.property_path = end
                .property_path
                .iter()
                .map(|id| element_map.get(id).copied().unwrap_or(*id))
                .collect();
            end.role_id = element_map.get(&end.role_id).copied().unwrap_or(end.role_id);
            end.port_id = end
                .port_id
                .map(|port| element_map.get(&port).copied().unwrap_or(port));
        }
        flow.conveyed_item_ids = flow
            .conveyed_item_ids
            .iter()
            .map(|id| element_map.get(id).copied().unwrap_or(*id))
            .collect();
    }
    project.relationships.insert(id, duplicate);
    Ok(id)
}

fn duplicate_vertex_in_regions(regions: &mut [Region], wanted: VertexId) -> Option<VertexId> {
    for region in regions {
        if let Some(index) = region.vertices.iter().position(|vertex| vertex.id == wanted) {
            let mut duplicate = region.vertices[index].clone();
            duplicate.id = VertexId::new();
            if let VertexKind::State(state) = &mut duplicate.kind {
                state.regions.clear();
                state.submachine = None;
            }
            let id = duplicate.id;
            duplicate.name = format!("{} Copy", duplicate.name);
            region.vertices.push(duplicate);
            return Some(id);
        }
        for vertex in &mut region.vertices {
            if let VertexKind::State(state) = &mut vertex.kind
                && let Some(id) = duplicate_vertex_in_regions(&mut state.regions, wanted)
            {
                return Some(id);
            }
        }
    }
    None
}

fn duplicate_transition_in_regions(
    regions: &mut [Region],
    wanted: TransitionId,
    vertex_map: &HashMap<VertexId, VertexId>,
) -> Option<TransitionId> {
    for region in regions {
        if let Some(index) = region
            .transitions
            .iter()
            .position(|transition| transition.id == wanted)
        {
            let mut duplicate = region.transitions[index].clone();
            duplicate.id = TransitionId::new();
            duplicate.source_id = vertex_map
                .get(&duplicate.source_id)
                .copied()
                .unwrap_or(duplicate.source_id);
            duplicate.target_id = vertex_map
                .get(&duplicate.target_id)
                .copied()
                .unwrap_or(duplicate.target_id);
            let id = duplicate.id;
            region.transitions.push(duplicate);
            return Some(id);
        }
        for vertex in &mut region.vertices {
            if let VertexKind::State(state) = &mut vertex.kind
                && let Some(id) =
                    duplicate_transition_in_regions(&mut state.regions, wanted, vertex_map)
            {
                return Some(id);
            }
        }
    }
    None
}

fn duplicate_behavior_items(
    snapshot: &mut EditingSnapshot,
    diagram_id: &str,
    items: &[ClipboardItem],
    selections: &mut Vec<WorkspaceSelection>,
) -> Result<(), String> {
    let diagram_index = snapshot
        .behavior_diagrams
        .iter()
        .position(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    let diagram = snapshot.behavior_diagrams[diagram_index].clone();
    match diagram.kind {
        BehaviorDiagramKind::StateMachine => {
            let machine_id = uuid::Uuid::parse_str(&diagram.semantic_id)
                .map(systems_modeler_core::StateMachineId)
                .map_err(|_| "invalid State Machine id")?;
            let machine = snapshot
                .behavior
                .state_machines
                .get_mut(&machine_id)
                .ok_or("State Machine not found")?;
            let mut vertex_map = HashMap::new();
            for item in items {
                let ClipboardItem::Behavior { kind, semantic_id } = item else {
                    continue;
                };
                if kind != "Vertex" {
                    continue;
                }
                let old = uuid::Uuid::parse_str(semantic_id)
                    .map(VertexId)
                    .map_err(|_| "invalid State vertex id")?;
                let new = duplicate_vertex_in_regions(&mut machine.regions, old)
                    .ok_or("State vertex not found")?;
                vertex_map.insert(old, new);
                let source = diagram
                    .state_nodes
                    .iter()
                    .find(|node| node.vertex_id == *semantic_id);
                let (x, y, width, height) = source.map_or((80.0, 80.0, 150.0, 80.0), |node| {
                    (node.x, node.y, node.width, node.height)
                });
                snapshot.behavior_diagrams[diagram_index]
                    .state_nodes
                    .push(behavior_workspace::StateNodePresentation {
                        vertex_id: new.to_string(),
                        x: x + PASTE_OFFSET,
                        y: y + PASTE_OFFSET,
                        width,
                        height,
                    });
                selections.push(WorkspaceSelection {
                    kind: "Vertex".into(),
                    id: new.to_string(),
                });
            }
            for item in items {
                let ClipboardItem::Behavior { kind, semantic_id } = item else {
                    continue;
                };
                if kind != "Transition" {
                    continue;
                }
                let old = uuid::Uuid::parse_str(semantic_id)
                    .map(TransitionId)
                    .map_err(|_| "invalid State transition id")?;
                let new = duplicate_transition_in_regions(&mut machine.regions, old, &vertex_map)
                    .ok_or("State transition not found")?;
                selections.push(WorkspaceSelection {
                    kind: "Transition".into(),
                    id: new.to_string(),
                });
            }
        }
        BehaviorDiagramKind::Sequence => {
            let interaction_id = uuid::Uuid::parse_str(&diagram.semantic_id)
                .map(InteractionId)
                .map_err(|_| "invalid Interaction id")?;
            let interaction = snapshot
                .behavior
                .interactions
                .get_mut(&interaction_id)
                .ok_or("Interaction not found")?;
            let mut lifeline_map = HashMap::new();
            for item in items {
                let ClipboardItem::Behavior { kind, semantic_id } = item else {
                    continue;
                };
                if kind != "Lifeline" {
                    continue;
                }
                let old = uuid::Uuid::parse_str(semantic_id)
                    .map(LifelineId)
                    .map_err(|_| "invalid Lifeline id")?;
                let source = interaction
                    .lifelines
                    .iter()
                    .find(|lifeline| lifeline.id == old)
                    .cloned()
                    .ok_or("Lifeline not found")?;
                let mut duplicate = source;
                duplicate.id = LifelineId::new();
                duplicate.name = format!("{} Copy", duplicate.name);
                let new = duplicate.id;
                interaction.lifelines.push(duplicate);
                lifeline_map.insert(old, new);
                let source_presentation = diagram
                    .lifelines
                    .iter()
                    .find(|presentation| presentation.lifeline_id == *semantic_id);
                let mut presentation = source_presentation.cloned().unwrap_or(
                    behavior_workspace::LifelinePresentation {
                        lifeline_id: semantic_id.clone(),
                        x: 150.0,
                        timeline_start_y: 102.0,
                        timeline_end_y: 840.0,
                    },
                );
                presentation.lifeline_id = new.to_string();
                presentation.x += PASTE_OFFSET;
                snapshot.behavior_diagrams[diagram_index]
                    .lifelines
                    .push(presentation);
                selections.push(WorkspaceSelection {
                    kind: "Lifeline".into(),
                    id: new.to_string(),
                });
            }
            for item in items {
                let ClipboardItem::Behavior { kind, semantic_id } = item else {
                    continue;
                };
                match kind.as_str() {
                    "Message" => {
                        let old = uuid::Uuid::parse_str(semantic_id)
                            .map(MessageId)
                            .map_err(|_| "invalid Message id")?;
                        let mut duplicate = interaction
                            .messages
                            .iter()
                            .find(|message| message.id == old)
                            .cloned()
                            .ok_or("Message not found")?;
                        duplicate.id = MessageId::new();
                        if let Some(send) = &mut duplicate.send_event {
                            send.id = OccurrenceId::new();
                            send.lifeline_id = lifeline_map
                                .get(&send.lifeline_id)
                                .copied()
                                .unwrap_or(send.lifeline_id);
                            send.order = send.order.saturating_add(10);
                        }
                        if let Some(receive) = &mut duplicate.receive_event {
                            receive.id = OccurrenceId::new();
                            receive.lifeline_id = lifeline_map
                                .get(&receive.lifeline_id)
                                .copied()
                                .unwrap_or(receive.lifeline_id);
                            receive.order = receive.order.saturating_add(10);
                        }
                        let id = duplicate.id;
                        interaction.messages.push(duplicate);
                        selections.push(WorkspaceSelection {
                            kind: "Message".into(),
                            id: id.to_string(),
                        });
                    }
                    "Execution" => {
                        let old = uuid::Uuid::parse_str(semantic_id)
                            .map(ExecutionId)
                            .map_err(|_| "invalid Execution id")?;
                        let mut duplicate = interaction
                            .executions
                            .iter()
                            .find(|execution| execution.id == old)
                            .cloned()
                            .ok_or("Execution Specification not found")?;
                        duplicate.id = ExecutionId::new();
                        duplicate.lifeline_id = lifeline_map
                            .get(&duplicate.lifeline_id)
                            .copied()
                            .unwrap_or(duplicate.lifeline_id);
                        duplicate.start.id = OccurrenceId::new();
                        duplicate.finish.id = OccurrenceId::new();
                        duplicate.start.lifeline_id = duplicate.lifeline_id;
                        duplicate.finish.lifeline_id = duplicate.lifeline_id;
                        duplicate.start.order = duplicate.start.order.saturating_add(10);
                        duplicate.finish.order = duplicate.finish.order.saturating_add(10);
                        let id = duplicate.id;
                        interaction.executions.push(duplicate);
                        selections.push(WorkspaceSelection {
                            kind: "Execution".into(),
                            id: id.to_string(),
                        });
                    }
                    "Fragment" => {
                        let old = uuid::Uuid::parse_str(semantic_id)
                            .map(FragmentId)
                            .map_err(|_| "invalid Combined Fragment id")?;
                        let mut duplicate = interaction
                            .fragments
                            .iter()
                            .find(|fragment| fragment.id == old)
                            .cloned()
                            .ok_or("Combined Fragment not found")?;
                        duplicate.id = FragmentId::new();
                        duplicate.covered_lifelines = duplicate
                            .covered_lifelines
                            .iter()
                            .map(|id| lifeline_map.get(id).copied().unwrap_or(*id))
                            .collect();
                        for operand in &mut duplicate.operands {
                            operand.id = systems_modeler_core::OperandId::new();
                            operand.start_order = operand.start_order.saturating_add(10);
                            operand.end_order = operand.end_order.saturating_add(10);
                        }
                        let id = duplicate.id;
                        interaction.fragments.push(duplicate);
                        selections.push(WorkspaceSelection {
                            kind: "Fragment".into(),
                            id: id.to_string(),
                        });
                    }
                    "Invariant" => {
                        let old = uuid::Uuid::parse_str(semantic_id)
                            .map(InvariantId)
                            .map_err(|_| "invalid State Invariant id")?;
                        let mut duplicate = interaction
                            .state_invariants
                            .iter()
                            .find(|invariant| invariant.id == old)
                            .cloned()
                            .ok_or("State Invariant not found")?;
                        duplicate.id = InvariantId::new();
                        duplicate.lifeline_id = lifeline_map
                            .get(&duplicate.lifeline_id)
                            .copied()
                            .unwrap_or(duplicate.lifeline_id);
                        duplicate.order = duplicate.order.saturating_add(10);
                        let id = duplicate.id;
                        interaction.state_invariants.push(duplicate);
                        selections.push(WorkspaceSelection {
                            kind: "Invariant".into(),
                            id: id.to_string(),
                        });
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn duplicate_activity_items(
    snapshot: &mut EditingSnapshot,
    diagram_id: &str,
    items: &[ClipboardItem],
    selections: &mut Vec<WorkspaceSelection>,
) -> Result<(), String> {
    let diagram_index = snapshot
        .activity_diagrams
        .iter()
        .position(|diagram| diagram.id == diagram_id)
        .ok_or("Activity diagram not found")?;
    let activity_id = activity_workspace::parse_activity_id(
        &snapshot.activity_diagrams[diagram_index].activity_id,
    )?;
    let activity = snapshot
        .activity
        .activities
        .get_mut(&activity_id)
        .ok_or("Activity not found")?;
    let source_diagram = snapshot.activity_diagrams[diagram_index].clone();
    let mut node_map = HashMap::new();
    let mut pin_map = HashMap::new();
    let mut presentation_map = HashMap::new();
    for item in items {
        let ClipboardItem::ActivityNode { node } = item else {
            continue;
        };
        let old = activity_workspace::parse_activity_node_id(&node.activity_node_id)?;
        let mut semantic = activity
            .nodes
            .iter()
            .find(|candidate| candidate.id == old)
            .cloned()
            .ok_or("Activity node not found")?;
        semantic.id = ActivityNodeId::new();
        semantic.name = format!("{} Copy", semantic.name);
        if let ActivityNodeKind::Action(action) = &mut semantic.kind {
            for pin in &mut action.pins {
                let old_pin = pin.id;
                pin.id = PinId::new();
                pin_map.insert(old_pin, pin.id);
            }
        }
        let new = semantic.id;
        activity.nodes.push(semantic);
        node_map.insert(old, new);
        let mut presentation = node.clone();
        let old_presentation = presentation.id.clone();
        presentation.id = uuid::Uuid::new_v4().to_string();
        presentation.activity_node_id = new.to_string();
        presentation.x += PASTE_OFFSET;
        presentation.y += PASTE_OFFSET;
        presentation_map.insert(old_presentation, presentation.id.clone());
        selections.push(WorkspaceSelection {
            kind: "ActivityNodePresentation".into(),
            id: presentation.id.clone(),
        });
        snapshot.activity_diagrams[diagram_index]
            .nodes
            .push(presentation);
    }
    for item in items {
        let ClipboardItem::ActivityEdge { edge, .. } = item else {
            continue;
        };
        let old = uuid::Uuid::parse_str(&edge.activity_edge_id)
            .map(ActivityEdgeId)
            .map_err(|_| "invalid Activity edge id")?;
        let mut semantic = activity
            .edges
            .iter()
            .find(|candidate| candidate.id == old)
            .cloned()
            .ok_or("Activity edge not found")?;
        semantic.id = ActivityEdgeId::new();
        semantic.source = match semantic.source {
            ActivityEndpoint::Node(id) => ActivityEndpoint::Node(
                node_map.get(&id).copied().unwrap_or(id),
            ),
            ActivityEndpoint::Pin(id) => {
                ActivityEndpoint::Pin(pin_map.get(&id).copied().unwrap_or(id))
            }
        };
        semantic.target = match semantic.target {
            ActivityEndpoint::Node(id) => ActivityEndpoint::Node(
                node_map.get(&id).copied().unwrap_or(id),
            ),
            ActivityEndpoint::Pin(id) => {
                ActivityEndpoint::Pin(pin_map.get(&id).copied().unwrap_or(id))
            }
        };
        let new = semantic.id;
        activity.edges.push(semantic);
        let source_presentation_id = presentation_map
            .get(&edge.source_node_id)
            .cloned()
            .unwrap_or_else(|| edge.source_node_id.clone());
        let target_presentation_id = presentation_map
            .get(&edge.target_node_id)
            .cloned()
            .unwrap_or_else(|| edge.target_node_id.clone());
        if !source_diagram
            .nodes
            .iter()
            .any(|node| node.id == edge.source_node_id)
        {
            return Err("Activity edge source presentation not found".into());
        }
        let mut presentation = edge.clone();
        presentation.id = uuid::Uuid::new_v4().to_string();
        presentation.activity_edge_id = new.to_string();
        presentation.source_node_id = source_presentation_id;
        presentation.target_node_id = target_presentation_id;
        for point in &mut presentation.points {
            point.x += PASTE_OFFSET;
            point.y += PASTE_OFFSET;
        }
        if let Some(anchor) = &mut presentation.label_anchor {
            anchor.x += PASTE_OFFSET;
            anchor.y += PASTE_OFFSET;
        }
        selections.push(WorkspaceSelection {
            kind: "ActivityEdgePresentation".into(),
            id: presentation.id.clone(),
        });
        snapshot.activity_diagrams[diagram_index]
            .edges
            .push(presentation);
    }
    Ok(())
}

fn duplicate_selection_items(
    snapshot: &mut EditingSnapshot,
    diagram_id: &str,
    payload: &ClipboardPayload,
) -> Result<Vec<WorkspaceSelection>, String> {
    let family = family_for(diagram_id, snapshot)?;
    if family != payload.family {
        return Err("Duplicate must run in the diagram where the selection was made".into());
    }
    let mut selections = Vec::new();
    match family {
        EditingFamily::Bdd | EditingFamily::Requirement => {
            let diagram_index = snapshot
                .diagrams
                .iter()
                .position(|diagram| diagram.id == diagram_id)
                .ok_or("diagram not found")?;
            let source_diagram = snapshot.diagrams[diagram_index].clone();
            let mut element_map = HashMap::new();
            let mut presentation_map = HashMap::new();
            for item in &payload.items {
                let ClipboardItem::BddNode { node } = item else {
                    continue;
                };
                let old = parse_element_id(&node.element_id)?;
                let new = duplicate_element(&mut snapshot.project, old)?;
                element_map.insert(old, new);
                let mut presentation = node.clone();
                let old_presentation = presentation.id.clone();
                presentation.id = uuid::Uuid::new_v4().to_string();
                presentation.element_id = new.to_string();
                presentation.x += PASTE_OFFSET;
                presentation.y += PASTE_OFFSET;
                presentation_map.insert(old_presentation, presentation.id.clone());
                selections.push(WorkspaceSelection {
                    kind: "BddNode".into(),
                    id: presentation.id.clone(),
                });
                snapshot.diagrams[diagram_index].nodes.push(presentation);
            }
            let mut relationship_map = HashMap::new();
            let mut relationship_items: Vec<_> = payload
                .items
                .iter()
                .filter_map(|item| match item {
                    ClipboardItem::BddEdge { edge, .. } => Some(edge),
                    _ => None,
                })
                .collect();
            relationship_items.sort_by_key(|edge| {
                snapshot
                    .project
                    .relationship(parse_relationship_id(&edge.relationship_id).unwrap())
                    .map(|relationship| matches!(relationship.kind, systems_modeler_core::RelationshipKind::ItemFlow))
                    .unwrap_or(false)
            });
            for edge in relationship_items {
                let old = parse_relationship_id(&edge.relationship_id)?;
                let new = duplicate_relationship(
                    &mut snapshot.project,
                    old,
                    &element_map,
                    &relationship_map,
                )?;
                relationship_map.insert(old, new);
                let source_old = source_diagram
                    .nodes
                    .iter()
                    .find(|node| node.id == edge.source_node_id)
                    .ok_or("relationship source presentation not found")?;
                let target_old = source_diagram
                    .nodes
                    .iter()
                    .find(|node| node.id == edge.target_node_id)
                    .ok_or("relationship target presentation not found")?;
                let source_element = element_map
                    .get(&parse_element_id(&source_old.element_id)?)
                    .copied()
                    .unwrap_or(parse_element_id(&source_old.element_id)?);
                let target_element = element_map
                    .get(&parse_element_id(&target_old.element_id)?)
                    .copied()
                    .unwrap_or(parse_element_id(&target_old.element_id)?);
                let source_id = presentation_map
                    .get(&edge.source_node_id)
                    .cloned()
                    .or_else(|| {
                        snapshot.diagrams[diagram_index]
                            .nodes
                            .iter()
                            .find(|node| node.element_id == source_element.to_string())
                            .map(|node| node.id.clone())
                    })
                    .ok_or("duplicate relationship source must be presented")?;
                let target_id = presentation_map
                    .get(&edge.target_node_id)
                    .cloned()
                    .or_else(|| {
                        snapshot.diagrams[diagram_index]
                            .nodes
                            .iter()
                            .find(|node| node.element_id == target_element.to_string())
                            .map(|node| node.id.clone())
                    })
                    .ok_or("duplicate relationship target must be presented")?;
                let source = snapshot.diagrams[diagram_index]
                    .nodes
                    .iter()
                    .find(|node| node.id == source_id)
                    .cloned()
                    .ok_or("source presentation missing")?;
                let target = snapshot.diagrams[diagram_index]
                    .nodes
                    .iter()
                    .find(|node| node.id == target_id)
                    .cloned()
                    .ok_or("target presentation missing")?;
                let id = uuid::Uuid::new_v4().to_string();
                let points = route_relationship(
                    &source,
                    &target,
                    &snapshot.diagrams[diagram_index].nodes,
                );
                snapshot.diagrams[diagram_index].edges.push(DiagramEdge {
                    id: id.clone(),
                    relationship_id: new.to_string(),
                    source_node_id: source_id,
                    target_node_id: target_id,
                    label_anchor: Some(routing::route_label_anchor(&points)),
                    points,
                });
                selections.push(WorkspaceSelection {
                    kind: "BddEdge".into(),
                    id,
                });
            }
        }
        EditingFamily::Ibd => {
            let diagram_index = snapshot
                .ibd_diagrams
                .iter()
                .position(|diagram| diagram.id == diagram_id)
                .ok_or("IBD not found")?;
            let source_diagram = snapshot.ibd_diagrams[diagram_index].clone();
            let mut element_map = HashMap::new();
            let mut presentation_map = HashMap::new();
            for item in &payload.items {
                match item {
                    ClipboardItem::IbdProperty { property } => {
                        let old = parse_element_id(&property.element_id)?;
                        let new = *element_map
                            .entry(old)
                            .or_insert(duplicate_element(&mut snapshot.project, old)?);
                        let mut presentation = property.clone();
                        let old_id = presentation.id.clone();
                        presentation.id = uuid::Uuid::new_v4().to_string();
                        presentation.element_id = new.to_string();
                        if let Some(last) = presentation.property_path.last_mut()
                            && *last == old.to_string()
                        {
                            *last = new.to_string();
                        }
                        presentation.x += PASTE_OFFSET;
                        presentation.y += PASTE_OFFSET;
                        presentation_map.insert(old_id, presentation.id.clone());
                        selections.push(WorkspaceSelection {
                            kind: "IbdProperty".into(),
                            id: presentation.id.clone(),
                        });
                        snapshot.ibd_diagrams[diagram_index]
                            .properties
                            .push(presentation);
                    }
                    ClipboardItem::IbdPort { port, parent } => {
                        let old = parse_element_id(&port.element_id)?;
                        let new = *element_map
                            .entry(old)
                            .or_insert(duplicate_element(&mut snapshot.project, old)?);
                        let mut presentation = port.clone();
                        presentation.id = uuid::Uuid::new_v4().to_string();
                        presentation.element_id = new.to_string();
                        presentation.x += PASTE_OFFSET;
                        presentation.y += PASTE_OFFSET;
                        if let Some(parent) = parent {
                            let parent_id = find_ibd_presentation(
                                &snapshot.ibd_diagrams[diagram_index],
                                parent,
                            )
                            .ok_or("nested port parent presentation not found")?;
                            snapshot.ibd_diagrams[diagram_index]
                                .properties
                                .iter_mut()
                                .find(|property| property.id == parent_id)
                                .ok_or("nested port parent presentation not found")?
                                .ports
                                .push(presentation.clone());
                        } else {
                            snapshot.ibd_diagrams[diagram_index]
                                .boundary_ports
                                .push(presentation.clone());
                        }
                        selections.push(WorkspaceSelection {
                            kind: "IbdPort".into(),
                            id: presentation.id,
                        });
                    }
                    _ => {}
                }
            }
            let mut relationship_map = HashMap::new();
            for item in &payload.items {
                let ClipboardItem::IbdConnector {
                    connector,
                    source,
                    target,
                } = item else {
                    continue;
                };
                let old = parse_relationship_id(&connector.relationship_id)?;
                let new = duplicate_relationship(
                    &mut snapshot.project,
                    old,
                    &element_map,
                    &relationship_map,
                )?;
                relationship_map.insert(old, new);
                let source_id = presentation_map
                    .get(&connector.source_presentation_id)
                    .cloned()
                    .or_else(|| find_ibd_presentation(&snapshot.ibd_diagrams[diagram_index], source))
                    .ok_or("connector source presentation missing")?;
                let target_id = presentation_map
                    .get(&connector.target_presentation_id)
                    .cloned()
                    .or_else(|| find_ibd_presentation(&snapshot.ibd_diagrams[diagram_index], target))
                    .ok_or("connector target presentation missing")?;
                let points = ibd::route_ibd_edge(
                    &snapshot.ibd_diagrams[diagram_index],
                    &source_id,
                    &target_id,
                )?;
                let id = uuid::Uuid::new_v4().to_string();
                snapshot.ibd_diagrams[diagram_index]
                    .connectors
                    .push(ibd::IbdConnectorPresentation {
                        id: id.clone(),
                        relationship_id: new.to_string(),
                        source_presentation_id: source_id,
                        target_presentation_id: target_id,
                        label_anchor: Some(routing::route_label_anchor(&points)),
                        points,
                    });
                selections.push(WorkspaceSelection {
                    kind: "IbdConnector".into(),
                    id,
                });
            }
            let _ = source_diagram;
        }
        EditingFamily::StateMachine | EditingFamily::Sequence => {
            duplicate_behavior_items(snapshot, diagram_id, &payload.items, &mut selections)?;
        }
        EditingFamily::Activity => {
            duplicate_activity_items(snapshot, diagram_id, &payload.items, &mut selections)?;
        }
    }
    if selections.is_empty() {
        return Err("none of the selected presentations can be semantically duplicated".into());
    }
    Ok(selections)
}

fn move_selection_items(
    snapshot: &mut EditingSnapshot,
    diagram_id: &str,
    selections: &[WorkspaceSelection],
    dx: f64,
    dy: f64,
) -> Result<usize, String> {
    require_selections(selections)?;
    if !dx.is_finite() || !dy.is_finite() {
        return Err("move delta must be finite".into());
    }
    let family = family_for(diagram_id, snapshot)?;
    let mut changed = 0;
    match family {
        EditingFamily::Bdd | EditingFamily::Requirement => {
            let diagram = snapshot
                .diagrams
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("diagram not found")?;
            for selection in selections {
                if let Some(node) = diagram.nodes.iter_mut().find(|node| {
                    node.id == selection.id || node.element_id == selection.id
                }) {
                    node.x = (node.x + dx).max(0.0);
                    node.y = (node.y + dy).max(42.0);
                    changed += 1;
                    continue;
                }
                if let Some(edge) = diagram.edges.iter_mut().find(|edge| {
                    edge.id == selection.id || edge.relationship_id == selection.id
                }) {
                    let anchor = edge.label_anchor.get_or_insert_with(|| routing::route_label_anchor(&edge.points));
                    anchor.x += dx;
                    anchor.y += dy;
                    changed += 1;
                }
            }
            let routes: Vec<_> = diagram
                .edges
                .iter()
                .filter_map(|edge| {
                    let source = diagram.nodes.iter().find(|node| node.id == edge.source_node_id)?;
                    let target = diagram.nodes.iter().find(|node| node.id == edge.target_node_id)?;
                    Some((edge.id.clone(), route_relationship(source, target, &diagram.nodes)))
                })
                .collect();
            for (edge_id, points) in routes {
                if let Some(edge) = diagram.edges.iter_mut().find(|edge| edge.id == edge_id) {
                    edge.points = points;
                }
            }
        }
        EditingFamily::Ibd => {
            let diagram = snapshot
                .ibd_diagrams
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("IBD not found")?;
            for selection in selections {
                if let Some(property) = diagram.properties.iter_mut().find(|property| {
                    property.id == selection.id || property.element_id == selection.id
                }) {
                    property.x = (property.x + dx).max(0.0);
                    property.y = (property.y + dy).max(42.0);
                    for port in &mut property.ports {
                        port.x += dx;
                        port.y += dy;
                    }
                    changed += 1;
                    continue;
                }
                let mut port_found = false;
                for property in &mut diagram.properties {
                    if let Some(port) = property.ports.iter_mut().find(|port| {
                        port.id == selection.id || port.element_id == selection.id
                    }) {
                        port.x += dx;
                        port.y += dy;
                        port_found = true;
                        changed += 1;
                        break;
                    }
                }
                if port_found {
                    continue;
                }
                if let Some(port) = diagram.boundary_ports.iter_mut().find(|port| {
                    port.id == selection.id || port.element_id == selection.id
                }) {
                    port.x += dx;
                    port.y += dy;
                    changed += 1;
                    continue;
                }
                if let Some(edge) = diagram.connectors.iter_mut().find(|edge| {
                    edge.id == selection.id || edge.relationship_id == selection.id
                }) {
                    let anchor = edge.label_anchor.get_or_insert_with(|| routing::route_label_anchor(&edge.points));
                    anchor.x += dx;
                    anchor.y += dy;
                    changed += 1;
                }
            }
            let endpoints: Vec<_> = diagram
                .connectors
                .iter()
                .map(|edge| {
                    (
                        edge.id.clone(),
                        edge.source_presentation_id.clone(),
                        edge.target_presentation_id.clone(),
                    )
                })
                .collect();
            for (edge_id, source, target) in endpoints {
                let points = ibd::route_ibd_edge(diagram, &source, &target)?;
                if let Some(edge) = diagram.connectors.iter_mut().find(|edge| edge.id == edge_id) {
                    edge.points = points;
                }
            }
        }
        EditingFamily::StateMachine | EditingFamily::Sequence => {
            let diagram = snapshot
                .behavior_diagrams
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("behavior diagram not found")?;
            for selection in selections {
                if kind_is(selection, &["BehaviorCopy"])
                    && let Some(copy) = diagram
                        .presentation_copies
                        .iter_mut()
                        .find(|copy| copy.id == selection.id)
                {
                    copy.offset_x += dx;
                    copy.offset_y += dy;
                    changed += 1;
                    continue;
                }
                let (semantic_id, _) = split_index(&selection.id);
                match behavior_kind(selection, family).as_str() {
                    "Vertex" => {
                        if let Some(node) = diagram
                            .state_nodes
                            .iter_mut()
                            .find(|node| node.vertex_id == semantic_id)
                        {
                            node.x = (node.x + dx).max(0.0);
                            node.y = (node.y + dy).max(42.0);
                            changed += 1;
                        }
                    }
                    "Lifeline" => {
                        if let Some(lifeline) = diagram
                            .lifelines
                            .iter_mut()
                            .find(|lifeline| lifeline.lifeline_id == semantic_id)
                        {
                            lifeline.x = (lifeline.x + dx).max(40.0);
                            changed += 1;
                        }
                    }
                    "Transition" | "Message" => {
                        if let Some(route) = diagram
                            .edge_routes
                            .iter_mut()
                            .find(|route| route.semantic_id == semantic_id)
                        {
                            if let Some(anchor) = &mut route.label_anchor {
                                anchor.x += dx;
                                anchor.y += dy;
                            }
                            changed += 1;
                        }
                    }
                    _ => {}
                }
            }
        }
        EditingFamily::Activity => {
            let diagram = snapshot
                .activity_diagrams
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("Activity diagram not found")?;
            for selection in selections {
                if let Some(node) = diagram.nodes.iter_mut().find(|node| {
                    node.id == selection.id || node.activity_node_id == selection.id
                }) {
                    node.x = (node.x + dx).max(0.0);
                    node.y = (node.y + dy).max(42.0);
                    changed += 1;
                    continue;
                }
                if let Some(edge) = diagram.edges.iter_mut().find(|edge| {
                    edge.id == selection.id || edge.activity_edge_id == selection.id
                }) {
                    let anchor = edge.label_anchor.get_or_insert_with(|| routing::route_label_anchor(&edge.points));
                    anchor.x += dx;
                    anchor.y += dy;
                    changed += 1;
                }
            }
        }
    }
    if changed == 0 {
        return Err("none of the selected presentations can be moved".into());
    }
    Ok(changed)
}

pub fn copy_selection(
    diagram_id: String,
    selections: Vec<WorkspaceSelection>,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    editing: tauri::State<'_, StandardEditingState>,
) -> Result<StandardEditingResult, String> {
    let snapshot = EditingSnapshot::capture(&workspace, &activity)?;
    let payload = collect_clipboard(&snapshot, &diagram_id, &selections)?;
    let changed = payload.items.len();
    *editing
        .clipboard
        .lock()
        .map_err(|_| "editing clipboard lock poisoned")? = Some(payload);
    Ok(StandardEditingResult { changed, selections })
}

pub fn paste_selection(
    diagram_id: String,
    selections: Vec<WorkspaceSelection>,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
    editing: tauri::State<'_, StandardEditingState>,
) -> Result<StandardEditingResult, String> {
    let _ = selections;
    let payload = editing
        .clipboard
        .lock()
        .map_err(|_| "editing clipboard lock poisoned")?
        .clone()
        .ok_or("clipboard is empty")?;
    let mut snapshot = EditingSnapshot::capture(&workspace, &activity)?;
    let new_selections = paste_clipboard(&mut snapshot, &diagram_id, &payload)?;
    snapshot.validate()?;
    history::checkpoint_states(&workspace, &activity, &history)?;
    snapshot.commit(&workspace, &activity)?;
    Ok(StandardEditingResult {
        changed: new_selections.len(),
        selections: new_selections,
    })
}

pub fn duplicate_selection(
    diagram_id: String,
    selections: Vec<WorkspaceSelection>,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<StandardEditingResult, String> {
    let mut snapshot = EditingSnapshot::capture(&workspace, &activity)?;
    let payload = collect_clipboard(&snapshot, &diagram_id, &selections)?;
    let new_selections = duplicate_selection_items(&mut snapshot, &diagram_id, &payload)?;
    snapshot.validate()?;
    history::checkpoint_states(&workspace, &activity, &history)?;
    snapshot.commit(&workspace, &activity)?;
    Ok(StandardEditingResult {
        changed: new_selections.len(),
        selections: new_selections,
    })
}

pub fn delete_active_selection(
    diagram_id: String,
    selections: Vec<WorkspaceSelection>,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<StandardEditingResult, String> {
    let mut snapshot = EditingSnapshot::capture(&workspace, &activity)?;
    let changed = remove_presentations(&mut snapshot, &diagram_id, &selections)?;
    snapshot.validate()?;
    history::checkpoint_states(&workspace, &activity, &history)?;
    snapshot.commit(&workspace, &activity)?;
    Ok(StandardEditingResult {
        changed,
        selections: Vec::new(),
    })
}

pub fn move_active_selection(
    diagram_id: String,
    selections: Vec<WorkspaceSelection>,
    dx: f64,
    dy: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<StandardEditingResult, String> {
    let mut snapshot = EditingSnapshot::capture(&workspace, &activity)?;
    let changed = move_selection_items(&mut snapshot, &diagram_id, &selections, dx, dy)?;
    snapshot.validate()?;
    history::checkpoint_states(&workspace, &activity, &history)?;
    snapshot.commit(&workspace, &activity)?;
    Ok(StandardEditingResult { changed, selections })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requirement_copy_generates_new_stable_and_human_ids() {
        let mut project = Project::new("P");
        let package = project
            .create_element(ElementKind::Package, "Requirements", project.root_id)
            .expect("package");
        let requirement = project
            .create_requirement("Power", "REQ-1", "Provide power", package)
            .expect("requirement");
        let duplicate = duplicate_element(&mut project, requirement).expect("duplicate");
        let source = project.element(requirement).expect("source");
        let copy = project.element(duplicate).expect("copy");
        assert_ne!(source.id, copy.id);
        assert_ne!(source.external_id, copy.external_id);
        assert_ne!(source.requirement_id, copy.requirement_id);
        assert_eq!(source.owner_id, copy.owner_id);
        project.validate().expect("duplicate project validates");
    }

    #[test]
    fn copy_name_is_deterministic_without_colliding() {
        let mut project = Project::new("P");
        let block = project
            .create_element(ElementKind::Block, "Controller", project.root_id)
            .expect("block");
        let first = duplicate_element(&mut project, block).expect("first copy");
        let second = duplicate_element(&mut project, block).expect("second copy");
        assert_eq!(project.element(first).unwrap().name, "Controller Copy");
        assert_eq!(project.element(second).unwrap().name, "Controller Copy 2");
    }
}
