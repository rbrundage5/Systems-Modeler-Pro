//! Rust-authoritative Requirement and traceability workspace operations.
//!
//! Requirement diagrams deliberately reuse the qualified structural diagram
//! geometry, routing, persistence, and history infrastructure.

use super::*;
use serde::Deserialize;
use systems_modeler_core::{ElementKind, RelationshipKind};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequirementUpdateRequest {
    element_id: String,
    name: String,
    requirement_id: String,
    text: String,
    documentation: String,
}

fn traceability_kind(value: &str) -> Result<RelationshipKind, String> {
    match value {
        "DeriveRequirement" | "deriveReqt" => Ok(RelationshipKind::DeriveRequirement),
        "Satisfy" | "satisfy" => Ok(RelationshipKind::Satisfy),
        "Verify" | "verify" => Ok(RelationshipKind::Verify),
        "Refine" | "refine" => Ok(RelationshipKind::Refine),
        "Trace" | "trace" => Ok(RelationshipKind::Trace),
        "Copy" | "copy" => Ok(RelationshipKind::Copy),
        _ => Err(format!("unsupported Requirement relationship: {value}")),
    }
}

fn checkpoint(
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &history::HistoryState,
) -> Result<(), String> {
    history::checkpoint_states(workspace, activity, history)
}

#[tauri::command]
pub fn create_requirement_diagram(
    owner_id: String,
    name: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let owner_id = parse_element_id(&owner_id)?;
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let owner = project
        .as_ref()
        .ok_or("no project open")?
        .element(owner_id)
        .map_err(|error| error.to_string())?;
    if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
        return Err("Requirement Diagram owner must be a Model or Package".into());
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
            family: "requirement".into(),
            nodes: Vec::new(),
            edges: Vec::new(),
        });
    Ok(id)
}

