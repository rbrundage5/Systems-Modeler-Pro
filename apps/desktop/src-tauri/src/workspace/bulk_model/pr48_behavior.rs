#![allow(clippy::collapsible_if)]

use super::*;
use systems_modeler_core::behavior::{
    BehaviorRepository, BehaviorSemanticId, Event, PseudostateKind, Region, RegionId, State,
    StateMachineId, Transition, TransitionId, TransitionKind, Trigger, Vertex, VertexId,
    VertexKind,
};
use systems_modeler_core::{
    Action, ActionKind, Activity, ActivityEdge, ActivityEdgeId, ActivityEdgeKind, ActivityEndpoint,
    ActivityId, ActivityNode, ActivityNodeId, ActivityNodeKind, ActivityParameterNode,
    ActivityPartition, ActivityPartitionId, ActivityRepository, ActivitySemanticId, ElementId,
    Multiplicity, ObjectNode, ObjectNodeKind, ObjectNodeOrdering, Pin, PinDirection, PinId,
    Project, StructuredActivityNode, StructuredActivityNodeKind, StructuredNodeId,
};

pub type ActivityReference = BuildReference<ActivityId>;
pub type ActivityNodeReference = BuildReference<ActivityNodeId>;
pub type ActivityEdgeReference = BuildReference<ActivityEdgeId>;
pub type PinReference = BuildReference<PinId>;
pub type ActivityPartitionReference = BuildReference<ActivityPartitionId>;
pub type StructuredNodeReference = BuildReference<StructuredNodeId>;
pub type StateMachineReference = BuildReference<StateMachineId>;
pub type RegionReference = BuildReference<RegionId>;
pub type VertexReference = BuildReference<VertexId>;
pub type TransitionReference = BuildReference<TransitionId>;

#[derive(Debug, Clone)]
pub enum ActionBuildKind {
    Opaque { body: String },
    CallBehavior { activity: ActivityReference },
    CallOperation { operation: ElementReference },
    SendSignal { signal: ElementReference },
    AcceptEvent { signal: Option<ElementReference> },
    AcceptTimeEvent { expression: String },
}

#[derive(Debug, Clone)]
pub enum ActivityNodeBuildKind {
    Initial,
    ActivityFinal,
    FlowFinal,
    Decision {
        decision_input: Option<String>,
    },
    Merge,
    Fork,
    Join {
        join_specification: Option<String>,
    },
    Action(ActionBuildKind),
    Object {
        kind: ObjectNodeKind,
        type_ref: Option<ElementReference>,
        multiplicity: Multiplicity,
        ordering: ObjectNodeOrdering,
        selection: Option<String>,
    },
    ActivityParameter {
        parameter: ElementReference,
    },
}

#[derive(Debug, Clone)]
pub enum ActivityEndpointReference {
    Node(ActivityNodeReference),
    Pin(PinReference),
}

#[derive(Debug, Clone)]
pub enum ActivityBuildOperation {
    CreateActivity {
        external_id: String,
        name: String,
        owner: ElementReference,
        context: Option<ElementReference>,
    },
    UpdateActivity {
        activity: ActivityReference,
        name: Option<String>,
        owner: Option<ElementReference>,
        context: Option<Option<ElementReference>>,
    },
    CreatePartition {
        external_id: String,
        activity: ActivityReference,
        name: String,
        represented_element: Option<ElementReference>,
        is_dimension: bool,
        is_external: bool,
    },
    UpdatePartition {
        partition: ActivityPartitionReference,
        name: Option<String>,
        represented_element: Option<Option<ElementReference>>,
        is_dimension: Option<bool>,
        is_external: Option<bool>,
    },
    CreateStructuredNode {
        external_id: String,
        activity: ActivityReference,
        name: String,
        kind: StructuredActivityNodeKind,
        parent: Option<StructuredNodeReference>,
    },
    UpdateStructuredNode {
        node: StructuredNodeReference,
        name: Option<String>,
        kind: Option<StructuredActivityNodeKind>,
        parent: Option<Option<StructuredNodeReference>>,
    },
    CreateNode {
        external_id: String,
        activity: ActivityReference,
        name: String,
        kind: ActivityNodeBuildKind,
        partition: Option<ActivityPartitionReference>,
        structured_node: Option<StructuredNodeReference>,
    },
    UpdateNode {
        node: ActivityNodeReference,
        name: Option<String>,
        kind: Option<ActivityNodeBuildKind>,
        partition: Option<Option<ActivityPartitionReference>>,
        structured_node: Option<Option<StructuredNodeReference>>,
    },
    CreatePin {
        external_id: String,
        owner_action: ActivityNodeReference,
        name: String,
        direction: PinDirection,
        type_ref: Option<ElementReference>,
        multiplicity: Multiplicity,
        is_ordered: bool,
        is_unique: bool,
        value: Option<String>,
        parameter: Option<ElementReference>,
    },
    UpdatePin {
        pin: PinReference,
        name: Option<String>,
        direction: Option<PinDirection>,
        type_ref: Option<Option<ElementReference>>,
        multiplicity: Option<Multiplicity>,
        is_ordered: Option<bool>,
        is_unique: Option<bool>,
        value: Option<Option<String>>,
        parameter: Option<Option<ElementReference>>,
    },
    CreateEdge {
        external_id: String,
        activity: ActivityReference,
        name: String,
        kind: ActivityEdgeKind,
        source: ActivityEndpointReference,
        target: ActivityEndpointReference,
        guard: Option<String>,
        weight: Option<String>,
        selection: Option<String>,
        transformation: Option<String>,
        interrupting_region: Option<StructuredNodeReference>,
    },
    UpdateEdge {
        edge: ActivityEdgeReference,
        name: Option<String>,
        kind: Option<ActivityEdgeKind>,
        source: Option<ActivityEndpointReference>,
        target: Option<ActivityEndpointReference>,
        guard: Option<Option<String>>,
        weight: Option<Option<String>>,
        selection: Option<Option<String>>,
        transformation: Option<Option<String>>,
        interrupting_region: Option<Option<StructuredNodeReference>>,
    },
}

#[derive(Debug, Clone)]
pub enum RegionParentReference {
    StateMachine(StateMachineReference),
    State(VertexReference),
}

#[derive(Debug, Clone)]
pub enum VertexBuildKind {
    State {
        entry: Option<String>,
        do_activity: Option<String>,
        exit: Option<String>,
        submachine: Option<StateMachineReference>,
    },
    FinalState,
    Pseudostate(PseudostateKind),
}

#[derive(Debug, Clone)]
pub enum TriggerBuild {
    Signal(ElementReference),
    Call(ElementReference),
    Time {
        expression: String,
        is_relative: bool,
    },
    Change {
        expression: String,
    },
    AnyReceive,
}

#[derive(Debug, Clone)]
pub enum StateMachineBuildOperation {
    CreateStateMachine {
        external_id: String,
        name: String,
        context: ElementReference,
    },
    UpdateStateMachine {
        state_machine: StateMachineReference,
        name: Option<String>,
        context: Option<ElementReference>,
    },
    CreateRegion {
        external_id: String,
        parent: RegionParentReference,
        name: String,
    },
    UpdateRegion {
        region: RegionReference,
        name: Option<String>,
    },
    CreateVertex {
        external_id: String,
        region: RegionReference,
        name: String,
        kind: VertexBuildKind,
    },
    UpdateVertex {
        vertex: VertexReference,
        name: Option<String>,
        kind: Option<VertexBuildKind>,
    },
    CreateTransition {
        external_id: String,
        region: RegionReference,
        source: VertexReference,
        target: VertexReference,
        kind: TransitionKind,
        trigger: Option<TriggerBuild>,
        guard: Option<String>,
        effect: Option<String>,
    },
    UpdateTransition {
        transition: TransitionReference,
        source: Option<VertexReference>,
        target: Option<VertexReference>,
        kind: Option<TransitionKind>,
        trigger: Option<Option<TriggerBuild>>,
        guard: Option<Option<String>>,
        effect: Option<Option<String>>,
    },
}

pub(super) fn activity_create_external_id(operation: &ActivityBuildOperation) -> Option<&String> {
    match operation {
        ActivityBuildOperation::CreateActivity { external_id, .. }
        | ActivityBuildOperation::CreatePartition { external_id, .. }
        | ActivityBuildOperation::CreateStructuredNode { external_id, .. }
        | ActivityBuildOperation::CreateNode { external_id, .. }
        | ActivityBuildOperation::CreatePin { external_id, .. }
        | ActivityBuildOperation::CreateEdge { external_id, .. } => Some(external_id),
        _ => None,
    }
}

pub(super) fn state_machine_create_external_id(
    operation: &StateMachineBuildOperation,
) -> Option<&String> {
    match operation {
        StateMachineBuildOperation::CreateStateMachine { external_id, .. }
        | StateMachineBuildOperation::CreateRegion { external_id, .. }
        | StateMachineBuildOperation::CreateVertex { external_id, .. }
        | StateMachineBuildOperation::CreateTransition { external_id, .. } => Some(external_id),
        _ => None,
    }
}

