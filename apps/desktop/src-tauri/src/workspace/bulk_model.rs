use super::*;
use std::collections::{HashMap, HashSet};
use systems_modeler_core::{
    Connector, ConnectorEnd, ConnectorKind, DiagramFamilyId, FlowDirection, ItemFlow,
    ParameterDirection, supported_diagram_families,
};

mod pr48_behavior;
pub use pr48_behavior::*;
mod pr49_semantics;
pub use pr49_semantics::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildReference<T> {
    External(String),
    Existing(T),
}

pub type ElementReference = BuildReference<ElementId>;
pub type RelationshipReference = BuildReference<RelationshipId>;
pub type DiagramReference = BuildReference<DiagramId>;

#[derive(Debug, Clone, Default)]
pub struct AssociationEndBuildFields {
    pub role_name: Option<String>,
    pub multiplicity: Option<Multiplicity>,
    pub navigable: Option<bool>,
    pub aggregation: Option<AggregationKind>,
}

/// Unresolved CATIA-style connector end carried through the PR36 plan. Each
/// segment is resolved in the classifier reached by the preceding segment,
/// after earlier plan-local element operations have populated the candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectorEndBuildSpec {
    pub segments: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum ModelBuildOperation {
    CreateElement {
        external_id: String,
        kind: ElementKind,
        name: String,
        owner: ElementReference,
        type_ref: Option<ElementReference>,
    },
    UpdateElement {
        element: ElementReference,
        name: String,
    },
    /// Spreadsheet/interchange scalar and owned-feature update path.
    /// Mutation stays inside the PR36 candidate build so preview/apply remain atomic.
    UpdateElementFields {
        element: ElementReference,
        name: Option<String>,
        owner: Option<ElementReference>,
        type_ref: Option<ElementReference>,
        external_id: Option<String>,
        documentation: Option<String>,
        visibility: Option<VisibilityKind>,
        requirement_id: Option<String>,
        requirement_text: Option<String>,
        multiplicity: Option<Multiplicity>,
        default_value: Option<String>,
        parameter_direction: Option<ParameterDirection>,
        flow_direction: Option<FlowDirection>,
        is_conjugated: Option<bool>,
        extension_points: Option<Vec<String>>,
    },
    CreateRelationship {
        external_id: String,
        kind: RelationshipKind,
        source: ElementReference,
        target: ElementReference,
        owner: Option<ElementReference>,
    },
    CreateConnector {
        external_id: String,
        context: ElementReference,
        kind: ConnectorKind,
        source: ConnectorEndBuildSpec,
        target: ConnectorEndBuildSpec,
        name: String,
        documentation: String,
        visibility: VisibilityKind,
    },
    UpdateConnectorFields {
        relationship: RelationshipReference,
        context: ElementReference,
        kind: ConnectorKind,
        source: ConnectorEndBuildSpec,
        target: ConnectorEndBuildSpec,
        external_id: Option<String>,
        name: Option<String>,
        documentation: Option<String>,
        visibility: Option<VisibilityKind>,
    },
    CreateItemFlow {
        external_id: String,
        connector: RelationshipReference,
        source: ConnectorEndBuildSpec,
        target: ConnectorEndBuildSpec,
        conveyed_items: Vec<ElementReference>,
        name: String,
        documentation: String,
        visibility: VisibilityKind,
    },
    UpdateItemFlowFields {
        relationship: RelationshipReference,
        connector: RelationshipReference,
        source: ConnectorEndBuildSpec,
        target: ConnectorEndBuildSpec,
        conveyed_items: Vec<ElementReference>,
        external_id: Option<String>,
        name: Option<String>,
        documentation: Option<String>,
        visibility: Option<VisibilityKind>,
    },
    /// PR40 mapped relationship update path. All endpoint/owner resolution and
    /// mutation stays in the PR36 candidate so preview/apply remain atomic.
    Activity {
        operation: ActivityBuildOperation,
    },
    StateMachine {
        operation: StateMachineBuildOperation,
    },
    Sequence {
        operation: SequenceBuildOperation,
    },
    Parametric {
        operation: ParametricBuildOperation,
    },
    UpdateRelationshipFields {
        relationship: RelationshipReference,
        name: Option<String>,
        owner: Option<ElementReference>,
        source: Option<ElementReference>,
        target: Option<ElementReference>,
        external_id: Option<String>,
        documentation: Option<String>,
        visibility: Option<VisibilityKind>,
        source_end: Option<AssociationEndBuildFields>,
        target_end: Option<AssociationEndBuildFields>,
        alias: Option<Option<String>>,
        extension_condition: Option<Option<String>>,
        extension_location: Option<Option<String>>,
    },
    CreateDiagram {
        external_id: String,
        family: String,
        name: String,
        owner: ElementReference,
        semantic_context: Option<ElementReference>,
    },
    PresentElement {
        diagram: DiagramReference,
        element: ElementReference,
        x: f64,
        y: f64,
    },
    PresentRelationship {
        diagram: DiagramReference,
        relationship: RelationshipReference,
    },
    RestorePortableState {
        state: Box<super::portable_interchange::PortableAuthoredStateV1>,
    },
}

