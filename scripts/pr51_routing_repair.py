from pathlib import Path

path = Path("apps/desktop/src-tauri/src/workspace/routing.rs")
text = path.read_text(encoding="utf-8")
old = '''pub fn route_diagram_with_bounds(
    edges: &[DiagramRouteEdge],
    obstacles: &[RouteRect],
    bounds: Option<RouteRect>,
) -> Result<Vec<RoutedDiagramEdge>, String> {
    let mut reserved_routes = Vec::new();
    let mut reserved_labels = Vec::new();
    let mut routed = Vec::new();
    for (index, edge) in edges.iter().enumerate() {
        let same_source_count = edges[..index]
            .iter()
            .filter(|candidate| candidate.source_id == edge.source_id)
            .count();
        let edge_obstacles: Vec<_> = obstacles
            .iter()
            .copied()
            .filter(|obstacle| *obstacle != edge.source && *obstacle != edge.target)
            .chain(reserved_labels.iter().copied())
            .collect();
        let label_obstacles: Vec<_> = obstacles
            .iter()
            .copied()
            .chain(reserved_labels.iter().copied())
            .collect();
        let points = orthogonal_route(RouteRequest {
            source: edge.source,
            target: edge.target,
            obstacles: &edge_obstacles,
            lane_index: same_source_count,
            reserved_routes: &reserved_routes,
            allow_shared_departure: same_source_count > 0,
            bounds,
        })?;
        let label_anchor =
            route_label_anchor_avoiding(&points, &label_obstacles, &reserved_routes, bounds)?;
        reserved_routes.push(points.clone());
        reserved_labels.push(label_rect(label_anchor));
        routed.push(RoutedDiagramEdge {
            id: edge.id.clone(),
            label_anchor,
            points,
        });
    }
    Ok(routed)
}
'''
new = '''pub fn route_diagram_with_bounds(
    edges: &[DiagramRouteEdge],
    obstacles: &[RouteRect],
    bounds: Option<RouteRect>,
) -> Result<Vec<RoutedDiagramEdge>, String> {
    // Route relationship geometry before placing labels. A label chosen for an
    // earlier edge must never trap a later edge at its semantic endpoint and
    // make Route/Clean Layout fail. Node/port geometry remains a hard obstacle,
    // and reserved relationship corridors remain preferred-routing constraints.
    let mut reserved_routes = Vec::new();
    let mut routed_geometry = Vec::new();
    for (index, edge) in edges.iter().enumerate() {
        let same_source_count = edges[..index]
            .iter()
            .filter(|candidate| candidate.source_id == edge.source_id)
            .count();
        let edge_obstacles: Vec<_> = obstacles
            .iter()
            .copied()
            .filter(|obstacle| *obstacle != edge.source && *obstacle != edge.target)
            .collect();
        let points = orthogonal_route(RouteRequest {
            source: edge.source,
            target: edge.target,
            obstacles: &edge_obstacles,
            lane_index: same_source_count,
            reserved_routes: &reserved_routes,
            allow_shared_departure: same_source_count > 0,
            bounds,
        })?;
        reserved_routes.push(points.clone());
        routed_geometry.push((edge.id.clone(), points));
    }

    // Labels are presentation metadata, not routing barriers. Place them only
    // after every relationship has an obstacle-clear route, while still asking
    // the shared label service to avoid nodes, other labels, and all routes.
    let all_routes: Vec<_> = routed_geometry
        .iter()
        .map(|(_, points)| points.clone())
        .collect();
    let mut reserved_labels = Vec::new();
    let mut routed = Vec::new();
    for (id, points) in routed_geometry {
        let label_obstacles: Vec<_> = obstacles
            .iter()
            .copied()
            .chain(reserved_labels.iter().copied())
            .collect();
        let label_anchor =
            route_label_anchor_avoiding(&points, &label_obstacles, &all_routes, bounds)?;
        reserved_labels.push(label_rect(label_anchor));
        routed.push(RoutedDiagramEdge {
            id,
            label_anchor,
            points,
        });
    }
    Ok(routed)
}
'''
if old in text:
    path.write_text(text.replace(old, new, 1), encoding="utf-8")
elif new not in text:
    raise SystemExit("shared batch-routing function was not found")