#[tauri::command]
pub fn create_requirement(
    owner_id: String,
    name: String,
    requirement_id: String,
    text: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let owner_id = parse_element_id(&owner_id)?;
    checkpoint(&workspace, &activity, &history)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    project
        .as_mut()
        .ok_or("no project open")?
        .create_requirement(name, requirement_id, text, owner_id)
        .map(|id| id.to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn create_test_case(
    owner_id: String,
    name: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let owner_id = parse_element_id(&owner_id)?;
    checkpoint(&workspace, &activity, &history)?;
    workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .as_mut()
        .ok_or("no project open")?
        .create_element(ElementKind::TestCase, name, owner_id)
        .map(|id| id.to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn update_requirement(
    details: RequirementUpdateRequest,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let element_id = parse_element_id(&details.element_id)?;
    checkpoint(&workspace, &activity, &history)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let project = project.as_mut().ok_or("no project open")?;
    project
        .update_requirement(element_id, details.requirement_id, details.text)
        .map_err(|error| error.to_string())?;
    let requirement = project
        .element_mut(element_id)
        .map_err(|error| error.to_string())?;
    requirement.name = details.name;
    requirement.documentation = details.documentation;
    Ok(())
}

#[tauri::command]
pub fn place_on_requirement_diagram(
    diagram_id: String,
    element_id: String,
    x: f64,
    y: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    if !x.is_finite() || !y.is_finite() {
        return Err("Requirement placement coordinates must be finite".into());
    }
    let element_id = parse_element_id(&element_id)?;
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let open_project = project.as_ref().ok_or("no project open")?;
    let element = open_project
        .element(element_id)
        .map_err(|error| error.to_string())?;
    let has_owned_content = open_project.children(element_id).next().is_some();
    let (width, height) = match element.kind {
        ElementKind::Requirement => (260.0, 180.0),
        ElementKind::TestCase => (220.0, 72.0),
        _ if has_owned_content => (220.0, 130.0),
        _ => (220.0, 58.0),
    };
    drop(project);
    checkpoint(&workspace, &activity, &history)?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|candidate| candidate.id == diagram_id && candidate.family == "requirement")
        .ok_or("Requirement Diagram not found")?;
    if diagram
        .nodes
        .iter()
        .any(|node| node.element_id == element_id.to_string())
    {
        return Err("element already has a presentation on this Requirement Diagram".into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    diagram.nodes.push(DiagramNode {
        id: id.clone(),
        element_id: element_id.to_string(),
        x,
        y,
        width,
        height,
    });
    Ok(id)
}

#[tauri::command]
pub fn reconnect_traceability_relationship(
    diagram_id: String,
    relationship_id: String,
    side: String,
    element_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    if side != "source" && side != "target" {
        return Err("relationship side must be source or target".into());
    }
    let relationship_id = parse_relationship_id(&relationship_id)?;
    let element_id = parse_element_id(&element_id)?;
    checkpoint(&workspace, &activity, &history)?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|candidate| candidate.id == diagram_id && candidate.family == "requirement")
        .ok_or("Requirement Diagram not found")?;
    if !diagram
        .nodes
        .iter()
        .any(|node| node.element_id == element_id.to_string())
    {
        return Err("replacement endpoint must be presented on this Requirement Diagram".into());
    }

    let mut project_guard = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_mut().ok_or("no project open")?;
    let original = project
        .relationship(relationship_id)
        .map_err(|error| error.to_string())?
        .clone();
    if !matches!(
        original.kind,
        RelationshipKind::DeriveRequirement
            | RelationshipKind::Satisfy
            | RelationshipKind::Verify
            | RelationshipKind::Refine
            | RelationshipKind::Trace
            | RelationshipKind::Copy
    ) {
        return Err("selected relationship is not Requirement traceability".into());
    }
    let (new_source, new_target) = if side == "source" {
        (element_id, original.target_id)
    } else {
        (original.source_id, element_id)
    };
    {
        let relationship = project
            .relationships
            .get_mut(&relationship_id)
            .ok_or("relationship not found")?;
        relationship.source_id = new_source;
        relationship.target_id = new_target;
    }
    if let Err(error) = project.validate() {
        project
            .relationships
            .insert(relationship_id, original.clone());
        return Err(error.to_string());
    }
    if original.kind == RelationshipKind::Copy {
        let master_text = project
            .element(new_target)
            .map_err(|error| error.to_string())?
            .requirement_text
            .clone();
        project
            .element_mut(new_source)
            .map_err(|error| error.to_string())?
            .requirement_text = master_text;
    }

    let source_node = diagram
        .nodes
        .iter()
        .find(|node| node.element_id == new_source.to_string())
        .cloned()
        .ok_or("new source endpoint must be presented on the Requirement Diagram")?;
    let target_node = diagram
        .nodes
        .iter()
        .find(|node| node.element_id == new_target.to_string())
        .cloned()
        .ok_or("new target endpoint must be presented on the Requirement Diagram")?;
    let edge = diagram
        .edges
        .iter_mut()
        .find(|edge| edge.relationship_id == relationship_id.to_string())
        .ok_or("Requirement relationship presentation not found")?;
    edge.source_node_id = source_node.id.clone();
    edge.target_node_id = target_node.id.clone();
    edge.points = route_relationship(&source_node, &target_node, &diagram.nodes)?;
    Ok(())
}

#[tauri::command]
pub fn create_traceability_relationship(
    diagram_id: String,
    relationship_kind: String,
    source_node_id: String,
    target_node_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let kind = traceability_kind(&relationship_kind)?;
    checkpoint(&workspace, &activity, &history)?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|candidate| candidate.id == diagram_id && candidate.family == "requirement")
        .ok_or("Requirement Diagram not found")?;
    let source = diagram
        .nodes
        .iter()
        .find(|node| node.id == source_node_id)
        .ok_or("source node not found")?;
    let target = diagram
        .nodes
        .iter()
        .find(|node| node.id == target_node_id)
        .ok_or("target node not found")?;
    let source_element = parse_element_id(&source.element_id)?;
    let target_element = parse_element_id(&target.element_id)?;
    let owner_id = parse_element_id(&diagram.owner_id)?;
    let points = vec![
        DiagramPoint {
            x: source.x + source.width / 2.0,
            y: source.y + source.height,
        },
        DiagramPoint {
            x: target.x + target.width / 2.0,
            y: target.y,
        },
    ];
    let relationship_id = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .as_mut()
        .ok_or("no project open")?
        .create_relationship(kind, source_element, target_element, Some(owner_id))
        .map_err(|error| error.to_string())?;
    diagram.edges.push(DiagramEdge {
        id: uuid::Uuid::new_v4().to_string(),
        relationship_id: relationship_id.to_string(),
        source_node_id,
        target_node_id,
        points,
        label_anchor: None,
    });
    Ok(relationship_id.to_string())
}
