#![allow(
    clippy::collapsible_if,
    clippy::redundant_closure,
    clippy::too_many_arguments
)]

use super::*;
use crate::workspace::bulk_model::{
    ActionBuildKind, ActivityBuildOperation, ActivityEndpointReference, ActivityNodeBuildKind,
    ActivityNodeReference, ActivityPartitionReference, ActivityReference, RegionParentReference,
    RegionReference, StateMachineBuildOperation, StateMachineReference, StructuredNodeReference,
    TriggerBuild, VertexBuildKind, VertexReference,
};
use systems_modeler_core::behavior::{
    BehaviorRepository, BehaviorSemanticId, Event, PseudostateKind, Region, RegionId,
    StateMachineId, TransitionId, TransitionKind, VertexId, VertexKind,
};
use systems_modeler_core::{
    ActionKind, ActivityEdgeId, ActivityEdgeKind, ActivityEndpoint, ActivityId, ActivityNodeId,
    ActivityNodeKind, ActivityPartitionId, ActivityRepository, ActivitySemanticId, ObjectNodeKind,
    ObjectNodeOrdering, PinDirection, PinId, StructuredActivityNodeKind, StructuredNodeId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BehaviorRowKind {
    Activity,
    ActivityPartition,
    StructuredActivityNode,
    ActivityInitial,
    ActivityFinal,
    FlowFinal,
    Decision,
    Merge,
    ActivityFork,
    ActivityJoin,
    OpaqueAction,
    CallBehaviorAction,
    CallOperationAction,
    SendSignalAction,
    AcceptEventAction,
    AcceptTimeEventAction,
    ObjectNode,
    CentralBufferNode,
    DataStoreNode,
    ActivityParameterNode,
    Pin,
    ControlFlow,
    ObjectFlow,
    StateMachine,
    Region,
    State,
    FinalState,
    StateInitial,
    Choice,
    Junction,
    StateFork,
    StateJoin,
    ShallowHistory,
    DeepHistory,
    EntryPoint,
    ExitPoint,
    Terminate,
    Transition,
    Interaction,
    Lifeline,
    Occurrence,
    Message,
    ExecutionSpecification,
    CombinedFragment,
    InteractionOperand,
    StateInvariant,
    ParametricElement,
    BindingConnector,
}

#[derive(Debug, Clone)]
pub(super) struct PlannedBehaviorRecord {
    pub(super) external_id: String,
    pub(super) kind: BehaviorRowKind,
    pub(super) name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct BehaviorPlanningIndex {
    records: Vec<PlannedBehaviorRecord>,
}
impl BehaviorPlanningIndex {
    pub(super) fn register(&mut self, record: PlannedBehaviorRecord) {
        self.records.push(record);
    }
    pub(super) fn by_external(&self, external: &str) -> Option<&PlannedBehaviorRecord> {
        self.records
            .iter()
            .find(|record| record.external_id == external)
    }
    pub(super) fn named(&self, kind: BehaviorRowKind, name: &str) -> Vec<&PlannedBehaviorRecord> {
        self.records
            .iter()
            .filter(|record| record.kind == kind && record.name.as_deref() == Some(name))
            .collect()
    }
}

pub(super) struct BehaviorRowPlan {
    pub action: SpreadsheetRowAction,
    pub operations: Vec<ModelBuildOperation>,
    pub planned: Option<PlannedBehaviorRecord>,
}

#[derive(Debug, Clone, Copy)]
enum ExistingTarget {
    Activity(ActivityId),
    Partition(ActivityPartitionId),
    Structured(StructuredNodeId),
    Node(ActivityNodeId),
    Pin(PinId),
    Edge(ActivityEdgeId),
    StateMachine(StateMachineId),
    Region(RegionId),
    Vertex(VertexId),
    Transition(TransitionId),
}
#[derive(Debug, Clone, Copy)]
struct ExistingBehavior {
    kind: BehaviorRowKind,
    target: ExistingTarget,
}

pub(super) fn is_behavior_map(map: &SpreadsheetImportMap) -> bool {
    map.column_mappings
        .iter()
        .any(|mapping| mapping.property == SpreadsheetSemanticProperty::BehaviorKind)
}

pub(super) fn validate_behavior_map(
    map: &SpreadsheetImportMap,
    project: &Project,
) -> Result<(), SpreadsheetImportDiagnostic> {
    if map.source_namespace.trim().is_empty() {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "SOURCE_NAMESPACE_REQUIRED",
            "behavior import source namespace is required",
        ));
    }
    if map.mapping_version.trim().is_empty() {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "MAPPING_VERSION_REQUIRED",
            "mapping version is required",
        ));
    }
    let has = |property| {
        map.column_mappings
            .iter()
            .any(|mapping| mapping.property == property)
    };
    for property in [
        SpreadsheetSemanticProperty::BehaviorKind,
        SpreadsheetSemanticProperty::ExternalId,
    ] {
        if !has(property) {
            return Err(diagnostic(
                Some(map),
                None,
                None,
                Some(property),
                None,
                "BEHAVIOR_COLUMN_REQUIRED",
                format!("PR48 behavior mappings require a mapped {property:?} column"),
            ));
        }
    }
    let target = project.element(map.target_scope).map_err(|_| {
        diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "TARGET_SCOPE_UNRESOLVED",
            format!("target scope {} does not resolve", map.target_scope),
        )
    })?;
    if !target.is_namespace() {
        return Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            None,
            "TARGET_SCOPE_INVALID",
            format!(
                "target '{}' ({:?}) is not a semantic namespace",
                target.name, target.kind
            ),
        ));
    }
    Ok(())
}

