#![allow(clippy::too_many_arguments)]

use super::*;
use crate::workspace::bulk_model::{
    BindingEndpointBuild, InteractionReference, LifelineReference, MessageSignatureBuild,
    ParametricBuildOperation, SequenceBuildOperation,
};
use systems_modeler_core::behavior::{
    BehaviorRepository, BehaviorSemanticId, ExecutionId, FragmentId, InteractionOperator,
    InvariantId, MessageId, MessageSignature, MessageSort, Occurrence, OccurrenceId, OperandId,
};

pub(super) fn is_pr49_semantic_row(values: &BTreeMap<SpreadsheetSemanticProperty, String>) -> bool {
    let Some(value) = non_empty_value(values, SpreadsheetSemanticProperty::BehaviorKind) else {
        return false;
    };
    matches!(
        normalized(value).as_str(),
        "interaction"
            | "lifeline"
            | "occurrence"
            | "message"
            | "executionspecification"
            | "execution"
            | "combinedfragment"
            | "interactionoperand"
            | "operand"
            | "stateinvariant"
            | "invariant"
            | "parametricelement"
            | "engineeringmetadata"
            | "bindingconnector"
            | "binding"
    )
}

fn normalized(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn row_kind(value: &str) -> BehaviorRowKind {
    match normalized(value).as_str() {
        "interaction" => BehaviorRowKind::Interaction,
        "lifeline" => BehaviorRowKind::Lifeline,
        "occurrence" => BehaviorRowKind::Occurrence,
        "message" => BehaviorRowKind::Message,
        "executionspecification" | "execution" => BehaviorRowKind::ExecutionSpecification,
        "combinedfragment" => BehaviorRowKind::CombinedFragment,
        "interactionoperand" | "operand" => BehaviorRowKind::InteractionOperand,
        "stateinvariant" | "invariant" => BehaviorRowKind::StateInvariant,
        "parametricelement" | "engineeringmetadata" => BehaviorRowKind::ParametricElement,
        "bindingconnector" | "binding" => BehaviorRowKind::BindingConnector,
        _ => unreachable!("caller filters PR49 row kinds"),
    }
}

fn required_value<'a>(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &'a BTreeMap<SpreadsheetSemanticProperty, String>,
    property: SpreadsheetSemanticProperty,
    label: &str,
) -> Result<&'a str, SpreadsheetImportDiagnostic> {
    non_empty_value(values, property).ok_or_else(|| {
        diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, property),
            Some(property),
            non_empty_value(values, SpreadsheetSemanticProperty::ExternalId).map(ToOwned::to_owned),
            "PR49_FIELD_REQUIRED",
            format!("{label} is required for this PR49 semantic record"),
        )
    })
}

fn parse_u32(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    property: SpreadsheetSemanticProperty,
    label: &str,
) -> Result<u32, SpreadsheetImportDiagnostic> {
    let value = required_value(map, row, values, property, label)?;
    value.parse::<u32>().map_err(|_| {
        diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, property),
            Some(property),
            Some(value.into()),
            "PR49_NUMBER_INVALID",
            format!("{label} must be a non-negative integer"),
        )
    })
}

fn parse_optional_f64(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    property: SpreadsheetSemanticProperty,
    label: &str,
) -> Result<Option<f64>, SpreadsheetImportDiagnostic> {
    let Some(value) = non_empty_value(values, property) else {
        return Ok(None);
    };
    value
        .parse::<f64>()
        .ok()
        .filter(|parsed| parsed.is_finite())
        .map(Some)
        .ok_or_else(|| {
            diagnostic(
                Some(map),
                Some(row),
                mapped_column_name(map, property),
                Some(property),
                Some(value.into()),
                "PR49_NUMBER_INVALID",
                format!("{label} must be a finite decimal number"),
            )
        })
}

fn list(value: Option<&str>) -> Vec<String> {
    value
        .into_iter()
        .flat_map(|value| value.split([';', '\n']))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn element_reference(
    map: &SpreadsheetImportMap,
    row: usize,
    project: &Project,
    planned: &[PlannedElement],
    value: &str,
    property: SpreadsheetSemanticProperty,
    label: &str,
) -> Result<(ElementReference, ElementKind), SpreadsheetImportDiagnostic> {
    let key = external_key(&map.source_namespace, value);
    let planned_matches = planned
        .iter()
        .filter(|record| {
            record.external_id == value
                || record.qualified_name == value
                || record.qualified_name.ends_with(&format!("::{value}"))
        })
        .collect::<Vec<_>>();
    if let [record] = planned_matches.as_slice() {
        return Ok((
            BuildReference::External(record.external_id.clone()),
            record.kind.clone(),
        ));
    }
    if planned_matches.len() > 1 {
        return Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, property),
            Some(property),
            Some(value.into()),
            "PR49_REFERENCE_AMBIGUOUS",
            format!("{label} '{value}' matches multiple plan-local elements"),
        ));
    }
    let matches = project
        .elements
        .values()
        .filter(|record| {
            record.external_id == key
                || record.external_id == value
                || record.name == value
                || project
                    .qualified_name(record.id)
                    .is_ok_and(|qualified| qualified == value)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [record] => Ok((BuildReference::Existing(record.id), record.kind.clone())),
        [] => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, property),
            Some(property),
            Some(value.into()),
            "PR49_REFERENCE_UNRESOLVED",
            format!("{label} '{value}' could not be resolved in existing or plan-local semantics"),
        )),
        _ => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, property),
            Some(property),
            Some(value.into()),
            "PR49_REFERENCE_AMBIGUOUS",
            format!("{label} '{value}' is ambiguous; use External ID or qualified name"),
        )),
    }
}