pub(super) fn behavior_operation_description(operation: &ActivityBuildOperation) -> String {
    match operation {
        ActivityBuildOperation::CreateActivity { external_id, .. } => {
            format!("CREATE Activity {external_id}")
        }
        ActivityBuildOperation::CreatePartition { external_id, .. } => {
            format!("CREATE ActivityPartition {external_id}")
        }
        ActivityBuildOperation::CreateStructuredNode { external_id, .. } => {
            format!("CREATE StructuredActivityNode {external_id}")
        }
        ActivityBuildOperation::CreateNode { external_id, .. } => {
            format!("CREATE ActivityNode {external_id}")
        }
        ActivityBuildOperation::CreatePin { external_id, .. } => {
            format!("CREATE Pin {external_id}")
        }
        ActivityBuildOperation::CreateEdge { external_id, .. } => {
            format!("CREATE ActivityEdge {external_id}")
        }
        ActivityBuildOperation::UpdateActivity { .. } => "UPDATE Activity".into(),
        ActivityBuildOperation::UpdatePartition { .. } => "UPDATE ActivityPartition".into(),
        ActivityBuildOperation::UpdateStructuredNode { .. } => {
            "UPDATE StructuredActivityNode".into()
        }
        ActivityBuildOperation::UpdateNode { .. } => "UPDATE ActivityNode".into(),
        ActivityBuildOperation::UpdatePin { .. } => "UPDATE Pin".into(),
        ActivityBuildOperation::UpdateEdge { .. } => "UPDATE ActivityEdge".into(),
    }
}

pub(super) fn state_machine_operation_description(
    operation: &StateMachineBuildOperation,
) -> String {
    match operation {
        StateMachineBuildOperation::CreateStateMachine { external_id, .. } => {
            format!("CREATE StateMachine {external_id}")
        }
        StateMachineBuildOperation::CreateRegion { external_id, .. } => {
            format!("CREATE Region {external_id}")
        }
        StateMachineBuildOperation::CreateVertex { external_id, .. } => {
            format!("CREATE Vertex {external_id}")
        }
        StateMachineBuildOperation::CreateTransition { external_id, .. } => {
            format!("CREATE Transition {external_id}")
        }
        StateMachineBuildOperation::UpdateStateMachine { .. } => "UPDATE StateMachine".into(),
        StateMachineBuildOperation::UpdateRegion { .. } => "UPDATE Region".into(),
        StateMachineBuildOperation::UpdateVertex { .. } => "UPDATE Vertex".into(),
        StateMachineBuildOperation::UpdateTransition { .. } => "UPDATE Transition".into(),
    }
}

fn activity_identity_key(repository: &ActivityRepository, id: ActivitySemanticId) -> Option<&str> {
    repository
        .external_ids
        .iter()
        .find_map(|(key, candidate)| (*candidate == id).then_some(key.as_str()))
}

fn behavior_identity_key(repository: &BehaviorRepository, id: BehaviorSemanticId) -> Option<&str> {
    repository
        .external_ids
        .iter()
        .find_map(|(key, candidate)| (*candidate == id).then_some(key.as_str()))
}

fn ensure_external_available(
    key: &str,
    project: &Project,
    activities: &ActivityRepository,
    behavior: &BehaviorRepository,
    operation: usize,
) -> Result<(), BuildDiagnostic> {
    let project_collision = project
        .elements
        .values()
        .any(|element| element.external_id == key)
        || project
            .relationships
            .values()
            .any(|relationship| relationship.external_id == key);
    let activity_collision = activities
        .activities
        .values()
        .any(|activity| activity.external_id == key)
        || activities.external_ids.contains_key(key);
    let behavior_collision = behavior
        .state_machines
        .values()
        .any(|machine| machine.external_id == key)
        || behavior
            .interactions
            .values()
            .any(|interaction| interaction.external_id == key)
        || behavior.external_ids.contains_key(key);
    if project_collision || activity_collision || behavior_collision {
        return Err(error(
            "DUPLICATE_EXTERNAL_ID",
            Some(operation),
            format!("external ID already exists across authored semantic stores: {key}"),
        ));
    }
    Ok(())
}

fn resolve_activity(
    repository: &ActivityRepository,
    namespace: &str,
    reference: &ActivityReference,
    operation: usize,
) -> Result<ActivityId, BuildDiagnostic> {
    match reference {
        BuildReference::Existing(id) if repository.activities.contains_key(id) => Ok(*id),
        BuildReference::Existing(id) => Err(error(
            "UNRESOLVED_ACTIVITY_REFERENCE",
            Some(operation),
            format!("Activity {id} was not found"),
        )),
        BuildReference::External(external) => {
            let key = external_key(namespace, external);
            let matches = repository
                .activities
                .values()
                .filter(|activity| activity.external_id == key)
                .map(|activity| activity.id)
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [id] => Ok(*id),
                [] => Err(error(
                    "UNRESOLVED_ACTIVITY_REFERENCE",
                    Some(operation),
                    format!("Activity external ID '{external}' was not found"),
                )),
                _ => Err(error(
                    "AMBIGUOUS_ACTIVITY_REFERENCE",
                    Some(operation),
                    format!("Activity external ID '{external}' is ambiguous"),
                )),
            }
        }
    }
}

fn resolve_activity_semantic<T: Copy>(
    repository: &ActivityRepository,
    namespace: &str,
    reference: &BuildReference<T>,
    operation: usize,
    expected: fn(ActivitySemanticId) -> Option<T>,
    label: &str,
) -> Result<T, BuildDiagnostic> {
    match reference {
        BuildReference::Existing(id) => Ok(*id),
        BuildReference::External(external) => {
            let key = external_key(namespace, external);
            match repository
                .external_ids
                .get(&key)
                .copied()
                .and_then(expected)
            {
                Some(id) => Ok(id),
                None => Err(error(
                    "UNRESOLVED_BEHAVIOR_REFERENCE",
                    Some(operation),
                    format!(
                        "{label} external ID '{external}' was not found with the required semantic kind"
                    ),
                )),
            }
        }
    }
}

fn resolve_node(
    repository: &ActivityRepository,
    namespace: &str,
    reference: &ActivityNodeReference,
    operation: usize,
) -> Result<ActivityNodeId, BuildDiagnostic> {
    resolve_activity_semantic(
        repository,
        namespace,
        reference,
        operation,
        |id| match id {
            ActivitySemanticId::Node(value) => Some(value),
            _ => None,
        },
        "ActivityNode",
    )
}
fn resolve_pin(
    repository: &ActivityRepository,
    namespace: &str,
    reference: &PinReference,
    operation: usize,
) -> Result<PinId, BuildDiagnostic> {
    resolve_activity_semantic(
        repository,
        namespace,
        reference,
        operation,
        |id| match id {
            ActivitySemanticId::Pin(value) => Some(value),
            _ => None,
        },
        "Pin",
    )
}
fn resolve_edge(
    repository: &ActivityRepository,
    namespace: &str,
    reference: &ActivityEdgeReference,
    operation: usize,
) -> Result<ActivityEdgeId, BuildDiagnostic> {
    resolve_activity_semantic(
        repository,
        namespace,
        reference,
        operation,
        |id| match id {
            ActivitySemanticId::Edge(value) => Some(value),
            _ => None,
        },
        "ActivityEdge",
    )
}
fn resolve_partition(
    repository: &ActivityRepository,
    namespace: &str,
    reference: &ActivityPartitionReference,
    operation: usize,
) -> Result<ActivityPartitionId, BuildDiagnostic> {
    resolve_activity_semantic(
        repository,
        namespace,
        reference,
        operation,
        |id| match id {
            ActivitySemanticId::Partition(value) => Some(value),
            _ => None,
        },
        "ActivityPartition",
    )
}
fn resolve_structured(
    repository: &ActivityRepository,
    namespace: &str,
    reference: &StructuredNodeReference,
    operation: usize,
) -> Result<StructuredNodeId, BuildDiagnostic> {
    resolve_activity_semantic(
        repository,
        namespace,
        reference,
        operation,
        |id| match id {
            ActivitySemanticId::StructuredNode(value) => Some(value),
            _ => None,
        },
        "StructuredActivityNode",
    )
}

fn activity_for_semantic(
    repository: &ActivityRepository,
    identity: ActivitySemanticId,
) -> Option<ActivityId> {
    repository.activities.values().find_map(|activity| {
        let found = match identity {
            ActivitySemanticId::Node(id) => activity.nodes.iter().any(|node| node.id == id),
            ActivitySemanticId::Pin(id) => activity.nodes.iter().any(|node| match &node.kind {
                ActivityNodeKind::Action(action) => action.pins.iter().any(|pin| pin.id == id),
                _ => false,
            }),
            ActivitySemanticId::Edge(id) => activity.edges.iter().any(|edge| edge.id == id),
            ActivitySemanticId::Partition(id) => activity
                .partitions
                .iter()
                .any(|partition| partition.id == id),
            ActivitySemanticId::StructuredNode(id) => {
                activity.structured_nodes.iter().any(|node| node.id == id)
            }
        };
        found.then_some(activity.id)
    })
}