fn normalize_kind(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
fn required<'a>(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &'a BTreeMap<SpreadsheetSemanticProperty, String>,
    property: SpreadsheetSemanticProperty,
    label: &str,
) -> Result<&'a str, SpreadsheetImportDiagnostic> {
    non_empty_value(values, property).ok_or_else(|| {
        diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, property),
            Some(property),
            None,
            "BEHAVIOR_FIELD_REQUIRED",
            format!("{label} is required"),
        )
    })
}
fn optional_text(
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    property: SpreadsheetSemanticProperty,
) -> Option<String> {
    values
        .get(&property)
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
}
fn mapped(
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    property: SpreadsheetSemanticProperty,
) -> bool {
    values.contains_key(&property)
}
fn parse_bool(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    property: SpreadsheetSemanticProperty,
    default: bool,
) -> Result<bool, SpreadsheetImportDiagnostic> {
    match values.get(&property).map(|v| v.trim().to_ascii_lowercase()) {
        None => Ok(default),
        Some(v) if v.is_empty() => Ok(default),
        Some(v) if matches!(v.as_str(), "true" | "yes" | "1") => Ok(true),
        Some(v) if matches!(v.as_str(), "false" | "no" | "0") => Ok(false),
        Some(v) => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, property),
            Some(property),
            Some(v),
            "BOOLEAN_INVALID",
            "expected true/false, yes/no, or 1/0",
        )),
    }
}
fn parse_multiplicity_value(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> Result<Multiplicity, SpreadsheetImportDiagnostic> {
    match non_empty_value(values, SpreadsheetSemanticProperty::Multiplicity) {
        None => Ok(Multiplicity::ONE),
        Some(value) => super::super::parametrics::parse_multiplicity(value).map_err(|reason| {
            diagnostic(
                Some(map),
                Some(row),
                mapped_column_name(map, SpreadsheetSemanticProperty::Multiplicity),
                Some(SpreadsheetSemanticProperty::Multiplicity),
                Some(value.to_string()),
                "MULTIPLICITY_INVALID",
                reason,
            )
        }),
    }
}

fn behavior_row_kind(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> Result<BehaviorRowKind, SpreadsheetImportDiagnostic> {
    let raw = required(
        map,
        row,
        values,
        SpreadsheetSemanticProperty::BehaviorKind,
        "Behavior Kind",
    )?;
    let n = normalize_kind(raw);
    let activity_context = non_empty_value(values, SpreadsheetSemanticProperty::Activity).is_some();
    let state_context = non_empty_value(values, SpreadsheetSemanticProperty::Region).is_some()
        || non_empty_value(values, SpreadsheetSemanticProperty::StateMachine).is_some();
    let kind = match n.as_str() {
        "activity" => BehaviorRowKind::Activity,
        "activitypartition" | "partition" => BehaviorRowKind::ActivityPartition,
        "structuredactivitynode" | "structurednode" => BehaviorRowKind::StructuredActivityNode,
        "activityinitial" => BehaviorRowKind::ActivityInitial,
        "initial" if activity_context && !state_context => BehaviorRowKind::ActivityInitial,
        "activityfinal" => BehaviorRowKind::ActivityFinal,
        "flowfinal" => BehaviorRowKind::FlowFinal,
        "decision" => BehaviorRowKind::Decision,
        "merge" => BehaviorRowKind::Merge,
        "activityfork" => BehaviorRowKind::ActivityFork,
        "fork" if activity_context && !state_context => BehaviorRowKind::ActivityFork,
        "activityjoin" => BehaviorRowKind::ActivityJoin,
        "join" if activity_context && !state_context => BehaviorRowKind::ActivityJoin,
        "opaqueaction" | "opaque" => BehaviorRowKind::OpaqueAction,
        "callbehavioraction" | "callbehavior" => BehaviorRowKind::CallBehaviorAction,
        "calloperationaction" | "calloperation" => BehaviorRowKind::CallOperationAction,
        "sendsignalaction" | "sendsignal" => BehaviorRowKind::SendSignalAction,
        "accepteventaction" | "acceptevent" => BehaviorRowKind::AcceptEventAction,
        "accepttimeeventaction" | "accepttimeevent" => BehaviorRowKind::AcceptTimeEventAction,
        "objectnode" => BehaviorRowKind::ObjectNode,
        "centralbuffernode" | "centralbuffer" => BehaviorRowKind::CentralBufferNode,
        "datastorenode" | "datastore" => BehaviorRowKind::DataStoreNode,
        "activityparameternode" => BehaviorRowKind::ActivityParameterNode,
        "pin" | "inputpin" | "outputpin" | "valuepin" => BehaviorRowKind::Pin,
        "controlflow" => BehaviorRowKind::ControlFlow,
        "objectflow" => BehaviorRowKind::ObjectFlow,
        "statemachine" => BehaviorRowKind::StateMachine,
        "region" => BehaviorRowKind::Region,
        "state" => BehaviorRowKind::State,
        "finalstate" => BehaviorRowKind::FinalState,
        "stateinitial" => BehaviorRowKind::StateInitial,
        "initial" if state_context => BehaviorRowKind::StateInitial,
        "choice" => BehaviorRowKind::Choice,
        "junction" => BehaviorRowKind::Junction,
        "statefork" => BehaviorRowKind::StateFork,
        "fork" if state_context => BehaviorRowKind::StateFork,
        "statejoin" => BehaviorRowKind::StateJoin,
        "join" if state_context => BehaviorRowKind::StateJoin,
        "shallowhistory" => BehaviorRowKind::ShallowHistory,
        "deephistory" => BehaviorRowKind::DeepHistory,
        "entrypoint" => BehaviorRowKind::EntryPoint,
        "exitpoint" => BehaviorRowKind::ExitPoint,
        "terminate" => BehaviorRowKind::Terminate,
        "transition" | "externaltransition" | "internaltransition" | "localtransition" => {
            BehaviorRowKind::Transition
        }
        _ => {
            return Err(diagnostic(
                Some(map),
                Some(row),
                mapped_column_name(map, SpreadsheetSemanticProperty::BehaviorKind),
                Some(SpreadsheetSemanticProperty::BehaviorKind),
                Some(raw.to_string()),
                "BEHAVIOR_KIND_UNSUPPORTED",
                format!(
                    "behavior kind '{raw}' is not represented by the native PR48 Activity/State Machine model"
                ),
            ));
        }
    };
    Ok(kind)
}

fn activity_node_row_kind(kind: &ActivityNodeKind) -> BehaviorRowKind {
    match kind {
        ActivityNodeKind::Initial => BehaviorRowKind::ActivityInitial,
        ActivityNodeKind::ActivityFinal => BehaviorRowKind::ActivityFinal,
        ActivityNodeKind::FlowFinal => BehaviorRowKind::FlowFinal,
        ActivityNodeKind::Decision { .. } => BehaviorRowKind::Decision,
        ActivityNodeKind::Merge => BehaviorRowKind::Merge,
        ActivityNodeKind::Fork => BehaviorRowKind::ActivityFork,
        ActivityNodeKind::Join { .. } => BehaviorRowKind::ActivityJoin,
        ActivityNodeKind::Action(action) => match action.kind {
            ActionKind::Opaque { .. } => BehaviorRowKind::OpaqueAction,
            ActionKind::CallBehavior { .. } => BehaviorRowKind::CallBehaviorAction,
            ActionKind::CallOperation { .. } => BehaviorRowKind::CallOperationAction,
            ActionKind::SendSignal { .. } => BehaviorRowKind::SendSignalAction,
            ActionKind::AcceptEvent { .. } => BehaviorRowKind::AcceptEventAction,
            ActionKind::AcceptTimeEvent { .. } => BehaviorRowKind::AcceptTimeEventAction,
        },
        ActivityNodeKind::Object(object) => match object.kind {
            ObjectNodeKind::Object => BehaviorRowKind::ObjectNode,
            ObjectNodeKind::CentralBuffer => BehaviorRowKind::CentralBufferNode,
            ObjectNodeKind::DataStore => BehaviorRowKind::DataStoreNode,
        },
        ActivityNodeKind::ActivityParameter(_) => BehaviorRowKind::ActivityParameterNode,
    }
}
fn vertex_row_kind(kind: &VertexKind) -> BehaviorRowKind {
    match kind {
        VertexKind::State(_) => BehaviorRowKind::State,
        VertexKind::FinalState => BehaviorRowKind::FinalState,
        VertexKind::Pseudostate(kind) => match kind {
            PseudostateKind::Initial => BehaviorRowKind::StateInitial,
            PseudostateKind::Choice => BehaviorRowKind::Choice,
            PseudostateKind::Junction => BehaviorRowKind::Junction,
            PseudostateKind::Fork => BehaviorRowKind::StateFork,
            PseudostateKind::Join => BehaviorRowKind::StateJoin,
            PseudostateKind::ShallowHistory => BehaviorRowKind::ShallowHistory,
            PseudostateKind::DeepHistory => BehaviorRowKind::DeepHistory,
            PseudostateKind::EntryPoint => BehaviorRowKind::EntryPoint,
            PseudostateKind::ExitPoint => BehaviorRowKind::ExitPoint,
            PseudostateKind::Terminate => BehaviorRowKind::Terminate,
        },
    }
}

fn activity_record_for_identity(
    repo: &ActivityRepository,
    id: ActivitySemanticId,
) -> Option<ExistingBehavior> {
    match id {
        ActivitySemanticId::Partition(pid) => repo
            .activities
            .values()
            .find(|a| a.partitions.iter().any(|p| p.id == pid))
            .map(|_| ExistingBehavior {
                kind: BehaviorRowKind::ActivityPartition,
                target: ExistingTarget::Partition(pid),
            }),
        ActivitySemanticId::StructuredNode(sid) => repo
            .activities
            .values()
            .find(|a| a.structured_nodes.iter().any(|s| s.id == sid))
            .map(|_| ExistingBehavior {
                kind: BehaviorRowKind::StructuredActivityNode,
                target: ExistingTarget::Structured(sid),
            }),
        ActivitySemanticId::Node(nid) => repo.activities.values().find_map(|a| {
            a.nodes
                .iter()
                .find(|n| n.id == nid)
                .map(|n| ExistingBehavior {
                    kind: activity_node_row_kind(&n.kind),
                    target: ExistingTarget::Node(nid),
                })
        }),
        ActivitySemanticId::Pin(pid) => repo.activities.values().find_map(|a| {
            a.nodes.iter().find_map(|n| match &n.kind {
                ActivityNodeKind::Action(action) if action.pins.iter().any(|p| p.id == pid) => {
                    Some(ExistingBehavior {
                        kind: BehaviorRowKind::Pin,
                        target: ExistingTarget::Pin(pid),
                    })
                }
                _ => None,
            })
        }),
        ActivitySemanticId::Edge(eid) => repo.activities.values().find_map(|a| {
            a.edges
                .iter()
                .find(|e| e.id == eid)
                .map(|e| ExistingBehavior {
                    kind: if e.kind == ActivityEdgeKind::ControlFlow {
                        BehaviorRowKind::ControlFlow
                    } else {
                        BehaviorRowKind::ObjectFlow
                    },
                    target: ExistingTarget::Edge(eid),
                })
        }),
    }
}
fn find_region(regions: &[Region], id: RegionId) -> Option<&Region> {
    for r in regions {
        if r.id == id {
            return Some(r);
        }
        for v in &r.vertices {
            if let VertexKind::State(s) = &v.kind {
                if let Some(x) = find_region(&s.regions, id) {
                    return Some(x);
                }
            }
        }
    }
    None
}
fn find_vertex(
    regions: &[Region],
    id: VertexId,
) -> Option<&systems_modeler_core::behavior::Vertex> {
    for r in regions {
        for v in &r.vertices {
            if v.id == id {
                return Some(v);
            }
            if let VertexKind::State(s) = &v.kind {
                if let Some(x) = find_vertex(&s.regions, id) {
                    return Some(x);
                }
            }
        }
    }
    None
}
fn find_transition(
    regions: &[Region],
    id: TransitionId,
) -> Option<&systems_modeler_core::behavior::Transition> {
    for r in regions {
        if let Some(t) = r.transitions.iter().find(|t| t.id == id) {
            return Some(t);
        }
        for v in &r.vertices {
            if let VertexKind::State(s) = &v.kind {
                if let Some(x) = find_transition(&s.regions, id) {
                    return Some(x);
                }
            }
        }
    }
    None
}
fn behavior_record_for_identity(
    repo: &BehaviorRepository,
    id: BehaviorSemanticId,
) -> Option<ExistingBehavior> {
    match id {
        BehaviorSemanticId::Region(rid) => repo
            .state_machines
            .values()
            .find(|m| find_region(&m.regions, rid).is_some())
            .map(|_| ExistingBehavior {
                kind: BehaviorRowKind::Region,
                target: ExistingTarget::Region(rid),
            }),
        BehaviorSemanticId::Vertex(vid) => repo.state_machines.values().find_map(|m| {
            find_vertex(&m.regions, vid).map(|v| ExistingBehavior {
                kind: vertex_row_kind(&v.kind),
                target: ExistingTarget::Vertex(vid),
            })
        }),
        BehaviorSemanticId::Transition(tid) => repo
            .state_machines
            .values()
            .find(|m| find_transition(&m.regions, tid).is_some())
            .map(|_| ExistingBehavior {
                kind: BehaviorRowKind::Transition,
                target: ExistingTarget::Transition(tid),
            }),
        BehaviorSemanticId::Lifeline(_)
        | BehaviorSemanticId::Occurrence(_)
        | BehaviorSemanticId::Message(_)
        | BehaviorSemanticId::Execution(_)
        | BehaviorSemanticId::Fragment(_)
        | BehaviorSemanticId::Operand(_)
        | BehaviorSemanticId::Invariant(_) => None,
    }
}

fn existing_by_external(
    map: &SpreadsheetImportMap,
    project: &Project,
    activities: &ActivityRepository,
    behavior: &BehaviorRepository,
    external: &str,
) -> Result<Option<ExistingBehavior>, SpreadsheetImportDiagnostic> {
    let key = external_key(&map.source_namespace, external);
    if project.elements.values().any(|e| e.external_id == key)
        || project.relationships.values().any(|r| r.external_id == key)
    {
        return Err(diagnostic(
            Some(map),
            None,
            mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId),
            Some(external.into()),
            "BEHAVIOR_IDENTITY_KIND_MISMATCH",
            "External ID already identifies an ordinary Project semantic object",
        ));
    }
    if let Some(a) = activities
        .activities
        .values()
        .find(|a| a.external_id == key)
    {
        return Ok(Some(ExistingBehavior {
            kind: BehaviorRowKind::Activity,
            target: ExistingTarget::Activity(a.id),
        }));
    }
    if let Some(id) = activities.external_ids.get(&key).copied() {
        return Ok(activity_record_for_identity(activities, id));
    }
    if let Some(m) = behavior
        .state_machines
        .values()
        .find(|m| m.external_id == key)
    {
        return Ok(Some(ExistingBehavior {
            kind: BehaviorRowKind::StateMachine,
            target: ExistingTarget::StateMachine(m.id),
        }));
    }
    if behavior.interactions.values().any(|i| i.external_id == key) {
        return Err(diagnostic(
            Some(map),
            None,
            mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId),
            Some(external.into()),
            "BEHAVIOR_IDENTITY_KIND_MISMATCH",
            "External ID already identifies a Sequence Interaction",
        ));
    }
    if let Some(id) = behavior.external_ids.get(&key).copied() {
        return Ok(behavior_record_for_identity(behavior, id));
    }
    Ok(None)
}

