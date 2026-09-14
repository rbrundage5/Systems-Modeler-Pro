//! Desktop exposure of the model-core routing service. Diagram geometry and
//! routing policy remain Rust-owned and shared with the collaboration server.
pub use systems_modeler_core::routing::{
    DiagramRouteEdge, ROUTE_CLEARANCE, RouteRect, RouteRequest, RoutedDiagramEdge, label_rect,
    orthogonal_route, route_avoids_reserved, route_diagram_with_bounds, route_is_clear,
    route_label_anchor, route_label_anchor_avoiding,
};

#[tauri::command]
pub fn route_diagram_geometry(
    edges: Vec<DiagramRouteEdge>,
    obstacles: Vec<RouteRect>,
) -> Result<Vec<RoutedDiagramEdge>, String> {
    systems_modeler_core::routing::route_diagram_geometry(edges, obstacles)
}
