use crate::{ElementId, ElementKind, ModelError, Project};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

macro_rules! behavior_id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);
        impl $name {
            pub fn new() -> Self { Self(Uuid::new_v4()) }
        }
        impl Default for $name {
            fn default() -> Self { Self::new() }
        }
        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { self.0.fmt(f) }
        }
    };
}

behavior_id_type!(StateMachineId);
behavior_id_type!(RegionId);
behavior_id_type!(VertexId);
behavior_id_type!(TransitionId);
behavior_id_type!(InteractionId);
behavior_id_type!(LifelineId);
behavior_id_type!(OccurrenceId);
behavior_id_type!(MessageId);
behavior_id_type!(ExecutionId);
behavior_id_type!(FragmentId);
behavior_id_type!(OperandId);
behavior_id_type!(InvariantId);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorRepository {
    pub state_machines: HashMap<StateMachineId, StateMachine>,
    pub interactions: HashMap<InteractionId, Interaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateMachine {
    pub id: StateMachineId,
    pub external_id: String,
    pub name: String,
    pub context_id: ElementId,
    pub regions: Vec<Region>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    pub id: RegionId,
    pub name: String,
    pub vertices: Vec<Vertex>,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vertex {
    pub id: VertexId,
    pub name: String,
    pub kind: VertexKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VertexKind {
    State(State),
    FinalState,
    Pseudostate(PseudostateKind),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    pub entry: Option<String>,
    pub do_activity: Option<String>,
    pub exit: Option<String>,
    pub regions: Vec<Region>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PseudostateKind {
    Initial,
    Choice,
    Junction,
    Fork,
    Join,
    ShallowHistory,
    DeepHistory,
    EntryPoint,
    ExitPoint,
    Terminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransitionKind {
    External,
    Internal,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub id: TransitionId,
    pub source_id: VertexId,
    pub target_id: VertexId,
    pub kind: TransitionKind,
    pub trigger: Option<Trigger>,
    pub guard: Option<String>,
    pub effect: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    pub event: Event,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Signal { signal_id: ElementId },
    Call { operation_id: ElementId },
    Time { expression: String, is_relative: bool },
    Change { expression: String },
    AnyReceive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    pub id: InteractionId,
    pub external_id: String,
    pub name: String,
    pub context_id: ElementId,
    pub lifelines: Vec<Lifeline>,
    pub messages: Vec<Message>,
    pub executions: Vec<ExecutionSpecification>,
    pub fragments: Vec<CombinedFragment>,
    pub state_invariants: Vec<StateInvariant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lifeline {
    pub id: LifelineId,
    pub name: String,
    /// Stable semantic property path from the interaction context to the represented role.
    pub represented_path: Vec<ElementId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Occurrence {
    pub id: OccurrenceId,
    pub lifeline_id: LifelineId,
    /// Monotonic semantic vertical ordering value, independent of pixels.
    pub order: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageSort {
    SynchCall,
    AsynchCall,
    AsynchSignal,
    Reply,
    Create,
    Delete,
    Lost,
    Found,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageSignature {
    Operation(ElementId),
    Signal(ElementId),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: MessageId,
    pub name: String,
    pub sort: MessageSort,
    pub send_event: Option<Occurrence>,
    pub receive_event: Option<Occurrence>,
    pub signature: Option<MessageSignature>,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSpecification {
    pub id: ExecutionId,
    pub lifeline_id: LifelineId,
    pub start: Occurrence,
    pub finish: Occurrence,
    pub behavior_id: Option<ElementId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionOperator {
    Alt,
    Opt,
    Loop,
    Break,
    Par,
    Critical,
    Neg,
    Assert,
    Strict,
    Seq,
    Ignore,
    Consider,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionOperand {
    pub id: OperandId,
    pub guard: Option<String>,
    pub start_order: u32,
    pub end_order: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedFragment {
    pub id: FragmentId,
    pub operator: InteractionOperator,
    pub covered_lifelines: Vec<LifelineId>,
    pub operands: Vec<InteractionOperand>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateInvariant {
    pub id: InvariantId,
    pub lifeline_id: LifelineId,
    pub order: u32,
    pub constraint: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BehaviorError {
    #[error("behavior context must be a classifier: {0}")]
    InvalidContext(ElementId),
    #[error("state machine region may contain at most one initial pseudostate")]
    MultipleInitialVertices,
    #[error("initial pseudostate cannot have an incoming transition")]
    InitialHasIncoming,
    #[error("initial transition must be triggerless and guardless")]
    InitialTransitionHasTriggerOrGuard,
    #[error("initial pseudostate must have exactly one outgoing transition")]
    InitialRequiresOneOutgoing,
    #[error("final state cannot have outgoing transitions")]
    FinalStateHasOutgoing,
    #[error("fork requires exactly one incoming and at least two outgoing transitions")]
    InvalidFork,
    #[error("join requires at least two incoming and exactly one outgoing transition")]
    InvalidJoin,
    #[error("transition references a vertex that does not exist in the state machine")]
    UnknownTransitionVertex,
    #[error("signal trigger/signature must reference a Signal: {0}")]
    InvalidSignal(ElementId),
    #[error("call trigger/message signature must reference an Operation: {0}")]
    InvalidOperation(ElementId),
    #[error("lifeline represented path is invalid for interaction context")]
    InvalidLifelinePath,
    #[error("message occurrence references an unknown lifeline")]
    UnknownLifeline,
    #[error("message sort requires a send occurrence")]
    MessageRequiresSend,
    #[error("message sort requires a receive occurrence")]
    MessageRequiresReceive,
    #[error("found message must omit the send occurrence and include a receive occurrence")]
    InvalidFoundMessage,
    #[error("lost message must include the send occurrence and omit the receive occurrence")]
    InvalidLostMessage,
    #[error("synchronous/asynchronous call message requires an Operation signature")]
    CallMessageRequiresOperation,
    #[error("asynchronous signal message requires a Signal signature")]
    SignalMessageRequiresSignal,
    #[error("execution start/finish must reference the execution lifeline and increase in order")]
    InvalidExecution,
    #[error("combined fragment references an unknown lifeline")]
    FragmentUnknownLifeline,
    #[error("alt combined fragment requires at least two operands")]
    AltRequiresTwoOperands,
    #[error("combined fragment operand end must be after its start")]
    InvalidOperandRange,
    #[error("state invariant references an unknown lifeline")]
    InvariantUnknownLifeline,
    #[error("state invariant constraint cannot be empty")]
    EmptyStateInvariant,
    #[error(transparent)]
    Model(#[from] ModelError),
}

impl BehaviorRepository {
    pub fn create_state_machine(&mut self, project: &Project, context_id: ElementId, name: impl Into<String>) -> Result<StateMachineId, BehaviorError> {
        ensure_behavior_context(project, context_id)?;
        let id = StateMachineId::new();
        self.state_machines.insert(id, StateMachine {
            id,
            external_id: format!("SM-{id}"),
            name: name.into(),
            context_id,
            regions: vec![Region { id: RegionId::new(), name: "Region1".into(), vertices: Vec::new(), transitions: Vec::new() }],
        });
        Ok(id)
    }

    pub fn create_interaction(&mut self, project: &Project, context_id: ElementId, name: impl Into<String>) -> Result<InteractionId, BehaviorError> {
        ensure_behavior_context(project, context_id)?;
        let id = InteractionId::new();
        self.interactions.insert(id, Interaction {
            id,
            external_id: format!("INT-{id}"),
            name: name.into(),
            context_id,
            lifelines: Vec::new(), messages: Vec::new(), executions: Vec::new(), fragments: Vec::new(), state_invariants: Vec::new(),
        });
        Ok(id)
    }

    pub fn validate(&self, project: &Project) -> Result<(), BehaviorError> {
        for machine in self.state_machines.values() { validate_state_machine(project, machine)?; }
        for interaction in self.interactions.values() { validate_interaction(project, interaction)?; }
        Ok(())
    }
}

fn ensure_behavior_context(project: &Project, context_id: ElementId) -> Result<(), BehaviorError> {
    let context = project.element(context_id)?;
    if !context.is_classifier() { return Err(BehaviorError::InvalidContext(context_id)); }
    Ok(())
}

pub fn validate_state_machine(project: &Project, machine: &StateMachine) -> Result<(), BehaviorError> {
    ensure_behavior_context(project, machine.context_id)?;
    let mut vertices = HashMap::new();
    collect_vertices(&machine.regions, &mut vertices);
    for region in &machine.regions { validate_region(project, region, &vertices)?; }
    Ok(())
}

fn collect_vertices<'a>(regions: &'a [Region], out: &mut HashMap<VertexId, &'a Vertex>) {
    for region in regions {
        for vertex in &region.vertices {
            out.insert(vertex.id, vertex);
            if let VertexKind::State(state) = &vertex.kind { collect_vertices(&state.regions, out); }
        }
    }
}

fn validate_region(project: &Project, region: &Region, vertices: &HashMap<VertexId, &Vertex>) -> Result<(), BehaviorError> {
    let initials: Vec<_> = region.vertices.iter().filter(|v| matches!(v.kind, VertexKind::Pseudostate(PseudostateKind::Initial))).collect();
    if initials.len() > 1 { return Err(BehaviorError::MultipleInitialVertices); }
    for transition in &region.transitions {
        let source = vertices.get(&transition.source_id).ok_or(BehaviorError::UnknownTransitionVertex)?;
        vertices.get(&transition.target_id).ok_or(BehaviorError::UnknownTransitionVertex)?;
        validate_trigger(project, transition.trigger.as_ref())?;
        if matches!(source.kind, VertexKind::Pseudostate(PseudostateKind::Initial)) && (transition.trigger.is_some() || transition.guard.as_ref().is_some_and(|g| !g.trim().is_empty())) {
            return Err(BehaviorError::InitialTransitionHasTriggerOrGuard);
        }
    }
    for vertex in &region.vertices {
        let incoming = region.transitions.iter().filter(|t| t.target_id == vertex.id).count();
        let outgoing = region.transitions.iter().filter(|t| t.source_id == vertex.id).count();
        match &vertex.kind {
            VertexKind::Pseudostate(PseudostateKind::Initial) => {
                if incoming != 0 { return Err(BehaviorError::InitialHasIncoming); }
                if outgoing != 1 { return Err(BehaviorError::InitialRequiresOneOutgoing); }
            }
            VertexKind::Pseudostate(PseudostateKind::Fork) if incoming != 1 || outgoing < 2 => return Err(BehaviorError::InvalidFork),
            VertexKind::Pseudostate(PseudostateKind::Join) if incoming < 2 || outgoing != 1 => return Err(BehaviorError::InvalidJoin),
            VertexKind::FinalState if outgoing != 0 => return Err(BehaviorError::FinalStateHasOutgoing),
            VertexKind::State(state) => for child in &state.regions { validate_region(project, child, vertices)?; },
            _ => {}
        }
    }
    Ok(())
}

fn validate_trigger(project: &Project, trigger: Option<&Trigger>) -> Result<(), BehaviorError> {
    let Some(trigger) = trigger else { return Ok(()); };
    match trigger.event {
        Event::Signal { signal_id } => {
            if project.element(signal_id)?.kind != ElementKind::Signal { return Err(BehaviorError::InvalidSignal(signal_id)); }
        }
        Event::Call { operation_id } => {
            if project.element(operation_id)?.kind != ElementKind::Operation { return Err(BehaviorError::InvalidOperation(operation_id)); }
        }
        Event::Time { ref expression, .. } | Event::Change { ref expression } if expression.trim().is_empty() => return Err(BehaviorError::EmptyStateInvariant),
        _ => {}
    }
    Ok(())
}

pub fn validate_interaction(project: &Project, interaction: &Interaction) -> Result<(), BehaviorError> {
    ensure_behavior_context(project, interaction.context_id)?;
    let lifelines: HashSet<_> = interaction.lifelines.iter().map(|l| l.id).collect();
    for lifeline in &interaction.lifelines {
        if lifeline.represented_path.is_empty() || project.resolve_structural_path(interaction.context_id, &lifeline.represented_path).is_err() {
            return Err(BehaviorError::InvalidLifelinePath);
        }
    }
    for message in &interaction.messages { validate_message(project, message, &lifelines)?; }
    for execution in &interaction.executions {
        if !lifelines.contains(&execution.lifeline_id)
            || execution.start.lifeline_id != execution.lifeline_id
            || execution.finish.lifeline_id != execution.lifeline_id
            || execution.start.order >= execution.finish.order { return Err(BehaviorError::InvalidExecution); }
    }
    for fragment in &interaction.fragments {
        if fragment.covered_lifelines.iter().any(|id| !lifelines.contains(id)) { return Err(BehaviorError::FragmentUnknownLifeline); }
        if fragment.operator == InteractionOperator::Alt && fragment.operands.len() < 2 { return Err(BehaviorError::AltRequiresTwoOperands); }
        if fragment.operands.iter().any(|op| op.start_order >= op.end_order) { return Err(BehaviorError::InvalidOperandRange); }
    }
    for invariant in &interaction.state_invariants {
        if !lifelines.contains(&invariant.lifeline_id) { return Err(BehaviorError::InvariantUnknownLifeline); }
        if invariant.constraint.trim().is_empty() { return Err(BehaviorError::EmptyStateInvariant); }
    }
    Ok(())
}

fn validate_occurrence(occurrence: &Occurrence, lifelines: &HashSet<LifelineId>) -> Result<(), BehaviorError> {
    if !lifelines.contains(&occurrence.lifeline_id) { return Err(BehaviorError::UnknownLifeline); }
    Ok(())
}

fn validate_message(project: &Project, message: &Message, lifelines: &HashSet<LifelineId>) -> Result<(), BehaviorError> {
    if let Some(send) = &message.send_event { validate_occurrence(send, lifelines)?; }
    if let Some(receive) = &message.receive_event { validate_occurrence(receive, lifelines)?; }
    match message.sort {
        MessageSort::Found if message.send_event.is_some() || message.receive_event.is_none() => return Err(BehaviorError::InvalidFoundMessage),
        MessageSort::Lost if message.send_event.is_none() || message.receive_event.is_some() => return Err(BehaviorError::InvalidLostMessage),
        MessageSort::Found | MessageSort::Lost => {}
        _ if message.send_event.is_none() => return Err(BehaviorError::MessageRequiresSend),
        _ if message.receive_event.is_none() => return Err(BehaviorError::MessageRequiresReceive),
        _ => {}
    }
    match message.sort {
        MessageSort::SynchCall | MessageSort::AsynchCall => match message.signature {
            Some(MessageSignature::Operation(id)) if project.element(id)?.kind == ElementKind::Operation => {}
            _ => return Err(BehaviorError::CallMessageRequiresOperation),
        },
        MessageSort::AsynchSignal => match message.signature {
            Some(MessageSignature::Signal(id)) if project.element(id)?.kind == ElementKind::Signal => {}
            _ => return Err(BehaviorError::SignalMessageRequiresSignal),
        },
        _ => {}
    }
    if let Some(MessageSignature::Operation(id)) = message.signature {
        if project.element(id)?.kind != ElementKind::Operation { return Err(BehaviorError::InvalidOperation(id)); }
    }
    if let Some(MessageSignature::Signal(id)) = message.signature {
        if project.element(id)?.kind != ElementKind::Signal { return Err(BehaviorError::InvalidSignal(id)); }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Multiplicity, Project};

    #[test]
    fn initial_transition_rejects_trigger_and_guard() {
        let mut project = Project::new("P");
        let block = project.create_element(ElementKind::Block, "System", project.root_id).unwrap();
        let signal = project.create_element(ElementKind::Signal, "Start", project.root_id).unwrap();
        let mut repo = BehaviorRepository::default();
        let id = repo.create_state_machine(&project, block, "Lifecycle").unwrap();
        let machine = repo.state_machines.get_mut(&id).unwrap();
        let region = &mut machine.regions[0];
        let initial = Vertex { id: VertexId::new(), name: String::new(), kind: VertexKind::Pseudostate(PseudostateKind::Initial) };
        let state = Vertex { id: VertexId::new(), name: "Ready".into(), kind: VertexKind::State(State::default()) };
        region.transitions.push(Transition { id: TransitionId::new(), source_id: initial.id, target_id: state.id, kind: TransitionKind::External, trigger: Some(Trigger { event: Event::Signal { signal_id: signal } }), guard: None, effect: None });
        region.vertices.extend([initial, state]);
        assert_eq!(repo.validate(&project), Err(BehaviorError::InitialTransitionHasTriggerOrGuard));
    }

    #[test]
    fn sequence_lifeline_uses_context_property_path_and_call_signature() {
        let mut project = Project::new("P");
        let system = project.create_element(ElementKind::Block, "System", project.root_id).unwrap();
        let component_type = project.create_element(ElementKind::Block, "Component", project.root_id).unwrap();
        let part = project.create_typed_feature(ElementKind::PartProperty, "component", system, component_type, Multiplicity::ONE).unwrap();
        let operation = project.create_element(ElementKind::Operation, "run", component_type).unwrap();
        let mut repo = BehaviorRepository::default();
        let interaction_id = repo.create_interaction(&project, system, "Nominal").unwrap();
        let interaction = repo.interactions.get_mut(&interaction_id).unwrap();
        let a = Lifeline { id: LifelineId::new(), name: "component".into(), represented_path: vec![part] };
        interaction.lifelines.push(a.clone());
        interaction.messages.push(Message { id: MessageId::new(), name: "run".into(), sort: MessageSort::SynchCall, send_event: Some(Occurrence { id: OccurrenceId::new(), lifeline_id: a.id, order: 10 }), receive_event: Some(Occurrence { id: OccurrenceId::new(), lifeline_id: a.id, order: 20 }), signature: Some(MessageSignature::Operation(operation)), arguments: vec![] });
        repo.validate(&project).unwrap();
    }
}