fn project_ref(
    map: &SpreadsheetImportMap,
    project: &Project,
    planned: &[PlannedElement],
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    property: SpreadsheetSemanticProperty,
    label: &str,
    required_value: bool,
    accept: impl Fn(&ElementKind) -> bool,
) -> Result<Option<ElementReference>, SpreadsheetImportDiagnostic> {
    let value = non_empty_value(values, property);
    if value.is_none() {
        if required_value {
            return Err(diagnostic(
                Some(map),
                None,
                mapped_column_name(map, property),
                Some(property),
                None,
                "BEHAVIOR_REFERENCE_REQUIRED",
                format!("{label} reference is required"),
            ));
        }
        return Ok(None);
    }
    let resolved = resolve_semantic_reference(
        map,
        project,
        planned,
        value.unwrap(),
        property,
        label,
        false,
    )?;
    if !accept(&resolved.kind) {
        return Err(diagnostic(
            Some(map),
            None,
            mapped_column_name(map, property),
            Some(property),
            Some(value.unwrap().into()),
            "BEHAVIOR_REFERENCE_KIND_INVALID",
            format!(
                "{label} resolves to {:?}, which is not a valid semantic kind",
                resolved.kind
            ),
        ));
    }
    Ok(Some(resolved.reference))
}
fn any_project_ref(
    map: &SpreadsheetImportMap,
    project: &Project,
    planned: &[PlannedElement],
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    property: SpreadsheetSemanticProperty,
    label: &str,
    required_value: bool,
) -> Result<Option<ElementReference>, SpreadsheetImportDiagnostic> {
    project_ref(
        map,
        project,
        planned,
        values,
        property,
        label,
        required_value,
        |_| true,
    )
}
fn classifier_ref(
    map: &SpreadsheetImportMap,
    project: &Project,
    planned: &[PlannedElement],
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    property: SpreadsheetSemanticProperty,
    label: &str,
    required_value: bool,
) -> Result<Option<ElementReference>, SpreadsheetImportDiagnostic> {
    project_ref(
        map,
        project,
        planned,
        values,
        property,
        label,
        required_value,
        |kind| classifier_kind(kind),
    )
}
fn exact_project_kind(
    map: &SpreadsheetImportMap,
    project: &Project,
    planned: &[PlannedElement],
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    property: SpreadsheetSemanticProperty,
    label: &str,
    required_value: bool,
    kind: ElementKind,
) -> Result<Option<ElementReference>, SpreadsheetImportDiagnostic> {
    project_ref(
        map,
        project,
        planned,
        values,
        property,
        label,
        required_value,
        |candidate| *candidate == kind,
    )
}

fn activity_ref(
    map: &SpreadsheetImportMap,
    activities: &ActivityRepository,
    planned: &BehaviorPlanningIndex,
    token: &str,
) -> Result<ActivityReference, SpreadsheetImportDiagnostic> {
    let key = external_key(&map.source_namespace, token);
    if let Some(a) = activities
        .activities
        .values()
        .find(|a| a.external_id == key)
    {
        return Ok(BuildReference::Existing(a.id));
    }
    if let Some(p) = planned.by_external(token) {
        if p.kind == BehaviorRowKind::Activity {
            return Ok(BuildReference::External(token.into()));
        }
        return Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            Some(token.into()),
            "BEHAVIOR_REFERENCE_KIND_INVALID",
            "reference identifies a different planned behavior kind",
        ));
    }
    let existing_names = activities
        .activities
        .values()
        .filter(|a| a.name == token)
        .collect::<Vec<_>>();
    let planned_names = planned.named(BehaviorRowKind::Activity, token);
    match (existing_names.as_slice(), planned_names.as_slice()) {
        ([a], []) => Ok(BuildReference::Existing(a.id)),
        ([], [p]) => Ok(BuildReference::External(p.external_id.clone())),
        ([], []) => Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            Some(token.into()),
            "ACTIVITY_REFERENCE_UNRESOLVED",
            format!(
                "Activity '{token}' could not be resolved by namespaced External ID or unique exact name"
            ),
        )),
        _ => Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            Some(token.into()),
            "ACTIVITY_REFERENCE_AMBIGUOUS",
            format!("Activity '{token}' is ambiguous"),
        )),
    }
}
fn state_machine_ref(
    map: &SpreadsheetImportMap,
    behavior: &BehaviorRepository,
    planned: &BehaviorPlanningIndex,
    token: &str,
) -> Result<StateMachineReference, SpreadsheetImportDiagnostic> {
    let key = external_key(&map.source_namespace, token);
    if let Some(m) = behavior
        .state_machines
        .values()
        .find(|m| m.external_id == key)
    {
        return Ok(BuildReference::Existing(m.id));
    }
    if let Some(p) = planned.by_external(token) {
        if p.kind == BehaviorRowKind::StateMachine {
            return Ok(BuildReference::External(token.into()));
        }
        return Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            Some(token.into()),
            "BEHAVIOR_REFERENCE_KIND_INVALID",
            "reference identifies a different planned behavior kind",
        ));
    }
    let existing_names = behavior
        .state_machines
        .values()
        .filter(|m| m.name == token)
        .collect::<Vec<_>>();
    let planned_names = planned.named(BehaviorRowKind::StateMachine, token);
    match (existing_names.as_slice(), planned_names.as_slice()) {
        ([m], []) => Ok(BuildReference::Existing(m.id)),
        ([], [p]) => Ok(BuildReference::External(p.external_id.clone())),
        ([], []) => Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            Some(token.into()),
            "STATE_MACHINE_REFERENCE_UNRESOLVED",
            format!(
                "StateMachine '{token}' could not be resolved by namespaced External ID or unique exact name"
            ),
        )),
        _ => Err(diagnostic(
            Some(map),
            None,
            None,
            None,
            Some(token.into()),
            "STATE_MACHINE_REFERENCE_AMBIGUOUS",
            format!("StateMachine '{token}' is ambiguous"),
        )),
    }
}

fn partition_ref(
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
                Some(map),
                None,
                None,
                None,
                Some(token.into()),
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
                Some(map),
                None,
                None,
                None,
                Some(token.into()),
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
            Some(map),
            None,
            None,
            None,
            Some(token.into()),
            "BEHAVIOR_REFERENCE_KIND_INVALID",
            format!(
                "reference '{token}' identifies planned {:?}, not {:?}",
                record.kind, expected
            ),
        ));
    }
    Err(diagnostic(
        Some(map),
        None,
        None,
        None,
        Some(token.into()),
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
                Some(map),
                None,
                None,
                None,
                Some(token.into()),
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
            Some(map),
            None,
            None,
            None,
            Some(token.into()),
            "BEHAVIOR_REFERENCE_KIND_INVALID",
            format!(
                "reference '{token}' identifies planned {:?}, not {:?}",
                record.kind,
                BehaviorRowKind::Region,
            ),
        ));
    }
    Err(diagnostic(
        Some(map),
        None,
        None,
        None,
        Some(token.into()),
        "BEHAVIOR_REFERENCE_UNRESOLVED",
        format!("specialized behavior reference '{token}' must resolve by namespaced External ID"),
    ))
}

fn is_activity_node_kind(kind: BehaviorRowKind) -> bool {
    matches!(
        kind,
        BehaviorRowKind::ActivityInitial
            | BehaviorRowKind::ActivityFinal
            | BehaviorRowKind::FlowFinal
            | BehaviorRowKind::Decision
            | BehaviorRowKind::Merge
            | BehaviorRowKind::ActivityFork
            | BehaviorRowKind::ActivityJoin
            | BehaviorRowKind::OpaqueAction
            | BehaviorRowKind::CallBehaviorAction
            | BehaviorRowKind::CallOperationAction
            | BehaviorRowKind::SendSignalAction
            | BehaviorRowKind::AcceptEventAction
            | BehaviorRowKind::AcceptTimeEventAction
            | BehaviorRowKind::ObjectNode
            | BehaviorRowKind::CentralBufferNode
            | BehaviorRowKind::DataStoreNode
            | BehaviorRowKind::ActivityParameterNode
    )
}
fn is_vertex_kind(kind: BehaviorRowKind) -> bool {
    matches!(
        kind,
        BehaviorRowKind::State
            | BehaviorRowKind::FinalState
            | BehaviorRowKind::StateInitial
            | BehaviorRowKind::Choice
            | BehaviorRowKind::Junction
            | BehaviorRowKind::StateFork
            | BehaviorRowKind::StateJoin
            | BehaviorRowKind::ShallowHistory
            | BehaviorRowKind::DeepHistory
            | BehaviorRowKind::EntryPoint
            | BehaviorRowKind::ExitPoint
            | BehaviorRowKind::Terminate
    )
}
fn activity_node_ref(
    map: &SpreadsheetImportMap,
    activities: &ActivityRepository,
    planned: &BehaviorPlanningIndex,
    token: &str,
) -> Result<ActivityNodeReference, SpreadsheetImportDiagnostic> {
    let key = external_key(&map.source_namespace, token);
    if let Some(ActivitySemanticId::Node(id)) = activities.external_ids.get(&key).copied() {
        return Ok(BuildReference::Existing(id));
    }
    if let Some(p) = planned.by_external(token) {
        if is_activity_node_kind(p.kind) {
            return Ok(BuildReference::External(token.into()));
        }
    }
    Err(diagnostic(
        Some(map),
        None,
        None,
        None,
        Some(token.into()),
        "ACTIVITY_NODE_REFERENCE_UNRESOLVED",
        format!("ActivityNode '{token}' must resolve by namespaced External ID"),
    ))
}
fn state_vertex_ref(
    map: &SpreadsheetImportMap,
    behavior: &BehaviorRepository,
    planned: &BehaviorPlanningIndex,
    token: &str,
) -> Result<VertexReference, SpreadsheetImportDiagnostic> {
    let key = external_key(&map.source_namespace, token);
    if let Some(BehaviorSemanticId::Vertex(id)) = behavior.external_ids.get(&key).copied() {
        return Ok(BuildReference::Existing(id));
    }
    if let Some(p) = planned.by_external(token) {
        if is_vertex_kind(p.kind) {
            return Ok(BuildReference::External(token.into()));
        }
    }
    Err(diagnostic(
        Some(map),
        None,
        None,
        None,
        Some(token.into()),
        "VERTEX_REFERENCE_UNRESOLVED",
        format!("Vertex '{token}' must resolve by namespaced External ID"),
    ))
}
fn endpoint_ref(
    map: &SpreadsheetImportMap,
    activities: &ActivityRepository,
    planned: &BehaviorPlanningIndex,
    token: &str,
) -> Result<ActivityEndpointReference, SpreadsheetImportDiagnostic> {
    let key = external_key(&map.source_namespace, token);
    match activities.external_ids.get(&key).copied() {
        Some(ActivitySemanticId::Node(id)) => {
            return Ok(ActivityEndpointReference::Node(BuildReference::Existing(
                id,
            )));
        }
        Some(ActivitySemanticId::Pin(id)) => {
            return Ok(ActivityEndpointReference::Pin(BuildReference::Existing(id)));
        }
        Some(_) => {
            return Err(diagnostic(
                Some(map),
                None,
                None,
                None,
                Some(token.into()),
                "ACTIVITY_ENDPOINT_KIND_INVALID",
                "edge endpoint must identify an ActivityNode or Pin",
            ));
        }
        None => {}
    }
    if let Some(p) = planned.by_external(token) {
        if is_activity_node_kind(p.kind) {
            return Ok(ActivityEndpointReference::Node(BuildReference::External(
                token.into(),
            )));
        }
        if p.kind == BehaviorRowKind::Pin {
            return Ok(ActivityEndpointReference::Pin(BuildReference::External(
                token.into(),
            )));
        }
    }
    Err(diagnostic(
        Some(map),
        None,
        None,
        None,
        Some(token.into()),
        "ACTIVITY_ENDPOINT_UNRESOLVED",
        format!("Activity endpoint '{token}' must resolve by namespaced External ID"),
    ))
}

