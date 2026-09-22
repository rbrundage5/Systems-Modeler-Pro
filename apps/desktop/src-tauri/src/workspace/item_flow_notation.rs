use super::*;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ItemFlowNotationSnapshot {
    pub relationship_id: String,
    pub connector_id: String,
    pub name: String,
    pub direction: super::item_flow_editing::ItemFlowDirection,
    pub conveyed_item_ids: Vec<String>,
    pub conveyed_item_names: Vec<String>,
}

#[tauri::command]
pub fn ibd_item_flow_notation(
    state: tauri::State<'_, WorkspaceState>,
) -> Result<Vec<ItemFlowNotationSnapshot>, String> {
    let project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;

    notation(project)
}

fn notation(project: &Project) -> Result<Vec<ItemFlowNotationSnapshot>, String> {
    use super::item_flow_editing::ItemFlowDirection;
    let mut result = Vec::new();
    for relationship in project.relationships.values() {
        let Some(flow) = relationship.item_flow.as_ref() else {
            continue;
        };
        project
            .validate_item_flow(flow)
            .map_err(|error| error.to_string())?;
        let connector = project
            .relationship(flow.connector_id)
            .map_err(|error| error.to_string())?
            .connector
            .as_ref()
            .ok_or("ItemFlow realizing relationship is not a Connector")?;
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
            name: relationship.name.clone(),
            direction: if flow.source == connector.source {
                ItemFlowDirection::Forward
            } else {
                ItemFlowDirection::Reverse
            },
            conveyed_item_ids: ids,
            conveyed_item_names: names,
        });
    }
    result.sort_by(|a, b| a.relationship_id.cmp(&b.relationship_id));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::super::item_flow_editing::ItemFlowDirection;
    use super::*;
    use systems_modeler_core::ItemFlow;

    #[test]
    fn notation_preserves_opposite_flow_identities_and_validates_endpoints() {
        let (mut project, diagram) = super::super::ibd_geometry::tests::fixture();
        let connector_id = parse_relationship_id(&diagram.connectors[0].relationship_id).unwrap();
        let connector = project
            .relationship(connector_id)
            .unwrap()
            .connector
            .clone()
            .unwrap();
        let conveyed = parse_element_id(&diagram.context_block_id).unwrap();
        for (source, target) in [
            (connector.source.clone(), connector.target.clone()),
            (connector.target, connector.source),
        ] {
            project
                .create_item_flow(ItemFlow {
                    connector_id,
                    source,
                    target,
                    conveyed_item_ids: vec![conveyed],
                })
                .unwrap();
        }
        let rows = notation(&project).unwrap();
        assert_eq!(rows.len(), 2);
        assert_ne!(rows[0].relationship_id, rows[1].relationship_id);
        assert_eq!(rows[0].conveyed_item_ids, rows[1].conveyed_item_ids);
        assert!(
            rows.iter()
                .any(|row| row.direction == ItemFlowDirection::Forward)
        );
        assert!(
            rows.iter()
                .any(|row| row.direction == ItemFlowDirection::Reverse)
        );
        let flow_id = parse_relationship_id(&rows[0].relationship_id).unwrap();
        let flow = project
            .relationships
            .get_mut(&flow_id)
            .unwrap()
            .item_flow
            .as_mut()
            .unwrap();
        flow.source = flow.target.clone();
        assert!(notation(&project).is_err());
    }
}
