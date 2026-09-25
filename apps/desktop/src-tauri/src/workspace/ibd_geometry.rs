//! Authored IBD boundaries and the read-only geometry used during a port gesture.
use super::DiagramPoint;
use super::ibd::{self, IbdDiagram, IbdPortPresentation, IbdPropertyPresentation};
use super::routing::RouteRect;
use super::shared_workspace::DiagramFramePreference;

pub(super) fn default_context_frame() -> DiagramFramePreference {
    DiagramFramePreference {
        x: 54.0,
        y: 70.0,
        width: 1018.0,
        height: 662.0,
        manually_sized: true,
    }
}

fn frame_rect(frame: &DiagramFramePreference) -> RouteRect {
    RouteRect {
        x: frame.x,
        y: frame.y,
        width: frame.width,
        height: frame.height,
    }
}

pub(super) fn property_rect(property: &IbdPropertyPresentation) -> RouteRect {
    RouteRect {
        x: property.x,
        y: property.y,
        width: property.width,
        height: property.height,
    }
}

fn validate_port(rect: RouteRect, x: f64, y: f64, size: f64) -> Result<(), String> {
    if ![rect.x, rect.y, rect.width, rect.height, x, y, size]
        .iter()
        .all(|value| value.is_finite() && value.abs() <= 100_000.0)
        || size < 10.0
        || size > rect.width.min(rect.height)
    {
        return Err("port geometry must be finite and fit on its owning boundary".into());
    }
    Ok(())
}

fn side_point(rect: RouteRect, side: usize, x: f64, y: f64, size: f64) -> DiagramPoint {
    let margin = size / 2.0;
    let along_x = x.clamp(rect.x + margin, rect.x + rect.width - margin);
    let along_y = y.clamp(rect.y + margin, rect.y + rect.height - margin);
    match side {
        0 => DiagramPoint {
            x: rect.x,
            y: along_y,
        },
        1 => DiagramPoint {
            x: rect.x + rect.width,
            y: along_y,
        },
        2 => DiagramPoint {
            x: along_x,
            y: rect.y,
        },
        _ => DiagramPoint {
            x: along_x,
            y: rect.y + rect.height,
        },
    }
}

fn nearest_side(rect: RouteRect, x: f64, y: f64, size: f64) -> usize {
    (0..4)
        .min_by(|a, b| {
            let distance = |side| {
                let p = side_point(rect, side, x, y, size);
                (p.x - x).hypot(p.y - y)
            };
            distance(*a).total_cmp(&distance(*b))
        })
        .unwrap_or(0)
}

fn project_port(
    port: &IbdPortPresentation,
    rect: RouteRect,
    x: f64,
    y: f64,
    size: f64,
) -> Result<IbdPortPresentation, String> {
    validate_port(rect, x, y, size)?;
    let mut side = nearest_side(rect, x, y, size);
    // A small tie band keeps a corner gesture stable. Preview and commit use
    // this same Rust function, including the same authored starting side.
    let previous = nearest_side(rect, port.x, port.y, size);
    let best = side_point(rect, side, x, y, size);
    let retained = side_point(rect, previous, x, y, size);
    if (retained.x - x).hypot(retained.y - y) <= (best.x - x).hypot(best.y - y) + 4.0 {
        side = previous;
    }
    let point = side_point(rect, side, x, y, size);
    let mut result = port.clone();
    result.x = point.x;
    result.y = point.y;
    result.size = size;
    Ok(result)
}

/// Offset a newly copied port along its owner's boundary without changing side.
pub(super) fn offset_copied_port(
    diagram: &mut IbdDiagram,
    presentation_id: &str,
    dx: f64,
    dy: f64,
) -> Result<(), String> {
    let (port, rect) = if let Some(property) = diagram
        .properties
        .iter()
        .find(|property| property.ports.iter().any(|port| port.id == presentation_id))
    {
        (
            property
                .ports
                .iter()
                .find(|port| port.id == presentation_id)
                .ok_or("copied port not found")?,
            property_rect(property),
        )
    } else {
        (
            diagram
                .boundary_ports
                .iter()
                .find(|port| port.id == presentation_id)
                .ok_or("copied port not found")?,
            frame_rect(
                diagram
                    .context_frame
                    .as_ref()
                    .ok_or("IBD frame is required")?,
            ),
        )
    };
    validate_port(rect, port.x + dx, port.y + dy, port.size)?;
    let side = nearest_side(rect, port.x, port.y, port.size);
    let point = side_point(rect, side, port.x + dx, port.y + dy, port.size);
    let port = diagram
        .properties
        .iter_mut()
        .flat_map(|property| &mut property.ports)
        .chain(&mut diagram.boundary_ports)
        .find(|port| port.id == presentation_id)
        .ok_or("copied port not found")?;
    port.x = point.x;
    port.y = point.y;
    Ok(())
}

