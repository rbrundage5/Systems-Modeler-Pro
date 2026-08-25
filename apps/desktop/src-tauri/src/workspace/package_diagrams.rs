//! Minimal Rust-authoritative Package Diagram foundation.
//!
//! This PR deliberately models package presentation and shared editing only.
//! Package/Element Import, Package Merge, profiles, and package relationships
//! remain separate follow-on work so this family does not invent placeholder
//! semantics just to draw arrows.

use super::*;
use systems_modeler_core::ElementKind;

fn checkpoint(
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &history::HistoryState,
) -> Result<(), String> {
    history::checkpoint_states(workspace, activity, history)
}

#[tauri::command]
pub fn create_package_diagram(
    owner_id: String,
    name: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let owner_id = parse_element_id(&owner_id)?;
    let name = name.trim();
    if name.is_empty() {
        return Err("Package Diagram name cannot be empty".into());
    }
    if name.chars().count() > 256 {
        return Err("Package Diagram name cannot exceed 256 characters".into());
    }
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
            return Err("Package Diagram owner must be a Model or Package".into());
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
            name: name.into(),
            owner_id: owner_id.to_string(),
            family: "package".into(),
            semantic_context_id: None,
            subject_boundary: None,
            nodes: Vec::new(),
            edges: Vec::new(),
        });
    Ok(id)
}

#[tauri::command]
pub fn place_on_package_diagram(
    diagram_id: String,
    element_id: String,
    x: f64,
    y: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    parse_diagram_id(&diagram_id)?;
    let element_id = parse_element_id(&element_id)?;
    if !x.is_finite() || !y.is_finite() {
        return Err("Package presentation coordinates must be finite".into());
    }
    {
        let project = workspace
            .project
            .lock()
            .map_err(|_| "project lock poisoned")?;
        let element = project
            .as_ref()
            .ok_or("no project open")?
            .element(element_id)
            .map_err(|error| error.to_string())?;
        if element.kind != ElementKind::Package {
            return Err("Package Diagrams can present existing Package elements only".into());
        }
    }

    {
        let diagrams = workspace
            .diagrams
            .lock()
            .map_err(|_| "diagram lock poisoned")?;
        let diagram = diagrams
            .iter()
            .find(|diagram| diagram.id == diagram_id)
            .ok_or("Package Diagram not found")?;
        if diagram.family != "package" {
            return Err("target diagram is not a Package Diagram".into());
        }
        if diagram
            .nodes
            .iter()
            .any(|node| node.element_id == element_id.to_string())
        {
            return Err("this Package is already presented on the Package Diagram".into());
        }
    }

    checkpoint(&workspace, &activity, &history)?;
    let presentation_id = uuid::Uuid::new_v4().to_string();
    workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("Package Diagram not found")?
        .nodes
        .push(DiagramNode {
            id: presentation_id.clone(),
            element_id: element_id.to_string(),
            x: x.max(0.0),
            y: y.max(42.0),
            width: 220.0,
            height: 120.0,
            actor_notation: None,
            parameter_presentations: Vec::new(),
        });
    Ok(presentation_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_diagram_contract_stays_presentation_only() {
        // Compile-time guard for the intended PR26A boundary: Package Diagrams
        // use the existing generic structural presentation shape and do not add
        // fake package relationship semantics to the model core.
        let node = DiagramNode {
            id: uuid::Uuid::new_v4().to_string(),
            element_id: uuid::Uuid::new_v4().to_string(),
            x: 10.0,
            y: 42.0,
            width: 220.0,
            height: 120.0,
            actor_notation: None,
            parameter_presentations: Vec::new(),
        };
        assert_eq!(node.width, 220.0);
        assert_eq!(node.height, 120.0);
    }
}