fn interaction_reference(
    map: &SpreadsheetImportMap,
    row: usize,
    behavior: &BehaviorRepository,
    planned: &BehaviorPlanningIndex,
    value: &str,
) -> Result<InteractionReference, SpreadsheetImportDiagnostic> {
    let key = external_key(&map.source_namespace, value);
    if let Some(record) = behavior
        .interactions
        .values()
        .find(|record| record.external_id == key)
    {
        return Ok(BuildReference::Existing(record.id));
    }
    if let Some(record) = planned.by_external(value) {
        if record.kind == BehaviorRowKind::Interaction {
            return Ok(BuildReference::External(value.into()));
        }
        return Err(wrong_kind(
            map,
            row,
            value,
            BehaviorRowKind::Interaction,
            record.kind,
        ));
    }
    let matches = behavior
        .interactions
        .values()
        .filter(|record| record.name == value)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [record] => Ok(BuildReference::Existing(record.id)),
        [] => Err(reference_error(map, row, value, "Interaction", false)),
        _ => Err(reference_error(map, row, value, "Interaction", true)),
    }
}

fn wrong_kind(
    map: &SpreadsheetImportMap,
    row: usize,
    value: &str,
    expected: BehaviorRowKind,
    actual: BehaviorRowKind,
) -> SpreadsheetImportDiagnostic {
    diagnostic(
        Some(map),
        Some(row),
        None,
        None,
        Some(value.into()),
        "PR49_IDENTITY_KIND_COLLISION",
        format!("semantic reference '{value}' is {actual:?}, expected {expected:?}"),
    )
}

fn reference_error(
    map: &SpreadsheetImportMap,
    row: usize,
    value: &str,
    label: &str,
    ambiguous: bool,
) -> SpreadsheetImportDiagnostic {
    diagnostic(
        Some(map),
        Some(row),
        None,
        None,
        Some(value.into()),
        if ambiguous {
            "PR49_REFERENCE_AMBIGUOUS"
        } else {
            "PR49_REFERENCE_UNRESOLVED"
        },
        if ambiguous {
            format!("{label} '{value}' is ambiguous; use its External ID")
        } else {
            format!("{label} '{value}' was not found in existing or plan-local semantics")
        },
    )
}

fn semantic_identity_kind(identity: BehaviorSemanticId) -> Option<BehaviorRowKind> {
    Some(match identity {
        BehaviorSemanticId::Lifeline(_) => BehaviorRowKind::Lifeline,
        BehaviorSemanticId::Occurrence(_) => BehaviorRowKind::Occurrence,
        BehaviorSemanticId::Message(_) => BehaviorRowKind::Message,
        BehaviorSemanticId::Execution(_) => BehaviorRowKind::ExecutionSpecification,
        BehaviorSemanticId::Fragment(_) => BehaviorRowKind::CombinedFragment,
        BehaviorSemanticId::Operand(_) => BehaviorRowKind::InteractionOperand,
        BehaviorSemanticId::Invariant(_) => BehaviorRowKind::StateInvariant,
        BehaviorSemanticId::Region(_)
        | BehaviorSemanticId::Vertex(_)
        | BehaviorSemanticId::Transition(_) => return None,
    })
}

fn semantic_reference<T: Copy>(
    map: &SpreadsheetImportMap,
    row: usize,
    behavior: &BehaviorRepository,
    planned: &BehaviorPlanningIndex,
    value: &str,
    expected_kind: BehaviorRowKind,
    extract: fn(BehaviorSemanticId) -> Option<T>,
    by_name: impl Fn(&BehaviorRepository, &str) -> Vec<T>,
    label: &str,
) -> Result<BuildReference<T>, SpreadsheetImportDiagnostic> {
    let key = external_key(&map.source_namespace, value);
    if let Some(identity) = behavior.external_ids.get(&key).copied() {
        if let Some(id) = extract(identity) {
            return Ok(BuildReference::Existing(id));
        }
        return Err(diagnostic(
            Some(map),
            Some(row),
            None,
            None,
            Some(value.into()),
            "PR49_IDENTITY_KIND_COLLISION",
            format!(
                "semantic External ID '{value}' has kind {:?}, expected {expected_kind:?}",
                semantic_identity_kind(identity)
            ),
        ));
    }
    if let Some(record) = planned.by_external(value) {
        if record.kind == expected_kind {
            return Ok(BuildReference::External(value.into()));
        }
        return Err(wrong_kind(map, row, value, expected_kind, record.kind));
    }
    let matches = by_name(behavior, value);
    match matches.as_slice() {
        [id] => Ok(BuildReference::Existing(*id)),
        [] => Err(reference_error(map, row, value, label, false)),
        _ => Err(reference_error(map, row, value, label, true)),
    }
}

fn lifeline_reference(
    map: &SpreadsheetImportMap,
    row: usize,
    behavior: &BehaviorRepository,
    planned: &BehaviorPlanningIndex,
    value: &str,
) -> Result<LifelineReference, SpreadsheetImportDiagnostic> {
    semantic_reference(
        map,
        row,
        behavior,
        planned,
        value,
        BehaviorRowKind::Lifeline,
        |identity| match identity {
            BehaviorSemanticId::Lifeline(id) => Some(id),
            _ => None,
        },
        |repository, name| {
            repository
                .interactions
                .values()
                .flat_map(|interaction| interaction.lifelines.iter())
                .filter(|record| record.name == name)
                .map(|record| record.id)
                .collect()
        },
        "Lifeline",
    )
}

macro_rules! semantic_reference_fn {
    ($name:ident, $target:ty, $kind:ident, $variant:ident, $label:literal, $body:expr) => {
        #[allow(dead_code)]
        fn $name(
            map: &SpreadsheetImportMap,
            row: usize,
            behavior: &BehaviorRepository,
            planned: &BehaviorPlanningIndex,
            value: &str,
        ) -> Result<BuildReference<$target>, SpreadsheetImportDiagnostic> {
            semantic_reference(
                map,
                row,
                behavior,
                planned,
                value,
                BehaviorRowKind::$kind,
                |identity| match identity {
                    BehaviorSemanticId::$variant(id) => Some(id),
                    _ => None,
                },
                $body,
                $label,
            )
        }
    };
}

