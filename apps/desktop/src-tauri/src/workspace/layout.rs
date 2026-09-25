use std::collections::{BTreeMap, BTreeSet, VecDeque};
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

#[cfg(test)]
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
    // Presentation identifiers can legitimately be generated UUIDs. They are
    // identity, not ordering semantics. Preserve the caller-provided node order
    // and use it as the deterministic tie-breaker for equal graph levels.
    let mut nodes_by_id = BTreeMap::new();
    let mut ordered_ids = Vec::new();
    for mut node in nodes {
        node.width = node.width.max(1.0);
        node.height = node.height.max(1.0);
        if !nodes_by_id.contains_key(&node.id) {
            ordered_ids.push(node.id.clone());
        }
        nodes_by_id.insert(node.id.clone(), node);
    }
    let rank: BTreeMap<_, _> = ordered_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.clone(), index))
        .collect();
    let mut indegree: BTreeMap<_, usize> = ordered_ids.iter().map(|id| (id.clone(), 0)).collect();
    let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (source, target) in edges {
        if source == target
            || !nodes_by_id.contains_key(source)
            || !nodes_by_id.contains_key(target)
        {
            continue;
        }
        let targets = outgoing.entry(source.clone()).or_default();
        if !targets.contains(target) {
            targets.push(target.clone());
            *indegree.entry(target.clone()).or_default() += 1;
        }
    }
    for targets in outgoing.values_mut() {
        targets.sort_by_key(|target| rank.get(target).copied().unwrap_or(usize::MAX));
    }

    let mut ready: BTreeSet<(usize, String)> = indegree
        .iter()
        .filter_map(|(id, degree)| {
            (*degree == 0).then_some((rank.get(id).copied().unwrap_or(usize::MAX), id.clone()))
        })
        .collect();
    let mut levels: BTreeMap<String, usize> =
        ordered_ids.iter().map(|id| (id.clone(), 0)).collect();
    let mut placed = BTreeSet::new();
    while let Some((_, id)) = ready.pop_first() {
        placed.insert(id.clone());
        let next_level = levels[&id].saturating_add(1);
        for target in outgoing.get(&id).into_iter().flatten() {
            levels
                .entry(target.clone())
                .and_modify(|level| *level = (*level).max(next_level));
            let degree = indegree.get_mut(target).expect("known layout node");
            *degree -= 1;
            if *degree == 0 {
                ready.insert((
                    rank.get(target).copied().unwrap_or(usize::MAX),
                    target.clone(),
                ));
            }
        }
    }

    // A state machine commonly contains transition cycles, so Kahn layering alone
    // leaves an entire strongly connected portion on level zero. Complete a stable
    // spanning hierarchy for those remaining vertices while ignoring back edges.
    // This keeps the layout bounded and produces useful top-to-bottom/left-to-right
    // flow without introducing a separate graph-layout subsystem.
    for start in &ordered_ids {
        if placed.contains(start) {
            continue;
        }
        placed.insert(start.clone());
        let mut pending = VecDeque::from([start.clone()]);
        while let Some(source) = pending.pop_front() {
            let next_level = levels[&source].saturating_add(1);
            for target in outgoing.get(&source).into_iter().flatten() {
                if placed.insert(target.clone()) {
                    levels.insert(target.clone(), next_level);
                    pending.push_back(target.clone());
                }
            }
        }
    }

    let mut by_level: BTreeMap<usize, Vec<&LayoutNode>> = BTreeMap::new();
    for id in &ordered_ids {
        let level = levels[id];
        by_level.entry(level).or_default().push(&nodes_by_id[id]);
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
                LayoutNode {
                    id: "wide".into(),
                    width: 320.0,
                    height: 80.0,
                },
                LayoutNode {
                    id: "next".into(),
                    width: 240.0,
                    height: 160.0,
                },
                LayoutNode {
                    id: "peer".into(),
                    width: 280.0,
                    height: 100.0,
                },
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
            PreferredFlowDirection::TopToBottom,
        );
        assert!(positions.values().all(|(x, y)| *x < 1000.0 && *y < 1000.0));
        assert_ne!(positions["a"].1, positions["b"].1);
    }
    #[test]
    fn equal_level_nodes_preserve_caller_order_instead_of_uuid_lexical_order() {
        let positions = hierarchical_positions_sized(
            [
                LayoutNode {
                    id: "z-first".into(),
                    width: 100.0,
                    height: 60.0,
                },
                LayoutNode {
                    id: "a-second".into(),
                    width: 100.0,
                    height: 60.0,
                },
                LayoutNode {
                    id: "m-third".into(),
                    width: 100.0,
                    height: 60.0,
                },
            ],
            &[],
            PreferredFlowDirection::TopToBottom,
        );
        assert!(positions["z-first"].0 < positions["a-second"].0);
        assert!(positions["a-second"].0 < positions["m-third"].0);
    }
}