fn activity_mut_for_semantic(
    repository: &mut ActivityRepository,
    identity: ActivitySemanticId,
) -> Option<&mut Activity> {
    let activity_id = activity_for_semantic(repository, identity)?;
    repository.activities.get_mut(&activity_id)
}

fn resolve_activity_endpoint(
    repository: &ActivityRepository,
    namespace: &str,
    reference: &ActivityEndpointReference,
    operation: usize,
) -> Result<ActivityEndpoint, BuildDiagnostic> {
    match reference {
        ActivityEndpointReference::Node(reference) => {
            resolve_node(repository, namespace, reference, operation).map(ActivityEndpoint::Node)
        }
        ActivityEndpointReference::Pin(reference) => {
            resolve_pin(repository, namespace, reference, operation).map(ActivityEndpoint::Pin)
        }
    }
}

fn action_kind(
    spec: &ActionBuildKind,
    project: &Project,
    activities: &ActivityRepository,
    element_ids: &HashMap<String, ElementId>,
    namespace: &str,
    operation: usize,
) -> Result<ActionKind, BuildDiagnostic> {
    Ok(match spec {
        ActionBuildKind::Opaque { body } => ActionKind::Opaque { body: body.clone() },
        ActionBuildKind::CallBehavior { activity } => ActionKind::CallBehavior {
            activity_id: resolve_activity(activities, namespace, activity, operation)?,
        },
        ActionBuildKind::CallOperation {
            operation: reference,
        } => ActionKind::CallOperation {
            operation_id: resolve_element(project, element_ids, namespace, reference, operation)?,
        },
        ActionBuildKind::SendSignal { signal } => ActionKind::SendSignal {
            signal_id: resolve_element(project, element_ids, namespace, signal, operation)?,
        },
        ActionBuildKind::AcceptEvent { signal } => ActionKind::AcceptEvent {
            signal_id: signal
                .as_ref()
                .map(|reference| {
                    resolve_element(project, element_ids, namespace, reference, operation)
                })
                .transpose()?,
        },
        ActionBuildKind::AcceptTimeEvent { expression } => ActionKind::AcceptTimeEvent {
            expression: expression.clone(),
        },
    })
}

fn node_kind(
    spec: &ActivityNodeBuildKind,
    project: &Project,
    activities: &ActivityRepository,
    element_ids: &HashMap<String, ElementId>,
    namespace: &str,
    operation: usize,
) -> Result<ActivityNodeKind, BuildDiagnostic> {
    Ok(match spec {
        ActivityNodeBuildKind::Initial => ActivityNodeKind::Initial,
        ActivityNodeBuildKind::ActivityFinal => ActivityNodeKind::ActivityFinal,
        ActivityNodeBuildKind::FlowFinal => ActivityNodeKind::FlowFinal,
        ActivityNodeBuildKind::Decision { decision_input } => ActivityNodeKind::Decision {
            decision_input: decision_input.clone(),
        },
        ActivityNodeBuildKind::Merge => ActivityNodeKind::Merge,
        ActivityNodeBuildKind::Fork => ActivityNodeKind::Fork,
        ActivityNodeBuildKind::Join { join_specification } => ActivityNodeKind::Join {
            join_specification: join_specification.clone(),
        },
        ActivityNodeBuildKind::Action(action) => ActivityNodeKind::Action(Action {
            kind: action_kind(
                action,
                project,
                activities,
                element_ids,
                namespace,
                operation,
            )?,
            pins: Vec::new(),
        }),
        ActivityNodeBuildKind::Object {
            kind,
            type_ref,
            multiplicity,
            ordering,
            selection,
        } => ActivityNodeKind::Object(ObjectNode {
            kind: *kind,
            type_id: type_ref
                .as_ref()
                .map(|reference| {
                    resolve_element(project, element_ids, namespace, reference, operation)
                })
                .transpose()?,
            multiplicity: *multiplicity,
            ordering: *ordering,
            selection: selection.clone(),
        }),
        ActivityNodeBuildKind::ActivityParameter { parameter } => {
            ActivityNodeKind::ActivityParameter(ActivityParameterNode {
                parameter_id: resolve_element(
                    project,
                    element_ids,
                    namespace,
                    parameter,
                    operation,
                )?,
            })
        }
    })
}

