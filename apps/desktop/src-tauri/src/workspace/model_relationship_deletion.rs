use super::*;
use systems_modeler_core::RelationshipId;

fn parse_relationship(value: &str) -> Result<RelationshipId, String> {
    uuid::Uuid::parse_str(value)
        .map(RelationshipId)
        .map_err(|_| format!("invalid relationship id: {value}"))
}

/// Deletes one semantic relationship from the model while keeping presentation
/// cleanup transactional. Any remaining semantic reference (for example an
/// ItemFlow that still realizes a Connector) causes validation to reject the
/// deletion before workspace state is committed.
#[tauri::command]
pub fn delete_project_relationship(
    relationship_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let relationship_id = parse_relationship(&relationship_id)?;
    let relationship_key = relationship_id.to_string();

    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let mut bdd_diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let mut ibd_diagrams = workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")?
        .clone();
    let behavior = workspace
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?
        .clone();
    let behavior_diagrams = workspace
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .clone();
    let activity_repository = activity
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?
        .clone();
    let activity_diagrams = activity
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?
        .clone();

    if project.relationships.remove(&relationship_id).is_none() {
        return Err("relationship not found".into());
    }
    for diagram in &mut bdd_diagrams {
        diagram
            .edges
            .retain(|edge| edge.relationship_id != relationship_key);
    }
    for diagram in &mut ibd_diagrams {
        diagram
            .connectors
            .retain(|edge| edge.relationship_id != relationship_key);
    }

    project
        .validate()
        .map_err(|error| format!("Delete from Model rejected: {error}"))?;
    validate_loaded_diagrams(&project, &bdd_diagrams)?;
    ibd::validate_ibd_diagrams(&project, &ibd_diagrams)?;
    behavior_workspace::validate_behavior_workspace(
        &project,
        &behavior,
        &behavior_diagrams,
    )?;
    activity_repository
        .validate(&project)
        .map_err(|error| error.to_string())?;
    for owner_id in activity_diagrams.iter().map(|diagram| diagram.owner_id.as_str()) {
        project
            .element(parse_element_id(owner_id)?)
            .map_err(|error| error.to_string())?;
    }

    history::checkpoint_states(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = bdd_diagrams;
    *workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")? = ibd_diagrams;
    Ok(())
}
