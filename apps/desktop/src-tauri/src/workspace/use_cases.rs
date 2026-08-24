//! Rust-authoritative Use Case semantics and diagram mutations.
//!
//! Use Case diagrams reuse the structural presentation store, routing, layout,
//! history, persistence, and standard-editing services. This module only adds
//! semantic eligibility and Use Case-specific property updates.

use super::*;
use systems_modeler_core::{AggregationKind, ElementKind, Multiplicity, RelationshipKind};

const SUBJECT_PADDING_X: f64 = 48.0;
const SUBJECT_PADDING_TOP: f64 = 58.0;
const SUBJECT_PADDING_BOTTOM: f64 = 42.0;
const SUBJECT_MIN_WIDTH: f64 = 280.0;
const SUBJECT_MIN_HEIGHT: f64 = 220.0;

fn checkpoint(
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &history::HistoryState,
) -> Result<(), String> {
    history::checkpoint_states(workspace, activity, history)
}

fn use_case_kind(value: &str) -> Result<ElementKind, String> {
    match value {
        "Actor" => Ok(ElementKind::Actor),
        "UseCase" => Ok(ElementKind::UseCase),
        _ => Err(format!("unsupported Use Case element: {value}")),
    }
}

fn relationship_kind(value: &str) -> Result<RelationshipKind, String> {
    match value {
        "Association" => Ok(RelationshipKind::Association),
        "Include" | "include" => Ok(RelationshipKind::Include),
        "Extend" | "extend" => Ok(RelationshipKind::Extend),
        "Generalization" => Ok(RelationshipKind::Generalization),
        _ => Err(format!("unsupported Use Case relationship: {value}")),
    }
}

fn default_subject_boundary() -> UseCaseSubjectBoundary {
    UseCaseSubjectBoundary {
        id: uuid::Uuid::new_v4().to_string(),
        x: 300.0,
        y: 84.0,
        width: 580.0,
        height: 500.0,
    }
}

fn presented_use_case_bounds(
    diagram: &BddDiagram,
    project: &Project,
) -> Option<(f64, f64, f64, f64)> {
    let mut use_cases = diagram.nodes.iter().filter(|node| {
        parse_element_id(&node.element_id)
            .ok()
            .and_then(|id| project.element(id).ok())
            .is_some_and(|element| element.kind == ElementKind::UseCase)
    });
    let first = use_cases.next()?;
    let mut left = first.x;
    let mut top = first.y;
    let mut right = first.x + first.width;
    let mut bottom = first.y + first.height;
    for node in use_cases {
        left = left.min(node.x);
        top = top.min(node.y);
        right = right.max(node.x + node.width);
        bottom = bottom.max(node.y + node.height);
    }
    Some((left, top, right, bottom))
}

/// Keeps the persisted subject rectangle valid without turning it into a
/// routing obstacle. Actors remain outside; only Use Cases determine bounds.
pub(super) fn fit_use_case_subject_boundary(
    diagram: &mut BddDiagram,
    project: &Project,
    reset: bool,
) {
    if diagram.family != "use-case" || diagram.semantic_context_id.is_none() {
        return;
    }
    if diagram.subject_boundary.is_none() {
        diagram.subject_boundary = Some(default_subject_boundary());
    }
    let Some((node_left, node_top, node_right, node_bottom)) =
        presented_use_case_bounds(diagram, project)
    else {
        return;
    };
    let required_left = (node_left - SUBJECT_PADDING_X).max(0.0);
    let required_top = (node_top - SUBJECT_PADDING_TOP).max(42.0);
    let required_right = node_right + SUBJECT_PADDING_X;
    let required_bottom = node_bottom + SUBJECT_PADDING_BOTTOM;
    let boundary = diagram
        .subject_boundary
        .as_mut()
        .expect("Use Case subject boundary initialized");
    let current_right = boundary.x + boundary.width;
    let current_bottom = boundary.y + boundary.height;
    let left = if reset {
        required_left
    } else {
        boundary.x.min(required_left)
    };
    let top = if reset {
        required_top
    } else {
        boundary.y.min(required_top)
    };
    let right = if reset {
        required_right
    } else {
        current_right.max(required_right)
    };
    let bottom = if reset {
        required_bottom
    } else {
        current_bottom.max(required_bottom)
    };
    boundary.x = left;
    boundary.y = top;
    boundary.width = (right - left).max(SUBJECT_MIN_WIDTH);
    boundary.height = (bottom - top).max(SUBJECT_MIN_HEIGHT);
}