pub(super) fn reanchor_port(
    port: &mut IbdPortPresentation,
    old: RouteRect,
    new: RouteRect,
) -> Result<(), String> {
    validate_port(old, port.x, port.y, port.size)?;
    validate_port(new, port.x, port.y, port.size)?;
    let side = nearest_side(old, port.x, port.y, port.size);
    let fraction_x = ((port.x - old.x) / old.width).clamp(0.0, 1.0);
    let fraction_y = ((port.y - old.y) / old.height).clamp(0.0, 1.0);
    let point = side_point(
        new,
        side,
        new.x + new.width * fraction_x,
        new.y + new.height * fraction_y,
        port.size,
    );
    port.x = point.x;
    port.y = point.y;
    Ok(())
}

pub(super) fn reroute_connected(
    diagram: &mut IbdDiagram,
    affected_ids: &[String],
) -> Result<(), String> {
    let mut routing_diagram = diagram.clone();
    routing_diagram.connectors.retain(|edge| {
        affected_ids
            .iter()
            .any(|id| id == &edge.source_presentation_id || id == &edge.target_presentation_id)
    });
    for routed in ibd::routed_ibd_connectors(&routing_diagram, None)? {
        if let Some(edge) = diagram
            .connectors
            .iter_mut()
            .find(|edge| edge.id == routed.id)
        {
            *edge = routed;
        }
    }
    Ok(())
}

pub(super) fn apply_context_frame(
    diagram: &mut IbdDiagram,
    mut frame: DiagramFramePreference,
) -> Result<(), String> {
    frame.validate()?;
    frame.manually_sized = true;
    if diagram.context_frame.as_ref() == Some(&frame) {
        return Ok(());
    }
    let new = frame_rect(&frame);
    for port in &mut diagram.boundary_ports {
        if let Some(old) = &diagram.context_frame {
            reanchor_port(port, frame_rect(old), new)?;
        } else {
            // Adopt the existing renderer's side interpretation exactly once.
            // Legacy projection preserved the coordinate along the edge.
            validate_port(new, port.x, port.y, port.size)?;
            let legacy = default_context_frame();
            let distances = [
                (port.x - legacy.x).abs(),
                (port.x - legacy.x - legacy.width).abs(),
                (port.y - legacy.y).abs(),
                (port.y - legacy.y - legacy.height).abs(),
            ];
            let side = (0..4)
                .min_by(|a, b| distances[*a].total_cmp(&distances[*b]))
                .unwrap_or(0);
            let point = side_point(new, side, port.x, port.y, port.size);
            port.x = point.x;
            port.y = point.y;
        }
    }
    diagram.context_frame = Some(frame);
    let ids = diagram
        .boundary_ports
        .iter()
        .map(|port| port.id.clone())
        .collect::<Vec<_>>();
    reroute_connected(diagram, &ids)
}

pub(super) fn preview_port(
    diagram: &IbdDiagram,
    presentation_id: &str,
    x: f64,
    y: f64,
    size: f64,
    visible_frame: Option<&DiagramFramePreference>,
) -> Result<IbdPortPresentation, String> {
    for property in &diagram.properties {
        if let Some(port) = property
            .ports
            .iter()
            .find(|port| port.id == presentation_id)
        {
            return project_port(port, property_rect(property), x, y, size);
        }
    }
    let port = diagram
        .boundary_ports
        .iter()
        .find(|port| port.id == presentation_id)
        .ok_or("IBD port presentation not found")?;
    let frame = diagram
        .context_frame
        .as_ref()
        .or(visible_frame)
        .ok_or("the visible IBD context frame is required for this legacy diagram")?;
    frame.validate()?;
    project_port(port, frame_rect(frame), x, y, size)
}