semantic_reference_fn!(
    occurrence_reference,
    OccurrenceId,
    Occurrence,
    Occurrence,
    "Occurrence",
    |repository: &BehaviorRepository, _name: &str| repository
        .interactions
        .values()
        .flat_map(|interaction| interaction
            .messages
            .iter()
            .flat_map(|message| [message.send_event.as_ref(), message.receive_event.as_ref()])
            .flatten()
            .chain(
                interaction
                    .executions
                    .iter()
                    .flat_map(|execution| [&execution.start, &execution.finish])
            ))
        .map(|record| record.id)
        .collect()
);
semantic_reference_fn!(
    message_reference,
    MessageId,
    Message,
    Message,
    "Message",
    |repository: &BehaviorRepository, name: &str| repository
        .interactions
        .values()
        .flat_map(|interaction| interaction.messages.iter())
        .filter(|record| record.name == name)
        .map(|record| record.id)
        .collect()
);
semantic_reference_fn!(
    execution_reference,
    ExecutionId,
    ExecutionSpecification,
    Execution,
    "ExecutionSpecification",
    |repository: &BehaviorRepository, _name: &str| repository
        .interactions
        .values()
        .flat_map(|interaction| interaction.executions.iter())
        .map(|record| record.id)
        .collect()
);
semantic_reference_fn!(
    fragment_reference,
    FragmentId,
    CombinedFragment,
    Fragment,
    "CombinedFragment",
    |repository: &BehaviorRepository, _name: &str| repository
        .interactions
        .values()
        .flat_map(|interaction| interaction.fragments.iter())
        .map(|record| record.id)
        .collect()
);
semantic_reference_fn!(
    operand_reference,
    OperandId,
    InteractionOperand,
    Operand,
    "InteractionOperand",
    |repository: &BehaviorRepository, _name: &str| repository
        .interactions
        .values()
        .flat_map(|interaction| interaction.fragments.iter())
        .flat_map(|fragment| fragment.operands.iter())
        .map(|record| record.id)
        .collect()
);
semantic_reference_fn!(
    invariant_reference,
    InvariantId,
    StateInvariant,
    Invariant,
    "StateInvariant",
    |repository: &BehaviorRepository, _name: &str| repository
        .interactions
        .values()
        .flat_map(|interaction| interaction.state_invariants.iter())
        .map(|record| record.id)
        .collect()
);

fn parse_message_sort(
    map: &SpreadsheetImportMap,
    row: usize,
    value: &str,
) -> Result<MessageSort, SpreadsheetImportDiagnostic> {
    match normalized(value).as_str() {
        "synchcall" | "synchronouscall" => Ok(MessageSort::SynchCall),
        "asynchcall" | "asynchronouscall" => Ok(MessageSort::AsynchCall),
        "asynchsignal" | "asynchronoussignal" => Ok(MessageSort::AsynchSignal),
        "reply" => Ok(MessageSort::Reply),
        "create" => Ok(MessageSort::Create),
        "delete" => Ok(MessageSort::Delete),
        "lost" => Ok(MessageSort::Lost),
        "found" => Ok(MessageSort::Found),
        _ => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::MessageSort),
            Some(SpreadsheetSemanticProperty::MessageSort),
            Some(value.into()),
            "MESSAGE_SORT_INVALID",
            "Message Sort must be SynchCall, AsynchCall, AsynchSignal, Reply, Create, Delete, Lost, or Found",
        )),
    }
}

fn parse_operator(
    map: &SpreadsheetImportMap,
    row: usize,
    value: &str,
) -> Result<InteractionOperator, SpreadsheetImportDiagnostic> {
    match normalized(value).as_str() {
        "alt" => Ok(InteractionOperator::Alt),
        "opt" => Ok(InteractionOperator::Opt),
        "loop" => Ok(InteractionOperator::Loop),
        "break" => Ok(InteractionOperator::Break),
        "par" => Ok(InteractionOperator::Par),
        "critical" => Ok(InteractionOperator::Critical),
        "neg" => Ok(InteractionOperator::Neg),
        "assert" => Ok(InteractionOperator::Assert),
        "strict" => Ok(InteractionOperator::Strict),
        "seq" => Ok(InteractionOperator::Seq),
        "ignore" => Ok(InteractionOperator::Ignore),
        "consider" => Ok(InteractionOperator::Consider),
        _ => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::Operator),
            Some(SpreadsheetSemanticProperty::Operator),
            Some(value.into()),
            "INTERACTION_OPERATOR_INVALID",
            "CombinedFragment Operator is not supported by the native model",
        )),
    }
}

fn represented_path(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    project: &Project,
    planned_project: &[PlannedElement],
) -> Result<Vec<ElementReference>, SpreadsheetImportDiagnostic> {
    let value = required_value(
        map,
        row,
        values,
        SpreadsheetSemanticProperty::RepresentedPath,
        "Represented Path",
    )?;
    let parts = if value.contains('>') || value.contains('/') || value.contains('\n') {
        value.split(['>', '/', '\n']).collect::<Vec<_>>()
    } else {
        value.split('.').collect::<Vec<_>>()
    };
    parts
        .into_iter()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            element_reference(
                map,
                row,
                project,
                planned_project,
                part,
                SpreadsheetSemanticProperty::RepresentedPath,
                "represented-path segment",
            )
            .map(|value| value.0)
        })
        .collect()
}

fn occurrence_by_id(behavior: &BehaviorRepository, id: OccurrenceId) -> Option<Occurrence> {
    behavior
        .interactions
        .values()
        .flat_map(|interaction| {
            interaction
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
        })
        .find(|record| record.id == id)
        .cloned()
}

