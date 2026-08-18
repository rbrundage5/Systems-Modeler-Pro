use super::DiagramPoint;
use serde::{Deserialize, Serialize};

pub const ROUTE_CLEARANCE: f64 = 18.0;
pub const LANE_SPACING: f64 = 12.0;

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
}

/// Shared deterministic orthogonal router for BDD and IBD presentations.
/// It never intentionally returns a segment through an obstacle. If direct and
/// side-channel candidates are blocked, progressively wider outer channels are
/// searched in deterministic order.
pub fn orthogonal_route(request: RouteRequest<'_>) -> Vec<DiagramPoint> {
    let source_center = request.source.center();
    let target_center = request.target.center();
    let dx = target_center.x - source_center.x;
    let dy = target_center.y - source_center.y;
    let horizontal = dx.abs() >= dy.abs();
    let lane_offset = request.lane_index as f64 * LANE_SPACING;

    let (start, end) = attached_endpoints(request.source, request.target, horizontal);
    let mut candidates = Vec::new();
    if horizontal {
        let mid_x = (start.x + end.x) / 2.0 + lane_offset;
        candidates.push(compact(vec![
            start,
            DiagramPoint {
                x: mid_x,
                y: start.y,
            },
            DiagramPoint { x: mid_x, y: end.y },
            end,
        ]));
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
        for ring in 1..=8 {
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
    } else {
        let mid_y = (start.y + end.y) / 2.0 + lane_offset;
        candidates.push(compact(vec![
            start,
            DiagramPoint {
                x: start.x,
                y: mid_y,
            },
            DiagramPoint { x: end.x, y: mid_y },
            end,
        ]));
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
        for ring in 1..=8 {
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
    }

    candidates
        .into_iter()
        .find(|candidate| route_is_clear(candidate, request.obstacles))
        .unwrap_or_else(|| {
            // Preserve endpoint attachment and orthogonality even in a severely
            // constrained diagram; the outer ring is deterministic and avoids
            // silently falling back to a diagonal through model elements.
            let padding = 10.0 * (ROUTE_CLEARANCE + LANE_SPACING) + lane_offset;
            if horizontal {
                let x = request.source.x.min(request.target.x).min(
                    request
                        .obstacles
                        .iter()
                        .map(|o| o.x)
                        .fold(f64::INFINITY, f64::min),
                ) - padding;
                compact(vec![
                    start,
                    DiagramPoint { x, y: start.y },
                    DiagramPoint { x, y: end.y },
                    end,
                ])
            } else {
                let y = request.source.y.min(request.target.y).min(
                    request
                        .obstacles
                        .iter()
                        .map(|o| o.y)
                        .fold(f64::INFINITY, f64::min),
                ) - padding;
                compact(vec![
                    start,
                    DiagramPoint { x: start.x, y },
                    DiagramPoint { x: end.x, y },
                    end,
                ])
            }
        })
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
        });
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
        };
        let first = orthogonal_route(base);
        let second = orthogonal_route(RouteRequest {
            lane_index: 1,
            ..base
        });
        assert_ne!(first, second);
    }
}
