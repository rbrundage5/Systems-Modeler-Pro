//! Rust-authoritative Use Case semantics and diagram mutations.
//!
//! Use Case diagrams reuse the structural presentation store, routing, layout,
//! history, persistence, and standard-editing services. This module only adds
//! semantic eligibility and Use Case-specific property updates.

use super::*;
use systems_modeler_core::{AggregationKind, ElementKind, Multiplicity, RelationshipKind};

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
            return Err("Use Case diagram subject must be a represented system classifier".into());
        }
    }
    drop(project);
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
            nodes: Vec::new(),
            edges: Vec::new(),
        });
    Ok(id)
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
        .map_err(|_| "project lock poisoned")?;
    let element = project
        .as_ref()
        .ok_or("no project open")?
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
    drop(project);
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
    });
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