fn build_signature(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    project: &Project,
    planned_project: &[PlannedElement],
    sort: MessageSort,
) -> Result<Option<MessageSignatureBuild>, SpreadsheetImportDiagnostic> {
    let Some(value) = non_empty_value(values, SpreadsheetSemanticProperty::Signature) else {
        return Ok(None);
    };
    let (reference, kind) = element_reference(
        map,
        row,
        project,
        planned_project,
        value,
        SpreadsheetSemanticProperty::Signature,
        "Message Signature",
    )?;
    match sort {
        MessageSort::SynchCall | MessageSort::AsynchCall if kind == ElementKind::Operation => {
            Ok(Some(MessageSignatureBuild::Operation(reference)))
        }
        MessageSort::AsynchSignal if kind == ElementKind::Signal => {
            Ok(Some(MessageSignatureBuild::Signal(reference)))
        }
        MessageSort::SynchCall | MessageSort::AsynchCall => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::Signature),
            Some(SpreadsheetSemanticProperty::Signature),
            Some(value.into()),
            "MESSAGE_SIGNATURE_KIND_INVALID",
            "SynchCall/AsynchCall requires an Operation signature",
        )),
        MessageSort::AsynchSignal => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::Signature),
            Some(SpreadsheetSemanticProperty::Signature),
            Some(value.into()),
            "MESSAGE_SIGNATURE_KIND_INVALID",
            "AsynchSignal requires a Signal signature",
        )),
        _ if kind == ElementKind::Operation => {
            Ok(Some(MessageSignatureBuild::Operation(reference)))
        }
        _ if kind == ElementKind::Signal => Ok(Some(MessageSignatureBuild::Signal(reference))),
        _ => Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::Signature),
            Some(SpreadsheetSemanticProperty::Signature),
            Some(value.into()),
            "MESSAGE_SIGNATURE_KIND_INVALID",
            "Message Signature must resolve to an Operation or Signal",
        )),
    }
}

fn signature_matches(
    existing: &Option<MessageSignature>,
    build: &Option<MessageSignatureBuild>,
) -> bool {
    match (existing, build) {
        (None, None) => true,
        (
            Some(MessageSignature::Operation(left)),
            Some(MessageSignatureBuild::Operation(BuildReference::Existing(right))),
        ) => left == right,
        (
            Some(MessageSignature::Signal(left)),
            Some(MessageSignatureBuild::Signal(BuildReference::Existing(right))),
        ) => left == right,
        _ => false,
    }
}

fn binding_endpoint(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    project: &Project,
    planned_project: &[PlannedElement],
    role_property: SpreadsheetSemanticProperty,
    parameter_property: SpreadsheetSemanticProperty,
    label: &str,
) -> Result<BindingEndpointBuild, SpreadsheetImportDiagnostic> {
    let role = required_value(map, row, values, role_property, label)?;
    let (role, _) = element_reference(
        map,
        row,
        project,
        planned_project,
        role,
        role_property,
        label,
    )?;
    let parameter = non_empty_value(values, parameter_property)
        .map(|value| {
            element_reference(
                map,
                row,
                project,
                planned_project,
                value,
                parameter_property,
                "Binding ConstraintParameter",
            )
            .map(|value| value.0)
        })
        .transpose()?;
    Ok(BindingEndpointBuild { role, parameter })
}

fn plan_parametric_element(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    project: &Project,
    planned_project: &[PlannedElement],
) -> Result<(SpreadsheetRowAction, Vec<ModelBuildOperation>), SpreadsheetImportDiagnostic> {
    let target = required_value(
        map,
        row,
        values,
        SpreadsheetSemanticProperty::Target,
        "Parametric Target",
    )?;
    let (element, _) = element_reference(
        map,
        row,
        project,
        planned_project,
        target,
        SpreadsheetSemanticProperty::Target,
        "Parametric Target",
    )?;
    let expression = values
        .get(&SpreadsheetSemanticProperty::ConstraintExpression)
        .cloned();
    let quantity_kind = values
        .get(&SpreadsheetSemanticProperty::QuantityKind)
        .map(|value| {
            (!value.trim().is_empty()).then(|| external_key(&map.source_namespace, value.trim()))
        });
    let unit = values.get(&SpreadsheetSemanticProperty::Unit).map(|value| {
        (!value.trim().is_empty()).then(|| external_key(&map.source_namespace, value.trim()))
    });
    let dimension = values
        .get(&SpreadsheetSemanticProperty::QuantityDimension)
        .map(|value| (!value.trim().is_empty()).then(|| value.trim().to_owned()));
    let symbol = values
        .get(&SpreadsheetSemanticProperty::UnitSymbol)
        .map(|value| (!value.trim().is_empty()).then(|| value.trim().to_owned()));
    let scale = parse_optional_f64(
        map,
        row,
        values,
        SpreadsheetSemanticProperty::UnitScaleToBase,
        "Unit Scale To Base",
    )?;
    let changed = match element {
        BuildReference::External(_) => true,
        BuildReference::Existing(id) => {
            let record = project.element(id).unwrap();
            expression
                .as_ref()
                .is_some_and(|value| record.constraint_expression != *value)
                || quantity_kind
                    .as_ref()
                    .is_some_and(|value| record.quantity_kind_external_id != *value)
                || unit
                    .as_ref()
                    .is_some_and(|value| record.unit_external_id != *value)
                || dimension
                    .as_ref()
                    .is_some_and(|value| record.quantity_dimension != *value)
                || symbol
                    .as_ref()
                    .is_some_and(|value| record.unit_symbol != *value)
                || scale.is_some_and(|value| record.unit_scale_to_base != value)
        }
    };
    let operations = changed
        .then(|| ModelBuildOperation::Parametric {
            operation: ParametricBuildOperation::UpdateElementSemantics {
                element,
                constraint_expression: expression,
                quantity_kind_external_id: quantity_kind,
                unit_external_id: unit,
                quantity_dimension: dimension,
                unit_symbol: symbol,
                unit_scale_to_base: scale,
            },
        })
        .into_iter()
        .collect();
    Ok((
        if changed {
            SpreadsheetRowAction::Update
        } else {
            SpreadsheetRowAction::NoChange
        },
        operations,
    ))
}

