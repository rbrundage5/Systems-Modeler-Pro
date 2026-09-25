use super::*;
use std::fs;
use systems_modeler_core::behavior::{BehaviorSemanticId, VertexKind};
use systems_modeler_core::{ActivityNodeKind, ActivitySemanticId};

const NS: &str = "catia:pr48";
fn temp_csv(prefix: &str, body: &str) -> String {
    let path = std::env::temp_dir().join(format!("pr48-{prefix}-{}.csv", uuid::Uuid::new_v4()));
    fs::write(&path, body).unwrap();
    path.to_string_lossy().into_owned()
}
fn states(
    name: &str,
) -> (
    WorkspaceState,
    super::super::activity_workspace::ActivityWorkspaceState,
    ElementId,
) {
    let state = WorkspaceState::default();
    let project = Project::new(name);
    let root = project.root_id;
    *state.project.lock().unwrap() = Some(project);
    (
        state,
        super::super::activity_workspace::ActivityWorkspaceState::default(),
        root,
    )
}
fn map(
    name: &str,
    source: String,
    kind: ElementKind,
    root: ElementId,
    cols: &[(&str, SpreadsheetSemanticProperty)],
) -> SpreadsheetImportMap {
    SpreadsheetImportMap {
        name: name.into(),
        source,
        worksheet: None,
        header_row: 1,
        element_kind: kind,
        relationship_kind: None,
        relationship_identity: SpreadsheetRelationshipIdentityPolicy::ExternalId,
        target_scope: root,
        identification_property: SpreadsheetIdentificationProperty::ExternalId,
        search_scope: SpreadsheetSearchScope::TargetRecursive,
        source_namespace: NS.into(),
        mapping_version: "1".into(),
        column_mappings: cols
            .iter()
            .map(|(c, p)| SpreadsheetColumnMapping {
                source_column: (*c).into(),
                property: *p,
            })
            .collect(),
    }
}
fn behavior_map(
    name: &str,
    source: String,
    root: ElementId,
    cols: &[(&str, SpreadsheetSemanticProperty)],
) -> SpreadsheetImportMap {
    map(name, source, ElementKind::Package, root, cols)
}

