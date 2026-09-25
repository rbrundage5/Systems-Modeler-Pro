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
    /// None preserves existing typing for older callers; Some with empty ID untypes.
    #[serde(default)]
    pub typing: Option<ConnectorTypingEdit>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectorTypingEdit {
    pub association_type_id: Option<String>,
    pub source_multiplicity: String,
    pub target_multiplicity: String,
}

#[derive(Debug, Serialize)]
pub struct ConnectorAssociationChoice {
    pub id: String,
    pub label: String,
    pub ends: [String; 2],
}

fn association_choices(project: &Project) -> Result<Vec<ConnectorAssociationChoice>, String> {
    let mut choices = Vec::new();
    for relationship in project.relationships.values().filter(|relationship| {
        matches!(relationship.kind, RelationshipKind::Association | RelationshipKind::Composition)
            && relationship.association_ends.len() == 2
    }) {
        let end_label = |index: usize| -> Result<String, String> {
            let end = &relationship.association_ends[index];
            let classifier = project.qualified_name(end.classifier_id).map_err(|error| error.to_string())?;
            Ok(format!("{}: {} [{}]", end.role_name, classifier, end.multiplicity.notation()))
        };
        let owner = relationship.owner_id.map(|id| project.qualified_name(id)).transpose().map_err(|error| error.to_string())?.unwrap_or_default();
        choices.push(ConnectorAssociationChoice {
            id: relationship.id.to_string(),
            label: format!("{}::{} [{}]", owner, if relationship.name.is_empty() { "Association" } else { &relationship.name }, relationship.id),
            ends: [end_label(0)?, end_label(1)?],
        });
    }
    choices.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(choices)
}

pub(super) fn connector_label(project: &Project, relationship: &systems_modeler_core::Relationship) -> Option<String> {
    let connector = relationship.connector.as_ref()?;
    let Some(type_id) = connector.association_type_id else { return Some(relationship.name.clone()); };
    let association = project.relationship(type_id).ok()?;
    let type_name = if association.name.is_empty() { "Association" } else { &association.name };
    Some(format!("{}: {}", relationship.name, type_name))
}

#[tauri::command]
pub fn ibd_connector_type_choices(workspace: tauri::State<'_, WorkspaceState>) -> Result<Vec<ConnectorAssociationChoice>, String> {
    let guard = workspace.project.lock().map_err(|_| "project lock poisoned")?;
    association_choices(guard.as_ref().ok_or("no project open")?)
}

