use crate::{ElementId, ElementKind, ModelError, Multiplicity, ParameterDirection, Project};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

macro_rules! activity_id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);
        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }
        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

activity_id_type!(ActivityId);
activity_id_type!(ActivityNodeId);
activity_id_type!(ActivityEdgeId);
activity_id_type!(PinId);
activity_id_type!(ActivityPartitionId);
activity_id_type!(StructuredNodeId);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActivityRepository {
    pub activities: HashMap<ActivityId, Activity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: ActivityId,
    pub external_id: String,
    pub name: String,
    pub owner_id: ElementId,
    pub context_id: Option<ElementId>,
    pub nodes: Vec<ActivityNode>,
    pub edges: Vec<ActivityEdge>,
    pub partitions: Vec<ActivityPartition>,
    pub structured_nodes: Vec<StructuredActivityNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityNode {
    pub id: ActivityNodeId,
    pub name: String,
    pub kind: ActivityNodeKind,
    pub partition_id: Option<ActivityPartitionId>,
    pub structured_node_id: Option<StructuredNodeId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityNodeKind {
    Initial,
    ActivityFinal,
    FlowFinal,
    Decision { decision_input: Option<String> },
    Merge,
    Fork,
    Join { join_specification: Option<String> },
    Action(Action),
    Object(ObjectNode),
    ActivityParameter(ActivityParameterNode),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub kind: ActionKind,
    pub pins: Vec<Pin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionKind {
    Opaque { body: String },
    CallBehavior { activity_id: ActivityId },
    CallOperation { operation_id: ElementId },
    SendSignal { signal_id: ElementId },
    AcceptEvent { signal_id: Option<ElementId> },
    AcceptTimeEvent { expression: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinDirection {
    Input,
    Output,
    Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub id: PinId,
    pub name: String,
    pub direction: PinDirection,
    pub type_id: Option<ElementId>,
    pub multiplicity: Multiplicity,
    pub is_ordered: bool,
    pub is_unique: bool,
    pub value: Option<String>,
    pub parameter_id: Option<ElementId>,
}

impl Pin {
    pub fn input(name: impl Into<String>, type_id: Option<ElementId>) -> Self {
        Self {
            id: PinId::new(),
            name: name.into(),
            direction: PinDirection::Input,
            type_id,
            multiplicity: Multiplicity::ONE,
            is_ordered: false,
            is_unique: true,
            value: None,
            parameter_id: None,
        }
    }

    pub fn output(name: impl Into<String>, type_id: Option<ElementId>) -> Self {
        Self {
            direction: PinDirection::Output,
            ..Self::input(name, type_id)
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectNode {
    pub kind: ObjectNodeKind,
    pub type_id: Option<ElementId>,
    pub multiplicity: Multiplicity,
    pub ordering: ObjectNodeOrdering,
    pub selection: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectNodeKind {
    Object,
    CentralBuffer,
    DataStore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ObjectNodeOrdering {
    #[default]
    Unordered,
    Ordered,
    Fifo,
    Lifo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityParameterNode {
    pub parameter_id: ElementId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActivityEndpoint {
    Node(ActivityNodeId),
    Pin(PinId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityEdgeKind {
    ControlFlow,
    ObjectFlow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityEdge {
    pub id: ActivityEdgeId,
    pub name: String,
    pub kind: ActivityEdgeKind,
    pub source: ActivityEndpoint,
    pub target: ActivityEndpoint,
    pub guard: Option<String>,
    pub weight: Option<String>,
    pub selection: Option<String>,
    pub transformation: Option<String>,
    pub interrupting_region_id: Option<StructuredNodeId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityPartition {
    pub id: ActivityPartitionId,
    pub name: String,
    pub represented_element_id: Option<ElementId>,
    pub is_dimension: bool,
    pub is_external: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StructuredActivityNodeKind {
    Structured,
    Conditional,
    Loop,
    Sequence,
    ExpansionRegion,
    InterruptibleRegion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredActivityNode {
    pub id: StructuredNodeId,
    pub name: String,
    pub kind: StructuredActivityNodeKind,
    pub parent_id: Option<StructuredNodeId>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ActivityError {
    #[error("activity owner must be a namespace or classifier: {0}")]
    InvalidOwner(ElementId),
    #[error("activity context must be a classifier: {0}")]
    InvalidContext(ElementId),
    #[error("activity contains duplicate node identity")]
    DuplicateNode,
    #[error("activity contains duplicate pin identity")]
    DuplicatePin,
    #[error("activity edge references an unknown endpoint")]
    UnknownEndpoint,
    #[error("activity node references an unknown partition")]
    UnknownPartition,
    #[error("activity node references an unknown structured node")]
    UnknownStructuredNode,
    #[error("structured activity node references an unknown parent")]
    UnknownStructuredParent,
    #[error("structured activity node containment contains a cycle")]
    StructuredContainmentCycle,
    #[error("initial node cannot have incoming edges and must have exactly one outgoing control flow")]
    InvalidInitialTopology,
    #[error("activity final and flow final nodes cannot have outgoing edges")]
    InvalidFinalTopology,
    #[error("fork node requires exactly one incoming and at least two outgoing control flows")]
    InvalidForkTopology,
    #[error("join node requires at least two incoming and exactly one outgoing control flow")]
    InvalidJoinTopology,
    #[error("decision requires one incoming and at least one outgoing edge")]
    InvalidDecisionTopology,
    #[error("merge requires at least one incoming and exactly one outgoing edge")]
    InvalidMergeTopology,
    #[error("control flow cannot connect directly to pins")]
    ControlFlowPinEndpoint,
    #[error("object flow must connect object-capable endpoints")]
    InvalidObjectFlowEndpoint,
    #[error("object flow source and target types are incompatible")]
    ObjectFlowTypeMismatch,
    #[error("object flow direction is invalid for the selected pin endpoint")]
    InvalidPinDirection,
    #[error("action references an unknown activity")]
    UnknownCalledActivity,
    #[error("call operation action must reference an Operation: {0}")]
    InvalidOperation(ElementId),
    #[error("signal action must reference a Signal: {0}")]
    InvalidSignal(ElementId),
    #[error("pin references an invalid semantic type: {0}")]
    InvalidPinType(ElementId),
    #[error("call operation pin does not match the referenced operation parameter: {0}")]
    InvalidOperationPin(ElementId),
    #[error("activity parameter node must reference a Parameter: {0}")]
    InvalidActivityParameter(ElementId),
    #[error("interrupting edge must reference an InterruptibleActivityRegion")]
    InvalidInterruptingRegion,
    #[error(transparent)]
    Model(#[from] ModelError),
}

impl ActivityRepository {
    pub fn create_activity(
        &mut self,
        project: &Project,
        owner_id: ElementId,
        context_id: Option<ElementId>,
        name: impl Into<String>,
    ) -> Result<ActivityId, ActivityError> {
        let owner = project.element(owner_id)?;
        if !owner.is_namespace() && !owner.is_classifier() {
            return Err(ActivityError::InvalidOwner(owner_id));
        }
        if let Some(context_id) = context_id {
            let context = project.element(context_id)?;
            if !context.is_classifier() {
                return Err(ActivityError::InvalidContext(context_id));
            }
        }
        let id = ActivityId::new();
        self.activities.insert(
            id,
            Activity {
                id,
                external_id: format!("ACT-{id}"),
                name: name.into(),
                owner_id,
                context_id,
                nodes: Vec::new(),
                edges: Vec::new(),
                partitions: Vec::new(),
                structured_nodes: Vec::new(),
            },
        );
        Ok(id)
    }

    pub fn validate(&self, project: &Project) -> Result<(), ActivityError> {
        for activity in self.activities.values() {
            validate_activity(self, project, activity)?;
        }
        Ok(())
    }
}

pub fn validate_activity(
    repository: &ActivityRepository,
    project: &Project,
    activity: &Activity,
) -> Result<(), ActivityError> {
    let owner = project.element(activity.owner_id)?;
    if !owner.is_namespace() && !owner.is_classifier() {
        return Err(ActivityError::InvalidOwner(activity.owner_id));
    }
    if let Some(context_id) = activity.context_id
        && !project.element(context_id)?.is_classifier()
    {
        return Err(ActivityError::InvalidContext(context_id));
    }

    validate_structure(activity)?;
    validate_references(repository, project, activity)?;
    validate_edges(project, activity)?;
    validate_control_topology(activity)?;
    Ok(())
}

fn validate_structure(activity: &Activity) -> Result<(), ActivityError> {
    let mut node_ids = HashSet::new();
    let mut pin_ids = HashSet::new();
    let partition_ids: HashSet<_> = activity.partitions.iter().map(|p| p.id).collect();
    let structured_ids: HashSet<_> = activity.structured_nodes.iter().map(|n| n.id).collect();

    for node in &activity.nodes {
        if !node_ids.insert(node.id) {
            return Err(ActivityError::DuplicateNode);
        }
        if node.partition_id.is_some_and(|id| !partition_ids.contains(&id)) {
            return Err(ActivityError::UnknownPartition);
        }
        if node
            .structured_node_id
            .is_some_and(|id| !structured_ids.contains(&id))
        {
            return Err(ActivityError::UnknownStructuredNode);
        }
        if let ActivityNodeKind::Action(action) = &node.kind {
            for pin in &action.pins {
                if !pin_ids.insert(pin.id) {
                    return Err(ActivityError::DuplicatePin);
                }
            }
        }
    }

    for structured in &activity.structured_nodes {
        if structured
            .parent_id
            .is_some_and(|parent| !structured_ids.contains(&parent))
        {
            return Err(ActivityError::UnknownStructuredParent);
        }
        let mut seen = HashSet::new();
        let mut current = structured.parent_id;
        while let Some(parent) = current {
            if !seen.insert(parent) || parent == structured.id {
                return Err(ActivityError::StructuredContainmentCycle);
            }
            current = activity
                .structured_nodes
                .iter()
                .find(|candidate| candidate.id == parent)
                .and_then(|candidate| candidate.parent_id);
        }
    }
    Ok(())
}

fn validate_references(
    repository: &ActivityRepository,
    project: &Project,
    activity: &Activity,
) -> Result<(), ActivityError> {
    for node in &activity.nodes {
        match &node.kind {
            ActivityNodeKind::Action(action) => {
                validate_action(repository, project, action)?;
            }
            ActivityNodeKind::Object(object) => {
                if let Some(type_id) = object.type_id {
                    ensure_data_type(project, type_id)?;
                }
            }
            ActivityNodeKind::ActivityParameter(parameter) => {
                let element = project.element(parameter.parameter_id)?;
                if element.kind != ElementKind::Parameter {
                    return Err(ActivityError::InvalidActivityParameter(parameter.parameter_id));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_action(
    repository: &ActivityRepository,
    project: &Project,
    action: &Action,
) -> Result<(), ActivityError> {
    match action.kind {
        ActionKind::CallBehavior { activity_id } => {
            if !repository.activities.contains_key(&activity_id) {
                return Err(ActivityError::UnknownCalledActivity);
            }
        }
        ActionKind::CallOperation { operation_id } => {
            if project.element(operation_id)?.kind != ElementKind::Operation {
                return Err(ActivityError::InvalidOperation(operation_id));
            }
            validate_operation_pins(project, operation_id, &action.pins)?;
        }
        ActionKind::SendSignal { signal_id }
        | ActionKind::AcceptEvent {
            signal_id: Some(signal_id),
        } => {
            if project.element(signal_id)?.kind != ElementKind::Signal {
                return Err(ActivityError::InvalidSignal(signal_id));
            }
        }
        ActionKind::AcceptEvent { signal_id: None }
        | ActionKind::Opaque { .. }
        | ActionKind::AcceptTimeEvent { .. } => {}
    }
    for pin in &action.pins {
        if let Some(type_id) = pin.type_id {
            ensure_data_type(project, type_id)?;
        }
    }
    Ok(())
}

fn validate_operation_pins(
    project: &Project,
    operation_id: ElementId,
    pins: &[Pin],
) -> Result<(), ActivityError> {
    for pin in pins {
        let Some(parameter_id) = pin.parameter_id else {
            continue;
        };
        let parameter = project.element(parameter_id)?;
        if parameter.kind != ElementKind::Parameter || parameter.owner_id != Some(operation_id) {
            return Err(ActivityError::InvalidOperationPin(parameter_id));
        }
        if pin.type_id != parameter.type_id {
            return Err(ActivityError::InvalidOperationPin(parameter_id));
        }
        let expected = match parameter.parameter_direction {
            Some(ParameterDirection::Out | ParameterDirection::Return) => PinDirection::Output,
            Some(ParameterDirection::In | ParameterDirection::InOut) | None => PinDirection::Input,
        };
        if pin.direction != expected && pin.direction != PinDirection::Value {
            return Err(ActivityError::InvalidOperationPin(parameter_id));
        }
    }
    Ok(())
}

fn ensure_data_type(project: &Project, id: ElementId) -> Result<(), ActivityError> {
    let element = project.element(id)?;
    if !element.is_classifier() {
        return Err(ActivityError::InvalidPinType(id));
    }
    Ok(())
}

fn validate_edges(project: &Project, activity: &Activity) -> Result<(), ActivityError> {
    for edge in &activity.edges {
        let source = endpoint_info(activity, edge.source)?;
        let target = endpoint_info(activity, edge.target)?;
        match edge.kind {
            ActivityEdgeKind::ControlFlow => {
                if source.pin.is_some() || target.pin.is_some() {
                    return Err(ActivityError::ControlFlowPinEndpoint);
                }
            }
            ActivityEdgeKind::ObjectFlow => {
                if !source.object_capable || !target.object_capable {
                    return Err(ActivityError::InvalidObjectFlowEndpoint);
                }
                if source.pin.is_some_and(|pin| pin.direction == PinDirection::Input)
                    || target.pin.is_some_and(|pin| pin.direction == PinDirection::Output)
                {
                    return Err(ActivityError::InvalidPinDirection);
                }
                if let (Some(source_type), Some(target_type)) = (source.type_id, target.type_id)
                    && source_type != target_type
                {
                    let source_element = project.element(source_type)?;
                    let target_element = project.element(target_type)?;
                    if source_element.kind != target_element.kind {
                        return Err(ActivityError::ObjectFlowTypeMismatch);
                    }
                    return Err(ActivityError::ObjectFlowTypeMismatch);
                }
            }
        }
        if let Some(region_id) = edge.interrupting_region_id {
            let region = activity
                .structured_nodes
                .iter()
                .find(|node| node.id == region_id)
                .ok_or(ActivityError::InvalidInterruptingRegion)?;
            if region.kind != StructuredActivityNodeKind::InterruptibleRegion {
                return Err(ActivityError::InvalidInterruptingRegion);
            }
        }
    }
    Ok(())
}

struct EndpointInfo<'a> {
    pin: Option<&'a Pin>,
    object_capable: bool,
    type_id: Option<ElementId>,
}

fn endpoint_info(
    activity: &Activity,
    endpoint: ActivityEndpoint,
) -> Result<EndpointInfo<'_>, ActivityError> {
    match endpoint {
        ActivityEndpoint::Node(id) => {
            let node = activity
                .nodes
                .iter()
                .find(|node| node.id == id)
                .ok_or(ActivityError::UnknownEndpoint)?;
            match &node.kind {
                ActivityNodeKind::Object(object) => Ok(EndpointInfo {
                    pin: None,
                    object_capable: true,
                    type_id: object.type_id,
                }),
                ActivityNodeKind::ActivityParameter(parameter) => Ok(EndpointInfo {
                    pin: None,
                    object_capable: true,
                    type_id: None.or_else(|| {
                        let _ = parameter;
                        None
                    }),
                }),
                _ => Ok(EndpointInfo {
                    pin: None,
                    object_capable: false,
                    type_id: None,
                }),
            }
        }
        ActivityEndpoint::Pin(id) => {
            let pin = activity
                .nodes
                .iter()
                .filter_map(|node| match &node.kind {
                    ActivityNodeKind::Action(action) => Some(&action.pins),
                    _ => None,
                })
                .flatten()
                .find(|pin| pin.id == id)
                .ok_or(ActivityError::UnknownEndpoint)?;
            Ok(EndpointInfo {
                pin: Some(pin),
                object_capable: true,
                type_id: pin.type_id,
            })
        }
    }
}

fn validate_control_topology(activity: &Activity) -> Result<(), ActivityError> {
    for node in &activity.nodes {
        let incoming: Vec<_> = activity
            .edges
            .iter()
            .filter(|edge| edge.target == ActivityEndpoint::Node(node.id))
            .collect();
        let outgoing: Vec<_> = activity
            .edges
            .iter()
            .filter(|edge| edge.source == ActivityEndpoint::Node(node.id))
            .collect();
        match node.kind {
            ActivityNodeKind::Initial => {
                if !incoming.is_empty()
                    || outgoing.len() != 1
                    || outgoing[0].kind != ActivityEdgeKind::ControlFlow
                {
                    return Err(ActivityError::InvalidInitialTopology);
                }
            }
            ActivityNodeKind::ActivityFinal | ActivityNodeKind::FlowFinal => {
                if !outgoing.is_empty() {
                    return Err(ActivityError::InvalidFinalTopology);
                }
            }
            ActivityNodeKind::Fork => {
                if incoming.len() != 1
                    || outgoing.len() < 2
                    || incoming
                        .iter()
                        .chain(outgoing.iter())
                        .any(|edge| edge.kind != ActivityEdgeKind::ControlFlow)
                {
                    return Err(ActivityError::InvalidForkTopology);
                }
            }
            ActivityNodeKind::Join { .. } => {
                if incoming.len() < 2
                    || outgoing.len() != 1
                    || incoming
                        .iter()
                        .chain(outgoing.iter())
                        .any(|edge| edge.kind != ActivityEdgeKind::ControlFlow)
                {
                    return Err(ActivityError::InvalidJoinTopology);
                }
            }
            ActivityNodeKind::Decision { .. } => {
                if incoming.len() != 1 || outgoing.is_empty() {
                    return Err(ActivityError::InvalidDecisionTopology);
                }
            }
            ActivityNodeKind::Merge => {
                if incoming.is_empty() || outgoing.len() != 1 {
                    return Err(ActivityError::InvalidMergeTopology);
                }
            }
            _ => {}
        }
    }
    Ok(())
}