fn apply_activity_operation(
    operation_spec: &ActivityBuildOperation,
    project: &Project,
    element_ids: &HashMap<String, ElementId>,
    activities: &mut ActivityRepository,
    behavior: &BehaviorRepository,
    namespace: &str,
    operation: usize,
) -> Result<(), BuildDiagnostic> {
    match operation_spec {
        ActivityBuildOperation::CreateActivity {
            external_id,
            name,
            owner,
            context,
        } => {
            let key = external_key(namespace, external_id);
            ensure_external_available(&key, project, activities, behavior, operation)?;
            let owner_id = resolve_element(project, element_ids, namespace, owner, operation)?;
            let context_id = context
                .as_ref()
                .map(|reference| {
                    resolve_element(project, element_ids, namespace, reference, operation)
                })
                .transpose()?;
            let id = activities
                .create_activity(project, owner_id, context_id, name)
                .map_err(|cause| {
                    error(
                        "ACTIVITY_SEMANTIC_VALIDATION",
                        Some(operation),
                        cause.to_string(),
                    )
                })?;
            activities
                .activities
                .get_mut(&id)
                .expect("created Activity")
                .external_id = key;
        }
        ActivityBuildOperation::UpdateActivity {
            activity,
            name,
            owner,
            context,
        } => {
            let id = resolve_activity(activities, namespace, activity, operation)?;
            let owner_id = owner
                .as_ref()
                .map(|reference| {
                    resolve_element(project, element_ids, namespace, reference, operation)
                })
                .transpose()?;
            let context_id = context
                .as_ref()
                .map(|value| {
                    value
                        .as_ref()
                        .map(|reference| {
                            resolve_element(project, element_ids, namespace, reference, operation)
                        })
                        .transpose()
                })
                .transpose()?;
            let record = activities
                .activities
                .get_mut(&id)
                .expect("resolved Activity");
            if let Some(value) = name {
                record.name = value.clone();
            }
            if let Some(value) = owner_id {
                record.owner_id = value;
            }
            if let Some(value) = context_id {
                record.context_id = value;
            }
        }
        ActivityBuildOperation::CreatePartition {
            external_id,
            activity,
            name,
            represented_element,
            is_dimension,
            is_external,
        } => {
            let key = external_key(namespace, external_id);
            ensure_external_available(&key, project, activities, behavior, operation)?;
            let activity_id = resolve_activity(activities, namespace, activity, operation)?;
            let represented_element_id = represented_element
                .as_ref()
                .map(|reference| {
                    resolve_element(project, element_ids, namespace, reference, operation)
                })
                .transpose()?;
            let id = ActivityPartitionId::new();
            activities
                .activities
                .get_mut(&activity_id)
                .expect("resolved Activity")
                .partitions
                .push(ActivityPartition {
                    id,
                    name: name.clone(),
                    represented_element_id,
                    is_dimension: *is_dimension,
                    is_external: *is_external,
                });
            activities
                .external_ids
                .insert(key, ActivitySemanticId::Partition(id));
        }
        ActivityBuildOperation::UpdatePartition {
            partition,
            name,
            represented_element,
            is_dimension,
            is_external,
        } => {
            let id = resolve_partition(activities, namespace, partition, operation)?;
            let represented = represented_element
                .as_ref()
                .map(|value| {
                    value
                        .as_ref()
                        .map(|reference| {
                            resolve_element(project, element_ids, namespace, reference, operation)
                        })
                        .transpose()
                })
                .transpose()?;
            let activity = activity_mut_for_semantic(activities, ActivitySemanticId::Partition(id))
                .ok_or_else(|| {
                    error(
                        "UNRESOLVED_BEHAVIOR_REFERENCE",
                        Some(operation),
                        "ActivityPartition owner Activity was not found",
                    )
                })?;
            let record = activity
                .partitions
                .iter_mut()
                .find(|candidate| candidate.id == id)
                .expect("resolved partition");
            if let Some(value) = name {
                record.name = value.clone();
            }
            if let Some(value) = represented {
                record.represented_element_id = value;
            }
            if let Some(value) = is_dimension {
                record.is_dimension = *value;
            }
            if let Some(value) = is_external {
                record.is_external = *value;
            }
        }
        ActivityBuildOperation::CreateStructuredNode {
            external_id,
            activity,
            name,
            kind,
            parent,
        } => {
            let key = external_key(namespace, external_id);
            ensure_external_available(&key, project, activities, behavior, operation)?;
            let activity_id = resolve_activity(activities, namespace, activity, operation)?;
            let parent_id = parent
                .as_ref()
                .map(|reference| resolve_structured(activities, namespace, reference, operation))
                .transpose()?;
            if parent_id.is_some_and(|id| {
                activity_for_semantic(activities, ActivitySemanticId::StructuredNode(id))
                    != Some(activity_id)
            }) {
                return Err(error(
                    "ACTIVITY_CONTAINMENT_INVALID",
                    Some(operation),
                    "StructuredActivityNode parent belongs to another Activity",
                ));
            }
            let id = StructuredNodeId::new();
            activities
                .activities
                .get_mut(&activity_id)
                .expect("resolved Activity")
                .structured_nodes
                .push(StructuredActivityNode {
                    id,
                    name: name.clone(),
                    kind: *kind,
                    parent_id,
                });
            activities
                .external_ids
                .insert(key, ActivitySemanticId::StructuredNode(id));
        }
        ActivityBuildOperation::UpdateStructuredNode {
            node,
            name,
            kind,
            parent,
        } => {
            let id = resolve_structured(activities, namespace, node, operation)?;
            let activity_id =
                activity_for_semantic(activities, ActivitySemanticId::StructuredNode(id))
                    .ok_or_else(|| {
                        error(
                            "UNRESOLVED_BEHAVIOR_REFERENCE",
                            Some(operation),
                            "StructuredActivityNode owner Activity was not found",
                        )
                    })?;
            let parent_id = parent
                .as_ref()
                .map(|value| {
                    value
                        .as_ref()
                        .map(|reference| {
                            resolve_structured(activities, namespace, reference, operation)
                        })
                        .transpose()
                })
                .transpose()?;
            if parent_id.flatten().is_some_and(|parent| {
                activity_for_semantic(activities, ActivitySemanticId::StructuredNode(parent))
                    != Some(activity_id)
            }) {
                return Err(error(
                    "ACTIVITY_CONTAINMENT_INVALID",
                    Some(operation),
                    "StructuredActivityNode parent belongs to another Activity",
                ));
            }
            let record = activities
                .activities
                .get_mut(&activity_id)
                .unwrap()
                .structured_nodes
                .iter_mut()
                .find(|candidate| candidate.id == id)
                .unwrap();
            if let Some(value) = name {
                record.name = value.clone();
            }
            if let Some(value) = kind {
                record.kind = *value;
            }
            if let Some(value) = parent_id {
                record.parent_id = value;
            }
        }
        ActivityBuildOperation::CreateNode {
            external_id,
            activity,
            name,
            kind,
            partition,
            structured_node,
        } => {
            let key = external_key(namespace, external_id);
            ensure_external_available(&key, project, activities, behavior, operation)?;
            let activity_id = resolve_activity(activities, namespace, activity, operation)?;
            let partition_id = partition
                .as_ref()
                .map(|reference| resolve_partition(activities, namespace, reference, operation))
                .transpose()?;
            let structured_node_id = structured_node
                .as_ref()
                .map(|reference| resolve_structured(activities, namespace, reference, operation))
                .transpose()?;
            for identity in partition_id
                .map(ActivitySemanticId::Partition)
                .into_iter()
                .chain(structured_node_id.map(ActivitySemanticId::StructuredNode))
            {
                if activity_for_semantic(activities, identity) != Some(activity_id) {
                    return Err(error(
                        "ACTIVITY_CONTAINMENT_INVALID",
                        Some(operation),
                        "Activity node containment reference belongs to another Activity",
                    ));
                }
            }
            let id = ActivityNodeId::new();
            let native_kind =
                node_kind(kind, project, activities, element_ids, namespace, operation)?;
            activities
                .activities
                .get_mut(&activity_id)
                .unwrap()
                .nodes
                .push(ActivityNode {
                    id,
                    name: name.clone(),
                    kind: native_kind,
                    partition_id,
                    structured_node_id,
                });
            activities
                .external_ids
                .insert(key, ActivitySemanticId::Node(id));
        }
        ActivityBuildOperation::UpdateNode {
            node,
            name,
            kind,
            partition,
            structured_node,
        } => {
            let id = resolve_node(activities, namespace, node, operation)?;
            let activity_id = activity_for_semantic(activities, ActivitySemanticId::Node(id))
                .ok_or_else(|| {
                    error(
                        "UNRESOLVED_BEHAVIOR_REFERENCE",
                        Some(operation),
                        "ActivityNode owner Activity was not found",
                    )
                })?;
            let partition_id = partition
                .as_ref()
                .map(|value| {
                    value
                        .as_ref()
                        .map(|reference| {
                            resolve_partition(activities, namespace, reference, operation)
                        })
                        .transpose()
                })
                .transpose()?;
            let structured_id = structured_node
                .as_ref()
                .map(|value| {
                    value
                        .as_ref()
                        .map(|reference| {
                            resolve_structured(activities, namespace, reference, operation)
                        })
                        .transpose()
                })
                .transpose()?;
            if partition_id.flatten().is_some_and(|value| {
                activity_for_semantic(activities, ActivitySemanticId::Partition(value))
                    != Some(activity_id)
            }) || structured_id.flatten().is_some_and(|value| {
                activity_for_semantic(activities, ActivitySemanticId::StructuredNode(value))
                    != Some(activity_id)
            }) {
                return Err(error(
                    "ACTIVITY_CONTAINMENT_INVALID",
                    Some(operation),
                    "Activity node containment reference belongs to another Activity",
                ));
            }
            let native_kind = kind
                .as_ref()
                .map(|value| {
                    node_kind(
                        value,
                        project,
                        activities,
                        element_ids,
                        namespace,
                        operation,
                    )
                })
                .transpose()?;
            let record = activities
                .activities
                .get_mut(&activity_id)
                .unwrap()
                .nodes
                .iter_mut()
                .find(|candidate| candidate.id == id)
                .unwrap();
            if let Some(value) = name {
                record.name = value.clone();
            }
            if let Some(mut value) = native_kind {
                if let (ActivityNodeKind::Action(existing), ActivityNodeKind::Action(replacement)) =
                    (&record.kind, &mut value)
                {
                    replacement.pins = existing.pins.clone();
                }
                record.kind = value;
            }
            if let Some(value) = partition_id {
                record.partition_id = value;
            }
            if let Some(value) = structured_id {
                record.structured_node_id = value;
            }
        }
        ActivityBuildOperation::CreatePin {
            external_id,
            owner_action,
            name,
            direction,
            type_ref,
            multiplicity,
            is_ordered,
            is_unique,
            value,
            parameter,
        } => {
            let key = external_key(namespace, external_id);
            ensure_external_available(&key, project, activities, behavior, operation)?;
            let node_id = resolve_node(activities, namespace, owner_action, operation)?;
            let type_id = type_ref
                .as_ref()
                .map(|reference| {
                    resolve_element(project, element_ids, namespace, reference, operation)
                })
                .transpose()?;
            let parameter_id = parameter
                .as_ref()
                .map(|reference| {
                    resolve_element(project, element_ids, namespace, reference, operation)
                })
                .transpose()?;
            let activity = activity_mut_for_semantic(activities, ActivitySemanticId::Node(node_id))
                .ok_or_else(|| {
                    error(
                        "UNRESOLVED_BEHAVIOR_REFERENCE",
                        Some(operation),
                        "Pin owner Action was not found",
                    )
                })?;
            let node = activity
                .nodes
                .iter_mut()
                .find(|candidate| candidate.id == node_id)
                .unwrap();
            let ActivityNodeKind::Action(action) = &mut node.kind else {
                return Err(error(
                    "PIN_OWNER_INVALID",
                    Some(operation),
                    "Pin owner must be an Action node",
                ));
            };
            let id = PinId::new();
            action.pins.push(Pin {
                id,
                name: name.clone(),
                direction: *direction,
                type_id,
                multiplicity: *multiplicity,
                is_ordered: *is_ordered,
                is_unique: *is_unique,
                value: value.clone(),
                parameter_id,
            });
            activities
                .external_ids
                .insert(key, ActivitySemanticId::Pin(id));
        }
        ActivityBuildOperation::UpdatePin {
            pin,
            name,
            direction,
            type_ref,
            multiplicity,
            is_ordered,
            is_unique,
            value,
            parameter,
        } => {
            let id = resolve_pin(activities, namespace, pin, operation)?;
            let type_id = type_ref
                .as_ref()
                .map(|value| {
                    value
                        .as_ref()
                        .map(|reference| {
                            resolve_element(project, element_ids, namespace, reference, operation)
                        })
                        .transpose()
                })
                .transpose()?;
            let parameter_id = parameter
                .as_ref()
                .map(|value| {
                    value
                        .as_ref()
                        .map(|reference| {
                            resolve_element(project, element_ids, namespace, reference, operation)
                        })
                        .transpose()
                })
                .transpose()?;
            let activity = activity_mut_for_semantic(activities, ActivitySemanticId::Pin(id))
                .ok_or_else(|| {
                    error(
                        "UNRESOLVED_BEHAVIOR_REFERENCE",
                        Some(operation),
                        "Pin owner Activity was not found",
                    )
                })?;
            let record = activity
                .nodes
                .iter_mut()
                .filter_map(|node| match &mut node.kind {
                    ActivityNodeKind::Action(action) => Some(&mut action.pins),
                    _ => None,
                })
                .flatten()
                .find(|candidate| candidate.id == id)
                .unwrap();
            if let Some(v) = name {
                record.name = v.clone();
            }
            if let Some(v) = direction {
                record.direction = *v;
            }
            if let Some(v) = type_id {
                record.type_id = v;
            }
            if let Some(v) = multiplicity {
                record.multiplicity = *v;
            }
            if let Some(v) = is_ordered {
                record.is_ordered = *v;
            }
            if let Some(v) = is_unique {
                record.is_unique = *v;
            }
            if let Some(v) = value {
                record.value = v.clone();
            }
            if let Some(v) = parameter_id {
                record.parameter_id = v;
            }
        }
        ActivityBuildOperation::CreateEdge {
            external_id,
            activity,
            name,
            kind,
            source,
            target,
            guard,
            weight,
            selection,
            transformation,
            interrupting_region,
        } => {
            let key = external_key(namespace, external_id);
            ensure_external_available(&key, project, activities, behavior, operation)?;
            let activity_id = resolve_activity(activities, namespace, activity, operation)?;
            let source = resolve_activity_endpoint(activities, namespace, source, operation)?;
            let target = resolve_activity_endpoint(activities, namespace, target, operation)?;
            let interrupting_region_id = interrupting_region
                .as_ref()
                .map(|reference| resolve_structured(activities, namespace, reference, operation))
                .transpose()?;
            for identity in [source, target]
                .into_iter()
                .map(|endpoint| match endpoint {
                    ActivityEndpoint::Node(id) => ActivitySemanticId::Node(id),
                    ActivityEndpoint::Pin(id) => ActivitySemanticId::Pin(id),
                })
                .chain(interrupting_region_id.map(ActivitySemanticId::StructuredNode))
            {
                if activity_for_semantic(activities, identity) != Some(activity_id) {
                    return Err(error(
                        "ACTIVITY_ENDPOINT_SCOPE_INVALID",
                        Some(operation),
                        "Activity edge endpoint/region belongs to another Activity",
                    ));
                }
            }
            let id = ActivityEdgeId::new();
            activities
                .activities
                .get_mut(&activity_id)
                .unwrap()
                .edges
                .push(ActivityEdge {
                    id,
                    name: name.clone(),
                    kind: *kind,
                    source,
                    target,
                    guard: guard.clone(),
                    weight: weight.clone(),
                    selection: selection.clone(),
                    transformation: transformation.clone(),
                    interrupting_region_id,
                });
            activities
                .external_ids
                .insert(key, ActivitySemanticId::Edge(id));
        }
        ActivityBuildOperation::UpdateEdge {
            edge,
            name,
            kind,
            source,
            target,
            guard,
            weight,
            selection,
            transformation,
            interrupting_region,
        } => {
            let id = resolve_edge(activities, namespace, edge, operation)?;
            let activity_id = activity_for_semantic(activities, ActivitySemanticId::Edge(id))
                .ok_or_else(|| {
                    error(
                        "UNRESOLVED_BEHAVIOR_REFERENCE",
                        Some(operation),
                        "ActivityEdge owner Activity was not found",
                    )
                })?;
            let source_id = source
                .as_ref()
                .map(|reference| {
                    resolve_activity_endpoint(activities, namespace, reference, operation)
                })
                .transpose()?;
            let target_id = target
                .as_ref()
                .map(|reference| {
                    resolve_activity_endpoint(activities, namespace, reference, operation)
                })
                .transpose()?;
            let interrupt_id = interrupting_region
                .as_ref()
                .map(|value| {
                    value
                        .as_ref()
                        .map(|reference| {
                            resolve_structured(activities, namespace, reference, operation)
                        })
                        .transpose()
                })
                .transpose()?;
            for identity in source_id
                .into_iter()
                .chain(target_id)
                .map(|endpoint| match endpoint {
                    ActivityEndpoint::Node(id) => ActivitySemanticId::Node(id),
                    ActivityEndpoint::Pin(id) => ActivitySemanticId::Pin(id),
                })
                .chain(
                    interrupt_id
                        .flatten()
                        .map(ActivitySemanticId::StructuredNode),
                )
            {
                if activity_for_semantic(activities, identity) != Some(activity_id) {
                    return Err(error(
                        "ACTIVITY_ENDPOINT_SCOPE_INVALID",
                        Some(operation),
                        "Activity edge endpoint/region belongs to another Activity",
                    ));
                }
            }
            let record = activities
                .activities
                .get_mut(&activity_id)
                .unwrap()
                .edges
                .iter_mut()
                .find(|candidate| candidate.id == id)
                .unwrap();
            if let Some(v) = name {
                record.name = v.clone();
            }
            if let Some(v) = kind {
                record.kind = *v;
            }
            if let Some(v) = source_id {
                record.source = v;
            }
            if let Some(v) = target_id {
                record.target = v;
            }
            if let Some(v) = guard {
                record.guard = v.clone();
            }
            if let Some(v) = weight {
                record.weight = v.clone();
            }
            if let Some(v) = selection {
                record.selection = v.clone();
            }
            if let Some(v) = transformation {
                record.transformation = v.clone();
            }
            if let Some(v) = interrupt_id {
                record.interrupting_region_id = v;
            }
        }
    }
    Ok(())
}

