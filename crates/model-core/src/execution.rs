use crate::{ElementId, ElementKind, Project, ProjectId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExecutionSessionId(pub Uuid);

impl ExecutionSessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ExecutionSessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ExecutionSessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RuntimeInstanceId(pub Uuid);

impl RuntimeInstanceId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RuntimeInstanceId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RuntimeInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct SimulationTime(pub u64);

impl SimulationTime {
    pub const ZERO: Self = Self(0);

    pub const fn from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    pub const fn as_nanos(self) -> u64 {
        self.0
    }

    pub fn checked_add(self, nanos: u64) -> Option<Self> {
        self.0.checked_add(nanos).map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionState {
    Created,
    Initialized,
    Running,
    Paused,
    Completed,
    Failed,
    Terminated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuntimeValue {
    Unset,
    Boolean(bool),
    Integer(i64),
    Real(f64),
    Text(String),
    ElementReference(ElementId),
}

impl RuntimeValue {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Unset => "unset",
            Self::Boolean(_) => "boolean",
            Self::Integer(_) => "integer",
            Self::Real(_) => "real",
            Self::Text(_) => "text",
            Self::ElementReference(_) => "element reference",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimeValueKey {
    pub instance_id: Option<RuntimeInstanceId>,
    pub semantic_element_id: ElementId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeValueSnapshot {
    pub key: RuntimeValueKey,
    pub value: RuntimeValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeInstance {
    pub id: RuntimeInstanceId,
    pub semantic_element_id: ElementId,
    pub classifier_id: Option<ElementId>,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeEventKind {
    Signal,
    Call,
    Change,
    Time,
    Completion,
    Internal,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeEventAddress {
    pub source_semantic_id: Option<ElementId>,
    pub target_semantic_id: Option<ElementId>,
    pub source_runtime_instance_id: Option<RuntimeInstanceId>,
    pub target_runtime_instance_id: Option<RuntimeInstanceId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEvent {
    pub sequence: u64,
    pub kind: RuntimeEventKind,
    pub name: String,
    /// Stable semantic identity of the Signal, Operation, or other modeled
    /// event type. This is separate from source/target runtime addressing.
    #[serde(default)]
    pub semantic_event_id: Option<ElementId>,
    pub source_semantic_id: Option<ElementId>,
    pub target_semantic_id: Option<ElementId>,
    pub source_runtime_instance_id: Option<RuntimeInstanceId>,
    pub target_runtime_instance_id: Option<RuntimeInstanceId>,
    pub payload: Vec<(String, RuntimeValue)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEventRequest {
    pub due_time: SimulationTime,
    pub kind: RuntimeEventKind,
    pub name: String,
    pub semantic_event_id: Option<ElementId>,
    pub address: RuntimeEventAddress,
    pub payload: Vec<(String, RuntimeValue)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduledEvent {
    pub due_time: SimulationTime,
    pub event: RuntimeEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceKind {
    Session,
    StateChange,
    EventQueued,
    EventDispatched,
    ValueSet,
    ActiveSetChanged,
    Diagnostic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionTraceEntry {
    pub sequence: u64,
    pub simulation_time: SimulationTime,
    pub kind: TraceKind,
    pub semantic_element_id: Option<ElementId>,
    pub runtime_instance_id: Option<RuntimeInstanceId>,
    pub event_sequence: Option<u64>,
    pub source_semantic_id: Option<ElementId>,
    pub target_semantic_id: Option<ElementId>,
    pub source_runtime_instance_id: Option<RuntimeInstanceId>,
    pub target_runtime_instance_id: Option<RuntimeInstanceId>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionDiagnostic {
    pub severity: DiagnosticSeverity,
    pub semantic_element_id: Option<ElementId>,
    pub runtime_instance_id: Option<RuntimeInstanceId>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionConfiguration {
    pub root_semantic_id: ElementId,
    pub random_seed: u64,
    pub max_steps: u64,
    pub max_queued_events: usize,
}

impl ExecutionConfiguration {
    pub fn for_project(project: &Project) -> Self {
        Self {
            root_semantic_id: project.root_id,
            random_seed: 0,
            max_steps: 100_000,
            max_queued_events: 10_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionSnapshot {
    pub session_id: ExecutionSessionId,
    pub revision: u64,
    pub configuration: ExecutionConfiguration,
    pub state: ExecutionState,
    pub simulation_time: SimulationTime,
    pub steps_executed: u64,
    pub cancellation_requested: bool,
    pub runtime_instances: Vec<RuntimeInstance>,
    pub runtime_values: Vec<RuntimeValueSnapshot>,
    pub scheduled_events: Vec<ScheduledEvent>,
    pub active_semantic_element_ids: Vec<ElementId>,
    pub trace: Vec<ExecutionTraceEntry>,
    pub diagnostics: Vec<ExecutionDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineStepOutcome {
    Idle,
    Progressed,
    Completed,
}

/// Shared boundary for future executable semantic engines. Activity, State
/// Machine, and later engines implement this contract instead of embedding their
/// semantics directly in `ExecutionSession`.
pub trait ExecutionEngine {
    fn initialize(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<(), ExecutionError>;

    fn step(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<EngineStepOutcome, ExecutionError>;

    fn handle_event(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        event: &RuntimeEvent,
    ) -> Result<EngineStepOutcome, ExecutionError>;
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExecutionError {
    #[error("execution session belongs to a different project")]
    ProjectMismatch,
    #[error("execution root does not exist in the project: {0}")]
    ExecutionRootNotFound(ElementId),
    #[error("cannot {operation} while execution session is {state:?}")]
    InvalidState {
        operation: &'static str,
        state: ExecutionState,
    },
    #[error("runtime semantic element does not exist in the project")]
    SemanticElementNotFound,
    #[error("runtime classifier does not exist in the project")]
    ClassifierNotFound,
    #[error("runtime instance does not exist: {0}")]
    RuntimeInstanceNotFound(RuntimeInstanceId),
    #[error("runtime instance does not represent the supplied semantic element")]
    RuntimeInstanceSemanticMismatch,
    #[error("runtime value is not finite")]
    NonFiniteValue,
    #[error("runtime value kind '{actual}' is incompatible with '{expected}' for '{element}'")]
    RuntimeValueTypeMismatch {
        element: String,
        expected: String,
        actual: String,
    },
    #[error("runtime value references a semantic element that does not exist")]
    RuntimeReferenceNotFound,
    #[error("scheduled simulation time overflow")]
    SimulationTimeOverflow,
    #[error("execution step limit exceeded: {limit}")]
    StepLimitExceeded { limit: u64 },
    #[error("execution event queue limit exceeded: {limit}")]
    EventQueueLimitExceeded { limit: usize },
    #[error("execution cancellation was requested")]
    CancellationRequested,
    #[error("execution session not found: {0}")]
    SessionNotFound(ExecutionSessionId),
    #[error("{message}")]
    Engine { message: String },
}

#[derive(Debug, Clone, Default)]
struct TraceContext {
    semantic_element_id: Option<ElementId>,
    runtime_instance_id: Option<RuntimeInstanceId>,
    event_sequence: Option<u64>,
    source_semantic_id: Option<ElementId>,
    target_semantic_id: Option<ElementId>,
    source_runtime_instance_id: Option<RuntimeInstanceId>,
    target_runtime_instance_id: Option<RuntimeInstanceId>,
}

#[derive(Debug, Clone)]
pub struct ExecutionSession {
    pub id: ExecutionSessionId,
    pub project_id: ProjectId,
    pub project_root_id: ElementId,
    pub configuration: ExecutionConfiguration,
    pub state: ExecutionState,
    pub revision: u64,
    pub simulation_time: SimulationTime,
    pub instances: HashMap<RuntimeInstanceId, RuntimeInstance>,
    pub values: HashMap<RuntimeValueKey, RuntimeValue>,
    pub event_queue: VecDeque<ScheduledEvent>,
    pub trace: Vec<ExecutionTraceEntry>,
    pub diagnostics: Vec<ExecutionDiagnostic>,
    pub steps_executed: u64,
    pub cancellation_requested: bool,
    active_semantic_elements: HashSet<ElementId>,
    next_event_sequence: u64,
    next_trace_sequence: u64,
}

impl ExecutionSession {
    pub fn new(project: &Project) -> Self {
        Self::with_configuration(project, ExecutionConfiguration::for_project(project))
            .expect("project root must exist")
    }

    pub fn with_configuration(
        project: &Project,
        configuration: ExecutionConfiguration,
    ) -> Result<Self, ExecutionError> {
        if !project
            .elements
            .contains_key(&configuration.root_semantic_id)
        {
            return Err(ExecutionError::ExecutionRootNotFound(
                configuration.root_semantic_id,
            ));
        }
        Ok(Self {
            id: ExecutionSessionId::new(),
            project_id: project.id,
            project_root_id: project.root_id,
            configuration,
            state: ExecutionState::Created,
            revision: 0,
            simulation_time: SimulationTime::ZERO,
            instances: HashMap::new(),
            values: HashMap::new(),
            event_queue: VecDeque::new(),
            trace: Vec::new(),
            diagnostics: Vec::new(),
            steps_executed: 0,
            cancellation_requested: false,
            active_semantic_elements: HashSet::new(),
            next_event_sequence: 0,
            next_trace_sequence: 0,
        })
    }

    pub fn initialize(&mut self, project: &Project) -> Result<(), ExecutionError> {
        self.require_project(project)?;
        if !matches!(
            self.state,
            ExecutionState::Created
                | ExecutionState::Completed
                | ExecutionState::Failed
                | ExecutionState::Terminated
        ) {
            return Err(self.invalid_state("initialize"));
        }
        if !project
            .elements
            .contains_key(&self.configuration.root_semantic_id)
        {
            return Err(ExecutionError::ExecutionRootNotFound(
                self.configuration.root_semantic_id,
            ));
        }
        self.clear_runtime_state();
        self.state = ExecutionState::Initialized;
        self.push_trace(
            TraceKind::Session,
            TraceContext {
                semantic_element_id: Some(self.configuration.root_semantic_id),
                ..TraceContext::default()
            },
            format!("Initialized execution session for {}", project.name),
        );
        self.touch();
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), ExecutionError> {
        match self.state {
            ExecutionState::Initialized | ExecutionState::Paused => {
                self.state = ExecutionState::Running;
                self.push_trace(
                    TraceKind::StateChange,
                    TraceContext::default(),
                    "Execution running".into(),
                );
                self.touch();
                Ok(())
            }
            _ => Err(self.invalid_state("run")),
        }
    }

    pub fn pause(&mut self) -> Result<(), ExecutionError> {
        if self.state != ExecutionState::Running {
            return Err(self.invalid_state("pause"));
        }
        self.state = ExecutionState::Paused;
        self.push_trace(
            TraceKind::StateChange,
            TraceContext::default(),
            "Execution paused".into(),
        );
        self.touch();
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), ExecutionError> {
        self.run()
    }

    /// Dispatches one scheduled event in deterministic simulation-time/sequence
    /// order. Future semantic engines consume the returned event and apply their
    /// own Activity/State Machine behavior.
    pub fn step(&mut self) -> Result<Option<RuntimeEvent>, ExecutionError> {
        if !matches!(
            self.state,
            ExecutionState::Initialized | ExecutionState::Running | ExecutionState::Paused
        ) {
            return Err(self.invalid_state("step"));
        }
        self.consume_step_budget()?;
        let scheduled = self.event_queue.pop_front();
        let Some(scheduled) = scheduled else {
            return Ok(None);
        };
        if scheduled.due_time > self.simulation_time {
            self.simulation_time = scheduled.due_time;
        }
        let event = scheduled.event;
        self.push_trace(
            TraceKind::EventDispatched,
            TraceContext {
                semantic_element_id: event.target_semantic_id,
                runtime_instance_id: event.target_runtime_instance_id,
                event_sequence: Some(event.sequence),
                source_semantic_id: event.source_semantic_id,
                target_semantic_id: event.target_semantic_id,
                source_runtime_instance_id: event.source_runtime_instance_id,
                target_runtime_instance_id: event.target_runtime_instance_id,
            },
            format!(
                "Dispatched {} event '{}'",
                event_kind_name(event.kind),
                event.name
            ),
        );
        self.touch();
        Ok(Some(event))
    }

    /// Consumes one semantic step from the configured deterministic execution
    /// budget. Future engines call this for internal steps that do not dispatch
    /// a queued event.
    pub fn consume_step_budget(&mut self) -> Result<u64, ExecutionError> {
        if self.cancellation_requested {
            self.state = ExecutionState::Terminated;
            self.diagnostics.push(ExecutionDiagnostic {
                severity: DiagnosticSeverity::Info,
                semantic_element_id: Some(self.configuration.root_semantic_id),
                runtime_instance_id: None,
                message: "Execution cancelled".into(),
            });
            self.push_trace(
                TraceKind::Diagnostic,
                TraceContext {
                    semantic_element_id: Some(self.configuration.root_semantic_id),
                    ..TraceContext::default()
                },
                "Execution cancelled".into(),
            );
            self.touch();
            return Err(ExecutionError::CancellationRequested);
        }
        if self.steps_executed >= self.configuration.max_steps {
            self.state = ExecutionState::Failed;
            let message = format!(
                "Execution stopped after reaching the configured {} step limit",
                self.configuration.max_steps
            );
            self.diagnostics.push(ExecutionDiagnostic {
                severity: DiagnosticSeverity::Error,
                semantic_element_id: Some(self.configuration.root_semantic_id),
                runtime_instance_id: None,
                message: message.clone(),
            });
            self.push_trace(
                TraceKind::Diagnostic,
                TraceContext {
                    semantic_element_id: Some(self.configuration.root_semantic_id),
                    ..TraceContext::default()
                },
                message,
            );
            self.touch();
            return Err(ExecutionError::StepLimitExceeded {
                limit: self.configuration.max_steps,
            });
        }
        let sequence = self.steps_executed;
        self.steps_executed += 1;
        self.touch();
        Ok(sequence)
    }

    pub fn complete(&mut self) -> Result<(), ExecutionError> {
        if !matches!(
            self.state,
            ExecutionState::Initialized | ExecutionState::Running | ExecutionState::Paused
        ) {
            return Err(self.invalid_state("complete"));
        }
        self.state = ExecutionState::Completed;
        self.push_trace(
            TraceKind::StateChange,
            TraceContext::default(),
            "Execution completed".into(),
        );
        self.touch();
        Ok(())
    }

    pub fn fail(&mut self, semantic_element_id: Option<ElementId>, message: impl Into<String>) {
        let message = message.into();
        self.state = ExecutionState::Failed;
        self.diagnostics.push(ExecutionDiagnostic {
            severity: DiagnosticSeverity::Error,
            semantic_element_id,
            runtime_instance_id: None,
            message: message.clone(),
        });
        self.push_trace(
            TraceKind::Diagnostic,
            TraceContext {
                semantic_element_id,
                ..TraceContext::default()
            },
            message,
        );
        self.touch();
    }

    /// Adds an engine-owned diagnostic without exposing the session's storage
    /// implementation to Activity, State Machine, or later execution engines.
    pub fn add_diagnostic(
        &mut self,
        severity: DiagnosticSeverity,
        semantic_element_id: Option<ElementId>,
        message: impl Into<String>,
    ) {
        let message = message.into();
        self.diagnostics.push(ExecutionDiagnostic {
            severity,
            semantic_element_id,
            runtime_instance_id: None,
            message: message.clone(),
        });
        self.push_trace(
            TraceKind::Diagnostic,
            TraceContext {
                semantic_element_id,
                ..TraceContext::default()
            },
            message,
        );
        self.touch();
    }

    /// Records a deterministic semantic engine step. The message must use
    /// model names rather than UUIDs for normal execution UX.
    pub fn record_engine_trace(
        &mut self,
        semantic_element_id: Option<ElementId>,
        message: impl Into<String>,
    ) {
        self.push_trace(
            TraceKind::StateChange,
            TraceContext {
                semantic_element_id,
                ..TraceContext::default()
            },
            message.into(),
        );
        self.touch();
    }

    pub fn terminate(&mut self) -> Result<(), ExecutionError> {
        if self.state == ExecutionState::Terminated {
            return Err(self.invalid_state("terminate"));
        }
        self.state = ExecutionState::Terminated;
        self.push_trace(
            TraceKind::StateChange,
            TraceContext::default(),
            "Execution terminated".into(),
        );
        self.touch();
        Ok(())
    }

    pub fn reset(&mut self, project: &Project) -> Result<(), ExecutionError> {
        self.require_project(project)?;
        self.clear_runtime_state();
        self.state = ExecutionState::Initialized;
        self.push_trace(
            TraceKind::Session,
            TraceContext {
                semantic_element_id: Some(self.configuration.root_semantic_id),
                ..TraceContext::default()
            },
            format!("Reset execution session for {}", project.name),
        );
        self.touch();
        Ok(())
    }

    pub fn request_cancellation(&mut self) {
        if !self.cancellation_requested {
            self.cancellation_requested = true;
            self.touch();
        }
    }

    pub fn clear_cancellation(&mut self) {
        if self.cancellation_requested {
            self.cancellation_requested = false;
            self.touch();
        }
    }

    pub fn create_instance(
        &mut self,
        project: &Project,
        semantic_element_id: ElementId,
        classifier_id: Option<ElementId>,
    ) -> Result<RuntimeInstanceId, ExecutionError> {
        self.require_project(project)?;
        let element = project
            .elements
            .get(&semantic_element_id)
            .ok_or(ExecutionError::SemanticElementNotFound)?;
        if let Some(classifier_id) = classifier_id
            && !project.elements.contains_key(&classifier_id)
        {
            return Err(ExecutionError::ClassifierNotFound);
        }
        let id = RuntimeInstanceId::new();
        self.instances.insert(
            id,
            RuntimeInstance {
                id,
                semantic_element_id,
                classifier_id,
                name: element.name.clone(),
            },
        );
        self.touch();
        Ok(id)
    }

    pub fn set_value(
        &mut self,
        project: &Project,
        instance_id: Option<RuntimeInstanceId>,
        semantic_element_id: ElementId,
        value: RuntimeValue,
    ) -> Result<(), ExecutionError> {
        self.require_project(project)?;
        let element = project
            .elements
            .get(&semantic_element_id)
            .ok_or(ExecutionError::SemanticElementNotFound)?;
        if let Some(instance_id) = instance_id {
            self.require_instance_semantic(instance_id, None)?;
        }
        validate_runtime_assignment(project, element, &value)?;
        self.values.insert(
            RuntimeValueKey {
                instance_id,
                semantic_element_id,
            },
            value,
        );
        self.push_trace(
            TraceKind::ValueSet,
            TraceContext {
                semantic_element_id: Some(semantic_element_id),
                runtime_instance_id: instance_id,
                ..TraceContext::default()
            },
            format!("Updated runtime value for {}", element.name),
        );
        self.touch();
        Ok(())
    }

    pub fn value(
        &self,
        instance_id: Option<RuntimeInstanceId>,
        semantic_element_id: ElementId,
    ) -> Option<&RuntimeValue> {
        self.values.get(&RuntimeValueKey {
            instance_id,
            semantic_element_id,
        })
    }

    pub fn queue_event(
        &mut self,
        project: &Project,
        kind: RuntimeEventKind,
        name: impl Into<String>,
        source_semantic_id: Option<ElementId>,
        target_semantic_id: Option<ElementId>,
        payload: Vec<(String, RuntimeValue)>,
    ) -> Result<u64, ExecutionError> {
        self.queue_event_at(
            project,
            self.simulation_time,
            kind,
            name,
            RuntimeEventAddress {
                source_semantic_id,
                target_semantic_id,
                ..RuntimeEventAddress::default()
            },
            payload,
        )
    }

    pub fn queue_event_after(
        &mut self,
        project: &Project,
        delay_nanos: u64,
        kind: RuntimeEventKind,
        name: impl Into<String>,
        address: RuntimeEventAddress,
        payload: Vec<(String, RuntimeValue)>,
    ) -> Result<u64, ExecutionError> {
        let due_time = self
            .simulation_time
            .checked_add(delay_nanos)
            .ok_or(ExecutionError::SimulationTimeOverflow)?;
        self.queue_event_at(project, due_time, kind, name, address, payload)
    }

    pub fn queue_event_at(
        &mut self,
        project: &Project,
        due_time: SimulationTime,
        kind: RuntimeEventKind,
        name: impl Into<String>,
        address: RuntimeEventAddress,
        payload: Vec<(String, RuntimeValue)>,
    ) -> Result<u64, ExecutionError> {
        self.queue_typed_event_at(
            project,
            RuntimeEventRequest {
                due_time,
                kind,
                name: name.into(),
                semantic_event_id: None,
                address,
                payload,
            },
        )
    }

    pub fn queue_typed_event_at(
        &mut self,
        project: &Project,
        request: RuntimeEventRequest,
    ) -> Result<u64, ExecutionError> {
        self.require_project(project)?;
        if self.event_queue.len() >= self.configuration.max_queued_events {
            return Err(ExecutionError::EventQueueLimitExceeded {
                limit: self.configuration.max_queued_events,
            });
        }
        for id in request
            .address
            .source_semantic_id
            .into_iter()
            .chain(request.address.target_semantic_id)
        {
            if !project.elements.contains_key(&id) {
                return Err(ExecutionError::SemanticElementNotFound);
            }
        }
        if request
            .semantic_event_id
            .is_some_and(|id| !project.elements.contains_key(&id))
        {
            return Err(ExecutionError::SemanticElementNotFound);
        }
        let source_semantic_id = self.resolve_instance_semantic(
            request.address.source_runtime_instance_id,
            request.address.source_semantic_id,
        )?;
        let target_semantic_id = self.resolve_instance_semantic(
            request.address.target_runtime_instance_id,
            request.address.target_semantic_id,
        )?;
        for (_, value) in &request.payload {
            validate_runtime_payload_value(project, value)?;
        }

        let sequence = self.next_event_sequence;
        self.next_event_sequence += 1;
        let event = RuntimeEvent {
            sequence,
            kind: request.kind,
            name: request.name,
            semantic_event_id: request.semantic_event_id,
            source_semantic_id,
            target_semantic_id,
            source_runtime_instance_id: request.address.source_runtime_instance_id,
            target_runtime_instance_id: request.address.target_runtime_instance_id,
            payload: request.payload,
        };
        self.push_trace(
            TraceKind::EventQueued,
            TraceContext {
                semantic_element_id: target_semantic_id,
                runtime_instance_id: request.address.target_runtime_instance_id,
                event_sequence: Some(sequence),
                source_semantic_id,
                target_semantic_id,
                source_runtime_instance_id: request.address.source_runtime_instance_id,
                target_runtime_instance_id: request.address.target_runtime_instance_id,
            },
            format!(
                "Queued {} event '{}'",
                event_kind_name(request.kind),
                event.name
            ),
        );
        self.insert_scheduled_event(ScheduledEvent {
            due_time: request.due_time,
            event,
        });
        self.touch();
        Ok(sequence)
    }

    pub fn next_event(&self) -> Option<&ScheduledEvent> {
        self.event_queue.front()
    }

    pub fn set_active_semantic_elements(
        &mut self,
        project: &Project,
        element_ids: impl IntoIterator<Item = ElementId>,
    ) -> Result<(), ExecutionError> {
        self.require_project(project)?;
        let mut next = HashSet::new();
        for id in element_ids {
            if !project.elements.contains_key(&id) {
                return Err(ExecutionError::SemanticElementNotFound);
            }
            next.insert(id);
        }
        if next != self.active_semantic_elements {
            self.active_semantic_elements = next;
            self.push_trace(
                TraceKind::ActiveSetChanged,
                TraceContext::default(),
                "Active semantic set changed".into(),
            );
            self.touch();
        }
        Ok(())
    }

    pub fn snapshot(&self) -> ExecutionSnapshot {
        let mut runtime_instances: Vec<_> = self.instances.values().cloned().collect();
        runtime_instances.sort_by_key(|instance| instance.id.to_string());

        let mut runtime_values: Vec<_> = self
            .values
            .iter()
            .map(|(key, value)| RuntimeValueSnapshot {
                key: *key,
                value: value.clone(),
            })
            .collect();
        runtime_values.sort_by(|left, right| runtime_value_key_sort(&left.key, &right.key));

        let mut active_semantic_element_ids: Vec<_> =
            self.active_semantic_elements.iter().copied().collect();
        active_semantic_element_ids.sort_by_key(ToString::to_string);

        ExecutionSnapshot {
            session_id: self.id,
            revision: self.revision,
            configuration: self.configuration.clone(),
            state: self.state,
            simulation_time: self.simulation_time,
            steps_executed: self.steps_executed,
            cancellation_requested: self.cancellation_requested,
            runtime_instances,
            runtime_values,
            scheduled_events: self.event_queue.iter().cloned().collect(),
            active_semantic_element_ids,
            trace: self.trace.clone(),
            diagnostics: self.diagnostics.clone(),
        }
    }

    fn insert_scheduled_event(&mut self, scheduled: ScheduledEvent) {
        let position = self
            .event_queue
            .iter()
            .position(|existing| {
                (scheduled.due_time, scheduled.event.sequence)
                    < (existing.due_time, existing.event.sequence)
            })
            .unwrap_or(self.event_queue.len());
        self.event_queue.insert(position, scheduled);
    }

    fn resolve_instance_semantic(
        &self,
        instance_id: Option<RuntimeInstanceId>,
        semantic_id: Option<ElementId>,
    ) -> Result<Option<ElementId>, ExecutionError> {
        let Some(instance_id) = instance_id else {
            return Ok(semantic_id);
        };
        let instance = self.require_instance_semantic(instance_id, semantic_id)?;
        Ok(Some(instance.semantic_element_id))
    }

    fn require_instance_semantic(
        &self,
        instance_id: RuntimeInstanceId,
        semantic_id: Option<ElementId>,
    ) -> Result<&RuntimeInstance, ExecutionError> {
        let instance = self
            .instances
            .get(&instance_id)
            .ok_or(ExecutionError::RuntimeInstanceNotFound(instance_id))?;
        if semantic_id.is_some_and(|id| id != instance.semantic_element_id) {
            return Err(ExecutionError::RuntimeInstanceSemanticMismatch);
        }
        Ok(instance)
    }

    fn clear_runtime_state(&mut self) {
        self.instances.clear();
        self.values.clear();
        self.event_queue.clear();
        self.trace.clear();
        self.diagnostics.clear();
        self.active_semantic_elements.clear();
        self.simulation_time = SimulationTime::ZERO;
        self.steps_executed = 0;
        self.cancellation_requested = false;
        self.next_event_sequence = 0;
        self.next_trace_sequence = 0;
    }

    fn require_project(&self, project: &Project) -> Result<(), ExecutionError> {
        if self.project_id != project.id || self.project_root_id != project.root_id {
            return Err(ExecutionError::ProjectMismatch);
        }
        Ok(())
    }

    fn invalid_state(&self, operation: &'static str) -> ExecutionError {
        ExecutionError::InvalidState {
            operation,
            state: self.state,
        }
    }

    fn touch(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    fn push_trace(&mut self, kind: TraceKind, context: TraceContext, message: String) {
        let sequence = self.next_trace_sequence;
        self.next_trace_sequence += 1;
        self.trace.push(ExecutionTraceEntry {
            sequence,
            simulation_time: self.simulation_time,
            kind,
            semantic_element_id: context.semantic_element_id,
            runtime_instance_id: context.runtime_instance_id,
            event_sequence: context.event_sequence,
            source_semantic_id: context.source_semantic_id,
            target_semantic_id: context.target_semantic_id,
            source_runtime_instance_id: context.source_runtime_instance_id,
            target_runtime_instance_id: context.target_runtime_instance_id,
            message,
        });
    }
}

#[derive(Debug, Default)]
pub struct ExecutionManager {
    sessions: HashMap<ExecutionSessionId, ExecutionSession>,
}

impl ExecutionManager {
    pub fn create_session(
        &mut self,
        project: &Project,
        configuration: ExecutionConfiguration,
    ) -> Result<ExecutionSessionId, ExecutionError> {
        let session = ExecutionSession::with_configuration(project, configuration)?;
        let id = session.id;
        self.sessions.insert(id, session);
        Ok(id)
    }

    pub fn create_default_session(&mut self, project: &Project) -> ExecutionSessionId {
        let session = ExecutionSession::new(project);
        let id = session.id;
        self.sessions.insert(id, session);
        id
    }

    pub fn session(&self, id: ExecutionSessionId) -> Result<&ExecutionSession, ExecutionError> {
        self.sessions
            .get(&id)
            .ok_or(ExecutionError::SessionNotFound(id))
    }

    pub fn session_mut(
        &mut self,
        id: ExecutionSessionId,
    ) -> Result<&mut ExecutionSession, ExecutionError> {
        self.sessions
            .get_mut(&id)
            .ok_or(ExecutionError::SessionNotFound(id))
    }

    pub fn terminate_session(&mut self, id: ExecutionSessionId) -> Result<(), ExecutionError> {
        self.session_mut(id)?.terminate()
    }

    pub fn remove_session(&mut self, id: ExecutionSessionId) -> Option<ExecutionSession> {
        self.sessions.remove(&id)
    }

    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

fn validate_runtime_assignment(
    project: &Project,
    element: &crate::Element,
    value: &RuntimeValue,
) -> Result<(), ExecutionError> {
    validate_runtime_payload_value(project, value)?;
    let Some(type_id) = element.type_id else {
        return Ok(());
    };
    let expected = project
        .elements
        .get(&type_id)
        .ok_or(ExecutionError::ClassifierNotFound)?;
    if runtime_value_matches_type(expected, value) {
        return Ok(());
    }
    Err(ExecutionError::RuntimeValueTypeMismatch {
        element: element.name.clone(),
        expected: expected.name.clone(),
        actual: value.kind_name().into(),
    })
}

fn validate_runtime_payload_value(
    project: &Project,
    value: &RuntimeValue,
) -> Result<(), ExecutionError> {
    if matches!(value, RuntimeValue::Real(number) if !number.is_finite()) {
        return Err(ExecutionError::NonFiniteValue);
    }
    if let RuntimeValue::ElementReference(id) = value
        && !project.elements.contains_key(id)
    {
        return Err(ExecutionError::RuntimeReferenceNotFound);
    }
    Ok(())
}

fn runtime_value_matches_type(expected: &crate::Element, value: &RuntimeValue) -> bool {
    if matches!(value, RuntimeValue::Unset) {
        return true;
    }
    match expected.kind {
        ElementKind::PrimitiveType => match expected.name.trim().to_ascii_lowercase().as_str() {
            "boolean" | "bool" => matches!(value, RuntimeValue::Boolean(_)),
            "integer" | "int" => matches!(value, RuntimeValue::Integer(_)),
            "real" | "double" | "float" => {
                matches!(value, RuntimeValue::Integer(_) | RuntimeValue::Real(_))
            }
            "string" | "text" => matches!(value, RuntimeValue::Text(_)),
            _ => !matches!(value, RuntimeValue::ElementReference(_)),
        },
        ElementKind::Enumeration => matches!(value, RuntimeValue::Text(_)),
        ElementKind::Block
        | ElementKind::AssociationBlock
        | ElementKind::InterfaceBlock
        | ElementKind::ConstraintBlock
        | ElementKind::Signal
        | ElementKind::Actor
        | ElementKind::UseCase
        | ElementKind::Requirement
        | ElementKind::TestCase => matches!(value, RuntimeValue::ElementReference(_)),
        ElementKind::ValueType | ElementKind::DataType => {
            !matches!(value, RuntimeValue::ElementReference(_))
        }
        _ => true,
    }
}

fn runtime_value_key_sort(left: &RuntimeValueKey, right: &RuntimeValueKey) -> std::cmp::Ordering {
    left.instance_id
        .map(|id| id.to_string())
        .cmp(&right.instance_id.map(|id| id.to_string()))
        .then_with(|| {
            left.semantic_element_id
                .to_string()
                .cmp(&right.semantic_element_id.to_string())
        })
}

fn event_kind_name(kind: RuntimeEventKind) -> &'static str {
    match kind {
        RuntimeEventKind::Signal => "signal",
        RuntimeEventKind::Call => "call",
        RuntimeEventKind::Change => "change",
        RuntimeEventKind::Time => "time",
        RuntimeEventKind::Completion => "completion",
        RuntimeEventKind::Internal => "internal",
    }
}