fn ref_element_matches(reference: &ElementReference, current: Option<ElementId>) -> bool {
    matches!(reference,BuildReference::Existing(id) if Some(*id)==current)
}
fn ref_activity_matches(reference: &ActivityReference, current: ActivityId) -> bool {
    matches!(reference,BuildReference::Existing(id) if *id==current)
}
fn ref_nested_matches<T: PartialEq>(reference: &BuildReference<T>, current: T) -> bool {
    matches!(reference,BuildReference::Existing(id) if *id==current)
}

fn parse_structured_kind(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> Result<StructuredActivityNodeKind, SpreadsheetImportDiagnostic> {
    let value = required(
        map,
        row,
        values,
        SpreadsheetSemanticProperty::StructuredKind,
        "Structured Node Kind",
    )?;
    match normalize_kind(value).as_str() {
        "structured" => Ok(StructuredActivityNodeKind::Structured),
        "conditional" => Ok(StructuredActivityNodeKind::Conditional),
        "loop" => Ok(StructuredActivityNodeKind::Loop),
        "sequence" => Ok(StructuredActivityNodeKind::Sequence),
        "expansionregion" => Ok(StructuredActivityNodeKind::ExpansionRegion),
        "interruptibleregion" => Ok(StructuredActivityNodeKind::InterruptibleRegion),
        _ => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::StructuredKind),
            Some(SpreadsheetSemanticProperty::StructuredKind),
            Some(value.into()),
            "STRUCTURED_KIND_INVALID",
            "expected Structured, Conditional, Loop, Sequence, ExpansionRegion, or InterruptibleRegion",
        )),
    }
}
fn parse_ordering(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> Result<ObjectNodeOrdering, SpreadsheetImportDiagnostic> {
    match non_empty_value(values, SpreadsheetSemanticProperty::ObjectOrdering)
        .map(normalize_kind)
        .as_deref()
    {
        None | Some("unordered") => Ok(ObjectNodeOrdering::Unordered),
        Some("ordered") => Ok(ObjectNodeOrdering::Ordered),
        Some("fifo") => Ok(ObjectNodeOrdering::Fifo),
        Some("lifo") => Ok(ObjectNodeOrdering::Lifo),
        Some(value) => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::ObjectOrdering),
            Some(SpreadsheetSemanticProperty::ObjectOrdering),
            Some(value.into()),
            "OBJECT_ORDERING_INVALID",
            "expected Unordered, Ordered, FIFO, or LIFO",
        )),
    }
}
fn parse_pin_direction(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> Result<PinDirection, SpreadsheetImportDiagnostic> {
    let raw_kind = normalize_kind(
        non_empty_value(values, SpreadsheetSemanticProperty::BehaviorKind).unwrap_or(""),
    );
    if raw_kind == "inputpin" {
        return Ok(PinDirection::Input);
    }
    if raw_kind == "outputpin" {
        return Ok(PinDirection::Output);
    }
    if raw_kind == "valuepin" {
        return Ok(PinDirection::Value);
    }
    let value = required(
        map,
        row,
        values,
        SpreadsheetSemanticProperty::PinDirection,
        "Pin Direction",
    )?;
    match normalize_kind(value).as_str() {
        "input" | "in" => Ok(PinDirection::Input),
        "output" | "out" => Ok(PinDirection::Output),
        "value" => Ok(PinDirection::Value),
        _ => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::PinDirection),
            Some(SpreadsheetSemanticProperty::PinDirection),
            Some(value.into()),
            "PIN_DIRECTION_INVALID",
            "expected Input, Output, or Value",
        )),
    }
}
fn parse_transition_kind(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> Result<TransitionKind, SpreadsheetImportDiagnostic> {
    let raw_behavior = normalize_kind(
        non_empty_value(values, SpreadsheetSemanticProperty::BehaviorKind).unwrap_or(""),
    );
    if raw_behavior == "externaltransition" {
        return Ok(TransitionKind::External);
    }
    if raw_behavior == "internaltransition" {
        return Ok(TransitionKind::Internal);
    }
    if raw_behavior == "localtransition" {
        return Ok(TransitionKind::Local);
    }
    match non_empty_value(values, SpreadsheetSemanticProperty::TransitionKind)
        .map(normalize_kind)
        .as_deref()
    {
        None | Some("external") => Ok(TransitionKind::External),
        Some("internal") => Ok(TransitionKind::Internal),
        Some("local") => Ok(TransitionKind::Local),
        Some(value) => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::TransitionKind),
            Some(SpreadsheetSemanticProperty::TransitionKind),
            Some(value.into()),
            "TRANSITION_KIND_INVALID",
            "expected External, Internal, or Local",
        )),
    }
}

fn pseudostate_for(kind: BehaviorRowKind) -> Option<PseudostateKind> {
    Some(match kind {
        BehaviorRowKind::StateInitial => PseudostateKind::Initial,
        BehaviorRowKind::Choice => PseudostateKind::Choice,
        BehaviorRowKind::Junction => PseudostateKind::Junction,
        BehaviorRowKind::StateFork => PseudostateKind::Fork,
        BehaviorRowKind::StateJoin => PseudostateKind::Join,
        BehaviorRowKind::ShallowHistory => PseudostateKind::ShallowHistory,
        BehaviorRowKind::DeepHistory => PseudostateKind::DeepHistory,
        BehaviorRowKind::EntryPoint => PseudostateKind::EntryPoint,
        BehaviorRowKind::ExitPoint => PseudostateKind::ExitPoint,
        BehaviorRowKind::Terminate => PseudostateKind::Terminate,
        _ => return None,
    })
}

fn find_activity_node(
    repo: &ActivityRepository,
    id: ActivityNodeId,
) -> Option<(
    &systems_modeler_core::Activity,
    &systems_modeler_core::ActivityNode,
)> {
    repo.activities
        .values()
        .find_map(|a| a.nodes.iter().find(|n| n.id == id).map(|n| (a, n)))
}
fn find_pin(
    repo: &ActivityRepository,
    id: PinId,
) -> Option<(
    &systems_modeler_core::Activity,
    ActivityNodeId,
    &systems_modeler_core::Pin,
)> {
    repo.activities.values().find_map(|a| {
        a.nodes.iter().find_map(|n| match &n.kind {
            ActivityNodeKind::Action(action) => action
                .pins
                .iter()
                .find(|p| p.id == id)
                .map(|p| (a, n.id, p)),
            _ => None,
        })
    })
}
fn find_edge(
    repo: &ActivityRepository,
    id: ActivityEdgeId,
) -> Option<(
    &systems_modeler_core::Activity,
    &systems_modeler_core::ActivityEdge,
)> {
    repo.activities
        .values()
        .find_map(|a| a.edges.iter().find(|e| e.id == id).map(|e| (a, e)))
}
fn find_partition(
    repo: &ActivityRepository,
    id: ActivityPartitionId,
) -> Option<(
    &systems_modeler_core::Activity,
    &systems_modeler_core::ActivityPartition,
)> {
    repo.activities
        .values()
        .find_map(|a| a.partitions.iter().find(|p| p.id == id).map(|p| (a, p)))
}
fn find_structured(
    repo: &ActivityRepository,
    id: StructuredNodeId,
) -> Option<(
    &systems_modeler_core::Activity,
    &systems_modeler_core::StructuredActivityNode,
)> {
    repo.activities.values().find_map(|a| {
        a.structured_nodes
            .iter()
            .find(|s| s.id == id)
            .map(|s| (a, s))
    })
}
fn find_machine_region(
    repo: &BehaviorRepository,
    id: RegionId,
) -> Option<(StateMachineId, &Region)> {
    repo.state_machines
        .values()
        .find_map(|m| find_region(&m.regions, id).map(|r| (m.id, r)))
}
fn find_machine_vertex(
    repo: &BehaviorRepository,
    id: VertexId,
) -> Option<(StateMachineId, &systems_modeler_core::behavior::Vertex)> {
    repo.state_machines
        .values()
        .find_map(|m| find_vertex(&m.regions, id).map(|v| (m.id, v)))
}
fn find_machine_transition(
    repo: &BehaviorRepository,
    id: TransitionId,
) -> Option<(StateMachineId, &systems_modeler_core::behavior::Transition)> {
    repo.state_machines
        .values()
        .find_map(|m| find_transition(&m.regions, id).map(|t| (m.id, t)))
}