fn reroute_use_case_relationships(diagram: &mut BddDiagram) -> Result<(), String> {
    let routes = diagram
        .edges
        .iter()
        .map(|edge| {
            let source = diagram
                .nodes
                .iter()
                .find(|node| node.id == edge.source_node_id)
                .ok_or("Use Case relationship source presentation not found")?;
            let target = diagram
                .nodes
                .iter()
                .find(|node| node.id == edge.target_node_id)
                .ok_or("Use Case relationship target presentation not found")?;
            Ok((
                edge.id.clone(),
                route_relationship(source, target, &diagram.nodes)?,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    for (edge_id, points) in routes {
        let edge = diagram
            .edges
            .iter_mut()
            .find(|edge| edge.id == edge_id)
            .ok_or("Use Case relationship presentation not found")?;
        edge.label_anchor = Some(routing::route_label_anchor(&points));
        edge.points = points;
    }
    Ok(())
}

#[tauri::command]
pub fn create_use_case_diagram(
    owner_id: String,
    name: String,
    semantic_context_id: Option<String>,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let owner_id = parse_element_id(&owner_id)?;
    let context_id = semantic_context_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(parse_element_id)
        .transpose()?;
    {
        let project = workspace
            .project
            .lock()
            .map_err(|_| "project lock poisoned")?;
        let project = project.as_ref().ok_or("no project open")?;
        let owner = project
            .element(owner_id)
            .map_err(|error| error.to_string())?;
        if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
            return Err("Use Case Diagram owner must be a Model or Package".into());
        }
        if let Some(context_id) = context_id {
            let context = project
                .element(context_id)
                .map_err(|error| error.to_string())?;
            if !context.is_classifier()
                || matches!(context.kind, ElementKind::Actor | ElementKind::UseCase)
            {
                return Err(
                    "Use Case diagram subject must be a represented system classifier".into(),
                );
            }
        }
    }
    checkpoint(&workspace, &activity, &history)?;
    let id = DiagramId::new().to_string();
    workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .push(BddDiagram {
            id: id.clone(),
            name,
            owner_id: owner_id.to_string(),
            family: "use-case".into(),
            semantic_context_id: context_id.map(|id| id.to_string()),
            subject_boundary: context_id.map(|_| default_subject_boundary()),
            nodes: Vec::new(),
            edges: Vec::new(),
        });
    Ok(id)
}

#[tauri::command]
pub fn update_use_case_diagram_subject(
    diagram_id: String,
    semantic_context_id: Option<String>,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    parse_diagram_id(&diagram_id)?;
    let context_id = semantic_context_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(parse_element_id)
        .transpose()?;
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    if let Some(context_id) = context_id {
        let context = project
            .element(context_id)
            .map_err(|error| error.to_string())?;
        if !context.is_classifier()
            || matches!(context.kind, ElementKind::Actor | ElementKind::UseCase)
        {
            return Err("Use Case diagram subject must be a represented system classifier".into());
        }
    }
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id && diagram.family == "use-case")
        .ok_or("Use Case Diagram not found")?;
    let next = context_id.map(|id| id.to_string());
    let boundary_expected = next.is_some();
    if diagram.semantic_context_id == next
        && diagram.subject_boundary.is_some() == boundary_expected
    {
        return Ok(());
    }
    diagram.semantic_context_id = next;
    if diagram.semantic_context_id.is_some() {
        if diagram.subject_boundary.is_none() {
            diagram.subject_boundary = Some(default_subject_boundary());
        }
        fit_use_case_subject_boundary(diagram, &project, false);
    } else {
        diagram.subject_boundary = None;
    }
    validate_loaded_diagrams(&project, &diagrams)?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable named-field Tauri IPC boundary.
pub fn update_use_case_subject_boundary_geometry(
    diagram_id: String,
    boundary_id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    parse_diagram_id(&diagram_id)?;
    if !x.is_finite() || !y.is_finite() || !width.is_finite() || !height.is_finite() {
        return Err("Use Case subject-boundary geometry must be finite".into());
    }
    if width < SUBJECT_MIN_WIDTH || height < SUBJECT_MIN_HEIGHT {
        return Err(format!(
            "Use Case subject boundary must be at least {SUBJECT_MIN_WIDTH} x {SUBJECT_MIN_HEIGHT}"
        ));
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
        .find(|diagram| diagram.id == diagram_id && diagram.family == "use-case")
        .ok_or("Use Case Diagram not found")?;
    let original = diagram
        .subject_boundary
        .clone()
        .ok_or("Use Case Diagram has no represented subject boundary")?;
    if original.id != boundary_id {
        return Err("Use Case subject-boundary presentation not found".into());
    }
    let position_changed = original.x != x || original.y != y;
    let size_changed = original.width != width || original.height != height;
    if !position_changed && !size_changed {
        return Ok(());
    }
    if position_changed && !size_changed {
        let dx = x - original.x;
        let dy = y - original.y;
        for node in &mut diagram.nodes {
            let is_use_case = parse_element_id(&node.element_id)
                .ok()
                .and_then(|id| project.element(id).ok())
                .is_some_and(|element| element.kind == ElementKind::UseCase);
            if is_use_case {
                node.x = (node.x + dx).max(0.0);
                node.y = (node.y + dy).max(42.0);
            }
        }
    }
    diagram.subject_boundary = Some(UseCaseSubjectBoundary {
        id: original.id,
        x: x.max(0.0),
        y: y.max(42.0),
        width,
        height,
    });
    fit_use_case_subject_boundary(diagram, &project, false);
    if position_changed && !size_changed {
        reroute_use_case_relationships(diagram)?;
    }
    validate_loaded_diagrams(&project, &diagrams)?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(())
}

#[tauri::command]
pub fn update_use_case_actor_notation(
    diagram_id: String,
    presentation_id: String,
    notation: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    parse_diagram_id(&diagram_id)?;
    if !matches!(notation.as_str(), "stick" | "rectangle") {
        return Err("Actor notation must be stick or rectangle".into());
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
        .find(|diagram| diagram.id == diagram_id && diagram.family == "use-case")
        .ok_or("Use Case Diagram not found")?;
    let node = diagram
        .nodes
        .iter_mut()
        .find(|node| node.id == presentation_id)
        .ok_or("Actor presentation not found")?;
    let element = project
        .element(parse_element_id(&node.element_id)?)
        .map_err(|error| error.to_string())?;
    if element.kind != ElementKind::Actor {
        return Err("Actor notation can only update an Actor presentation".into());
    }
    if node.actor_notation.as_deref() == Some(notation.as_str()) {
        return Ok(());
    }
    node.actor_notation = Some(notation);
    validate_loaded_diagrams(&project, &diagrams)?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(())
}

#[tauri::command]
pub fn create_use_case_element(
    kind: String,
    owner_id: String,
    name: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let kind = use_case_kind(&kind)?;
    let owner_id = parse_element_id(&owner_id)?;
    let mut candidate = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let id = candidate
        .create_element(kind, name, owner_id)
        .map_err(|error| error.to_string())?;
    candidate.validate().map_err(|error| error.to_string())?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(candidate);
    Ok(id.to_string())
}

#[tauri::command]
pub fn update_actor_details(
    element_id: String,
    name: String,
    documentation: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let element_id = parse_element_id(&element_id)?;
    let mut candidate = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    if candidate
        .element(element_id)
        .map_err(|error| error.to_string())?
        .kind
        != ElementKind::Actor
    {
        return Err("Actor properties can only update an Actor".into());
    }
    candidate
        .rename_element(element_id, name)
        .map_err(|error| error.to_string())?;
    candidate
        .element_mut(element_id)
        .map_err(|error| error.to_string())?
        .documentation = documentation;
    candidate.validate().map_err(|error| error.to_string())?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(candidate);
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable named-field Tauri IPC boundary.
pub fn update_use_case_specification(
    element_id: String,
    name: String,
    documentation: String,
    specification: String,
    extension_points: Vec<String>,
    represented_classifier_id: Option<String>,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let element_id = parse_element_id(&element_id)?;
    let represented_classifier_id = represented_classifier_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(parse_element_id)
        .transpose()?;
    let mut candidate = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    candidate
        .rename_element(element_id, name)
        .map_err(|error| error.to_string())?;
    candidate
        .element_mut(element_id)
        .map_err(|error| error.to_string())?
        .documentation = documentation;
    candidate
        .update_use_case(
            element_id,
            specification,
            extension_points,
            represented_classifier_id,
        )
        .map_err(|error| error.to_string())?;
    candidate.validate().map_err(|error| error.to_string())?;
    let minimum_height = {
        let rows = candidate
            .element(element_id)
            .map_err(|error| error.to_string())?
            .extension_points
            .len() as f64;
        (105.0 + rows * 20.0).max(115.0)
    };
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    for diagram in &mut diagrams {
        for node in diagram
            .nodes
            .iter_mut()
            .filter(|node| node.element_id == element_id.to_string())
        {
            node.height = node.height.max(minimum_height);
        }
        fit_use_case_subject_boundary(diagram, &candidate, false);
    }
    validate_loaded_diagrams(&candidate, &diagrams)?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(candidate);
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(())
}

#[tauri::command]
pub fn place_on_use_case_diagram(
    diagram_id: String,
    element_id: String,
    x: f64,
    y: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    if !x.is_finite() || !y.is_finite() {
        return Err("Use Case presentation coordinates must be finite".into());
    }
    parse_diagram_id(&diagram_id)?;
    let element_id = parse_element_id(&element_id)?;
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let element = project
        .element(element_id)
        .map_err(|error| error.to_string())?;
    if !matches!(element.kind, ElementKind::Actor | ElementKind::UseCase) {
        return Err("only Actors and Use Cases can be placed on a Use Case Diagram".into());
    }
    let (width, height) = match element.kind {
        ElementKind::Actor => (110.0, 150.0),
        ElementKind::UseCase => {
            let rows = element.extension_points.len() as f64;
            (210.0, (105.0 + rows * 20.0).max(115.0))
        }
        _ => unreachable!(),
    };
    let actor = element.kind == ElementKind::Actor;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id && diagram.family == "use-case")
        .ok_or("Use Case Diagram not found")?;
    if diagram
        .nodes
        .iter()
        .any(|node| node.element_id == element_id.to_string())
    {
        return Err("this semantic element is already presented on the Use Case Diagram".into());
    }
    let presentation_id = uuid::Uuid::new_v4().to_string();
    diagram.nodes.push(DiagramNode {
        id: presentation_id.clone(),
        element_id: element_id.to_string(),
        x,
        y,
        width,
        height,
        actor_notation: actor.then(|| "stick".into()),
    });
    fit_use_case_subject_boundary(diagram, &project, false);
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(presentation_id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable named-field Tauri IPC boundary.
pub fn create_use_case_relationship(
    diagram_id: String,
    kind: String,
    source_element_id: String,
    target_element_id: String,
    condition: Option<String>,
    extension_location: Option<String>,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    parse_diagram_id(&diagram_id)?;
    let kind = relationship_kind(&kind)?;
    let source_id = parse_element_id(&source_element_id)?;
    let target_id = parse_element_id(&target_element_id)?;
    let mut project = workspace
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
        .find(|diagram| diagram.id == diagram_id && diagram.family == "use-case")
        .ok_or("Use Case Diagram not found")?;
    let source_node = diagram
        .nodes
        .iter()
        .find(|node| node.element_id == source_element_id)
        .cloned()
        .ok_or("source element must be presented on the selected Use Case Diagram")?;
    let target_node = diagram
        .nodes
        .iter()
        .find(|node| node.element_id == target_element_id)
        .cloned()
        .ok_or("target element must be presented on the selected Use Case Diagram")?;
    if project.relationships.values().any(|relationship| {
        relationship.kind == kind
            && relationship.source_id == source_id
            && relationship.target_id == target_id
    }) {
        return Err(format!("an equivalent {kind:?} already exists"));
    }
    let owner_id = Some(parse_element_id(&diagram.owner_id)?);
    let relationship_id = if kind == RelationshipKind::Association {
        project
            .create_association(
                owner_id,
                vec![
                    Project::association_end(
                        source_id,
                        "",
                        Multiplicity::ONE,
                        false,
                        AggregationKind::None,
                    ),
                    Project::association_end(
                        target_id,
                        "",
                        Multiplicity::ONE,
                        false,
                        AggregationKind::None,
                    ),
                ],
            )
            .map_err(|error| error.to_string())?
    } else {
        project
            .create_relationship(kind.clone(), source_id, target_id, owner_id)
            .map_err(|error| error.to_string())?
    };
    if kind == RelationshipKind::Extend {
        project
            .update_extend_relationship(relationship_id, condition, extension_location)
            .map_err(|error| error.to_string())?;
    }
    let points = route_relationship(&source_node, &target_node, &diagram.nodes)?;
    let edge_id = uuid::Uuid::new_v4().to_string();
    diagram.edges.push(DiagramEdge {
        id: edge_id,
        relationship_id: relationship_id.to_string(),
        source_node_id: source_node.id,
        target_node_id: target_node.id,
        points,
        label_anchor: None,
    });
    project.validate().map_err(|error| error.to_string())?;
    validate_loaded_diagrams(&project, &diagrams)?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(relationship_id.to_string())
}

#[tauri::command]
pub fn update_extend_specification(
    relationship_id: String,
    condition: Option<String>,
    extension_location: Option<String>,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let relationship_id = parse_relationship_id(&relationship_id)?;
    let mut candidate = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    candidate
        .update_extend_relationship(relationship_id, condition, extension_location)
        .map_err(|error| error.to_string())?;
    candidate.validate().map_err(|error| error.to_string())?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(candidate);
    Ok(())
}

#[tauri::command]
pub fn reconnect_use_case_relationship(
    diagram_id: String,
    relationship_id: String,
    side: String,
    element_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    parse_diagram_id(&diagram_id)?;
    let relationship_id = parse_relationship_id(&relationship_id)?;
    let element_id = parse_element_id(&element_id)?;
    if !matches!(side.as_str(), "source" | "target") {
        return Err("relationship side must be source or target".into());
    }
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let original = project
        .relationship(relationship_id)
        .map_err(|error| error.to_string())?
        .clone();
    let (source_id, target_id) = if side == "source" {
        (element_id, original.target_id)
    } else {
        (original.source_id, element_id)
    };
    if project.relationships.values().any(|relationship| {
        relationship.id != relationship_id
            && relationship.kind == original.kind
            && relationship.source_id == source_id
            && relationship.target_id == target_id
    }) {
        return Err(format!("an equivalent {:?} already exists", original.kind));
    }
    {
        let relationship = project
            .relationships
            .get_mut(&relationship_id)
            .ok_or("relationship not found")?;
        relationship.source_id = source_id;
        relationship.target_id = target_id;
        if relationship.kind == RelationshipKind::Association
            && relationship.association_ends.len() == 2
        {
            relationship.association_ends[0].classifier_id = source_id;
            relationship.association_ends[1].classifier_id = target_id;
        }
        if relationship.kind == RelationshipKind::Extend && side == "target" {
            relationship.extension_location = None;
        }
    }
    project.validate().map_err(|error| error.to_string())?;

    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id && diagram.family == "use-case")
        .ok_or("Use Case Diagram not found")?;
    let source = diagram
        .nodes
        .iter()
        .find(|node| node.element_id == source_id.to_string())
        .cloned()
        .ok_or("new source must be presented on the Use Case Diagram")?;
    let target = diagram
        .nodes
        .iter()
        .find(|node| node.element_id == target_id.to_string())
        .cloned()
        .ok_or("new target must be presented on the Use Case Diagram")?;
    let points = route_relationship(&source, &target, &diagram.nodes)?;
    let edge = diagram
        .edges
        .iter_mut()
        .find(|edge| edge.relationship_id == relationship_id.to_string())
        .ok_or("Use Case relationship presentation not found")?;
    edge.source_node_id = source.id;
    edge.target_node_id = target.id;
    edge.points = points;
    validate_loaded_diagrams(&project, &diagrams)?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(())
}

#[tauri::command]
pub fn delete_use_case_relationship(
    relationship_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let relationship_id = parse_relationship_id(&relationship_id)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let relationship = project
        .relationship(relationship_id)
        .map_err(|error| error.to_string())?;
    if !matches!(
        relationship.kind,
        RelationshipKind::Association
            | RelationshipKind::Include
            | RelationshipKind::Extend
            | RelationshipKind::Generalization
    ) {
        return Err("relationship is not a Use Case relationship".into());
    }
    project.relationships.remove(&relationship_id);
    let relationship_id = relationship_id.to_string();
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    for diagram in &mut diagrams {
        diagram
            .edges
            .retain(|edge| edge.relationship_id != relationship_id);
    }
    project.validate().map_err(|error| error.to_string())?;
    validate_loaded_diagrams(&project, &diagrams)?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subject_boundary_and_actor_notation_round_trip_with_diagram_presentation() {
        let boundary_id = uuid::Uuid::new_v4().to_string();
        let diagram = BddDiagram {
            id: uuid::Uuid::new_v4().to_string(),
            name: "System Use Cases".into(),
            owner_id: uuid::Uuid::new_v4().to_string(),
            family: "use-case".into(),
            semantic_context_id: Some(uuid::Uuid::new_v4().to_string()),
            subject_boundary: Some(UseCaseSubjectBoundary {
                id: boundary_id.clone(),
                x: 315.0,
                y: 96.0,
                width: 640.0,
                height: 520.0,
            }),
            nodes: vec![DiagramNode {
                id: uuid::Uuid::new_v4().to_string(),
                element_id: uuid::Uuid::new_v4().to_string(),
                x: 90.0,
                y: 180.0,
                width: 110.0,
                height: 150.0,
                actor_notation: Some("rectangle".into()),
            }],
            edges: Vec::new(),
        };

        let encoded = serde_json::to_string(&diagram).expect("serialize Use Case presentation");
        let decoded: BddDiagram =
            serde_json::from_str(&encoded).expect("deserialize Use Case presentation");
        assert_eq!(decoded.subject_boundary.unwrap().id, boundary_id);
        assert_eq!(decoded.nodes[0].actor_notation.as_deref(), Some("rectangle"));
    }

    #[test]
    fn fitting_subject_boundary_uses_only_presented_use_cases() {
        let mut project = Project::new("P");
        let package = project
            .create_element(ElementKind::Package, "Package", project.root_id)
            .expect("package");
        let subject = project
            .create_element(ElementKind::Block, "System", package)
            .expect("subject");
        let actor = project
            .create_element(ElementKind::Actor, "Operator", package)
            .expect("actor");
        let use_case = project
            .create_element(ElementKind::UseCase, "Operate", package)
            .expect("use case");
        let actor_node = DiagramNode {
            id: uuid::Uuid::new_v4().to_string(),
            element_id: actor.to_string(),
            x: 40.0,
            y: 170.0,
            width: 110.0,
            height: 150.0,
            actor_notation: Some("stick".into()),
        };
        let use_case_node = DiagramNode {
            id: uuid::Uuid::new_v4().to_string(),
            element_id: use_case.to_string(),
            x: 520.0,
            y: 230.0,
            width: 210.0,
            height: 115.0,
            actor_notation: None,
        };
        let mut diagram = BddDiagram {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Use Cases".into(),
            owner_id: package.to_string(),
            family: "use-case".into(),
            semantic_context_id: Some(subject.to_string()),
            subject_boundary: Some(default_subject_boundary()),
            nodes: vec![actor_node.clone(), use_case_node.clone()],
            edges: Vec::new(),
        };

        fit_use_case_subject_boundary(&mut diagram, &project, true);
        let boundary = diagram.subject_boundary.as_ref().unwrap();
        assert_eq!(boundary.x, use_case_node.x - SUBJECT_PADDING_X);
        assert!(use_case_node.x + use_case_node.width <= boundary.x + boundary.width);
        assert!(actor_node.x + actor_node.width < boundary.x);
    }
}