fn apply_typing(connector: &mut Connector, typing: Option<&ConnectorTypingEdit>) -> Result<(), String> {
    if let Some(typing) = typing {
        connector.association_type_id = typing.association_type_id.as_deref()
            .filter(|id| !id.is_empty()).map(parse_relationship_id).transpose()?;
        connector.end_multiplicities = [
            super::parametrics::parse_multiplicity(&typing.source_multiplicity)?,
            super::parametrics::parse_multiplicity(&typing.target_multiplicity)?,
        ];
    }
    Ok(())
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
    pub association_type_id: Option<String>,
    pub source_multiplicity: String,
    pub target_multiplicity: String,
    pub associations: Vec<ConnectorAssociationChoice>,
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
        association_type_id: connector.association_type_id.map(|id| id.to_string()),
        source_multiplicity: connector.end_multiplicities[0].notation(),
        target_multiplicity: connector.end_multiplicities[1].notation(),
        associations: association_choices(project)?,
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
    let old = project
        .relationship(relationship_id)
        .map_err(|error| error.to_string())?
        .connector
        .as_ref()
        .ok_or("relationship is not a Connector")?;
    let mut connector = Connector {
        association_type_id: old.association_type_id,
        end_multiplicities: old.end_multiplicities,
        context_id: parse_element_id(&diagram.context_block_id)?,
        kind: edit.kind,
        source,
        target,
    };
    apply_typing(&mut connector, edit.typing.as_ref())?;
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

fn stage_creation(project: &Project, diagrams: &[IbdDiagram], diagram_id: &str, edit: &ConnectorSpecificationEdit) -> Result<(Project, Vec<IbdDiagram>, RelationshipId), String> {
    let mut candidate = project.clone();
    let mut staged = diagrams.to_vec();
    let diagram = staged.iter_mut().find(|diagram| diagram.id == diagram_id).ok_or("IBD not found")?;
    let (source, _) = ibd::ibd_end_for_presentation(diagram, &edit.source_presentation_id)?;
    let (target, _) = ibd::ibd_end_for_presentation(diagram, &edit.target_presentation_id)?;
    let mut connector = Connector {
        context_id: parse_element_id(&diagram.context_block_id)?, kind: edit.kind, source, target,
        association_type_id: None, end_multiplicities: Default::default(),
    };
    apply_typing(&mut connector, edit.typing.as_ref())?;
    let id = candidate.create_connector(connector).map_err(|error| error.to_string())?;
    candidate.relationships.get_mut(&id).unwrap().name = edit.name.trim().to_owned();
    let points = ibd::route_ibd_edge(diagram, &edit.source_presentation_id, &edit.target_presentation_id)?;
    diagram.connectors.push(ibd::IbdConnectorPresentation {
        id: uuid::Uuid::new_v4().to_string(), relationship_id: id.to_string(), context_path: Vec::new(),
        source_presentation_id: edit.source_presentation_id.clone(), target_presentation_id: edit.target_presentation_id.clone(),
        label_anchor: Some(routing::route_label_anchor(&points)), points,
    });
    candidate.validate().map_err(|error| error.to_string())?;
    ibd::validate_ibd_diagrams(&candidate, &staged)?;
    Ok((candidate, staged, id))
}

#[tauri::command]
pub fn create_ibd_connector_specification(diagram_id: String, edit: ConnectorSpecificationEdit,
    workspace: tauri::State<'_, WorkspaceState>, activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>) -> Result<String, String> {
    let mut created = None;
    history::apply_structural_specification(&workspace, &activity, &history, |project, diagrams| {
        let (candidate, staged, id) = stage_creation(project, diagrams, &diagram_id, &edit)?;
        created = Some(id);
        Ok((candidate, staged))
    })?;
    created.map(|id| id.to_string()).ok_or("Connector was not created".into())
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

    #[test]
    fn typed_creation_supports_different_components_and_atomic_history() {
        let mut project = Project::new("Laboratory");
        let root = project.root_id;
        let context = project.create_element(ElementKind::Block, "Installation", root).unwrap();
        let producer = project.create_element(ElementKind::Block, "Producer", root).unwrap();
        let consumer = project.create_element(ElementKind::Block, "Consumer", root).unwrap();
        let source = project.create_typed_feature(ElementKind::PartProperty, "source", context, producer, Multiplicity::ONE).unwrap();
        let target = project.create_typed_feature(ElementKind::PartProperty, "destination", context, consumer, Multiplicity::ONE).unwrap();
        let association = project.create_association(Some(root), vec![
            Project::association_end(producer, "provider", Multiplicity::ONE, true, AggregationKind::None),
            Project::association_end(consumer, "client", Multiplicity::ONE, true, AggregationKind::None),
        ]).unwrap();
        project.relationships.get_mut(&association).unwrap().name = "Transfer".into();
        let diagram_id = uuid::Uuid::new_v4().to_string();
        let diagram = IbdDiagram {
            id: diagram_id.clone(), name: "Installation internals".into(), owner_id: root.to_string(),
            context_block_id: context.to_string(), context_frame: None, boundary_ports: Vec::new(), connectors: Vec::new(),
            properties: [source, target].iter().enumerate().map(|(index, id)| ibd::IbdPropertyPresentation {
                id: format!("p{index}"), element_id: id.to_string(), property_path: vec![id.to_string()],
                collapsed: false, x: 80.0 + index as f64 * 350.0, y: 80.0, width: 220.0, height: 100.0, ports: Vec::new(),
            }).collect(),
        };
        let workspace = WorkspaceState::default();
        let activity = ActivityWorkspaceState::default();
        let history = HistoryState::default();
        *workspace.project.lock().unwrap() = Some(project);
        *workspace.ibd_diagrams.lock().unwrap() = vec![diagram];
        let edit = ConnectorSpecificationEdit {
            name: "transfer".into(), kind: ConnectorKind::Assembly,
            source_presentation_id: "p0".into(), target_presentation_id: "p1".into(),
            typing: Some(ConnectorTypingEdit { association_type_id: Some(association.to_string()), source_multiplicity: "1".into(), target_multiplicity: "1".into() }),
        };
        let apply = |edit: &ConnectorSpecificationEdit| {
            let mut id = None;
            history::apply_structural_specification(&workspace, &activity, &history, |project, diagrams| {
                let (candidate, staged, created) = stage_creation(project, diagrams, &diagram_id, edit)?;
                id = Some(created);
                Ok((candidate, staged))
            })?;
            Ok::<_, String>(id.unwrap())
        };
        let id = apply(&edit).unwrap();
        assert_eq!(history::undo_len(&history), 1);
        {
            let guard = workspace.project.lock().unwrap();
            let project = guard.as_ref().unwrap();
            let relationship = project.relationship(id).unwrap();
            assert_eq!(relationship.owner_id, Some(context));
            assert_eq!(connector_label(project, relationship).as_deref(), Some("transfer: Transfer"));
            let snapshot = super::super::snapshot_project(project);
            let rendered = snapshot.relationships.iter().find(|relationship| relationship.id == id.to_string()).unwrap();
            assert_eq!(rendered.connector.as_ref().unwrap().association_type_id, Some(association));
            assert_eq!(rendered.connector_label.as_deref(), Some("transfer: Transfer"));
            assert_eq!(association_choices(project).unwrap()[0].id, association.to_string());
            assert_eq!(project.element(source).unwrap().type_id, Some(producer));
        }
        assert!(history::undo_states(&workspace, &activity, &history).unwrap());
        let before = serde_json::to_value(&*workspace.project.lock().unwrap()).unwrap();
        let mut invalid = edit;
        invalid.typing.as_mut().unwrap().target_multiplicity = "0..*".into();
        assert!(apply(&invalid).is_err());
        assert_eq!(serde_json::to_value(&*workspace.project.lock().unwrap()).unwrap(), before);
        assert_eq!(history::undo_len(&history), 0);
        assert!(history::redo_states(&workspace, &activity, &history).unwrap());
        assert!(workspace.project.lock().unwrap().as_ref().unwrap().relationship(id).is_ok());
    }

    #[test]
    fn typing_edit_preserves_item_flow_direction_and_legacy_edits_retain_type() {
        let (mut project, diagrams, id) = fixture();
        let port = project.relationship(id).unwrap().connector.as_ref().unwrap().source.port_id.unwrap();
        let port_type = project.element(port).unwrap().type_id.unwrap();
        let association = project.create_association(Some(project.root_id), vec![
            Project::association_end(port_type, "source", Multiplicity::new(0, None).unwrap(), true, AggregationKind::None),
            Project::association_end(port_type, "target", Multiplicity::new(0, None).unwrap(), true, AggregationKind::None),
        ]).unwrap();
        let mut edit = edit();
        edit.target_presentation_id = "internal".into();
        edit.typing = Some(ConnectorTypingEdit { association_type_id: Some(association.to_string()), source_multiplicity: "1".into(), target_multiplicity: "1..4".into() });
        let (candidate, staged) = stage_specification(&project, &diagrams, &diagrams[0].id, id, &edit).unwrap();
        for (key, relationship) in &project.relationships {
            if relationship.item_flow.is_some() {
                assert_eq!(relationship.item_flow, candidate.relationship(*key).unwrap().item_flow);
            }
        }
        edit.typing = None;
        let (candidate, _) = stage_specification(&candidate, &staged, &diagrams[0].id, id, &edit).unwrap();
        assert_eq!(candidate.relationship(id).unwrap().connector.as_ref().unwrap().association_type_id, Some(association));
    }

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
            typing: None,
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
