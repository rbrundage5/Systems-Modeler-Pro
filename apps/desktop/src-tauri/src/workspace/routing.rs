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
    pub reserved_routes: &'a [Vec<DiagramPoint>],
    pub allow_shared_departure: bool,
}

/// Shared deterministic orthogonal router for BDD and IBD presentations.
/// It never intentionally returns a segment through an obstacle. If the direct
/// dogleg is blocked, perpendicular outer channels are searched in deterministic
/// order. Horizontal relationships detour above/below obstacles; vertical
/// relationships detour left/right.
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
    }

    candidates
        .into_iter()
        .find(|candidate| {
            route_is_clear(candidate, request.obstacles)
                && route_avoids_reserved(
                    candidate,
                    request.reserved_routes,
                    request.allow_shared_departure,
                )
        })
        .unwrap_or_else(|| {
            // Preserve endpoint attachment and orthogonality even in a severely
            // constrained diagram. The fallback also escapes perpendicular to
            // the primary relationship direction, so it cannot reproduce the
            // blocked-axis failure mode above.
            let padding = 10.0 * (ROUTE_CLEARANCE + LANE_SPACING) + lane_offset;
            if horizontal {
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
            } else {
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
            }
        })
}

pub fn route_avoids_reserved(
    candidate: &[DiagramPoint],
    reserved_routes: &[Vec<DiagramPoint>],
    allow_shared_departure: bool,
) -> bool {
    reserved_routes.iter().all(|reserved| {
        candidate.windows(2).enumerate().all(|(candidate_index, candidate_segment)| {
            reserved.windows(2).enumerate().all(|(reserved_index, reserved_segment)| {
                let shared_departure = allow_shared_departure
                    && candidate_index == 0
                    && reserved_index == 0
                    && candidate_segment[0] == reserved_segment[0];
                shared_departure
                    || !segments_overlap(
                        candidate_segment[0], candidate_segment[1], reserved_segment[0],
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
        });
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
            reserved_routes: &[],
            allow_shared_departure: false,
        };
        let first = orthogonal_route(base);
        let second = orthogonal_route(RouteRequest {
            lane_index: 1,
            ..base
        });
        assert_ne!(first, second);
    }

    #[test]
    fn unrelated_routes_cannot_overlap_reserved_segments() {
        let reserved = vec![vec![
            DiagramPoint { x: 120.0, y: 50.0 },
            DiagramPoint { x: 240.0, y: 50.0 },
        ]];
        let route = orthogonal_route(RouteRequest {
            source: RouteRect { x: 20.0, y: 20.0, width: 100.0, height: 60.0 },
            target: RouteRect { x: 240.0, y: 20.0, width: 100.0, height: 60.0 },
            obstacles: &[],
            lane_index: 0,
            reserved_routes: &reserved,
            allow_shared_departure: false,
        });
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
}
