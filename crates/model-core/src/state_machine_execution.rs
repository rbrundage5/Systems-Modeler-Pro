use crate::behavior::{
    BehaviorRepository, Event, PseudostateKind, Region, RegionId, State, StateMachine,
    StateMachineId, Transition, TransitionId, TransitionKind, Trigger, Vertex, VertexId,
    VertexKind,
};
use crate::{
    DiagnosticSeverity, EngineStepOutcome, ExecutionEngine, ExecutionError, ExecutionSession,
    ExecutionSnapshot, Project, RuntimeEvent, RuntimeEventAddress, RuntimeEventKind,
    RuntimeEventRequest, RuntimeValue, SimulationTime, evaluate_execution_expression,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const MAX_RUN_TO_COMPLETION_STEPS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveStateSnapshot {
    pub state_id: VertexId,
    pub state_name: String,
    pub region_id: RegionId,
    pub ancestry: Vec<VertexId>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateMachineExecutionSnapshot {
    pub execution: ExecutionSnapshot,
    pub state_machine_id: StateMachineId,
    pub active_states: Vec<ActiveStateSnapshot>,
    pub active_region_ids: Vec<RegionId>,
    pub final_region_ids: Vec<RegionId>,
    pub enabled_transition_ids: Vec<TransitionId>,
    pub completed_transition_ids: Vec<TransitionId>,
    pub last_transition_id: Option<TransitionId>,
    pub current_event: Option<RuntimeEvent>,
    pub pending_event_count: usize,
    pub waiting_state_ids: Vec<VertexId>,
}

#[derive(Debug, Clone)]
struct VertexLocation {
    vertex: Vertex,
    region_id: RegionId,
    ancestry: Vec<VertexId>,
}

#[derive(Debug, Clone)]
struct TransitionLocation {
    transition: Transition,
}

pub struct StateMachineExecutionEngine {
    repository: BehaviorRepository,
    state_machine_id: StateMachineId,
    active_states: HashSet<VertexId>,
    active_regions: HashSet<RegionId>,
    final_regions: HashSet<RegionId>,
    completed_transitions: HashSet<TransitionId>,
    join_arrivals: HashMap<VertexId, HashSet<TransitionId>>,
    enabled_transition_ids: Vec<TransitionId>,
    last_transition_id: Option<TransitionId>,
    current_event: Option<RuntimeEvent>,
    state_activation_generations: HashMap<VertexId, u64>,
    next_state_activation_generation: u64,
    change_event_values: HashMap<TransitionId, bool>,
}

impl StateMachineExecutionEngine {
    pub fn new(repository: BehaviorRepository, state_machine_id: StateMachineId) -> Self {
        Self {
            repository,
            state_machine_id,
            active_states: HashSet::new(),
            active_regions: HashSet::new(),
            final_regions: HashSet::new(),
            completed_transitions: HashSet::new(),
            join_arrivals: HashMap::new(),
            enabled_transition_ids: Vec::new(),
            last_transition_id: None,
            current_event: None,
            state_activation_generations: HashMap::new(),
            next_state_activation_generation: 0,
            change_event_values: HashMap::new(),
        }
    }

    pub fn state_machine_id(&self) -> StateMachineId {
        self.state_machine_id
    }

    pub fn authored_repository(&self) -> &BehaviorRepository {
        &self.repository
    }

    pub fn snapshot(&self, session: &ExecutionSession) -> StateMachineExecutionSnapshot {
        let machine = self.machine().ok();
        let vertex_index = machine.map(build_vertex_index).unwrap_or_default();
        let mut active_states: Vec<_> = self
            .active_states
            .iter()
            .filter_map(|id| vertex_index.get(id))
            .map(|location| ActiveStateSnapshot {
                state_id: location.vertex.id,
                state_name: location.vertex.name.clone(),
                region_id: location.region_id,
                ancestry: location.ancestry.clone(),
            })
            .collect();
        active_states.sort_by_key(|state| (state.ancestry.len(), state.state_id.to_string()));
        let mut active_region_ids: Vec<_> = self.active_regions.iter().copied().collect();
        active_region_ids.sort_by_key(ToString::to_string);
        let mut final_region_ids: Vec<_> = self.final_regions.iter().copied().collect();
        final_region_ids.sort_by_key(ToString::to_string);
        let mut completed_transition_ids: Vec<_> =
            self.completed_transitions.iter().copied().collect();
        completed_transition_ids.sort_by_key(ToString::to_string);
        let mut waiting_state_ids: Vec<_> = self
            .active_states
            .iter()
            .filter(|id| {
                outgoing_transitions(machine, **id)
                    .iter()
                    .any(|transition| transition.trigger.is_some())
            })
            .copied()
            .collect();
        waiting_state_ids.sort_by_key(ToString::to_string);
        StateMachineExecutionSnapshot {
            execution: session.snapshot(),
            state_machine_id: self.state_machine_id,
            active_states,
            active_region_ids,
            final_region_ids,
            enabled_transition_ids: self.enabled_transition_ids.clone(),
            completed_transition_ids,
            last_transition_id: self.last_transition_id,
            current_event: self.current_event.clone(),
            pending_event_count: session.event_queue.len(),
            waiting_state_ids,
        }
    }

    pub fn reset(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<(), ExecutionError> {
        session.reset(project)?;
        initialize_authored_defaults(project, session)?;
        self.clear_runtime();
        self.establish_initial_configuration(project, session)
    }

    pub fn advance(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<EngineStepOutcome, ExecutionError> {
        if matches!(session.state, crate::ExecutionState::Completed) {
            return Ok(EngineStepOutcome::Completed);
        }
        self.current_event = None;
        if let Some(candidate) = self.select_automatic_transition(project, session)? {
            self.fire_transition(project, session, &candidate, None)?;
            return self.finish_step(session);
        }
        self.refresh_enabled_transitions(project, session, None)?;
        let Some(position) = self.next_relevant_event_position(session) else {
            return Ok(EngineStepOutcome::Idle);
        };
        if position > 0 {
            let selected = session
                .event_queue
                .remove(position)
                .expect("an inspected State Machine event must remain queued");
            session.event_queue.push_front(selected);
        }
        let event = session
            .step()?
            .expect("an inspected State Machine event must remain queued");
        self.handle_event(project, session, &event)
    }

    pub fn queue_signal(
        &self,
        project: &Project,
        session: &mut ExecutionSession,
        signal_id: crate::ElementId,
        name: impl Into<String>,
        payload: Vec<(String, RuntimeValue)>,
    ) -> Result<u64, ExecutionError> {
        let due_time = session.simulation_time;
        session.queue_typed_event_at(
            project,
            RuntimeEventRequest {
                due_time,
                kind: RuntimeEventKind::Signal,
                name: name.into(),
                semantic_event_id: Some(signal_id),
                address: RuntimeEventAddress {
                    target_semantic_id: Some(self.machine()?.context_id),
                    ..RuntimeEventAddress::default()
                },
                payload,
            },
        )
    }

    fn machine(&self) -> Result<&StateMachine, ExecutionError> {
        self.repository
            .state_machines
            .get(&self.state_machine_id)
            .ok_or_else(|| engine_error("State Machine does not exist in the behavior repository"))
    }

    fn clear_runtime(&mut self) {
        self.active_states.clear();
        self.active_regions.clear();
        self.final_regions.clear();
        self.completed_transitions.clear();
        self.join_arrivals.clear();
        self.enabled_transition_ids.clear();
        self.last_transition_id = None;
        self.current_event = None;
        self.state_activation_generations.clear();
        self.next_state_activation_generation = 0;
        self.change_event_values.clear();
    }

    fn establish_initial_configuration(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<(), ExecutionError> {
        let machine = self.machine()?.clone();
        if machine.regions.is_empty() {
            return Err(engine_error(format!(
                "Cannot initialize State Machine '{}': it has no Region",
                machine.name
            )));
        }
        let mut rtc_steps = 0;
        for region in &machine.regions {
            self.enter_region_initial(project, session, region.id, &mut rtc_steps)?;
        }
        self.refresh_enabled_transitions(project, session, None)?;
        self.complete_if_root_final(session)?;
        Ok(())
    }

    fn enter_region_initial(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        region_id: RegionId,
        rtc_steps: &mut usize,
    ) -> Result<(), ExecutionError> {
        let region = find_region(&self.machine()?.regions, region_id)
            .cloned()
            .ok_or_else(|| engine_error("State Machine Region cannot be resolved"))?;
        self.active_regions.insert(region_id);
        self.final_regions.remove(&region_id);
        let mut initials: Vec<_> = region
            .vertices
            .iter()
            .filter(|vertex| {
                matches!(
                    vertex.kind,
                    VertexKind::Pseudostate(PseudostateKind::Initial)
                )
            })
            .cloned()
            .collect();
        initials.sort_by_key(|vertex| vertex.id.to_string());
        let initial = initials.first().ok_or_else(|| {
            engine_error(format!(
                "Region '{}' has no Initial pseudostate",
                region.name
            ))
        })?;
        self.traverse_pseudostate(project, session, initial, None, None, rtc_steps)
    }

    fn traverse_pseudostate(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        vertex: &Vertex,
        arrived_via: Option<TransitionId>,
        event: Option<&RuntimeEvent>,
        rtc_steps: &mut usize,
    ) -> Result<(), ExecutionError> {
        self.consume_rtc_budget(session, rtc_steps)?;
        let VertexKind::Pseudostate(kind) = vertex.kind else {
            return self.enter_vertex(project, session, vertex.id, arrived_via, event, rtc_steps);
        };
        match kind {
            PseudostateKind::Initial | PseudostateKind::Choice | PseudostateKind::Junction => {
                let transition = self
                    .select_pseudostate_transition(project, session, vertex.id, event)?
                    .ok_or_else(|| {
                        engine_error(format!(
                            "Pseudostate '{}' has no enabled outgoing transition",
                            vertex.name
                        ))
                    })?;
                self.fire_transition_inner(project, session, &transition, event, rtc_steps)
            }
            PseudostateKind::EntryPoint | PseudostateKind::ExitPoint => {
                let message = format!(
                    "State Machine execution reached {:?} '{}', but the authored metamodel does not identify a qualified connection-point owner/entry-exit mapping",
                    kind, vertex.name
                );
                session.add_diagnostic(
                    DiagnosticSeverity::Error,
                    Some(self.machine()?.context_id),
                    message.clone(),
                );
                Err(engine_error(message))
            }
            PseudostateKind::Fork => {
                let mut outgoing = self.transitions_from(vertex.id);
                outgoing.sort_by_key(|transition| transition.transition.id.to_string());
                if outgoing.len() < 2 {
                    return Err(engine_error(format!(
                        "Fork '{}' requires at least two outgoing transitions",
                        vertex.name
                    )));
                }
                for transition in outgoing {
                    if self.guard_allows(project, session, &transition.transition.guard, event)? {
                        self.fire_transition_inner(
                            project,
                            session,
                            &transition,
                            event,
                            rtc_steps,
                        )?;
                    }
                }
                Ok(())
            }
            PseudostateKind::Join => {
                let arrived_via = arrived_via.ok_or_else(|| {
                    engine_error(format!(
                        "Join '{}' was reached without an incoming transition",
                        vertex.name
                    ))
                })?;
                self.join_arrivals
                    .entry(vertex.id)
                    .or_default()
                    .insert(arrived_via);
                let incoming_count = self.transitions_to(vertex.id).len();
                if self.join_arrivals[&vertex.id].len() < incoming_count {
                    return Ok(());
                }
                self.join_arrivals.remove(&vertex.id);
                let outgoing = self.transitions_from(vertex.id);
                if outgoing.len() != 1 {
                    return Err(engine_error(format!(
                        "Join '{}' requires exactly one outgoing transition",
                        vertex.name
                    )));
                }
                self.fire_transition_inner(project, session, &outgoing[0], event, rtc_steps)
            }
            PseudostateKind::Terminate => {
                session.complete()?;
                Ok(())
            }
            PseudostateKind::ShallowHistory | PseudostateKind::DeepHistory => {
                let message = format!(
                    "State Machine execution reached {} '{}', but the authored model does not store a qualified history default/restoration policy",
                    if kind == PseudostateKind::DeepHistory {
                        "DeepHistory"
                    } else {
                        "ShallowHistory"
                    },
                    vertex.name
                );
                session.add_diagnostic(
                    DiagnosticSeverity::Error,
                    Some(self.machine()?.context_id),
                    message.clone(),
                );
                Err(engine_error(message))
            }
        }
    }

    fn enter_vertex(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        vertex_id: VertexId,
        arrived_via: Option<TransitionId>,
        event: Option<&RuntimeEvent>,
        rtc_steps: &mut usize,
    ) -> Result<(), ExecutionError> {
        let index = build_vertex_index(self.machine()?);
        let location = index
            .get(&vertex_id)
            .cloned()
            .ok_or_else(|| engine_error("Transition target cannot be resolved"))?;

        for (position, ancestor_id) in location.ancestry.iter().enumerate() {
            if self.active_states.contains(ancestor_id) {
                continue;
            }
            let ancestor = index
                .get(ancestor_id)
                .cloned()
                .ok_or_else(|| engine_error("Composite-state ancestry cannot be resolved"))?;
            self.active_states.insert(*ancestor_id);
            self.active_regions.insert(ancestor.region_id);
            session.record_engine_trace(
                Some(self.machine()?.context_id),
                format!("Entered composite State '{}'", ancestor.vertex.name),
            );
            if let VertexKind::State(state) = &ancestor.vertex.kind {
                self.report_untyped_state_behaviors(session, &ancestor.vertex, state);
                self.activate_state_runtime(project, session, &ancestor.vertex)?;
                let next_vertex_id = location
                    .ancestry
                    .get(position + 1)
                    .copied()
                    .unwrap_or(vertex_id);
                for region in sorted_regions(&state.regions) {
                    if region_contains_vertex(region, next_vertex_id) {
                        self.active_regions.insert(region.id);
                        self.final_regions.remove(&region.id);
                    } else {
                        self.enter_region_initial(project, session, region.id, rtc_steps)?;
                    }
                }
            }
        }

        self.active_regions.insert(location.region_id);
        match &location.vertex.kind {
            VertexKind::State(state) => {
                let newly_active = self.active_states.insert(vertex_id);
                self.final_regions.remove(&location.region_id);
                if newly_active {
                    session.record_engine_trace(
                        Some(self.machine()?.context_id),
                        format!("Entered State '{}'", location.vertex.name),
                    );
                    self.report_untyped_state_behaviors(session, &location.vertex, state);
                    self.activate_state_runtime(project, session, &location.vertex)?;
                    for region in sorted_regions(&state.regions) {
                        self.enter_region_initial(project, session, region.id, rtc_steps)?;
                    }
                }
                Ok(())
            }
            VertexKind::FinalState => {
                self.final_regions.insert(location.region_id);
                session.record_engine_trace(
                    Some(self.machine()?.context_id),
                    format!("Region reached FinalState '{}'", location.vertex.name),
                );
                Ok(())
            }
            VertexKind::Pseudostate(_) => self.traverse_pseudostate(
                project,
                session,
                &location.vertex,
                arrived_via,
                event,
                rtc_steps,
            ),
        }
    }

    fn activate_state_runtime(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        vertex: &Vertex,
    ) -> Result<(), ExecutionError> {
        let generation = self.next_state_activation_generation;
        self.next_state_activation_generation =
            self.next_state_activation_generation.saturating_add(1);
        self.state_activation_generations
            .insert(vertex.id, generation);
        self.schedule_time_events(project, session, vertex, generation)?;
        self.initialize_change_event_values(project, session, vertex)
    }

    fn report_untyped_state_behaviors(
        &self,
        session: &mut ExecutionSession,
        vertex: &Vertex,
        state: &State,
    ) {
        for (label, value) in [
            ("entry", state.entry.as_deref()),
            ("doActivity", state.do_activity.as_deref()),
            ("exit", state.exit.as_deref()),
        ] {
            if value.is_some_and(|value| !value.trim().is_empty()) {
                session.add_diagnostic(
                    DiagnosticSeverity::Warning,
                    self.machine().ok().map(|machine| machine.context_id),
                    format!(
                        "State '{}' has {} text, but the current metamodel does not provide a stable Behavior/Activity reference; execution did not interpret arbitrary model text",
                        vertex.name, label
                    ),
                );
            }
        }
    }

    fn schedule_time_events(
        &self,
        project: &Project,
        session: &mut ExecutionSession,
        vertex: &Vertex,
        generation: u64,
    ) -> Result<(), ExecutionError> {
        for location in self.transitions_from(vertex.id) {
            let Some(Trigger {
                event:
                    Event::Time {
                        expression,
                        is_relative,
                    },
            }) = &location.transition.trigger
            else {
                continue;
            };
            let nanos = parse_duration(expression)?;
            let due_time = if *is_relative {
                session
                    .simulation_time
                    .checked_add(nanos)
                    .ok_or(ExecutionError::SimulationTimeOverflow)?
            } else {
                SimulationTime::from_nanos(nanos)
            };
            session.queue_typed_event_at(
                project,
                RuntimeEventRequest {
                    due_time,
                    kind: RuntimeEventKind::Time,
                    name: time_event_name(location.transition.id, generation),
                    semantic_event_id: None,
                    address: RuntimeEventAddress {
                        target_semantic_id: Some(self.machine()?.context_id),
                        ..RuntimeEventAddress::default()
                    },
                    payload: Vec::new(),
                },
            )?;
        }
        Ok(())
    }

    fn initialize_change_event_values(
        &mut self,
        project: &Project,
        session: &ExecutionSession,
        vertex: &Vertex,
    ) -> Result<(), ExecutionError> {
        for location in self.transitions_from(vertex.id) {
            let Some(Trigger {
                event: Event::Change { expression },
            }) = &location.transition.trigger
            else {
                continue;
            };
            let current = self.expression_is_true(project, session, expression, None)?;
            self.change_event_values
                .insert(location.transition.id, current);
        }
        Ok(())
    }

    fn select_automatic_transition(
        &mut self,
        project: &Project,
        session: &ExecutionSession,
    ) -> Result<Option<TransitionLocation>, ExecutionError> {
        let mut candidates = Vec::new();
        let active_state_ids: Vec<_> = self.active_states.iter().copied().collect();
        for state_id in active_state_ids {
            for transition in self.transitions_from(state_id) {
                let eligible = match transition
                    .transition
                    .trigger
                    .as_ref()
                    .map(|trigger| &trigger.event)
                {
                    None => self.source_is_complete(state_id),
                    Some(Event::Change { expression }) => {
                        let current =
                            self.expression_is_true(project, session, expression, None)?;
                        let previous = self
                            .change_event_values
                            .insert(transition.transition.id, current)
                            .unwrap_or(current);
                        !previous && current
                    }
                    _ => false,
                };
                if eligible
                    && self.guard_allows(project, session, &transition.transition.guard, None)?
                {
                    candidates.push(transition);
                }
            }
        }
        self.sort_candidates(&mut candidates);
        self.enabled_transition_ids = candidates
            .iter()
            .map(|candidate| candidate.transition.id)
            .collect();
        Ok(candidates.into_iter().next())
    }

    fn select_pseudostate_transition(
        &self,
        project: &Project,
        session: &ExecutionSession,
        source_id: VertexId,
        event: Option<&RuntimeEvent>,
    ) -> Result<Option<TransitionLocation>, ExecutionError> {
        let mut outgoing = self.transitions_from(source_id);
        outgoing.sort_by_key(|location| location.transition.id.to_string());
        let mut enabled = Vec::new();
        let mut fallback = Vec::new();
        for transition in outgoing {
            if transition.transition.guard.as_deref() == Some("else") {
                fallback.push(transition);
            } else if self.guard_allows(project, session, &transition.transition.guard, event)? {
                enabled.push(transition);
            }
        }
        if enabled.len() > 1 {
            let ids = enabled
                .iter()
                .map(|transition| transition.transition.id.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(engine_error(format!(
                "Pseudostate has multiple enabled outgoing transitions with no semantic priority: {ids}"
            )));
        }
        if let Some(selected) = enabled.into_iter().next() {
            return Ok(Some(selected));
        }
        if fallback.len() > 1 {
            return Err(engine_error(
                "Pseudostate has more than one 'else' transition",
            ));
        }
        Ok(fallback.into_iter().next())
    }

    fn refresh_enabled_transitions(
        &mut self,
        project: &Project,
        session: &ExecutionSession,
        event: Option<&RuntimeEvent>,
    ) -> Result<(), ExecutionError> {
        if let Some(event) = event {
            let selected = self.select_event_transition_set(project, session, event)?;
            self.enabled_transition_ids = selected
                .iter()
                .map(|candidate| candidate.transition.id)
                .collect();
        } else {
            self.enabled_transition_ids.clear();
        }
        Ok(())
    }

    fn next_relevant_event_position(&self, session: &ExecutionSession) -> Option<usize> {
        let context_id = self.machine().ok()?.context_id;
        session.event_queue.iter().position(|scheduled| {
            scheduled.event.target_semantic_id == Some(context_id)
                || (scheduled.event.target_semantic_id.is_none()
                    && self.trigger_candidates(&scheduled.event).next().is_some())
        })
    }

    fn trigger_candidates<'a>(
        &'a self,
        event: &'a RuntimeEvent,
    ) -> impl Iterator<Item = TransitionLocation> + 'a {
        let mut candidates = Vec::new();
        for state_id in &self.active_states {
            candidates.extend(
                self.transitions_from(*state_id)
                    .into_iter()
                    .filter(|location| self.trigger_matches(&location.transition, event)),
            );
        }
        candidates.into_iter()
    }

    fn trigger_matches(&self, transition: &Transition, event: &RuntimeEvent) -> bool {
        let Some(trigger) = &transition.trigger else {
            return event.kind == RuntimeEventKind::Completion;
        };
        match &trigger.event {
            Event::Signal { signal_id } => {
                event.kind == RuntimeEventKind::Signal
                    && event.semantic_event_id == Some(*signal_id)
            }
            Event::Call { operation_id } => {
                event.kind == RuntimeEventKind::Call
                    && event.semantic_event_id == Some(*operation_id)
            }
            Event::Time { .. } => {
                let Some(generation) = self
                    .state_activation_generations
                    .get(&transition.source_id)
                    .copied()
                else {
                    return false;
                };
                event.kind == RuntimeEventKind::Time
                    && event.name == time_event_name(transition.id, generation)
            }
            Event::AnyReceive => !matches!(
                event.kind,
                RuntimeEventKind::Completion | RuntimeEventKind::Internal
            ),
            Event::Change { .. } => false,
        }
    }

    fn sort_candidates(&self, candidates: &mut [TransitionLocation]) {
        let index = self.machine().map(build_vertex_index).unwrap_or_default();
        candidates.sort_by_key(|candidate| {
            let depth = index
                .get(&candidate.transition.source_id)
                .map(|location| location.ancestry.len())
                .unwrap_or_default();
            (usize::MAX - depth, candidate.transition.id.to_string())
        });
    }

    fn candidate_depth(&self, candidate: &TransitionLocation) -> usize {
        self.machine()
            .map(build_vertex_index)
            .ok()
            .and_then(|index| {
                index
                    .get(&candidate.transition.source_id)
                    .map(|location| location.ancestry.len())
            })
            .unwrap_or_default()
    }

    fn select_event_transition_set(
        &mut self,
        project: &Project,
        session: &ExecutionSession,
        event: &RuntimeEvent,
    ) -> Result<Vec<TransitionLocation>, ExecutionError> {
        let mut qualified = Vec::new();
        for candidate in self.trigger_candidates(event) {
            if self.guard_allows(project, session, &candidate.transition.guard, Some(event))? {
                qualified.push(candidate);
            }
        }
        self.sort_candidates(&mut qualified);

        let mut selected: Vec<TransitionLocation> = Vec::new();
        for candidate in qualified {
            let candidate_depth = self.candidate_depth(&candidate);
            let mut conflicting_depths = Vec::new();
            for chosen in &selected {
                if self.transitions_conflict(&candidate.transition, &chosen.transition)? {
                    conflicting_depths.push(self.candidate_depth(chosen));
                }
            }
            if conflicting_depths.is_empty() {
                selected.push(candidate);
                continue;
            }
            let highest_selected_depth = conflicting_depths.into_iter().max().unwrap_or_default();
            if highest_selected_depth > candidate_depth {
                continue;
            }
            let event_name = if event.name.is_empty() {
                format!("{:?}", event.kind)
            } else {
                event.name.clone()
            };
            return Err(engine_error(format!(
                "Ambiguous State Machine transition selection for event '{event_name}': multiple conflicting transitions are enabled at the same hierarchy priority"
            )));
        }

        self.enabled_transition_ids = selected
            .iter()
            .map(|candidate| candidate.transition.id)
            .collect();
        Ok(selected)
    }

    fn transitions_conflict(
        &self,
        left: &Transition,
        right: &Transition,
    ) -> Result<bool, ExecutionError> {
        if left.source_id == right.source_id {
            return Ok(true);
        }
        let left_exit = self.exit_state_ids_for_transition(left)?;
        let right_exit = self.exit_state_ids_for_transition(right)?;
        Ok(!left_exit.is_disjoint(&right_exit))
    }

    fn fire_transition_set(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        transitions: &[TransitionLocation],
        event: Option<&RuntimeEvent>,
        rtc_steps: &mut usize,
    ) -> Result<(), ExecutionError> {
        for transition in transitions {
            self.fire_transition_inner(project, session, transition, event, rtc_steps)?;
        }
        Ok(())
    }

    fn fire_transition(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        transition: &TransitionLocation,
        event: Option<&RuntimeEvent>,
    ) -> Result<(), ExecutionError> {
        let mut rtc_steps = 0;
        self.fire_transition_inner(project, session, transition, event, &mut rtc_steps)
    }

    fn fire_transition_inner(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        location: &TransitionLocation,
        event: Option<&RuntimeEvent>,
        rtc_steps: &mut usize,
    ) -> Result<(), ExecutionError> {
        self.consume_rtc_budget(session, rtc_steps)?;
        let transition = &location.transition;
        if !self.guard_allows(project, session, &transition.guard, event)? {
            return Err(engine_error(
                "Selected State Machine transition guard is not true",
            ));
        }
        match transition.kind {
            TransitionKind::External | TransitionKind::Local => {
                self.exit_for_transition(session, transition)?;
            }
            TransitionKind::Internal => {}
        }
        self.execute_effect(project, session, transition, event)?;
        self.completed_transitions.insert(transition.id);
        self.last_transition_id = Some(transition.id);
        let vertex_index = build_vertex_index(self.machine()?);
        let source_name = vertex_index
            .get(&transition.source_id)
            .map(|location| location.vertex.name.as_str())
            .unwrap_or("unresolved source");
        let target_name = vertex_index
            .get(&transition.target_id)
            .map(|location| location.vertex.name.as_str())
            .unwrap_or("unresolved target");
        session.record_engine_trace(
            Some(self.machine()?.context_id),
            format!("Fired transition '{source_name}' -> '{target_name}'"),
        );
        match transition.kind {
            TransitionKind::External | TransitionKind::Local => {
                self.enter_vertex(
                    project,
                    session,
                    transition.target_id,
                    Some(transition.id),
                    event,
                    rtc_steps,
                )?;
            }
            TransitionKind::Internal => {}
        }
        Ok(())
    }

    fn exit_state_ids_for_transition(
        &self,
        transition: &Transition,
    ) -> Result<HashSet<VertexId>, ExecutionError> {
        if transition.kind == TransitionKind::Internal {
            return Ok(HashSet::new());
        }
        let index = build_vertex_index(self.machine()?);
        let source = index
            .get(&transition.source_id)
            .ok_or_else(|| engine_error("Transition source cannot be resolved"))?;
        let target = index
            .get(&transition.target_id)
            .ok_or_else(|| engine_error("Transition target cannot be resolved"))?;
        let source_is_state = matches!(source.vertex.kind, VertexKind::State(_));
        let target_inside_source = source_is_state
            && (target.vertex.id == source.vertex.id
                || target.ancestry.contains(&source.vertex.id));

        if transition.kind == TransitionKind::Local && target_inside_source {
            return Ok(self
                .active_states
                .iter()
                .filter_map(|id| index.get(id))
                .filter(|location| location.ancestry.contains(&source.vertex.id))
                .map(|location| location.vertex.id)
                .collect());
        }

        let source_path = state_path(source);
        let target_path = state_path(target);
        let exit_roots = if target_inside_source {
            vec![source.vertex.id]
        } else {
            let common = common_prefix_len(&source_path, &target_path);
            source_path[common..].to_vec()
        };

        Ok(self
            .active_states
            .iter()
            .filter_map(|id| index.get(id))
            .filter(|location| {
                exit_roots
                    .iter()
                    .any(|root| location.vertex.id == *root || location.ancestry.contains(root))
            })
            .map(|location| location.vertex.id)
            .collect())
    }

    fn exit_for_transition(
        &mut self,
        session: &mut ExecutionSession,
        transition: &Transition,
    ) -> Result<(), ExecutionError> {
        let index = build_vertex_index(self.machine()?);
        let exiting_ids = self.exit_state_ids_for_transition(transition)?;
        let mut exiting: Vec<_> = exiting_ids
            .iter()
            .filter_map(|id| index.get(id))
            .cloned()
            .collect();
        exiting.sort_by_key(|location| {
            (
                usize::MAX - location.ancestry.len(),
                location.vertex.id.to_string(),
            )
        });
        for location in exiting {
            if let VertexKind::State(state) = &location.vertex.kind
                && state
                    .exit
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
            {
                session.add_diagnostic(
                    DiagnosticSeverity::Warning,
                    Some(self.machine()?.context_id),
                    format!(
                        "State '{}' exit text was not executed because it is not a stable Behavior reference",
                        location.vertex.name
                    ),
                );
            }
            for outgoing in self.transitions_from(location.vertex.id) {
                if matches!(
                    outgoing
                        .transition
                        .trigger
                        .as_ref()
                        .map(|trigger| &trigger.event),
                    Some(Event::Change { .. })
                ) {
                    self.change_event_values.remove(&outgoing.transition.id);
                }
            }
            self.state_activation_generations
                .remove(&location.vertex.id);
            self.active_states.remove(&location.vertex.id);
            for region in state_regions(&location.vertex) {
                self.active_regions.remove(&region.id);
                self.final_regions.remove(&region.id);
            }
            session.record_engine_trace(
                Some(self.machine()?.context_id),
                format!("Exited State '{}'", location.vertex.name),
            );
        }
        Ok(())
    }

    fn execute_effect(
        &self,
        project: &Project,
        session: &mut ExecutionSession,
        transition: &Transition,
        event: Option<&RuntimeEvent>,
    ) -> Result<(), ExecutionError> {
        let Some(effect) = transition
            .effect
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        else {
            return Ok(());
        };
        let value = self.evaluate(project, session, effect, event)?;
        session.record_engine_trace(
            Some(self.machine()?.context_id),
            format!(
                "Transition effect evaluated to {}",
                runtime_value_label(&value)
            ),
        );
        Ok(())
    }

    fn guard_allows(
        &self,
        project: &Project,
        session: &ExecutionSession,
        guard: &Option<String>,
        event: Option<&RuntimeEvent>,
    ) -> Result<bool, ExecutionError> {
        let Some(guard) = guard.as_deref().filter(|value| !value.trim().is_empty()) else {
            return Ok(true);
        };
        if guard == "else" {
            return Ok(true);
        }
        self.expression_is_true(project, session, guard, event)
    }

    fn expression_is_true(
        &self,
        project: &Project,
        session: &ExecutionSession,
        expression: &str,
        event: Option<&RuntimeEvent>,
    ) -> Result<bool, ExecutionError> {
        match self.evaluate(project, session, expression, event)? {
            RuntimeValue::Boolean(value) => Ok(value),
            other => Err(engine_error(format!(
                "State Machine guard must evaluate to boolean, not {}",
                other.kind_name()
            ))),
        }
    }

    fn evaluate(
        &self,
        project: &Project,
        session: &ExecutionSession,
        expression: &str,
        event: Option<&RuntimeEvent>,
    ) -> Result<RuntimeValue, ExecutionError> {
        evaluate_execution_expression(expression, |name| {
            event
                .and_then(|event| {
                    event
                        .payload
                        .iter()
                        .find(|(key, _)| key == name)
                        .map(|(_, value)| value.clone())
                })
                .or_else(|| {
                    let mut matching: Vec<_> = project
                        .elements
                        .values()
                        .filter(|element| element.name == name)
                        .collect();
                    matching.sort_by_key(|element| element.id.to_string());
                    matching
                        .into_iter()
                        .find_map(|element| session.value(None, element.id).cloned())
                })
        })
        .map_err(|error| {
            engine_error(format!(
                "State Machine expression '{expression}' is invalid: {error}"
            ))
        })
    }

    fn source_is_complete(&self, state_id: VertexId) -> bool {
        let Ok(machine) = self.machine() else {
            return false;
        };
        let index = build_vertex_index(machine);
        let Some(location) = index.get(&state_id) else {
            return false;
        };
        let VertexKind::State(state) = &location.vertex.kind else {
            return false;
        };
        state.regions.is_empty()
            || state
                .regions
                .iter()
                .all(|region| self.final_regions.contains(&region.id))
    }

    fn transitions_from(&self, source_id: VertexId) -> Vec<TransitionLocation> {
        all_transitions(self.machine().ok())
            .into_iter()
            .filter(|location| location.transition.source_id == source_id)
            .collect()
    }

    fn transitions_to(&self, target_id: VertexId) -> Vec<TransitionLocation> {
        all_transitions(self.machine().ok())
            .into_iter()
            .filter(|location| location.transition.target_id == target_id)
            .collect()
    }

    fn consume_rtc_budget(
        &self,
        session: &mut ExecutionSession,
        rtc_steps: &mut usize,
    ) -> Result<(), ExecutionError> {
        if *rtc_steps >= MAX_RUN_TO_COMPLETION_STEPS {
            let message = format!(
                "State Machine run-to-completion step limit exceeded: {MAX_RUN_TO_COMPLETION_STEPS}"
            );
            session.fail(
                self.machine().ok().map(|machine| machine.context_id),
                message.clone(),
            );
            return Err(engine_error(message));
        }
        *rtc_steps += 1;
        session.consume_step_budget()?;
        Ok(())
    }

    fn complete_if_root_final(&self, session: &mut ExecutionSession) -> Result<(), ExecutionError> {
        let machine = self.machine()?;
        if machine
            .regions
            .iter()
            .all(|region| self.final_regions.contains(&region.id))
        {
            session.complete()?;
        }
        Ok(())
    }

    fn finish_step(
        &self,
        session: &mut ExecutionSession,
    ) -> Result<EngineStepOutcome, ExecutionError> {
        self.complete_if_root_final(session)?;
        Ok(
            if matches!(session.state, crate::ExecutionState::Completed) {
                EngineStepOutcome::Completed
            } else {
                EngineStepOutcome::Progressed
            },
        )
    }
}

impl ExecutionEngine for StateMachineExecutionEngine {
    fn initialize(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<(), ExecutionError> {
        self.repository.validate(project).map_err(|error| {
            engine_error(format!(
                "Cannot initialize State Machine execution: {error}"
            ))
        })?;
        session.initialize(project)?;
        initialize_authored_defaults(project, session)?;
        self.clear_runtime();
        self.establish_initial_configuration(project, session)
    }

    fn step(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<EngineStepOutcome, ExecutionError> {
        self.advance(project, session)
    }

    fn handle_event(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        event: &RuntimeEvent,
    ) -> Result<EngineStepOutcome, ExecutionError> {
        self.current_event = Some(event.clone());
        let selected = self.select_event_transition_set(project, session, event)?;
        if selected.is_empty() {
            return Ok(EngineStepOutcome::Idle);
        }

        let mut rtc_steps = 0;
        self.fire_transition_set(project, session, &selected, Some(event), &mut rtc_steps)?;

        while !matches!(session.state, crate::ExecutionState::Completed) {
            let Some(automatic) = self.select_automatic_transition(project, session)? else {
                break;
            };
            self.fire_transition_inner(project, session, &automatic, None, &mut rtc_steps)?;
        }
        self.finish_step(session)
    }
}

fn build_vertex_index(machine: &StateMachine) -> HashMap<VertexId, VertexLocation> {
    fn visit(
        regions: &[Region],
        ancestry: &[VertexId],
        output: &mut HashMap<VertexId, VertexLocation>,
    ) {
        for region in regions {
            for vertex in &region.vertices {
                output.insert(
                    vertex.id,
                    VertexLocation {
                        vertex: vertex.clone(),
                        region_id: region.id,
                        ancestry: ancestry.to_vec(),
                    },
                );
                if let VertexKind::State(state) = &vertex.kind {
                    let mut nested = ancestry.to_vec();
                    nested.push(vertex.id);
                    visit(&state.regions, &nested, output);
                }
            }
        }
    }
    let mut output = HashMap::new();
    visit(&machine.regions, &[], &mut output);
    output
}

fn state_path(location: &VertexLocation) -> Vec<VertexId> {
    let mut path = location.ancestry.clone();
    if matches!(location.vertex.kind, VertexKind::State(_)) {
        path.push(location.vertex.id);
    }
    path
}

fn common_prefix_len(left: &[VertexId], right: &[VertexId]) -> usize {
    left.iter()
        .zip(right)
        .take_while(|(left, right)| left == right)
        .count()
}

fn region_contains_vertex(region: &Region, wanted: VertexId) -> bool {
    for vertex in &region.vertices {
        if vertex.id == wanted {
            return true;
        }
        if let VertexKind::State(state) = &vertex.kind
            && state
                .regions
                .iter()
                .any(|nested| region_contains_vertex(nested, wanted))
        {
            return true;
        }
    }
    false
}

fn find_region(regions: &[Region], wanted: RegionId) -> Option<&Region> {
    for region in regions {
        if region.id == wanted {
            return Some(region);
        }
        for vertex in region.vertices.iter() {
            if let VertexKind::State(state) = &vertex.kind
                && let Some(found) = find_region(&state.regions, wanted)
            {
                return Some(found);
            }
        }
    }
    None
}

fn all_transitions(machine: Option<&StateMachine>) -> Vec<TransitionLocation> {
    fn visit(regions: &[Region], output: &mut Vec<TransitionLocation>) {
        for region in regions {
            output.extend(
                region
                    .transitions
                    .iter()
                    .cloned()
                    .map(|transition| TransitionLocation { transition }),
            );
            for vertex in &region.vertices {
                if let VertexKind::State(state) = &vertex.kind {
                    visit(&state.regions, output);
                }
            }
        }
    }
    let mut output = Vec::new();
    if let Some(machine) = machine {
        visit(&machine.regions, &mut output);
    }
    output
}

fn outgoing_transitions(machine: Option<&StateMachine>, source_id: VertexId) -> Vec<Transition> {
    all_transitions(machine)
        .into_iter()
        .filter(|location| location.transition.source_id == source_id)
        .map(|location| location.transition)
        .collect()
}

fn initialize_authored_defaults(
    project: &Project,
    session: &mut ExecutionSession,
) -> Result<(), ExecutionError> {
    let mut defaults: Vec<_> = project
        .elements
        .values()
        .filter_map(|element| {
            element
                .default_value
                .as_deref()
                .map(|value| (element.id, value.to_owned()))
        })
        .collect();
    defaults.sort_by_key(|(id, _)| id.to_string());
    for (element_id, authored) in defaults {
        let value = evaluate_execution_expression(&authored, |_| None)
            .unwrap_or_else(|_| RuntimeValue::Text(authored));
        session.set_value(project, None, element_id, value)?;
    }
    Ok(())
}

fn parse_duration(expression: &str) -> Result<u64, ExecutionError> {
    let trimmed = expression.trim();
    let trimmed = trimmed.strip_prefix("after ").unwrap_or(trimmed).trim();
    let (numeric, multiplier) = if let Some(value) = trimmed.strip_suffix("ns") {
        (value.trim(), 1_f64)
    } else if let Some(value) = trimmed.strip_suffix("us") {
        (value.trim(), 1_000_f64)
    } else if let Some(value) = trimmed.strip_suffix("ms") {
        (value.trim(), 1_000_000_f64)
    } else if let Some(value) = trimmed.strip_suffix('s') {
        (value.trim(), 1_000_000_000_f64)
    } else {
        (trimmed, 1_f64)
    };
    let value: f64 = numeric
        .parse()
        .map_err(|_| engine_error(format!("TimeEvent expression '{expression}' is invalid")))?;
    let nanos = value * multiplier;
    if !nanos.is_finite() || nanos < 0.0 || nanos > u64::MAX as f64 {
        return Err(engine_error(format!(
            "TimeEvent expression '{expression}' is outside deterministic SimulationTime"
        )));
    }
    Ok(nanos.round() as u64)
}

fn time_event_name(transition_id: TransitionId, generation: u64) -> String {
    format!("state-machine-time:{transition_id}:{generation}")
}

fn state_regions(vertex: &Vertex) -> &[Region] {
    match &vertex.kind {
        VertexKind::State(state) => &state.regions,
        _ => &[],
    }
}

fn sorted_regions(regions: &[Region]) -> Vec<&Region> {
    let mut sorted: Vec<_> = regions.iter().collect();
    sorted.sort_by_key(|region| region.id.to_string());
    sorted
}

fn runtime_value_label(value: &RuntimeValue) -> String {
    match value {
        RuntimeValue::Unset => "unset".into(),
        RuntimeValue::Boolean(value) => value.to_string(),
        RuntimeValue::Integer(value) => value.to_string(),
        RuntimeValue::Real(value) => value.to_string(),
        RuntimeValue::Text(value) => value.clone(),
        RuntimeValue::ElementReference(id) => format!("element {id}"),
    }
}

fn engine_error(message: impl Into<String>) -> ExecutionError {
    ExecutionError::Engine {
        message: message.into(),
    }
}
