use super::StateMachineExecutionEngine;
use crate::{
    ActionKind, ActivityExecutionEngine, ActivityId, ActivityNodeExecutionState, ActivityNodeKind,
    ActivityRepository, EngineStepOutcome, ExecutionEngine, ExecutionError, ExecutionSession,
    ExecutionState, Project, RuntimeEvent, RuntimeEventKind, State, Vertex, VertexId,
};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct StateActivityRuntime {
    repository: Option<ActivityRepository>,
    do_activities: HashMap<VertexId, ActivityExecutionEngine>,
    completed_do_activities: HashSet<VertexId>,
    time_event_sequences: HashMap<VertexId, HashSet<u64>>,
}

impl StateActivityRuntime {
    fn clear_execution(&mut self) {
        self.do_activities.clear();
        self.completed_do_activities.clear();
        self.time_event_sequences.clear();
    }
}

impl StateMachineExecutionEngine {
    pub fn with_activity_repository(mut self, repository: ActivityRepository) -> Self {
        self.state_activity_runtime.repository = Some(repository);
        self
    }

    pub(super) fn clear_state_activity_execution(&mut self) {
        self.state_activity_runtime.clear_execution();
    }

    pub(super) fn activate_state_activities(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        vertex: &Vertex,
        state: &State,
    ) -> Result<(), ExecutionError> {
        if let Some(reference) = non_empty(state.entry.as_deref()) {
            self.execute_synchronous_state_activity(project, session, vertex, reference, "entry")?;
        }
        if let Some(reference) = non_empty(state.do_activity.as_deref()) {
            let activity_id = self.parse_state_activity_id(vertex, "doActivity", reference)?;
            let activity_name = self
                .activity_name(activity_id)
                .unwrap_or("unresolved Activity")
                .to_string();
            let engine = self.build_embedded_activity_engine(project, session, activity_id)?;
            self.state_activity_runtime
                .do_activities
                .insert(vertex.id, engine);
            self.state_activity_runtime
                .completed_do_activities
                .remove(&vertex.id);
            self.state_activity_runtime
                .time_event_sequences
                .remove(&vertex.id);
            session.record_engine_trace(
                Some(self.machine()?.context_id),
                format!(
                    "State '{}' started doActivity '{}'",
                    vertex.name, activity_name
                ),
            );
        }
        Ok(())
    }

    pub(super) fn exit_state_activities(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        vertex: &Vertex,
        state: &State,
    ) -> Result<(), ExecutionError> {
        if self
            .state_activity_runtime
            .do_activities
            .contains_key(&vertex.id)
        {
            self.cancel_do_activity(session, vertex.id);
            session.record_engine_trace(
                Some(self.machine()?.context_id),
                format!("State '{}' terminated doActivity on exit", vertex.name),
            );
        }
        if let Some(reference) = non_empty(state.exit.as_deref()) {
            self.execute_synchronous_state_activity(project, session, vertex, reference, "exit")?;
        }
        Ok(())
    }