fn resolve_state_machine(
    repository: &BehaviorRepository,
    namespace: &str,
    reference: &StateMachineReference,
    operation: usize,
) -> Result<StateMachineId, BuildDiagnostic> {
    match reference {
        BuildReference::Existing(id) if repository.state_machines.contains_key(id) => Ok(*id),
        BuildReference::Existing(id) => Err(error(
            "UNRESOLVED_STATE_MACHINE_REFERENCE",
            Some(operation),
            format!("StateMachine {id} was not found"),
        )),
        BuildReference::External(external) => {
            let key = external_key(namespace, external);
            let matches = repository
                .state_machines
                .values()
                .filter(|machine| machine.external_id == key)
                .map(|machine| machine.id)
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [id] => Ok(*id),
                [] => Err(error(
                    "UNRESOLVED_STATE_MACHINE_REFERENCE",
                    Some(operation),
                    format!("StateMachine external ID '{external}' was not found"),
                )),
                _ => Err(error(
                    "AMBIGUOUS_STATE_MACHINE_REFERENCE",
                    Some(operation),
                    format!("StateMachine external ID '{external}' is ambiguous"),
                )),
            }
        }
    }
}

fn resolve_behavior_semantic<T: Copy>(
    repository: &BehaviorRepository,
    namespace: &str,
    reference: &BuildReference<T>,
    operation: usize,
    expected: fn(BehaviorSemanticId) -> Option<T>,
    label: &str,
) -> Result<T, BuildDiagnostic> {
    match reference {
        BuildReference::Existing(id) => Ok(*id),
        BuildReference::External(external) => {
            let key = external_key(namespace, external);
            repository.external_ids.get(&key).copied().and_then(expected).ok_or_else(|| error("UNRESOLVED_BEHAVIOR_REFERENCE",Some(operation),format!("{label} external ID '{external}' was not found with the required semantic kind")))
        }
    }
}
fn resolve_region(
    repository: &BehaviorRepository,
    namespace: &str,
    reference: &RegionReference,
    operation: usize,
) -> Result<RegionId, BuildDiagnostic> {
    resolve_behavior_semantic(
        repository,
        namespace,
        reference,
        operation,
        |id| match id {
            BehaviorSemanticId::Region(v) => Some(v),
            _ => None,
        },
        "Region",
    )
}
fn resolve_vertex(
    repository: &BehaviorRepository,
    namespace: &str,
    reference: &VertexReference,
    operation: usize,
) -> Result<VertexId, BuildDiagnostic> {
    resolve_behavior_semantic(
        repository,
        namespace,
        reference,
        operation,
        |id| match id {
            BehaviorSemanticId::Vertex(v) => Some(v),
            _ => None,
        },
        "Vertex",
    )
}
fn resolve_transition(
    repository: &BehaviorRepository,
    namespace: &str,
    reference: &TransitionReference,
    operation: usize,
) -> Result<TransitionId, BuildDiagnostic> {
    resolve_behavior_semantic(
        repository,
        namespace,
        reference,
        operation,
        |id| match id {
            BehaviorSemanticId::Transition(v) => Some(v),
            _ => None,
        },
        "Transition",
    )
}

