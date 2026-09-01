use super::*;
use systems_modeler_core::behavior::{
    BehaviorRepository, BehaviorSemanticId, CombinedFragment, ExecutionId,
    ExecutionSpecification, FragmentId, Interaction, InteractionId, InteractionOperand,
    InteractionOperator, InvariantId, Lifeline, LifelineId, Message, MessageId, MessageSignature,
    MessageSort, Occurrence, OccurrenceId, OperandId, StateInvariant,
};
use systems_modeler_core::{BindingConnector, BindingEndpoint, ElementId, RelationshipId};

pub type InteractionReference = BuildReference<InteractionId>;
pub type LifelineReference = BuildReference<LifelineId>;
pub type OccurrenceReference = BuildReference<OccurrenceId>;
pub type MessageReference = BuildReference<MessageId>;
pub type ExecutionReference = BuildReference<ExecutionId>;
pub type FragmentReference = BuildReference<FragmentId>;
pub type OperandReference = BuildReference<OperandId>;
pub type InvariantReference = BuildReference<InvariantId>;

#[derive(Debug, Clone)]
pub enum MessageSignatureBuild {
    Operation(ElementReference),
    Signal(ElementReference),
}

#[derive(Debug, Clone)]
pub enum SequenceBuildOperation {
    CreateInteraction {
        external_id: String,
        name: String,
        context: ElementReference,
    },
    UpdateInteraction {
        interaction: InteractionReference,
        name: Option<String>,
        context: Option<ElementReference>,
    },
    CreateLifeline {
        external_id: String,
        interaction: InteractionReference,
        name: String,
        represented_path: Vec<ElementReference>,
    },
    UpdateLifeline {
        lifeline: LifelineReference,
        name: Option<String>,
        represented_path: Option<Vec<ElementReference>>,
    },
    CreateOccurrence {
        external_id: String,
        interaction: InteractionReference,
        lifeline: LifelineReference,
        order: u32,
    },
    UpdateOccurrence {
        occurrence: OccurrenceReference,
        lifeline: Option<LifelineReference>,
        order: Option<u32>,
    },
    CreateMessage {
        external_id: String,
        interaction: InteractionReference,
        name: String,
        sort: MessageSort,
        send: Option<OccurrenceReference>,
        receive: Option<OccurrenceReference>,
        signature: Option<MessageSignatureBuild>,
        arguments: Vec<String>,
    },
    UpdateMessage {
        message: MessageReference,
        name: Option<String>,
        sort: Option<MessageSort>,
        send: Option<Option<OccurrenceReference>>,
        receive: Option<Option<OccurrenceReference>>,
        signature: Option<Option<MessageSignatureBuild>>,
        arguments: Option<Vec<String>>,
    },
    CreateExecution {
        external_id: String,
        interaction: InteractionReference,
        lifeline: LifelineReference,
        start: OccurrenceReference,
        finish: OccurrenceReference,
        behavior: Option<ElementReference>,
    },
    UpdateExecution {
        execution: ExecutionReference,
        lifeline: Option<LifelineReference>,
        start: Option<OccurrenceReference>,
        finish: Option<OccurrenceReference>,
        behavior: Option<Option<ElementReference>>,
    },
    CreateFragment {
        external_id: String,
        interaction: InteractionReference,
        operator: InteractionOperator,
        covered_lifelines: Vec<LifelineReference>,
    },
    UpdateFragment {
        fragment: FragmentReference,
        operator: Option<InteractionOperator>,
        covered_lifelines: Option<Vec<LifelineReference>>,
    },
    CreateOperand {
        external_id: String,
        fragment: FragmentReference,
        guard: Option<String>,
        start_order: u32,
        end_order: u32,
    },
    UpdateOperand {
        operand: OperandReference,
        guard: Option<Option<String>>,
        start_order: Option<u32>,
        end_order: Option<u32>,
    },
    CreateInvariant {
        external_id: String,
        interaction: InteractionReference,
        lifeline: LifelineReference,
        order: u32,
        constraint: String,
    },
    UpdateInvariant {
        invariant: InvariantReference,
        lifeline: Option<LifelineReference>,
        order: Option<u32>,
        constraint: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct BindingEndpointBuild {
    pub role: ElementReference,
    pub parameter: Option<ElementReference>,
}

#[derive(Debug, Clone)]
pub enum ParametricBuildOperation {
    UpdateElementSemantics {
        element: ElementReference,
        constraint_expression: Option<String>,
        quantity_kind_external_id: Option<Option<String>>,
        unit_external_id: Option<Option<String>>,
        quantity_dimension: Option<Option<String>>,
        unit_symbol: Option<Option<String>>,
        unit_scale_to_base: Option<f64>,
    },
    CreateBinding {
        external_id: String,
        name: String,
        owner: ElementReference,
        source: BindingEndpointBuild,
        target: BindingEndpointBuild,
    },
    UpdateBinding {
        relationship: RelationshipReference,
        name: Option<String>,
        owner: Option<ElementReference>,
        source: Option<BindingEndpointBuild>,
        target: Option<BindingEndpointBuild>,
    },
}

#[derive(Default)]
pub(super) struct SequenceBuildState {
    occurrences: HashMap<OccurrenceId, (InteractionId, Occurrence)>,
}

impl SequenceBuildState {
    pub(super) fn from_repository(
        repository: &BehaviorRepository,
    ) -> Result<Self, BuildDiagnostic> {
        let mut state = Self::default();
        for interaction in repository.interactions.values() {
            for occurrence in interaction
                .messages
                .iter()
                .flat_map(|message| [message.send_event.as_ref(), message.receive_event.as_ref()])
                .flatten()
                .chain(
                    interaction
                        .executions
                        .iter()
                        .flat_map(|execution| [&execution.start, &execution.finish]),
                )
            {
                if let Some((owner, previous)) = state.occurrences.get(&occurrence.id)
                    && (*owner != interaction.id
                        || previous.lifeline_id != occurrence.lifeline_id
                        || previous.order != occurrence.order)
                {
                    return Err(error(
                        "OCCURRENCE_ID_CONFLICT",
                        None,
                        format!(
                            "Occurrence {} has inconsistent embedded semantic values",
                            occurrence.id
                        ),
                    ));
                }
                state
                    .occurrences
                    .insert(occurrence.id, (interaction.id, occurrence.clone()));
            }
        }
        Ok(state)
    }
}

pub(super) fn sequence_create_external_id(operation: &SequenceBuildOperation) -> Option<&String> {
    match operation {
        SequenceBuildOperation::CreateInteraction { external_id, .. }
        | SequenceBuildOperation::CreateLifeline { external_id, .. }
        | SequenceBuildOperation::CreateOccurrence { external_id, .. }
        | SequenceBuildOperation::CreateMessage { external_id, .. }
        | SequenceBuildOperation::CreateExecution { external_id, .. }
        | SequenceBuildOperation::CreateFragment { external_id, .. }
        | SequenceBuildOperation::CreateOperand { external_id, .. }
        | SequenceBuildOperation::CreateInvariant { external_id, .. } => Some(external_id),
        _ => None,
    }
}

pub(super) fn parametric_create_external_id(
    operation: &ParametricBuildOperation,
) -> Option<&String> {
    match operation {
        ParametricBuildOperation::CreateBinding { external_id, .. } => Some(external_id),
        _ => None,
    }
}

pub(super) fn sequence_operation_description(operation: &SequenceBuildOperation) -> String {
    let (action, kind, external) = match operation {
        SequenceBuildOperation::CreateInteraction { external_id, .. } =>
            ("CREATE", "Interaction", Some(external_id)),
        SequenceBuildOperation::CreateLifeline { external_id, .. } =>
            ("CREATE", "Lifeline", Some(external_id)),
        SequenceBuildOperation::CreateOccurrence { external_id, .. } =>
            ("CREATE", "Occurrence", Some(external_id)),
        SequenceBuildOperation::CreateMessage { external_id, .. } =>
            ("CREATE", "Message", Some(external_id)),
        SequenceBuildOperation::CreateExecution { external_id, .. } =>
            ("CREATE", "ExecutionSpecification", Some(external_id)),
        SequenceBuildOperation::CreateFragment { external_id, .. } =>
            ("CREATE", "CombinedFragment", Some(external_id)),
        SequenceBuildOperation::CreateOperand { external_id, .. } =>
            ("CREATE", "InteractionOperand", Some(external_id)),
        SequenceBuildOperation::CreateInvariant { external_id, .. } =>
            ("CREATE", "StateInvariant", Some(external_id)),
        SequenceBuildOperation::UpdateInteraction { .. } => ("UPDATE", "Interaction", None),
        SequenceBuildOperation::UpdateLifeline { .. } => ("UPDATE", "Lifeline", None),
        SequenceBuildOperation::UpdateOccurrence { .. } => ("UPDATE", "Occurrence", None),
        SequenceBuildOperation::UpdateMessage { .. } => ("UPDATE", "Message", None),
        SequenceBuildOperation::UpdateExecution { .. } =>
            ("UPDATE", "ExecutionSpecification", None),
        SequenceBuildOperation::UpdateFragment { .. } =>
            ("UPDATE", "CombinedFragment", None),
        SequenceBuildOperation::UpdateOperand { .. } =>
            ("UPDATE", "InteractionOperand", None),
        SequenceBuildOperation::UpdateInvariant { .. } =>
            ("UPDATE", "StateInvariant", None),
    };
    external.map_or_else(
        || format!("{action} {kind}"),
        |external| format!("{action} {kind} {external}"),
    )
}

pub(super) fn parametric_operation_description(operation: &ParametricBuildOperation) -> String {
    match operation {
        ParametricBuildOperation::UpdateElementSemantics { .. } =>
            "UPDATE Parametric element semantics".into(),
        ParametricBuildOperation::CreateBinding { external_id, .. } =>
            format!("CREATE BindingConnector {external_id}"),
        ParametricBuildOperation::UpdateBinding { .. } => "UPDATE BindingConnector".into(),
    }
}

fn resolve_element_reference(
    project: &Project,
    planned: &HashMap<String, ElementId>,
    namespace: &str,
    reference: &ElementReference,
    operation: usize,
) -> Result<ElementId, BuildDiagnostic> {
    resolve_element(project, planned, namespace, reference, operation)
}

fn resolve_relationship_reference(
    project: &Project,
    planned: &HashMap<String, RelationshipId>,
    namespace: &str,
    reference: &RelationshipReference,
    operation: usize,
) -> Result<RelationshipId, BuildDiagnostic> {
    resolve_relationship(project, planned, namespace, reference, operation)
}

fn behavior_external_available(
    key: &str,
    project: &Project,
    repository: &BehaviorRepository,
    operation: usize,
) -> Result<(), BuildDiagnostic> {
    let collision = project
        .elements
        .values()
        .any(|element| element.external_id == key)
        || project
            .relationships
            .values()
            .any(|relationship| relationship.external_id == key)
        || repository
            .state_machines
            .values()
            .any(|machine| machine.external_id == key)
        || repository
            .interactions
            .values()
            .any(|interaction| interaction.external_id == key)
        || repository.external_ids.contains_key(key);
    if collision {
        Err(error(
            "DUPLICATE_EXTERNAL_ID",
            Some(operation),
            format!("external ID already exists across authored semantic stores: {key}"),
        ))
    } else {
        Ok(())
    }
}

fn resolve_interaction(
    repository: &BehaviorRepository,
    namespace: &str,
    reference: &InteractionReference,
    operation: usize,
) -> Result<InteractionId, BuildDiagnostic> {
    match reference {
        BuildReference::Existing(id) if repository.interactions.contains_key(id) => Ok(*id),
        BuildReference::Existing(id) => Err(error(
            "UNRESOLVED_SEQUENCE_REFERENCE",
            Some(operation),
            format!("Interaction {id} was not found"),
        )),
        BuildReference::External(external) => {
            let key = external_key(namespace, external);
            let matches = repository
                .interactions
                .values()
                .filter(|record| record.external_id == key)
                .map(|record| record.id)
                .collect::<Vec<_>>();
            match matches.as_slice() {
                [id] => Ok(*id),
                [] => Err(error(
                    "UNRESOLVED_SEQUENCE_REFERENCE",
                    Some(operation),
                    format!("Interaction external ID '{external}' was not found"),
                )),
                _ => Err(error(
                    "AMBIGUOUS_SEQUENCE_REFERENCE",
                    Some(operation),
                    format!("Interaction external ID '{external}' is ambiguous"),
                )),
            }
        }
    }
}

fn resolve_semantic<T: Copy>(
    repository: &BehaviorRepository,
    namespace: &str,
    reference: &BuildReference<T>,
    operation: usize,
    expected: fn(BehaviorSemanticId) -> Option<T>,
    label: &str,
) -> Result<T, BuildDiagnostic> {
    match reference {
        BuildReference::Existing(id) => Ok(*id),
        BuildReference::External(external) => repository
            .external_ids
            .get(&external_key(namespace, external))
            .copied()
            .and_then(expected)
            .ok_or_else(|| {
                error(
                    "UNRESOLVED_SEQUENCE_REFERENCE",
                    Some(operation),
                    format!(
                        "{label} external ID '{external}' was not found with the required semantic kind"
                    ),
                )
            }),
    }
}

macro_rules! semantic_resolver {
    ($name:ident, $reference:ty, $target:ty, $variant:ident, $label:literal) => {
        fn $name(
            repository: &BehaviorRepository,
            namespace: &str,
            reference: &$reference,
            operation: usize,
        ) -> Result<$target, BuildDiagnostic> {
            resolve_semantic(
                repository,
                namespace,
                reference,
                operation,
                |identity| match identity {
                    BehaviorSemanticId::$variant(id) => Some(id),
                    _ => None,
                },
                $label,
            )
        }
    };
}

semantic_resolver!(resolve_lifeline, LifelineReference, LifelineId, Lifeline, "Lifeline");
semantic_resolver!(resolve_occurrence, OccurrenceReference, OccurrenceId, Occurrence, "Occurrence");
semantic_resolver!(resolve_message, MessageReference, MessageId, Message, "Message");
semantic_resolver!(resolve_execution, ExecutionReference, ExecutionId, Execution, "ExecutionSpecification");
semantic_resolver!(resolve_fragment, FragmentReference, FragmentId, Fragment, "CombinedFragment");
semantic_resolver!(resolve_operand, OperandReference, OperandId, Operand, "InteractionOperand");
semantic_resolver!(resolve_invariant, InvariantReference, InvariantId, Invariant, "StateInvariant");

fn resolve_signature(
    signature: &MessageSignatureBuild,
    project: &Project,
    planned: &HashMap<String, ElementId>,
    namespace: &str,
    operation: usize,
) -> Result<MessageSignature, BuildDiagnostic> {
    Ok(match signature {
        MessageSignatureBuild::Operation(reference) => MessageSignature::Operation(
            resolve_element_reference(project, planned, namespace, reference, operation)?,
        ),
        MessageSignatureBuild::Signal(reference) => MessageSignature::Signal(
            resolve_element_reference(project, planned, namespace, reference, operation)?,
        ),
    })
}

fn interaction_for_lifeline(
    repository: &BehaviorRepository,
    id: LifelineId,
) -> Option<InteractionId> {
    repository
        .interactions
        .values()
        .find(|interaction| interaction.lifelines.iter().any(|record| record.id == id))
        .map(|interaction| interaction.id)
}

fn interaction_for_message(repository: &BehaviorRepository, id: MessageId) -> Option<InteractionId> {
    repository
        .interactions
        .values()
        .find(|interaction| interaction.messages.iter().any(|record| record.id == id))
        .map(|interaction| interaction.id)
}

fn interaction_for_execution(
    repository: &BehaviorRepository,
    id: ExecutionId,
) -> Option<InteractionId> {
    repository
        .interactions
        .values()
        .find(|interaction| interaction.executions.iter().any(|record| record.id == id))
        .map(|interaction| interaction.id)
}

fn interaction_for_fragment(
    repository: &BehaviorRepository,
    id: FragmentId,
) -> Option<InteractionId> {
    repository
        .interactions
        .values()
        .find(|interaction| interaction.fragments.iter().any(|record| record.id == id))
        .map(|interaction| interaction.id)
}

fn fragment_for_operand(repository: &BehaviorRepository, id: OperandId) -> Option<FragmentId> {
    repository.interactions.values().find_map(|interaction| {
        interaction.fragments.iter().find_map(|fragment| {
            fragment
                .operands
                .iter()
                .any(|operand| operand.id == id)
                .then_some(fragment.id)
        })
    })
}

fn interaction_for_invariant(
    repository: &BehaviorRepository,
    id: InvariantId,
) -> Option<InteractionId> {
    repository
        .interactions
        .values()
        .find(|interaction| interaction.state_invariants.iter().any(|record| record.id == id))
        .map(|interaction| interaction.id)
}

fn occurrence_value(
    state: &SequenceBuildState,
    id: OccurrenceId,
    interaction: InteractionId,
    operation: usize,
) -> Result<Occurrence, BuildDiagnostic> {
    let (owner, occurrence) = state.occurrences.get(&id).ok_or_else(|| {
        error(
            "UNRESOLVED_SEQUENCE_REFERENCE",
            Some(operation),
            format!("Occurrence {id} was not found"),
        )
    })?;
    if *owner != interaction {
        return Err(error(
            "SEQUENCE_INTERACTION_MISMATCH",
            Some(operation),
            format!("Occurrence {id} belongs to a different Interaction"),
        ));
    }
    Ok(occurrence.clone())
}

fn propagate_occurrence(repository: &mut BehaviorRepository, occurrence: &Occurrence) {
    for interaction in repository.interactions.values_mut() {
        for message in &mut interaction.messages {
            if message.send_event.as_ref().is_some_and(|record| record.id == occurrence.id) {
                message.send_event = Some(occurrence.clone());
            }
            if message.receive_event.as_ref().is_some_and(|record| record.id == occurrence.id) {
                message.receive_event = Some(occurrence.clone());
            }
        }
        for execution in &mut interaction.executions {
            if execution.start.id == occurrence.id {
                execution.start = occurrence.clone();
            }
            if execution.finish.id == occurrence.id {
                execution.finish = occurrence.clone();
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_sequence_operation(
    build: &SequenceBuildOperation,
    project: &Project,
    planned_elements: &HashMap<String, ElementId>,
    repository: &mut BehaviorRepository,
    state: &mut SequenceBuildState,
    namespace: &str,
    operation: usize,
) -> Result<(), BuildDiagnostic> {
    match build {
        SequenceBuildOperation::CreateInteraction { external_id, name, context } => {
            let key = external_key(namespace, external_id);
            behavior_external_available(&key, project, repository, operation)?;
            let context_id = resolve_element_reference(
                project, planned_elements, namespace, context, operation,
            )?;
            let id = InteractionId::new();
            repository.interactions.insert(id, Interaction {
                id,
                external_id: key,
                name: name.clone(),
                context_id,
                lifelines: Vec::new(),
                messages: Vec::new(),
                executions: Vec::new(),
                fragments: Vec::new(),
                state_invariants: Vec::new(),
            });
        }
        SequenceBuildOperation::UpdateInteraction { interaction, name, context } => {
            let id = resolve_interaction(repository, namespace, interaction, operation)?;
            let context_id = context.as_ref().map(|reference| {
                resolve_element_reference(project, planned_elements, namespace, reference, operation)
            }).transpose()?;
            let record = repository.interactions.get_mut(&id).expect("resolved Interaction");
            if let Some(value) = name { record.name = value.clone(); }
            if let Some(value) = context_id { record.context_id = value; }
        }
        SequenceBuildOperation::CreateLifeline { external_id, interaction, name, represented_path } => {
            let key = external_key(namespace, external_id);
            behavior_external_available(&key, project, repository, operation)?;
            let interaction_id = resolve_interaction(repository, namespace, interaction, operation)?;
            let path = represented_path.iter().map(|reference| {
                resolve_element_reference(project, planned_elements, namespace, reference, operation)
            }).collect::<Result<Vec<_>, _>>()?;
            let id = LifelineId::new();
            repository.interactions.get_mut(&interaction_id).expect("resolved Interaction")
                .lifelines.push(Lifeline { id, name: name.clone(), represented_path: path });
            repository.external_ids.insert(key, BehaviorSemanticId::Lifeline(id));
        }
        SequenceBuildOperation::UpdateLifeline { lifeline, name, represented_path } => {
            let id = resolve_lifeline(repository, namespace, lifeline, operation)?;
            let interaction_id = interaction_for_lifeline(repository, id).ok_or_else(|| error(
                "UNRESOLVED_SEQUENCE_REFERENCE", Some(operation), format!("Lifeline {id} was not found")))?;
            let path = represented_path.as_ref().map(|values| values.iter().map(|reference| {
                resolve_element_reference(project, planned_elements, namespace, reference, operation)
            }).collect::<Result<Vec<_>, _>>()).transpose()?;
            let record = repository.interactions.get_mut(&interaction_id).unwrap().lifelines
                .iter_mut().find(|record| record.id == id).unwrap();
            if let Some(value) = name { record.name = value.clone(); }
            if let Some(value) = path { record.represented_path = value; }
        }
        SequenceBuildOperation::CreateOccurrence { external_id, interaction, lifeline, order } => {
            let key = external_key(namespace, external_id);
            behavior_external_available(&key, project, repository, operation)?;
            let interaction_id = resolve_interaction(repository, namespace, interaction, operation)?;
            let lifeline_id = resolve_lifeline(repository, namespace, lifeline, operation)?;
            if interaction_for_lifeline(repository, lifeline_id) != Some(interaction_id) {
                return Err(error("SEQUENCE_INTERACTION_MISMATCH", Some(operation),
                    "Occurrence Lifeline belongs to a different Interaction"));
            }
            let id = OccurrenceId::new();
            state.occurrences.insert(id, (interaction_id, Occurrence { id, lifeline_id, order: *order }));
            repository.external_ids.insert(key, BehaviorSemanticId::Occurrence(id));
        }
        SequenceBuildOperation::UpdateOccurrence { occurrence, lifeline, order } => {
            let id = resolve_occurrence(repository, namespace, occurrence, operation)?;
            let (interaction_id, current) = state.occurrences.get(&id).cloned().ok_or_else(|| error(
                "UNRESOLVED_SEQUENCE_REFERENCE", Some(operation), format!("Occurrence {id} was not found")))?;
            let lifeline_id = lifeline.as_ref().map(|reference| resolve_lifeline(
                repository, namespace, reference, operation)).transpose()?.unwrap_or(current.lifeline_id);
            if interaction_for_lifeline(repository, lifeline_id) != Some(interaction_id) {
                return Err(error("SEQUENCE_INTERACTION_MISMATCH", Some(operation),
                    "Occurrence Lifeline belongs to a different Interaction"));
            }
            let updated = Occurrence { id, lifeline_id, order: order.unwrap_or(current.order) };
            state.occurrences.insert(id, (interaction_id, updated.clone()));
            propagate_occurrence(repository, &updated);
        }
        SequenceBuildOperation::CreateMessage { external_id, interaction, name, sort, send, receive, signature, arguments } => {
            let key = external_key(namespace, external_id);
            behavior_external_available(&key, project, repository, operation)?;
            let interaction_id = resolve_interaction(repository, namespace, interaction, operation)?;
            let send_event = send.as_ref().map(|reference| resolve_occurrence(repository, namespace, reference, operation)
                .and_then(|id| occurrence_value(state, id, interaction_id, operation))).transpose()?;
            let receive_event = receive.as_ref().map(|reference| resolve_occurrence(repository, namespace, reference, operation)
                .and_then(|id| occurrence_value(state, id, interaction_id, operation))).transpose()?;
            let native_signature = signature.as_ref().map(|value| resolve_signature(
                value, project, planned_elements, namespace, operation)).transpose()?;
            let id = MessageId::new();
            repository.interactions.get_mut(&interaction_id).unwrap().messages.push(Message {
                id, name: name.clone(), sort: *sort, send_event, receive_event,
                signature: native_signature, arguments: arguments.clone(),
            });
            repository.external_ids.insert(key, BehaviorSemanticId::Message(id));
        }
        SequenceBuildOperation::UpdateMessage { message, name, sort, send, receive, signature, arguments } => {
            let id = resolve_message(repository, namespace, message, operation)?;
            let interaction_id = interaction_for_message(repository, id).ok_or_else(|| error(
                "UNRESOLVED_SEQUENCE_REFERENCE", Some(operation), format!("Message {id} was not found")))?;
            let send_event = send.as_ref().map(|value| value.as_ref().map(|reference| resolve_occurrence(
                repository, namespace, reference, operation).and_then(|occurrence| occurrence_value(
                    state, occurrence, interaction_id, operation))).transpose()).transpose()?;
            let receive_event = receive.as_ref().map(|value| value.as_ref().map(|reference| resolve_occurrence(
                repository, namespace, reference, operation).and_then(|occurrence| occurrence_value(
                    state, occurrence, interaction_id, operation))).transpose()).transpose()?;
            let native_signature = signature.as_ref().map(|value| value.as_ref().map(|signature| resolve_signature(
                signature, project, planned_elements, namespace, operation)).transpose()).transpose()?;
            let record = repository.interactions.get_mut(&interaction_id).unwrap().messages
                .iter_mut().find(|record| record.id == id).unwrap();
            if let Some(value) = name { record.name = value.clone(); }
            if let Some(value) = sort { record.sort = *value; }
            if let Some(value) = send_event { record.send_event = value; }
            if let Some(value) = receive_event { record.receive_event = value; }
            if let Some(value) = native_signature { record.signature = value; }
            if let Some(value) = arguments { record.arguments = value.clone(); }
        }
        SequenceBuildOperation::CreateExecution { external_id, interaction, lifeline, start, finish, behavior } => {
            let key = external_key(namespace, external_id);
            behavior_external_available(&key, project, repository, operation)?;
            let interaction_id = resolve_interaction(repository, namespace, interaction, operation)?;
            let lifeline_id = resolve_lifeline(repository, namespace, lifeline, operation)?;
            let start = occurrence_value(state, resolve_occurrence(repository, namespace, start, operation)?, interaction_id, operation)?;
            let finish = occurrence_value(state, resolve_occurrence(repository, namespace, finish, operation)?, interaction_id, operation)?;
            let behavior_id = behavior.as_ref().map(|reference| resolve_element_reference(
                project, planned_elements, namespace, reference, operation)).transpose()?;
            let id = ExecutionId::new();
            repository.interactions.get_mut(&interaction_id).unwrap().executions.push(
                ExecutionSpecification { id, lifeline_id, start, finish, behavior_id });
            repository.external_ids.insert(key, BehaviorSemanticId::Execution(id));
        }
        SequenceBuildOperation::UpdateExecution { execution, lifeline, start, finish, behavior } => {
            let id = resolve_execution(repository, namespace, execution, operation)?;
            let interaction_id = interaction_for_execution(repository, id).ok_or_else(|| error(
                "UNRESOLVED_SEQUENCE_REFERENCE", Some(operation), format!("ExecutionSpecification {id} was not found")))?;
            let lifeline_id = lifeline.as_ref().map(|reference| resolve_lifeline(
                repository, namespace, reference, operation)).transpose()?;
            let start_value = start.as_ref().map(|reference| resolve_occurrence(repository, namespace, reference, operation)
                .and_then(|occurrence| occurrence_value(state, occurrence, interaction_id, operation))).transpose()?;
            let finish_value = finish.as_ref().map(|reference| resolve_occurrence(repository, namespace, reference, operation)
                .and_then(|occurrence| occurrence_value(state, occurrence, interaction_id, operation))).transpose()?;
            let behavior_id = behavior.as_ref().map(|value| value.as_ref().map(|reference| resolve_element_reference(
                project, planned_elements, namespace, reference, operation)).transpose()).transpose()?;
            let record = repository.interactions.get_mut(&interaction_id).unwrap().executions
                .iter_mut().find(|record| record.id == id).unwrap();
            if let Some(value) = lifeline_id { record.lifeline_id = value; }
            if let Some(value) = start_value { record.start = value; }
            if let Some(value) = finish_value { record.finish = value; }
            if let Some(value) = behavior_id { record.behavior_id = value; }
        }
        SequenceBuildOperation::CreateFragment { external_id, interaction, operator, covered_lifelines } => {
            let key = external_key(namespace, external_id);
            behavior_external_available(&key, project, repository, operation)?;
            let interaction_id = resolve_interaction(repository, namespace, interaction, operation)?;
            let covered = covered_lifelines.iter().map(|reference| resolve_lifeline(
                repository, namespace, reference, operation)).collect::<Result<Vec<_>, _>>()?;
            if covered.iter().any(|lifeline| interaction_for_lifeline(repository, *lifeline) != Some(interaction_id)) {
                return Err(error("SEQUENCE_INTERACTION_MISMATCH", Some(operation),
                    "CombinedFragment covered Lifeline belongs to a different Interaction"));
            }
            let id = FragmentId::new();
            repository.interactions.get_mut(&interaction_id).unwrap().fragments.push(
                CombinedFragment { id, operator: *operator, covered_lifelines: covered, operands: Vec::new() });
            repository.external_ids.insert(key, BehaviorSemanticId::Fragment(id));
        }
        SequenceBuildOperation::UpdateFragment { fragment, operator, covered_lifelines } => {
            let id = resolve_fragment(repository, namespace, fragment, operation)?;
            let interaction_id = interaction_for_fragment(repository, id).ok_or_else(|| error(
                "UNRESOLVED_SEQUENCE_REFERENCE", Some(operation), format!("CombinedFragment {id} was not found")))?;
            let covered = covered_lifelines.as_ref().map(|values| values.iter().map(|reference| resolve_lifeline(
                repository, namespace, reference, operation)).collect::<Result<Vec<_>, _>>()).transpose()?;
            let record = repository.interactions.get_mut(&interaction_id).unwrap().fragments
                .iter_mut().find(|record| record.id == id).unwrap();
            if let Some(value) = operator { record.operator = *value; }
            if let Some(value) = covered { record.covered_lifelines = value; }
        }
        SequenceBuildOperation::CreateOperand { external_id, fragment, guard, start_order, end_order } => {
            let key = external_key(namespace, external_id);
            behavior_external_available(&key, project, repository, operation)?;
            let fragment_id = resolve_fragment(repository, namespace, fragment, operation)?;
            let interaction_id = interaction_for_fragment(repository, fragment_id).ok_or_else(|| error(
                "UNRESOLVED_SEQUENCE_REFERENCE", Some(operation), format!("CombinedFragment {fragment_id} was not found")))?;
            let id = OperandId::new();
            repository.interactions.get_mut(&interaction_id).unwrap().fragments.iter_mut()
                .find(|record| record.id == fragment_id).unwrap().operands.push(InteractionOperand {
                    id, guard: guard.clone(), start_order: *start_order, end_order: *end_order,
                });
            repository.external_ids.insert(key, BehaviorSemanticId::Operand(id));
        }
        SequenceBuildOperation::UpdateOperand { operand, guard, start_order, end_order } => {
            let id = resolve_operand(repository, namespace, operand, operation)?;
            let fragment_id = fragment_for_operand(repository, id).ok_or_else(|| error(
                "UNRESOLVED_SEQUENCE_REFERENCE", Some(operation), format!("InteractionOperand {id} was not found")))?;
            let interaction_id = interaction_for_fragment(repository, fragment_id).unwrap();
            let record = repository.interactions.get_mut(&interaction_id).unwrap().fragments.iter_mut()
                .find(|record| record.id == fragment_id).unwrap().operands.iter_mut()
                .find(|record| record.id == id).unwrap();
            if let Some(value) = guard { record.guard = value.clone(); }
            if let Some(value) = start_order { record.start_order = *value; }
            if let Some(value) = end_order { record.end_order = *value; }
        }
        SequenceBuildOperation::CreateInvariant { external_id, interaction, lifeline, order, constraint } => {
            let key = external_key(namespace, external_id);
            behavior_external_available(&key, project, repository, operation)?;
            let interaction_id = resolve_interaction(repository, namespace, interaction, operation)?;
            let lifeline_id = resolve_lifeline(repository, namespace, lifeline, operation)?;
            let id = InvariantId::new();
            repository.interactions.get_mut(&interaction_id).unwrap().state_invariants.push(
                StateInvariant { id, lifeline_id, order: *order, constraint: constraint.clone() });
            repository.external_ids.insert(key, BehaviorSemanticId::Invariant(id));
        }
        SequenceBuildOperation::UpdateInvariant { invariant, lifeline, order, constraint } => {
            let id = resolve_invariant(repository, namespace, invariant, operation)?;
            let interaction_id = interaction_for_invariant(repository, id).ok_or_else(|| error(
                "UNRESOLVED_SEQUENCE_REFERENCE", Some(operation), format!("StateInvariant {id} was not found")))?;
            let lifeline_id = lifeline.as_ref().map(|reference| resolve_lifeline(
                repository, namespace, reference, operation)).transpose()?;
            let record = repository.interactions.get_mut(&interaction_id).unwrap().state_invariants
                .iter_mut().find(|record| record.id == id).unwrap();
            if let Some(value) = lifeline_id { record.lifeline_id = value; }
            if let Some(value) = order { record.order = *value; }
            if let Some(value) = constraint { record.constraint = value.clone(); }
        }
    }
    Ok(())
}

fn resolve_binding_endpoint(
    build: &BindingEndpointBuild,
    project: &Project,
    planned: &HashMap<String, ElementId>,
    namespace: &str,
    operation: usize,
) -> Result<BindingEndpoint, BuildDiagnostic> {
    Ok(BindingEndpoint {
        role_id: resolve_element_reference(project, planned, namespace, &build.role, operation)?,
        parameter_id: build.parameter.as_ref().map(|reference| resolve_element_reference(
            project, planned, namespace, reference, operation)).transpose()?,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_parametric_operation(
    build: &ParametricBuildOperation,
    project: &mut Project,
    planned_elements: &HashMap<String, ElementId>,
    planned_relationships: &mut HashMap<String, RelationshipId>,
    namespace: &str,
    operation: usize,
) -> Result<(), BuildDiagnostic> {
    match build {
        ParametricBuildOperation::UpdateElementSemantics {
            element, constraint_expression, quantity_kind_external_id, unit_external_id,
            quantity_dimension, unit_symbol, unit_scale_to_base,
        } => {
            let id = resolve_element_reference(project, planned_elements, namespace, element, operation)?;
            let record = project.element_mut(id).map_err(|cause| error(
                "PARAMETRIC_REFERENCE_UNRESOLVED", Some(operation), cause.to_string()))?;
            if let Some(value) = constraint_expression { record.constraint_expression = value.clone(); }
            if let Some(value) = quantity_kind_external_id { record.quantity_kind_external_id = value.clone(); }
            if let Some(value) = unit_external_id { record.unit_external_id = value.clone(); }
            if let Some(value) = quantity_dimension { record.quantity_dimension = value.clone(); }
            if let Some(value) = unit_symbol { record.unit_symbol = value.clone(); }
            if let Some(value) = unit_scale_to_base { record.unit_scale_to_base = *value; }
        }
        ParametricBuildOperation::CreateBinding { external_id, name, owner, source, target } => {
            let key = external_key(namespace, external_id);
            if project.elements.values().any(|record| record.external_id == key)
                || project.relationships.values().any(|record| record.external_id == key)
            {
                return Err(error("DUPLICATE_EXTERNAL_ID", Some(operation),
                    format!("external ID already exists in Project: {key}")));
            }
            let owner_id = resolve_element_reference(project, planned_elements, namespace, owner, operation)?;
            let source = resolve_binding_endpoint(source, project, planned_elements, namespace, operation)?;
            let target = resolve_binding_endpoint(target, project, planned_elements, namespace, operation)?;
            let id = project.create_binding_connector(owner_id, source, target).map_err(|cause| error(
                "PARAMETRIC_SEMANTIC_VALIDATION", Some(operation), cause.to_string()))?;
            let relationship = project.relationships.get_mut(&id).unwrap();
            relationship.external_id = key.clone();
            relationship.name = name.clone();
            planned_relationships.insert(key, id);
        }
        ParametricBuildOperation::UpdateBinding { relationship, name, owner, source, target } => {
            let id = resolve_relationship_reference(project, planned_relationships, namespace, relationship, operation)?;
            let owner_id = owner.as_ref().map(|reference| resolve_element_reference(
                project, planned_elements, namespace, reference, operation)).transpose()?;
            let current = project.relationship(id).map_err(|cause| error(
                "PARAMETRIC_REFERENCE_UNRESOLVED", Some(operation), cause.to_string()))?
                .binding.clone().ok_or_else(|| error("PARAMETRIC_BINDING_REQUIRED", Some(operation),
                    "relationship does not contain native BindingConnector endpoints"))?;
            let source = source.as_ref().map(|value| resolve_binding_endpoint(
                value, project, planned_elements, namespace, operation)).transpose()?.unwrap_or(current.source);
            let target = target.as_ref().map(|value| resolve_binding_endpoint(
                value, project, planned_elements, namespace, operation)).transpose()?.unwrap_or(current.target);
            let record = project.relationships.get_mut(&id).unwrap();
            if let Some(value) = name { record.name = value.clone(); }
            if let Some(value) = owner_id { record.owner_id = Some(value); }
            record.source_id = source.role_id;
            record.target_id = target.role_id;
            record.binding = Some(BindingConnector { source, target });
        }
    }
    Ok(())
}
