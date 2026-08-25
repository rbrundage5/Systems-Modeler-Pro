use super::activity_workspace::ActivityWorkspaceState;
use super::history::{self, HistoryState};
use super::{
    WorkspaceState, behavior_workspace, ibd, route_relationship, use_cases,
    validate_loaded_diagrams,
};

fn validate_geometry(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    min_width: f64,
    min_height: f64,
) -> Result<(), String> {
    if !x.is_finite() || !y.is_finite() || !width.is_finite() || !height.is_finite() {
        return Err("presentation geometry must be finite".into());
    }
    if width < min_width || height < min_height {
        return Err(format!(
            "presentation size must be at least {min_width} x {min_height}"
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)] // Shared geometry primitive mirrors the named IPC fields.
fn apply_bdd_presentation_geometry(
    project: &systems_modeler_core::Project,
    diagrams: &mut [super::BddDiagram],
    diagram_id: &str,
    presentation_id: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    validate_geometry(x, y, width, height, 48.0, 32.0)?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("BDD not found")?;
    let node = diagram
        .nodes
        .iter_mut()
        .find(|node| node.id == presentation_id)
        .ok_or("BDD presentation not found")?;
    node.x = x;
    node.y = y;
    node.width = width;
    node.height = height;
    use_cases::fit_use_case_subject_boundary(diagram, project, false);

    diagram.edges = routed_bdd_edges(diagram, None)?;
    validate_loaded_diagrams(project, diagrams)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable named-field Tauri IPC boundary.
pub fn update_bdd_presentation_geometry(
    diagram_id: String,
    presentation_id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    state: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<(), String> {
    let project = state
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let mut diagrams = state
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    apply_bdd_presentation_geometry(
        &project,
        &mut diagrams,
        &diagram_id,
        &presentation_id,
        x,
        y,
        width,
        height,
    )?;

    history::checkpoint_states(&state, &activity, &history)?;
    *state.diagrams.lock().map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable named-field Tauri IPC boundary.
pub fn update_ibd_property_geometry(
    diagram_id: String,
    presentation_id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    state: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<(), String> {
    validate_geometry(x, y, width, height, 60.0, 40.0)?;
    let mut diagrams = state
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("IBD not found")?;
    let property = diagram
        .properties
        .iter_mut()
        .find(|property| property.id == presentation_id)
        .ok_or("IBD property presentation not found")?;
    property.x = x;
    property.y = y;
    property.width = width;
    property.height = height;

    let endpoints: Vec<_> = diagram
        .connectors
        .iter()
        .map(|edge| {
            (
                edge.id.clone(),
                edge.source_presentation_id.clone(),
                edge.target_presentation_id.clone(),
            )
        })
        .collect();
    let routes = endpoints
        .into_iter()
        .map(|(edge_id, source, target)| {
            Ok((edge_id, ibd::route_ibd_edge(diagram, &source, &target)?))
        })
        .collect::<Result<Vec<_>, String>>()?;
    for (edge_id, points) in routes {
        if let Some(edge) = diagram
            .connectors
            .iter_mut()
            .find(|edge| edge.id == edge_id)
        {
            edge.points = points;
        }
    }

    history::checkpoint_states(&state, &activity, &history)?;
    *state.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")? = diagrams;
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable named-field Tauri IPC boundary.
pub fn update_ibd_port_geometry(
    diagram_id: String,
    presentation_id: String,
    x: f64,
    y: f64,
    size: f64,
    state: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<(), String> {
    if !x.is_finite() || !y.is_finite() || !size.is_finite() || size < 10.0 {
        return Err("port presentation geometry is invalid".into());
    }
    let mut diagrams = state
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("IBD not found")?;
    let mut found = false;
    for property in &mut diagram.properties {
        if let Some(port) = property
            .ports
            .iter_mut()
            .find(|port| port.id == presentation_id)
        {
            let left = property.x;
            let right = property.x + property.width;
            let top = property.y;
            let bottom = property.y + property.height;
            let distances = [
                (x - left).abs(),
                (x - right).abs(),
                (y - top).abs(),
                (y - bottom).abs(),
            ];
            let edge = distances
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.total_cmp(b.1))
                .map(|(index, _)| index)
                .unwrap_or(0);
            match edge {
                0 => {
                    port.x = left;
                    port.y = y.clamp(top, bottom);
                }
                1 => {
                    port.x = right;
                    port.y = y.clamp(top, bottom);
                }
                2 => {
                    port.x = x.clamp(left, right);
                    port.y = top;
                }
                _ => {
                    port.x = x.clamp(left, right);
                    port.y = bottom;
                }
            }
            port.size = size;
            found = true;
            break;
        }
    }
    if !found
        && let Some(port) = diagram
            .boundary_ports
            .iter_mut()
            .find(|port| port.id == presentation_id)
    {
        let left = 0.0;
        let right = 1800.0;
        let top = 42.0;
        let bottom = 1100.0;
        let distances = [
            (x - left).abs(),
            (x - right).abs(),
            (y - top).abs(),
            (y - bottom).abs(),
        ];
        let edge = distances
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.total_cmp(b.1))
            .map(|(index, _)| index)
            .unwrap_or(0);
        match edge {
            0 => {
                port.x = left;
                port.y = y.clamp(top, bottom);
            }
            1 => {
                port.x = right;
                port.y = y.clamp(top, bottom);
            }
            2 => {
                port.x = x.clamp(left, right);
                port.y = top;
            }
            _ => {
                port.x = x.clamp(left, right);
                port.y = bottom;
            }
        }
        port.size = size;
        found = true;
    }
    if !found {
        return Err("IBD port presentation not found".into());
    }

    let endpoints: Vec<_> = diagram
        .connectors
        .iter()
        .map(|edge| {
            (
                edge.id.clone(),
                edge.source_presentation_id.clone(),
                edge.target_presentation_id.clone(),
            )
        })
        .collect();
    let routes = endpoints
        .into_iter()
        .map(|(edge_id, source, target)| {
            Ok((edge_id, ibd::route_ibd_edge(diagram, &source, &target)?))
        })
        .collect::<Result<Vec<_>, String>>()?;
    for (edge_id, points) in routes {
        if let Some(edge) = diagram
            .connectors
            .iter_mut()
            .find(|edge| edge.id == edge_id)
        {
            edge.points = points;
        }
    }

    history::checkpoint_states(&state, &activity, &history)?;
    *state.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")? = diagrams;
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable named-field Tauri IPC boundary.
pub fn update_state_presentation_geometry(
    diagram_id: String,
    state_vertex_id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    state: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<(), String> {
    validate_geometry(x, y, width, height, 20.0, 20.0)?;
    let mut diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("behavior diagram not found")?;
    let presentation = diagram
        .state_nodes
        .iter_mut()
        .find(|node| node.vertex_id == state_vertex_id)
        .ok_or("State presentation not found")?;
    presentation.x = x;
    presentation.y = y;
    presentation.width = width;
    presentation.height = height;

    let repository = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    behavior_workspace::reroute_behavior_presentation(diagram, &repository, None)?;
    drop(repository);

    history::checkpoint_states(&state, &activity, &history)?;
    *state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")? = diagrams;
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable named-field Tauri IPC boundary.
pub fn update_activity_presentation_geometry(
    diagram_id: String,
    presentation_id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    state: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<(), String> {
    validate_geometry(x, y, width, height, 20.0, 20.0)?;
    let mut diagrams = state
        .diagrams
        .lock()
        .map_err(|_| "activity diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("Activity diagram not found")?;
    let node = diagram
        .nodes
        .iter_mut()
        .find(|node| node.id == presentation_id)
        .ok_or("Activity presentation not found")?;
    node.x = x;
    node.y = y;
    node.width = width;
    node.height = height;

    let obstacles: Vec<_> = diagram
        .nodes
        .iter()
        .map(|node| super::routing::RouteRect {
            x: node.x,
            y: node.y,
            width: node.width,
            height: node.height,
        })
        .collect();
    for edge in &mut diagram.edges {
        let source = diagram
            .nodes
            .iter()
            .find(|node| node.id == edge.source_node_id)
            .ok_or("Activity edge source presentation not found")?;
        let target = diagram
            .nodes
            .iter()
            .find(|node| node.id == edge.target_node_id)
            .ok_or("Activity edge target presentation not found")?;
        let edge_obstacles: Vec<_> = obstacles
            .iter()
            .copied()
            .filter(|rect| {
                !((rect.x == source.x
                    && rect.y == source.y
                    && rect.width == source.width
                    && rect.height == source.height)
                    || (rect.x == target.x
                        && rect.y == target.y
                        && rect.width == target.width
                        && rect.height == target.height))
            })
            .collect();
        edge.points = super::routing::orthogonal_route(super::routing::RouteRequest {
            source: super::routing::RouteRect {
                x: source.x,
                y: source.y,
                width: source.width,
                height: source.height,
            },
            target: super::routing::RouteRect {
                x: target.x,
                y: target.y,
                width: target.width,
                height: target.height,
            },
            obstacles: &edge_obstacles,
            lane_index: 0,
            reserved_routes: &[],
            allow_shared_departure: false,
            bounds: None,
        })?;
    }

    history::checkpoint_states(&workspace, &state, &history)?;
    *state
        .diagrams
        .lock()
        .map_err(|_| "activity diagram lock poisoned")? = diagrams;
    Ok(())
}

#[cfg(test)]
mod shared_resize_tests {
    use super::*;
    use systems_modeler_core::{ElementKind, Project};
    use systems_modeler_persistence::ProjectDatabase;

    fn presentation(id: &str, element_id: String) -> super::super::DiagramNode {
        super::super::DiagramNode {
            id: id.into(),
            element_id,
            x: 120.0,
            y: 160.0,
            width: 180.0,
            height: 90.0,
            actor_notation: None,
            parameter_presentations: Vec::new(),
        }
    }

    #[test]
    fn structural_and_package_resize_snapshot_persistence_and_history_round_trip() {
        let mut project = Project::new("Shared Resize");
        let owner = project
            .create_element(ElementKind::Package, "Owner", project.root_id)
            .expect("owner Package");
        let block = project
            .create_element(ElementKind::Block, "Controller", owner)
            .expect("Block");
        let package = project
            .create_element(ElementKind::Package, "Subsystem", owner)
            .expect("presented Package");
        let bdd_id = uuid::Uuid::new_v4().to_string();
        let package_diagram_id = uuid::Uuid::new_v4().to_string();
        let bdd_node_id = uuid::Uuid::new_v4().to_string();
        let package_node_id = uuid::Uuid::new_v4().to_string();
        let original = vec![
            super::super::BddDiagram {
                id: bdd_id.clone(),
                name: "Structure".into(),
                owner_id: owner.to_string(),
                family: "bdd".into(),
                semantic_context_id: None,
                subject_boundary: None,
                nodes: vec![presentation(&bdd_node_id, block.to_string())],
                edges: Vec::new(),
            },
            super::super::BddDiagram {
                id: package_diagram_id.clone(),
                name: "Packages".into(),
                owner_id: owner.to_string(),
                family: "package".into(),
                semantic_context_id: None,
                subject_boundary: None,
                nodes: vec![presentation(&package_node_id, package.to_string())],
                edges: Vec::new(),
            },
        ];
        let workspace = WorkspaceState::default();
        *workspace.project.lock().expect("project lock") = Some(project.clone());
        *workspace.diagrams.lock().expect("diagram lock") = original;
        let activity = ActivityWorkspaceState::default();
        let history = HistoryState::default();
        history::checkpoint_states(&workspace, &activity, &history).expect("history checkpoint");

        let mut resized = workspace.diagrams.lock().expect("diagram lock").clone();
        apply_bdd_presentation_geometry(
            &project,
            &mut resized,
            &bdd_id,
            &bdd_node_id,
            140.0,
            180.0,
            320.0,
            170.0,
        )
        .expect("resize structural presentation");
        apply_bdd_presentation_geometry(
            &project,
            &mut resized,
            &package_diagram_id,
            &package_node_id,
            360.0,
            240.0,
            280.0,
            150.0,
        )
        .expect("resize Package presentation");
        *workspace.diagrams.lock().expect("diagram lock") = resized.clone();

        let snapshot = workspace.diagrams.lock().expect("diagram lock").clone();
        assert_eq!(
            (snapshot[0].nodes[0].width, snapshot[0].nodes[0].height),
            (320.0, 170.0)
        );
        assert_eq!(
            (snapshot[1].nodes[0].width, snapshot[1].nodes[0].height),
            (280.0, 150.0)
        );

        let path = std::env::temp_dir().join(format!(
            "systems-modeler-shared-resize-{}.smproj",
            uuid::Uuid::new_v4()
        ));
        {
            let mut database = ProjectDatabase::open(&path).expect("open database");
            database.save_project(&project).expect("save project");
            database
                .save_metadata(
                    project.id,
                    super::super::BDD_METADATA_KEY,
                    &serde_json::to_string(&resized).expect("serialize diagrams"),
                )
                .expect("save diagram geometry");
        }
        {
            let database = ProjectDatabase::open(&path).expect("reopen database");
            let payload = database
                .load_metadata(project.id, super::super::BDD_METADATA_KEY)
                .expect("load metadata")
                .expect("diagram metadata");
            let reopened: Vec<super::super::BddDiagram> =
                serde_json::from_str(&payload).expect("deserialize diagrams");
            assert_eq!(
                (reopened[0].nodes[0].width, reopened[0].nodes[0].height),
                (320.0, 170.0)
            );
            assert_eq!(
                (reopened[1].nodes[0].width, reopened[1].nodes[0].height),
                (280.0, 150.0)
            );
        }
        let _ = std::fs::remove_file(path);

        assert!(history::undo_states(&workspace, &activity, &history).expect("undo resize"));
        let undone = workspace.diagrams.lock().expect("diagram lock").clone();
        assert_eq!(
            (undone[0].nodes[0].width, undone[0].nodes[0].height),
            (180.0, 90.0)
        );
        assert_eq!(
            (undone[1].nodes[0].width, undone[1].nodes[0].height),
            (180.0, 90.0)
        );
        assert!(history::redo_states(&workspace, &activity, &history).expect("redo resize"));
        let redone = workspace.diagrams.lock().expect("diagram lock").clone();
        assert_eq!(
            (redone[0].nodes[0].width, redone[0].nodes[0].height),
            (320.0, 170.0)
        );
        assert_eq!(
            (redone[1].nodes[0].width, redone[1].nodes[0].height),
            (280.0, 150.0)
        );
    }
}