pub(super) fn apply_port(
    diagram: &mut IbdDiagram,
    presentation_id: &str,
    x: f64,
    y: f64,
    size: f64,
    visible_frame: Option<&DiagramFramePreference>,
) -> Result<(), String> {
    // Project before adopting a legacy frame so preview and commit use the same
    // authored starting side. All mutation occurs on the history helper's clone.
    let projected = preview_port(diagram, presentation_id, x, y, size, visible_frame)?;
    if diagram.context_frame.is_none()
        && diagram
            .boundary_ports
            .iter()
            .any(|port| port.id == presentation_id)
    {
        apply_context_frame(
            diagram,
            visible_frame.cloned().ok_or("IBD frame is required")?,
        )?;
    }
    let port = diagram
        .properties
        .iter_mut()
        .flat_map(|property| &mut property.ports)
        .chain(&mut diagram.boundary_ports)
        .find(|port| port.id == presentation_id)
        .ok_or("IBD port presentation not found")?;
    if port.x == projected.x && port.y == projected.y && port.size == projected.size {
        return Ok(());
    }
    *port = projected;
    reroute_connected(diagram, &[presentation_id.to_owned()])
}

#[derive(serde::Serialize)]
pub struct IbdPortGeometryPreview {
    // Preserve the existing x/y/size reply fields for older rendering adapters.
    #[serde(flatten)]
    pub port: IbdPortPresentation,
    pub connectors: Vec<ibd::IbdConnectorPresentation>,
}

