use super::*;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ItemFlowNotationSnapshot {
    pub relationship_id: String,
    pub connector_id: String,
    pub conveyed_item_ids: Vec<String>,
    pub conveyed_item_names: Vec<String>,
}

#[tauri::command]
pub fn ibd_item_flow_notation(
    state: tauri::State<'_, WorkspaceState>,
) -> Result<Vec<ItemFlowNotationSnapshot>, String> {
    let project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;

    let mut result = Vec::new();
    for relationship in project.relationships.values() {
        let Some(flow) = relationship.item_flow.as_ref() else {
            continue;
        };
        let mut ids = Vec::new();
        let mut names = Vec::new();
        for item_id in &flow.conveyed_item_ids {
            let item = project
                .element(*item_id)
                .map_err(|error| error.to_string())?;
            ids.push(item.id.to_string());
            names.push(item.name.clone());
        }
        result.push(ItemFlowNotationSnapshot {
            relationship_id: relationship.id.to_string(),
            connector_id: flow.connector_id.to_string(),
            conveyed_item_ids: ids,
            conveyed_item_names: names,
        });
    }
    result.sort_by(|a, b| a.relationship_id.cmp(&b.relationship_id));
    Ok(result)
}