#[test]
fn pr48_csv_plan_locally_constructs_project_activity_and_state_machine_atomically() {
    let (state, activity, root) = states("PR48 CSV");
    let blocks = map(
        "Blocks",
        temp_csv("blocks", "ID,Name\nSYS,Controller\nDATA,Payload\n"),
        ElementKind::Block,
        root,
        &[
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Name", SpreadsheetSemanticProperty::Name),
        ],
    );
    let signals = map(
        "Signals",
        temp_csv("signals", "ID,Name\nSIG,Ready\n"),
        ElementKind::Signal,
        root,
        &[
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Name", SpreadsheetSemanticProperty::Name),
        ],
    );
    let operations = map(
        "Operations",
        temp_csv("ops", "ID,Name,Owner\nOP,initialize,SYS\n"),
        ElementKind::Operation,
        root,
        &[
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Name", SpreadsheetSemanticProperty::Name),
            ("Owner", SpreadsheetSemanticProperty::Owner),
        ],
    );
    let activities = behavior_map(
        "Activities",
        temp_csv(
            "activities",
            "Kind,ID,Name,Owner,Context\nActivity,ACT,Operate,SYS,SYS\n",
        ),
        root,
        &[
            ("Kind", SpreadsheetSemanticProperty::BehaviorKind),
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Name", SpreadsheetSemanticProperty::Name),
            ("Owner", SpreadsheetSemanticProperty::Owner),
            ("Context", SpreadsheetSemanticProperty::Context),
        ],
    );
    let nodes = behavior_map(
        "Nodes",
        temp_csv(
            "nodes",
            "Kind,ID,Name,Activity,Operation,Signal,Expression\nActivityInitial,N0,Start,ACT,,,\nCallOperationAction,N1,Initialize,ACT,OP,,\nSendSignalAction,N2,Ready,ACT,,SIG,\nAcceptTimeEventAction,N3,Timeout,ACT,,,5s\nActivityFinal,N4,Done,ACT,,,\n",
        ),
        root,
        &[
            ("Kind", SpreadsheetSemanticProperty::BehaviorKind),
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Name", SpreadsheetSemanticProperty::Name),
            ("Activity", SpreadsheetSemanticProperty::Activity),
            ("Operation", SpreadsheetSemanticProperty::Operation),
            ("Signal", SpreadsheetSemanticProperty::Signal),
            ("Expression", SpreadsheetSemanticProperty::Expression),
        ],
    );
    let edges = behavior_map(
        "Edges",
        temp_csv(
            "edges",
            "Kind,ID,Name,Activity,Source,Target,Guard\nControlFlow,E0,,ACT,N0,N1,\nControlFlow,E1,,ACT,N1,N2,\nControlFlow,E2,,ACT,N2,N3,\nControlFlow,E3,,ACT,N3,N4,\n",
        ),
        root,
        &[
            ("Kind", SpreadsheetSemanticProperty::BehaviorKind),
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Name", SpreadsheetSemanticProperty::Name),
            ("Activity", SpreadsheetSemanticProperty::Activity),
            ("Source", SpreadsheetSemanticProperty::Source),
            ("Target", SpreadsheetSemanticProperty::Target),
            ("Guard", SpreadsheetSemanticProperty::Guard),
        ],
    );
    let machines = behavior_map(
        "Machines",
        temp_csv(
            "machines",
            "Kind,ID,Name,Context\nStateMachine,SM,Lifecycle,SYS\n",
        ),
        root,
        &[
            ("Kind", SpreadsheetSemanticProperty::BehaviorKind),
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Name", SpreadsheetSemanticProperty::Name),
            ("Context", SpreadsheetSemanticProperty::Context),
        ],
    );
    let regions = behavior_map(
        "Regions",
        temp_csv(
            "regions",
            "Kind,ID,Name,StateMachine,ParentState\nRegion,R0,main,SM,\n",
        ),
        root,
        &[
            ("Kind", SpreadsheetSemanticProperty::BehaviorKind),
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Name", SpreadsheetSemanticProperty::Name),
            ("StateMachine", SpreadsheetSemanticProperty::StateMachine),
            ("ParentState", SpreadsheetSemanticProperty::ParentState),
        ],
    );
    let vertices = behavior_map(
        "Vertices",
        temp_csv(
            "vertices",
            "Kind,ID,Name,Region\nStateInitial,S0,Initial,R0\nState,S1,Idle,R0\nState,S2,Running,R0\n",
        ),
        root,
        &[
            ("Kind", SpreadsheetSemanticProperty::BehaviorKind),
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Name", SpreadsheetSemanticProperty::Name),
            ("Region", SpreadsheetSemanticProperty::Region),
        ],
    );
    let transitions = behavior_map(
        "Transitions",
        temp_csv(
            "transitions",
            "Kind,ID,Region,Source,Target,Transition Kind,Trigger Kind,Trigger Ref,Guard,Effect\nTransition,T0,R0,S0,S1,External,None,,,\nTransition,T1,R0,S1,S2,External,Signal,SIG,,start\nTransition,T2,R0,S2,S1,External,Call,OP,,stop\n",
        ),
        root,
        &[
            ("Kind", SpreadsheetSemanticProperty::BehaviorKind),
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Region", SpreadsheetSemanticProperty::Region),
            ("Source", SpreadsheetSemanticProperty::Source),
            ("Target", SpreadsheetSemanticProperty::Target),
            (
                "Transition Kind",
                SpreadsheetSemanticProperty::TransitionKind,
            ),
            ("Trigger Kind", SpreadsheetSemanticProperty::TriggerKind),
            ("Trigger Ref", SpreadsheetSemanticProperty::TriggerReference),
            ("Guard", SpreadsheetSemanticProperty::Guard),
            ("Effect", SpreadsheetSemanticProperty::Effect),
        ],
    );
    let group = SpreadsheetImportMapGroup {
        mappings: vec![
            blocks,
            signals,
            operations,
            activities,
            nodes,
            edges,
            machines,
            regions,
            vertices,
            transitions,
        ],
    };
    let preview = preview_spreadsheet_import_group_with_activity(&group, &state, &activity);
    assert!(preview.is_valid(), "{:?}", preview.diagnostics);
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len(),
        1
    );
    assert!(activity.repository.lock().unwrap().activities.is_empty());
    assert!(state.behavior.lock().unwrap().state_machines.is_empty());
    apply_spreadsheet_import_group_with_activity(&group, &state, &activity).unwrap();
    let activities = activity.repository.lock().unwrap();
    assert!(matches!(
        activities.external_ids.get("catia:pr48::N1"),
        Some(ActivitySemanticId::Node(_))
    ));
    let act = activities
        .activities
        .values()
        .find(|a| a.external_id == "catia:pr48::ACT")
        .unwrap();
    assert!(act.nodes.iter().any(|n|matches!(&n.kind,ActivityNodeKind::Action(a) if matches!(a.kind,systems_modeler_core::ActionKind::CallOperation{..}))));
    drop(activities);
    let behavior = state.behavior.lock().unwrap();
    assert!(matches!(
        behavior.external_ids.get("catia:pr48::T1"),
        Some(BehaviorSemanticId::Transition(_))
    ));
    let machine = behavior
        .state_machines
        .values()
        .find(|m| m.external_id == "catia:pr48::SM")
        .unwrap();
    assert!(matches!(
        &machine.regions[0].vertices[1].kind,
        VertexKind::State(_)
    ));
}

