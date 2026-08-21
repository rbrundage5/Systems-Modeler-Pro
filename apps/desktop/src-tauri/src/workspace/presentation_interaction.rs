use super::activity_workspace::ActivityWorkspaceState;
use super::history::{self, HistoryState};
use super::{WorkspaceState, ibd, route_relationship};

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

#[tauri::command]
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
    validate_geometry(x, y, width, height, 48.0, 32.0)?;
    let mut diagrams = state
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
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

    let routes: Vec<_> = diagram
        .edges
        .iter()
        .map(|edge| {
            let source = diagram
                .nodes
                .iter()
                .find(|node| node.id == edge.source_node_id)
                .cloned()
                .ok_or("BDD edge source presentation not found")?;
            let target = diagram
                .nodes
                .iter()
                .find(|node| node.id == edge.target_node_id)
                .cloned()
                .ok_or("BDD edge target presentation not found")?;
            Ok((
                edge.id.clone(),
                route_relationship(&source, &target, &diagram.nodes),
            ))
        })
        .collect::<Result<_, String>>()?;
    for (edge_id, points) in routes {
        if let Some(edge) = diagram.edges.iter_mut().find(|edge| edge.id == edge_id) {
            edge.points = points;
        }
    }

    history::checkpoint_states(&state, &activity, &history)?;
    *state.diagrams.lock().map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(())
}

#[tauri::command]
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

    history::checkpoint_states(&state, &activity, &history)?;
    *state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")? = diagrams;
    Ok(())
}

#[tauri::command]
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
        });
    }

    history::checkpoint_states(&workspace, &state, &history)?;
    *state
        .diagrams
        .lock()
        .map_err(|_| "activity diagram lock poisoned")? = diagrams;
    Ok(())
}
