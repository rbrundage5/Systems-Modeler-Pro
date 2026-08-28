use crate::execution::validate_runtime_assignment;
use crate::{
    BehaviorRepository, ElementId, ElementKind, EngineStepOutcome, ExecutionEngine, ExecutionError,
    ExecutionSession, ExecutionSnapshot, InteractionId, LifelineId, Message, MessageId,
    MessageSignature, MessageSort, ParameterDirection, Project, RuntimeEvent, RuntimeEventAddress,
    RuntimeEventKind, RuntimeEventRequest, RuntimeInstanceId, RuntimeValue,
    StateMachineExecutionEngine, evaluate_execution_expression,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct ModeledOperationRequest {
    pub operation_id: ElementId,
    pub target_runtime_instance_id: RuntimeInstanceId,
    pub arguments: Vec<(String, RuntimeValue)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModeledOperationResult {
    pub operation_id: ElementId,
    pub target_runtime_instance_id: RuntimeInstanceId,
    pub outputs: Vec<(String, RuntimeValue)>,
}

/// Executes the bounded semantics represented by the current Operation model.
/// Inputs are validated and stored in the selected occurrence context. InOut
/// values are echoed and authored defaults provide deterministic Out/Return
/// values. Arbitrary implementation text is deliberately never interpreted.
pub fn invoke_modeled_operation(
    project: &Project,
    session: &mut ExecutionSession,
    request: &ModeledOperationRequest,
) -> Result<ModeledOperationResult, ExecutionError> {
    let operation = project
        .element(request.operation_id)
        .map_err(|error| engine_error(error.to_string()))?;
    if operation.kind != ElementKind::Operation {
        return Err(engine_error(format!(
            "'{}' is not a modeled Operation and cannot be invoked.",
            readable_path(project, request.operation_id)
        )));
    }
    let owner_id = operation.owner_id.ok_or_else(|| {
        engine_error(format!(
            "Operation '{}' has no owning classifier.",
            readable_path(project, operation.id)
        ))
    })?;
    let owner = project
        .element(owner_id)
        .map_err(|error| engine_error(error.to_string()))?;
    let target = session
        .instances
        .get(&request.target_runtime_instance_id)
        .ok_or(ExecutionError::RuntimeInstanceNotFound(
            request.target_runtime_instance_id,
        ))?
        .clone();
    let compatible = session.structural_runtime.as_ref().is_some_and(|runtime| {
        runtime.instance_conforms_to(project, request.target_runtime_instance_id, owner_id)
    });
    if !compatible {
        return Err(engine_error(format!(
            "{} cannot invoke Operation '{}': the Operation is owned by '{}', while the selected runtime occurrence is typed by '{}'.",
            target.qualified_path, operation.name, owner.name, target.classifier_name
        )));
    }

    let mut parameters: Vec<_> = project
        .children(operation.id)
        .filter(|element| element.kind == ElementKind::Parameter)
        .collect();
    parameters.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.id.to_string().cmp(&right.id.to_string()))
    });
    let mut supplied = HashMap::new();
    for (name, value) in &request.arguments {
        if supplied.insert(name.as_str(), value).is_some() {
            return Err(engine_error(format!(
                "Operation '{}': parameter '{}' was supplied more than once.",
                operation.name, name
            )));
        }
    }
    let known_inputs: HashSet<_> = parameters
        .iter()
        .filter(|parameter| {
            matches!(
                parameter
                    .parameter_direction
                    .unwrap_or(ParameterDirection::In),
                ParameterDirection::In | ParameterDirection::InOut
            )
        })
        .map(|parameter| parameter.name.as_str())
        .collect();
    if let Some(unknown) = supplied.keys().find(|name| !known_inputs.contains(**name)) {
        return Err(engine_error(format!(
            "Operation '{}': '{}' is not an input parameter. Expected: {}.",
            operation.name,
            unknown,
            readable_names(&known_inputs)
        )));
    }

    let mut inputs = HashMap::new();
    for parameter in parameters.iter().filter(|parameter| {
        matches!(
            parameter
                .parameter_direction
                .unwrap_or(ParameterDirection::In),
            ParameterDirection::In | ParameterDirection::InOut
        )
    }) {
        let value = if let Some(value) = supplied.get(parameter.name.as_str()) {
            Some((*value).clone())
        } else if let Some(authored) = parameter.default_value.as_deref() {
            Some(parse_authored_value(authored)?)
        } else {
            None
        };
        let required = parameter.multiplicity.unwrap_or_default().lower > 0;
        let value = match value {
            Some(value) => value,
            None if required => {
                return Err(engine_error(format!(
                    "Operation '{}': required input parameter '{}' was not supplied.",
                    operation.name, parameter.name
                )));
            }
            None => RuntimeValue::Unset,
        };
        validate_runtime_assignment(project, parameter, &value).map_err(|error| {
            engine_error(format!(
                "Operation '{}': input parameter '{}' is invalid: {error}",
                operation.name, parameter.name
            ))
        })?;
        session.set_value(
            project,
            Some(request.target_runtime_instance_id),
            parameter.id,
            value.clone(),
        )?;
        inputs.insert(parameter.id, value);
    }

    let mut outputs = Vec::new();
    for parameter in parameters.iter().filter(|parameter| {
        matches!(
            parameter.parameter_direction,
            Some(ParameterDirection::Out | ParameterDirection::Return | ParameterDirection::InOut)
        )
    }) {
        let direction = parameter
            .parameter_direction
            .expect("filtered Operation parameter direction");
        let value = if direction == ParameterDirection::InOut {
            inputs
                .get(&parameter.id)
                .cloned()
                .unwrap_or(RuntimeValue::Unset)
        } else if let Some(authored) = parameter.default_value.as_deref() {
            parse_authored_value(authored)?
        } else if parameter.multiplicity.unwrap_or_default().lower == 0 {
            RuntimeValue::Unset
        } else {
            return Err(engine_error(format!(
                "Operation '{}': {} parameter '{}' has no bounded authored value. Add a default value or leave this execution semantic unsupported.",
                operation.name,
                direction_name(direction),
                parameter.name
            )));
        };
        validate_runtime_assignment(project, parameter, &value).map_err(|error| {
            engine_error(format!(
                "Operation '{}': {} parameter '{}' is invalid: {error}",
                operation.name,
                direction_name(direction),
                parameter.name
            ))
        })?;
        session.set_value(
            project,
            Some(request.target_runtime_instance_id),
            parameter.id,
            value.clone(),
        )?;
        outputs.push((parameter.name.clone(), value));
    }
    session.record_engine_trace(
        Some(operation.id),
        format!(
            "Invoked Operation '{}.{}' on {}",
            owner.name, operation.name, target.qualified_path
        ),
    );
    session.queue_typed_event_at(
        project,
        RuntimeEventRequest {
            due_time: session.simulation_time,
            kind: RuntimeEventKind::Call,
            name: operation.name.clone(),
            semantic_event_id: Some(operation.id),
            address: RuntimeEventAddress {
                target_semantic_id: Some(owner_id),
                target_runtime_instance_id: Some(request.target_runtime_instance_id),
                ..RuntimeEventAddress::default()
            },
            payload: request.arguments.clone(),
        },
    )?;
    Ok(ModeledOperationResult {
        operation_id: operation.id,
        target_runtime_instance_id: request.target_runtime_instance_id,
        outputs,
    })
}

