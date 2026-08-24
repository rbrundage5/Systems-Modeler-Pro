use super::DiagramPoint;
use serde::{Deserialize, Serialize};

pub const ROUTE_CLEARANCE: f64 = 18.0;
pub const LANE_SPACING: f64 = 12.0;
const LABEL_WIDTH: f64 = 120.0;
const LABEL_HEIGHT: f64 = 24.0;
const LABEL_GAP: f64 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RouteRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl RouteRect {
    pub fn center(&self) -> DiagramPoint {
        DiagramPoint {
            x: self.x + self.width / 2.0,
            y: self.y + self.height / 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RouteRequest<'a> {
    pub source: RouteRect,
    pub target: RouteRect,
    pub obstacles: &'a [RouteRect],
    pub lane_index: usize,
    pub reserved_routes: &'a [Vec<DiagramPoint>],
    pub allow_shared_departure: bool,
    /// Optional diagram-frame interior in diagram coordinates. Routes may touch
    /// presented endpoints but may not escape this committed frame.
    pub bounds: Option<RouteRect>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagramRouteEdge {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub source: RouteRect,
    pub target: RouteRect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoutedDiagramEdge {
    pub id: String,
    pub points: Vec<DiagramPoint>,
    pub label_anchor: DiagramPoint,
}

pub fn route_label_anchor(points: &[DiagramPoint]) -> DiagramPoint {
    let segment = points
        .windows(2)
        .max_by(|left, right| {
            segment_length(left[0], left[1]).total_cmp(&segment_length(right[0], right[1]))
        })
        .unwrap_or(&[
            DiagramPoint { x: 0.0, y: 0.0 },
            DiagramPoint { x: 0.0, y: 0.0 },
        ]);
    let horizontal = (segment[0].y - segment[1].y).abs() < 0.001;
    DiagramPoint {
        x: (segment[0].x + segment[1].x) / 2.0
            + if horizontal {
                0.0
            } else {
                LABEL_HEIGHT / 2.0 + LABEL_GAP
            },
        y: (segment[0].y + segment[1].y) / 2.0
            - if horizontal {
                LABEL_HEIGHT / 2.0 + LABEL_GAP
            } else {
                0.0
            },
    }
}

pub fn label_rect(anchor: DiagramPoint) -> RouteRect {
    RouteRect {
        x: anchor.x - LABEL_WIDTH / 2.0,
        y: anchor.y - LABEL_HEIGHT,
        width: LABEL_WIDTH,
        height: LABEL_HEIGHT,
    }
}

pub fn route_label_anchor_avoiding(
    points: &[DiagramPoint],
    obstacles: &[RouteRect],
    reserved_routes: &[Vec<DiagramPoint>],
    bounds: Option<RouteRect>,
) -> Result<DiagramPoint, String> {
    let mut segments: Vec<_> = points
        .windows(2)
        .filter(|segment| segment_length(segment[0], segment[1]) > 0.001)
        .collect();
    segments.sort_by(|left, right| {
        segment_length(right[0], right[1]).total_cmp(&segment_length(left[0], left[1]))
    });
    for segment in segments {
        let horizontal = (segment[0].y - segment[1].y).abs() < 0.001;
        for ratio in [0.5, 0.25, 0.75, 0.125, 0.875] {
            for side in [-1.0, 1.0] {
                let anchor = DiagramPoint {
                    x: segment[0].x
                        + (segment[1].x - segment[0].x) * ratio
                        + if horizontal {
                            0.0
                        } else {
                            side * (LABEL_WIDTH / 2.0 + LABEL_GAP)
                        },
                    y: segment[0].y
                        + (segment[1].y - segment[0].y) * ratio
                        + if horizontal {
                            side * (LABEL_HEIGHT + LABEL_GAP)
                        } else {
                            0.0
                        },
                };
                let rect = label_rect(anchor);
                let inside_bounds = bounds.is_none_or(|frame| rect_inside(rect, frame));
                let clears_obstacles = obstacles
                    .iter()
                    .all(|obstacle| !rects_overlap(rect, *obstacle));
                let clears_routes = reserved_routes.iter().all(|route| {
                    route.windows(2).all(|reserved| {
                        !segment_intersects_rect_exact(reserved[0], reserved[1], rect)
                    })
                });
                if inside_bounds && clears_obstacles && clears_routes {
                    return Ok(anchor);
                }
            }
        }
    }
    Err("no legal label position is available without overlapping diagram content; existing geometry was preserved".into())
}

fn segment_length(start: DiagramPoint, end: DiagramPoint) -> f64 {
    (end.x - start.x).abs() + (end.y - start.y).abs()
}

/// Application-wide batch routing contract. Diagram families provide semantic
/// endpoint identity and geometry; this service owns corridor reservation.
pub fn route_diagram(
    edges: &[DiagramRouteEdge],
    obstacles: &[RouteRect],
) -> Result<Vec<RoutedDiagramEdge>, String> {
    route_diagram_with_bounds(edges, obstacles, None)
}

pub fn route_diagram_with_bounds(
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

#[tauri::command]
pub fn route_diagram_geometry(
    edges: Vec<DiagramRouteEdge>,
    obstacles: Vec<RouteRect>,
) -> Result<Vec<RoutedDiagramEdge>, String> {
    if edges.iter().any(|edge| {
        edge.id.trim().is_empty()
            || edge.source_id.trim().is_empty()
            || edge.target_id.trim().is_empty()
    }) {
        return Err("diagram routing requires stable edge and endpoint identifiers".into());
    }
    let invalid = obstacles
        .iter()
        .chain(edges.iter().flat_map(|edge| [&edge.source, &edge.target]))
        .any(|rect| {
            ![rect.x, rect.y, rect.width, rect.height]
                .iter()
                .all(|value| value.is_finite())
                || rect.width <= 0.0
                || rect.height <= 0.0
        });
    if invalid {
        return Err("diagram routing contains invalid geometry".into());
    }
    route_diagram(&edges, &obstacles)
}

/// Shared deterministic orthogonal router for BDD and IBD presentations.
/// It never intentionally returns a segment through an obstacle. If the direct
/// dogleg is blocked, perpendicular outer channels are searched in deterministic
/// order. Horizontal relationships detour above/below obstacles; vertical
/// relationships detour left/right.
pub fn orthogonal_route(request: RouteRequest<'_>) -> Result<Vec<DiagramPoint>, String> {
    if request.source == request.target {
        return self_transition_route(&request);
    }

    let source_center = request.source.center();
    let target_center = request.target.center();
    let dx = target_center.x - source_center.x;
    let dy = target_center.y - source_center.y;
    let horizontal = dx.abs() >= dy.abs();
    let lane_offset = request.lane_index as f64 * LANE_SPACING;

    let (start, end) = attached_endpoints(request.source, request.target, horizontal);
    let mut candidates = Vec::new();

    if horizontal {
        if request.lane_index == 0 {
            let mid_x = (start.x + end.x) / 2.0;
            candidates.push(compact(vec![
                start,
                DiagramPoint {
                    x: mid_x,
                    y: start.y,
                },
                DiagramPoint { x: mid_x, y: end.y },
                end,
            ]));
        } else {
            let lane_y = start.y.max(end.y) + lane_offset;
            candidates.push(compact(vec![
                start,
                DiagramPoint {
                    x: start.x,
                    y: lane_y,
                },
                DiagramPoint {
                    x: end.x,
                    y: lane_y,
                },
                end,
            ]));
        }

        // A left/right relationship blocked between its endpoints must escape
        // perpendicular to the relationship, then traverse above or below all
        // blocking geometry. Searching another x channel cannot help because
        // the final horizontal segment would still cross the obstacle.
        let min_y = request
            .obstacles
            .iter()
            .chain([&request.source, &request.target])
            .map(|rect| rect.y)
            .fold(f64::INFINITY, f64::min);
        let max_y = request
            .obstacles
            .iter()
            .chain([&request.source, &request.target])
            .map(|rect| rect.y + rect.height)
            .fold(f64::NEG_INFINITY, f64::max);
        for ring in 1..=routing_ring_limit(&request) {
            let clearance = ROUTE_CLEARANCE + lane_offset + ring as f64 * LANE_SPACING;
            for y in [min_y - clearance, max_y + clearance] {
                candidates.push(compact(vec![
                    start,
                    DiagramPoint { x: start.x, y },
                    DiagramPoint { x: end.x, y },
                    end,
                ]));
            }
        }
    } else {
        if request.lane_index == 0 {
            let mid_y = (start.y + end.y) / 2.0;
            candidates.push(compact(vec![
                start,
                DiagramPoint {
                    x: start.x,
                    y: mid_y,
                },
                DiagramPoint { x: end.x, y: mid_y },
                end,
            ]));
        } else {
            let lane_x = start.x.max(end.x) + lane_offset;
            candidates.push(compact(vec![
                start,
                DiagramPoint {
                    x: lane_x,
                    y: start.y,
                },
                DiagramPoint {
                    x: lane_x,
                    y: end.y,
                },
                end,
            ]));
        }

        // A top/bottom relationship detours left or right of blocking geometry.
        let min_x = request
            .obstacles
            .iter()
            .chain([&request.source, &request.target])
            .map(|rect| rect.x)
            .fold(f64::INFINITY, f64::min);
        let max_x = request
            .obstacles
            .iter()
            .chain([&request.source, &request.target])
            .map(|rect| rect.x + rect.width)
            .fold(f64::NEG_INFINITY, f64::max);
        for ring in 1..=routing_ring_limit(&request) {
            let clearance = ROUTE_CLEARANCE + lane_offset + ring as f64 * LANE_SPACING;
            for x in [min_x - clearance, max_x + clearance] {
                candidates.push(compact(vec![
                    start,
                    DiagramPoint { x, y: start.y },
                    DiagramPoint { x, y: end.y },
                    end,
                ]));
            }
        }
    }

    add_escape_candidates(&mut candidates, start, end, &request, lane_offset);
    if request.lane_index > 0 {
        candidates.retain(|candidate| {
            if horizontal {
                let minimum = start.y.min(end.y) - lane_offset;
                let maximum = start.y.max(end.y) + lane_offset;
                candidate
                    .iter()
                    .any(|point| point.y <= minimum || point.y >= maximum)
            } else {
                let minimum = start.x.min(end.x) - lane_offset;
                let maximum = start.x.max(end.x) + lane_offset;
                candidate
                    .iter()
                    .any(|point| point.x <= minimum || point.x >= maximum)
            }
        });
    }
    candidates.sort_by(|left, right| route_cost(left).total_cmp(&route_cost(right)));
    candidates
        .into_iter()
        .find(|candidate| route_is_valid(candidate, &request))
        .ok_or_else(|| route_failure(&request))
}

fn self_transition_route(request: &RouteRequest<'_>) -> Result<Vec<DiagramPoint>, String> {
    let rect = request.source;
    let center = rect.center();
    let clearance = ROUTE_CLEARANCE + (request.lane_index + 1) as f64 * LANE_SPACING;
    let top = DiagramPoint {
        x: center.x,
        y: rect.y,
    };
    let right = DiagramPoint {
        x: rect.x + rect.width,
        y: center.y,
    };
    let bottom = DiagramPoint {
        x: center.x,
        y: rect.y + rect.height,
    };
    let left = DiagramPoint {
        x: rect.x,
        y: center.y,
    };
    let outer_top = rect.y - clearance;
    let outer_right = rect.x + rect.width + clearance;
    let outer_bottom = rect.y + rect.height + clearance;
    let outer_left = rect.x - clearance;
    let mut candidates = vec![
        vec![
            top,
            DiagramPoint {
                x: top.x,
                y: outer_top,
            },
            DiagramPoint {
                x: outer_right,
                y: outer_top,
            },
            DiagramPoint {
                x: outer_right,
                y: right.y,
            },
            right,
        ],
        vec![
            right,
            DiagramPoint {
                x: outer_right,
                y: right.y,
            },
            DiagramPoint {
                x: outer_right,
                y: outer_bottom,
            },
            DiagramPoint {
                x: bottom.x,
                y: outer_bottom,
            },
            bottom,
        ],
        vec![
            bottom,
            DiagramPoint {
                x: bottom.x,
                y: outer_bottom,
            },
            DiagramPoint {
                x: outer_left,
                y: outer_bottom,
            },
            DiagramPoint {
                x: outer_left,
                y: left.y,
            },
            left,
        ],
        vec![
            left,
            DiagramPoint {
                x: outer_left,
                y: left.y,
            },
            DiagramPoint {
                x: outer_left,
                y: outer_top,
            },
            DiagramPoint {
                x: top.x,
                y: outer_top,
            },
            top,
        ],
    ];
    candidates.sort_by(|left, right| route_cost(left).total_cmp(&route_cost(right)));
    candidates
        .into_iter()
        .find(|candidate| route_is_valid(candidate, request))
        .ok_or_else(|| route_failure(request))
}

fn route_failure(request: &RouteRequest<'_>) -> String {
    let scope = if request.bounds.is_some() {
        " inside the diagram frame"
    } else {
        ""
    };
    format!(
        "no validated obstacle-clear route is available{scope}; existing geometry was preserved"
    )
}

fn routing_ring_limit(request: &RouteRequest<'_>) -> usize {
    (12 + request.obstacles.len() + request.reserved_routes.len() * 2).min(96)
}

fn add_escape_candidates(
    candidates: &mut Vec<Vec<DiagramPoint>>,
    start: DiagramPoint,
    end: DiagramPoint,
    request: &RouteRequest<'_>,
    lane_offset: f64,
) {
    let mut x_tracks = vec![start.x, end.x, (start.x + end.x) / 2.0 + lane_offset];
    let mut y_tracks = vec![start.y, end.y, (start.y + end.y) / 2.0 + lane_offset];
    for rect in request
        .obstacles
        .iter()
        .chain([&request.source, &request.target])
    {
        x_tracks.extend([
            rect.x - ROUTE_CLEARANCE - LANE_SPACING - lane_offset,
            rect.x + rect.width + ROUTE_CLEARANCE + LANE_SPACING + lane_offset,
        ]);
        y_tracks.extend([
            rect.y - ROUTE_CLEARANCE - LANE_SPACING - lane_offset,
            rect.y + rect.height + ROUTE_CLEARANCE + LANE_SPACING + lane_offset,
        ]);
    }
    for route in request.reserved_routes {
        for point in route {
            x_tracks.extend([point.x - LANE_SPACING, point.x + LANE_SPACING]);
            y_tracks.extend([point.y - LANE_SPACING, point.y + LANE_SPACING]);
        }
    }
    if let Some(bounds) = request.bounds {
        x_tracks.extend([
            bounds.x + ROUTE_CLEARANCE,
            bounds.x + bounds.width - ROUTE_CLEARANCE,
        ]);
        y_tracks.extend([
            bounds.y + ROUTE_CLEARANCE,
            bounds.y + bounds.height - ROUTE_CLEARANCE,
        ]);
    }
    sort_tracks(&mut x_tracks, (start.x + end.x) / 2.0);
    sort_tracks(&mut y_tracks, (start.y + end.y) / 2.0);
    x_tracks.truncate(48);
    y_tracks.truncate(48);

    for &x in &x_tracks {
        candidates.push(compact(vec![
            start,
            DiagramPoint { x, y: start.y },
            DiagramPoint { x, y: end.y },
            end,
        ]));
    }
    for &y in &y_tracks {
        candidates.push(compact(vec![
            start,
            DiagramPoint { x: start.x, y },
            DiagramPoint { x: end.x, y },
            end,
        ]));
    }
    for &x in &x_tracks {
        for &y in &y_tracks {
            candidates.push(compact(vec![
                start,
                DiagramPoint { x, y: start.y },
                DiagramPoint { x, y },
                DiagramPoint { x: end.x, y },
                end,
            ]));
            candidates.push(compact(vec![
                start,
                DiagramPoint { x: start.x, y },
                DiagramPoint { x, y },
                DiagramPoint { x, y: end.y },
                end,
            ]));
        }
    }
}

fn sort_tracks(values: &mut Vec<f64>, center: f64) {
    values.retain(|value| value.is_finite());
    values.sort_by(|left, right| {
        (left - center)
            .abs()
            .total_cmp(&(right - center).abs())
            .then_with(|| left.total_cmp(right))
    });
    values.dedup_by(|left, right| (*left - *right).abs() < 0.001);
}

fn route_cost(points: &[DiagramPoint]) -> f64 {
    let length: f64 = points
        .windows(2)
        .map(|segment| segment_length(segment[0], segment[1]))
        .sum();
    length + points.len().saturating_sub(2) as f64 * LANE_SPACING
}

fn route_is_valid(points: &[DiagramPoint], request: &RouteRequest<'_>) -> bool {
    points.len() >= 2
        && points
            .iter()
            .all(|point| point.x.is_finite() && point.y.is_finite())
        && points
            .windows(2)
            .all(|segment| is_orthogonal(segment[0], segment[1]))
        && route_is_clear(points, request.obstacles)
        && route_avoids_reserved(
            points,
            request.reserved_routes,
            request.allow_shared_departure,
        )
        && request
            .bounds
            .is_none_or(|bounds| points.iter().all(|point| point_inside(*point, bounds)))
}

fn is_orthogonal(start: DiagramPoint, end: DiagramPoint) -> bool {
    (start.x - end.x).abs() < 0.001 || (start.y - end.y).abs() < 0.001
}

fn point_inside(point: DiagramPoint, rect: RouteRect) -> bool {
    point.x >= rect.x
        && point.x <= rect.x + rect.width
        && point.y >= rect.y
        && point.y <= rect.y + rect.height
}

fn rect_inside(inner: RouteRect, outer: RouteRect) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.x + inner.width <= outer.x + outer.width
        && inner.y + inner.height <= outer.y + outer.height
}

fn rects_overlap(left: RouteRect, right: RouteRect) -> bool {
    left.x < right.x + right.width
        && left.x + left.width > right.x
        && left.y < right.y + right.height
        && left.y + left.height > right.y
}

pub fn route_avoids_reserved(
    candidate: &[DiagramPoint],
    reserved_routes: &[Vec<DiagramPoint>],
    allow_shared_departure: bool,
) -> bool {
    reserved_routes.iter().all(|reserved| {
        let candidate_last = candidate.len().saturating_sub(2);
        let reserved_last = reserved.len().saturating_sub(2);
        candidate
            .windows(2)
            .enumerate()
            .all(|(candidate_index, candidate_segment)| {
                reserved
                    .windows(2)
                    .enumerate()
                    .all(|(reserved_index, reserved_segment)| {
                        let shared_departure = allow_shared_departure
                            && candidate_index == 0
                            && reserved_index == 0
                            && candidate_segment[0] == reserved_segment[0];
                        let shared_arrival = candidate_index == candidate_last
                            && reserved_index == reserved_last
                            && candidate_segment[1] == reserved_segment[1];
                        shared_departure
                            || shared_arrival
                            || !segments_overlap(
                                candidate_segment[0],
                                candidate_segment[1],
                                reserved_segment[0],
                                reserved_segment[1],
                            )
                    })
            })
    })
}

pub fn segments_overlap(
    a1: DiagramPoint,
    a2: DiagramPoint,
    b1: DiagramPoint,
    b2: DiagramPoint,
) -> bool {
    const EPSILON: f64 = 0.001;
    let a_vertical = (a1.x - a2.x).abs() < EPSILON;
    let b_vertical = (b1.x - b2.x).abs() < EPSILON;
    if a_vertical && b_vertical && (a1.x - b1.x).abs() < EPSILON {
        return ranges_overlap(a1.y, a2.y, b1.y, b2.y, EPSILON);
    }
    let a_horizontal = (a1.y - a2.y).abs() < EPSILON;
    let b_horizontal = (b1.y - b2.y).abs() < EPSILON;
    a_horizontal
        && b_horizontal
        && (a1.y - b1.y).abs() < EPSILON
        && ranges_overlap(a1.x, a2.x, b1.x, b2.x, EPSILON)
}

fn ranges_overlap(a1: f64, a2: f64, b1: f64, b2: f64, epsilon: f64) -> bool {
    a1.min(a2).max(b1.min(b2)) < a1.max(a2).min(b1.max(b2)) - epsilon
}

fn attached_endpoints(
    source: RouteRect,
    target: RouteRect,
    horizontal: bool,
) -> (DiagramPoint, DiagramPoint) {
    let source_center = source.center();
    let target_center = target.center();
    if horizontal {
        let rightward = target_center.x >= source_center.x;
        (
            DiagramPoint {
                x: if rightward {
                    source.x + source.width
                } else {
                    source.x
                },
                y: source_center.y,
            },
            DiagramPoint {
                x: if rightward {
                    target.x
                } else {
                    target.x + target.width
                },
                y: target_center.y,
            },
        )
    } else {
        let downward = target_center.y >= source_center.y;
        (
            DiagramPoint {
                x: source_center.x,
                y: if downward {
                    source.y + source.height
                } else {
                    source.y
                },
            },
            DiagramPoint {
                x: target_center.x,
                y: if downward {
                    target.y
                } else {
                    target.y + target.height
                },
            },
        )
    }
}

fn compact(points: Vec<DiagramPoint>) -> Vec<DiagramPoint> {
    let mut result = Vec::new();
    for point in points {
        if result.last().is_some_and(|last: &DiagramPoint| {
            (last.x - point.x).abs() < f64::EPSILON && (last.y - point.y).abs() < f64::EPSILON
        }) {
            continue;
        }
        result.push(point);
    }
    result
}

pub fn route_is_clear(points: &[DiagramPoint], obstacles: &[RouteRect]) -> bool {
    points.windows(2).all(|segment| {
        obstacles
            .iter()
            .all(|obstacle| !segment_intersects_rect(segment[0], segment[1], *obstacle))
    })
}

pub fn segment_intersects_rect(a: DiagramPoint, b: DiagramPoint, rect: RouteRect) -> bool {
    let left = rect.x - ROUTE_CLEARANCE;
    let right = rect.x + rect.width + ROUTE_CLEARANCE;
    let top = rect.y - ROUTE_CLEARANCE;
    let bottom = rect.y + rect.height + ROUTE_CLEARANCE;
    if (a.x - b.x).abs() < f64::EPSILON {
        let y_min = a.y.min(b.y);
        let y_max = a.y.max(b.y);
        a.x >= left && a.x <= right && y_max >= top && y_min <= bottom
    } else if (a.y - b.y).abs() < f64::EPSILON {
        let x_min = a.x.min(b.x);
        let x_max = a.x.max(b.x);
        a.y >= top && a.y <= bottom && x_max >= left && x_min <= right
    } else {
        true
    }
}

fn segment_intersects_rect_exact(a: DiagramPoint, b: DiagramPoint, rect: RouteRect) -> bool {
    let left = rect.x;
    let right = rect.x + rect.width;
    let top = rect.y;
    let bottom = rect.y + rect.height;
    if (a.x - b.x).abs() < f64::EPSILON {
        let y_min = a.y.min(b.y);
        let y_max = a.y.max(b.y);
        a.x >= left && a.x <= right && y_max >= top && y_min <= bottom
    } else if (a.y - b.y).abs() < f64::EPSILON {
        let x_min = a.x.min(b.x);
        let x_max = a.x.max(b.x);
        a.y >= top && a.y <= bottom && x_max >= left && x_min <= right
    } else {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_avoids_blocking_rectangle_and_is_orthogonal() {
        let obstacle = RouteRect {
            x: 190.0,
            y: 80.0,
            width: 100.0,
            height: 100.0,
        };
        let points = orthogonal_route(RouteRequest {
            source: RouteRect {
                x: 20.0,
                y: 100.0,
                width: 100.0,
                height: 60.0,
            },
            target: RouteRect {
                x: 360.0,
                y: 100.0,
                width: 100.0,
                height: 60.0,
            },
            obstacles: &[obstacle],
            lane_index: 0,
            reserved_routes: &[],
            allow_shared_departure: false,
            bounds: None,
        })
        .expect("validated route");
        assert!(route_is_clear(&points, &[obstacle]));
        assert!(
            points
                .windows(2)
                .all(|p| p[0].x == p[1].x || p[0].y == p[1].y)
        );
    }

    #[test]
    fn vertical_route_avoids_blocking_rectangle_and_is_orthogonal() {
        let obstacle = RouteRect {
            x: 80.0,
            y: 190.0,
            width: 100.0,
            height: 100.0,
        };
        let points = orthogonal_route(RouteRequest {
            source: RouteRect {
                x: 100.0,
                y: 20.0,
                width: 60.0,
                height: 100.0,
            },
            target: RouteRect {
                x: 100.0,
                y: 360.0,
                width: 60.0,
                height: 100.0,
            },
            obstacles: &[obstacle],
            lane_index: 0,
            reserved_routes: &[],
            allow_shared_departure: false,
            bounds: None,
        })
        .expect("validated route");
        assert!(route_is_clear(&points, &[obstacle]));
        assert!(
            points
                .windows(2)
                .all(|p| p[0].x == p[1].x || p[0].y == p[1].y)
        );
    }

    #[test]
    fn parallel_lanes_are_deterministically_separated() {
        let base = RouteRequest {
            source: RouteRect {
                x: 20.0,
                y: 20.0,
                width: 100.0,
                height: 60.0,
            },
            target: RouteRect {
                x: 360.0,
                y: 20.0,
                width: 100.0,
                height: 60.0,
            },
            obstacles: &[],
            lane_index: 0,
            reserved_routes: &[],
            allow_shared_departure: false,
            bounds: None,
        };
        let first = orthogonal_route(base).expect("first route");
        let second = orthogonal_route(RouteRequest {
            lane_index: 1,
            ..base
        })
        .expect("second route");
        assert_ne!(first, second);
    }

    #[test]
    fn unrelated_routes_cannot_overlap_reserved_segments() {
        let reserved = vec![vec![
            DiagramPoint { x: 120.0, y: 50.0 },
            DiagramPoint { x: 240.0, y: 50.0 },
        ]];
        let route = orthogonal_route(RouteRequest {
            source: RouteRect {
                x: 20.0,
                y: 20.0,
                width: 100.0,
                height: 60.0,
            },
            target: RouteRect {
                x: 240.0,
                y: 20.0,
                width: 100.0,
                height: 60.0,
            },
            obstacles: &[],
            lane_index: 0,
            reserved_routes: &reserved,
            allow_shared_departure: false,
            bounds: None,
        })
        .expect("validated route");
        assert!(route_avoids_reserved(&route, &reserved, false));
    }

    #[test]
    fn only_a_common_source_may_share_the_departure_segment() {
        let reserved = vec![vec![
            DiagramPoint { x: 120.0, y: 50.0 },
            DiagramPoint { x: 180.0, y: 50.0 },
        ]];
        let candidate = vec![
            DiagramPoint { x: 120.0, y: 50.0 },
            DiagramPoint { x: 180.0, y: 50.0 },
            DiagramPoint { x: 180.0, y: 120.0 },
        ];
        assert!(route_avoids_reserved(&candidate, &reserved, true));
        assert!(!route_avoids_reserved(&candidate, &reserved, false));
    }

    #[test]
    fn a_common_target_may_share_only_the_final_arrival_segment() {
        let reserved = vec![vec![
            DiagramPoint { x: 120.0, y: 120.0 },
            DiagramPoint { x: 180.0, y: 120.0 },
            DiagramPoint { x: 180.0, y: 50.0 },
        ]];
        let candidate = vec![
            DiagramPoint { x: 240.0, y: 120.0 },
            DiagramPoint { x: 180.0, y: 120.0 },
            DiagramPoint { x: 180.0, y: 50.0 },
        ];
        assert!(route_avoids_reserved(&candidate, &reserved, false));

        let unrelated_overlap = vec![
            DiagramPoint { x: 240.0, y: 120.0 },
            DiagramPoint { x: 180.0, y: 120.0 },
            DiagramPoint { x: 180.0, y: 80.0 },
        ];
        assert!(!route_avoids_reserved(
            &unrelated_overlap,
            &reserved,
            false
        ));
    }

    #[test]
    fn label_search_uses_clear_positions_away_from_a_blocked_midpoint() {
        let points = vec![
            DiagramPoint { x: 0.0, y: 50.0 },
            DiagramPoint { x: 300.0, y: 50.0 },
        ];
        let midpoint_obstacle = RouteRect {
            x: 135.0,
            y: -20.0,
            width: 30.0,
            height: 140.0,
        };
        let anchor = route_label_anchor_avoiding(&points, &[midpoint_obstacle], &[], None)
            .expect("clear quarter-segment label position");
        assert_eq!(anchor.x, 75.0);
        assert!(!rects_overlap(label_rect(anchor), midpoint_obstacle));
    }

    #[test]
    fn self_transition_uses_a_valid_external_loop() {
        let state = RouteRect {
            x: 100.0,
            y: 100.0,
            width: 120.0,
            height: 80.0,
        };
        let route = orthogonal_route(RouteRequest {
            source: state,
            target: state,
            obstacles: &[],
            lane_index: 0,
            reserved_routes: &[],
            allow_shared_departure: false,
            bounds: None,
        })
        .expect("validated self-transition loop");
        assert_eq!(route.len(), 5);
        assert_ne!(route.first(), route.last());
        assert!(route.iter().any(|point| {
            point.x < state.x
                || point.x > state.x + state.width
                || point.y < state.y
                || point.y > state.y + state.height
        }));
    }

    #[test]
    fn batch_router_applies_one_policy_to_every_diagram_family_adapter() {
        let source = RouteRect {
            x: 20.0,
            y: 20.0,
            width: 100.0,
            height: 60.0,
        };
        let edges = vec![
            DiagramRouteEdge {
                id: "a".into(),
                source_id: "source".into(),
                target_id: "one".into(),
                source,
                target: RouteRect {
                    x: 320.0,
                    y: 20.0,
                    width: 100.0,
                    height: 60.0,
                },
            },
            DiagramRouteEdge {
                id: "b".into(),
                source_id: "other".into(),
                target_id: "two".into(),
                source: RouteRect {
                    x: 20.0,
                    y: 120.0,
                    width: 100.0,
                    height: 60.0,
                },
                target: RouteRect {
                    x: 320.0,
                    y: 120.0,
                    width: 100.0,
                    height: 60.0,
                },
            },
        ];
        let routed = route_diagram(&edges, &[]).expect("batch route");
        assert_eq!(routed.len(), 2);
        assert!(route_avoids_reserved(
            &routed[1].points,
            &[routed[0].points.clone()],
            false
        ));
        assert!(label_rect(routed[0].label_anchor).width >= LABEL_WIDTH);
    }

    #[test]
    fn label_anchor_uses_the_longest_clear_segment() {
        let anchor = route_label_anchor(&[
            DiagramPoint { x: 10.0, y: 10.0 },
            DiagramPoint { x: 30.0, y: 10.0 },
            DiagramPoint { x: 30.0, y: 110.0 },
        ]);
        assert_eq!(anchor, DiagramPoint { x: 50.0, y: 60.0 });
    }

    #[test]
    fn router_rejects_unproven_geometry_instead_of_returning_a_fallback() {
        let source = RouteRect {
            x: 140.0,
            y: 140.0,
            width: 40.0,
            height: 40.0,
        };
        let target = RouteRect {
            x: 260.0,
            y: 140.0,
            width: 40.0,
            height: 40.0,
        };
        let barriers = [
            RouteRect {
                x: 0.0,
                y: 90.0,
                width: 400.0,
                height: 20.0,
            },
            RouteRect {
                x: 0.0,
                y: 210.0,
                width: 400.0,
                height: 20.0,
            },
            RouteRect {
                x: 90.0,
                y: 0.0,
                width: 20.0,
                height: 320.0,
            },
            RouteRect {
                x: 330.0,
                y: 0.0,
                width: 20.0,
                height: 320.0,
            },
            RouteRect {
                x: 205.0,
                y: 90.0,
                width: 20.0,
                height: 140.0,
            },
        ];
        let result = orthogonal_route(RouteRequest {
            source,
            target,
            obstacles: &barriers,
            lane_index: 0,
            reserved_routes: &[],
            allow_shared_departure: false,
            bounds: Some(RouteRect {
                x: 0.0,
                y: 0.0,
                width: 400.0,
                height: 320.0,
            }),
        });
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("existing geometry was preserved")
        );
    }
}