fn trigger_build(
    map: &SpreadsheetImportMap,
    project: &Project,
    planned_project: &[PlannedElement],
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> Result<Option<TriggerBuild>, SpreadsheetImportDiagnostic> {
    let Some(kind) = non_empty_value(values, SpreadsheetSemanticProperty::TriggerKind) else {
        return Ok(None);
    };
    match normalize_kind(kind).as_str() {
        "signal" => Ok(Some(TriggerBuild::Signal(
            exact_project_kind(
                map,
                project,
                planned_project,
                values,
                SpreadsheetSemanticProperty::TriggerReference,
                "Signal trigger",
                true,
                ElementKind::Signal,
            )?
            .unwrap(),
        ))),
        "call" => Ok(Some(TriggerBuild::Call(
            exact_project_kind(
                map,
                project,
                planned_project,
                values,
                SpreadsheetSemanticProperty::TriggerReference,
                "Call trigger",
                true,
                ElementKind::Operation,
            )?
            .unwrap(),
        ))),
        "time" => {
            let expression = required(
                map,
                0,
                values,
                SpreadsheetSemanticProperty::TriggerExpression,
                "Time trigger expression",
            )?
            .to_string();
            let is_relative = parse_bool(
                map,
                0,
                values,
                SpreadsheetSemanticProperty::TriggerRelative,
                true,
            )?;
            Ok(Some(TriggerBuild::Time {
                expression,
                is_relative,
            }))
        }
        "change" => Ok(Some(TriggerBuild::Change {
            expression: required(
                map,
                0,
                values,
                SpreadsheetSemanticProperty::TriggerExpression,
                "Change trigger expression",
            )?
            .to_string(),
        })),
        "anyreceive" => Ok(Some(TriggerBuild::AnyReceive)),
        "none" | "" => Ok(None),
        _ => Err(diagnostic(
            Some(map),
            None,
            mapped_column_name(map, SpreadsheetSemanticProperty::TriggerKind),
            Some(SpreadsheetSemanticProperty::TriggerKind),
            Some(kind.into()),
            "TRIGGER_KIND_INVALID",
            "expected Signal, Call, Time, Change, AnyReceive, or None",
        )),
    }
}

fn event_matches_build(event: &Event, spec: &TriggerBuild) -> bool {
    match (event, spec) {
        (Event::Signal { signal_id }, TriggerBuild::Signal(BuildReference::Existing(id))) => {
            signal_id == id
        }
        (Event::Call { operation_id }, TriggerBuild::Call(BuildReference::Existing(id))) => {
            operation_id == id
        }
        (
            Event::Time {
                expression,
                is_relative,
            },
            TriggerBuild::Time {
                expression: e,
                is_relative: r,
            },
        ) => expression == e && is_relative == r,
        (Event::Change { expression }, TriggerBuild::Change { expression: e }) => expression == e,
        (Event::AnyReceive, TriggerBuild::AnyReceive) => true,
        _ => false,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn plan_behavior_row(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    project: &Project,
    activities: &ActivityRepository,
    behavior: &BehaviorRepository,
    planned_project: &[PlannedElement],
    planned: &BehaviorPlanningIndex,
    seen: &mut HashSet<String>,
) -> Result<BehaviorRowPlan, SpreadsheetImportDiagnostic> {
    if is_pr49_semantic_row(values) {
        return plan_pr49_semantic_row(
            map,
            row,
            values,
            project,
            behavior,
            planned_project,
            planned,
            seen,
        );
    }
    let kind = behavior_row_kind(map, row, values)?;
    let external = required(
        map,
        row,
        values,
        SpreadsheetSemanticProperty::ExternalId,
        "External ID",
    )?
    .to_string();
    let key = external_key(&map.source_namespace, &external);
    if !seen.insert(key.clone()) {
        return Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId),
            Some(external),
            "DUPLICATE_SOURCE_EXTERNAL_ID",
            format!("source external ID '{key}' appears more than once in this import group"),
        ));
    }
    if let Some(p) = planned.by_external(&external) {
        return Err(diagnostic(
            Some(map),
            Some(row),
            None,
            None,
            Some(external),
            "DUPLICATE_SOURCE_EXTERNAL_ID",
            format!("source external identity already planned as {:?}", p.kind),
        ));
    }
    let existing = existing_by_external(map, project, activities, behavior, &external)?;
    if let Some(existing) = existing {
        if existing.kind != kind {
            return Err(diagnostic(
                Some(map),
                Some(row),
                mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
                Some(SpreadsheetSemanticProperty::ExternalId),
                Some(external),
                "BEHAVIOR_IDENTITY_KIND_MISMATCH",
                format!("External ID identifies {:?}, not {:?}", existing.kind, kind),
            ));
        }
    }
    let name = optional_text(values, SpreadsheetSemanticProperty::Name);
    let create_name = || {
        name.clone().ok_or_else(|| {
            diagnostic(
                Some(map),
                Some(row),
                mapped_column_name(map, SpreadsheetSemanticProperty::Name),
                Some(SpreadsheetSemanticProperty::Name),
                Some(external.clone()),
                "BEHAVIOR_NAME_REQUIRED",
                format!("new {kind:?} rows require a non-empty Name"),
            )
        })
    };
    let mut operations = Vec::new();
    let mut changed = false;
    match kind {
        BehaviorRowKind::Activity => {
            let owner = any_project_ref(
                map,
                project,
                planned_project,
                values,
                SpreadsheetSemanticProperty::Owner,
                "Activity owner",
                existing.is_none(),
            )?;
            let context = classifier_ref(
                map,
                project,
                planned_project,
                values,
                SpreadsheetSemanticProperty::Context,
                "Activity context",
                false,
            )?;
            match existing {
                None => operations.push(ModelBuildOperation::Activity {
                    operation: ActivityBuildOperation::CreateActivity {
                        external_id: external.clone(),
                        name: create_name()?,
                        owner: owner.unwrap(),
                        context,
                    },
                }),
                Some(ExistingBehavior {
                    target: ExistingTarget::Activity(id),
                    ..
                }) => {
                    let current = activities.activities.get(&id).unwrap();
                    changed = name.as_ref().is_some_and(|v| current.name != *v)
                        || owner
                            .as_ref()
                            .is_some_and(|r| !ref_element_matches(r, Some(current.owner_id)))
                        || (mapped(values, SpreadsheetSemanticProperty::Context)
                            && match &context {
                                Some(r) => !ref_element_matches(r, current.context_id),
                                None => current.context_id.is_some(),
                            });
                    if changed {
                        operations.push(ModelBuildOperation::Activity {
                            operation: ActivityBuildOperation::UpdateActivity {
                                activity: BuildReference::Existing(id),
                                name: name.clone(),
                                owner,
                                context: mapped(values, SpreadsheetSemanticProperty::Context)
                                    .then_some(context),
                            },
                        })
                    }
                }
                _ => unreachable!(),
            }
        }
        BehaviorRowKind::ActivityPartition => {
            let activity = activity_ref(
                map,
                activities,
                planned,
                required(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Activity,
                    "Activity",
                )?,
            )?;
            let represented = any_project_ref(
                map,
                project,
                planned_project,
                values,
                SpreadsheetSemanticProperty::RepresentedElement,
                "represented element",
                false,
            )?;
            let dim = parse_bool(
                map,
                row,
                values,
                SpreadsheetSemanticProperty::IsDimension,
                false,
            )?;
            let ext = parse_bool(
                map,
                row,
                values,
                SpreadsheetSemanticProperty::IsExternal,
                false,
            )?;
            match existing {
                None => operations.push(ModelBuildOperation::Activity {
                    operation: ActivityBuildOperation::CreatePartition {
                        external_id: external.clone(),
                        activity,
                        name: create_name()?,
                        represented_element: represented,
                        is_dimension: dim,
                        is_external: ext,
                    },
                }),
                Some(ExistingBehavior {
                    target: ExistingTarget::Partition(id),
                    ..
                }) => {
                    let (a, p) = find_partition(activities, id).unwrap();
                    if !ref_activity_matches(&activity, a.id) {
                        return Err(diagnostic(
                            Some(map),
                            Some(row),
                            None,
                            None,
                            Some(external),
                            "BEHAVIOR_REPARENT_UNSUPPORTED",
                            "ActivityPartition cannot be moved to another Activity during reimport",
                        ));
                    }
                    changed = name.as_ref().is_some_and(|v| p.name != *v)
                        || (mapped(values, SpreadsheetSemanticProperty::RepresentedElement)
                            && match &represented {
                                Some(r) => !ref_element_matches(r, p.represented_element_id),
                                None => p.represented_element_id.is_some(),
                            })
                        || (mapped(values, SpreadsheetSemanticProperty::IsDimension)
                            && p.is_dimension != dim)
                        || (mapped(values, SpreadsheetSemanticProperty::IsExternal)
                            && p.is_external != ext);
                    if changed {
                        operations.push(ModelBuildOperation::Activity {
                            operation: ActivityBuildOperation::UpdatePartition {
                                partition: BuildReference::Existing(id),
                                name: name.clone(),
                                represented_element: mapped(
                                    values,
                                    SpreadsheetSemanticProperty::RepresentedElement,
                                )
                                .then_some(represented),
                                is_dimension: mapped(
                                    values,
                                    SpreadsheetSemanticProperty::IsDimension,
                                )
                                .then_some(dim),
                                is_external: mapped(
                                    values,
                                    SpreadsheetSemanticProperty::IsExternal,
                                )
                                .then_some(ext),
                            },
                        })
                    }
                }
                _ => unreachable!(),
            }
        }
        BehaviorRowKind::StructuredActivityNode => {
            let activity = activity_ref(
                map,
                activities,
                planned,
                required(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Activity,
                    "Activity",
                )?,
            )?;
            let skind = parse_structured_kind(map, row, values)?;
            let parent = non_empty_value(values, SpreadsheetSemanticProperty::ParentStructuredNode)
                .map(|v| structured_ref(map, activities, behavior, planned, v))
                .transpose()?;
            match existing {
                None => operations.push(ModelBuildOperation::Activity {
                    operation: ActivityBuildOperation::CreateStructuredNode {
                        external_id: external.clone(),
                        activity,
                        name: create_name()?,
                        kind: skind,
                        parent,
                    },
                }),
                Some(ExistingBehavior {
                    target: ExistingTarget::Structured(id),
                    ..
                }) => {
                    let (a, s) = find_structured(activities, id).unwrap();
                    if !ref_activity_matches(&activity, a.id) {
                        return Err(diagnostic(
                            Some(map),
                            Some(row),
                            None,
                            None,
                            Some(external),
                            "BEHAVIOR_REPARENT_UNSUPPORTED",
                            "StructuredActivityNode cannot move to another Activity",
                        ));
                    }
                    changed = name.as_ref().is_some_and(|v| s.name != *v)
                        || s.kind != skind
                        || (mapped(values, SpreadsheetSemanticProperty::ParentStructuredNode)
                            && match &parent {
                                Some(r) => !ref_nested_matches(r, s.parent_id.unwrap_or(id)),
                                None => s.parent_id.is_some(),
                            });
                    if changed {
                        operations.push(ModelBuildOperation::Activity {
                            operation: ActivityBuildOperation::UpdateStructuredNode {
                                node: BuildReference::Existing(id),
                                name: name.clone(),
                                kind: Some(skind),
                                parent: mapped(
                                    values,
                                    SpreadsheetSemanticProperty::ParentStructuredNode,
                                )
                                .then_some(parent),
                            },
                        })
                    }
                }
                _ => unreachable!(),
            }
        }
        k if is_activity_node_kind(k) => {
            let activity = activity_ref(
                map,
                activities,
                planned,
                required(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Activity,
                    "Activity",
                )?,
            )?;
            let partition = non_empty_value(values, SpreadsheetSemanticProperty::Partition)
                .map(|v| partition_ref(map, activities, behavior, planned, v))
                .transpose()?;
            let structured = non_empty_value(values, SpreadsheetSemanticProperty::StructuredNode)
                .map(|v| structured_ref(map, activities, behavior, planned, v))
                .transpose()?;
            let build_kind = activity_node_build_kind(
                map,
                row,
                values,
                k,
                project,
                activities,
                behavior,
                planned_project,
                planned,
            )?;
            match existing {
                None => operations.push(ModelBuildOperation::Activity {
                    operation: ActivityBuildOperation::CreateNode {
                        external_id: external.clone(),
                        activity,
                        name: create_name()?,
                        kind: build_kind,
                        partition,
                        structured_node: structured,
                    },
                }),
                Some(ExistingBehavior {
                    target: ExistingTarget::Node(id),
                    ..
                }) => {
                    let (a, n) = find_activity_node(activities, id).unwrap();
                    if !ref_activity_matches(&activity, a.id) {
                        return Err(diagnostic(
                            Some(map),
                            Some(row),
                            None,
                            None,
                            Some(external),
                            "BEHAVIOR_REPARENT_UNSUPPORTED",
                            "ActivityNode cannot move to another Activity",
                        ));
                    }
                    changed =
                        node_fields_changed(n, &name, &build_kind, &partition, &structured, values);
                    if changed {
                        operations.push(ModelBuildOperation::Activity {
                            operation: ActivityBuildOperation::UpdateNode {
                                node: BuildReference::Existing(id),
                                name: name.clone(),
                                kind: node_kind_fields_mapped(k, values).then_some(build_kind),
                                partition: mapped(values, SpreadsheetSemanticProperty::Partition)
                                    .then_some(partition),
                                structured_node: mapped(
                                    values,
                                    SpreadsheetSemanticProperty::StructuredNode,
                                )
                                .then_some(structured),
                            },
                        })
                    }
                }
                _ => unreachable!(),
            }
        }
        BehaviorRowKind::Pin => {
            let owner = node_ref_any(
                map,
                activities,
                planned,
                required(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::OwnerAction,
                    "Owner Action",
                )?,
            )?;
            let direction = parse_pin_direction(map, row, values)?;
            let type_ref = classifier_ref(
                map,
                project,
                planned_project,
                values,
                SpreadsheetSemanticProperty::Type,
                "Pin type",
                false,
            )?;
            let multiplicity = parse_multiplicity_value(map, row, values)?;
            let ordered = parse_bool(
                map,
                row,
                values,
                SpreadsheetSemanticProperty::Ordered,
                false,
            )?;
            let unique = parse_bool(map, row, values, SpreadsheetSemanticProperty::Unique, true)?;
            let value = optional_text(values, SpreadsheetSemanticProperty::Value);
            let parameter = exact_project_kind(
                map,
                project,
                planned_project,
                values,
                SpreadsheetSemanticProperty::Parameter,
                "Pin Parameter",
                false,
                ElementKind::Parameter,
            )?;
            match existing {
                None => operations.push(ModelBuildOperation::Activity {
                    operation: ActivityBuildOperation::CreatePin {
                        external_id: external.clone(),
                        owner_action: owner,
                        name: create_name()?,
                        direction,
                        type_ref,
                        multiplicity,
                        is_ordered: ordered,
                        is_unique: unique,
                        value,
                        parameter,
                    },
                }),
                Some(ExistingBehavior {
                    target: ExistingTarget::Pin(id),
                    ..
                }) => {
                    let (_, owner_id, p) = find_pin(activities, id).unwrap();
                    if !ref_nested_matches(&owner, owner_id) {
                        return Err(diagnostic(
                            Some(map),
                            Some(row),
                            None,
                            None,
                            Some(external),
                            "BEHAVIOR_REPARENT_UNSUPPORTED",
                            "Pin cannot move to another Action during reimport",
                        ));
                    }
                    changed = name.as_ref().is_some_and(|v| p.name != *v)
                        || (mapped(values, SpreadsheetSemanticProperty::PinDirection)
                            && p.direction != direction)
                        || (mapped(values, SpreadsheetSemanticProperty::Type)
                            && match &type_ref {
                                Some(r) => !ref_element_matches(r, p.type_id),
                                None => p.type_id.is_some(),
                            })
                        || (mapped(values, SpreadsheetSemanticProperty::Multiplicity)
                            && p.multiplicity != multiplicity)
                        || (mapped(values, SpreadsheetSemanticProperty::Ordered)
                            && p.is_ordered != ordered)
                        || (mapped(values, SpreadsheetSemanticProperty::Unique)
                            && p.is_unique != unique)
                        || (mapped(values, SpreadsheetSemanticProperty::Value) && p.value != value)
                        || (mapped(values, SpreadsheetSemanticProperty::Parameter)
                            && match &parameter {
                                Some(r) => !ref_element_matches(r, p.parameter_id),
                                None => p.parameter_id.is_some(),
                            });
                    if changed {
                        operations.push(ModelBuildOperation::Activity {
                            operation: ActivityBuildOperation::UpdatePin {
                                pin: BuildReference::Existing(id),
                                name: name.clone(),
                                direction: mapped(
                                    values,
                                    SpreadsheetSemanticProperty::PinDirection,
                                )
                                .then_some(direction),
                                type_ref: mapped(values, SpreadsheetSemanticProperty::Type)
                                    .then_some(type_ref),
                                multiplicity: mapped(
                                    values,
                                    SpreadsheetSemanticProperty::Multiplicity,
                                )
                                .then_some(multiplicity),
                                is_ordered: mapped(values, SpreadsheetSemanticProperty::Ordered)
                                    .then_some(ordered),
                                is_unique: mapped(values, SpreadsheetSemanticProperty::Unique)
                                    .then_some(unique),
                                value: mapped(values, SpreadsheetSemanticProperty::Value)
                                    .then_some(value),
                                parameter: mapped(values, SpreadsheetSemanticProperty::Parameter)
                                    .then_some(parameter),
                            },
                        })
                    }
                }
                _ => unreachable!(),
            }
        }
        BehaviorRowKind::ControlFlow | BehaviorRowKind::ObjectFlow => {
            let activity = activity_ref(
                map,
                activities,
                planned,
                required(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Activity,
                    "Activity",
                )?,
            )?;
            let source = endpoint_ref(
                map,
                activities,
                planned,
                required(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Source,
                    "Source endpoint",
                )?,
            )?;
            let target = endpoint_ref(
                map,
                activities,
                planned,
                required(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Target,
                    "Target endpoint",
                )?,
            )?;
            let edge_kind = if kind == BehaviorRowKind::ControlFlow {
                ActivityEdgeKind::ControlFlow
            } else {
                ActivityEdgeKind::ObjectFlow
            };
            let guard = optional_text(values, SpreadsheetSemanticProperty::Guard);
            let weight = optional_text(values, SpreadsheetSemanticProperty::Weight);
            let selection = optional_text(values, SpreadsheetSemanticProperty::Selection);
            let transformation = optional_text(values, SpreadsheetSemanticProperty::Transformation);
            let interrupt =
                non_empty_value(values, SpreadsheetSemanticProperty::InterruptingRegion)
                    .map(|v| structured_ref(map, activities, behavior, planned, v))
                    .transpose()?;
            match existing {
                None => operations.push(ModelBuildOperation::Activity {
                    operation: ActivityBuildOperation::CreateEdge {
                        external_id: external.clone(),
                        activity,
                        name: name.clone().unwrap_or_default(),
                        kind: edge_kind,
                        source,
                        target,
                        guard,
                        weight,
                        selection,
                        transformation,
                        interrupting_region: interrupt,
                    },
                }),
                Some(ExistingBehavior {
                    target: ExistingTarget::Edge(id),
                    ..
                }) => {
                    let (a, e) = find_edge(activities, id).unwrap();
                    if !ref_activity_matches(&activity, a.id) {
                        return Err(diagnostic(
                            Some(map),
                            Some(row),
                            None,
                            None,
                            Some(external),
                            "BEHAVIOR_REPARENT_UNSUPPORTED",
                            "ActivityEdge cannot move to another Activity",
                        ));
                    }
                    changed = name.as_ref().is_some_and(|v| e.name != *v)
                        || e.kind != edge_kind
                        || !endpoint_matches(&source, e.source)
                        || !endpoint_matches(&target, e.target)
                        || (mapped(values, SpreadsheetSemanticProperty::Guard) && e.guard != guard)
                        || (mapped(values, SpreadsheetSemanticProperty::Weight)
                            && e.weight != weight)
                        || (mapped(values, SpreadsheetSemanticProperty::Selection)
                            && e.selection != selection)
                        || (mapped(values, SpreadsheetSemanticProperty::Transformation)
                            && e.transformation != transformation)
                        || (mapped(values, SpreadsheetSemanticProperty::InterruptingRegion)
                            && match &interrupt {
                                Some(r) => !ref_nested_matches(
                                    r,
                                    e.interrupting_region_id
                                        .unwrap_or(id_to_structured_sentinel()),
                                ),
                                None => e.interrupting_region_id.is_some(),
                            });
                    if changed {
                        operations.push(ModelBuildOperation::Activity {
                            operation: ActivityBuildOperation::UpdateEdge {
                                edge: BuildReference::Existing(id),
                                name: name.clone(),
                                kind: Some(edge_kind),
                                source: Some(source),
                                target: Some(target),
                                guard: mapped(values, SpreadsheetSemanticProperty::Guard)
                                    .then_some(guard),
                                weight: mapped(values, SpreadsheetSemanticProperty::Weight)
                                    .then_some(weight),
                                selection: mapped(values, SpreadsheetSemanticProperty::Selection)
                                    .then_some(selection),
                                transformation: mapped(
                                    values,
                                    SpreadsheetSemanticProperty::Transformation,
                                )
                                .then_some(transformation),
                                interrupting_region: mapped(
                                    values,
                                    SpreadsheetSemanticProperty::InterruptingRegion,
                                )
                                .then_some(interrupt),
                            },
                        })
                    }
                }
                _ => unreachable!(),
            }
        }
        BehaviorRowKind::StateMachine => {
            let context = classifier_ref(
                map,
                project,
                planned_project,
                values,
                SpreadsheetSemanticProperty::Context,
                "StateMachine context",
                existing.is_none(),
            )?;
            match existing {
                None => operations.push(ModelBuildOperation::StateMachine {
                    operation: StateMachineBuildOperation::CreateStateMachine {
                        external_id: external.clone(),
                        name: create_name()?,
                        context: context.unwrap(),
                    },
                }),
                Some(ExistingBehavior {
                    target: ExistingTarget::StateMachine(id),
                    ..
                }) => {
                    let m = behavior.state_machines.get(&id).unwrap();
                    changed = name.as_ref().is_some_and(|v| m.name != *v)
                        || context
                            .as_ref()
                            .is_some_and(|r| !ref_element_matches(r, Some(m.context_id)));
                    if changed {
                        operations.push(ModelBuildOperation::StateMachine {
                            operation: StateMachineBuildOperation::UpdateStateMachine {
                                state_machine: BuildReference::Existing(id),
                                name: name.clone(),
                                context,
                            },
                        })
                    }
                }
                _ => unreachable!(),
            }
        }
        BehaviorRowKind::Region => {
            let machine = state_machine_ref(
                map,
                behavior,
                planned,
                required(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::StateMachine,
                    "StateMachine",
                )?,
            )?;
            let parent_state = non_empty_value(values, SpreadsheetSemanticProperty::ParentState)
                .map(|v| state_vertex_ref(map, behavior, planned, v))
                .transpose()?;
            match existing {
                None => operations.push(ModelBuildOperation::StateMachine {
                    operation: StateMachineBuildOperation::CreateRegion {
                        external_id: external.clone(),
                        parent: match parent_state {
                            Some(state) => RegionParentReference::State(state),
                            None => RegionParentReference::StateMachine(machine),
                        },
                        name: create_name()?,
                    },
                }),
                Some(ExistingBehavior {
                    target: ExistingTarget::Region(id),
                    ..
                }) => {
                    let (mid, r) = find_machine_region(behavior, id).unwrap();
                    if !ref_nested_matches(&machine, mid) || parent_state.is_some() { /* nested existing parent is checked conservatively below */
                    }
                    let actual_parent = region_parent_vertex(
                        &behavior.state_machines.get(&mid).unwrap().regions,
                        id,
                    );
                    match (&parent_state, actual_parent) {
                        (None, None) => {}
                        (Some(reference), Some(vertex))
                            if ref_nested_matches(reference, vertex) => {}
                        _ => {
                            return Err(diagnostic(
                                Some(map),
                                Some(row),
                                None,
                                None,
                                Some(external),
                                "BEHAVIOR_REPARENT_UNSUPPORTED",
                                "Region parent cannot change during reimport",
                            ));
                        }
                    }
                    changed = name.as_ref().is_some_and(|v| r.name != *v);
                    if changed {
                        operations.push(ModelBuildOperation::StateMachine {
                            operation: StateMachineBuildOperation::UpdateRegion {
                                region: BuildReference::Existing(id),
                                name: name.clone(),
                            },
                        })
                    }
                }
                _ => unreachable!(),
            }
        }
        k if is_vertex_kind(k) => {
            let region = region_ref(
                map,
                activities,
                behavior,
                planned,
                required(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Region,
                    "Region",
                )?,
            )?;
            let build_kind = vertex_build_kind(map, row, values, k, behavior, planned)?;
            match existing {
                None => operations.push(ModelBuildOperation::StateMachine {
                    operation: StateMachineBuildOperation::CreateVertex {
                        external_id: external.clone(),
                        region,
                        name: create_name()?,
                        kind: build_kind,
                    },
                }),
                Some(ExistingBehavior {
                    target: ExistingTarget::Vertex(id),
                    ..
                }) => {
                    let (mid, v) = find_machine_vertex(behavior, id).unwrap();
                    let actual_region = vertex_parent_region(
                        &behavior.state_machines.get(&mid).unwrap().regions,
                        id,
                    )
                    .unwrap();
                    if !ref_nested_matches(&region, actual_region) {
                        return Err(diagnostic(
                            Some(map),
                            Some(row),
                            None,
                            None,
                            Some(external),
                            "BEHAVIOR_REPARENT_UNSUPPORTED",
                            "Vertex cannot move to another Region during reimport",
                        ));
                    }
                    changed = vertex_fields_changed(v, &name, &build_kind, values);
                    if changed {
                        operations.push(ModelBuildOperation::StateMachine {
                            operation: StateMachineBuildOperation::UpdateVertex {
                                vertex: BuildReference::Existing(id),
                                name: name.clone(),
                                kind: vertex_kind_fields_mapped(k, values).then_some(build_kind),
                            },
                        })
                    }
                }
                _ => unreachable!(),
            }
        }
        BehaviorRowKind::Transition => {
            let region = region_ref(
                map,
                activities,
                behavior,
                planned,
                required(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Region,
                    "Region",
                )?,
            )?;
            let source = state_vertex_ref(
                map,
                behavior,
                planned,
                required(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Source,
                    "Transition source",
                )?,
            )?;
            let target = state_vertex_ref(
                map,
                behavior,
                planned,
                required(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Target,
                    "Transition target",
                )?,
            )?;
            let tkind = parse_transition_kind(map, row, values)?;
            let trigger = trigger_build(map, project, planned_project, values)?;
            let guard = optional_text(values, SpreadsheetSemanticProperty::Guard);
            let effect = optional_text(values, SpreadsheetSemanticProperty::Effect);
            match existing {
                None => operations.push(ModelBuildOperation::StateMachine {
                    operation: StateMachineBuildOperation::CreateTransition {
                        external_id: external.clone(),
                        region,
                        source,
                        target,
                        kind: tkind,
                        trigger,
                        guard,
                        effect,
                    },
                }),
                Some(ExistingBehavior {
                    target: ExistingTarget::Transition(id),
                    ..
                }) => {
                    let (mid, t) = find_machine_transition(behavior, id).unwrap();
                    let actual_region = transition_parent_region(
                        &behavior.state_machines.get(&mid).unwrap().regions,
                        id,
                    )
                    .unwrap();
                    if !ref_nested_matches(&region, actual_region) {
                        return Err(diagnostic(
                            Some(map),
                            Some(row),
                            None,
                            None,
                            Some(external),
                            "BEHAVIOR_REPARENT_UNSUPPORTED",
                            "Transition cannot move to another Region during reimport",
                        ));
                    }
                    changed = !ref_nested_matches(&source, t.source_id)
                        || !ref_nested_matches(&target, t.target_id)
                        || t.kind != tkind
                        || (mapped(values, SpreadsheetSemanticProperty::TriggerKind)
                            && match (&t.trigger, &trigger) {
                                (None, None) => false,
                                (Some(existing), Some(spec)) => {
                                    !event_matches_build(&existing.event, spec)
                                }
                                _ => true,
                            })
                        || (mapped(values, SpreadsheetSemanticProperty::Guard) && t.guard != guard)
                        || (mapped(values, SpreadsheetSemanticProperty::Effect)
                            && t.effect != effect);
                    if changed {
                        operations.push(ModelBuildOperation::StateMachine {
                            operation: StateMachineBuildOperation::UpdateTransition {
                                transition: BuildReference::Existing(id),
                                source: Some(source),
                                target: Some(target),
                                kind: Some(tkind),
                                trigger: mapped(values, SpreadsheetSemanticProperty::TriggerKind)
                                    .then_some(trigger),
                                guard: mapped(values, SpreadsheetSemanticProperty::Guard)
                                    .then_some(guard),
                                effect: mapped(values, SpreadsheetSemanticProperty::Effect)
                                    .then_some(effect),
                            },
                        })
                    }
                }
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }
    let action = if existing.is_none() {
        SpreadsheetRowAction::Create
    } else if changed {
        SpreadsheetRowAction::Update
    } else {
        SpreadsheetRowAction::NoChange
    };
    let planned_record = (existing.is_none()).then(|| PlannedBehaviorRecord {
        external_id: external.clone(),
        kind,
        name: name.clone(),
    });
    Ok(BehaviorRowPlan {
        action,
        operations,
        planned: planned_record,
    })
}

fn node_ref_any(
    map: &SpreadsheetImportMap,
    activities: &ActivityRepository,
    planned: &BehaviorPlanningIndex,
    token: &str,
) -> Result<ActivityNodeReference, SpreadsheetImportDiagnostic> {
    activity_node_ref(map, activities, planned, token)
}
fn endpoint_matches(reference: &ActivityEndpointReference, current: ActivityEndpoint) -> bool {
    match (reference, current) {
        (ActivityEndpointReference::Node(r), ActivityEndpoint::Node(id)) => {
            ref_nested_matches(r, id)
        }
        (ActivityEndpointReference::Pin(r), ActivityEndpoint::Pin(id)) => ref_nested_matches(r, id),
        _ => false,
    }
}
fn id_to_structured_sentinel() -> StructuredNodeId {
    StructuredNodeId(uuid::Uuid::nil())
}

#[allow(clippy::too_many_arguments)]
fn activity_node_build_kind(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    kind: BehaviorRowKind,
    project: &Project,
    activities: &ActivityRepository,
    _behavior: &BehaviorRepository,
    planned_project: &[PlannedElement],
    planned: &BehaviorPlanningIndex,
) -> Result<ActivityNodeBuildKind, SpreadsheetImportDiagnostic> {
    Ok(match kind {
        BehaviorRowKind::ActivityInitial => ActivityNodeBuildKind::Initial,
        BehaviorRowKind::ActivityFinal => ActivityNodeBuildKind::ActivityFinal,
        BehaviorRowKind::FlowFinal => ActivityNodeBuildKind::FlowFinal,
        BehaviorRowKind::Decision => ActivityNodeBuildKind::Decision {
            decision_input: optional_text(values, SpreadsheetSemanticProperty::DecisionInput),
        },
        BehaviorRowKind::Merge => ActivityNodeBuildKind::Merge,
        BehaviorRowKind::ActivityFork => ActivityNodeBuildKind::Fork,
        BehaviorRowKind::ActivityJoin => ActivityNodeBuildKind::Join {
            join_specification: optional_text(
                values,
                SpreadsheetSemanticProperty::JoinSpecification,
            ),
        },
        BehaviorRowKind::OpaqueAction => ActivityNodeBuildKind::Action(ActionBuildKind::Opaque {
            body: values
                .get(&SpreadsheetSemanticProperty::Body)
                .cloned()
                .unwrap_or_default(),
        }),
        BehaviorRowKind::CallBehaviorAction => {
            ActivityNodeBuildKind::Action(ActionBuildKind::CallBehavior {
                activity: activity_ref(
                    map,
                    activities,
                    planned,
                    required(
                        map,
                        row,
                        values,
                        SpreadsheetSemanticProperty::CalledActivity,
                        "Called Activity",
                    )?,
                )?,
            })
        }
        BehaviorRowKind::CallOperationAction => {
            ActivityNodeBuildKind::Action(ActionBuildKind::CallOperation {
                operation: exact_project_kind(
                    map,
                    project,
                    planned_project,
                    values,
                    SpreadsheetSemanticProperty::Operation,
                    "Called Operation",
                    true,
                    ElementKind::Operation,
                )?
                .unwrap(),
            })
        }
        BehaviorRowKind::SendSignalAction => {
            ActivityNodeBuildKind::Action(ActionBuildKind::SendSignal {
                signal: exact_project_kind(
                    map,
                    project,
                    planned_project,
                    values,
                    SpreadsheetSemanticProperty::Signal,
                    "Sent Signal",
                    true,
                    ElementKind::Signal,
                )?
                .unwrap(),
            })
        }
        BehaviorRowKind::AcceptEventAction => {
            ActivityNodeBuildKind::Action(ActionBuildKind::AcceptEvent {
                signal: exact_project_kind(
                    map,
                    project,
                    planned_project,
                    values,
                    SpreadsheetSemanticProperty::Signal,
                    "Accepted Signal",
                    false,
                    ElementKind::Signal,
                )?,
            })
        }
        BehaviorRowKind::AcceptTimeEventAction => {
            ActivityNodeBuildKind::Action(ActionBuildKind::AcceptTimeEvent {
                expression: required(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Expression,
                    "Time expression",
                )?
                .into(),
            })
        }
        BehaviorRowKind::ObjectNode
        | BehaviorRowKind::CentralBufferNode
        | BehaviorRowKind::DataStoreNode => ActivityNodeBuildKind::Object {
            kind: match kind {
                BehaviorRowKind::ObjectNode => ObjectNodeKind::Object,
                BehaviorRowKind::CentralBufferNode => ObjectNodeKind::CentralBuffer,
                _ => ObjectNodeKind::DataStore,
            },
            type_ref: classifier_ref(
                map,
                project,
                planned_project,
                values,
                SpreadsheetSemanticProperty::Type,
                "Object node type",
                false,
            )?,
            multiplicity: parse_multiplicity_value(map, row, values)?,
            ordering: parse_ordering(map, row, values)?,
            selection: optional_text(values, SpreadsheetSemanticProperty::Selection),
        },
        BehaviorRowKind::ActivityParameterNode => ActivityNodeBuildKind::ActivityParameter {
            parameter: exact_project_kind(
                map,
                project,
                planned_project,
                values,
                SpreadsheetSemanticProperty::Parameter,
                "Activity Parameter",
                true,
                ElementKind::Parameter,
            )?
            .unwrap(),
        },
        _ => unreachable!(),
    })
}
fn node_kind_fields_mapped(
    kind: BehaviorRowKind,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> bool {
    match kind {
        BehaviorRowKind::Decision => mapped(values, SpreadsheetSemanticProperty::DecisionInput),
        BehaviorRowKind::ActivityJoin => {
            mapped(values, SpreadsheetSemanticProperty::JoinSpecification)
        }
        BehaviorRowKind::OpaqueAction => mapped(values, SpreadsheetSemanticProperty::Body),
        BehaviorRowKind::CallBehaviorAction => {
            mapped(values, SpreadsheetSemanticProperty::CalledActivity)
        }
        BehaviorRowKind::CallOperationAction => {
            mapped(values, SpreadsheetSemanticProperty::Operation)
        }
        BehaviorRowKind::SendSignalAction | BehaviorRowKind::AcceptEventAction => {
            mapped(values, SpreadsheetSemanticProperty::Signal)
        }
        BehaviorRowKind::AcceptTimeEventAction => {
            mapped(values, SpreadsheetSemanticProperty::Expression)
        }
        BehaviorRowKind::ObjectNode
        | BehaviorRowKind::CentralBufferNode
        | BehaviorRowKind::DataStoreNode => [
            SpreadsheetSemanticProperty::Type,
            SpreadsheetSemanticProperty::Multiplicity,
            SpreadsheetSemanticProperty::ObjectOrdering,
            SpreadsheetSemanticProperty::Selection,
        ]
        .into_iter()
        .any(|p| mapped(values, p)),
        BehaviorRowKind::ActivityParameterNode => {
            mapped(values, SpreadsheetSemanticProperty::Parameter)
        }
        _ => false,
    }
}
fn node_fields_changed(
    node: &systems_modeler_core::ActivityNode,
    name: &Option<String>,
    spec: &ActivityNodeBuildKind,
    partition: &Option<ActivityPartitionReference>,
    structured: &Option<StructuredNodeReference>,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> bool {
    name.as_ref().is_some_and(|v| node.name != *v)
        || (mapped(values, SpreadsheetSemanticProperty::Partition)
            && match partition {
                Some(r) => !ref_nested_matches(
                    r,
                    node.partition_id
                        .unwrap_or(ActivityPartitionId(uuid::Uuid::nil())),
                ),
                None => node.partition_id.is_some(),
            })
        || (mapped(values, SpreadsheetSemanticProperty::StructuredNode)
            && match structured {
                Some(r) => !ref_nested_matches(
                    r,
                    node.structured_node_id
                        .unwrap_or(StructuredNodeId(uuid::Uuid::nil())),
                ),
                None => node.structured_node_id.is_some(),
            })
        || (node_kind_fields_mapped(activity_node_row_kind(&node.kind), values)
            && !native_node_kind_matches(&node.kind, spec))
}
fn native_node_kind_matches(native: &ActivityNodeKind, spec: &ActivityNodeBuildKind) -> bool {
    match (native, spec) {
        (
            ActivityNodeKind::Decision { decision_input },
            ActivityNodeBuildKind::Decision { decision_input: d },
        ) => decision_input == d,
        (
            ActivityNodeKind::Join { join_specification },
            ActivityNodeBuildKind::Join {
                join_specification: j,
            },
        ) => join_specification == j,
        (ActivityNodeKind::Action(a), ActivityNodeBuildKind::Action(s)) => match (&a.kind, s) {
            (ActionKind::Opaque { body }, ActionBuildKind::Opaque { body: b }) => body == b,
            (
                ActionKind::CallBehavior { activity_id },
                ActionBuildKind::CallBehavior { activity },
            ) => ref_activity_matches(activity, *activity_id),
            (
                ActionKind::CallOperation { operation_id },
                ActionBuildKind::CallOperation { operation },
            ) => ref_element_matches(operation, Some(*operation_id)),
            (ActionKind::SendSignal { signal_id }, ActionBuildKind::SendSignal { signal }) => {
                ref_element_matches(signal, Some(*signal_id))
            }
            (ActionKind::AcceptEvent { signal_id }, ActionBuildKind::AcceptEvent { signal }) => {
                match (signal, signal_id) {
                    (None, None) => true,
                    (Some(r), Some(id)) => ref_element_matches(r, Some(*id)),
                    _ => false,
                }
            }
            (
                ActionKind::AcceptTimeEvent { expression },
                ActionBuildKind::AcceptTimeEvent { expression: e },
            ) => expression == e,
            _ => false,
        },
        (
            ActivityNodeKind::Object(o),
            ActivityNodeBuildKind::Object {
                kind,
                type_ref,
                multiplicity,
                ordering,
                selection,
            },
        ) => {
            o.kind == *kind
                && o.multiplicity == *multiplicity
                && o.ordering == *ordering
                && o.selection == *selection
                && match (type_ref, o.type_id) {
                    (None, None) => true,
                    (Some(r), id) => ref_element_matches(r, id),
                    _ => false,
                }
        }
        (
            ActivityNodeKind::ActivityParameter(p),
            ActivityNodeBuildKind::ActivityParameter { parameter },
        ) => ref_element_matches(parameter, Some(p.parameter_id)),
        _ => Trueish::yes(native, spec),
    }
}
struct Trueish;
impl Trueish {
    fn yes(native: &ActivityNodeKind, spec: &ActivityNodeBuildKind) -> bool {
        matches!(
            (native, spec),
            (ActivityNodeKind::Initial, ActivityNodeBuildKind::Initial)
                | (
                    ActivityNodeKind::ActivityFinal,
                    ActivityNodeBuildKind::ActivityFinal
                )
                | (
                    ActivityNodeKind::FlowFinal,
                    ActivityNodeBuildKind::FlowFinal
                )
                | (ActivityNodeKind::Merge, ActivityNodeBuildKind::Merge)
                | (ActivityNodeKind::Fork, ActivityNodeBuildKind::Fork)
        )
    }
}

fn vertex_build_kind(
    map: &SpreadsheetImportMap,
    _row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    kind: BehaviorRowKind,
    behavior: &BehaviorRepository,
    planned: &BehaviorPlanningIndex,
) -> Result<VertexBuildKind, SpreadsheetImportDiagnostic> {
    Ok(match kind {
        BehaviorRowKind::State => VertexBuildKind::State {
            entry: optional_text(values, SpreadsheetSemanticProperty::Entry),
            do_activity: optional_text(values, SpreadsheetSemanticProperty::DoActivity),
            exit: optional_text(values, SpreadsheetSemanticProperty::Exit),
            submachine: non_empty_value(values, SpreadsheetSemanticProperty::Submachine)
                .map(|v| state_machine_ref(map, behavior, planned, v))
                .transpose()?,
        },
        BehaviorRowKind::FinalState => VertexBuildKind::FinalState,
        k => VertexBuildKind::Pseudostate(pseudostate_for(k).unwrap()),
    })
}
fn vertex_kind_fields_mapped(
    kind: BehaviorRowKind,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> bool {
    kind == BehaviorRowKind::State
        && [
            SpreadsheetSemanticProperty::Entry,
            SpreadsheetSemanticProperty::DoActivity,
            SpreadsheetSemanticProperty::Exit,
            SpreadsheetSemanticProperty::Submachine,
        ]
        .into_iter()
        .any(|p| mapped(values, p))
}
fn vertex_fields_changed(
    vertex: &systems_modeler_core::behavior::Vertex,
    name: &Option<String>,
    spec: &VertexBuildKind,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
) -> bool {
    name.as_ref().is_some_and(|v| vertex.name != *v)
        || (vertex_kind_fields_mapped(vertex_row_kind(&vertex.kind), values)
            && match (&vertex.kind, spec) {
                (
                    VertexKind::State(s),
                    VertexBuildKind::State {
                        entry,
                        do_activity,
                        exit,
                        submachine,
                    },
                ) => {
                    s.entry != *entry
                        || s.do_activity != *do_activity
                        || s.exit != *exit
                        || match (submachine, s.submachine) {
                            (None, None) => false,
                            (Some(r), Some(id)) => !ref_nested_matches(r, id),
                            _ => true,
                        }
                }
                _ => false,
            })
}
fn region_parent_vertex(regions: &[Region], target: RegionId) -> Option<VertexId> {
    for r in regions {
        for v in &r.vertices {
            if let VertexKind::State(s) = &v.kind {
                if s.regions.iter().any(|child| child.id == target) {
                    return Some(v.id);
                }
                if let Some(found) = region_parent_vertex(&s.regions, target) {
                    return Some(found);
                }
            }
        }
    }
    None
}
fn vertex_parent_region(regions: &[Region], target: VertexId) -> Option<RegionId> {
    for r in regions {
        if r.vertices.iter().any(|v| v.id == target) {
            return Some(r.id);
        }
        for v in &r.vertices {
            if let VertexKind::State(s) = &v.kind {
                if let Some(found) = vertex_parent_region(&s.regions, target) {
                    return Some(found);
                }
            }
        }
    }
    None
}
fn transition_parent_region(regions: &[Region], target: TransitionId) -> Option<RegionId> {
    for r in regions {
        if r.transitions.iter().any(|t| t.id == target) {
            return Some(r.id);
        }
        for v in &r.vertices {
            if let VertexKind::State(s) = &v.kind {
                if let Some(found) = transition_parent_region(&s.regions, target) {
                    return Some(found);
                }
            }
        }
    }
    None
}