fn find_region(regions: &[Region], id: RegionId) -> Option<&Region> {
    for region in regions {
        if region.id == id {
            return Some(region);
        }
        for vertex in &region.vertices {
            if let VertexKind::State(state) = &vertex.kind {
                if let Some(found) = find_region(&state.regions, id) {
                    return Some(found);
                }
            }
        }
    }
    None
}
fn find_region_mut(regions: &mut [Region], id: RegionId) -> Option<&mut Region> {
    for region in regions {
        if region.id == id {
            return Some(region);
        }
        for vertex in &mut region.vertices {
            if let VertexKind::State(state) = &mut vertex.kind {
                if let Some(found) = find_region_mut(&mut state.regions, id) {
                    return Some(found);
                }
            }
        }
    }
    None
}
fn find_vertex(regions: &[Region], id: VertexId) -> Option<&Vertex> {
    for region in regions {
        for vertex in &region.vertices {
            if vertex.id == id {
                return Some(vertex);
            }
            if let VertexKind::State(state) = &vertex.kind {
                if let Some(found) = find_vertex(&state.regions, id) {
                    return Some(found);
                }
            }
        }
    }
    None
}
fn find_vertex_mut(regions: &mut [Region], id: VertexId) -> Option<&mut Vertex> {
    for region in regions {
        for vertex in &mut region.vertices {
            if vertex.id == id {
                return Some(vertex);
            }
            if let VertexKind::State(state) = &mut vertex.kind {
                if let Some(found) = find_vertex_mut(&mut state.regions, id) {
                    return Some(found);
                }
            }
        }
    }
    None
}
fn find_transition_mut(regions: &mut [Region], id: TransitionId) -> Option<&mut Transition> {
    for region in regions {
        if let Some(found) = region.transitions.iter_mut().find(|t| t.id == id) {
            return Some(found);
        }
        for vertex in &mut region.vertices {
            if let VertexKind::State(state) = &mut vertex.kind {
                if let Some(found) = find_transition_mut(&mut state.regions, id) {
                    return Some(found);
                }
            }
        }
    }
    None
}

fn machine_for_behavior_identity(
    repository: &BehaviorRepository,
    identity: BehaviorSemanticId,
) -> Option<StateMachineId> {
    repository.state_machines.values().find_map(|machine| {
        let found = match identity {
            BehaviorSemanticId::Region(id) => find_region(&machine.regions, id).is_some(),
            BehaviorSemanticId::Vertex(id) => find_vertex(&machine.regions, id).is_some(),
            BehaviorSemanticId::Transition(id) => {
                fn has(regions: &[Region], id: TransitionId) -> bool {
                    regions.iter().any(|r| {
                        r.transitions.iter().any(|t| t.id == id)
                            || r.vertices.iter().any(
                                |v| matches!(&v.kind,VertexKind::State(s) if has(&s.regions,id)),
                            )
                    })
                }
                has(&machine.regions, id)
            }
            _ => false,
        };
        found.then_some(machine.id)
    })
}

fn trigger_from_build(
    spec: &TriggerBuild,
    project: &Project,
    element_ids: &HashMap<String, ElementId>,
    namespace: &str,
    operation: usize,
) -> Result<Trigger, BuildDiagnostic> {
    Ok(Trigger {
        event: match spec {
            TriggerBuild::Signal(reference) => Event::Signal {
                signal_id: resolve_element(project, element_ids, namespace, reference, operation)?,
            },
            TriggerBuild::Call(reference) => Event::Call {
                operation_id: resolve_element(
                    project,
                    element_ids,
                    namespace,
                    reference,
                    operation,
                )?,
            },
            TriggerBuild::Time {
                expression,
                is_relative,
            } => Event::Time {
                expression: expression.clone(),
                is_relative: *is_relative,
            },
            TriggerBuild::Change { expression } => Event::Change {
                expression: expression.clone(),
            },
            TriggerBuild::AnyReceive => Event::AnyReceive,
        },
    })
}

fn vertex_kind_from_build(
    spec: &VertexBuildKind,
    behavior: &BehaviorRepository,
    namespace: &str,
    operation: usize,
) -> Result<VertexKind, BuildDiagnostic> {
    Ok(match spec {
        VertexBuildKind::State {
            entry,
            do_activity,
            exit,
            submachine,
        } => VertexKind::State(State {
            entry: entry.clone(),
            do_activity: do_activity.clone(),
            exit: exit.clone(),
            submachine: submachine
                .as_ref()
                .map(|reference| resolve_state_machine(behavior, namespace, reference, operation))
                .transpose()?,
            regions: Vec::new(),
        }),
        VertexBuildKind::FinalState => VertexKind::FinalState,
        VertexBuildKind::Pseudostate(kind) => VertexKind::Pseudostate(*kind),
    })
}

