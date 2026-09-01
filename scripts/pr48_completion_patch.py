from pathlib import Path

path = Path("apps/desktop/src-tauri/src/workspace/bulk_model/pr48_tests.rs")
if path.exists():
    text = path.read_text()
    text = text.replace(
        "use systems_modeler_core::{ActivityEdgeKind, ActivityNodeKind, ActivitySemanticId, ElementKind};",
        "use systems_modeler_core::{ActivityEdgeKind, ActivitySemanticId, ElementKind};",
    )
    path.write_text(text)

path = Path("apps/desktop/src-tauri/src/workspace/spreadsheet_import/pr48_behavior.rs")
if path.exists():
    text = path.read_text()
    import_anchor = "use super::*;\n"
    imports = """use super::*;
use crate::workspace::bulk_model::{
    ActionBuildKind, ActivityBuildOperation, ActivityEndpointReference, ActivityNodeBuildKind,
    ActivityNodeReference, ActivityPartitionReference, ActivityReference, RegionParentReference,
    RegionReference, StateMachineBuildOperation, StateMachineReference, StructuredNodeReference,
    TriggerBuild, VertexBuildKind, VertexReference,
};
"""
    if import_anchor not in text:
        raise SystemExit("missing PR48 spreadsheet behavior import anchor")
    text = text.replace(import_anchor, imports, 1)

    macro_start = text.index("macro_rules! nested_ref_fn")
    macro_end = text.index("\n\nfn is_activity_node_kind", macro_start)
    reference_helpers = r'''fn partition_ref(
    map: &SpreadsheetImportMap,
    activities: &ActivityRepository,
    _behavior: &BehaviorRepository,
    planned: &BehaviorPlanningIndex,
    token: &str,
) -> Result<ActivityPartitionReference, SpreadsheetImportDiagnostic> {
    let key = external_key(&map.source_namespace, token);
    if let Some(identity) = activities.external_ids.get(&key).copied() {
        return match identity {
            ActivitySemanticId::Partition(id) => Ok(BuildReference::Existing(id)),
            _ => Err(diagnostic(
                Some(map), None, None, None, Some(token.into()),
                "BEHAVIOR_REFERENCE_KIND_INVALID",
                format!("reference '{token}' identifies the wrong Activity semantic kind"),
            )),
        };
    }
    planned_activity_reference(map, planned, token, BehaviorRowKind::ActivityPartition)
}

fn structured_ref(
    map: &SpreadsheetImportMap,
    activities: &ActivityRepository,
    _behavior: &BehaviorRepository,
    planned: &BehaviorPlanningIndex,
    token: &str,
) -> Result<StructuredNodeReference, SpreadsheetImportDiagnostic> {
    let key = external_key(&map.source_namespace, token);
    if let Some(identity) = activities.external_ids.get(&key).copied() {
        return match identity {
            ActivitySemanticId::StructuredNode(id) => Ok(BuildReference::Existing(id)),
            _ => Err(diagnostic(
                Some(map), None, None, None, Some(token.into()),
                "BEHAVIOR_REFERENCE_KIND_INVALID",
                format!("reference '{token}' identifies the wrong Activity semantic kind"),
            )),
        };
    }
    planned_activity_reference(map, planned, token, BehaviorRowKind::StructuredActivityNode)
}

fn planned_activity_reference<T>(
    map: &SpreadsheetImportMap,
    planned: &BehaviorPlanningIndex,
    token: &str,
    expected: BehaviorRowKind,
) -> Result<BuildReference<T>, SpreadsheetImportDiagnostic> {
    if let Some(record) = planned.by_external(token) {
        if record.kind == expected {
            return Ok(BuildReference::External(token.into()));
        }
        return Err(diagnostic(
            Some(map), None, None, None, Some(token.into()),
            "BEHAVIOR_REFERENCE_KIND_INVALID",
            format!("reference '{token}' identifies planned {:?}, not {:?}", record.kind, expected),
        ));
    }
    Err(diagnostic(
        Some(map), None, None, None, Some(token.into()),
        "BEHAVIOR_REFERENCE_UNRESOLVED",
        format!("specialized behavior reference '{token}' must resolve by namespaced External ID"),
    ))
}

fn region_ref(
    map: &SpreadsheetImportMap,
    _activities: &ActivityRepository,
    behavior: &BehaviorRepository,
    planned: &BehaviorPlanningIndex,
    token: &str,
) -> Result<RegionReference, SpreadsheetImportDiagnostic> {
    let key = external_key(&map.source_namespace, token);
    if let Some(identity) = behavior.external_ids.get(&key).copied() {
        return match identity {
            BehaviorSemanticId::Region(id) => Ok(BuildReference::Existing(id)),
            _ => Err(diagnostic(
                Some(map), None, None, None, Some(token.into()),
                "BEHAVIOR_REFERENCE_KIND_INVALID",
                format!("reference '{token}' identifies the wrong State Machine semantic kind"),
            )),
        };
    }
    if let Some(record) = planned.by_external(token) {
        if record.kind == BehaviorRowKind::Region {
            return Ok(BuildReference::External(token.into()));
        }
        return Err(diagnostic(
            Some(map), None, None, None, Some(token.into()),
            "BEHAVIOR_REFERENCE_KIND_INVALID",
            format!(
                "reference '{token}' identifies planned {:?}, not {:?}",
                record.kind,
                BehaviorRowKind::Region,
            ),
        ));
    }
    Err(diagnostic(
        Some(map), None, None, None, Some(token.into()),
        "BEHAVIOR_REFERENCE_UNRESOLVED",
        format!("specialized behavior reference '{token}' must resolve by namespaced External ID"),
    ))
}'''
    text = text[:macro_start] + reference_helpers + text[macro_end:]
    path.write_text(text)