pub(super) fn preview_connected_port(
    diagram: &IbdDiagram,
    presentation_id: &str,
    x: f64,
    y: f64,
    size: f64,
    visible_frame: Option<&DiagramFramePreference>,
) -> Result<IbdPortGeometryPreview, String> {
    let port = preview_port(diagram, presentation_id, x, y, size, visible_frame)?;
    // Keep the original obstacles, but route/copy only connectors attached to
    // this port. Unrelated stale connectors cannot invalidate a drag preview.
    let mut staged = IbdDiagram {
        id: diagram.id.clone(),
        name: diagram.name.clone(),
        context_block_id: diagram.context_block_id.clone(),
        owner_id: diagram.owner_id.clone(),
        context_frame: diagram.context_frame.clone(),
        properties: diagram.properties.clone(),
        boundary_ports: diagram.boundary_ports.clone(),
        connectors: diagram
            .connectors
            .iter()
            .filter(|edge| {
                edge.source_presentation_id == presentation_id
                    || edge.target_presentation_id == presentation_id
            })
            .cloned()
            .collect(),
    };
    apply_port(&mut staged, presentation_id, x, y, size, visible_frame)?;
    Ok(IbdPortGeometryPreview {
        port,
        connectors: staged.connectors,
    })
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::{
        WorkspaceState, activity_workspace::ActivityWorkspaceState, history, portable_interchange,
    };
    use super::*;
    use systems_modeler_core::{
        Connector, ConnectorEnd, ConnectorKind, ElementKind, Multiplicity, Project,
    };
    use systems_modeler_persistence::ProjectDatabase;

    pub(in crate::workspace) fn fixture() -> (Project, IbdDiagram) {
        let mut project = Project::new("Port boundaries");
        let system = project
            .create_element(ElementKind::Block, "System", project.root_id)
            .unwrap();
        let component = project
            .create_element(ElementKind::Block, "Unit", project.root_id)
            .unwrap();
        let interface = project
            .create_element(ElementKind::InterfaceBlock, "Contract", project.root_id)
            .unwrap();
        let part = project
            .create_typed_feature(
                ElementKind::PartProperty,
                "unit",
                system,
                component,
                Multiplicity::ONE,
            )
            .unwrap();
        let external = project
            .create_typed_feature(
                ElementKind::ProxyPort,
                "external",
                system,
                interface,
                Multiplicity::ONE,
            )
            .unwrap();
        let internal = project
            .create_typed_feature(
                ElementKind::ProxyPort,
                "internal",
                component,
                interface,
                Multiplicity::ONE,
            )
            .unwrap();
        let relationship = project
            .create_connector(Connector {
                association_type_id: None,
                end_multiplicities: Default::default(),
                context_id: system,
                kind: ConnectorKind::Delegation,
                source: ConnectorEnd::boundary(external),
                target: ConnectorEnd::nested_port(vec![part], internal),
            })
            .unwrap();
        let mut diagram = IbdDiagram {
            id: uuid::Uuid::new_v4().to_string(),
            name: "System internals".into(),
            context_block_id: system.to_string(),
            owner_id: project.root_id.to_string(),
            context_frame: Some(default_context_frame()),
            properties: vec![IbdPropertyPresentation {
                collapsed: false,
                id: "part".into(),
                element_id: part.to_string(),
                property_path: vec![part.to_string()],
                x: 200.0,
                y: 160.0,
                width: 180.0,
                height: 100.0,
                ports: vec![IbdPortPresentation {
                    id: "internal".into(),
                    element_id: internal.to_string(),
                    property_path: vec![part.to_string()],
                    x: 200.0,
                    y: 210.0,
                    size: 16.0,
                }],
            }],
            boundary_ports: vec![IbdPortPresentation {
                id: "external".into(),
                element_id: external.to_string(),
                property_path: Vec::new(),
                x: 54.0,
                y: 170.0,
                size: 16.0,
            }],
            connectors: vec![ibd::IbdConnectorPresentation {
                context_path: Vec::new(),
                id: "connector".into(),
                relationship_id: relationship.to_string(),
                source_presentation_id: "external".into(),
                target_presentation_id: "internal".into(),
                points: Vec::new(),
                label_anchor: None,
            }],
        };
        reroute_connected(&mut diagram, &["external".into()]).unwrap();
        ibd::validate_ibd_diagrams(&project, &[diagram.clone()]).unwrap();
        (project, diagram)
    }

    fn value(diagram: &IbdDiagram) -> serde_json::Value {
        serde_json::to_value(diagram).unwrap()
    }

    #[test]
    fn context_and_nested_ports_project_to_all_four_sides_without_mutation() {
        let (_, diagram) = fixture();
        let before = value(&diagram);
        for (x, y, expected_x, expected_y) in [
            (-100.0, 300.0, 54.0, 300.0),
            (1400.0, 300.0, 1072.0, 300.0),
            (600.0, -100.0, 600.0, 70.0),
            (600.0, 1000.0, 600.0, 732.0),
        ] {
            let preview = preview_port(&diagram, "external", x, y, 16.0, None).unwrap();
            assert_eq!((preview.x, preview.y), (expected_x, expected_y));
        }
        for (x, y, expected_x, expected_y) in [
            (100.0, 210.0, 200.0, 210.0),
            (600.0, 210.0, 380.0, 210.0),
            (290.0, 0.0, 290.0, 160.0),
            (290.0, 600.0, 290.0, 260.0),
        ] {
            let preview = preview_port(&diagram, "internal", x, y, 16.0, None).unwrap();
            assert_eq!((preview.x, preview.y), (expected_x, expected_y));
        }
        let corner = preview_port(&diagram, "external", -100.0, -100.0, 20.0, None).unwrap();
        assert!(corner.x >= 54.0 && corner.y >= 70.0);
        assert!(
            corner.x >= 64.0 || corner.y >= 80.0,
            "port must clear the corner"
        );
        assert_eq!(value(&diagram), before);
    }

    #[test]
    fn port_commit_matches_preview_and_reroutes_labels_without_changing_identity() {
        let (project, mut diagram) = fixture();
        let before = diagram.clone();
        let preview = preview_port(&diagram, "external", 1400.0, 400.0, 24.0, None).unwrap();
        apply_port(&mut diagram, "external", 1400.0, 400.0, 24.0, None).unwrap();
        assert_eq!(
            serde_json::to_value(&diagram.boundary_ports[0]).unwrap(),
            serde_json::to_value(preview).unwrap()
        );
        assert_eq!(
            diagram.boundary_ports[0].element_id,
            before.boundary_ports[0].element_id
        );
        assert_eq!(
            diagram.connectors[0].relationship_id,
            before.connectors[0].relationship_id
        );
        assert_ne!(diagram.connectors[0].points, before.connectors[0].points);
        assert_ne!(
            diagram.connectors[0].label_anchor,
            before.connectors[0].label_anchor
        );
        assert!(
            diagram.connectors[0]
                .points
                .windows(2)
                .all(|pair| pair[0].x == pair[1].x || pair[0].y == pair[1].y)
        );
        ibd::validate_ibd_diagrams(&project, &[diagram]).unwrap();
    }

    #[test]
    fn connected_port_preview_matches_committed_routes_and_never_mutates_source() {
        for port_id in ["external", "internal"] {
            let (_, diagram) = fixture();
            let before = value(&diagram);
            let preview =
                preview_connected_port(&diagram, port_id, 1400.0, 400.0, 24.0, None).unwrap();
            let mut committed = diagram.clone();
            apply_port(&mut committed, port_id, 1400.0, 400.0, 24.0, None).unwrap();
            assert_eq!(
                serde_json::to_value(&preview.connectors).unwrap(),
                serde_json::to_value(&committed.connectors).unwrap()
            );
            assert_eq!(value(&diagram), before);
            let wire = serde_json::to_value(preview).unwrap();
            assert!(wire.get("x").is_some() && wire.get("size").is_some());
            assert!(wire.get("connectors").unwrap().is_array());
        }
    }

    #[test]
    fn connected_preview_ignores_unrelated_stale_edges_and_rejects_invalid_geometry() {
        let (_, mut diagram) = fixture();
        let mut unrelated = diagram.connectors[0].clone();
        unrelated.id = "unrelated".into();
        unrelated.source_presentation_id = "missing-source".into();
        unrelated.target_presentation_id = "missing-target".into();
        diagram.connectors.push(unrelated);
        let before = value(&diagram);
        let preview =
            preview_connected_port(&diagram, "external", 1072.0, 400.0, 16.0, None).unwrap();
        assert_eq!(preview.connectors.len(), 1);
        assert_ne!(preview.connectors[0].id, "unrelated");
        assert!(preview_connected_port(&diagram, "external", f64::NAN, 170.0, 16.0, None).is_err());
        assert_eq!(value(&diagram), before);
    }

    #[test]
    fn frame_resize_preserves_side_and_fraction_and_legacy_files_adopt_once() {
        let (_, mut diagram) = fixture();
        let nested_before = serde_json::to_value(&diagram.properties).unwrap();
        let frame = DiagramFramePreference {
            x: 74.0,
            y: 90.0,
            width: 1200.0,
            height: 1324.0,
            manually_sized: true,
        };
        apply_context_frame(&mut diagram, frame.clone()).unwrap();
        assert_eq!(
            (diagram.boundary_ports[0].x, diagram.boundary_ports[0].y),
            (74.0, 290.0)
        );
        assert_eq!(
            serde_json::to_value(&diagram.properties).unwrap(),
            nested_before
        );
        let mut legacy_json = value(&diagram);
        legacy_json.as_object_mut().unwrap().remove("context_frame");
        let mut legacy: IbdDiagram = serde_json::from_value(legacy_json).unwrap();
        assert!(legacy.context_frame.is_none());
        legacy.boundary_ports[0].x = 55.0;
        apply_context_frame(&mut legacy, frame.clone()).unwrap();
        assert_eq!(
            (legacy.boundary_ports[0].x, legacy.boundary_ports[0].y),
            (74.0, 290.0)
        );
        let adopted = value(&legacy);
        apply_context_frame(&mut legacy, frame).unwrap();
        assert_eq!(value(&legacy), adopted);
    }

    #[test]
    fn frame_port_history_and_native_round_trip_are_atomic() {
        let (project, diagram) = fixture();
        let workspace = WorkspaceState::default();
        let activity = ActivityWorkspaceState::default();
        let history = history::HistoryState::default();
        let id = diagram.id.clone();
        let before = value(&diagram);
        *workspace.project.lock().unwrap() = Some(project.clone());
        *workspace.ibd_diagrams.lock().unwrap() = vec![diagram];
        history::edit_ibd_geometry(&workspace, &activity, &history, &id, |diagram| {
            apply_context_frame(
                diagram,
                DiagramFramePreference {
                    x: 74.0,
                    y: 90.0,
                    width: 1200.0,
                    height: 1324.0,
                    manually_sized: true,
                },
            )
        })
        .unwrap();
        assert_eq!(history::undo_len(&history), 1);
        let saved = workspace.ibd_diagrams.lock().unwrap().clone();
        let folder = tempfile::tempdir().unwrap();
        let path = folder.path().join("ports.smproj");
        {
            let mut database = ProjectDatabase::open(&path).unwrap();
            database.save_project(&project).unwrap();
            ibd::save_ibd_metadata(&mut database, &project, &saved).unwrap();
        }
        let database = ProjectDatabase::open(&path).unwrap();
        let reopened_project = database.load_project(project.id).unwrap();
        let reopened = ibd::load_ibd_metadata(&database, &reopened_project).unwrap();
        assert_eq!(value(&reopened[0]), value(&saved[0]));
        let portable = portable_interchange::export_from_states(&workspace, &activity).unwrap();
        let imported = WorkspaceState::default();
        portable_interchange::import_into_states(
            &portable,
            &imported,
            &ActivityWorkspaceState::default(),
        )
        .unwrap();
        assert_eq!(
            value(&imported.ibd_diagrams.lock().unwrap()[0]),
            value(&saved[0])
        );
        assert!(history::undo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(value(&workspace.ibd_diagrams.lock().unwrap()[0]), before);
        // A no-op and a rejected resize must both preserve the available redo.
        history::edit_ibd_geometry(&workspace, &activity, &history, &id, |diagram| {
            apply_port(diagram, "external", 54.0, 170.0, 16.0, None)
        })
        .unwrap();
        assert!(
            history::edit_ibd_geometry(&workspace, &activity, &history, &id, |diagram| apply_port(
                diagram,
                "external",
                f64::NAN,
                170.0,
                16.0,
                None
            ))
            .is_err()
        );
        assert_eq!(history::undo_len(&history), 0);
        assert!(history::redo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(
            value(&workspace.ibd_diagrams.lock().unwrap()[0]),
            value(&saved[0])
        );
    }

    #[test]
    fn invalid_geometry_missing_ports_and_routing_failure_leave_state_and_history_unchanged() {
        let (project, mut diagram) = fixture();
        assert!(preview_port(&diagram, "external", 54.0, 170.0, 1000.0, None).is_err());
        assert!(preview_port(&diagram, "missing", 54.0, 170.0, 16.0, None).is_err());
        assert!(preview_port(&diagram, "internal", f64::INFINITY, 170.0, 16.0, None).is_err());
        diagram.connectors[0].target_presentation_id = "missing".into();
        let before = value(&diagram);
        let id = diagram.id.clone();
        let workspace = WorkspaceState::default();
        *workspace.project.lock().unwrap() = Some(project);
        *workspace.ibd_diagrams.lock().unwrap() = vec![diagram];
        let activity = ActivityWorkspaceState::default();
        let history = history::HistoryState::default();
        assert!(
            history::edit_ibd_geometry(&workspace, &activity, &history, &id, |diagram| apply_port(
                diagram, "external", 1072.0, 400.0, 16.0, None
            ))
            .is_err()
        );
        assert_eq!(value(&workspace.ibd_diagrams.lock().unwrap()[0]), before);
        assert_eq!(history::undo_len(&history), 0);
    }

    #[test]
    fn unrelated_stale_routes_do_not_block_port_movement() {
        let (_, mut diagram) = fixture();
        let mut unrelated = diagram.connectors[0].clone();
        unrelated.id = "unrelated".into();
        unrelated.source_presentation_id = "unrelated-missing".into();
        unrelated.target_presentation_id = "another-missing".into();
        diagram.connectors.push(unrelated.clone());
        apply_port(&mut diagram, "external", 1072.0, 400.0, 16.0, None).unwrap();
        assert_eq!(
            serde_json::to_value(&diagram.connectors[1]).unwrap(),
            serde_json::to_value(unrelated).unwrap()
        );
    }
}
