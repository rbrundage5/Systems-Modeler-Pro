use crate::{ElementId, Project, ProjectId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimeValueKey {
    pub instance_id: Option<RuntimeInstanceId>,
    pub semantic_element_id: ElementId,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeEvent {
    pub sequence: u64,
    pub kind: RuntimeEventKind,
    pub name: String,
    pub source_semantic_id: Option<ElementId>,
    pub target_semantic_id: Option<ElementId>,
    pub payload: Vec<(String, RuntimeValue)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TraceKind {
    Session,
    StateChange,
    EventQueued,
    EventDispatched,
    ValueSet,
    Diagnostic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionTraceEntry {
    pub sequence: u64,
    pub kind: TraceKind,
    pub semantic_element_id: Option<ElementId>,
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
    pub message: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExecutionError {
    #[error("execution session belongs to a different project")]
    ProjectMismatch,
    #[error("cannot {operation} while execution session is {state:?}")]
    InvalidState {
        operation: &'static str,
        state: ExecutionState,
    },
    #[error("runtime semantic element does not exist in the project")]
    SemanticElementNotFound,
    #[error("runtime classifier does not exist in the project")]
    ClassifierNotFound,
    #[error("runtime instance does not exist")]
    RuntimeInstanceNotFound,
    #[error("runtime value is not finite")]
    NonFiniteValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSession {
    pub id: ExecutionSessionId,
    pub project_id: ProjectId,
    pub project_root_id: ElementId,
    pub state: ExecutionState,
    pub instances: HashMap<RuntimeInstanceId, RuntimeInstance>,
    pub values: HashMap<RuntimeValueKey, RuntimeValue>,
    pub event_queue: VecDeque<RuntimeEvent>,
    pub trace: Vec<ExecutionTraceEntry>,
    pub diagnostics: Vec<ExecutionDiagnostic>,
    next_event_sequence: u64,
    next_trace_sequence: u64,
}

impl ExecutionSession {
    pub fn new(project: &Project) -> Self {
        Self {
            id: ExecutionSessionId::new(),
            project_id: project.id,
            project_root_id: project.root_id,
            state: ExecutionState::Created,
            instances: HashMap::new(),
            values: HashMap::new(),
            event_queue: VecDeque::new(),
            trace: Vec::new(),
            diagnostics: Vec::new(),
            next_event_sequence: 0,
            next_trace_sequence: 0,
        }
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
        self.clear_runtime_state();
        self.state = ExecutionState::Initialized;
        self.push_trace(
            TraceKind::Session,
            Some(project.root_id),
            format!("Initialized execution session for {}", project.name),
        );
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), ExecutionError> {
        match self.state {
            ExecutionState::Initialized | ExecutionState::Paused => {
                self.state = ExecutionState::Running;
                self.push_trace(TraceKind::StateChange, None, "Execution running".into());
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
        self.push_trace(TraceKind::StateChange, None, "Execution paused".into());
        Ok(())
    }

    pub fn resume(&mut self) -> Result<(), ExecutionError> {
        self.run()
    }

    /// Dispatches one queued event in deterministic FIFO order. This foundation
    /// does not execute Activity or State Machine semantics yet; future engines
    /// consume the returned event and apply their own semantic step.
    pub fn step(&mut self) -> Result<Option<RuntimeEvent>, ExecutionError> {
        if !matches!(
            self.state,
            ExecutionState::Initialized | ExecutionState::Running | ExecutionState::Paused
        ) {
            return Err(self.invalid_state("step"));
        }
        let event = self.event_queue.pop_front();
        if let Some(event) = event.as_ref() {
            self.push_trace(
                TraceKind::EventDispatched,
                event.target_semantic_id,
                format!("Dispatched {} event '{}'", event_kind_name(event.kind), event.name),
            );
        }
        Ok(event)
    }

    pub fn complete(&mut self) -> Result<(), ExecutionError> {
        if !matches!(
            self.state,
            ExecutionState::Initialized | ExecutionState::Running | ExecutionState::Paused
        ) {
            return Err(self.invalid_state("complete"));
        }
        self.state = ExecutionState::Completed;
        self.push_trace(TraceKind::StateChange, None, "Execution completed".into());
        Ok(())
    }

    pub fn fail(
        &mut self,
        semantic_element_id: Option<ElementId>,
        message: impl Into<String>,
    ) {
        let message = message.into();
        self.state = ExecutionState::Failed;
        self.diagnostics.push(ExecutionDiagnostic {
            severity: DiagnosticSeverity::Error,
            semantic_element_id,
            message: message.clone(),
        });
        self.push_trace(TraceKind::Diagnostic, semantic_element_id, message);
    }

    pub fn terminate(&mut self) -> Result<(), ExecutionError> {
        if self.state == ExecutionState::Terminated {
            return Err(self.invalid_state("terminate"));
        }
        self.state = ExecutionState::Terminated;
        self.push_trace(TraceKind::StateChange, None, "Execution terminated".into());
        Ok(())
    }

    pub fn reset(&mut self, project: &Project) -> Result<(), ExecutionError> {
        self.require_project(project)?;
        self.clear_runtime_state();
        self.state = ExecutionState::Initialized;
        self.push_trace(
            TraceKind::Session,
            Some(project.root_id),
            format!("Reset execution session for {}", project.name),
        );
        Ok(())
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
        if let Some(instance_id) = instance_id
            && !self.instances.contains_key(&instance_id)
        {
            return Err(ExecutionError::RuntimeInstanceNotFound);
        }
        if matches!(&value, RuntimeValue::Real(number) if !number.is_finite()) {
            return Err(ExecutionError::NonFiniteValue);
        }
        self.values.insert(
            RuntimeValueKey {
                instance_id,
                semantic_element_id,
            },
            value,
        );
        self.push_trace(
            TraceKind::ValueSet,
            Some(semantic_element_id),
            format!("Updated runtime value for {}", element.name),
        );
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
        self.require_project(project)?;
        for id in source_semantic_id.into_iter().chain(target_semantic_id) {
            if !project.elements.contains_key(&id) {
                return Err(ExecutionError::SemanticElementNotFound);
            }
        }
        for (_, value) in &payload {
            if matches!(value, RuntimeValue::Real(number) if !number.is_finite()) {
                return Err(ExecutionError::NonFiniteValue);
            }
        }

        let sequence = self.next_event_sequence;
        self.next_event_sequence += 1;
        let event = RuntimeEvent {
            sequence,
            kind,
            name: name.into(),
            source_semantic_id,
            target_semantic_id,
            payload,
        };
        self.push_trace(
            TraceKind::EventQueued,
            target_semantic_id,
            format!("Queued {} event '{}'", event_kind_name(kind), event.name),
        );
        self.event_queue.push_back(event);
        Ok(sequence)
    }

    pub fn next_event(&self) -> Option<&RuntimeEvent> {
        self.event_queue.front()
    }

    fn clear_runtime_state(&mut self) {
        self.instances.clear();
        self.values.clear();
        self.event_queue.clear();
        self.trace.clear();
        self.diagnostics.clear();
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

    fn push_trace(
        &mut self,
        kind: TraceKind,
        semantic_element_id: Option<ElementId>,
        message: String,
    ) {
        let sequence = self.next_trace_sequence;
        self.next_trace_sequence += 1;
        self.trace.push(ExecutionTraceEntry {
            sequence,
            kind,
            semantic_element_id,
            message,
        });
    }
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
