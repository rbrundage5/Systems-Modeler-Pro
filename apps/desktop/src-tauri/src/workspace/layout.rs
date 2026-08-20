use std::collections::{BTreeMap, BTreeSet};
use systems_modeler_core::PreferredFlowDirection;

const ORIGIN_X: f64 = 80.0;
const ORIGIN_Y: f64 = 90.0;
const LEVEL_GAP: f64 = 260.0;
const LANE_GAP: f64 = 180.0;

pub fn hierarchical_positions(
    node_ids: impl IntoIterator<Item = String>,
    edges: &[(String, String)],
    flow: PreferredFlowDirection,
) -> BTreeMap<String, (f64, f64)> {
    let ids: BTreeSet<_> = node_ids.into_iter().collect();
    let mut levels: BTreeMap<String, usize> = ids.iter().map(|id| (id.clone(), 0)).collect();
    for _ in 0..ids.len() {
        let mut changed = false;
        for (source, target) in edges {
            if source == target || !ids.contains(source) || !ids.contains(target) {
                continue;
            }
            let next = levels[source].saturating_add(1).min(ids.len());
            if next > levels[target] {
                levels.insert(target.clone(), next);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let mut lanes: BTreeMap<usize, usize> = BTreeMap::new();
    levels
        .into_iter()
        .map(|(id, level)| {
            let lane = lanes.entry(level).or_default();
            let (x, y) = match flow {
                PreferredFlowDirection::TopToBottom | PreferredFlowDirection::Freeform => (
                    ORIGIN_X + *lane as f64 * LANE_GAP,
                    ORIGIN_Y + level as f64 * LEVEL_GAP,
                ),
                PreferredFlowDirection::LeftToRight => (
                    ORIGIN_X + level as f64 * LEVEL_GAP,
                    ORIGIN_Y + *lane as f64 * LANE_GAP,
                ),
            };
            *lane += 1;
            (id, (x, y))
        })
        .collect()
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
        let freeform = hierarchical_positions(
            ["c", "a", "b"].map(String::from),
            &edges,
            PreferredFlowDirection::Freeform,
        );
        assert!(freeform["a"].1 < freeform["b"].1);
    }
}
