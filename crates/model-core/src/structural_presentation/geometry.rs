//! Native structural geometry adapter and atomic BDD commands.
use super::{BddDiagram, DiagramEdge, DiagramNode};
use crate::routing;
use serde::{Deserialize, Serialize};

pub fn node_route_rect(node: &DiagramNode) -> routing::RouteRect {
    routing::RouteRect {
        x: node.x,
        y: node.y,
        width: node.width,
        height: node.height,
    }
}

pub fn diagram_node_route_rect(diagram: &BddDiagram, node: &DiagramNode) -> routing::RouteRect {
    let mut rect = node_route_rect(node);
    if diagram.family == "package" {
        // UML Package notation has a tab above the persisted body rectangle.
        // Reserve it in the authoritative routing geometry so edges and labels
        // cannot pass behind the visible tab header.
        const PACKAGE_TAB_HEIGHT: f64 = 20.0;
        rect.y -= PACKAGE_TAB_HEIGHT;
        rect.height += PACKAGE_TAB_HEIGHT;
    }
    rect
}

pub fn routed_bdd_edges(
    diagram: &BddDiagram,
    bounds: Option<routing::RouteRect>,
) -> Result<Vec<DiagramEdge>, String> {
    let obstacles: Vec<_> = diagram
        .nodes
        .iter()
        .map(|node| diagram_node_route_rect(diagram, node))
        .collect();
    let route_edges = diagram
        .edges
        .iter()
        .map(|edge| {
            let source = diagram
                .nodes
                .iter()
                .find(|node| node.id == edge.source_node_id)
                .ok_or("diagram edge source presentation not found")?;
            let target = diagram
                .nodes
                .iter()
                .find(|node| node.id == edge.target_node_id)
                .ok_or("diagram edge target presentation not found")?;
            Ok(routing::DiagramRouteEdge {
                id: edge.id.clone(),
                source_id: edge.source_node_id.clone(),
                target_id: edge.target_node_id.clone(),
                source: diagram_node_route_rect(diagram, source),
                target: diagram_node_route_rect(diagram, target),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let routed = routing::route_diagram_with_bounds(&route_edges, &obstacles, bounds)?;
    let mut edges = diagram.edges.clone();
    for route in routed {
        let edge = edges
            .iter_mut()
            .find(|edge| edge.id == route.id)
            .ok_or("routed diagram edge presentation not found")?;
        edge.points = route.points;
        edge.label_anchor = Some(route.label_anchor);
    }
    Ok(edges)
}

/// Chosen by the trusted caller, never a permission supplied by the remote editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BddRoutingScope {
    IncidentEdges,
    AllEdges,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum BddGeometryCommand {
    UpdateNode {
        presentation_id: String,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    Route,
}

/// Apply to a private candidate first: a failed route cannot leave a moved node
/// beside old edges. Semantic validation/permission/revision/history belong to
/// the enclosing workspace or shared transaction, as before this extraction.
pub fn apply_bdd_geometry(
    diagram: &mut BddDiagram,
    command: &BddGeometryCommand,
    scope: BddRoutingScope,
    bounds: Option<routing::RouteRect>,
) -> Result<(), String> {
    if diagram.family != "bdd" {
        return Err("BDD geometry command requires a BDD".into());
    }
    let mut candidate = diagram.clone();
    let moved_id = match command {
        BddGeometryCommand::UpdateNode {
            presentation_id,
            x,
            y,
            width,
            height,
        } => {
            if !x.is_finite() || !y.is_finite() || !width.is_finite() || !height.is_finite() {
                return Err("presentation geometry must be finite".into());
            }
            if *width < 48.0 || *height < 32.0 {
                return Err("presentation size must be at least 48 x 32".into());
            }
            let node = candidate
                .nodes
                .iter_mut()
                .find(|node| node.id == *presentation_id)
                .ok_or("BDD presentation not found")?;
            node.x = *x;
            node.y = *y;
            node.width = *width;
            node.height = *height;
            Some(presentation_id)
        }
        BddGeometryCommand::Route => None,
    };
    let mut routing_candidate = candidate.clone();
    if let Some(id) = moved_id
        && scope == BddRoutingScope::IncidentEdges
    {
        routing_candidate
            .edges
            .retain(|edge| edge.source_node_id == *id || edge.target_node_id == *id);
    }
    for routed in routed_bdd_edges(&routing_candidate, bounds)? {
        let edge = candidate
            .edges
            .iter_mut()
            .find(|edge| edge.id == routed.id)
            .ok_or("routed diagram edge presentation not found")?;
        *edge = routed;
    }
    *diagram = candidate;
    Ok(())
}