#[allow(clippy::too_many_lines)]
pub(super) fn plan_pr49_semantic_row(
    map: &SpreadsheetImportMap,
    row: usize,
    values: &BTreeMap<SpreadsheetSemanticProperty, String>,
    project: &Project,
    behavior: &BehaviorRepository,
    planned_project: &[PlannedElement],
    planned: &BehaviorPlanningIndex,
    seen: &mut HashSet<String>,
) -> Result<BehaviorRowPlan, SpreadsheetImportDiagnostic> {
    let kind_value = required_value(
        map,
        row,
        values,
        SpreadsheetSemanticProperty::BehaviorKind,
        "Behavior Kind",
    )?;
    let kind = row_kind(kind_value);
    let external = required_value(
        map,
        row,
        values,
        SpreadsheetSemanticProperty::ExternalId,
        "External ID",
    )?
    .to_owned();
    let key = external_key(&map.source_namespace, &external);
    if !seen.insert(key.clone()) {
        return Err(diagnostic(
            Some(map),
            Some(row),
            mapped_column_name(map, SpreadsheetSemanticProperty::ExternalId),
            Some(SpreadsheetSemanticProperty::ExternalId),
            Some(external),
            "DUPLICATE_SOURCE_EXTERNAL_ID",
            format!("source external ID '{key}' appears more than once in this import group"),
        ));
    }
    if let Some(record) = planned.by_external(&external) {
        return Err(wrong_kind(map, row, &external, kind, record.kind));
    }

    if kind == BehaviorRowKind::ParametricElement {
        let (action, operations) =
            plan_parametric_element(map, row, values, project, planned_project)?;
        return Ok(BehaviorRowPlan {
            action,
            operations,
            planned: Some(PlannedBehaviorRecord {
                external_id: external,
                kind,
                name: None,
            }),
        });
    }

    if kind == BehaviorRowKind::BindingConnector {
        let owner_value = non_empty_value(values, SpreadsheetSemanticProperty::Context)
            .or_else(|| non_empty_value(values, SpreadsheetSemanticProperty::Owner))
            .ok_or_else(|| reference_error(map, row, "", "Binding Context", false))?;
        let (owner, _) = element_reference(
            map,
            row,
            project,
            planned_project,
            owner_value,
            SpreadsheetSemanticProperty::Context,
            "Binding Context",
        )?;
        let source = binding_endpoint(
            map,
            row,
            values,
            project,
            planned_project,
            SpreadsheetSemanticProperty::BindingSourceRole,
            SpreadsheetSemanticProperty::BindingSourceParameter,
            "Binding Source Role",
        )?;
        let target = binding_endpoint(
            map,
            row,
            values,
            project,
            planned_project,
            SpreadsheetSemanticProperty::BindingTargetRole,
            SpreadsheetSemanticProperty::BindingTargetParameter,
            "Binding Target Role",
        )?;
        let name = values
            .get(&SpreadsheetSemanticProperty::Name)
            .cloned()
            .unwrap_or_default();
        let existing = project
            .relationships
            .values()
            .find(|record| record.external_id == key);
        let (action, operation) = if let Some(record) = existing {
            if record.kind != RelationshipKind::BindingConnector {
                return Err(diagnostic(
                    Some(map),
                    Some(row),
                    None,
                    None,
                    Some(external),
                    "PR49_IDENTITY_KIND_COLLISION",
                    "External ID resolves to a non-BindingConnector relationship",
                ));
            }
            let matches_endpoint =
                |native: &systems_modeler_core::BindingEndpoint, build: &BindingEndpointBuild| {
                    matches!(build.role, BuildReference::Existing(id) if id == native.role_id)
                        && match (&build.parameter, native.parameter_id) {
                            (None, None) => true,
                            (Some(BuildReference::Existing(left)), Some(right)) => *left == right,
                            _ => false,
                        }
                };
            let unchanged = matches!(owner, BuildReference::Existing(id) if record.owner_id == Some(id))
                && record.name == name
                && record.binding.as_ref().is_some_and(|binding| {
                    matches_endpoint(&binding.source, &source)
                        && matches_endpoint(&binding.target, &target)
                });
            if unchanged {
                (SpreadsheetRowAction::NoChange, None)
            } else {
                (
                    SpreadsheetRowAction::Update,
                    Some(ModelBuildOperation::Parametric {
                        operation: ParametricBuildOperation::UpdateBinding {
                            relationship: BuildReference::Existing(record.id),
                            name: Some(name),
                            owner: Some(owner),
                            source: Some(source),
                            target: Some(target),
                        },
                    }),
                )
            }
        } else {
            (
                SpreadsheetRowAction::Create,
                Some(ModelBuildOperation::Parametric {
                    operation: ParametricBuildOperation::CreateBinding {
                        external_id: external.clone(),
                        name,
                        owner,
                        source,
                        target,
                    },
                }),
            )
        };
        return Ok(BehaviorRowPlan {
            action,
            operations: operation.into_iter().collect(),
            planned: Some(PlannedBehaviorRecord {
                external_id: external,
                kind,
                name: None,
            }),
        });
    }

    let existing_identity = behavior.external_ids.get(&key).copied();
    let existing_interaction = behavior
        .interactions
        .values()
        .find(|record| record.external_id == key);
    if let Some(actual) = existing_identity.and_then(semantic_identity_kind)
        && actual != kind
    {
        return Err(wrong_kind(map, row, &external, kind, actual));
    }
    if existing_interaction.is_some() && kind != BehaviorRowKind::Interaction {
        return Err(wrong_kind(
            map,
            row,
            &external,
            kind,
            BehaviorRowKind::Interaction,
        ));
    }
    let name = values.get(&SpreadsheetSemanticProperty::Name).cloned();
    let (action, operations) = match kind {
        BehaviorRowKind::Interaction => {
            let name = required_value(
                map,
                row,
                values,
                SpreadsheetSemanticProperty::Name,
                "Interaction Name",
            )?
            .to_owned();
            let context_value = required_value(
                map,
                row,
                values,
                SpreadsheetSemanticProperty::Context,
                "Interaction Context",
            )?;
            let (context, _) = element_reference(
                map,
                row,
                project,
                planned_project,
                context_value,
                SpreadsheetSemanticProperty::Context,
                "Interaction Context",
            )?;
            if let Some(record) = existing_interaction {
                let changed = record.name != name
                    || !matches!(context, BuildReference::Existing(id) if id == record.context_id);
                let operation = changed.then(|| ModelBuildOperation::Sequence { operation: SequenceBuildOperation::UpdateInteraction {
                    interaction: BuildReference::Existing(record.id), name: (record.name != name).then_some(name),
                    context: (!matches!(context, BuildReference::Existing(id) if id == record.context_id)).then_some(context),
                }});
                (
                    if changed {
                        SpreadsheetRowAction::Update
                    } else {
                        SpreadsheetRowAction::NoChange
                    },
                    operation.into_iter().collect(),
                )
            } else {
                (
                    SpreadsheetRowAction::Create,
                    vec![ModelBuildOperation::Sequence {
                        operation: SequenceBuildOperation::CreateInteraction {
                            external_id: external.clone(),
                            name,
                            context,
                        },
                    }],
                )
            }
        }
        BehaviorRowKind::Lifeline => {
            let interaction_value = required_value(
                map,
                row,
                values,
                SpreadsheetSemanticProperty::Interaction,
                "Interaction",
            )?;
            let interaction =
                interaction_reference(map, row, behavior, planned, interaction_value)?;
            let name = required_value(
                map,
                row,
                values,
                SpreadsheetSemanticProperty::Name,
                "Lifeline Name",
            )?
            .to_owned();
            let path = represented_path(map, row, values, project, planned_project)?;
            if let Some(BehaviorSemanticId::Lifeline(id)) = existing_identity {
                let record = behavior
                    .interactions
                    .values()
                    .flat_map(|interaction| interaction.lifelines.iter())
                    .find(|record| record.id == id)
                    .ok_or_else(|| reference_error(map, row, &external, "Lifeline", false))?;
                let existing_path = path
                    .iter()
                    .map(|reference| match reference {
                        BuildReference::Existing(id) => Some(*id),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>();
                let changed =
                    record.name != name || existing_path.as_ref() != Some(&record.represented_path);
                (
                    if changed {
                        SpreadsheetRowAction::Update
                    } else {
                        SpreadsheetRowAction::NoChange
                    },
                    changed
                        .then(|| ModelBuildOperation::Sequence {
                            operation: SequenceBuildOperation::UpdateLifeline {
                                lifeline: BuildReference::Existing(id),
                                name: (record.name != name).then_some(name),
                                represented_path: (existing_path.as_ref()
                                    != Some(&record.represented_path))
                                .then_some(path),
                            },
                        })
                        .into_iter()
                        .collect(),
                )
            } else {
                (
                    SpreadsheetRowAction::Create,
                    vec![ModelBuildOperation::Sequence {
                        operation: SequenceBuildOperation::CreateLifeline {
                            external_id: external.clone(),
                            interaction,
                            name,
                            represented_path: path,
                        },
                    }],
                )
            }
        }
        BehaviorRowKind::Occurrence => {
            let interaction = interaction_reference(
                map,
                row,
                behavior,
                planned,
                required_value(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Interaction,
                    "Interaction",
                )?,
            )?;
            let lifeline = lifeline_reference(
                map,
                row,
                behavior,
                planned,
                required_value(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Lifeline,
                    "Lifeline",
                )?,
            )?;
            let order = parse_u32(
                map,
                row,
                values,
                SpreadsheetSemanticProperty::Order,
                "Occurrence Order",
            )?;
            if let Some(BehaviorSemanticId::Occurrence(id)) = existing_identity {
                let record = occurrence_by_id(behavior, id)
                    .ok_or_else(|| reference_error(map, row, &external, "Occurrence", false))?;
                let changed = record.order != order
                    || !matches!(lifeline, BuildReference::Existing(value) if value == record.lifeline_id);
                (if changed { SpreadsheetRowAction::Update } else { SpreadsheetRowAction::NoChange }, changed.then(|| ModelBuildOperation::Sequence {
                    operation: SequenceBuildOperation::UpdateOccurrence { occurrence: BuildReference::Existing(id),
                        lifeline: (!matches!(lifeline, BuildReference::Existing(value) if value == record.lifeline_id)).then_some(lifeline),
                        order: (record.order != order).then_some(order) }
                }).into_iter().collect())
            } else {
                (
                    SpreadsheetRowAction::Create,
                    vec![ModelBuildOperation::Sequence {
                        operation: SequenceBuildOperation::CreateOccurrence {
                            external_id: external.clone(),
                            interaction,
                            lifeline,
                            order,
                        },
                    }],
                )
            }
        }
        BehaviorRowKind::Message => {
            let interaction = interaction_reference(
                map,
                row,
                behavior,
                planned,
                required_value(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Interaction,
                    "Interaction",
                )?,
            )?;
            let name = required_value(
                map,
                row,
                values,
                SpreadsheetSemanticProperty::Name,
                "Message Name",
            )?
            .to_owned();
            let sort = parse_message_sort(
                map,
                row,
                required_value(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::MessageSort,
                    "Message Sort",
                )?,
            )?;
            let send = non_empty_value(values, SpreadsheetSemanticProperty::SendOccurrence)
                .map(|value| occurrence_reference(map, row, behavior, planned, value))
                .transpose()?;
            let receive = non_empty_value(values, SpreadsheetSemanticProperty::ReceiveOccurrence)
                .map(|value| occurrence_reference(map, row, behavior, planned, value))
                .transpose()?;
            let signature = build_signature(map, row, values, project, planned_project, sort)?;
            let arguments = list(
                values
                    .get(&SpreadsheetSemanticProperty::Arguments)
                    .map(String::as_str),
            );
            if let Some(BehaviorSemanticId::Message(id)) = existing_identity {
                let record = behavior
                    .interactions
                    .values()
                    .flat_map(|interaction| interaction.messages.iter())
                    .find(|record| record.id == id)
                    .ok_or_else(|| reference_error(map, row, &external, "Message", false))?;
                let occurrence_matches =
                    |native: &Option<Occurrence>, build: &Option<BuildReference<OccurrenceId>>| {
                        match (native, build) {
                            (None, None) => true,
                            (Some(left), Some(BuildReference::Existing(right))) => {
                                left.id == *right
                            }
                            _ => false,
                        }
                    };
                let changed = record.name != name
                    || record.sort != sort
                    || !occurrence_matches(&record.send_event, &send)
                    || !occurrence_matches(&record.receive_event, &receive)
                    || !signature_matches(&record.signature, &signature)
                    || record.arguments != arguments;
                (
                    if changed {
                        SpreadsheetRowAction::Update
                    } else {
                        SpreadsheetRowAction::NoChange
                    },
                    changed
                        .then(|| ModelBuildOperation::Sequence {
                            operation: SequenceBuildOperation::UpdateMessage {
                                message: BuildReference::Existing(id),
                                name: Some(name),
                                sort: Some(sort),
                                send: Some(send),
                                receive: Some(receive),
                                signature: Some(signature),
                                arguments: Some(arguments),
                            },
                        })
                        .into_iter()
                        .collect(),
                )
            } else {
                (
                    SpreadsheetRowAction::Create,
                    vec![ModelBuildOperation::Sequence {
                        operation: SequenceBuildOperation::CreateMessage {
                            external_id: external.clone(),
                            interaction,
                            name,
                            sort,
                            send,
                            receive,
                            signature,
                            arguments,
                        },
                    }],
                )
            }
        }
        BehaviorRowKind::ExecutionSpecification => {
            let interaction = interaction_reference(
                map,
                row,
                behavior,
                planned,
                required_value(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Interaction,
                    "Interaction",
                )?,
            )?;
            let lifeline = lifeline_reference(
                map,
                row,
                behavior,
                planned,
                required_value(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Lifeline,
                    "Lifeline",
                )?,
            )?;
            let start = occurrence_reference(
                map,
                row,
                behavior,
                planned,
                required_value(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::StartOccurrence,
                    "Start Occurrence",
                )?,
            )?;
            let finish = occurrence_reference(
                map,
                row,
                behavior,
                planned,
                required_value(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::FinishOccurrence,
                    "Finish Occurrence",
                )?,
            )?;
            let behavior_ref = non_empty_value(values, SpreadsheetSemanticProperty::Behavior)
                .map(|value| {
                    element_reference(
                        map,
                        row,
                        project,
                        planned_project,
                        value,
                        SpreadsheetSemanticProperty::Behavior,
                        "Execution Behavior",
                    )
                    .map(|value| value.0)
                })
                .transpose()?;
            if let Some(BehaviorSemanticId::Execution(id)) = existing_identity {
                let record = behavior
                    .interactions
                    .values()
                    .flat_map(|interaction| interaction.executions.iter())
                    .find(|record| record.id == id)
                    .ok_or_else(|| {
                        reference_error(map, row, &external, "ExecutionSpecification", false)
                    })?;
                let changed = !matches!(lifeline, BuildReference::Existing(value) if value == record.lifeline_id)
                    || !matches!(start, BuildReference::Existing(value) if value == record.start.id)
                    || !matches!(finish, BuildReference::Existing(value) if value == record.finish.id)
                    || match (&behavior_ref, record.behavior_id) {
                        (None, None) => false,
                        (Some(BuildReference::Existing(left)), Some(right)) => *left != right,
                        _ => true,
                    };
                (
                    if changed {
                        SpreadsheetRowAction::Update
                    } else {
                        SpreadsheetRowAction::NoChange
                    },
                    changed
                        .then(|| ModelBuildOperation::Sequence {
                            operation: SequenceBuildOperation::UpdateExecution {
                                execution: BuildReference::Existing(id),
                                lifeline: Some(lifeline),
                                start: Some(start),
                                finish: Some(finish),
                                behavior: Some(behavior_ref),
                            },
                        })
                        .into_iter()
                        .collect(),
                )
            } else {
                (
                    SpreadsheetRowAction::Create,
                    vec![ModelBuildOperation::Sequence {
                        operation: SequenceBuildOperation::CreateExecution {
                            external_id: external.clone(),
                            interaction,
                            lifeline,
                            start,
                            finish,
                            behavior: behavior_ref,
                        },
                    }],
                )
            }
        }
        BehaviorRowKind::CombinedFragment => {
            let interaction = interaction_reference(
                map,
                row,
                behavior,
                planned,
                required_value(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Interaction,
                    "Interaction",
                )?,
            )?;
            let operator = parse_operator(
                map,
                row,
                required_value(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Operator,
                    "CombinedFragment Operator",
                )?,
            )?;
            let covered = list(non_empty_value(
                values,
                SpreadsheetSemanticProperty::CoveredLifelines,
            ))
            .iter()
            .map(|value| lifeline_reference(map, row, behavior, planned, value))
            .collect::<Result<Vec<_>, _>>()?;
            if let Some(BehaviorSemanticId::Fragment(id)) = existing_identity {
                let record = behavior
                    .interactions
                    .values()
                    .flat_map(|interaction| interaction.fragments.iter())
                    .find(|record| record.id == id)
                    .ok_or_else(|| {
                        reference_error(map, row, &external, "CombinedFragment", false)
                    })?;
                let covered_ids = covered
                    .iter()
                    .map(|value| match value {
                        BuildReference::Existing(id) => Some(*id),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>();
                let changed = record.operator != operator
                    || covered_ids.as_ref() != Some(&record.covered_lifelines);
                (
                    if changed {
                        SpreadsheetRowAction::Update
                    } else {
                        SpreadsheetRowAction::NoChange
                    },
                    changed
                        .then(|| ModelBuildOperation::Sequence {
                            operation: SequenceBuildOperation::UpdateFragment {
                                fragment: BuildReference::Existing(id),
                                operator: Some(operator),
                                covered_lifelines: Some(covered),
                            },
                        })
                        .into_iter()
                        .collect(),
                )
            } else {
                (
                    SpreadsheetRowAction::Create,
                    vec![ModelBuildOperation::Sequence {
                        operation: SequenceBuildOperation::CreateFragment {
                            external_id: external.clone(),
                            interaction,
                            operator,
                            covered_lifelines: covered,
                        },
                    }],
                )
            }
        }
        BehaviorRowKind::InteractionOperand => {
            let fragment = fragment_reference(
                map,
                row,
                behavior,
                planned,
                required_value(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::CombinedFragment,
                    "CombinedFragment",
                )?,
            )?;
            let guard = values
                .get(&SpreadsheetSemanticProperty::Guard)
                .map(|value| (!value.trim().is_empty()).then(|| value.trim().to_owned()))
                .unwrap_or(None);
            let start_order = parse_u32(
                map,
                row,
                values,
                SpreadsheetSemanticProperty::StartOrder,
                "Operand Start Order",
            )?;
            let end_order = parse_u32(
                map,
                row,
                values,
                SpreadsheetSemanticProperty::EndOrder,
                "Operand End Order",
            )?;
            if let Some(BehaviorSemanticId::Operand(id)) = existing_identity {
                let record = behavior
                    .interactions
                    .values()
                    .flat_map(|interaction| interaction.fragments.iter())
                    .flat_map(|fragment| fragment.operands.iter())
                    .find(|record| record.id == id)
                    .ok_or_else(|| {
                        reference_error(map, row, &external, "InteractionOperand", false)
                    })?;
                let changed = record.guard != guard
                    || record.start_order != start_order
                    || record.end_order != end_order;
                (
                    if changed {
                        SpreadsheetRowAction::Update
                    } else {
                        SpreadsheetRowAction::NoChange
                    },
                    changed
                        .then(|| ModelBuildOperation::Sequence {
                            operation: SequenceBuildOperation::UpdateOperand {
                                operand: BuildReference::Existing(id),
                                guard: Some(guard),
                                start_order: Some(start_order),
                                end_order: Some(end_order),
                            },
                        })
                        .into_iter()
                        .collect(),
                )
            } else {
                (
                    SpreadsheetRowAction::Create,
                    vec![ModelBuildOperation::Sequence {
                        operation: SequenceBuildOperation::CreateOperand {
                            external_id: external.clone(),
                            fragment,
                            guard,
                            start_order,
                            end_order,
                        },
                    }],
                )
            }
        }
        BehaviorRowKind::StateInvariant => {
            let interaction = interaction_reference(
                map,
                row,
                behavior,
                planned,
                required_value(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Interaction,
                    "Interaction",
                )?,
            )?;
            let lifeline = lifeline_reference(
                map,
                row,
                behavior,
                planned,
                required_value(
                    map,
                    row,
                    values,
                    SpreadsheetSemanticProperty::Lifeline,
                    "Lifeline",
                )?,
            )?;
            let order = parse_u32(
                map,
                row,
                values,
                SpreadsheetSemanticProperty::Order,
                "StateInvariant Order",
            )?;
            let constraint = required_value(
                map,
                row,
                values,
                SpreadsheetSemanticProperty::Constraint,
                "StateInvariant Constraint",
            )?
            .to_owned();
            if let Some(BehaviorSemanticId::Invariant(id)) = existing_identity {
                let record = behavior
                    .interactions
                    .values()
                    .flat_map(|interaction| interaction.state_invariants.iter())
                    .find(|record| record.id == id)
                    .ok_or_else(|| reference_error(map, row, &external, "StateInvariant", false))?;
                let changed = record.order != order
                    || record.constraint != constraint
                    || !matches!(lifeline, BuildReference::Existing(value) if value == record.lifeline_id);
                (
                    if changed {
                        SpreadsheetRowAction::Update
                    } else {
                        SpreadsheetRowAction::NoChange
                    },
                    changed
                        .then(|| ModelBuildOperation::Sequence {
                            operation: SequenceBuildOperation::UpdateInvariant {
                                invariant: BuildReference::Existing(id),
                                lifeline: Some(lifeline),
                                order: Some(order),
                                constraint: Some(constraint),
                            },
                        })
                        .into_iter()
                        .collect(),
                )
            } else {
                (
                    SpreadsheetRowAction::Create,
                    vec![ModelBuildOperation::Sequence {
                        operation: SequenceBuildOperation::CreateInvariant {
                            external_id: external.clone(),
                            interaction,
                            lifeline,
                            order,
                            constraint,
                        },
                    }],
                )
            }
        }
        BehaviorRowKind::ParametricElement | BehaviorRowKind::BindingConnector => unreachable!(),
        _ => unreachable!("PR48 kinds are handled by the PR48 planner"),
    };
    Ok(BehaviorRowPlan {
        action,
        operations,
        planned: Some(PlannedBehaviorRecord {
            external_id: external,
            kind,
            name,
        }),
    })
}