fn apply_state_machine_operation(
    operation_spec: &StateMachineBuildOperation,
    project: &Project,
    element_ids: &HashMap<String, ElementId>,
    activities: &ActivityRepository,
    behavior: &mut BehaviorRepository,
    namespace: &str,
    operation: usize,
) -> Result<(), BuildDiagnostic> {
    match operation_spec {
        StateMachineBuildOperation::CreateStateMachine {
            external_id,
            name,
            context,
        } => {
            let key = external_key(namespace, external_id);
            ensure_external_available(&key, project, activities, behavior, operation)?;
            let context_id = resolve_element(project, element_ids, namespace, context, operation)?;
            let id = behavior
                .create_state_machine(project, context_id, name)
                .map_err(|cause| {
                    error(
                        "STATE_MACHINE_SEMANTIC_VALIDATION",
                        Some(operation),
                        cause.to_string(),
                    )
                })?;
            let machine = behavior.state_machines.get_mut(&id).unwrap();
            machine.external_id = key;
            machine.regions.clear();
        }
        StateMachineBuildOperation::UpdateStateMachine {
            state_machine,
            name,
            context,
        } => {
            let id = resolve_state_machine(behavior, namespace, state_machine, operation)?;
            let context_id = context
                .as_ref()
                .map(|reference| {
                    resolve_element(project, element_ids, namespace, reference, operation)
                })
                .transpose()?;
            let machine = behavior.state_machines.get_mut(&id).unwrap();
            if let Some(v) = name {
                machine.name = v.clone();
            }
            if let Some(v) = context_id {
                machine.context_id = v;
            }
        }
        StateMachineBuildOperation::CreateRegion {
            external_id,
            parent,
            name,
        } => {
            let key = external_key(namespace, external_id);
            ensure_external_available(&key, project, activities, behavior, operation)?;
            let id = RegionId::new();
            let record = Region {
                id,
                name: name.clone(),
                vertices: Vec::new(),
                transitions: Vec::new(),
            };
            match parent {
                RegionParentReference::StateMachine(reference) => {
                    let machine_id =
                        resolve_state_machine(behavior, namespace, reference, operation)?;
                    behavior
                        .state_machines
                        .get_mut(&machine_id)
                        .unwrap()
                        .regions
                        .push(record);
                }
                RegionParentReference::State(reference) => {
                    let vertex_id = resolve_vertex(behavior, namespace, reference, operation)?;
                    let machine_id = machine_for_behavior_identity(
                        behavior,
                        BehaviorSemanticId::Vertex(vertex_id),
                    )
                    .ok_or_else(|| {
                        error(
                            "UNRESOLVED_BEHAVIOR_REFERENCE",
                            Some(operation),
                            "nested Region parent State was not found",
                        )
                    })?;
                    let vertex = find_vertex_mut(
                        &mut behavior
                            .state_machines
                            .get_mut(&machine_id)
                            .unwrap()
                            .regions,
                        vertex_id,
                    )
                    .unwrap();
                    let VertexKind::State(state) = &mut vertex.kind else {
                        return Err(error(
                            "REGION_PARENT_INVALID",
                            Some(operation),
                            "nested Region parent must be a State",
                        ));
                    };
                    state.regions.push(record);
                }
            }
            behavior
                .external_ids
                .insert(key, BehaviorSemanticId::Region(id));
        }
        StateMachineBuildOperation::UpdateRegion { region, name } => {
            let id = resolve_region(behavior, namespace, region, operation)?;
            let machine_id =
                machine_for_behavior_identity(behavior, BehaviorSemanticId::Region(id))
                    .ok_or_else(|| {
                        error(
                            "UNRESOLVED_BEHAVIOR_REFERENCE",
                            Some(operation),
                            "Region owner StateMachine was not found",
                        )
                    })?;
            let record = find_region_mut(
                &mut behavior
                    .state_machines
                    .get_mut(&machine_id)
                    .unwrap()
                    .regions,
                id,
            )
            .unwrap();
            if let Some(v) = name {
                record.name = v.clone();
            }
        }
        StateMachineBuildOperation::CreateVertex {
            external_id,
            region,
            name,
            kind,
        } => {
            let key = external_key(namespace, external_id);
            ensure_external_available(&key, project, activities, behavior, operation)?;
            let region_id = resolve_region(behavior, namespace, region, operation)?;
            let machine_id =
                machine_for_behavior_identity(behavior, BehaviorSemanticId::Region(region_id))
                    .ok_or_else(|| {
                        error(
                            "UNRESOLVED_BEHAVIOR_REFERENCE",
                            Some(operation),
                            "Region owner StateMachine was not found",
                        )
                    })?;
            let native_kind = vertex_kind_from_build(kind, behavior, namespace, operation)?;
            let id = VertexId::new();
            find_region_mut(
                &mut behavior
                    .state_machines
                    .get_mut(&machine_id)
                    .unwrap()
                    .regions,
                region_id,
            )
            .unwrap()
            .vertices
            .push(Vertex {
                id,
                name: name.clone(),
                kind: native_kind,
            });
            behavior
                .external_ids
                .insert(key, BehaviorSemanticId::Vertex(id));
        }
        StateMachineBuildOperation::UpdateVertex { vertex, name, kind } => {
            let id = resolve_vertex(behavior, namespace, vertex, operation)?;
            let machine_id =
                machine_for_behavior_identity(behavior, BehaviorSemanticId::Vertex(id))
                    .ok_or_else(|| {
                        error(
                            "UNRESOLVED_BEHAVIOR_REFERENCE",
                            Some(operation),
                            "Vertex owner StateMachine was not found",
                        )
                    })?;
            let native_kind = kind
                .as_ref()
                .map(|value| vertex_kind_from_build(value, behavior, namespace, operation))
                .transpose()?;
            let record = find_vertex_mut(
                &mut behavior
                    .state_machines
                    .get_mut(&machine_id)
                    .unwrap()
                    .regions,
                id,
            )
            .unwrap();
            if let Some(v) = name {
                record.name = v.clone();
            }
            if let Some(mut v) = native_kind {
                if let (VertexKind::State(existing), VertexKind::State(replacement)) =
                    (&record.kind, &mut v)
                {
                    replacement.regions = existing.regions.clone();
                }
                record.kind = v;
            }
        }
        StateMachineBuildOperation::CreateTransition {
            external_id,
            region,
            source,
            target,
            kind,
            trigger,
            guard,
            effect,
        } => {
            let key = external_key(namespace, external_id);
            ensure_external_available(&key, project, activities, behavior, operation)?;
            let region_id = resolve_region(behavior, namespace, region, operation)?;
            let source_id = resolve_vertex(behavior, namespace, source, operation)?;
            let target_id = resolve_vertex(behavior, namespace, target, operation)?;
            let machine_id =
                machine_for_behavior_identity(behavior, BehaviorSemanticId::Region(region_id))
                    .ok_or_else(|| {
                        error(
                            "UNRESOLVED_BEHAVIOR_REFERENCE",
                            Some(operation),
                            "Region owner StateMachine was not found",
                        )
                    })?;
            if machine_for_behavior_identity(behavior, BehaviorSemanticId::Vertex(source_id))
                != Some(machine_id)
                || machine_for_behavior_identity(behavior, BehaviorSemanticId::Vertex(target_id))
                    != Some(machine_id)
            {
                return Err(error(
                    "TRANSITION_ENDPOINT_SCOPE_INVALID",
                    Some(operation),
                    "Transition source/target must belong to the same StateMachine",
                ));
            }
            let native_trigger = trigger
                .as_ref()
                .map(|value| trigger_from_build(value, project, element_ids, namespace, operation))
                .transpose()?;
            let id = TransitionId::new();
            find_region_mut(
                &mut behavior
                    .state_machines
                    .get_mut(&machine_id)
                    .unwrap()
                    .regions,
                region_id,
            )
            .unwrap()
            .transitions
            .push(Transition {
                id,
                source_id,
                target_id,
                kind: *kind,
                trigger: native_trigger,
                guard: guard.clone(),
                effect: effect.clone(),
            });
            behavior
                .external_ids
                .insert(key, BehaviorSemanticId::Transition(id));
        }
        StateMachineBuildOperation::UpdateTransition {
            transition,
            source,
            target,
            kind,
            trigger,
            guard,
            effect,
        } => {
            let id = resolve_transition(behavior, namespace, transition, operation)?;
            let machine_id =
                machine_for_behavior_identity(behavior, BehaviorSemanticId::Transition(id))
                    .ok_or_else(|| {
                        error(
                            "UNRESOLVED_BEHAVIOR_REFERENCE",
                            Some(operation),
                            "Transition owner StateMachine was not found",
                        )
                    })?;
            let source_id = source
                .as_ref()
                .map(|r| resolve_vertex(behavior, namespace, r, operation))
                .transpose()?;
            let target_id = target
                .as_ref()
                .map(|r| resolve_vertex(behavior, namespace, r, operation))
                .transpose()?;
            for endpoint in source_id.into_iter().chain(target_id) {
                if machine_for_behavior_identity(behavior, BehaviorSemanticId::Vertex(endpoint))
                    != Some(machine_id)
                {
                    return Err(error(
                        "TRANSITION_ENDPOINT_SCOPE_INVALID",
                        Some(operation),
                        "Transition source/target must belong to the same StateMachine",
                    ));
                }
            }
            let native_trigger = trigger
                .as_ref()
                .map(|value| {
                    value
                        .as_ref()
                        .map(|spec| {
                            trigger_from_build(spec, project, element_ids, namespace, operation)
                        })
                        .transpose()
                })
                .transpose()?;
            let record = find_transition_mut(
                &mut behavior
                    .state_machines
                    .get_mut(&machine_id)
                    .unwrap()
                    .regions,
                id,
            )
            .unwrap();
            if let Some(v) = source_id {
                record.source_id = v;
            }
            if let Some(v) = target_id {
                record.target_id = v;
            }
            if let Some(v) = kind {
                record.kind = *v;
            }
            if let Some(v) = native_trigger {
                record.trigger = v;
            }
            if let Some(v) = guard {
                record.guard = v.clone();
            }
            if let Some(v) = effect {
                record.effect = v.clone();
            }
        }
    }
    Ok(())
}

fn is_behavior_operation(operation: &ModelBuildOperation) -> bool {
    matches!(
        operation,
        ModelBuildOperation::Activity { .. }
            | ModelBuildOperation::StateMachine { .. }
            | ModelBuildOperation::Sequence { .. }
            | ModelBuildOperation::Parametric { .. }
    )
}

fn validate_cross_store_project_creates(
    plan: &ModelBuildPlan,
    activities: &ActivityRepository,
    behavior: &BehaviorRepository,
) -> Result<(), BuildDiagnostic> {
    let namespace = plan.source_namespace.trim();
    for (index, operation) in plan.operations.iter().enumerate() {
        let external = match operation {
            ModelBuildOperation::CreateElement { external_id, .. }
            | ModelBuildOperation::CreateRelationship { external_id, .. }
            | ModelBuildOperation::CreateConnector { external_id, .. }
            | ModelBuildOperation::CreateItemFlow { external_id, .. }
            | ModelBuildOperation::CreateDiagram { external_id, .. } => Some(external_id),
            _ => None,
        };
        if let Some(external) = external {
            let key = external_key(namespace, external);
            if activities.activities.values().any(|a| a.external_id == key)
                || activities.external_ids.contains_key(&key)
                || behavior
                    .state_machines
                    .values()
                    .any(|m| m.external_id == key)
                || behavior.interactions.values().any(|i| i.external_id == key)
                || behavior.external_ids.contains_key(&key)
            {
                return Err(error(
                    "DUPLICATE_EXTERNAL_ID",
                    Some(index),
                    format!(
                        "external ID already exists in a specialized authored semantic store: {key}"
                    ),
                ));
            }
        }
    }
    Ok(())
}

struct UnifiedCandidate {
    project: Project,
    diagrams: Vec<BddDiagram>,
    activities: ActivityRepository,
    behavior: BehaviorRepository,
    result: ModelBuildResult,
}

