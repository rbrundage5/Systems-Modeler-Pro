use std::collections::{BTreeMap, BTreeSet};
use systems_modeler_core::PreferredFlowDirection;

const ORIGIN_X: f64 = 80.0;
const ORIGIN_Y: f64 = 90.0;
const LEVEL_GAP: f64 = 120.0;
const LANE_GAP: f64 = 80.0;

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutNode {
    pub id: String,
    pub width: f64,
    pub height: f64,
}

pub fn hierarchical_positions(
    node_ids: impl IntoIterator<Item = String>,
    edges: &[(String, String)],
    flow: PreferredFlowDirection,
) -> BTreeMap<String, (f64, f64)> {
    hierarchical_positions_sized(
        node_ids.into_iter().map(|id| LayoutNode {
            id,
            width: 100.0,
            height: 60.0,
        }),
        edges,
        flow,
    )
}

/// Deterministic, size-aware layout for the six current diagram adapters.
/// Kahn layering avoids the runaway coordinates previously produced by cycles;
/// remaining cyclic vertices stay in a stable lane at level zero.
pub fn hierarchical_positions_sized(
    nodes: impl IntoIterator<Item = LayoutNode>,
    edges: &[(String, String)],
    flow: PreferredFlowDirection,
) -> BTreeMap<String, (f64, f64)> {
    let nodes: BTreeMap<_, _> = nodes
        .into_iter()
        .map(|mut node| {
            node.width = node.width.max(1.0);
            node.height = node.height.max(1.0);
            (node.id.clone(), node)
        })
        .collect();
    let ids: BTreeSet<_> = nodes.keys().cloned().collect();
    let mut indegree: BTreeMap<_, usize> = ids.iter().map(|id| (id.clone(), 0)).collect();
    let mut outgoing: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (source, target) in edges {
        if source == target || !ids.contains(source) || !ids.contains(target) {
            continue;
        }
        if outgoing
            .entry(source.clone())
            .or_default()
            .insert(target.clone())
        {
            *indegree.entry(target.clone()).or_default() += 1;
        }
    }

    let mut ready: BTreeSet<_> = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(id.clone()))
        .collect();
    let mut levels: BTreeMap<String, usize> = ids.iter().map(|id| (id.clone(), 0)).collect();
    while let Some(id) = ready.pop_first() {
        let next_level = levels[&id].saturating_add(1);
        for target in outgoing.get(&id).into_iter().flatten() {
            levels
                .entry(target.clone())
                .and_modify(|level| *level = (*level).max(next_level));
            let degree = indegree.get_mut(target).expect("known layout node");
            *degree -= 1;
            if *degree == 0 {
                ready.insert(target.clone());
            }
        }
    }

    let mut by_level: BTreeMap<usize, Vec<&LayoutNode>> = BTreeMap::new();
    for (id, level) in &levels {
        by_level.entry(*level).or_default().push(&nodes[id]);
    }

    let mut positions = BTreeMap::new();
    let mut level_cursor = match flow {
        PreferredFlowDirection::LeftToRight => ORIGIN_X,
        PreferredFlowDirection::TopToBottom | PreferredFlowDirection::Freeform => ORIGIN_Y,
    };
    for level_nodes in by_level.values() {
        let mut lane_cursor = match flow {
            PreferredFlowDirection::LeftToRight => ORIGIN_Y,
            PreferredFlowDirection::TopToBottom | PreferredFlowDirection::Freeform => ORIGIN_X,
        };
        let level_extent = level_nodes
            .iter()
            .map(|node| match flow {
                PreferredFlowDirection::LeftToRight => node.width,
                PreferredFlowDirection::TopToBottom | PreferredFlowDirection::Freeform => {
                    node.height
                }
            })
            .fold(1.0, f64::max);
        for node in level_nodes {
            let position = match flow {
                PreferredFlowDirection::LeftToRight => (level_cursor, lane_cursor),
                PreferredFlowDirection::TopToBottom | PreferredFlowDirection::Freeform => {
                    (lane_cursor, level_cursor)
                }
            };
            positions.insert(node.id.clone(), position);
            lane_cursor += match flow {
                PreferredFlowDirection::LeftToRight => node.height,
                PreferredFlowDirection::TopToBottom | PreferredFlowDirection::Freeform => {
                    node.width
                }
            } + LANE_GAP;
        }
        level_cursor += level_extent + LEVEL_GAP;
    }
    positions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hierarchy_is_deterministic_and_follows_registered_flow() {
        let ids = ["c", "a", "b"].map(String::from);
        let edges = vec![("a".into(), "b".into()), ("b".into(), "c".into())];
        let first =
            hierarchical_positions(ids.clone(), &edges, PreferredFlowDirection::TopToBottom);
        let second = hierarchical_positions(ids, &edges, PreferredFlowDirection::TopToBottom);
        assert_eq!(first, second);
        assert!(first["a"].1 < first["b"].1 && first["b"].1 < first["c"].1);
    }

    #[test]
    fn actual_sizes_and_spacing_prevent_overlap() {
        let positions = hierarchical_positions_sized(
            [
                LayoutNode { id: "wide".into(), width: 320.0, height: 80.0 },
                LayoutNode { id: "next".into(), width: 240.0, height: 160.0 },
                LayoutNode { id: "peer".into(), width: 280.0, height: 100.0 },
            ],
            &[("wide".into(), "next".into())],
            PreferredFlowDirection::TopToBottom,
        );
        assert!(
            positions["peer"].0 >= positions["wide"].0 + 320.0 + LANE_GAP
                || positions["wide"].0 >= positions["peer"].0 + 280.0 + LANE_GAP
        );
        assert!(positions["next"].1 >= positions["wide"].1 + 80.0 + LEVEL_GAP);
    }

    #[test]
    fn cycles_do_not_create_extreme_off_screen_levels() {
        let positions = hierarchical_positions(
            ["a", "b", "c"].map(String::from),
            &[("a".into(), "b".into()), ("b".into(), "a".into())],
            PreferredFlowDirection::LeftToRight,
        );
        assert!(positions.values().all(|(x, y)| *x < 1000.0 && *y < 1000.0));
    }
}
