use super::activity_workspace::ActivityWorkspaceState;
use super::history::{self, HistoryState};
use super::*;
use serde::{Deserialize, Serialize};
use systems_modeler_core::{ConnectorEnd, ItemFlow, RelationshipId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum ItemFlowDirection {
    Forward,
    Reverse,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemFlowSpecificationEdit {
    pub name: String,
    pub direction: ItemFlowDirection,
    pub conveyed_item_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ConveyedClassifierChoice {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct ItemFlowSpecification {
    pub name: String,
    pub direction: ItemFlowDirection,
    pub conveyed_item_ids: Vec<String>,
    pub source_label: String,
    pub target_label: String,
    pub classifiers: Vec<ConveyedClassifierChoice>,
}

fn endpoint_label(project: &Project, end: &ConnectorEnd) -> Result<String, String> {
    let mut ids = end.property_path.clone();
    let terminal = end.port_id.unwrap_or(end.role_id);
    if ids.last() != Some(&terminal) {
        ids.push(terminal);
    }
    ids.iter()
        .map(|id| {
            project
                .qualified_name(*id)
                .map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|labels| labels.join(" / "))
}

fn specification(project: &Project, id: RelationshipId) -> Result<ItemFlowSpecification, String> {
    let relationship = project
        .relationship(id)
        .map_err(|error| error.to_string())?;
    let flow = relationship
        .item_flow
        .as_ref()
        .ok_or("relationship is not an ItemFlow")?;
    project
        .validate_item_flow(flow)
        .map_err(|error| error.to_string())?;
    let connector = project
        .relationship(flow.connector_id)
        .map_err(|error| error.to_string())?
        .connector
        .as_ref()
        .ok_or("realizing relationship is not a Connector")?;
    let mut classifiers = project
        .elements
        .values()
        .filter(|element| element.is_classifier())
        .map(|element| {
            let qualified = project
                .qualified_name(element.id)
                .map_err(|error| error.to_string())?;
            Ok(ConveyedClassifierChoice {
                id: element.id.to_string(),
                label: format!("{qualified} [{}]", element.id),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    classifiers.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(ItemFlowSpecification {
        name: relationship.name.clone(),
        direction: if flow.source == connector.source {
            ItemFlowDirection::Forward
        } else {
            ItemFlowDirection::Reverse
        },
        conveyed_item_ids: flow
            .conveyed_item_ids
            .iter()
            .map(ToString::to_string)
            .collect(),
        source_label: endpoint_label(project, &connector.source)?,
        target_label: endpoint_label(project, &connector.target)?,
        classifiers,
    })
}

fn stage_specification(
    project: &Project,
    id: RelationshipId,
    edit: &ItemFlowSpecificationEdit,
) -> Result<Project, String> {
    let relationship = project
        .relationship(id)
        .map_err(|error| error.to_string())?;
    let old = relationship
        .item_flow
        .as_ref()
        .ok_or("relationship is not an ItemFlow")?;
    let connector = project
        .relationship(old.connector_id)
        .map_err(|error| error.to_string())?
        .connector
        .as_ref()
        .ok_or("realizing relationship is not a Connector")?;
    let (source, target) = match edit.direction {
        ItemFlowDirection::Forward => (connector.source.clone(), connector.target.clone()),
        ItemFlowDirection::Reverse => (connector.target.clone(), connector.source.clone()),
    };
    project.stage_item_flow_specification(
        id,
        &edit.name,
        ItemFlow {
            connector_id: old.connector_id,
            source,
            target,
            conveyed_item_ids: edit
                .conveyed_item_ids
                .iter()
                .map(|id| parse_element_id(id))
                .collect::<Result<_, _>>()?,
        },
    )
}

#[tauri::command]
pub fn ibd_item_flow_specification(
    relationship_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
) -> Result<ItemFlowSpecification, String> {
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    specification(
        project.as_ref().ok_or("no project open")?,
        parse_relationship_id(&relationship_id)?,
    )
}

#[tauri::command]
pub fn update_ibd_item_flow_specification(
    relationship_id: String,
    edit: ItemFlowSpecificationEdit,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<bool, String> {
    let id = parse_relationship_id(&relationship_id)?;
    history::apply_structural_specification(&workspace, &activity, &history, |project, diagrams| {
        Ok((stage_specification(project, id, &edit)?, diagrams.to_vec()))
    })
}

#[cfg(test)]
mod tests {
    use super::super::ibd::IbdDiagram;
    use super::*;

    fn fixture() -> (
        Project,
        IbdDiagram,
        RelationshipId,
        ItemFlowSpecificationEdit,
    ) {
        let (mut project, diagram) = super::super::ibd_geometry::tests::fixture();
        let connector_id = parse_relationship_id(&diagram.connectors[0].relationship_id).unwrap();
        let connector = project
            .relationship(connector_id)
            .unwrap()
            .connector
            .clone()
            .unwrap();
        let classifier = parse_element_id(&diagram.context_block_id).unwrap();
        let id = project
            .create_item_flow(ItemFlow {
                connector_id,
                source: connector.source,
                target: connector.target,
                conveyed_item_ids: vec![classifier],
            })
            .unwrap();
        let edit = ItemFlowSpecificationEdit {
            name: " Return data ".into(),
            direction: ItemFlowDirection::Reverse,
            conveyed_item_ids: vec![classifier.to_string()],
        };
        (project, diagram, id, edit)
    }

    #[test]
    fn flow_edit_preserves_identity_and_connector_and_reverses_semantic_ends() {
        let (project, diagram, id, edit) = fixture();
        let before = project.relationship(id).unwrap();
        let candidate = stage_specification(&project, id, &edit).unwrap();
        let after = candidate.relationship(id).unwrap();
        assert_eq!(after.external_id, before.external_id);
        assert_eq!(after.id, before.id);
        assert_eq!(after.name, "Return data");
        let old = before.item_flow.as_ref().unwrap();
        let flow = after.item_flow.as_ref().unwrap();
        assert_eq!(flow.connector_id, old.connector_id);
        assert_eq!(flow.source, old.target);
        assert_eq!(flow.target, old.source);
        assert_eq!(after.source_id, before.target_id);
        let view = specification(&candidate, id).unwrap();
        assert_eq!(view.direction, ItemFlowDirection::Reverse);
        assert!(view.classifiers.iter().any(|choice| choice.id == edit.conveyed_item_ids[0] && choice.label.contains("::")));
        assert!(
            !view
                .classifiers
                .iter()
                .any(|choice| choice.id == diagram.properties[0].element_id)
        );
        let decoded: Project =
            serde_json::from_value(serde_json::to_value(&candidate).unwrap()).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded.relationship(id).unwrap().item_flow, after.item_flow);
    }

    #[test]
    fn invalid_conveyed_sets_and_non_flow_ids_leave_model_unchanged() {
        let (project, diagram, id, edit) = fixture();
        let before = serde_json::to_value(&project).unwrap();
        for conveyed in [
            vec![],
            vec![edit.conveyed_item_ids[0].clone(); 2],
            vec![diagram.properties[0].element_id.clone()],
            vec![uuid::Uuid::new_v4().to_string()],
        ] {
            let mut invalid = edit.clone();
            invalid.conveyed_item_ids = conveyed;
            assert!(stage_specification(&project, id, &invalid).is_err());
        }
        assert!(
            stage_specification(
                &project,
                parse_relationship_id(&diagram.connectors[0].relationship_id).unwrap(),
                &edit
            )
            .is_err()
        );
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
    }

    #[test]
    fn flow_transaction_noop_undo_redo_and_rejected_edit_preserve_history() {
        let (project, diagram, id, edit) = fixture();
        let workspace = WorkspaceState::default();
        let activity = ActivityWorkspaceState::default();
        let history = HistoryState::default();
        *workspace.project.lock().unwrap() = Some(project);
        *workspace.ibd_diagrams.lock().unwrap() = vec![diagram];
        let before = serde_json::to_value(&*workspace.project.lock().unwrap()).unwrap();
        let diagrams_before =
            serde_json::to_value(&*workspace.ibd_diagrams.lock().unwrap()).unwrap();
        let apply = |edit: &ItemFlowSpecificationEdit| {
            history::apply_structural_specification(
                &workspace,
                &activity,
                &history,
                |project, diagrams| {
                    Ok((stage_specification(project, id, edit)?, diagrams.to_vec()))
                },
            )
        };
        assert!(apply(&edit).unwrap());
        assert!(!apply(&edit).unwrap());
        assert_eq!(history::undo_len(&history), 1);
        let after = serde_json::to_value(&*workspace.project.lock().unwrap()).unwrap();
        assert!(history::undo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(
            serde_json::to_value(&*workspace.project.lock().unwrap()).unwrap(),
            before
        );
        let mut invalid = edit.clone();
        invalid.conveyed_item_ids.clear();
        assert!(apply(&invalid).is_err());
        assert_eq!(history::undo_len(&history), 0);
        assert_eq!(
            serde_json::to_value(&*workspace.project.lock().unwrap()).unwrap(),
            before
        );
        assert!(history::redo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(
            serde_json::to_value(&*workspace.project.lock().unwrap()).unwrap(),
            after
        );
        assert_eq!(
            serde_json::to_value(&*workspace.ibd_diagrams.lock().unwrap()).unwrap(),
            diagrams_before
        );
    }
}