    pub(super) fn advance_state_do_activities(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<Option<EngineStepOutcome>, ExecutionError> {
        // A doActivity is asynchronous with respect to its owning State. If an event is
        // already queued that enables an owning State Machine transition, yield before
        // consuming another Activity step so a continuously progressing doActivity
        // cannot starve an interrupting transition.
        if self.queued_state_transition_event_exists(project, session)? {
            return Ok(None);
        }

        let mut state_ids: Vec<_> = self
            .state_activity_runtime
            .do_activities
            .keys()
            .copied()
            .collect();
        state_ids.sort_by_key(ToString::to_string);
        for state_id in state_ids {
            if self
                .state_activity_runtime
                .completed_do_activities
                .contains(&state_id)
            {
                continue;
            }
            session.consume_step_budget()?;
            let before = time_event_sequences(session);
            let context_id = self.machine()?.context_id;
            let state_name = self
                .state_name(state_id)
                .unwrap_or_else(|| "unresolved State".into());
            let outcome = {
                let engine = self
                    .state_activity_runtime
                    .do_activities
                    .get_mut(&state_id)
                    .ok_or_else(|| {
                        engine_error("active State doActivity lost its runtime engine")
                    })?;
                step_embedded_activity(
                    engine,
                    project,
                    session,
                    context_id,
                    &format!("State '{state_name}' doActivity"),
                )?
            };
            self.track_new_time_events(state_id, &before, session);
            match outcome {
                EngineStepOutcome::Progressed => {
                    return Ok(Some(EngineStepOutcome::Progressed));
                }
                EngineStepOutcome::Completed => {
                    self.state_activity_runtime
                        .completed_do_activities
                        .insert(state_id);
                    self.cancel_tracked_time_events(session, state_id);
                    session.record_engine_trace(
                        Some(context_id),
                        format!("State '{state_name}' doActivity completed"),
                    );
                    return Ok(Some(EngineStepOutcome::Progressed));
                }
                EngineStepOutcome::Idle => {}
            }
        }
        Ok(None)
    }

    pub(super) fn state_do_activity_is_complete(&self, state_id: VertexId, state: &State) -> bool {
        non_empty(state.do_activity.as_deref()).is_none()
            || self
                .state_activity_runtime
                .completed_do_activities
                .contains(&state_id)
    }

    pub(super) fn state_activity_event_is_relevant(
        &self,
        project: &Project,
        session: &ExecutionSession,
        event: &RuntimeEvent,
    ) -> bool {
        let Ok(machine) = self.machine() else {
            return false;
        };
        if event.kind == RuntimeEventKind::Time
            && self
                .state_activity_runtime
                .time_event_sequences
                .values()
                .any(|sequences| sequences.contains(&event.sequence))
        {
            return true;
        }
        if event.kind != RuntimeEventKind::Signal
            || (event.target_semantic_id.is_some()
                && event.target_semantic_id != Some(machine.context_id))
        {
            return false;
        }
        self.state_activity_runtime
            .do_activities
            .iter()
            .any(|(state_id, engine)| {
                !self
                    .state_activity_runtime
                    .completed_do_activities
                    .contains(state_id)
                    && self.do_activity_accepts_signal(project, session, engine, event)
            })
    }

    pub(super) fn handle_state_activity_event(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        event: &RuntimeEvent,
    ) -> Result<bool, ExecutionError> {
        let mut state_ids: Vec<_> = self
            .state_activity_runtime
            .do_activities
            .keys()
            .copied()
            .collect();
        state_ids.sort_by_key(ToString::to_string);
        for state_id in state_ids {
            if self
                .state_activity_runtime
                .completed_do_activities
                .contains(&state_id)
                || !self.do_activity_accepts_event_for_state(project, session, state_id, event)
            {
                continue;
            }
            let outcome = self
                .state_activity_runtime
                .do_activities
                .get_mut(&state_id)
                .ok_or_else(|| engine_error("active State doActivity lost its runtime engine"))?
                .handle_event(project, session, event)?;
            if let Some(sequences) = self
                .state_activity_runtime
                .time_event_sequences
                .get_mut(&state_id)
            {
                sequences.remove(&event.sequence);
            }
            if outcome != EngineStepOutcome::Idle {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn queued_state_transition_event_exists(
        &mut self,
        project: &Project,
        session: &ExecutionSession,
    ) -> Result<bool, ExecutionError> {
        let context_id = self.machine()?.context_id;
        let events: Vec<_> = session
            .event_queue
            .iter()
            .filter(|scheduled| {
                scheduled.event.target_semantic_id.is_none()
                    || scheduled.event.target_semantic_id == Some(context_id)
            })
            .map(|scheduled| scheduled.event.clone())
            .collect();
        for event in events {
            if !self
                .select_event_transition_set(project, session, &event)?
                .is_empty()
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn build_embedded_activity_engine(
        &self,
        project: &Project,
        session: &ExecutionSession,
        activity_id: ActivityId,
    ) -> Result<ActivityExecutionEngine, ExecutionError> {
        let repository = self
            .state_activity_runtime
            .repository
            .as_ref()
            .ok_or_else(|| {
                engine_error(
                    "State Activity execution requires the shared ActivityRepository runtime source",
                )
            })?;
        let activity = repository.activities.get(&activity_id).ok_or_else(|| {
            engine_error(format!(
                "State Activity reference {activity_id} no longer exists in the Activity repository"
            ))
        })?;
        let mut engine = ActivityExecutionEngine::new(repository.clone(), activity_id);
        for node in &activity.nodes {
            let ActivityNodeKind::ActivityParameter(parameter_node) = &node.kind else {
                continue;
            };
            if let Some(value) = session.value(None, parameter_node.parameter_id).cloned() {
                engine = engine.with_input(parameter_node.parameter_id, vec![value]);
            }
        }

        // PR31's public initializer owns standalone session initialization. Build the
        // Activity call frame on a clone so the actual State Machine session, clock,
        // values, events and trace are never reset. All subsequent Activity steps run
        // directly against the parent ExecutionSession.
        let mut initialization_session = session.clone();
        initialization_session.state = ExecutionState::Created;
        engine.initialize(project, &mut initialization_session)?;
        Ok(engine)
    }

    fn execute_synchronous_state_activity(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        vertex: &Vertex,
        reference: &str,
        role: &str,
    ) -> Result<(), ExecutionError> {
        let activity_id = self.parse_state_activity_id(vertex, role, reference)?;
        let activity_name = self
            .activity_name(activity_id)
            .unwrap_or("unresolved Activity")
            .to_string();
        let context_id = self.machine()?.context_id;
        let mut engine = self.build_embedded_activity_engine(project, session, activity_id)?;
        let mut scheduled_time_events = HashSet::new();
        for _ in 0..super::MAX_RUN_TO_COMPLETION_STEPS {
            session.consume_step_budget()?;
            let before = time_event_sequences(session);
            let outcome = step_embedded_activity(
                &mut engine,
                project,
                session,
                context_id,
                &format!("State '{}' {role} Activity '{activity_name}'", vertex.name),
            )?;
            scheduled_time_events
                .extend(time_event_sequences(session).difference(&before).copied());
            match outcome {
                EngineStepOutcome::Progressed => continue,
                EngineStepOutcome::Completed => {
                    cancel_sequences(session, &scheduled_time_events);
                    session.record_engine_trace(
                        Some(context_id),
                        format!(
                            "State '{}' completed {role} Activity '{}'",
                            vertex.name, activity_name
                        ),
                    );
                    return Ok(());
                }
                EngineStepOutcome::Idle => {
                    cancel_sequences(session, &scheduled_time_events);
                    let message = format!(
                        "State '{}' {role} Activity '{}' cannot complete synchronously because it is waiting for an event or time advance; PR32 executes entry/exit Activities inside the State Machine run-to-completion step and does not invent asynchronous entry/exit semantics",
                        vertex.name, activity_name
                    );
                    session.fail(Some(context_id), message.clone());
                    return Err(engine_error(message));
                }
            }
        }
        cancel_sequences(session, &scheduled_time_events);
        let message = format!(
            "State '{}' {role} Activity '{}' exceeded the bounded State Machine run-to-completion limit",
            vertex.name, activity_name
        );
        session.fail(Some(context_id), message.clone());
        Err(engine_error(message))
    }

    fn parse_state_activity_id(
        &self,
        vertex: &Vertex,
        role: &str,
        reference: &str,
    ) -> Result<ActivityId, ExecutionError> {
        let id = uuid::Uuid::parse_str(reference)
            .map(ActivityId)
            .map_err(|_| {
                engine_error(format!(
                    "State '{}' {role} must reference a modeled Activity by stable ID",
                    vertex.name
                ))
            })?;
        if !self
            .state_activity_runtime
            .repository
            .as_ref()
            .is_some_and(|repository| repository.activities.contains_key(&id))
        {
            return Err(engine_error(format!(
                "State '{}' {role} references missing Activity stable ID {id}",
                vertex.name
            )));
        }
        Ok(id)
    }

    fn activity_name(&self, id: ActivityId) -> Option<&str> {
        self.state_activity_runtime
            .repository
            .as_ref()?
            .activities
            .get(&id)
            .map(|activity| activity.name.as_str())
    }

    fn state_name(&self, id: VertexId) -> Option<String> {
        let machine = self.machine().ok()?;
        super::build_vertex_index(machine)
            .get(&id)
            .map(|location| location.vertex.name.clone())
    }

    fn track_new_time_events(
        &mut self,
        state_id: VertexId,
        before: &HashSet<u64>,
        session: &ExecutionSession,
    ) {
        let after = time_event_sequences(session);
        self.state_activity_runtime
            .time_event_sequences
            .entry(state_id)
            .or_default()
            .extend(after.difference(before).copied());
    }

    fn cancel_tracked_time_events(&mut self, session: &mut ExecutionSession, state_id: VertexId) {
        if let Some(sequences) = self
            .state_activity_runtime
            .time_event_sequences
            .remove(&state_id)
        {
            cancel_sequences(session, &sequences);
        }
    }

    fn cancel_do_activity(&mut self, session: &mut ExecutionSession, state_id: VertexId) {
        self.cancel_tracked_time_events(session, state_id);
        self.state_activity_runtime.do_activities.remove(&state_id);
        self.state_activity_runtime
            .completed_do_activities
            .remove(&state_id);
    }

    fn do_activity_accepts_event_for_state(
        &self,
        project: &Project,
        session: &ExecutionSession,
        state_id: VertexId,
        event: &RuntimeEvent,
    ) -> bool {
        if event.kind == RuntimeEventKind::Time
            && self
                .state_activity_runtime
                .time_event_sequences
                .get(&state_id)
                .is_some_and(|sequences| sequences.contains(&event.sequence))
        {
            return true;
        }
        let Some(engine) = self.state_activity_runtime.do_activities.get(&state_id) else {
            return false;
        };
        self.do_activity_accepts_signal(project, session, engine, event)
    }

    fn do_activity_accepts_signal(
        &self,
        project: &Project,
        session: &ExecutionSession,
        engine: &ActivityExecutionEngine,
        event: &RuntimeEvent,
    ) -> bool {
        if event.kind != RuntimeEventKind::Signal {
            return false;
        }
        let Some(repository) = self.state_activity_runtime.repository.as_ref() else {
            return false;
        };
        engine.snapshot(session).nodes.into_iter().any(|snapshot| {
            if snapshot.state != ActivityNodeExecutionState::Waiting {
                return false;
            }
            let Some(activity) = repository.activities.get(&snapshot.activity_id) else {
                return false;
            };
            let Some(node) = activity
                .nodes
                .iter()
                .find(|node| node.id == snapshot.node_id)
            else {
                return false;
            };
            let ActivityNodeKind::Action(action) = &node.kind else {
                return false;
            };
            let ActionKind::AcceptEvent { signal_id } = &action.kind else {
                return false;
            };
            match signal_id {
                None => true,
                Some(id) => {
                    event.semantic_event_id == Some(*id)
                        || (event.semantic_event_id.is_none()
                            && project
                                .element(*id)
                                .is_ok_and(|signal| signal.name == event.name))
                }
            }
        })
    }
}

fn step_embedded_activity(
    engine: &mut ActivityExecutionEngine,
    project: &Project,
    session: &mut ExecutionSession,
    context_id: crate::ElementId,
    label: &str,
) -> Result<EngineStepOutcome, ExecutionError> {
    let parent_state = session.state;
    let trace_start = session.trace.len();
    let outcome = engine.step(project, session)?;
    if outcome == EngineStepOutcome::Completed && session.state == ExecutionState::Completed {
        session.state = parent_state;
        if let Some(entry) = session.trace[trace_start..]
            .iter_mut()
            .rev()
            .find(|entry| entry.message == "Execution completed")
        {
            entry.semantic_element_id = Some(context_id);
            entry.message = format!("{label} completed");
        }
    }
    Ok(outcome)
}

fn time_event_sequences(session: &ExecutionSession) -> HashSet<u64> {
    session
        .event_queue
        .iter()
        .filter(|scheduled| scheduled.event.kind == RuntimeEventKind::Time)
        .map(|scheduled| scheduled.event.sequence)
        .collect()
}

fn cancel_sequences(session: &mut ExecutionSession, sequences: &HashSet<u64>) {
    if sequences.is_empty() {
        return;
    }
    session
        .event_queue
        .retain(|scheduled| !sequences.contains(&scheduled.event.sequence));
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn engine_error(message: impl Into<String>) -> ExecutionError {
    ExecutionError::Engine {
        message: message.into(),
    }
}
