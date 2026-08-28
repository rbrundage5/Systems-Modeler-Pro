use crate::{
    Action, ActionKind, Activity, ActivityEdge, ActivityEdgeId, ActivityEdgeKind, ActivityEndpoint,
    ActivityId, ActivityNode, ActivityNodeId, ActivityNodeKind, ActivityRepository,
    DiagnosticSeverity, ElementId, ElementKind, EngineStepOutcome, ExecutionEngine, ExecutionError,
    ExecutionSession, ExecutionSnapshot, ModeledOperationRequest, ObjectNodeKind,
    ObjectNodeOrdering, ParameterDirection, Pin, PinDirection, Project, RuntimeEvent,
    RuntimeEventAddress, RuntimeEventKind, RuntimeEventRequest, RuntimeInstanceId, RuntimeValue,
    StructuredActivityNodeKind, evaluate_execution_expression, invoke_modeled_operation,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityNodeExecutionState {
    Idle,
    Enabled,
    Executing,
    Waiting,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActivityTokenValue {
    Control,
    Object(RuntimeValue),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityRuntimeToken {
    pub id: u64,
    pub value: ActivityTokenValue,
    pub arrived_via_edge_id: Option<ActivityEdgeId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityNodeExecutionSnapshot {
    pub frame_id: u64,
    pub activity_id: ActivityId,
    pub node_id: ActivityNodeId,
    pub name: String,
    pub state: ActivityNodeExecutionState,
    pub activation_count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityTokenStoreSnapshot {
    pub frame_id: u64,
    pub activity_id: ActivityId,
    pub endpoint: ActivityEndpoint,
    pub tokens: Vec<ActivityRuntimeToken>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityCallFrameSnapshot {
    pub frame_id: u64,
    pub activity_id: ActivityId,
    pub activity_name: String,
    pub caller_frame_id: Option<u64>,
    pub caller_node_id: Option<ActivityNodeId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityExecutionSnapshot {
    pub execution: ExecutionSnapshot,
    pub root_activity_id: ActivityId,
    pub runtime_instance_id: Option<RuntimeInstanceId>,
    pub nodes: Vec<ActivityNodeExecutionSnapshot>,
    pub token_stores: Vec<ActivityTokenStoreSnapshot>,
    pub call_frames: Vec<ActivityCallFrameSnapshot>,
    pub active_node_ids: Vec<ActivityNodeId>,
    pub active_edge_ids: Vec<ActivityEdgeId>,
    pub completed_edge_ids: Vec<ActivityEdgeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivityAdvanceOutcome {
    Progressed,
    Waiting,
    Completed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OperationCallRequest {
    pub operation_id: ElementId,
    pub operation_name: String,
    pub arguments: Vec<(String, RuntimeValue)>,
}

pub trait OperationCallRuntime: Send {
    fn invoke(
        &mut self,
        request: &OperationCallRequest,
    ) -> Result<Vec<(String, RuntimeValue)>, String>;
}

#[derive(Debug, Clone)]
struct ActivityFrame {
    id: u64,
    activity_id: ActivityId,
    caller: Option<(u64, ActivityNodeId)>,
    node_states: HashMap<ActivityNodeId, ActivityNodeExecutionState>,
    activation_counts: HashMap<ActivityNodeId, u64>,
    stores: HashMap<ActivityEndpoint, VecDeque<ActivityRuntimeToken>>,
    completed_edges: HashSet<ActivityEdgeId>,
    data_store_forwarded: HashSet<(ActivityNodeId, u64)>,
    terminated: bool,
}

impl ActivityFrame {
    fn new(id: u64, activity: &Activity, caller: Option<(u64, ActivityNodeId)>) -> Self {
        Self {
            id,
            activity_id: activity.id,
            caller,
            node_states: activity
                .nodes
                .iter()
                .map(|node| (node.id, ActivityNodeExecutionState::Idle))
                .collect(),
            activation_counts: HashMap::new(),
            stores: HashMap::new(),
            completed_edges: HashSet::new(),
            data_store_forwarded: HashSet::new(),
            terminated: false,
        }
    }
}

pub struct ActivityExecutionEngine {
    repository: ActivityRepository,
    root_activity_id: ActivityId,
    frames: Vec<ActivityFrame>,
    next_token_id: u64,
    next_frame_id: u64,
    initial_inputs: HashMap<ElementId, Vec<RuntimeValue>>,
    time_event_targets: HashMap<u64, (u64, ActivityNodeId)>,
    active_node_ids: Vec<ActivityNodeId>,
    active_edge_ids: Vec<ActivityEdgeId>,
    completed_edge_ids: HashSet<ActivityEdgeId>,
    operation_runtime: Option<Box<dyn OperationCallRuntime>>,
    runtime_instance_id: Option<RuntimeInstanceId>,
}

impl ActivityExecutionEngine {
    pub fn new(repository: ActivityRepository, root_activity_id: ActivityId) -> Self {
        Self {
            repository,
            root_activity_id,
            frames: Vec::new(),
            next_token_id: 0,
            next_frame_id: 0,
            initial_inputs: HashMap::new(),
            time_event_targets: HashMap::new(),
            active_node_ids: Vec::new(),
            active_edge_ids: Vec::new(),
            completed_edge_ids: HashSet::new(),
            operation_runtime: None,
            runtime_instance_id: None,
        }
    }

    pub fn with_input(mut self, parameter_id: ElementId, values: Vec<RuntimeValue>) -> Self {
        self.initial_inputs.insert(parameter_id, values);
        self
    }

    pub fn with_operation_runtime(mut self, runtime: impl OperationCallRuntime + 'static) -> Self {
        self.operation_runtime = Some(Box::new(runtime));
        self
    }

    pub fn with_runtime_instance(mut self, runtime_instance_id: RuntimeInstanceId) -> Self {
        self.runtime_instance_id = Some(runtime_instance_id);
        self
    }

    pub fn runtime_instance_id(&self) -> Option<RuntimeInstanceId> {
        self.runtime_instance_id
    }

    pub fn root_activity_id(&self) -> ActivityId {
        self.root_activity_id
    }

    /// Initializes an Activity engine inside an already initialized execution
    /// session. This is the composition path for multiple behavior engines
    /// sharing the same structural occurrences and event queue.
    pub fn initialize_embedded(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<(), ExecutionError> {
        if self.runtime_instance_id.is_none() {
            self.runtime_instance_id = session.root_runtime_instance_id();
        }
        if let Some(instance_id) = self.runtime_instance_id {
            session
                .instances
                .get(&instance_id)
                .ok_or(ExecutionError::RuntimeInstanceNotFound(instance_id))?;
        }
        self.clear_runtime();
        self.initialize_root_frame(project, session)
    }

    pub fn reset(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<(), ExecutionError> {
        session.reset(project)?;
        if self.runtime_instance_id.is_none() {
            self.runtime_instance_id = session.root_runtime_instance_id();
        }
        self.clear_runtime();
        self.initialize_root_frame(project, session)
    }

    pub fn advance(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<ActivityAdvanceOutcome, ExecutionError> {
        let outcome = <Self as ExecutionEngine>::step(self, project, session)?;
        match outcome {
            EngineStepOutcome::Completed => return Ok(ActivityAdvanceOutcome::Completed),
            EngineStepOutcome::Progressed => return Ok(ActivityAdvanceOutcome::Progressed),
            EngineStepOutcome::Idle => {}
        }

        let accepts_next = session
            .next_event()
            .is_some_and(|scheduled| self.accepts_event(project, &scheduled.event));
        if accepts_next {
            let event = session
                .step()?
                .expect("an inspected scheduled event must remain queued");
            return match self.handle_event(project, session, &event)? {
                EngineStepOutcome::Completed => Ok(ActivityAdvanceOutcome::Completed),
                EngineStepOutcome::Progressed => Ok(ActivityAdvanceOutcome::Progressed),
                EngineStepOutcome::Idle => Ok(ActivityAdvanceOutcome::Waiting),
            };
        }
        Ok(ActivityAdvanceOutcome::Waiting)
    }

    pub fn snapshot(&self, session: &ExecutionSession) -> ActivityExecutionSnapshot {
        let mut nodes = Vec::new();
        let mut token_stores = Vec::new();
        let mut call_frames = Vec::new();
        for frame in &self.frames {
            let Some(activity) = self.repository.activities.get(&frame.activity_id) else {
                continue;
            };
            call_frames.push(ActivityCallFrameSnapshot {
                frame_id: frame.id,
                activity_id: frame.activity_id,
                activity_name: activity.name.clone(),
                caller_frame_id: frame.caller.map(|caller| caller.0),
                caller_node_id: frame.caller.map(|caller| caller.1),
            });
            for node in &activity.nodes {
                nodes.push(ActivityNodeExecutionSnapshot {
                    frame_id: frame.id,
                    activity_id: frame.activity_id,
                    node_id: node.id,
                    name: node.name.clone(),
                    state: frame
                        .node_states
                        .get(&node.id)
                        .copied()
                        .unwrap_or(ActivityNodeExecutionState::Idle),
                    activation_count: frame.activation_counts.get(&node.id).copied().unwrap_or(0),
                });
            }
            let mut stores: Vec<_> = frame.stores.iter().collect();
            stores.sort_by_key(|(endpoint, _)| endpoint_sort_key(**endpoint));
            for (endpoint, tokens) in stores {
                if !tokens.is_empty() {
                    token_stores.push(ActivityTokenStoreSnapshot {
                        frame_id: frame.id,
                        activity_id: frame.activity_id,
                        endpoint: *endpoint,
                        tokens: tokens.iter().cloned().collect(),
                    });
                }
            }
        }
        let mut completed_edge_ids: Vec<_> = self.completed_edge_ids.iter().copied().collect();
        completed_edge_ids.sort_by_key(ToString::to_string);
        ActivityExecutionSnapshot {
            execution: session.snapshot(),
            root_activity_id: self.root_activity_id,
            runtime_instance_id: self.runtime_instance_id,
            nodes,
            token_stores,
            call_frames,
            active_node_ids: self.active_node_ids.clone(),
            active_edge_ids: self.active_edge_ids.clone(),
            completed_edge_ids,
        }
    }

    fn clear_runtime(&mut self) {
        self.frames.clear();
        self.next_token_id = 0;
        self.next_frame_id = 0;
        self.time_event_targets.clear();
        self.active_node_ids.clear();
        self.active_edge_ids.clear();
        self.completed_edge_ids.clear();
    }

    fn initialize_root_frame(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<(), ExecutionError> {
        self.repository.validate(project).map_err(|error| {
            engine_error(format!("Cannot initialize Activity execution: {error}"))
        })?;
        let activity = self.activity(self.root_activity_id)?.clone();
        self.warn_for_structured_semantics(&activity, session);
        let authored_defaults: Vec<_> = project
            .elements
            .values()
            .filter_map(|element| {
                element
                    .default_value
                    .as_deref()
                    .and_then(parse_authored_value)
                    .map(|value| (element.id, value))
            })
            .collect();
        for (element_id, value) in authored_defaults {
            let element = project
                .element(element_id)
                .map_err(|error| engine_error(error.to_string()))?;
            let instance_id = if element.kind == ElementKind::ValueProperty {
                self.runtime_instance_id
            } else {
                None
            };
            if session
                .value_in_instance_context(instance_id, element_id)
                .is_none()
            {
                session.set_value(project, instance_id, element_id, value)?;
            }
        }
        let inputs = self.initial_inputs.clone();
        let frame = self.make_frame(project, session, &activity, None, &inputs)?;
        session.record_engine_trace(
            activity.context_id.or(Some(activity.owner_id)),
            format!("Initialized Activity '{}'", activity.name),
        );
        self.frames.push(frame);
        Ok(())
    }

    fn make_frame(
        &mut self,
        project: &Project,
        session: &ExecutionSession,
        activity: &Activity,
        caller: Option<(u64, ActivityNodeId)>,
        inputs: &HashMap<ElementId, Vec<RuntimeValue>>,
    ) -> Result<ActivityFrame, ExecutionError> {
        if self.frames.len() as u64 >= session.configuration.max_steps.max(1) {
            return Err(engine_error(format!(
                "Cannot invoke Activity '{}': call depth reached the configured execution step limit.",
                activity.name
            )));
        }
        let frame_id = self.next_frame_id;
        self.next_frame_id += 1;
        let mut frame = ActivityFrame::new(frame_id, activity, caller);
        for node in &activity.nodes {
            if matches!(node.kind, ActivityNodeKind::Initial) {
                frame
                    .node_states
                    .insert(node.id, ActivityNodeExecutionState::Enabled);
            }
            let ActivityNodeKind::ActivityParameter(parameter_node) = &node.kind else {
                continue;
            };
            let parameter = project
                .element(parameter_node.parameter_id)
                .map_err(|error| engine_error(error.to_string()))?;
            let direction = parameter
                .parameter_direction
                .unwrap_or(ParameterDirection::In);
            if !matches!(
                direction,
                ParameterDirection::In | ParameterDirection::InOut
            ) {
                continue;
            }
            let values = inputs
                .get(&parameter.id)
                .cloned()
                .or_else(|| {
                    session
                        .value(None, parameter.id)
                        .cloned()
                        .map(|value| vec![value])
                })
                .or_else(|| {
                    parameter
                        .default_value
                        .as_deref()
                        .and_then(parse_authored_value)
                        .map(|value| vec![value])
                })
                .unwrap_or_default();
            for value in values {
                let token = self.object_token(value);
                frame
                    .stores
                    .entry(ActivityEndpoint::Node(node.id))
                    .or_default()
                    .push_back(token);
            }
        }
        Ok(frame)
    }

    fn warn_for_structured_semantics(&self, activity: &Activity, session: &mut ExecutionSession) {
        for structured in &activity.structured_nodes {
            let limitation = match structured.kind {
                StructuredActivityNodeKind::ExpansionRegion => Some(
                    "ExpansionRegion execution requires expansion-node/mode semantics that are not represented by the current authored metamodel.",
                ),
                StructuredActivityNodeKind::Conditional => Some(
                    "ConditionalNode clauses are not represented by the current authored metamodel; contained control flows execute normally, but clause semantics are not inferred.",
                ),
                _ => None,
            };
            if let Some(limitation) = limitation {
                session.add_diagnostic(
                    DiagnosticSeverity::Warning,
                    activity.context_id.or(Some(activity.owner_id)),
                    format!(
                        "Activity '{}', structured node '{}': {limitation}",
                        activity.name, structured.name
                    ),
                );
            }
        }
    }

    fn activity(&self, id: ActivityId) -> Result<&Activity, ExecutionError> {
        self.repository.activities.get(&id).ok_or_else(|| {
            engine_error("Cannot execute Activity: referenced Activity no longer exists.".into())
        })
    }

    fn refresh_enabled(
        &mut self,
        project: &Project,
        session: &ExecutionSession,
        frame_index: usize,
    ) -> Result<(), ExecutionError> {
        let activity_id = self.frames[frame_index].activity_id;
        let activity = self.activity(activity_id)?.clone();
        for node in &activity.nodes {
            let current = self.frames[frame_index]
                .node_states
                .get(&node.id)
                .copied()
                .unwrap_or(ActivityNodeExecutionState::Idle);
            if matches!(
                current,
                ActivityNodeExecutionState::Waiting
                    | ActivityNodeExecutionState::Executing
                    | ActivityNodeExecutionState::Failed
            ) {
                continue;
            }
            let enabled = self.node_is_enabled(project, session, frame_index, &activity, node)?;
            self.frames[frame_index].node_states.insert(
                node.id,
                if enabled {
                    ActivityNodeExecutionState::Enabled
                } else if current == ActivityNodeExecutionState::Completed {
                    ActivityNodeExecutionState::Completed
                } else {
                    ActivityNodeExecutionState::Idle
                },
            );
        }
        Ok(())
    }

    fn node_is_enabled(
        &self,
        project: &Project,
        session: &ExecutionSession,
        frame_index: usize,
        activity: &Activity,
        node: &ActivityNode,
    ) -> Result<bool, ExecutionError> {
        let frame = &self.frames[frame_index];
        let activation_count = frame.activation_counts.get(&node.id).copied().unwrap_or(0);
        if matches!(node.kind, ActivityNodeKind::Initial) {
            return Ok(activation_count == 0);
        }
        if let ActivityNodeKind::Object(object) = &node.kind {
            let store = frame.stores.get(&ActivityEndpoint::Node(node.id));
            let has_token = match object.kind {
                ObjectNodeKind::DataStore => store.is_some_and(|tokens| {
                    tokens
                        .iter()
                        .any(|token| !frame.data_store_forwarded.contains(&(node.id, token.id)))
                }),
                _ => store.is_some_and(|tokens| !tokens.is_empty()),
            };
            return Ok(
                has_token && self.has_outgoing(activity, node.id, ActivityEdgeKind::ObjectFlow)
            );
        }
        if matches!(node.kind, ActivityNodeKind::ActivityParameter(_)) {
            return Ok(frame
                .stores
                .get(&ActivityEndpoint::Node(node.id))
                .is_some_and(|tokens| !tokens.is_empty())
                && self.has_outgoing(activity, node.id, ActivityEdgeKind::ObjectFlow));
        }

        let incoming_control = incoming_edges(activity, node.id, ActivityEdgeKind::ControlFlow);
        let control_ready = match &node.kind {
            ActivityNodeKind::Merge
            | ActivityNodeKind::Decision { .. }
            | ActivityNodeKind::FlowFinal
            | ActivityNodeKind::ActivityFinal => incoming_control
                .iter()
                .any(|edge| self.edge_token_count(frame, node.id, edge.id, true) > 0),
            _ if incoming_control.is_empty() => activation_count == 0,
            _ => {
                let mut ready = true;
                for edge in incoming_control {
                    let required = self.edge_weight(project, session, frame, edge)?;
                    if self.edge_token_count(frame, node.id, edge.id, true) < required {
                        ready = false;
                        break;
                    }
                }
                ready
            }
        };
        if !control_ready {
            return Ok(false);
        }
        let ActivityNodeKind::Action(action) = &node.kind else {
            return Ok(true);
        };
        for pin in action
            .pins
            .iter()
            .filter(|pin| matches!(pin.direction, PinDirection::Input))
        {
            let available = frame
                .stores
                .get(&ActivityEndpoint::Pin(pin.id))
                .map(VecDeque::len)
                .unwrap_or(0);
            if available < pin.multiplicity.lower as usize {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn has_outgoing(
        &self,
        activity: &Activity,
        node_id: ActivityNodeId,
        kind: ActivityEdgeKind,
    ) -> bool {
        activity
            .edges
            .iter()
            .any(|edge| edge.kind == kind && edge.source == ActivityEndpoint::Node(node_id))
    }

    fn edge_token_count(
        &self,
        frame: &ActivityFrame,
        node_id: ActivityNodeId,
        edge_id: ActivityEdgeId,
        control: bool,
    ) -> usize {
        frame
            .stores
            .get(&ActivityEndpoint::Node(node_id))
            .map(|tokens| {
                tokens
                    .iter()
                    .filter(|token| {
                        token.arrived_via_edge_id == Some(edge_id)
                            && (matches!(token.value, ActivityTokenValue::Control) == control)
                    })
                    .count()
            })
            .unwrap_or(0)
    }

    fn edge_weight(
        &self,
        project: &Project,
        session: &ExecutionSession,
        frame: &ActivityFrame,
        edge: &ActivityEdge,
    ) -> Result<usize, ExecutionError> {
        let Some(weight) = edge
            .weight
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        else {
            return Ok(1);
        };
        match self.evaluate(project, session, frame, &HashMap::new(), weight)? {
            RuntimeValue::Integer(value) if value > 0 => usize::try_from(value).map_err(|_| {
                engine_error(format!(
                    "Activity edge '{}' weight is too large.",
                    edge.name
                ))
            }),
            value => Err(engine_error(format!(
                "Activity edge '{}' weight must evaluate to a positive integer, but evaluated to {}.",
                edge.name,
                value.kind_name()
            ))),
        }
    }

    fn evaluate(
        &self,
        project: &Project,
        session: &ExecutionSession,
        frame: &ActivityFrame,
        local: &HashMap<String, RuntimeValue>,
        expression: &str,
    ) -> Result<RuntimeValue, ExecutionError> {
        evaluate_execution_expression(expression, |name| {
            local
                .get(name)
                .cloned()
                .or_else(|| {
                    let mut matching: Vec<_> = project
                        .elements
                        .values()
                        .filter(|element| {
                            element.name == name
                                || project
                                    .qualified_name(element.id)
                                    .is_ok_and(|qualified| qualified == name)
                        })
                        .collect();
                    matching.sort_by_key(|element| element.id.to_string());
                    matching.into_iter().find_map(|element| {
                        session
                            .value_in_instance_context(self.runtime_instance_id, element.id)
                            .cloned()
                    })
                })
                .or_else(|| {
                    frame.stores.iter().find_map(|(endpoint, tokens)| {
                        let pin_name = match endpoint {
                            ActivityEndpoint::Pin(pin_id) => self
                                .activity(frame.activity_id)
                                .ok()?
                                .nodes
                                .iter()
                                .filter_map(|node| match &node.kind {
                                    ActivityNodeKind::Action(action) => Some(&action.pins),
                                    _ => None,
                                })
                                .flatten()
                                .find(|pin| pin.id == *pin_id)
                                .map(|pin| pin.name.as_str()),
                            ActivityEndpoint::Node(_) => None,
                        }?;
                        if pin_name != name {
                            return None;
                        }
                        tokens.iter().find_map(|token| match &token.value {
                            ActivityTokenValue::Object(value) => Some(value.clone()),
                            ActivityTokenValue::Control => None,
                        })
                    })
                })
        })
        .map_err(|error| {
            engine_error(format!(
                "Cannot evaluate Activity expression '{expression}': {error}"
            ))
        })
    }

    fn fire_node(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        frame_index: usize,
        activity: &Activity,
        node: &ActivityNode,
    ) -> Result<EngineStepOutcome, ExecutionError> {
        session.consume_step_budget()?;
        self.active_node_ids.push(node.id);
        self.active_edge_ids.clear();
        self.frames[frame_index]
            .node_states
            .insert(node.id, ActivityNodeExecutionState::Executing);
        *self.frames[frame_index]
            .activation_counts
            .entry(node.id)
            .or_default() += 1;
        session.record_engine_trace(
            activity.context_id.or(Some(activity.owner_id)),
            format!(
                "Activity '{}': executed node '{}'",
                activity.name,
                display_node_name(node)
            ),
        );

        match &node.kind {
            ActivityNodeKind::Initial => {
                self.offer_control_outputs(project, session, frame_index, activity, node.id, None)?;
                self.complete_node(frame_index, node.id);
            }
            ActivityNodeKind::Action(action) => {
                return self.fire_action(project, session, frame_index, activity, node, action);
            }
            ActivityNodeKind::Decision { decision_input } => {
                self.consume_control_any(frame_index, activity, node.id)?;
                self.fire_decision(
                    project,
                    session,
                    frame_index,
                    activity,
                    node,
                    decision_input.as_deref(),
                )?;
                self.complete_node(frame_index, node.id);
            }
            ActivityNodeKind::Merge => {
                self.consume_control_any(frame_index, activity, node.id)?;
                self.offer_control_outputs(project, session, frame_index, activity, node.id, None)?;
                self.complete_node(frame_index, node.id);
            }
            ActivityNodeKind::Fork => {
                self.consume_required_control(project, session, frame_index, activity, node.id)?;
                self.offer_control_outputs(project, session, frame_index, activity, node.id, None)?;
                self.complete_node(frame_index, node.id);
            }
            ActivityNodeKind::Join { join_specification } => {
                self.consume_required_control(project, session, frame_index, activity, node.id)?;
                if let Some(specification) = join_specification
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    let local = HashMap::from([(
                        "incoming".into(),
                        RuntimeValue::Integer(
                            incoming_edges(activity, node.id, ActivityEdgeKind::ControlFlow).len()
                                as i64,
                        ),
                    )]);
                    let frame = &self.frames[frame_index];
                    if self.evaluate(project, session, frame, &local, specification)?
                        != RuntimeValue::Boolean(true)
                    {
                        return self.fail_node(
                            session,
                            frame_index,
                            activity,
                            node,
                            "join specification did not evaluate to true",
                        );
                    }
                }
                self.offer_control_outputs(project, session, frame_index, activity, node.id, None)?;
                self.complete_node(frame_index, node.id);
            }
            ActivityNodeKind::FlowFinal => {
                self.consume_control_any(frame_index, activity, node.id)?;
                self.complete_node(frame_index, node.id);
            }
            ActivityNodeKind::ActivityFinal => {
                self.consume_control_any(frame_index, activity, node.id)?;
                self.frames[frame_index].terminated = true;
                self.complete_node(frame_index, node.id);
            }
            ActivityNodeKind::Object(_) | ActivityNodeKind::ActivityParameter(_) => {
                self.forward_object_node(project, session, frame_index, activity, node)?;
                self.complete_node(frame_index, node.id);
            }
        }
        Ok(EngineStepOutcome::Progressed)
    }

    fn fire_action(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        frame_index: usize,
        activity: &Activity,
        node: &ActivityNode,
        action: &Action,
    ) -> Result<EngineStepOutcome, ExecutionError> {
        self.consume_required_control(project, session, frame_index, activity, node.id)?;
        let inputs = self.consume_action_inputs(frame_index, action)?;
        let local: HashMap<_, _> = inputs.iter().cloned().collect();
        match &action.kind {
            ActionKind::CallBehavior { activity_id } => {
                let called = self.activity(*activity_id)?.clone();
                let parameter_inputs = self.call_parameter_inputs(project, action, &local, &called);
                let caller = (self.frames[frame_index].id, node.id);
                let child =
                    self.make_frame(project, session, &called, Some(caller), &parameter_inputs)?;
                self.frames[frame_index]
                    .node_states
                    .insert(node.id, ActivityNodeExecutionState::Waiting);
                session.record_engine_trace(
                    activity.context_id.or(Some(activity.owner_id)),
                    format!(
                        "Activity '{}': '{}' called Activity '{}'",
                        activity.name,
                        display_node_name(node),
                        called.name
                    ),
                );
                self.frames.push(child);
                return Ok(EngineStepOutcome::Progressed);
            }
            ActionKind::SendSignal { signal_id } => {
                let signal = project
                    .element(*signal_id)
                    .map_err(|error| engine_error(error.to_string()))?;
                if let Some(source_instance_id) = self.runtime_instance_id
                    && session.structural_runtime.is_some()
                {
                    session.queue_structural_signal_from_instance(
                        project,
                        source_instance_id,
                        *signal_id,
                        signal.name.clone(),
                        inputs,
                    )?;
                } else {
                    session.queue_typed_event_at(
                        project,
                        RuntimeEventRequest {
                            due_time: session.simulation_time,
                            kind: RuntimeEventKind::Signal,
                            name: signal.name.clone(),
                            semantic_event_id: Some(*signal_id),
                            address: RuntimeEventAddress {
                                source_semantic_id: activity.context_id,
                                source_runtime_instance_id: self.runtime_instance_id,
                                ..RuntimeEventAddress::default()
                            },
                            payload: inputs,
                        },
                    )?;
                }
            }
            ActionKind::AcceptEvent { .. } => {
                self.frames[frame_index]
                    .node_states
                    .insert(node.id, ActivityNodeExecutionState::Waiting);
                return Ok(EngineStepOutcome::Progressed);
            }
            ActionKind::AcceptTimeEvent { expression } => {
                let delay = self.evaluate_duration(
                    project,
                    session,
                    &self.frames[frame_index],
                    expression,
                )?;
                let sequence = session.queue_event_after(
                    project,
                    delay,
                    RuntimeEventKind::Time,
                    format!("Timer for {}", display_node_name(node)),
                    RuntimeEventAddress {
                        target_semantic_id: activity.context_id,
                        target_runtime_instance_id: self.runtime_instance_id,
                        ..RuntimeEventAddress::default()
                    },
                    Vec::new(),
                )?;
                self.time_event_targets
                    .insert(sequence, (self.frames[frame_index].id, node.id));
                self.frames[frame_index]
                    .node_states
                    .insert(node.id, ActivityNodeExecutionState::Waiting);
                return Ok(EngineStepOutcome::Progressed);
            }
            ActionKind::CallOperation { operation_id } => {
                let operation = project
                    .element(*operation_id)
                    .map_err(|error| engine_error(error.to_string()))?;
                let outputs = if let Some(runtime) = self.operation_runtime.as_mut() {
                    let request = OperationCallRequest {
                        operation_id: *operation_id,
                        operation_name: operation.name.clone(),
                        arguments: inputs,
                    };
                    runtime.invoke(&request).map_err(|message| {
                        session.fail(
                            activity.context_id.or(Some(activity.owner_id)),
                            message.clone(),
                        );
                        engine_error(message)
                    })?
                } else {
                    let target_runtime_instance_id = self.runtime_instance_id.ok_or_else(|| {
                        engine_error(format!(
                            "CallOperationAction '{}' requires a selected runtime occurrence for modeled Operation '{}'.",
                            display_node_name(node), operation.name
                        ))
                    })?;
                    invoke_modeled_operation(
                        project,
                        session,
                        &ModeledOperationRequest {
                            operation_id: *operation_id,
                            target_runtime_instance_id,
                            arguments: inputs,
                        },
                    )?
                    .outputs
                };
                self.emit_named_outputs(project, session, frame_index, activity, action, &outputs)?;
            }
            ActionKind::Opaque { body } => {
                let body = body.trim();
                let result = if body.is_empty() {
                    None
                } else {
                    Some(self.evaluate(project, session, &self.frames[frame_index], &local, body).map_err(
                        |error| {
                            let message = format!(
                                "Cannot execute OpaqueAction '{}': only bounded pure expressions are supported. {error}",
                                display_node_name(node)
                            );
                            session.fail(activity.context_id.or(Some(activity.owner_id)), message.clone());
                            engine_error(message)
                        },
                    )?)
                };
                self.emit_expression_outputs(
                    project,
                    session,
                    frame_index,
                    action,
                    &local,
                    result,
                )?;
            }
        }
        self.offer_control_outputs(project, session, frame_index, activity, node.id, None)?;
        self.complete_node(frame_index, node.id);
        Ok(EngineStepOutcome::Progressed)
    }

    fn consume_action_inputs(
        &mut self,
        frame_index: usize,
        action: &Action,
    ) -> Result<Vec<(String, RuntimeValue)>, ExecutionError> {
        let mut values = Vec::new();
        for pin in action
            .pins
            .iter()
            .filter(|pin| matches!(pin.direction, PinDirection::Input | PinDirection::Value))
        {
            if pin.direction == PinDirection::Value {
                if let Some(expression) = pin
                    .value
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    let value =
                        evaluate_execution_expression(expression, |_| None).map_err(|error| {
                            engine_error(format!("ValuePin '{}': {error}", pin.name))
                        })?;
                    values.push((pin.name.clone(), value));
                }
                continue;
            }
            let store = self.frames[frame_index]
                .stores
                .entry(ActivityEndpoint::Pin(pin.id))
                .or_default();
            for _ in 0..pin.multiplicity.lower {
                let token = store.pop_front().ok_or_else(|| {
                    engine_error(format!(
                        "InputPin '{}' has insufficient object tokens.",
                        pin.name
                    ))
                })?;
                if let ActivityTokenValue::Object(value) = token.value {
                    values.push((pin.name.clone(), value));
                }
            }
        }
        Ok(values)
    }

    fn emit_expression_outputs(
        &mut self,
        project: &Project,
        session: &ExecutionSession,
        frame_index: usize,
        action: &Action,
        local: &HashMap<String, RuntimeValue>,
        body_result: Option<RuntimeValue>,
    ) -> Result<(), ExecutionError> {
        let activity = self.activity(self.frames[frame_index].activity_id)?.clone();
        let output_pins: Vec<_> = action
            .pins
            .iter()
            .filter(|pin| pin.direction == PinDirection::Output)
            .collect();
        for pin in &output_pins {
            let value = if let Some(expression) = pin
                .value
                .as_deref()
                .filter(|value| !value.trim().is_empty())
            {
                Some(self.evaluate(
                    project,
                    session,
                    &self.frames[frame_index],
                    local,
                    expression,
                )?)
            } else if output_pins.len() == 1 {
                body_result.clone()
            } else {
                None
            };
            if let Some(value) = value {
                self.emit_pin_value(frame_index, &activity, pin, value)?;
            }
        }
        Ok(())
    }

    fn emit_named_outputs(
        &mut self,
        _project: &Project,
        _session: &ExecutionSession,
        frame_index: usize,
        activity: &Activity,
        action: &Action,
        outputs: &[(String, RuntimeValue)],
    ) -> Result<(), ExecutionError> {
        for pin in action
            .pins
            .iter()
            .filter(|pin| pin.direction == PinDirection::Output)
        {
            if let Some((_, value)) = outputs.iter().find(|(name, _)| name == &pin.name) {
                self.emit_pin_value(frame_index, activity, pin, value.clone())?;
            }
        }
        Ok(())
    }

    fn emit_pin_value(
        &mut self,
        frame_index: usize,
        activity: &Activity,
        pin: &Pin,
        value: RuntimeValue,
    ) -> Result<(), ExecutionError> {
        let edges: Vec<_> = activity
            .edges
            .iter()
            .filter(|edge| {
                edge.kind == ActivityEdgeKind::ObjectFlow
                    && edge.source == ActivityEndpoint::Pin(pin.id)
            })
            .cloned()
            .collect();
        if edges.len() > 1 {
            return Err(engine_error(format!(
                "OutputPin '{}' has multiple ObjectFlows; explicit object-token routing is required.",
                pin.name
            )));
        }
        let token = self.object_token(value);
        if let Some(edge) = edges.first() {
            self.offer_token(frame_index, activity, edge, token)?;
        } else {
            self.push_store_token(frame_index, activity, ActivityEndpoint::Pin(pin.id), token)?;
        }
        Ok(())
    }

    fn call_parameter_inputs(
        &self,
        project: &Project,
        action: &Action,
        local: &HashMap<String, RuntimeValue>,
        called: &Activity,
    ) -> HashMap<ElementId, Vec<RuntimeValue>> {
        let mut inputs = HashMap::new();
        for parameter_node in &called.nodes {
            let ActivityNodeKind::ActivityParameter(parameter_node) = &parameter_node.kind else {
                continue;
            };
            let Ok(parameter) = project.element(parameter_node.parameter_id) else {
                continue;
            };
            if !matches!(
                parameter
                    .parameter_direction
                    .unwrap_or(ParameterDirection::In),
                ParameterDirection::In | ParameterDirection::InOut
            ) {
                continue;
            }
            let value = action
                .pins
                .iter()
                .find(|pin| {
                    pin.direction == PinDirection::Input
                        && (pin.parameter_id == Some(parameter.id) || pin.name == parameter.name)
                })
                .and_then(|pin| local.get(&pin.name))
                .cloned();
            if let Some(value) = value {
                inputs.insert(parameter.id, vec![value]);
            }
        }
        inputs
    }

    fn finish_child_frame(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<EngineStepOutcome, ExecutionError> {
        let child = self.frames.pop().expect("completed child frame exists");
        let (caller_frame_id, caller_node_id) = child
            .caller
            .expect("root frame is completed by the outer execution path");
        let child_activity = self.activity(child.activity_id)?.clone();
        let caller_index = self
            .frames
            .iter()
            .position(|frame| frame.id == caller_frame_id)
            .ok_or_else(|| engine_error("Activity call frame lost its caller.".into()))?;
        let caller_activity = self
            .activity(self.frames[caller_index].activity_id)?
            .clone();
        let caller_node = caller_activity
            .nodes
            .iter()
            .find(|node| node.id == caller_node_id)
            .cloned()
            .ok_or_else(|| {
                engine_error("Activity call frame references a missing caller node.".into())
            })?;
        let ActivityNodeKind::Action(caller_action) = &caller_node.kind else {
            return Err(engine_error(
                "Activity call frame caller is not an Action.".into(),
            ));
        };
        for parameter_node in &child_activity.nodes {
            let ActivityNodeKind::ActivityParameter(parameter_node_kind) = &parameter_node.kind
            else {
                continue;
            };
            let parameter = project
                .element(parameter_node_kind.parameter_id)
                .map_err(|error| engine_error(error.to_string()))?;
            if !matches!(
                parameter.parameter_direction,
                Some(
                    ParameterDirection::Out
                        | ParameterDirection::Return
                        | ParameterDirection::InOut
                )
            ) {
                continue;
            }
            let values = child
                .stores
                .get(&ActivityEndpoint::Node(parameter_node.id))
                .into_iter()
                .flatten()
                .filter_map(|token| match &token.value {
                    ActivityTokenValue::Object(value) => Some(value.clone()),
                    ActivityTokenValue::Control => None,
                });
            if let Some(pin) = caller_action.pins.iter().find(|pin| {
                pin.direction == PinDirection::Output
                    && (pin.parameter_id == Some(parameter.id) || pin.name == parameter.name)
            }) {
                for value in values {
                    self.emit_pin_value(caller_index, &caller_activity, pin, value)?;
                }
            }
        }
        self.offer_control_outputs(
            project,
            session,
            caller_index,
            &caller_activity,
            caller_node.id,
            None,
        )?;
        self.complete_node(caller_index, caller_node.id);
        session.record_engine_trace(
            caller_activity
                .context_id
                .or(Some(caller_activity.owner_id)),
            format!(
                "Activity '{}': call to Activity '{}' completed",
                caller_activity.name, child_activity.name
            ),
        );
        Ok(EngineStepOutcome::Progressed)
    }

    fn frame_is_quiescent(&self, frame_index: usize) -> bool {
        let frame = &self.frames[frame_index];
        !frame.node_states.values().any(|state| {
            matches!(
                state,
                ActivityNodeExecutionState::Enabled
                    | ActivityNodeExecutionState::Executing
                    | ActivityNodeExecutionState::Waiting
            )
        })
    }

    fn accepts_event(&self, project: &Project, event: &RuntimeEvent) -> bool {
        self.frames.iter().rev().any(|frame| {
            let Ok(activity) = self.activity(frame.activity_id) else {
                return false;
            };
            activity.nodes.iter().any(|node| {
                frame.node_states.get(&node.id) == Some(&ActivityNodeExecutionState::Waiting)
                    && self.node_accepts_event(project, frame, node, event)
            })
        })
    }

    fn node_accepts_event(
        &self,
        project: &Project,
        frame: &ActivityFrame,
        node: &ActivityNode,
        event: &RuntimeEvent,
    ) -> bool {
        if event
            .target_runtime_instance_id
            .is_some_and(|target| Some(target) != self.runtime_instance_id)
        {
            return false;
        }
        let ActivityNodeKind::Action(action) = &node.kind else {
            return false;
        };
        match action.kind {
            ActionKind::AcceptEvent { signal_id } => {
                if event.kind != RuntimeEventKind::Signal {
                    return false;
                }
                signal_id.is_none_or(|id| {
                    event.semantic_event_id == Some(id)
                        || (event.semantic_event_id.is_none()
                            && project
                                .element(id)
                                .is_ok_and(|signal| signal.name == event.name))
                })
            }
            ActionKind::AcceptTimeEvent { .. } => {
                event.kind == RuntimeEventKind::Time
                    && self.time_event_targets.get(&event.sequence) == Some(&(frame.id, node.id))
            }
            _ => false,
        }
    }

    fn complete_waiting_action(
        &mut self,
        project: &Project,
        session: &ExecutionSession,
        frame_index: usize,
        activity: &Activity,
        node: &ActivityNode,
        event: &RuntimeEvent,
    ) -> Result<(), ExecutionError> {
        let ActivityNodeKind::Action(action) = &node.kind else {
            return Err(engine_error(
                "waiting Activity node is not an Action.".into(),
            ));
        };
        for pin in action
            .pins
            .iter()
            .filter(|pin| pin.direction == PinDirection::Output)
        {
            if let Some((_, value)) = event.payload.iter().find(|(name, _)| name == &pin.name) {
                self.emit_pin_value(frame_index, activity, pin, value.clone())?;
            }
        }
        self.offer_control_outputs(project, session, frame_index, activity, node.id, None)?;
        self.complete_node(frame_index, node.id);
        self.time_event_targets.remove(&event.sequence);
        Ok(())
    }

    fn fire_decision(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        frame_index: usize,
        activity: &Activity,
        node: &ActivityNode,
        decision_input: Option<&str>,
    ) -> Result<(), ExecutionError> {
        let mut local = HashMap::new();
        if let Some(input) = decision_input.filter(|value| !value.trim().is_empty()) {
            let value =
                self.evaluate(project, session, &self.frames[frame_index], &local, input)?;
            local.insert("decisionInput".into(), value);
        }
        let outgoing = outgoing_edges(
            activity,
            ActivityEndpoint::Node(node.id),
            ActivityEdgeKind::ControlFlow,
        );
        let mut selected = None;
        let mut otherwise = None;
        for edge in outgoing {
            let guard = edge.guard.as_deref().unwrap_or("true").trim();
            if guard.eq_ignore_ascii_case("else") {
                otherwise = Some(edge);
                continue;
            }
            match self.evaluate(project, session, &self.frames[frame_index], &local, guard)? {
                RuntimeValue::Boolean(true) => {
                    selected = Some(edge);
                    break;
                }
                RuntimeValue::Boolean(false) => {}
                value => {
                    return Err(engine_error(format!(
                        "Decision '{}' guard '{}' evaluated to {}, not boolean.",
                        display_node_name(node),
                        guard,
                        value.kind_name()
                    )));
                }
            }
        }
        let edge = selected.or(otherwise).ok_or_else(|| {
            let message = format!(
                "Cannot execute Decision '{}': no outgoing guard evaluated to true and no else guard exists.",
                display_node_name(node)
            );
            session.fail(activity.context_id.or(Some(activity.owner_id)), message.clone());
            engine_error(message)
        })?;
        let token = self.control_token();
        self.offer_token(frame_index, activity, edge, token)
    }

    fn offer_control_outputs(
        &mut self,
        project: &Project,
        session: &ExecutionSession,
        frame_index: usize,
        activity: &Activity,
        node_id: ActivityNodeId,
        local: Option<&HashMap<String, RuntimeValue>>,
    ) -> Result<(), ExecutionError> {
        let outgoing: Vec<_> = outgoing_edges(
            activity,
            ActivityEndpoint::Node(node_id),
            ActivityEdgeKind::ControlFlow,
        )
        .into_iter()
        .cloned()
        .collect();
        for edge in &outgoing {
            let enabled = match edge
                .guard
                .as_deref()
                .filter(|guard| !guard.trim().is_empty())
            {
                None => true,
                Some(guard) if guard.trim().eq_ignore_ascii_case("else") => true,
                Some(guard) => matches!(
                    self.evaluate(
                        project,
                        session,
                        &self.frames[frame_index],
                        local.unwrap_or(&HashMap::new()),
                        guard,
                    )?,
                    RuntimeValue::Boolean(true)
                ),
            };
            if enabled {
                let token = self.control_token();
                self.offer_token(frame_index, activity, edge, token)?;
            }
        }
        Ok(())
    }

    fn offer_token(
        &mut self,
        frame_index: usize,
        activity: &Activity,
        edge: &ActivityEdge,
        mut token: ActivityRuntimeToken,
    ) -> Result<(), ExecutionError> {
        token.arrived_via_edge_id = Some(edge.id);
        self.push_store_token(frame_index, activity, edge.target, token)?;
        self.frames[frame_index].completed_edges.insert(edge.id);
        self.completed_edge_ids.insert(edge.id);
        self.active_edge_ids.push(edge.id);
        if let Some(region_id) = edge.interrupting_region_id {
            let member_ids: Vec<_> = activity
                .nodes
                .iter()
                .filter(|node| node.structured_node_id == Some(region_id))
                .map(|node| node.id)
                .collect();
            for member_id in member_ids {
                self.frames[frame_index]
                    .stores
                    .remove(&ActivityEndpoint::Node(member_id));
                self.frames[frame_index]
                    .node_states
                    .insert(member_id, ActivityNodeExecutionState::Completed);
            }
        }
        Ok(())
    }

    fn push_store_token(
        &mut self,
        frame_index: usize,
        activity: &Activity,
        endpoint: ActivityEndpoint,
        token: ActivityRuntimeToken,
    ) -> Result<(), ExecutionError> {
        let (upper, unique, endpoint_name) = endpoint_constraints(activity, endpoint)?;
        let store = self.frames[frame_index].stores.entry(endpoint).or_default();
        if unique && store.iter().any(|existing| existing.value == token.value) {
            return Ok(());
        }
        if upper.is_some_and(|limit| store.len() >= limit as usize) {
            return Err(engine_error(format!(
                "Cannot deliver token to '{endpoint_name}': multiplicity upper bound {} is full.",
                upper.expect("checked upper bound")
            )));
        }
        store.push_back(token);
        Ok(())
    }

    fn consume_required_control(
        &mut self,
        project: &Project,
        session: &ExecutionSession,
        frame_index: usize,
        activity: &Activity,
        node_id: ActivityNodeId,
    ) -> Result<(), ExecutionError> {
        let incoming = incoming_edges(activity, node_id, ActivityEdgeKind::ControlFlow);
        for edge in incoming {
            let required = self.edge_weight(project, session, &self.frames[frame_index], edge)?;
            for _ in 0..required {
                self.remove_edge_token(frame_index, node_id, edge.id, true)?;
            }
        }
        Ok(())
    }

    fn consume_control_any(
        &mut self,
        frame_index: usize,
        activity: &Activity,
        node_id: ActivityNodeId,
    ) -> Result<(), ExecutionError> {
        for edge in incoming_edges(activity, node_id, ActivityEdgeKind::ControlFlow) {
            if self.edge_token_count(&self.frames[frame_index], node_id, edge.id, true) > 0 {
                self.remove_edge_token(frame_index, node_id, edge.id, true)?;
                return Ok(());
            }
        }
        Err(engine_error(
            "required control token is unavailable.".into(),
        ))
    }

    fn remove_edge_token(
        &mut self,
        frame_index: usize,
        node_id: ActivityNodeId,
        edge_id: ActivityEdgeId,
        control: bool,
    ) -> Result<ActivityRuntimeToken, ExecutionError> {
        let store = self.frames[frame_index]
            .stores
            .entry(ActivityEndpoint::Node(node_id))
            .or_default();
        let position = store
            .iter()
            .position(|token| {
                token.arrived_via_edge_id == Some(edge_id)
                    && (matches!(token.value, ActivityTokenValue::Control) == control)
            })
            .ok_or_else(|| engine_error("required Activity token is unavailable.".into()))?;
        store
            .remove(position)
            .ok_or_else(|| engine_error("required Activity token disappeared.".into()))
    }

    fn forward_object_node(
        &mut self,
        _project: &Project,
        _session: &ExecutionSession,
        frame_index: usize,
        activity: &Activity,
        node: &ActivityNode,
    ) -> Result<(), ExecutionError> {
        let edges: Vec<_> = outgoing_edges(
            activity,
            ActivityEndpoint::Node(node.id),
            ActivityEdgeKind::ObjectFlow,
        )
        .into_iter()
        .cloned()
        .collect();
        if edges.len() > 1 {
            return Err(engine_error(format!(
                "Object node '{}' has multiple outgoing ObjectFlows; explicit selection semantics are required.",
                display_node_name(node)
            )));
        }
        let Some(edge) = edges.first() else {
            return Ok(());
        };
        let endpoint = ActivityEndpoint::Node(node.id);
        let token = match &node.kind {
            ActivityNodeKind::Object(object) if object.kind == ObjectNodeKind::DataStore => {
                let forwarded = self.frames[frame_index].data_store_forwarded.clone();
                let token = self.frames[frame_index]
                    .stores
                    .entry(endpoint)
                    .or_default()
                    .iter()
                    .find(|token| !forwarded.contains(&(node.id, token.id)))
                    .cloned()
                    .ok_or_else(|| engine_error("DataStoreNode has no unoffered token.".into()))?;
                self.frames[frame_index]
                    .data_store_forwarded
                    .insert((node.id, token.id));
                token
            }
            ActivityNodeKind::Object(object) => {
                let store = self.frames[frame_index].stores.entry(endpoint).or_default();
                match object.ordering {
                    ObjectNodeOrdering::Lifo => store.pop_back(),
                    ObjectNodeOrdering::Unordered
                    | ObjectNodeOrdering::Ordered
                    | ObjectNodeOrdering::Fifo => store.pop_front(),
                }
                .ok_or_else(|| engine_error("ObjectNode has no token to forward.".into()))?
            }
            ActivityNodeKind::ActivityParameter(_) => self.frames[frame_index]
                .stores
                .entry(endpoint)
                .or_default()
                .pop_front()
                .ok_or_else(|| {
                    engine_error("ActivityParameterNode has no token to forward.".into())
                })?,
            _ => return Err(engine_error("node is not object-capable.".into())),
        };
        self.offer_token(frame_index, activity, edge, token)
    }

    fn evaluate_duration(
        &self,
        project: &Project,
        session: &ExecutionSession,
        frame: &ActivityFrame,
        expression: &str,
    ) -> Result<u64, ExecutionError> {
        let trimmed = expression.trim();
        let trimmed = trimmed.strip_prefix("after ").unwrap_or(trimmed).trim();
        let (numeric, multiplier) = if let Some(value) = trimmed.strip_suffix("ns") {
            (value.trim(), 1.0)
        } else if let Some(value) = trimmed.strip_suffix("us") {
            (value.trim(), 1_000.0)
        } else if let Some(value) = trimmed.strip_suffix("ms") {
            (value.trim(), 1_000_000.0)
        } else if let Some(value) = trimmed.strip_suffix('s') {
            (value.trim(), 1_000_000_000.0)
        } else {
            (trimmed, 1.0)
        };
        let value = self.evaluate(project, session, frame, &HashMap::new(), numeric)?;
        let number = match value {
            RuntimeValue::Integer(value) => value as f64,
            RuntimeValue::Real(value) => value,
            other => {
                return Err(engine_error(format!(
                    "AcceptTimeEvent delay must be numeric, but evaluated to {}.",
                    other.kind_name()
                )));
            }
        };
        let nanos = number * multiplier;
        if !nanos.is_finite() || nanos < 0.0 || nanos > u64::MAX as f64 {
            return Err(engine_error(
                "AcceptTimeEvent delay is outside the supported simulation-time range.".into(),
            ));
        }
        Ok(nanos.round() as u64)
    }

    fn complete_node(&mut self, frame_index: usize, node_id: ActivityNodeId) {
        self.frames[frame_index]
            .node_states
            .insert(node_id, ActivityNodeExecutionState::Completed);
    }

    fn fail_node(
        &mut self,
        session: &mut ExecutionSession,
        frame_index: usize,
        activity: &Activity,
        node: &ActivityNode,
        reason: &str,
    ) -> Result<EngineStepOutcome, ExecutionError> {
        self.frames[frame_index]
            .node_states
            .insert(node.id, ActivityNodeExecutionState::Failed);
        let message = format!(
            "Cannot execute Activity node '{}' in '{}': {reason}.",
            display_node_name(node),
            activity.name
        );
        session.fail(
            activity.context_id.or(Some(activity.owner_id)),
            message.clone(),
        );
        Err(engine_error(message))
    }

    fn control_token(&mut self) -> ActivityRuntimeToken {
        self.next_token(ActivityTokenValue::Control)
    }

    fn object_token(&mut self, value: RuntimeValue) -> ActivityRuntimeToken {
        self.next_token(ActivityTokenValue::Object(value))
    }

    fn next_token(&mut self, value: ActivityTokenValue) -> ActivityRuntimeToken {
        let id = self.next_token_id;
        self.next_token_id += 1;
        ActivityRuntimeToken {
            id,
            value,
            arrived_via_edge_id: None,
        }
    }
}

impl ExecutionEngine for ActivityExecutionEngine {
    fn initialize(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<(), ExecutionError> {
        session.initialize(project)?;
        self.initialize_embedded(project, session)
    }

    fn step(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<EngineStepOutcome, ExecutionError> {
        self.active_node_ids.clear();
        self.active_edge_ids.clear();
        let Some(frame_index) = self.frames.len().checked_sub(1) else {
            return Err(engine_error("Activity execution has no call frame.".into()));
        };
        if self.frames[frame_index].terminated {
            if self.frames[frame_index].caller.is_some() {
                return self.finish_child_frame(project, session);
            }
            session.complete()?;
            return Ok(EngineStepOutcome::Completed);
        }
        self.refresh_enabled(project, session, frame_index)?;
        let activity = self.activity(self.frames[frame_index].activity_id)?.clone();
        if let Some(node) = activity.nodes.iter().find(|node| {
            self.frames[frame_index].node_states.get(&node.id)
                == Some(&ActivityNodeExecutionState::Enabled)
        }) {
            let outcome = self.fire_node(project, session, frame_index, &activity, node)?;
            if self.frames[frame_index].terminated && self.frames[frame_index].caller.is_none() {
                session.complete()?;
                return Ok(EngineStepOutcome::Completed);
            }
            return Ok(outcome);
        }
        if self.frame_is_quiescent(frame_index) {
            if self.frames[frame_index].caller.is_some() {
                return self.finish_child_frame(project, session);
            }
            session.complete()?;
            return Ok(EngineStepOutcome::Completed);
        }
        Ok(EngineStepOutcome::Idle)
    }

    fn handle_event(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        event: &RuntimeEvent,
    ) -> Result<EngineStepOutcome, ExecutionError> {
        self.active_node_ids.clear();
        self.active_edge_ids.clear();
        for frame_index in (0..self.frames.len()).rev() {
            let activity = self.activity(self.frames[frame_index].activity_id)?.clone();
            if let Some(node) = activity.nodes.iter().find(|node| {
                self.frames[frame_index].node_states.get(&node.id)
                    == Some(&ActivityNodeExecutionState::Waiting)
                    && self.node_accepts_event(project, &self.frames[frame_index], node, event)
            }) {
                self.active_node_ids.push(node.id);
                self.complete_waiting_action(
                    project,
                    session,
                    frame_index,
                    &activity,
                    node,
                    event,
                )?;
                session.record_engine_trace(
                    activity.context_id.or(Some(activity.owner_id)),
                    format!(
                        "Activity '{}': '{}' accepted {} event '{}'",
                        activity.name,
                        display_node_name(node),
                        event_kind_label(event.kind),
                        event.name
                    ),
                );
                return Ok(EngineStepOutcome::Progressed);
            }
        }
        Ok(EngineStepOutcome::Idle)
    }
}

fn incoming_edges(
    activity: &Activity,
    node_id: ActivityNodeId,
    kind: ActivityEdgeKind,
) -> Vec<&ActivityEdge> {
    activity
        .edges
        .iter()
        .filter(|edge| edge.kind == kind && edge.target == ActivityEndpoint::Node(node_id))
        .collect()
}

fn outgoing_edges(
    activity: &Activity,
    source: ActivityEndpoint,
    kind: ActivityEdgeKind,
) -> Vec<&ActivityEdge> {
    activity
        .edges
        .iter()
        .filter(|edge| edge.kind == kind && edge.source == source)
        .collect()
}

fn endpoint_constraints(
    activity: &Activity,
    endpoint: ActivityEndpoint,
) -> Result<(Option<u32>, bool, String), ExecutionError> {
    match endpoint {
        ActivityEndpoint::Node(node_id) => {
            let node = activity
                .nodes
                .iter()
                .find(|node| node.id == node_id)
                .ok_or_else(|| engine_error("Activity edge targets a missing node.".into()))?;
            match &node.kind {
                ActivityNodeKind::Object(object) => Ok((
                    object.multiplicity.upper,
                    false,
                    display_node_name(node).into(),
                )),
                ActivityNodeKind::ActivityParameter(_) => {
                    Ok((None, false, display_node_name(node).into()))
                }
                _ => Ok((None, false, display_node_name(node).into())),
            }
        }
        ActivityEndpoint::Pin(pin_id) => {
            let pin = activity
                .nodes
                .iter()
                .filter_map(|node| match &node.kind {
                    ActivityNodeKind::Action(action) => Some(&action.pins),
                    _ => None,
                })
                .flatten()
                .find(|pin| pin.id == pin_id)
                .ok_or_else(|| engine_error("Activity edge targets a missing pin.".into()))?;
            Ok((pin.multiplicity.upper, pin.is_unique, pin.name.clone()))
        }
    }
}

fn endpoint_sort_key(endpoint: ActivityEndpoint) -> String {
    match endpoint {
        ActivityEndpoint::Node(id) => format!("node:{id}"),
        ActivityEndpoint::Pin(id) => format!("pin:{id}"),
    }
}

fn display_node_name(node: &ActivityNode) -> &str {
    if node.name.trim().is_empty() {
        match &node.kind {
            ActivityNodeKind::Initial => "Initial",
            ActivityNodeKind::ActivityFinal => "Activity Final",
            ActivityNodeKind::FlowFinal => "Flow Final",
            ActivityNodeKind::Decision { .. } => "Decision",
            ActivityNodeKind::Merge => "Merge",
            ActivityNodeKind::Fork => "Fork",
            ActivityNodeKind::Join { .. } => "Join",
            ActivityNodeKind::Action(_) => "Action",
            ActivityNodeKind::Object(_) => "Object Node",
            ActivityNodeKind::ActivityParameter(_) => "Activity Parameter",
        }
    } else {
        &node.name
    }
}

fn parse_authored_value(value: &str) -> Option<RuntimeValue> {
    evaluate_execution_expression(value, |_| None)
        .ok()
        .or_else(|| Some(RuntimeValue::Text(value.into())))
}

fn event_kind_label(kind: RuntimeEventKind) -> &'static str {
    match kind {
        RuntimeEventKind::Signal => "signal",
        RuntimeEventKind::Call => "call",
        RuntimeEventKind::Change => "change",
        RuntimeEventKind::Time => "time",
        RuntimeEventKind::Completion => "completion",
        RuntimeEventKind::Internal => "internal",
    }
}

fn engine_error(message: String) -> ExecutionError {
    ExecutionError::Engine { message }
}