#[test]
fn pr48_late_invalid_transition_rolls_back_all_three_semantic_stores() {
    let (state, activity, root) = states("PR48 Rollback");
    let blocks = map(
        "Blocks",
        temp_csv("rb-block", "ID,Name\nSYS,Controller\n"),
        ElementKind::Block,
        root,
        &[
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Name", SpreadsheetSemanticProperty::Name),
        ],
    );
    let act = behavior_map(
        "Activity",
        temp_csv("rb-act", "Kind,ID,Name,Owner\nActivity,ACT,Flow,SYS\n"),
        root,
        &[
            ("Kind", SpreadsheetSemanticProperty::BehaviorKind),
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Name", SpreadsheetSemanticProperty::Name),
            ("Owner", SpreadsheetSemanticProperty::Owner),
        ],
    );
    let sm = behavior_map(
        "SM",
        temp_csv(
            "rb-sm",
            "Kind,ID,Name,Context\nStateMachine,SM,Machine,SYS\n",
        ),
        root,
        &[
            ("Kind", SpreadsheetSemanticProperty::BehaviorKind),
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Name", SpreadsheetSemanticProperty::Name),
            ("Context", SpreadsheetSemanticProperty::Context),
        ],
    );
    let reg = behavior_map(
        "Region",
        temp_csv("rb-reg", "Kind,ID,Name,StateMachine\nRegion,R,main,SM\n"),
        root,
        &[
            ("Kind", SpreadsheetSemanticProperty::BehaviorKind),
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Name", SpreadsheetSemanticProperty::Name),
            ("StateMachine", SpreadsheetSemanticProperty::StateMachine),
        ],
    );
    let v = behavior_map(
        "Vertices",
        temp_csv(
            "rb-v",
            "Kind,ID,Name,Region\nStateInitial,I,Initial,R\nFinalState,F,Final,R\n",
        ),
        root,
        &[
            ("Kind", SpreadsheetSemanticProperty::BehaviorKind),
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Name", SpreadsheetSemanticProperty::Name),
            ("Region", SpreadsheetSemanticProperty::Region),
        ],
    );
    let t = behavior_map(
        "Transition",
        temp_csv(
            "rb-t",
            "Kind,ID,Region,Source,Target,Trigger Kind,Guard\nTransition,T,R,I,F,AnyReceive,bad\n",
        ),
        root,
        &[
            ("Kind", SpreadsheetSemanticProperty::BehaviorKind),
            ("ID", SpreadsheetSemanticProperty::ExternalId),
            ("Region", SpreadsheetSemanticProperty::Region),
            ("Source", SpreadsheetSemanticProperty::Source),
            ("Target", SpreadsheetSemanticProperty::Target),
            ("Trigger Kind", SpreadsheetSemanticProperty::TriggerKind),
            ("Guard", SpreadsheetSemanticProperty::Guard),
        ],
    );
    let group = SpreadsheetImportMapGroup {
        mappings: vec![blocks, act, sm, reg, v, t],
    };
    let preview = preview_spreadsheet_import_group_with_activity(&group, &state, &activity);
    assert!(!preview.is_valid());
    assert!(apply_spreadsheet_import_group_with_activity(&group, &state, &activity).is_err());
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len(),
        1
    );
    assert!(activity.repository.lock().unwrap().activities.is_empty());
    assert!(state.behavior.lock().unwrap().state_machines.is_empty());
}
