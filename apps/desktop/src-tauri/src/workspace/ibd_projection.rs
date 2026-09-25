//! Projection of existing type-level connector definitions into contextual views.
use super::ibd::{self, IbdConnectorPresentation, IbdDiagram};
use super::*;
use systems_modeler_core::ConnectorEnd;

pub(super) fn project_end(prefix: &[ElementId], end: &ConnectorEnd) -> ConnectorEnd {
    if prefix.is_empty() {
        return end.clone();
    }
    let mut path = prefix.to_vec();
    path.extend(&end.property_path);
    ConnectorEnd {
        role_id: if end.port_id.is_some() {
            *path.last().expect("nonempty prefix")
        } else {
            end.role_id
        },
        property_path: path,
        port_id: end.port_id,
    }
}

fn presented_end(
    diagram: &IbdDiagram,
    end: &ConnectorEnd,
    ranges: &[Option<super::ibd_occurrences::OccurrenceRange>],
) -> Option<String> {
    diagram
        .boundary_ports
        .iter()
        .map(|port| port.id.clone())
        .chain(diagram.properties.iter().flat_map(|p| {
            std::iter::once(p.id.clone()).chain(p.ports.iter().map(|port| port.id.clone()))
        }))
        .find(|id| {
            super::ibd_occurrences::endpoint_matches_context(diagram, id, ranges)
                && ibd::ibd_end_for_presentation(diagram, id)
                    .is_ok_and(|(candidate, _)| candidate == *end)
        })
}

pub(super) fn show_existing_connectors(
    project: &Project,
    diagram: &mut IbdDiagram,
) -> Result<(), String> {
    let root = parse_element_id(&diagram.context_block_id)?;
    let mut contexts = vec![(Vec::new(), Vec::new())];
    contexts.extend(
        diagram
            .properties
            .iter()
            .filter(|p| !p.collapsed && super::ibd_structure::property_visible(diagram, p))
            .map(|p| (p.property_path.clone(), p.occurrence_path.clone())),
    );
    for (context_path, occurrence_path) in contexts {
        let prefix = context_path
            .iter()
            .map(|id| parse_element_id(id))
            .collect::<Result<Vec<_>, _>>()?;
        let classifier = project
            .resolve_structural_path(root, &prefix)
            .map_err(|error| error.to_string())?;
        for relationship in project.relationships.values() {
            let Some(connector) = &relationship.connector else {
                continue;
            };
            if connector.context_id != classifier
                || diagram.connectors.iter().any(|edge| {
                    edge.relationship_id == relationship.id.to_string()
                        && edge.context_path == context_path
                        && super::ibd_occurrences::ranges_equal(
                            &edge.context_occurrence_path,
                            &occurrence_path,
                            context_path.len(),
                        )
                })
            {
                continue;
            }
            let source = project_end(&prefix, &connector.source);
            let target = project_end(&prefix, &connector.target);
            let (Some(source_id), Some(target_id)) = (
                presented_end(diagram, &source, &occurrence_path),
                presented_end(diagram, &target, &occurrence_path),
            ) else {
                continue;
            };
            let points = ibd::route_ibd_edge(diagram, &source_id, &target_id)?;
            diagram.connectors.push(IbdConnectorPresentation {
                context_occurrence_path: occurrence_path.clone(),
                context_path: context_path.clone(),
                id: uuid::Uuid::new_v4().to_string(),
                relationship_id: relationship.id.to_string(),
                source_presentation_id: source_id,
                target_presentation_id: target_id,
                label_anchor: Some(routing::route_label_anchor(&points)),
                points,
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use systems_modeler_core::{Connector, ConnectorKind};

    #[test]
    fn repeated_type_usages_project_delegation_without_creating_relationships() {
        let (mut project, mut diagram) = super::super::ibd_geometry::tests::fixture();
        let context = parse_element_id(&diagram.context_block_id).unwrap();
        let internal = parse_element_id(&diagram.properties[0].element_id).unwrap();
        let component = project.element(internal).unwrap().type_id.unwrap();
        let port = parse_element_id(&diagram.properties[0].ports[0].element_id).unwrap();
        let interface = project.element(port).unwrap().type_id.unwrap();
        let child = project
            .create_typed_feature(
                ElementKind::PartProperty,
                "child",
                component,
                component,
                Multiplicity::new(0, Some(1)).unwrap(),
            )
            .unwrap();
        let connector_id = project
            .create_connector(Connector {
                association_type_id: None,
                end_multiplicities: Default::default(),
                context_id: component,
                kind: ConnectorKind::Delegation,
                source: ConnectorEnd::boundary(port),
                target: ConnectorEnd::nested_port(vec![child], port),
            })
            .unwrap();
        assert_eq!(project.element(port).unwrap().type_id, Some(interface));
        project
            .create_typed_feature(
                ElementKind::PartProperty,
                "second",
                context,
                component,
                Multiplicity::ONE,
            )
            .unwrap();
        ibd::populate_ibd_diagram_from_context(&project, &mut diagram).unwrap();
        let original = serde_json::to_value(&project).unwrap();
        let ids: Vec<_> = diagram.properties.iter().map(|p| p.id.clone()).collect();
        for id in &ids {
            super::super::ibd_structure::expand_property(&project, &mut diagram, id, true).unwrap();
        }
        let occurrences: Vec<_> = diagram
            .connectors
            .iter()
            .filter(|edge| edge.relationship_id == connector_id.to_string())
            .collect();
        assert_eq!(occurrences.len(), 2);
        assert_ne!(occurrences[0].context_path, occurrences[1].context_path);
        ibd::validate_ibd_diagrams(&project, &[diagram.clone()]).unwrap();
        let snapshot = serde_json::to_value(&diagram).unwrap();
        show_existing_connectors(&project, &mut diagram).unwrap();
        assert_eq!(serde_json::to_value(&diagram).unwrap(), snapshot);
        assert_eq!(serde_json::to_value(&project).unwrap(), original);
        let reopened: IbdDiagram = serde_json::from_value(snapshot).unwrap();
        ibd::validate_ibd_diagrams(&project, &[reopened]).unwrap();
    }
}