pub fn positional_operation_arguments(
    project: &Project,
    operation_id: ElementId,
    arguments: &[String],
) -> Result<Vec<(String, RuntimeValue)>, ExecutionError> {
    let operation = project
        .element(operation_id)
        .map_err(|error| engine_error(error.to_string()))?;
    let mut inputs: Vec<_> = project
        .children(operation_id)
        .filter(|parameter| {
            parameter.kind == ElementKind::Parameter
                && matches!(
                    parameter
                        .parameter_direction
                        .unwrap_or(ParameterDirection::In),
                    ParameterDirection::In | ParameterDirection::InOut
                )
        })
        .collect();
    inputs.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.id.to_string().cmp(&right.id.to_string()))
    });
    if arguments.len() > inputs.len() {
        return Err(engine_error(format!(
            "Operation '{}': received {} Sequence argument(s), but only {} input parameter(s) exist.",
            operation.name,
            arguments.len(),
            inputs.len()
        )));
    }
    arguments
        .iter()
        .zip(inputs)
        .map(|(expression, parameter)| {
            let value = parse_authored_value(expression)?;
            Ok((parameter.name.clone(), value))
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SequenceLifelineBinding {
    pub lifeline_id: LifelineId,
    pub lifeline_name: String,
    pub runtime_instance_id: RuntimeInstanceId,
    pub runtime_path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequenceExecutionSnapshot {
    pub execution: ExecutionSnapshot,
    pub interaction_id: InteractionId,
    pub lifeline_bindings: Vec<SequenceLifelineBinding>,
    pub active_message_id: Option<MessageId>,
    pub completed_message_ids: Vec<MessageId>,
    pub next_message_index: usize,
}

pub struct SequenceExecutionEngine {
    repository: BehaviorRepository,
    interaction_id: InteractionId,
    ordered_message_ids: Vec<MessageId>,
    lifeline_bindings: HashMap<LifelineId, RuntimeInstanceId>,
    next_message_index: usize,
    active_message_id: Option<MessageId>,
    completed_message_ids: Vec<MessageId>,
    state_machine_engines: Vec<StateMachineExecutionEngine>,
}

impl SequenceExecutionEngine {
    pub fn new(repository: BehaviorRepository, interaction_id: InteractionId) -> Self {
        Self {
            repository,
            interaction_id,
            ordered_message_ids: Vec::new(),
            lifeline_bindings: HashMap::new(),
            next_message_index: 0,
            active_message_id: None,
            completed_message_ids: Vec::new(),
            state_machine_engines: Vec::new(),
        }
    }

    pub fn reset(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<(), ExecutionError> {
        session.reset(project)?;
        self.initialize_runtime(project, session)
    }

    pub fn snapshot(&self, session: &ExecutionSession) -> SequenceExecutionSnapshot {
        let interaction = &self.repository.interactions[&self.interaction_id];
        let mut lifeline_bindings: Vec<_> = interaction
            .lifelines
            .iter()
            .filter_map(|lifeline| {
                let instance_id = self.lifeline_bindings.get(&lifeline.id).copied()?;
                let instance = session.instances.get(&instance_id)?;
                Some(SequenceLifelineBinding {
                    lifeline_id: lifeline.id,
                    lifeline_name: lifeline.name.clone(),
                    runtime_instance_id: instance_id,
                    runtime_path: instance.qualified_path.clone(),
                })
            })
            .collect();
        lifeline_bindings.sort_by(|left, right| left.lifeline_name.cmp(&right.lifeline_name));
        SequenceExecutionSnapshot {
            execution: session.snapshot(),
            interaction_id: self.interaction_id,
            lifeline_bindings,
            active_message_id: self.active_message_id,
            completed_message_ids: self.completed_message_ids.clone(),
            next_message_index: self.next_message_index,
        }
    }

    pub fn state_machine_snapshots(
        &self,
        session: &ExecutionSession,
    ) -> Vec<crate::StateMachineExecutionSnapshot> {
        self.state_machine_engines
            .iter()
            .map(|engine| engine.snapshot(session))
            .collect()
    }

    fn initialize_runtime(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<(), ExecutionError> {
        self.repository.validate(project).map_err(|error| {
            engine_error(format!("Cannot initialize Sequence execution: {error}"))
        })?;
        let interaction = self.interaction()?.clone();
        let runtime = session.structural_runtime.as_ref().ok_or_else(|| {
            engine_error(format!(
                "Sequence '{}' requires a structural runtime root so Lifelines can resolve to modeled occurrences.",
                interaction.name
            ))
        })?;
        self.lifeline_bindings.clear();
        for lifeline in &interaction.lifelines {
            let usage_id = *lifeline.represented_path.last().ok_or_else(|| {
                engine_error(format!(
                    "Sequence Lifeline '{}' has no represented structural path.",
                    lifeline.name
                ))
            })?;
            let candidates = runtime.instances_for_usage(usage_id);
            if candidates.len() != 1 {
                let paths = candidates
                    .iter()
                    .map(|instance| instance.qualified_path.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(engine_error(format!(
                    "Sequence Lifeline '{}' resolves to {} runtime occurrence(s){}. Select an unambiguous structural root or population configuration.",
                    lifeline.name,
                    candidates.len(),
                    if paths.is_empty() {
                        String::new()
                    } else {
                        format!(": {paths}")
                    }
                )));
            }
            self.lifeline_bindings.insert(lifeline.id, candidates[0].id);
        }
        self.state_machine_engines.clear();
        let mut bound_instances: Vec<_> = self.lifeline_bindings.values().copied().collect();
        bound_instances.sort_by_key(ToString::to_string);
        bound_instances.dedup();
        for instance_id in bound_instances {
            let classifier_id = session
                .instances
                .get(&instance_id)
                .and_then(|instance| instance.classifier_id)
                .ok_or_else(|| {
                    engine_error(format!(
                        "Sequence runtime occurrence {instance_id} has no classifier."
                    ))
                })?;
            let mut machines: Vec<_> = self
                .repository
                .state_machines
                .values()
                .filter(|machine| machine.context_id == classifier_id)
                .map(|machine| machine.id)
                .collect();
            machines.sort_by_key(ToString::to_string);
            if machines.len() > 1 {
                let path = &session.instances[&instance_id].qualified_path;
                return Err(engine_error(format!(
                    "Sequence participant '{path}' has {} modeled State Machines. PR34 requires one unambiguous executable State Machine per runtime occurrence.",
                    machines.len()
                )));
            }
            if let Some(machine_id) = machines.first().copied() {
                let mut engine =
                    StateMachineExecutionEngine::new_embedded(self.repository.clone(), machine_id)
                        .with_runtime_instance(instance_id);
                engine.initialize_embedded(project, session)?;
                self.state_machine_engines.push(engine);
            }
        }
        self.ordered_message_ids = interaction
            .messages
            .iter()
            .map(|message| message.id)
            .collect();
        self.ordered_message_ids.sort_by(|left, right| {
            let left = interaction
                .messages
                .iter()
                .find(|message| message.id == *left)
                .expect("Sequence message exists");
            let right = interaction
                .messages
                .iter()
                .find(|message| message.id == *right)
                .expect("Sequence message exists");
            message_order(left)
                .cmp(&message_order(right))
                .then_with(|| left.id.to_string().cmp(&right.id.to_string()))
        });
        for message in &interaction.messages {
            match message.sort {
                MessageSort::SynchCall
                | MessageSort::AsynchCall
                | MessageSort::AsynchSignal
                | MessageSort::Reply => {}
                _ => {
                    return Err(engine_error(format!(
                        "Sequence message '{}' uses {:?}, which is not executable in PR34.",
                        message.name, message.sort
                    )));
                }
            }
        }
        self.next_message_index = 0;
        self.active_message_id = None;
        self.completed_message_ids.clear();
        session.record_engine_trace(
            Some(interaction.context_id),
            format!("Initialized Sequence '{}'", interaction.name),
        );
        Ok(())
    }

    fn interaction(&self) -> Result<&crate::Interaction, ExecutionError> {
        self.repository
            .interactions
            .get(&self.interaction_id)
            .ok_or_else(|| {
                engine_error("Sequence execution references a missing Interaction.".into())
            })
    }

    fn execute_message(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
        message: &Message,
    ) -> Result<(), ExecutionError> {
        let source = message
            .send_event
            .as_ref()
            .and_then(|occurrence| self.lifeline_bindings.get(&occurrence.lifeline_id))
            .copied();
        let target = message
            .receive_event
            .as_ref()
            .and_then(|occurrence| self.lifeline_bindings.get(&occurrence.lifeline_id))
            .copied();
        match message.sort {
            MessageSort::SynchCall | MessageSort::AsynchCall => {
                let operation_id = match message.signature {
                    Some(MessageSignature::Operation(id)) => id,
                    _ => {
                        return Err(engine_error(format!(
                            "Sequence message '{}' has no modeled Operation target.",
                            message.name
                        )));
                    }
                };
                let target = target.ok_or_else(|| {
                    engine_error(format!(
                        "Sequence message '{}' has no resolvable target Lifeline.",
                        message.name
                    ))
                })?;
                let arguments =
                    positional_operation_arguments(project, operation_id, &message.arguments)?;
                invoke_modeled_operation(
                    project,
                    session,
                    &ModeledOperationRequest {
                        operation_id,
                        target_runtime_instance_id: target,
                        arguments,
                    },
                )?;
                self.dispatch_current_events(project, session)?;
            }
            MessageSort::AsynchSignal => {
                let signal_id = match message.signature {
                    Some(MessageSignature::Signal(id)) => id,
                    _ => {
                        return Err(engine_error(format!(
                            "Sequence message '{}' has no modeled Signal target.",
                            message.name
                        )));
                    }
                };
                let source = source.ok_or_else(|| {
                    engine_error(format!(
                        "Sequence Signal message '{}' has no resolvable source Lifeline.",
                        message.name
                    ))
                })?;
                let target = target.ok_or_else(|| {
                    engine_error(format!(
                        "Sequence Signal message '{}' has no resolvable target Lifeline.",
                        message.name
                    ))
                })?;
                let payload = message
                    .arguments
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        parse_authored_value(value).map(|value| (format!("arg{index}"), value))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                session.queue_structural_signal_to_instance(
                    project,
                    source,
                    target,
                    signal_id,
                    message.name.clone(),
                    payload,
                )?;
                self.dispatch_current_events(project, session)?;
            }
            MessageSort::Reply => {
                session.record_engine_trace(
                    None,
                    format!("Sequence reply '{}' completed", message.name),
                );
            }
            _ => unreachable!("unsupported Sequence sorts are rejected at initialization"),
        }
        session.record_engine_trace(
            None,
            format!("Sequence executed message '{}'", message.name),
        );
        Ok(())
    }

    fn dispatch_current_events(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<(), ExecutionError> {
        while session
            .next_event()
            .is_some_and(|scheduled| scheduled.due_time <= session.simulation_time)
        {
            let event = session
                .step()?
                .expect("an inspected current event remains queued");
            for engine in &mut self.state_machine_engines {
                engine.handle_event(project, session, &event)?;
            }
        }
        Ok(())
    }
}

impl ExecutionEngine for SequenceExecutionEngine {
    fn initialize(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<(), ExecutionError> {
        session.initialize(project)?;
        self.initialize_runtime(project, session)
    }

    fn step(
        &mut self,
        project: &Project,
        session: &mut ExecutionSession,
    ) -> Result<EngineStepOutcome, ExecutionError> {
        if self.next_message_index >= self.ordered_message_ids.len() {
            self.active_message_id = None;
            session.complete()?;
            return Ok(EngineStepOutcome::Completed);
        }
        session.consume_step_budget()?;
        let message_id = self.ordered_message_ids[self.next_message_index];
        let message = self
            .interaction()?
            .messages
            .iter()
            .find(|message| message.id == message_id)
            .cloned()
            .ok_or_else(|| engine_error("Sequence message disappeared during execution.".into()))?;
        self.active_message_id = Some(message.id);
        self.execute_message(project, session, &message)?;
        self.completed_message_ids.push(message.id);
        self.next_message_index += 1;
        Ok(EngineStepOutcome::Progressed)
    }

    fn handle_event(
        &mut self,
        _project: &Project,
        _session: &mut ExecutionSession,
        _event: &RuntimeEvent,
    ) -> Result<EngineStepOutcome, ExecutionError> {
        Ok(EngineStepOutcome::Idle)
    }
}

fn parse_authored_value(value: &str) -> Result<RuntimeValue, ExecutionError> {
    if let Ok(value) = evaluate_execution_expression(value, |_| None) {
        return Ok(value);
    }
    serde_json::from_str::<String>(value)
        .map(RuntimeValue::Text)
        .map_err(|error| {
            engine_error(format!(
                "Authored runtime value '{value}' is not a bounded literal: {error}"
            ))
        })
}

fn message_order(message: &Message) -> (u32, u32) {
    let send = message
        .send_event
        .as_ref()
        .map(|occurrence| occurrence.order)
        .unwrap_or(u32::MAX);
    let receive = message
        .receive_event
        .as_ref()
        .map(|occurrence| occurrence.order)
        .unwrap_or(u32::MAX);
    (send.min(receive), send.max(receive))
}

fn direction_name(direction: ParameterDirection) -> &'static str {
    match direction {
        ParameterDirection::In => "input",
        ParameterDirection::Out => "output",
        ParameterDirection::InOut => "inout",
        ParameterDirection::Return => "return",
    }
}

fn readable_path(project: &Project, id: ElementId) -> String {
    project
        .qualified_name(id)
        .unwrap_or_else(|_| id.to_string())
}

fn readable_names(names: &HashSet<&str>) -> String {
    let mut names: Vec<_> = names.iter().copied().collect();
    names.sort_unstable();
    if names.is_empty() {
        "none".into()
    } else {
        names.join(", ")
    }
}

fn engine_error(message: String) -> ExecutionError {
    ExecutionError::Engine { message }
}