#[derive(Debug, Clone)]
pub struct ModelBuildPlan {
    pub source_namespace: String,
    pub operations: Vec<ModelBuildOperation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildDiagnostic {
    pub severity: BuildDiagnosticSeverity,
    pub code: &'static str,
    pub operation: Option<usize>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposedBuildOperation {
    pub operation: usize,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelBuildPreview {
    pub proposed_operations: Vec<ProposedBuildOperation>,
    pub diagnostics: Vec<BuildDiagnostic>,
}

impl ModelBuildPreview {
    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|item| item.severity == BuildDiagnosticSeverity::Error)
    }
}

#[derive(Debug)]
pub struct ModelBuildResult {
    pub element_ids: HashMap<String, ElementId>,
    pub relationship_ids: HashMap<String, RelationshipId>,
    pub diagram_ids: HashMap<String, DiagramId>,
}

struct CandidateBuild {
    project: Project,
    diagrams: Vec<BddDiagram>,
    result: ModelBuildResult,
}

pub(super) fn external_key(namespace: &str, external_id: &str) -> String {
    format!("{namespace}::{external_id}")
}

fn operation_description(operation: &ModelBuildOperation) -> String {
    match operation {
        ModelBuildOperation::CreateElement {
            external_id, name, ..
        } => format!("CREATE element {external_id} ({name})"),
        ModelBuildOperation::UpdateElement { name, .. } => {
            format!("UPDATE element name to {name}")
        }
        ModelBuildOperation::UpdateElementFields { .. } => "UPDATE mapped element fields".into(),
        ModelBuildOperation::CreateRelationship { external_id, .. } => {
            format!("CREATE relationship {external_id}")
        }
        ModelBuildOperation::CreateConnector { external_id, .. } => {
            format!("CREATE connector {external_id}")
        }
        ModelBuildOperation::UpdateConnectorFields { .. } => {
            "UPDATE mapped connector fields".into()
        }
        ModelBuildOperation::CreateItemFlow { external_id, .. } => {
            format!("CREATE item flow {external_id}")
        }
        ModelBuildOperation::UpdateItemFlowFields { .. } => "UPDATE mapped item flow fields".into(),
        ModelBuildOperation::Activity { operation } => behavior_operation_description(operation),
        ModelBuildOperation::StateMachine { operation } => {
            state_machine_operation_description(operation)
        }
        ModelBuildOperation::Sequence { operation } => sequence_operation_description(operation),
        ModelBuildOperation::Parametric { operation } => {
            parametric_operation_description(operation)
        }
        ModelBuildOperation::UpdateRelationshipFields { .. } => {
            "UPDATE mapped relationship fields".into()
        }
        ModelBuildOperation::CreateDiagram {
            external_id, name, ..
        } => format!("CREATE diagram {external_id} ({name})"),
        ModelBuildOperation::PresentElement { .. } => "PRESENT element".into(),
        ModelBuildOperation::PresentRelationship { .. } => "PRESENT relationship".into(),
        ModelBuildOperation::RestorePortableState { .. } => {
            "RESTORE portable authored project state".into()
        }
    }
}

fn error(
    code: &'static str,
    operation: Option<usize>,
    message: impl Into<String>,
) -> BuildDiagnostic {
    BuildDiagnostic {
        severity: BuildDiagnosticSeverity::Error,
        code,
        operation,
        message: message.into(),
    }
}

fn resolve_element(
    project: &Project,
    planned: &HashMap<String, ElementId>,
    namespace: &str,
    reference: &ElementReference,
    operation: usize,
) -> Result<ElementId, BuildDiagnostic> {
    match reference {
        BuildReference::Existing(id) => project
            .element(*id)
            .map(|_| *id)
            .map_err(|cause| error("UNRESOLVED_REFERENCE", Some(operation), cause.to_string())),
        BuildReference::External(external_id) => {
            let key = external_key(namespace, external_id);
            if let Some(id) = planned.get(&key) {
                return Ok(*id);
            }
            let matches: Vec<_> = project
                .elements
                .values()
                .filter(|element| element.external_id == key)
                .map(|element| element.id)
                .collect();
            match matches.as_slice() {
                [id] => Ok(*id),
                [] => Err(error(
                    "UNRESOLVED_REFERENCE",
                    Some(operation),
                    format!(
                        "external ID \"{external_id}\" was not found in namespace \"{namespace}\""
                    ),
                )),
                _ => Err(error(
                    "AMBIGUOUS_REFERENCE",
                    Some(operation),
                    format!(
                        "external ID \"{external_id}\" is ambiguous in namespace \"{namespace}\""
                    ),
                )),
            }
        }
    }
}

fn resolve_relationship(
    project: &Project,
    planned: &HashMap<String, RelationshipId>,
    namespace: &str,
    reference: &RelationshipReference,
    operation: usize,
) -> Result<RelationshipId, BuildDiagnostic> {
    match reference {
        BuildReference::Existing(id) => project
            .relationship(*id)
            .map(|_| *id)
            .map_err(|cause| error("UNRESOLVED_REFERENCE", Some(operation), cause.to_string())),
        BuildReference::External(external_id) => {
            let key = external_key(namespace, external_id);
            if let Some(id) = planned.get(&key) {
                return Ok(*id);
            }
            let matches: Vec<_> = project
                .relationships
                .values()
                .filter(|relationship| relationship.external_id == key)
                .map(|relationship| relationship.id)
                .collect();
            match matches.as_slice() {
                [id] => Ok(*id),
                [] => Err(error(
                    "UNRESOLVED_REFERENCE",
                    Some(operation),
                    format!(
                        "relationship external ID \"{external_id}\" was not found in namespace \"{namespace}\""
                    ),
                )),
                _ => Err(error(
                    "AMBIGUOUS_REFERENCE",
                    Some(operation),
                    format!(
                        "relationship external ID \"{external_id}\" is ambiguous in namespace \"{namespace}\""
                    ),
                )),
            }
        }
    }
}

fn resolve_diagram(
    diagrams: &[BddDiagram],
    planned: &HashMap<String, DiagramId>,
    namespace: &str,
    reference: &DiagramReference,
    operation: usize,
) -> Result<DiagramId, BuildDiagnostic> {
    match reference {
        BuildReference::Existing(id) => diagrams
            .iter()
            .any(|diagram| diagram.id == id.to_string())
            .then_some(*id)
            .ok_or_else(|| {
                error(
                    "UNRESOLVED_REFERENCE",
                    Some(operation),
                    format!("diagram not found: {id}"),
                )
            }),
        BuildReference::External(external_id) => planned
            .get(&external_key(namespace, external_id))
            .copied()
            .ok_or_else(|| {
                error(
                    "UNRESOLVED_REFERENCE",
                    Some(operation),
                    format!(
                        "diagram external ID \"{external_id}\" was not found in namespace \"{namespace}\""
                    ),
                )
            }),
    }
}

fn resolve_connector_segment(
    project: &Project,
    namespace: &str,
    classifier_id: ElementId,
    requested: &str,
    operation: usize,
) -> Result<ElementId, BuildDiagnostic> {
    let external = external_key(namespace, requested);
    let by_external = project
        .elements
        .values()
        .filter(|element| element.owner_id == Some(classifier_id))
        .filter(|element| element.external_id == external)
        .map(|element| element.id)
        .collect::<Vec<_>>();
    match by_external.as_slice() {
        [id] => return Ok(*id),
        [] => {}
        _ => {
            return Err(error(
                "CONNECTOR_PATH_AMBIGUOUS",
                Some(operation),
                format!("connector path segment \"{requested}\" is ambiguous by external identity"),
            ));
        }
    }

    let by_exact_name = project
        .elements
        .values()
        .filter(|element| element.owner_id == Some(classifier_id))
        .filter(|element| {
            element.name == requested
                || project
                    .qualified_name(element.id)
                    .is_ok_and(|qualified| qualified == requested)
        })
        .map(|element| element.id)
        .collect::<Vec<_>>();
    match by_exact_name.as_slice() {
        [id] => Ok(*id),
        [] => Err(error(
            "CONNECTOR_PATH_UNRESOLVED",
            Some(operation),
            format!(
                "connector path segment \"{requested}\" was not found in {}",
                project
                    .qualified_name(classifier_id)
                    .unwrap_or_else(|_| classifier_id.to_string())
            ),
        )),
        _ => Err(error(
            "CONNECTOR_PATH_AMBIGUOUS",
            Some(operation),
            format!(
                "connector path segment \"{requested}\" resolves to more than one feature in {}",
                project
                    .qualified_name(classifier_id)
                    .unwrap_or_else(|_| classifier_id.to_string())
            ),
        )),
    }
}

pub(super) fn resolve_connector_end(
    project: &Project,
    namespace: &str,
    context_id: ElementId,
    spec: &ConnectorEndBuildSpec,
    operation: usize,
) -> Result<ConnectorEnd, BuildDiagnostic> {
    if spec.segments.is_empty()
        || spec
            .segments
            .iter()
            .any(|segment| segment.trim().is_empty())
    {
        return Err(error(
            "CONNECTOR_PATH_INVALID",
            Some(operation),
            "connector end must contain one or more non-empty path segments",
        ));
    }

    let mut classifier_id = context_id;
    let mut property_path = Vec::new();
    for (index, segment) in spec.segments.iter().enumerate() {
        let id = resolve_connector_segment(
            project,
            namespace,
            classifier_id,
            segment.trim(),
            operation,
        )?;
        let element = project.element(id).map_err(|cause| {
            error(
                "CONNECTOR_PATH_UNRESOLVED",
                Some(operation),
                cause.to_string(),
            )
        })?;
        let terminal = index + 1 == spec.segments.len();

        if element.is_port() {
            if !terminal {
                return Err(error(
                    "CONNECTOR_PATH_INVALID",
                    Some(operation),
                    format!(
                        "port '{}' must be the terminal connector path segment",
                        element.name
                    ),
                ));
            }
            return Ok(if property_path.is_empty() {
                ConnectorEnd::boundary(id)
            } else {
                ConnectorEnd::nested_port(property_path, id)
            });
        }

        if !matches!(
            element.kind,
            ElementKind::PartProperty | ElementKind::ReferenceProperty
        ) {
            return Err(error(
                "CONNECTOR_PATH_INVALID",
                Some(operation),
                format!(
                    "connector path segment '{}' is {:?}, not a structural property or port",
                    element.name, element.kind
                ),
            ));
        }
        property_path.push(id);
        if terminal {
            return Ok(ConnectorEnd {
                property_path,
                role_id: id,
                port_id: None,
            });
        }
        let type_id = element.type_id.ok_or_else(|| {
            error(
                "CONNECTOR_PATH_UNTYPED",
                Some(operation),
                format!("structural property '{}' has no type", element.name),
            )
        })?;
        let type_element = project.element(type_id).map_err(|cause| {
            error(
                "CONNECTOR_PATH_UNRESOLVED",
                Some(operation),
                cause.to_string(),
            )
        })?;
        if !matches!(
            type_element.kind,
            ElementKind::Block | ElementKind::AssociationBlock | ElementKind::InterfaceBlock
        ) {
            return Err(error(
                "CONNECTOR_PATH_INVALID",
                Some(operation),
                format!(
                    "structural property '{}' is typed by non-structured {:?}",
                    element.name, type_element.kind
                ),
            ));
        }
        classifier_id = type_id;
    }
    unreachable!("non-empty connector paths return from the loop")
}

fn preflight(plan: &ModelBuildPlan) -> Result<(), BuildDiagnostic> {
    if plan.source_namespace.trim().is_empty() {
        return Err(error(
            "SOURCE_NAMESPACE_REQUIRED",
            None,
            "source namespace is required",
        ));
    }
    let mut external_ids = HashSet::new();
    for (index, operation) in plan.operations.iter().enumerate() {
        let external_id = match operation {
            ModelBuildOperation::CreateElement { external_id, .. }
            | ModelBuildOperation::CreateRelationship { external_id, .. }
            | ModelBuildOperation::CreateConnector { external_id, .. }
            | ModelBuildOperation::CreateItemFlow { external_id, .. }
            | ModelBuildOperation::CreateDiagram { external_id, .. } => Some(external_id),
            ModelBuildOperation::Activity { operation } => activity_create_external_id(operation),
            ModelBuildOperation::StateMachine { operation } => {
                state_machine_create_external_id(operation)
            }
            ModelBuildOperation::Sequence { operation } => sequence_create_external_id(operation),
            ModelBuildOperation::Parametric { operation } => {
                parametric_create_external_id(operation)
            }
            ModelBuildOperation::RestorePortableState { .. } => None,
            _ => None,
        };
        if let Some(external_id) = external_id {
            if external_id.trim().is_empty() {
                return Err(error(
                    "EXTERNAL_ID_REQUIRED",
                    Some(index),
                    "external ID is required",
                ));
            }
            if !external_ids.insert(external_id) {
                return Err(error(
                    "DUPLICATE_EXTERNAL_ID",
                    Some(index),
                    format!(
                        "external ID \"{external_id}\" occurs more than once in namespace \"{}\"",
                        plan.source_namespace
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn build_candidate(
    plan: &ModelBuildPlan,
    mut project: Project,
    mut diagrams: Vec<BddDiagram>,
) -> Result<CandidateBuild, BuildDiagnostic> {
    preflight(plan)?;
    let namespace = plan.source_namespace.trim();
    let mut element_ids = HashMap::new();
    let mut relationship_ids = HashMap::new();
    let mut diagram_ids = HashMap::new();

    for (index, operation) in plan.operations.iter().enumerate() {
        let operation_result: Result<(), BuildDiagnostic> = (|| {
            match operation {
                ModelBuildOperation::CreateElement {
                    external_id,
                    kind,
                    name,
                    owner,
                    type_ref,
                } => {
                    let owner_id =
                        resolve_element(&project, &element_ids, namespace, owner, index)?;
                    let id = if let Some(type_ref) = type_ref {
                        let type_id =
                            resolve_element(&project, &element_ids, namespace, type_ref, index)?;
                        if *kind == ElementKind::Reception {
                            let id = project
                                .create_element(kind.clone(), name, owner_id)
                                .map_err(|cause| {
                                    error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                                })?;
                            project.set_element_type(id, type_id).map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?;
                            Ok::<ElementId, BuildDiagnostic>(id)
                        } else {
                            project
                                .create_typed_feature(
                                    kind.clone(),
                                    name,
                                    owner_id,
                                    type_id,
                                    Multiplicity::ONE,
                                )
                                .map_err(|cause| {
                                    error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                                })
                        }
                    } else {
                        project
                            .create_element(kind.clone(), name, owner_id)
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })
                    }?;
                    let key = external_key(namespace, external_id);
                    project.set_external_id(id, key.clone()).map_err(|cause| {
                        error("DUPLICATE_EXTERNAL_ID", Some(index), cause.to_string())
                    })?;
                    element_ids.insert(key, id);
                }
                ModelBuildOperation::UpdateElement { element, name } => {
                    let id = resolve_element(&project, &element_ids, namespace, element, index)?;
                    project.rename_element(id, name).map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
                }
                ModelBuildOperation::UpdateElementFields {
                    element,
                    name,
                    owner,
                    type_ref,
                    external_id,
                    documentation,
                    visibility,
                    requirement_id,
                    requirement_text,
                    multiplicity,
                    default_value,
                    parameter_direction,
                    flow_direction,
                    is_conjugated,
                    extension_points,
                } => {
                    let id = resolve_element(&project, &element_ids, namespace, element, index)?;
                    if let Some(owner) = owner {
                        let owner_id =
                            resolve_element(&project, &element_ids, namespace, owner, index)?;
                        project.move_element(id, owner_id).map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?;
                    }
                    if let Some(name) = name {
                        project.rename_element(id, name.clone()).map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?;
                    }
                    if let Some(type_ref) = type_ref {
                        let type_id =
                            resolve_element(&project, &element_ids, namespace, type_ref, index)?;
                        project.set_element_type(id, type_id).map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?;
                    }
                    if let Some(external_id) = external_id {
                        project
                            .set_external_id(id, external_key(namespace, external_id))
                            .map_err(|cause| {
                                error("DUPLICATE_EXTERNAL_ID", Some(index), cause.to_string())
                            })?;
                    }
                    if let Some(documentation) = documentation {
                        project
                            .element_mut(id)
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?
                            .documentation = documentation.clone();
                    }
                    if let Some(visibility) = visibility {
                        project
                            .element_mut(id)
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?
                            .visibility = *visibility;
                    }
                    if let Some(multiplicity) = multiplicity {
                        project
                            .set_multiplicity(id, *multiplicity)
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?;
                    }
                    if let Some(default_value) = default_value {
                        let kind = project
                            .element(id)
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?
                            .kind
                            .clone();
                        if !matches!(kind, ElementKind::ValueProperty | ElementKind::Parameter) {
                            return Err(error(
                                "SEMANTIC_VALIDATION",
                                Some(index),
                                "Default Value mapping is valid only for ValueProperty or Parameter",
                            ));
                        }
                        project
                            .element_mut(id)
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?
                            .default_value =
                            (!default_value.trim().is_empty()).then(|| default_value.clone());
                    }
                    if let Some(parameter_direction) = parameter_direction {
                        let kind = project
                            .element(id)
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?
                            .kind
                            .clone();
                        if kind != ElementKind::Parameter {
                            return Err(error(
                                "SEMANTIC_VALIDATION",
                                Some(index),
                                "Parameter Direction mapping is valid only for Parameter",
                            ));
                        }
                        project
                            .element_mut(id)
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?
                            .parameter_direction = Some(*parameter_direction);
                    }
                    if let Some(flow_direction) = flow_direction {
                        if project
                            .element(id)
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?
                            .kind
                            != ElementKind::FlowProperty
                        {
                            return Err(error(
                                "SEMANTIC_VALIDATION",
                                Some(index),
                                "Flow Direction mapping is valid only for FlowProperty",
                            ));
                        }
                        project
                            .element_mut(id)
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?
                            .flow_direction = Some(*flow_direction);
                    }
                    if let Some(is_conjugated) = is_conjugated {
                        let kind = project
                            .element(id)
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?
                            .kind
                            .clone();
                        if !matches!(kind, ElementKind::ProxyPort | ElementKind::FullPort) {
                            return Err(error(
                                "SEMANTIC_VALIDATION",
                                Some(index),
                                "Conjugated mapping is valid only for ProxyPort or FullPort",
                            ));
                        }
                        project
                            .element_mut(id)
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?
                            .is_conjugated = *is_conjugated;
                    }
                    if let Some(extension_points) = extension_points {
                        if project
                            .element(id)
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?
                            .kind
                            != ElementKind::UseCase
                        {
                            return Err(error(
                                "SEMANTIC_VALIDATION",
                                Some(index),
                                "Extension Points mapping is valid only for UseCase elements",
                            ));
                        }
                        project
                            .element_mut(id)
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?
                            .extension_points = extension_points.clone();
                    }
                    if requirement_id.is_some() || requirement_text.is_some() {
                        let current = project.element(id).map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?;
                        if current.kind != ElementKind::Requirement {
                            return Err(error(
                                "SEMANTIC_VALIDATION",
                                Some(index),
                                "Requirement ID/Text mappings are valid only for Requirement elements",
                            ));
                        }
                        let next_requirement_id = requirement_id
                            .clone()
                            .or_else(|| current.requirement_id.clone())
                            .ok_or_else(|| error(
                                "SEMANTIC_VALIDATION",
                                Some(index),
                                "Requirement ID is required when applying mapped Requirement fields",
                            ))?;
                        let next_requirement_text = requirement_text
                            .clone()
                            .or_else(|| current.requirement_text.clone())
                            .unwrap_or_default();
                        project
                            .update_requirement(id, next_requirement_id, next_requirement_text)
                            .map_err(|cause| {
                                error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                            })?;
                    }
                    project.validate_element(id).map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
                }
                ModelBuildOperation::CreateRelationship {
                    external_id,
                    kind,
                    source,
                    target,
                    owner,
                } => {
                    let source_id =
                        resolve_element(&project, &element_ids, namespace, source, index)?;
                    let target_id =
                        resolve_element(&project, &element_ids, namespace, target, index)?;
                    let owner_id = owner
                        .as_ref()
                        .map(|reference| {
                            resolve_element(&project, &element_ids, namespace, reference, index)
                        })
                        .transpose()?;
                    let id = if *kind == RelationshipKind::Association {
                        project.create_association(
                            owner_id,
                            vec![
                                Project::association_end(
                                    source_id,
                                    "",
                                    Multiplicity::ONE,
                                    true,
                                    AggregationKind::None,
                                ),
                                Project::association_end(
                                    target_id,
                                    "",
                                    Multiplicity::ONE,
                                    true,
                                    AggregationKind::None,
                                ),
                            ],
                        )
                    } else {
                        project.create_relationship(kind.clone(), source_id, target_id, owner_id)
                    }
                    .map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
                    let key = external_key(namespace, external_id);
                    if project
                        .elements
                        .values()
                        .any(|element| element.external_id == key)
                        || project.relationships.values().any(|relationship| {
                            relationship.id != id && relationship.external_id == key
                        })
                    {
                        return Err(error(
                            "DUPLICATE_EXTERNAL_ID",
                            Some(index),
                            format!("external ID already exists: {key}"),
                        ));
                    }
                    project.relationships.get_mut(&id).unwrap().external_id = key.clone();
                    relationship_ids.insert(key, id);
                }
                ModelBuildOperation::CreateConnector {
                    external_id,
                    context,
                    kind,
                    source,
                    target,
                    name,
                    documentation,
                    visibility,
                } => {
                    let context_id =
                        resolve_element(&project, &element_ids, namespace, context, index)?;
                    let source =
                        resolve_connector_end(&project, namespace, context_id, source, index)?;
                    let target =
                        resolve_connector_end(&project, namespace, context_id, target, index)?;
                    let id = project
                        .create_connector(Connector {
                            context_id,
                            kind: *kind,
                            source,
                            target,
                        })
                        .map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?;
                    let key = external_key(namespace, external_id);
                    if project
                        .elements
                        .values()
                        .any(|element| element.external_id == key)
                        || project.relationships.values().any(|relationship| {
                            relationship.id != id && relationship.external_id == key
                        })
                    {
                        return Err(error(
                            "DUPLICATE_EXTERNAL_ID",
                            Some(index),
                            format!("external ID already exists: {key}"),
                        ));
                    }
                    let relationship = project.relationships.get_mut(&id).unwrap();
                    relationship.external_id = key.clone();
                    relationship.name = name.clone();
                    relationship.documentation = documentation.clone();
                    relationship.visibility = *visibility;
                    relationship_ids.insert(key, id);
                }
                ModelBuildOperation::UpdateConnectorFields {
                    relationship,
                    context,
                    kind,
                    source,
                    target,
                    external_id,
                    name,
                    documentation,
                    visibility,
                } => {
                    let id = resolve_relationship(
                        &project,
                        &relationship_ids,
                        namespace,
                        relationship,
                        index,
                    )?;
                    let current = project.relationship(id).map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
                    if current.kind != RelationshipKind::Connector || current.connector.is_none() {
                        return Err(error(
                            "RELATIONSHIP_IDENTITY_KIND_MISMATCH",
                            Some(index),
                            "connector update target is not a native Connector",
                        ));
                    }
                    let context_id =
                        resolve_element(&project, &element_ids, namespace, context, index)?;
                    let source =
                        resolve_connector_end(&project, namespace, context_id, source, index)?;
                    let target =
                        resolve_connector_end(&project, namespace, context_id, target, index)?;
                    let next_connector = Connector {
                        context_id,
                        kind: *kind,
                        source,
                        target,
                    };

                    let mut validation_project = project.clone();
                    validation_project.relationships.remove(&id);
                    validation_project
                        .create_connector(next_connector.clone())
                        .map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?;

                    let next_external_id = external_id
                        .as_ref()
                        .map(|value| external_key(namespace, value));
                    if let Some(key) = &next_external_id
                        && (project
                            .elements
                            .values()
                            .any(|element| element.external_id == *key)
                            || project.relationships.values().any(|candidate| {
                                candidate.id != id && candidate.external_id == *key
                            }))
                    {
                        return Err(error(
                            "DUPLICATE_EXTERNAL_ID",
                            Some(index),
                            format!("external ID already exists: {key}"),
                        ));
                    }

                    let relationship = project.relationships.get_mut(&id).unwrap();
                    relationship.owner_id = Some(context_id);
                    relationship.source_id = next_connector
                        .source
                        .port_id
                        .unwrap_or(next_connector.source.role_id);
                    relationship.target_id = next_connector
                        .target
                        .port_id
                        .unwrap_or(next_connector.target.role_id);
                    relationship.connector = Some(next_connector);
                    if let Some(value) = next_external_id {
                        relationship.external_id = value;
                    }
                    if let Some(value) = name {
                        relationship.name = value.clone();
                    }
                    if let Some(value) = documentation {
                        relationship.documentation = value.clone();
                    }
                    if let Some(value) = visibility {
                        relationship.visibility = *value;
                    }
                    project.validate().map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
                }
                ModelBuildOperation::CreateItemFlow {
                    external_id,
                    connector,
                    source,
                    target,
                    conveyed_items,
                    name,
                    documentation,
                    visibility,
                } => {
                    let connector_id = resolve_relationship(
                        &project,
                        &relationship_ids,
                        namespace,
                        connector,
                        index,
                    )?;
                    let connector = project.relationship(connector_id).map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
                    let context_id = connector
                        .connector
                        .as_ref()
                        .filter(|_| connector.kind == RelationshipKind::Connector)
                        .map(|connector| connector.context_id)
                        .ok_or_else(|| {
                            error(
                                "RELATIONSHIP_IDENTITY_KIND_MISMATCH",
                                Some(index),
                                "ItemFlow Connector reference is not a native Connector",
                            )
                        })?;
                    let source =
                        resolve_connector_end(&project, namespace, context_id, source, index)?;
                    let target =
                        resolve_connector_end(&project, namespace, context_id, target, index)?;
                    let conveyed_item_ids = conveyed_items
                        .iter()
                        .map(|reference| {
                            resolve_element(&project, &element_ids, namespace, reference, index)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let id = project
                        .create_item_flow(ItemFlow {
                            connector_id,
                            source,
                            target,
                            conveyed_item_ids,
                        })
                        .map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?;
                    let key = external_key(namespace, external_id);
                    if project
                        .elements
                        .values()
                        .any(|element| element.external_id == key)
                        || project.relationships.values().any(|relationship| {
                            relationship.id != id && relationship.external_id == key
                        })
                    {
                        return Err(error(
                            "DUPLICATE_EXTERNAL_ID",
                            Some(index),
                            format!("external ID already exists: {key}"),
                        ));
                    }
                    let relationship = project.relationships.get_mut(&id).unwrap();
                    relationship.external_id = key.clone();
                    relationship.name = name.clone();
                    relationship.documentation = documentation.clone();
                    relationship.visibility = *visibility;
                    relationship_ids.insert(key, id);
                }
                ModelBuildOperation::UpdateItemFlowFields {
                    relationship,
                    connector,
                    source,
                    target,
                    conveyed_items,
                    external_id,
                    name,
                    documentation,
                    visibility,
                } => {
                    let id = resolve_relationship(
                        &project,
                        &relationship_ids,
                        namespace,
                        relationship,
                        index,
                    )?;
                    let current = project.relationship(id).map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
                    if current.kind != RelationshipKind::ItemFlow || current.item_flow.is_none() {
                        return Err(error(
                            "RELATIONSHIP_IDENTITY_KIND_MISMATCH",
                            Some(index),
                            "ItemFlow update target is not a native ItemFlow",
                        ));
                    }
                    let connector_id = resolve_relationship(
                        &project,
                        &relationship_ids,
                        namespace,
                        connector,
                        index,
                    )?;
                    let connector = project.relationship(connector_id).map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
                    let context_id = connector
                        .connector
                        .as_ref()
                        .filter(|_| connector.kind == RelationshipKind::Connector)
                        .map(|connector| connector.context_id)
                        .ok_or_else(|| {
                            error(
                                "RELATIONSHIP_IDENTITY_KIND_MISMATCH",
                                Some(index),
                                "ItemFlow Connector reference is not a native Connector",
                            )
                        })?;
                    let source =
                        resolve_connector_end(&project, namespace, context_id, source, index)?;
                    let target =
                        resolve_connector_end(&project, namespace, context_id, target, index)?;
                    let conveyed_item_ids = conveyed_items
                        .iter()
                        .map(|reference| {
                            resolve_element(&project, &element_ids, namespace, reference, index)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let next_flow = ItemFlow {
                        connector_id,
                        source,
                        target,
                        conveyed_item_ids,
                    };
                    let mut validation_project = project.clone();
                    validation_project.relationships.remove(&id);
                    validation_project
                        .create_item_flow(next_flow.clone())
                        .map_err(|cause| {
                            error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                        })?;

                    let next_external_id = external_id
                        .as_ref()
                        .map(|value| external_key(namespace, value));
                    if let Some(key) = &next_external_id
                        && (project
                            .elements
                            .values()
                            .any(|element| element.external_id == *key)
                            || project.relationships.values().any(|candidate| {
                                candidate.id != id && candidate.external_id == *key
                            }))
                    {
                        return Err(error(
                            "DUPLICATE_EXTERNAL_ID",
                            Some(index),
                            format!("external ID already exists: {key}"),
                        ));
                    }

                    let relationship = project.relationships.get_mut(&id).unwrap();
                    relationship.owner_id = Some(context_id);
                    relationship.source_id =
                        next_flow.source.port_id.unwrap_or(next_flow.source.role_id);
                    relationship.target_id =
                        next_flow.target.port_id.unwrap_or(next_flow.target.role_id);
                    relationship.item_flow = Some(next_flow);
                    if let Some(value) = next_external_id {
                        relationship.external_id = value;
                    }
                    if let Some(value) = name {
                        relationship.name = value.clone();
                    }
                    if let Some(value) = documentation {
                        relationship.documentation = value.clone();
                    }
                    if let Some(value) = visibility {
                        relationship.visibility = *value;
                    }
                    project.validate().map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
                }
                ModelBuildOperation::Activity { .. }
                | ModelBuildOperation::StateMachine { .. }
                | ModelBuildOperation::Sequence { .. }
                | ModelBuildOperation::Parametric { .. } => {
                    return Err(error(
                        "COMPLETE_BUILD_REQUIRED",
                        Some(index),
                        "specialized authored semantics require the unified candidate build path",
                    ));
                }
                ModelBuildOperation::UpdateRelationshipFields {
                    relationship,
                    name,
                    owner,
                    source,
                    target,
                    external_id,
                    documentation,
                    visibility,
                    source_end,
                    target_end,
                    alias,
                    extension_condition,
                    extension_location,
                } => {
                    let id = resolve_relationship(
                        &project,
                        &relationship_ids,
                        namespace,
                        relationship,
                        index,
                    )?;
                    let current = project.relationship(id).map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
                    let kind = current.kind.clone();
                    let next_source = source
                        .as_ref()
                        .map(|reference| {
                            resolve_element(&project, &element_ids, namespace, reference, index)
                        })
                        .transpose()?
                        .unwrap_or(current.source_id);
                    let next_target = target
                        .as_ref()
                        .map(|reference| {
                            resolve_element(&project, &element_ids, namespace, reference, index)
                        })
                        .transpose()?
                        .unwrap_or(current.target_id);
                    let next_owner = owner
                        .as_ref()
                        .map(|reference| {
                            resolve_element(&project, &element_ids, namespace, reference, index)
                        })
                        .transpose()?
                        .or(current.owner_id);

                    let mut next_association_ends = if kind == RelationshipKind::Association {
                        if current.association_ends.len() < 2 {
                            return Err(error(
                                "SEMANTIC_VALIDATION",
                                Some(index),
                                "Association update requires two existing semantic ends",
                            ));
                        }
                        let mut ends = current.association_ends.clone();
                        ends[0].classifier_id = next_source;
                        ends[1].classifier_id = next_target;
                        if let Some(fields) = source_end {
                            if let Some(value) = &fields.role_name {
                                ends[0].role_name = value.clone();
                            }
                            if let Some(value) = fields.multiplicity {
                                ends[0].multiplicity = value;
                            }
                            if let Some(value) = fields.navigable {
                                ends[0].navigable = value;
                            }
                            if let Some(value) = fields.aggregation {
                                ends[0].aggregation = value;
                            }
                        }
                        if let Some(fields) = target_end {
                            if let Some(value) = &fields.role_name {
                                ends[1].role_name = value.clone();
                            }
                            if let Some(value) = fields.multiplicity {
                                ends[1].multiplicity = value;
                            }
                            if let Some(value) = fields.navigable {
                                ends[1].navigable = value;
                            }
                            if let Some(value) = fields.aggregation {
                                ends[1].aggregation = value;
                            }
                        }
                        Some(ends)
                    } else {
                        if source_end.is_some() || target_end.is_some() {
                            return Err(error(
                                "SEMANTIC_VALIDATION",
                                Some(index),
                                "Association-end fields are valid only for Association",
                            ));
                        }
                        None
                    };

                    // Reuse the existing model-core creation validator on a candidate clone
                    // before reconnecting an existing relationship. This preserves existing
                    // Generalization cycle checks, Dependency endpoint checks, ownership
                    // rules, and Association classifier validation without a second importer
                    // validation model.
                    let mut validation_project = project.clone();
                    validation_project.relationships.remove(&id);
                    let replacement_id = if kind == RelationshipKind::Association {
                        validation_project.create_association(
                            next_owner,
                            next_association_ends.clone().unwrap_or_default(),
                        )
                    } else {
                        validation_project.create_relationship(
                            kind.clone(),
                            next_source,
                            next_target,
                            next_owner,
                        )
                    }
                    .map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
                    {
                        let candidate = validation_project
                            .relationships
                            .get_mut(&replacement_id)
                            .expect("replacement relationship exists");
                        if let Some(value) = alias {
                            candidate.alias = value.clone();
                        }
                        if let Some(value) = extension_condition {
                            candidate.extension_condition = value.clone();
                        }
                        if let Some(value) = extension_location {
                            candidate.extension_location = value.clone();
                        }
                    }
                    validation_project.validate().map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;

                    let next_external_id = external_id
                        .as_ref()
                        .map(|value| external_key(namespace, value));
                    if let Some(key) = &next_external_id
                        && (project
                            .elements
                            .values()
                            .any(|element| element.external_id == *key)
                            || project.relationships.values().any(|candidate| {
                                candidate.id != id && candidate.external_id == *key
                            }))
                    {
                        return Err(error(
                            "DUPLICATE_EXTERNAL_ID",
                            Some(index),
                            format!("external ID already exists: {key}"),
                        ));
                    }

                    let relationship = project.relationships.get_mut(&id).unwrap();
                    relationship.source_id = next_source;
                    relationship.target_id = next_target;
                    relationship.owner_id = next_owner;
                    if let Some(value) = name {
                        relationship.name = value.clone();
                    }
                    if let Some(value) = documentation {
                        relationship.documentation = value.clone();
                    }
                    if let Some(value) = visibility {
                        relationship.visibility = *value;
                    }
                    if let Some(value) = next_external_id {
                        relationship.external_id = value;
                    }
                    if let Some(ends) = next_association_ends.take() {
                        relationship.association_ends = ends;
                    }
                    if let Some(value) = alias {
                        relationship.alias = value.clone();
                    }
                    if let Some(value) = extension_condition {
                        relationship.extension_condition = value.clone();
                    }
                    if let Some(value) = extension_location {
                        relationship.extension_location = value.clone();
                    }
                    project.validate().map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
                }
                ModelBuildOperation::CreateDiagram {
                    external_id,
                    family,
                    name,
                    owner,
                    semantic_context,
                } => {
                    let family_id = DiagramFamilyId(family.clone());
                    let registry = supported_diagram_families();
                    let descriptor = registry.get(&family_id).ok_or_else(|| {
                        error(
                            "DIAGRAM_FAMILY_INVALID",
                            Some(index),
                            format!("unsupported diagram family: {family}"),
                        )
                    })?;
                    if family != "bdd" {
                        return Err(error(
                            "DIAGRAM_FAMILY_UNSUPPORTED",
                            Some(index),
                            "PR36 bulk presentation currently uses the existing BDD presentation API",
                        ));
                    }
                    let owner_id =
                        resolve_element(&project, &element_ids, namespace, owner, index)?;
                    let owner_kind = format!("{:?}", project.element(owner_id).unwrap().kind);
                    if !descriptor.permitted_owner_kinds.contains(&owner_kind) {
                        return Err(error(
                            "DIAGRAM_OWNER_INVALID",
                            Some(index),
                            format!("{family} cannot be owned by {owner_kind}"),
                        ));
                    }
                    let context_id = semantic_context
                        .as_ref()
                        .map(|reference| {
                            resolve_element(&project, &element_ids, namespace, reference, index)
                        })
                        .transpose()?;
                    let id = DiagramId::new();
                    diagrams.push(BddDiagram {
                        id: id.to_string(),
                        name: name.clone(),
                        owner_id: owner_id.to_string(),
                        family: family.clone(),
                        semantic_context_id: context_id.map(|value| value.to_string()),
                        subject_boundary: None,
                        nodes: Vec::new(),
                        edges: Vec::new(),
                    });
                    diagram_ids.insert(external_key(namespace, external_id), id);
                }
                ModelBuildOperation::PresentElement {
                    diagram,
                    element,
                    x,
                    y,
                } => {
                    if !x.is_finite() || !y.is_finite() {
                        return Err(error(
                            "PRESENTATION_GEOMETRY_INVALID",
                            Some(index),
                            "presentation coordinates must be finite",
                        ));
                    }
                    let diagram_id =
                        resolve_diagram(&diagrams, &diagram_ids, namespace, diagram, index)?;
                    let element_id =
                        resolve_element(&project, &element_ids, namespace, element, index)?;
                    let element = project.element(element_id).unwrap();
                    let diagram = diagrams
                        .iter_mut()
                        .find(|diagram| diagram.id == diagram_id.to_string())
                        .unwrap();
                    if diagram.family != "bdd" || element.kind != ElementKind::Block {
                        return Err(error(
                            "PRESENTATION_TARGET_INVALID",
                            Some(index),
                            format!(
                                "{:?} cannot be presented on {}",
                                element.kind, diagram.family
                            ),
                        ));
                    }
                    if diagram
                        .nodes
                        .iter()
                        .any(|node| node.element_id == element_id.to_string())
                    {
                        return Err(error(
                            "DUPLICATE_PRESENTATION",
                            Some(index),
                            "element is already presented on the diagram",
                        ));
                    }
                    diagram.nodes.push(DiagramNode {
                        id: uuid::Uuid::new_v4().to_string(),
                        element_id: element_id.to_string(),
                        x: *x,
                        y: *y,
                        width: 180.0,
                        height: 105.0,
                        actor_notation: None,
                        parameter_presentations: Vec::new(),
                    });
                }
                ModelBuildOperation::PresentRelationship {
                    diagram,
                    relationship,
                } => {
                    let diagram_id =
                        resolve_diagram(&diagrams, &diagram_ids, namespace, diagram, index)?;
                    let relationship_id = resolve_relationship(
                        &project,
                        &relationship_ids,
                        namespace,
                        relationship,
                        index,
                    )?;
                    let relationship = project.relationship(relationship_id).unwrap();
                    let diagram = diagrams
                        .iter_mut()
                        .find(|diagram| diagram.id == diagram_id.to_string())
                        .unwrap();
                    if diagram.family != "bdd" || relationship.kind != RelationshipKind::Association
                    {
                        return Err(error(
                            "PRESENTATION_TARGET_INVALID",
                            Some(index),
                            format!(
                                "{:?} cannot be presented on {}",
                                relationship.kind, diagram.family
                            ),
                        ));
                    }
                    if diagram
                        .edges
                        .iter()
                        .any(|edge| edge.relationship_id == relationship_id.to_string())
                    {
                        return Err(error(
                            "DUPLICATE_PRESENTATION",
                            Some(index),
                            "relationship is already presented on the diagram",
                        ));
                    }
                    let source_node = diagram
                        .nodes
                        .iter()
                        .find(|node| node.element_id == relationship.source_id.to_string())
                        .cloned()
                        .ok_or_else(|| {
                            error(
                                "UNRESOLVED_PRESENTATION_TARGET",
                                Some(index),
                                "relationship source is not presented on the diagram",
                            )
                        })?;
                    let target_node = diagram
                        .nodes
                        .iter()
                        .find(|node| node.element_id == relationship.target_id.to_string())
                        .cloned()
                        .ok_or_else(|| {
                            error(
                                "UNRESOLVED_PRESENTATION_TARGET",
                                Some(index),
                                "relationship target is not presented on the diagram",
                            )
                        })?;
                    let points = route_relationship(&source_node, &target_node, &diagram.nodes)
                        .map_err(|cause| error("PRESENTATION_ROUTING", Some(index), cause))?;
                    diagram.edges.push(DiagramEdge {
                        id: uuid::Uuid::new_v4().to_string(),
                        relationship_id: relationship_id.to_string(),
                        source_node_id: source_node.id,
                        target_node_id: target_node.id,
                        label_anchor: Some(routing::route_label_anchor(&points)),
                        points,
                    });
                }
                ModelBuildOperation::RestorePortableState { .. } => {
                    return Err(error(
                        "COMPLETE_BUILD_REQUIRED",
                        Some(index),
                        "portable authored state must use the complete PR36 build path",
                    ));
                }
            }
            Ok(())
        })();
        operation_result?;
    }

    project
        .validate()
        .map_err(|cause| error("SEMANTIC_VALIDATION", None, cause.to_string()))?;
    validate_loaded_diagrams(&project, &diagrams)
        .map_err(|cause| error("PRESENTATION_VALIDATION", None, cause))?;
    Ok(CandidateBuild {
        project,
        diagrams,
        result: ModelBuildResult {
            element_ids,
            relationship_ids,
            diagram_ids,
        },
    })
}

pub fn preview_model_build(plan: &ModelBuildPlan, state: &WorkspaceState) -> ModelBuildPreview {
    let proposed_operations = plan
        .operations
        .iter()
        .enumerate()
        .map(|(operation, item)| ProposedBuildOperation {
            operation,
            description: operation_description(item),
        })
        .collect();
    let candidate = state
        .project
        .lock()
        .map_err(|_| error("LOCK_FAILURE", None, "project lock poisoned"))
        .and_then(|guard| {
            guard
                .clone()
                .ok_or_else(|| error("NO_PROJECT", None, "no project open"))
        })
        .and_then(|project| {
            state
                .diagrams
                .lock()
                .map_err(|_| error("LOCK_FAILURE", None, "diagram lock poisoned"))
                .and_then(|diagrams| build_candidate(plan, project, diagrams.clone()))
        });
    ModelBuildPreview {
        proposed_operations,
        diagnostics: candidate.err().into_iter().collect(),
    }
}

pub fn apply_model_build(
    plan: &ModelBuildPlan,
    state: &WorkspaceState,
) -> Result<ModelBuildResult, ModelBuildPreview> {
    let proposed_operations = plan
        .operations
        .iter()
        .enumerate()
        .map(|(operation, item)| ProposedBuildOperation {
            operation,
            description: operation_description(item),
        })
        .collect::<Vec<_>>();
    let mut project_guard = state.project.lock().map_err(|_| ModelBuildPreview {
        proposed_operations: proposed_operations.clone(),
        diagnostics: vec![error("LOCK_FAILURE", None, "project lock poisoned")],
    })?;
    let mut diagram_guard = state.diagrams.lock().map_err(|_| ModelBuildPreview {
        proposed_operations: proposed_operations.clone(),
        diagnostics: vec![error("LOCK_FAILURE", None, "diagram lock poisoned")],
    })?;
    let project = project_guard.clone().ok_or_else(|| ModelBuildPreview {
        proposed_operations: proposed_operations.clone(),
        diagnostics: vec![error("NO_PROJECT", None, "no project open")],
    })?;
    let candidate =
        build_candidate(plan, project, diagram_guard.clone()).map_err(|diagnostic| {
            ModelBuildPreview {
                proposed_operations,
                diagnostics: vec![diagnostic],
            }
        })?;
    *project_guard = Some(candidate.project);
    *diagram_guard = candidate.diagrams;
    Ok(candidate.result)
}

fn portable_state_from_plan(
    plan: &ModelBuildPlan,
) -> Result<&super::portable_interchange::PortableAuthoredStateV1, BuildDiagnostic> {
    preflight(plan)?;
    match plan.operations.as_slice() {
        [ModelBuildOperation::RestorePortableState { state }] => Ok(state),
        _ => Err(error(
            "PORTABLE_PLAN_INVALID",
            None,
            "portable import requires exactly one authored-state restore operation",
        )),
    }
}

fn proposed_operations(plan: &ModelBuildPlan) -> Vec<ProposedBuildOperation> {
    plan.operations
        .iter()
        .enumerate()
        .map(|(operation, item)| ProposedBuildOperation {
            operation,
            description: operation_description(item),
        })
        .collect()
}

fn target_is_blank(
    project: &Option<Project>,
    diagrams: &[BddDiagram],
    ibd_diagrams: &[ibd::IbdDiagram],
    behavior: &systems_modeler_core::BehaviorRepository,
    behavior_diagrams: &[behavior_workspace::BehaviorDiagram],
    activities: &systems_modeler_core::ActivityRepository,
    activity_diagrams: &[activity_workspace::ActivityDiagram],
) -> bool {
    let semantic_blank = project.as_ref().is_none_or(|project| {
        project.elements.len() == 1
            && project.elements.contains_key(&project.root_id)
            && project.relationships.is_empty()
    });
    semantic_blank
        && diagrams.is_empty()
        && ibd_diagrams.is_empty()
        && behavior.state_machines.is_empty()
        && behavior.interactions.is_empty()
        && behavior_diagrams.is_empty()
        && activities.activities.is_empty()
        && activity_diagrams.is_empty()
}

pub fn preview_complete_model_build(
    plan: &ModelBuildPlan,
    state: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
) -> ModelBuildPreview {
    let proposed_operations = proposed_operations(plan);
    let diagnostic = (|| {
        let portable = portable_state_from_plan(plan)?;
        portable
            .validate(&plan.source_namespace)
            .map_err(|message| error("PORTABLE_VALIDATION", Some(0), message))?;
        let project = state
            .project
            .lock()
            .map_err(|_| error("LOCK_FAILURE", None, "project lock poisoned"))?;
        let diagrams = state
            .diagrams
            .lock()
            .map_err(|_| error("LOCK_FAILURE", None, "diagram lock poisoned"))?;
        let ibd_diagrams = state
            .ibd_diagrams
            .lock()
            .map_err(|_| error("LOCK_FAILURE", None, "IBD lock poisoned"))?;
        let behavior = state
            .behavior
            .lock()
            .map_err(|_| error("LOCK_FAILURE", None, "behavior lock poisoned"))?;
        let behavior_diagrams = state
            .behavior_diagrams
            .lock()
            .map_err(|_| error("LOCK_FAILURE", None, "behavior diagram lock poisoned"))?;
        let activities = activity
            .repository
            .lock()
            .map_err(|_| error("LOCK_FAILURE", None, "Activity repository lock poisoned"))?;
        let activity_diagrams = activity
            .diagrams
            .lock()
            .map_err(|_| error("LOCK_FAILURE", None, "Activity diagram lock poisoned"))?;
        if !target_is_blank(
            &project,
            &diagrams,
            &ibd_diagrams,
            &behavior,
            &behavior_diagrams,
            &activities,
            &activity_diagrams,
        ) {
            return Err(error(
                "TARGET_NOT_BLANK",
                None,
                "portable interchange import requires a blank target project",
            ));
        }
        Ok(())
    })()
    .err();
    ModelBuildPreview {
        proposed_operations,
        diagnostics: diagnostic.into_iter().collect(),
    }
}

pub fn apply_complete_model_build(
    plan: &ModelBuildPlan,
    state: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
) -> Result<ModelBuildResult, ModelBuildPreview> {
    let preview = preview_complete_model_build(plan, state, activity);
    if !preview.is_valid() {
        return Err(preview);
    }
    let portable = portable_state_from_plan(plan).map_err(|diagnostic| ModelBuildPreview {
        proposed_operations: proposed_operations(plan),
        diagnostics: vec![diagnostic],
    })?;

    let mut project = state.project.lock().map_err(|_| ModelBuildPreview {
        proposed_operations: proposed_operations(plan),
        diagnostics: vec![error("LOCK_FAILURE", None, "project lock poisoned")],
    })?;
    let mut diagrams = state.diagrams.lock().map_err(|_| ModelBuildPreview {
        proposed_operations: proposed_operations(plan),
        diagnostics: vec![error("LOCK_FAILURE", None, "diagram lock poisoned")],
    })?;
    let mut ibd_diagrams = state.ibd_diagrams.lock().map_err(|_| ModelBuildPreview {
        proposed_operations: proposed_operations(plan),
        diagnostics: vec![error("LOCK_FAILURE", None, "IBD lock poisoned")],
    })?;
    let mut behavior = state.behavior.lock().map_err(|_| ModelBuildPreview {
        proposed_operations: proposed_operations(plan),
        diagnostics: vec![error("LOCK_FAILURE", None, "behavior lock poisoned")],
    })?;
    let mut behavior_diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| ModelBuildPreview {
            proposed_operations: proposed_operations(plan),
            diagnostics: vec![error(
                "LOCK_FAILURE",
                None,
                "behavior diagram lock poisoned",
            )],
        })?;
    let mut activities = activity.repository.lock().map_err(|_| ModelBuildPreview {
        proposed_operations: proposed_operations(plan),
        diagnostics: vec![error(
            "LOCK_FAILURE",
            None,
            "Activity repository lock poisoned",
        )],
    })?;
    let mut activity_diagrams = activity.diagrams.lock().map_err(|_| ModelBuildPreview {
        proposed_operations: proposed_operations(plan),
        diagnostics: vec![error(
            "LOCK_FAILURE",
            None,
            "Activity diagram lock poisoned",
        )],
    })?;
    let mut current_file = state.current_file.lock().map_err(|_| ModelBuildPreview {
        proposed_operations: proposed_operations(plan),
        diagnostics: vec![error("LOCK_FAILURE", None, "project path lock poisoned")],
    })?;

    if !target_is_blank(
        &project,
        &diagrams,
        &ibd_diagrams,
        &behavior,
        &behavior_diagrams,
        &activities,
        &activity_diagrams,
    ) {
        return Err(ModelBuildPreview {
            proposed_operations: proposed_operations(plan),
            diagnostics: vec![error(
                "TARGET_NOT_BLANK",
                None,
                "portable interchange import requires a blank target project",
            )],
        });
    }

    let result = portable.build_result(&plan.source_namespace);
    *project = Some(portable.project.clone());
    *diagrams = portable.diagrams.clone();
    *ibd_diagrams = portable.ibd_diagrams.clone();
    *behavior = portable.behavior_repository.clone();
    *behavior_diagrams = portable.behavior_diagrams.clone();
    *activities = portable.activity_repository.clone();
    *activity_diagrams = portable.activity_diagrams.clone();
    *current_file = None;
    Ok(result)
}

#[cfg(test)]
mod pr48_tests;
#[cfg(test)]
mod pr49_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn ext(value: &str) -> ElementReference {
        BuildReference::External(value.into())
    }

    fn valid_plan(root_id: ElementId) -> ModelBuildPlan {
        ModelBuildPlan {
            source_namespace: "pr36_acceptance".into(),
            operations: vec![
                ModelBuildOperation::CreateElement {
                    external_id: "PKG".into(),
                    kind: ElementKind::Package,
                    name: "Example".into(),
                    owner: BuildReference::Existing(root_id),
                    type_ref: None,
                },
                ModelBuildOperation::CreateElement {
                    external_id: "A".into(),
                    kind: ElementKind::Block,
                    name: "Block A draft".into(),
                    owner: ext("PKG"),
                    type_ref: None,
                },
                ModelBuildOperation::UpdateElement {
                    element: ext("A"),
                    name: "Block A".into(),
                },
                ModelBuildOperation::CreateElement {
                    external_id: "B".into(),
                    kind: ElementKind::Block,
                    name: "Block B".into(),
                    owner: ext("PKG"),
                    type_ref: None,
                },
                ModelBuildOperation::CreateElement {
                    external_id: "A_PART".into(),
                    kind: ElementKind::PartProperty,
                    name: "b".into(),
                    owner: ext("A"),
                    type_ref: Some(ext("B")),
                },
                ModelBuildOperation::CreateRelationship {
                    external_id: "A_TO_B".into(),
                    kind: RelationshipKind::Association,
                    source: ext("A"),
                    target: ext("B"),
                    owner: Some(ext("PKG")),
                },
                ModelBuildOperation::CreateDiagram {
                    external_id: "BDD_MAIN".into(),
                    family: "bdd".into(),
                    name: "Example Structure".into(),
                    owner: ext("PKG"),
                    semantic_context: None,
                },
                ModelBuildOperation::PresentElement {
                    diagram: BuildReference::External("BDD_MAIN".into()),
                    element: ext("A"),
                    x: 100.0,
                    y: 100.0,
                },
                ModelBuildOperation::PresentElement {
                    diagram: BuildReference::External("BDD_MAIN".into()),
                    element: ext("B"),
                    x: 350.0,
                    y: 100.0,
                },
                ModelBuildOperation::PresentRelationship {
                    diagram: BuildReference::External("BDD_MAIN".into()),
                    relationship: BuildReference::External("A_TO_B".into()),
                },
            ],
        }
    }

    fn workspace() -> (WorkspaceState, ElementId) {
        let state = WorkspaceState::default();
        let project = Project::new("PR36");
        let root = project.root_id;
        *state.project.lock().unwrap() = Some(project);
        (state, root)
    }

    #[test]
    fn acceptance_plan_builds_semantics_and_presentations_once() {
        let (state, root) = workspace();
        let result = apply_model_build(&valid_plan(root), &state).expect("valid build");
        let project = state.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        let key = |id| external_key("pr36_acceptance", id);
        let package = result.element_ids[&key("PKG")];
        let a = result.element_ids[&key("A")];
        let b = result.element_ids[&key("B")];
        let part = result.element_ids[&key("A_PART")];
        let association = result.relationship_ids[&key("A_TO_B")];
        assert_eq!(project.element(a).unwrap().name, "Block A");
        assert_eq!(project.element(a).unwrap().owner_id, Some(package));
        assert_eq!(project.element(b).unwrap().owner_id, Some(package));
        assert_eq!(project.element(part).unwrap().owner_id, Some(a));
        assert_eq!(project.element(part).unwrap().type_id, Some(b));
        assert_eq!(project.relationship(association).unwrap().source_id, a);
        assert_eq!(project.relationship(association).unwrap().target_id, b);
        assert_eq!(
            project
                .elements
                .values()
                .filter(|element| matches!(element.kind, ElementKind::Block))
                .count(),
            2
        );
        assert_eq!(project.relationships.len(), 1);
        let diagrams = state.diagrams.lock().unwrap();
        let diagram = diagrams
            .iter()
            .find(|diagram| diagram.id == result.diagram_ids[&key("BDD_MAIN")].to_string())
            .unwrap();
        assert_eq!(diagram.owner_id, package.to_string());
        assert_eq!(diagram.nodes.len(), 2);
        assert_eq!(diagram.edges.len(), 1);
        assert!(
            diagram
                .nodes
                .iter()
                .any(|node| node.element_id == a.to_string())
        );
        assert!(
            diagram
                .nodes
                .iter()
                .any(|node| node.element_id == b.to_string())
        );
        assert_eq!(diagram.edges[0].relationship_id, association.to_string());
    }

    #[test]
    fn unresolved_reference_leaves_workspace_unchanged() {
        let (state, root) = workspace();
        let mut plan = valid_plan(root);
        if let ModelBuildOperation::CreateRelationship { target, .. } = &mut plan.operations[5] {
            *target = ext("DOES_NOT_EXIST");
        }
        let before = state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len();
        let failure = apply_model_build(&plan, &state).unwrap_err();
        assert_eq!(failure.diagnostics[0].code, "UNRESOLVED_REFERENCE");
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            before
        );
        assert!(state.diagrams.lock().unwrap().is_empty());
    }

    #[test]
    fn invalid_semantic_owner_leaves_workspace_unchanged() {
        let (state, root) = workspace();
        let mut plan = valid_plan(root);
        if let ModelBuildOperation::CreateElement { owner, .. } = &mut plan.operations[4] {
            *owner = ext("PKG");
        }
        let failure = apply_model_build(&plan, &state).unwrap_err();
        assert_eq!(failure.diagnostics[0].code, "SEMANTIC_VALIDATION");
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            1
        );
        assert!(state.diagrams.lock().unwrap().is_empty());
    }

    #[test]
    fn late_apply_failure_rolls_back_semantics_and_presentations() {
        let (state, root) = workspace();
        let mut plan = valid_plan(root);
        plan.operations.push(ModelBuildOperation::PresentElement {
            diagram: BuildReference::External("BDD_MAIN".into()),
            element: ext("A"),
            x: 600.0,
            y: 100.0,
        });
        let failure = apply_model_build(&plan, &state).unwrap_err();
        assert_eq!(failure.diagnostics[0].code, "DUPLICATE_PRESENTATION");
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            1
        );
        assert!(state.diagrams.lock().unwrap().is_empty());
    }

    #[test]
    fn preview_resolves_and_validates_without_mutation() {
        let (state, root) = workspace();
        let preview = preview_model_build(&valid_plan(root), &state);
        assert!(preview.is_valid());
        assert_eq!(preview.proposed_operations.len(), 10);
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            1
        );
        assert!(state.diagrams.lock().unwrap().is_empty());
    }
}