fn build_unified_candidate(
    plan: &ModelBuildPlan,
    project: Project,
    diagrams: Vec<BddDiagram>,
    activities: ActivityRepository,
    activity_diagrams: &[activity_workspace::ActivityDiagram],
    behavior: BehaviorRepository,
    behavior_diagrams: &[behavior_workspace::BehaviorDiagram],
) -> Result<UnifiedCandidate, BuildDiagnostic> {
    preflight(plan)?;
    validate_cross_store_project_creates(plan, &activities, &behavior)?;
    let first_behavior = plan
        .operations
        .iter()
        .position(is_behavior_operation)
        .unwrap_or(plan.operations.len());
    if plan.operations[first_behavior..]
        .iter()
        .any(|operation| !is_behavior_operation(operation))
    {
        return Err(error(
            "BUILD_ORDER_INVALID",
            None,
            "unified plans require ordinary Project operations before specialized repository operations",
        ));
    }
    let project_plan = ModelBuildPlan {
        source_namespace: plan.source_namespace.clone(),
        operations: plan.operations[..first_behavior].to_vec(),
    };
    let CandidateBuild {
        mut project,
        diagrams,
        mut result,
    } = build_candidate(&project_plan, project, diagrams)?;
    let mut activities = activities;
    let mut behavior = behavior;
    let mut sequence_state = SequenceBuildState::from_repository(&behavior)?;
    let namespace = plan.source_namespace.trim();
    for (index, operation) in plan.operations[first_behavior..].iter().enumerate() {
        let absolute = first_behavior + index;
        match operation {
            ModelBuildOperation::Activity { operation } => apply_activity_operation(
                operation,
                &project,
                &result.element_ids,
                &mut activities,
                &behavior,
                namespace,
                absolute,
            )?,
            ModelBuildOperation::StateMachine { operation } => apply_state_machine_operation(
                operation,
                &project,
                &result.element_ids,
                &activities,
                &mut behavior,
                namespace,
                absolute,
            )?,
            ModelBuildOperation::Sequence { operation } => apply_sequence_operation(
                operation,
                &project,
                &result.element_ids,
                &mut behavior,
                &mut sequence_state,
                namespace,
                absolute,
            )?,
            ModelBuildOperation::Parametric { operation } => apply_parametric_operation(
                operation,
                &mut project,
                &result.element_ids,
                &mut result.relationship_ids,
                namespace,
                absolute,
            )?,
            _ => unreachable!(),
        }
    }
    project
        .validate()
        .map_err(|cause| error("SEMANTIC_VALIDATION", None, cause.to_string()))?;
    activities
        .validate(&project)
        .map_err(|cause| error("ACTIVITY_SEMANTIC_VALIDATION", None, cause.to_string()))?;
    // Preserve PR48's state-machine diagnostic contract while validating the
    // expanded PR49 behavior repository. Sequence-only identities are removed
    // from this state-machine projection, then the complete repository is
    // validated immediately afterward for native Sequence semantics.
    let mut state_machine_behavior = behavior.clone();
    state_machine_behavior.interactions.clear();
    state_machine_behavior.external_ids.retain(|_, identity| {
        matches!(
            identity,
            BehaviorSemanticId::Region(_)
                | BehaviorSemanticId::Vertex(_)
                | BehaviorSemanticId::Transition(_)
        )
    });
    state_machine_behavior
        .validate(&project)
        .map_err(|cause| error("STATE_MACHINE_SEMANTIC_VALIDATION", None, cause.to_string()))?;
    // Preserve PR48's state-machine diagnostic contract while validating the
    // expanded PR49 behavior repository. Sequence-only identities are removed
    // from this state-machine projection, then the complete repository is
    // validated immediately afterward for native Sequence semantics.
    let mut state_machine_behavior = behavior.clone();
    state_machine_behavior.interactions.clear();
    state_machine_behavior.external_ids.retain(|_, identity| {
        matches!(
            identity,
            BehaviorSemanticId::Region(_)
                | BehaviorSemanticId::Vertex(_)
                | BehaviorSemanticId::Transition(_)
        )
    });
    state_machine_behavior
        .validate(&project)
        .map_err(|cause| error("STATE_MACHINE_SEMANTIC_VALIDATION", None, cause.to_string()))?;
    // Preserve PR48's state-machine diagnostic contract while validating the
    // expanded PR49 behavior repository. Sequence-only identities are removed
    // from this state-machine projection, then the complete repository is
    // validated immediately afterward for native Sequence semantics.
    let mut state_machine_behavior = behavior.clone();
    state_machine_behavior.interactions.clear();
    state_machine_behavior.external_ids.retain(|_, identity| {
        matches!(
            identity,
            BehaviorSemanticId::Region(_)
                | BehaviorSemanticId::Vertex(_)
                | BehaviorSemanticId::Transition(_)
        )
    });
    state_machine_behavior
        .validate(&project)
        .map_err(|cause| error("STATE_MACHINE_SEMANTIC_VALIDATION", None, cause.to_string()))?;
    // Preserve PR48's state-machine diagnostic contract while validating the
    // expanded PR49 behavior repository. Sequence-only identities are removed
    // from this state-machine projection, then the complete repository is
    // validated immediately afterward for native Sequence semantics.
    let mut state_machine_behavior = behavior.clone();
    state_machine_behavior.interactions.clear();
    state_machine_behavior.external_ids.retain(|_, identity| {
        matches!(
            identity,
            BehaviorSemanticId::Region(_)
                | BehaviorSemanticId::Vertex(_)
                | BehaviorSemanticId::Transition(_)
        )
    });
    state_machine_behavior
        .validate(&project)
        .map_err(|cause| error("STATE_MACHINE_SEMANTIC_VALIDATION", None, cause.to_string()))?;
    behavior
        .validate(&project)
        .map_err(|cause| error("BEHAVIOR_SEMANTIC_VALIDATION", None, cause.to_string()))?;
    activity_workspace::validate_activity_diagrams(&activities, activity_diagrams)
        .map_err(|cause| error("PRESENTATION_VALIDATION", None, cause))?;
    behavior_workspace::validate_behavior_workspace(&project, &behavior, behavior_diagrams)
        .map_err(|cause| error("PRESENTATION_VALIDATION", None, cause))?;
    Ok(UnifiedCandidate {
        project,
        diagrams,
        activities,
        behavior,
        result,
    })
}

pub fn preview_unified_model_build(
    plan: &ModelBuildPlan,
    state: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
) -> ModelBuildPreview {
    let proposed_operations = proposed_operations(plan);
    let candidate = (|| {
        let project = state
            .project
            .lock()
            .map_err(|_| error("LOCK_FAILURE", None, "project lock poisoned"))?
            .clone()
            .ok_or_else(|| error("NO_PROJECT", None, "no project open"))?;
        let diagrams = state
            .diagrams
            .lock()
            .map_err(|_| error("LOCK_FAILURE", None, "diagram lock poisoned"))?
            .clone();
        let behavior = state
            .behavior
            .lock()
            .map_err(|_| error("LOCK_FAILURE", None, "behavior lock poisoned"))?
            .clone();
        let behavior_diagrams = state
            .behavior_diagrams
            .lock()
            .map_err(|_| error("LOCK_FAILURE", None, "behavior diagram lock poisoned"))?
            .clone();
        let activities = activity
            .repository
            .lock()
            .map_err(|_| error("LOCK_FAILURE", None, "Activity repository lock poisoned"))?
            .clone();
        let activity_diagrams = activity
            .diagrams
            .lock()
            .map_err(|_| error("LOCK_FAILURE", None, "Activity diagram lock poisoned"))?
            .clone();
        build_unified_candidate(
            plan,
            project,
            diagrams,
            activities,
            &activity_diagrams,
            behavior,
            &behavior_diagrams,
        )
    })();
    ModelBuildPreview {
        proposed_operations,
        diagnostics: candidate.err().into_iter().collect(),
    }
}

pub fn apply_unified_model_build(
    plan: &ModelBuildPlan,
    state: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
) -> Result<ModelBuildResult, ModelBuildPreview> {
    let proposed = proposed_operations(plan);
    let mut project_guard = state.project.lock().map_err(|_| ModelBuildPreview {
        proposed_operations: proposed.clone(),
        diagnostics: vec![error("LOCK_FAILURE", None, "project lock poisoned")],
    })?;
    let mut diagram_guard = state.diagrams.lock().map_err(|_| ModelBuildPreview {
        proposed_operations: proposed.clone(),
        diagnostics: vec![error("LOCK_FAILURE", None, "diagram lock poisoned")],
    })?;
    let mut behavior_guard = state.behavior.lock().map_err(|_| ModelBuildPreview {
        proposed_operations: proposed.clone(),
        diagnostics: vec![error("LOCK_FAILURE", None, "behavior lock poisoned")],
    })?;
    let behavior_diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| ModelBuildPreview {
            proposed_operations: proposed.clone(),
            diagnostics: vec![error(
                "LOCK_FAILURE",
                None,
                "behavior diagram lock poisoned",
            )],
        })?;
    let mut activities_guard = activity.repository.lock().map_err(|_| ModelBuildPreview {
        proposed_operations: proposed.clone(),
        diagnostics: vec![error(
            "LOCK_FAILURE",
            None,
            "Activity repository lock poisoned",
        )],
    })?;
    let activity_diagrams = activity.diagrams.lock().map_err(|_| ModelBuildPreview {
        proposed_operations: proposed.clone(),
        diagnostics: vec![error(
            "LOCK_FAILURE",
            None,
            "Activity diagram lock poisoned",
        )],
    })?;
    let project = project_guard.clone().ok_or_else(|| ModelBuildPreview {
        proposed_operations: proposed.clone(),
        diagnostics: vec![error("NO_PROJECT", None, "no project open")],
    })?;
    let candidate = build_unified_candidate(
        plan,
        project,
        diagram_guard.clone(),
        activities_guard.clone(),
        &activity_diagrams,
        behavior_guard.clone(),
        &behavior_diagrams,
    )
    .map_err(|diagnostic| ModelBuildPreview {
        proposed_operations: proposed,
        diagnostics: vec![diagnostic],
    })?;
    *project_guard = Some(candidate.project);
    *diagram_guard = candidate.diagrams;
    *activities_guard = candidate.activities;
    *behavior_guard = candidate.behavior;
    Ok(candidate.result)
}

#[cfg(test)]
pub(super) fn activity_identity_for_external(
    repository: &ActivityRepository,
    key: &str,
) -> Option<ActivitySemanticId> {
    repository.external_ids.get(key).copied()
}
#[cfg(test)]
pub(super) fn behavior_identity_for_external(
    repository: &BehaviorRepository,
    key: &str,
) -> Option<BehaviorSemanticId> {
    repository.external_ids.get(key).copied()
}
