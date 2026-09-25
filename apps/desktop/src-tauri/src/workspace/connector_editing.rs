use super::activity_workspace::ActivityWorkspaceState;
use super::history::{self, HistoryState};
use super::ibd::{self, IbdDiagram};
use super::*;
use serde::{Deserialize, Serialize};
use systems_modeler_core::{Connector, ConnectorEnd, ConnectorKind, RelationshipId};

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectorSpecificationEdit {
    pub name: String,
    pub kind: ConnectorKind,
    pub source_presentation_id: String,
    pub target_presentation_id: String,
}

#[derive(Debug, Serialize)]
pub struct ConnectorEndpointChoice {
    pub presentation_id: String,
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct ConnectorSpecification {
    pub name: String,
    pub kind: ConnectorKind,
    pub source_presentation_id: String,
    pub target_presentation_id: String,
    pub endpoints: Vec<ConnectorEndpointChoice>,
}

fn endpoint_ids(diagram: &IbdDiagram) -> Vec<String> {
    diagram
        .boundary_ports
        .iter()
        .map(|port| port.id.clone())
        .chain(diagram.properties.iter().flat_map(|property| {
            std::iter::once(property.id.clone())
                .chain(property.ports.iter().map(|port| port.id.clone()))
        }))
        .collect()
}

fn specification(
    project: &Project,
    diagram: &IbdDiagram,
    relationship_id: RelationshipId,
) -> Result<ConnectorSpecification, String> {
    let relationship = project
        .relationship(relationship_id)
        .map_err(|error| error.to_string())?;
    let connector = relationship
        .connector
        .as_ref()
        .ok_or("relationship is not a Connector")?;
    if connector.context_id != parse_element_id(&diagram.context_block_id)? {
        return Err("connector does not belong to the active IBD context".into());
    }
    let edge = diagram
        .connectors
        .iter()
        .find(|edge| edge.relationship_id == relationship_id.to_string())
        .ok_or("connector must be presented on the active IBD")?;
    let mut endpoints = Vec::new();
    for presentation_id in endpoint_ids(diagram) {
        let (end, _) = ibd::ibd_end_for_presentation(diagram, &presentation_id)?;
        project
            .validate_connector_end(connector.context_id, &end)
            .map_err(|error| error.to_string())?;
        let terminal = end.port_id.unwrap_or(end.role_id);
        let path = end
            .property_path
            .iter()
            .map(|id| {
                project
                    .element(*id)
                    .map(|element| element.name.clone())
                    .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?
            .join(" / ");
        let qualified = project
            .qualified_name(terminal)
            .map_err(|error| error.to_string())?;
        let label = if path.is_empty() {
            format!("Boundary: {qualified} [{presentation_id}]")
        } else {
            format!("{path}: {qualified} [{presentation_id}]")
        };
        endpoints.push(ConnectorEndpointChoice {
            presentation_id,
            label,
        });
    }
    Ok(ConnectorSpecification {
        name: relationship.name.clone(),
        kind: connector.kind,
        source_presentation_id: edge.source_presentation_id.clone(),
        target_presentation_id: edge.target_presentation_id.clone(),
        endpoints,
    })
}

#[tauri::command]
pub fn ibd_connector_specification(
    diagram_id: String,
    relationship_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
) -> Result<ConnectorSpecification, String> {
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let diagrams = workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("IBD not found")?;
    specification(
        project.as_ref().ok_or("no project open")?,
        diagram,
        parse_relationship_id(&relationship_id)?,
    )
}

fn presented_endpoint(
    diagram: &IbdDiagram,
    previous: &str,
    end: &ConnectorEnd,
) -> Result<String, String> {
    if ibd::ibd_end_for_presentation(diagram, previous).is_ok_and(|(current, _)| current == *end) {
        return Ok(previous.to_owned());
    }
    endpoint_ids(diagram).into_iter().find(|id| {
        ibd::ibd_end_for_presentation(diagram, id).is_ok_and(|(current, _)| current == *end)
    }).ok_or_else(|| format!("IBD '{}' must present the replacement endpoint before applying this connector edit", diagram.name))
}

fn stage_specification(
    project: &Project,
    diagrams: &[IbdDiagram],
    diagram_id: &str,
    relationship_id: RelationshipId,
    edit: &ConnectorSpecificationEdit,
) -> Result<(Project, Vec<IbdDiagram>), String> {
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("IBD not found")?;
    specification(project, diagram, relationship_id)?;
    let (source, _) = ibd::ibd_end_for_presentation(diagram, &edit.source_presentation_id)?;
    let (target, _) = ibd::ibd_end_for_presentation(diagram, &edit.target_presentation_id)?;
    let connector = Connector {
        context_id: parse_element_id(&diagram.context_block_id)?,
        kind: edit.kind,
        source,
        target,
    };
    let candidate =
        project.stage_connector_specification(relationship_id, &edit.name, connector.clone())?;
    let mut staged = diagrams.to_vec();
    for diagram in &mut staged {
        for index in 0..diagram.connectors.len() {
            let edge = &diagram.connectors[index];
            if edge.relationship_id != relationship_id.to_string() {
                continue;
            }
            let prefix = edge
                .context_path
                .iter()
                .map(|id| parse_element_id(id))
                .collect::<Result<Vec<_>, _>>()?;
            let source_end = super::ibd_projection::project_end(&prefix, &connector.source);
            let target_end = super::ibd_projection::project_end(&prefix, &connector.target);
            let source = presented_endpoint(diagram, &edge.source_presentation_id, &source_end)?;
            let target = presented_endpoint(diagram, &edge.target_presentation_id, &target_end)?;
            if source == edge.source_presentation_id && target == edge.target_presentation_id {
                continue;
            }
            let points = ibd::route_ibd_edge(diagram, &source, &target)?;
            let edge = &mut diagram.connectors[index];
            edge.source_presentation_id = source;
            edge.target_presentation_id = target;
            edge.label_anchor = Some(routing::route_label_anchor(&points));
            edge.points = points;
        }
    }
    ibd::validate_ibd_diagrams(&candidate, &staged)?;
    Ok((candidate, staged))
}

#[tauri::command]
pub fn update_ibd_connector_specification(
    diagram_id: String,
    relationship_id: String,
    edit: ConnectorSpecificationEdit,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<bool, String> {
    let id = parse_relationship_id(&relationship_id)?;
    history::apply_structural_specification(&workspace, &activity, &history, |project, diagrams| {
        stage_specification(project, diagrams, &diagram_id, id, &edit)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use systems_modeler_core::{ElementKind, ItemFlow, Multiplicity};

    fn fixture() -> (Project, Vec<IbdDiagram>, RelationshipId) {
        let (mut project, mut diagram) = super::super::ibd_geometry::tests::fixture();
        let original = &diagram.properties[0];
        let source = project
            .element(parse_element_id(&original.element_id).unwrap())
            .unwrap()
            .clone();
        let part = project
            .create_typed_feature(
                ElementKind::PartProperty,
                "second",
                source.owner_id.unwrap(),
                source.type_id.unwrap(),
                Multiplicity::ONE,
            )
            .unwrap();
        let mut copy = original.clone();
        copy.id = "second-part".into();
        copy.element_id = part.to_string();
        copy.property_path = vec![part.to_string()];
        copy.x = 600.0;
        copy.ports[0].id = "second-port".into();
        copy.ports[0].property_path = copy.property_path.clone();
        copy.ports[0].x = 600.0;
        diagram.properties.push(copy);
        let id = parse_relationship_id(&diagram.connectors[0].relationship_id).unwrap();
        let connector = project
            .relationship(id)
            .unwrap()
            .connector
            .as_ref()
            .unwrap()
            .clone();
        for (source, target) in [
            (connector.source.clone(), connector.target.clone()),
            (connector.target, connector.source),
        ] {
            project
                .create_item_flow(ItemFlow {
                    connector_id: id,
                    source,
                    target,
                    conveyed_item_ids: vec![source_type(&project, &diagram)],
                })
                .unwrap();
        }
        let mut other = diagram.clone();
        other.id = uuid::Uuid::new_v4().to_string();
        for property in &mut other.properties {
            property.id.push_str("-other");
            for port in &mut property.ports {
                port.id.push_str("-other");
            }
        }
        for port in &mut other.boundary_ports {
            port.id.push_str("-other");
        }
        for edge in &mut other.connectors {
            edge.id.push_str("-other");
            edge.source_presentation_id.push_str("-other");
            edge.target_presentation_id.push_str("-other");
        }
        (project, vec![diagram, other], id)
    }

    fn source_type(project: &Project, diagram: &IbdDiagram) -> ElementId {
        project
            .element(parse_element_id(&diagram.properties[0].element_id).unwrap())
            .unwrap()
            .type_id
            .unwrap()
    }

    fn edit() -> ConnectorSpecificationEdit {
        ConnectorSpecificationEdit {
            name: "Data interface".into(),
            kind: ConnectorKind::Delegation,
            source_presentation_id: "external".into(),
            target_presentation_id: "second-port".into(),
        }
    }

    #[test]
    fn connector_specification_rewires_all_views_and_preserves_identity_and_flows() {
        let (project, diagrams, id) = fixture();
        let view = specification(&project, &diagrams[0], id).unwrap();
        assert!(view.endpoints.iter().any(
            |choice| choice.presentation_id == "second-port" && choice.label.contains("second")
        ));
        let (candidate, result) =
            stage_specification(&project, &diagrams, &diagrams[0].id, id, &edit()).unwrap();
        assert_eq!(candidate.relationship(id).unwrap().name, "Data interface");
        assert_eq!(
            candidate.relationship(id).unwrap().external_id,
            project.relationship(id).unwrap().external_id
        );
        for (index, diagram) in result.iter().enumerate() {
            assert_eq!(diagram.connectors[0].id, diagrams[index].connectors[0].id);
            assert!(
                diagram.connectors[0]
                    .target_presentation_id
                    .starts_with("second-port")
            );
        }
        for relationship in candidate.relationships.values() {
            if let Some(flow) = &relationship.item_flow {
                assert!(
                    flow.source.property_path
                        == vec![parse_element_id(&diagrams[0].properties[1].element_id).unwrap()]
                        || flow.target.property_path
                            == vec![
                                parse_element_id(&diagrams[0].properties[1].element_id).unwrap()
                            ]
                );
            }
        }
        let decoded: Project =
            serde_json::from_value(serde_json::to_value(&candidate).unwrap()).unwrap();
        ibd::validate_ibd_diagrams(&decoded, &result).unwrap();
    }

    #[test]
    fn connector_end_order_does_not_reverse_physical_item_flow_direction() {
        let (project, diagrams, id) = fixture();
        let mut connector = project.relationship(id).unwrap().connector.clone().unwrap();
        std::mem::swap(&mut connector.source, &mut connector.target);
        let candidate = project
            .stage_connector_specification(id, "Reversed display order", connector)
            .unwrap();
        for (key, relationship) in &project.relationships {
            if relationship.item_flow.is_some() {
                assert_eq!(
                    serde_json::to_value(&relationship.item_flow).unwrap(),
                    serde_json::to_value(&candidate.relationship(*key).unwrap().item_flow).unwrap()
                );
            }
        }
        let mut invalid = edit();
        invalid.kind = ConnectorKind::Assembly;
        assert!(stage_specification(&project, &diagrams, &diagrams[0].id, id, &invalid).is_err());
        let mut missing = diagrams.clone();
        missing[1].properties.pop();
        assert!(
            stage_specification(&project, &missing, &diagrams[0].id, id, &edit())
                .unwrap_err()
                .contains("must present")
        );
    }

    #[test]
    fn connector_transaction_undo_redo_noop_and_rejection_are_atomic() {
        let (project, diagrams, id) = fixture();
        let diagram_id = diagrams[0].id.clone();
        let workspace = WorkspaceState::default();
        let activity = ActivityWorkspaceState::default();
        let history = HistoryState::default();
        *workspace.project.lock().unwrap() = Some(project);
        *workspace.ibd_diagrams.lock().unwrap() = diagrams;
        let before = serde_json::to_value(&*workspace.project.lock().unwrap()).unwrap();
        let apply = |edit: &ConnectorSpecificationEdit| {
            history::apply_structural_specification(
                &workspace,
                &activity,
                &history,
                |project, diagrams| stage_specification(project, diagrams, &diagram_id, id, edit),
            )
        };
        assert!(apply(&edit()).unwrap());
        assert_eq!(history::undo_len(&history), 1);
        assert!(!apply(&edit()).unwrap());
        assert_eq!(history::undo_len(&history), 1);
        let after = serde_json::to_value(&*workspace.project.lock().unwrap()).unwrap();
        assert!(history::undo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(
            serde_json::to_value(&*workspace.project.lock().unwrap()).unwrap(),
            before
        );
        let mut invalid = edit();
        invalid.kind = ConnectorKind::Assembly;
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
    }
}
