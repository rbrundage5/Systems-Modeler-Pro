//! PR51 bounded Groovy-compatible/model-script host.
//!
//! The host deliberately does not embed a JVM or execute arbitrary Groovy.  A
//! `.groovy` file may wrap a JSON construction program in `modelScript('''...''')`
//! (or `modelScript("""...""")`).  The payload is compiled exclusively into the
//! existing Rust `ModelBuildPlan` operations and native presentation stores.
//! No script API exposes SQLite, filesystem, process, `Project`, or repository
//! mutation directly.

#![allow(clippy::result_large_err)] // Serialized PR51 diagnostics intentionally retain rich context.

use super::{
    BddDiagram, DiagramEdge, DiagramNode, DiagramPoint, WorkspaceState,
    activity_workspace::{
        ActivityDiagram, ActivityDiagramEdge, ActivityDiagramNode, ActivityWorkspaceState,
    },
    behavior_workspace::{
        BehaviorDiagram, BehaviorDiagramKind, BehaviorEdgePresentation, LifelinePresentation,
        StateNodePresentation,
    },
    bulk_model::{
        ActionBuildKind, ActivityBuildOperation, ActivityEndpointReference, ActivityNodeBuildKind,
        AssociationEndBuildFields, BindingEndpointBuild, BuildReference, ConnectorEndBuildSpec,
        ElementReference, MessageSignatureBuild, ModelBuildOperation, ModelBuildPlan,
        ParametricBuildOperation, RegionParentReference, SequenceBuildOperation,
        StateMachineBuildOperation, TriggerBuild, VertexBuildKind, apply_unified_model_build,
        external_key,
    },
    ibd::{IbdConnectorPresentation, IbdDiagram, IbdPortPresentation, IbdPropertyPresentation},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use systems_modeler_core::behavior::{
    BehaviorSemanticId, InteractionOperator, MessageSort, PseudostateKind, TransitionKind,
    VertexKind,
};
use systems_modeler_core::{
    ActivityEdgeKind, ActivityEndpoint, ActivityNodeKind, ActivitySemanticId, AggregationKind,
    ConnectorKind, ElementId, ElementKind, FlowDirection, Multiplicity, ObjectNodeKind,
    ObjectNodeOrdering, ParameterDirection, PinDirection, Project, RelationshipKind,
    StructuredActivityNodeKind, VisibilityKind,
};

const SCRIPT_HOST: &str = "systems-modeler-rust-model-script-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ModelScriptAction {
    Create,
    Update,
    NoChange,
    Blocked,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelScriptPreviewItem {
    pub statement: usize,
    pub action: ModelScriptAction,
    pub operation: String,
    pub external_id: Option<String>,
    pub semantic_name: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelScriptDiagnostic {
    pub script: String,
    pub line: Option<usize>,
    pub statement: Option<usize>,
    pub operation: Option<String>,
    pub external_id: Option<String>,
    pub semantic_name: Option<String>,
    pub code: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelScriptPreview {
    pub host: &'static str,
    pub applied: bool,
    pub source_namespace: String,
    pub items: Vec<ModelScriptPreviewItem>,
    pub diagnostics: Vec<ModelScriptDiagnostic>,
}

impl ModelScriptPreview {
    fn valid(&self) -> bool {
        self.diagnostics.is_empty()
            && !self
                .items
                .iter()
                .any(|item| item.action == ModelScriptAction::Blocked)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ScriptDocument {
    source_namespace: String,
    #[serde(default)]
    operations: Vec<ScriptOperation>,
    #[serde(default)]
    diagrams: Vec<ScriptDiagram>,
}

#[derive(Debug, Clone, Deserialize)]
struct ScriptDiagram {
    external_id: String,
    family: String,
    name: String,
    owner: String,
    #[serde(default)]
    context: Option<String>,
    /// External ID of the Activity, StateMachine, or Interaction represented by
    /// specialized diagrams.  For ordinary diagrams this is ignored.
    #[serde(default)]
    semantic: Option<String>,
    #[serde(default = "default_true")]
    populate: bool,
    #[serde(default = "default_true")]
    clean_layout: bool,
    #[serde(default = "default_true")]
    route: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
struct ScriptAssociationEnd {
    #[serde(default)]
    role_name: Option<String>,
    #[serde(default)]
    multiplicity: Option<Multiplicity>,
    #[serde(default)]
    navigable: Option<bool>,
    #[serde(default)]
    aggregation: Option<AggregationKind>,
}

impl From<ScriptAssociationEnd> for AssociationEndBuildFields {
    fn from(value: ScriptAssociationEnd) -> Self {
        Self {
            role_name: value.role_name,
            multiplicity: value.multiplicity,
            navigable: value.navigable,
            aggregation: value.aggregation,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ScriptActivityNodeKind {
    Initial,
    ActivityFinal,
    FlowFinal,
    Decision {
        #[serde(default)]
        decision_input: Option<String>,
    },
    Merge,
    Fork,
    Join {
        #[serde(default)]
        join_specification: Option<String>,
    },
    OpaqueAction {
        #[serde(default)]
        body: String,
    },
    CallBehavior {
        activity: String,
    },
    CallOperation {
        operation: String,
    },
    SendSignal {
        signal: String,
    },
    AcceptEvent {
        #[serde(default)]
        signal: Option<String>,
    },
    AcceptTimeEvent {
        expression: String,
    },
    Object {
        object_kind: ObjectNodeKind,
        #[serde(default)]
        type_ref: Option<String>,
        #[serde(default)]
        multiplicity: Multiplicity,
        #[serde(default)]
        ordering: ObjectNodeOrdering,
        #[serde(default)]
        selection: Option<String>,
    },
    ActivityParameter {
        parameter: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ScriptVertexKind {
    State {
        #[serde(default)]
        entry: Option<String>,
        #[serde(default)]
        do_activity: Option<String>,
        #[serde(default)]
        exit: Option<String>,
        #[serde(default)]
        submachine: Option<String>,
    },
    FinalState,
    Pseudostate {
        pseudostate: PseudostateKind,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ScriptTrigger {
    Signal {
        signal: String,
    },
    Call {
        operation: String,
    },
    Time {
        expression: String,
        is_relative: bool,
    },
    Change {
        expression: String,
    },
    AnyReceive,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ScriptMessageSignature {
    Operation { operation: String },
    Signal { signal: String },
}

#[derive(Debug, Clone, Deserialize)]
struct ScriptBindingEndpoint {
    role: String,
    #[serde(default)]
    parameter: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum ScriptOperation {
    Element {
        external_id: String,
        kind: ElementKind,
        name: String,
        owner: String,
        #[serde(default)]
        type_ref: Option<String>,
        #[serde(default)]
        documentation: Option<String>,
        #[serde(default)]
        visibility: Option<VisibilityKind>,
        #[serde(default)]
        requirement_id: Option<String>,
        #[serde(default)]
        requirement_text: Option<String>,
        #[serde(default)]
        multiplicity: Option<Multiplicity>,
        #[serde(default)]
        default_value: Option<String>,
        #[serde(default)]
        parameter_direction: Option<ParameterDirection>,
        #[serde(default)]
        flow_direction: Option<FlowDirection>,
        #[serde(default)]
        is_conjugated: Option<bool>,
        #[serde(default)]
        extension_points: Option<Vec<String>>,
    },
    Relationship {
        external_id: String,
        kind: RelationshipKind,
        source: String,
        target: String,
        #[serde(default)]
        owner: Option<String>,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        documentation: Option<String>,
        #[serde(default)]
        visibility: Option<VisibilityKind>,
        #[serde(default)]
        source_end: Option<ScriptAssociationEnd>,
        #[serde(default)]
        target_end: Option<ScriptAssociationEnd>,
        #[serde(default)]
        alias: Option<String>,
        #[serde(default)]
        extension_condition: Option<String>,
        #[serde(default)]
        extension_location: Option<String>,
    },
    Connector {
        external_id: String,
        context: String,
        kind: ConnectorKind,
        source_path: Vec<String>,
        target_path: Vec<String>,
        #[serde(default)]
        name: String,
        #[serde(default)]
        documentation: String,
        #[serde(default)]
        visibility: VisibilityKind,
    },
    ItemFlow {
        external_id: String,
        connector: String,
        source_path: Vec<String>,
        target_path: Vec<String>,
        conveyed_items: Vec<String>,
        #[serde(default)]
        name: String,
        #[serde(default)]
        documentation: String,
        #[serde(default)]
        visibility: VisibilityKind,
    },
    Activity {
        external_id: String,
        name: String,
        owner: String,
        #[serde(default)]
        context: Option<String>,
    },
    ActivityPartition {
        external_id: String,
        activity: String,
        name: String,
        #[serde(default)]
        represented_element: Option<String>,
        #[serde(default)]
        is_dimension: bool,
        #[serde(default)]
        is_external: bool,
    },
    StructuredActivityNode {
        external_id: String,
        activity: String,
        name: String,
        kind: StructuredActivityNodeKind,
        #[serde(default)]
        parent: Option<String>,
    },
    ActivityNode {
        external_id: String,
        activity: String,
        name: String,
        node: ScriptActivityNodeKind,
        #[serde(default)]
        partition: Option<String>,
        #[serde(default)]
        structured_node: Option<String>,
    },
    Pin {
        external_id: String,
        owner_action: String,
        name: String,
        direction: PinDirection,
        #[serde(default)]
        type_ref: Option<String>,
        #[serde(default)]
        multiplicity: Multiplicity,
        #[serde(default)]
        is_ordered: bool,
        #[serde(default = "default_true")]
        is_unique: bool,
        #[serde(default)]
        value: Option<String>,
        #[serde(default)]
        parameter: Option<String>,
    },
    ActivityEdge {
        external_id: String,
        activity: String,
        name: String,
        kind: ActivityEdgeKind,
        source: String,
        target: String,
        #[serde(default)]
        source_is_pin: bool,
        #[serde(default)]
        target_is_pin: bool,
        #[serde(default)]
        guard: Option<String>,
        #[serde(default)]
        weight: Option<String>,
        #[serde(default)]
        selection: Option<String>,
        #[serde(default)]
        transformation: Option<String>,
        #[serde(default)]
        interrupting_region: Option<String>,
    },
    StateMachine {
        external_id: String,
        name: String,
        context: String,
    },
    Region {
        external_id: String,
        name: String,
        #[serde(default)]
        state_machine: Option<String>,
        #[serde(default)]
        state: Option<String>,
    },
    Vertex {
        external_id: String,
        region: String,
        name: String,
        vertex: ScriptVertexKind,
    },
    Transition {
        external_id: String,
        region: String,
        source: String,
        target: String,
        #[serde(default = "default_transition_kind")]
        kind: TransitionKind,
        #[serde(default)]
        trigger: Option<ScriptTrigger>,
        #[serde(default)]
        guard: Option<String>,
        #[serde(default)]
        effect: Option<String>,
    },
    Interaction {
        external_id: String,
        name: String,
        context: String,
    },
    Lifeline {
        external_id: String,
        interaction: String,
        name: String,
        represented_path: Vec<String>,
    },
    Occurrence {
        external_id: String,
        interaction: String,
        lifeline: String,
        order: u32,
    },
    Message {
        external_id: String,
        interaction: String,
        name: String,
        sort: MessageSort,
        #[serde(default)]
        send: Option<String>,
        #[serde(default)]
        receive: Option<String>,
        #[serde(default)]
        signature: Option<ScriptMessageSignature>,
        #[serde(default)]
        arguments: Vec<String>,
    },
    Execution {
        external_id: String,
        interaction: String,
        lifeline: String,
        start: String,
        finish: String,
        #[serde(default)]
        behavior: Option<String>,
    },
    CombinedFragment {
        external_id: String,
        interaction: String,
        operator: InteractionOperator,
        covered_lifelines: Vec<String>,
    },
    Operand {
        external_id: String,
        fragment: String,
        #[serde(default)]
        guard: Option<String>,
        start_order: u32,
        end_order: u32,
    },
    StateInvariant {
        external_id: String,
        interaction: String,
        lifeline: String,
        order: u32,
        constraint: String,
    },
    ParametricMetadata {
        element: String,
        #[serde(default)]
        constraint_expression: Option<String>,
        #[serde(default)]
        quantity_kind_external_id: Option<String>,
        #[serde(default)]
        unit_external_id: Option<String>,
        #[serde(default)]
        quantity_dimension: Option<String>,
        #[serde(default)]
        unit_symbol: Option<String>,
        #[serde(default)]
        unit_scale_to_base: Option<f64>,
    },
    Binding {
        external_id: String,
        name: String,
        owner: String,
        source: ScriptBindingEndpoint,
        target: ScriptBindingEndpoint,
    },
}

fn default_transition_kind() -> TransitionKind {
    TransitionKind::External
}

#[derive(Debug)]
struct CompiledScript {
    document: ScriptDocument,
    plan: ModelBuildPlan,
    plan_statements: Vec<usize>,
    items: Vec<ModelScriptPreviewItem>,
}

fn diag(
    script: &str,
    statement: Option<usize>,
    operation: Option<String>,
    external_id: Option<String>,
    code: impl Into<String>,
    reason: impl Into<String>,
) -> ModelScriptDiagnostic {
    ModelScriptDiagnostic {
        script: script.into(),
        line: None,
        statement,
        operation,
        external_id,
        semantic_name: None,
        code: code.into(),
        reason: reason.into(),
    }
}

fn extract_payload(source: &str) -> Result<(&str, usize), String> {
    let trimmed = source.trim();
    if trimmed.starts_with('{') {
        return Ok((trimmed, 1));
    }
    for quote in ["'''", "\"\"\""] {
        if let Some(call) = source.find("modelScript") {
            let tail = &source[call + "modelScript".len()..];
            if let Some(open_rel) = tail.find(quote) {
                let start = call + "modelScript".len() + open_rel + quote.len();
                let rest = &source[start..];
                if let Some(close_rel) = rest.rfind(quote) {
                    let line = source[..start].lines().count();
                    return Ok((&rest[..close_rel], line));
                }
            }
        }
    }
    Err("script must be raw model-script JSON or modelScript triple-quoted JSON".into())
}

fn parse_document(
    script_name: &str,
    source: &str,
) -> Result<ScriptDocument, ModelScriptDiagnostic> {
    let (payload, base_line) = extract_payload(source).map_err(|reason| {
        diag(
            script_name,
            None,
            Some("parse".into()),
            None,
            "SCRIPT_SYNTAX",
            reason,
        )
    })?;
    serde_json::from_str::<ScriptDocument>(payload).map_err(|error| {
        let mut item = diag(
            script_name,
            None,
            Some("parse".into()),
            None,
            "SCRIPT_SYNTAX",
            error.to_string(),
        );
        item.line = Some(base_line + error.line().saturating_sub(1));
        item
    })
}

fn script_external_token(token: &str) -> &str {
    token
        .strip_prefix("ext:")
        .or_else(|| token.strip_prefix("handle:"))
        .unwrap_or(token)
}

fn element_reference(
    token: &str,
    project: &Project,
    statement: usize,
) -> Result<ElementReference, ModelScriptDiagnostic> {
    let token = token.trim();
    if token == "$root" {
        return Ok(BuildReference::Existing(project.root_id));
    }
    if let Some(qualified) = token.strip_prefix("qname:") {
        let matches = project
            .elements
            .values()
            .filter(|element| project.qualified_name(element.id).as_deref() == Ok(qualified))
            .map(|element| element.id)
            .collect::<Vec<_>>();
        return match matches.as_slice() {
            [id] => Ok(BuildReference::Existing(*id)),
            [] => Err(diag(
                "",
                Some(statement),
                Some("reference".into()),
                None,
                "UNRESOLVED_REFERENCE",
                format!("exact qualified name was not found: {qualified}"),
            )),
            _ => Err(diag(
                "",
                Some(statement),
                Some("reference".into()),
                None,
                "AMBIGUOUS_REFERENCE",
                format!("exact qualified name is ambiguous: {qualified}"),
            )),
        };
    }
    let external = script_external_token(token).trim();
    if external.is_empty() {
        return Err(diag(
            "",
            Some(statement),
            Some("reference".into()),
            None,
            "UNRESOLVED_REFERENCE",
            "blank External ID reference",
        ));
    }
    Ok(BuildReference::External(external.into()))
}

fn external_reference<T>(
    token: &str,
    statement: usize,
) -> Result<BuildReference<T>, ModelScriptDiagnostic> {
    let external = script_external_token(token).trim();
    if external.is_empty() || external == "$root" || external.starts_with("qname:") {
        return Err(diag(
            "",
            Some(statement),
            Some("reference".into()),
            None,
            "REFERENCE_KIND_INVALID",
            "specialized semantic references require an External ID or plan-local handle",
        ));
    }
    Ok(BuildReference::External(external.into()))
}

fn element_by_external<'a>(
    project: &'a Project,
    namespace: &str,
    external: &str,
) -> Vec<&'a systems_modeler_core::Element> {
    let key = external_key(namespace, external);
    project
        .elements
        .values()
        .filter(|element| element.external_id == key)
        .collect()
}

fn relationship_by_external<'a>(
    project: &'a Project,
    namespace: &str,
    external: &str,
) -> Vec<&'a systems_modeler_core::Relationship> {
    let key = external_key(namespace, external);
    project
        .relationships
        .values()
        .filter(|relationship| relationship.external_id == key)
        .collect()
}

fn item(
    statement: usize,
    action: ModelScriptAction,
    operation: impl Into<String>,
    external_id: Option<&str>,
    semantic_name: Option<&str>,
    detail: impl Into<String>,
) -> ModelScriptPreviewItem {
    ModelScriptPreviewItem {
        statement,
        action,
        operation: operation.into(),
        external_id: external_id.map(str::to_owned),
        semantic_name: semantic_name.map(str::to_owned),
        detail: detail.into(),
    }
}

fn activity_node_kind(
    spec: &ScriptActivityNodeKind,
    project: &Project,
    statement: usize,
) -> Result<ActivityNodeBuildKind, ModelScriptDiagnostic> {
    Ok(match spec {
        ScriptActivityNodeKind::Initial => ActivityNodeBuildKind::Initial,
        ScriptActivityNodeKind::ActivityFinal => ActivityNodeBuildKind::ActivityFinal,
        ScriptActivityNodeKind::FlowFinal => ActivityNodeBuildKind::FlowFinal,
        ScriptActivityNodeKind::Decision { decision_input } => ActivityNodeBuildKind::Decision {
            decision_input: decision_input.clone(),
        },
        ScriptActivityNodeKind::Merge => ActivityNodeBuildKind::Merge,
        ScriptActivityNodeKind::Fork => ActivityNodeBuildKind::Fork,
        ScriptActivityNodeKind::Join { join_specification } => ActivityNodeBuildKind::Join {
            join_specification: join_specification.clone(),
        },
        ScriptActivityNodeKind::OpaqueAction { body } => {
            ActivityNodeBuildKind::Action(ActionBuildKind::Opaque { body: body.clone() })
        }
        ScriptActivityNodeKind::CallBehavior { activity } => {
            ActivityNodeBuildKind::Action(ActionBuildKind::CallBehavior {
                activity: external_reference(activity, statement)?,
            })
        }
        ScriptActivityNodeKind::CallOperation { operation } => {
            ActivityNodeBuildKind::Action(ActionBuildKind::CallOperation {
                operation: element_reference(operation, project, statement)?,
            })
        }
        ScriptActivityNodeKind::SendSignal { signal } => {
            ActivityNodeBuildKind::Action(ActionBuildKind::SendSignal {
                signal: element_reference(signal, project, statement)?,
            })
        }
        ScriptActivityNodeKind::AcceptEvent { signal } => {
            ActivityNodeBuildKind::Action(ActionBuildKind::AcceptEvent {
                signal: signal
                    .as_deref()
                    .map(|value| element_reference(value, project, statement))
                    .transpose()?,
            })
        }
        ScriptActivityNodeKind::AcceptTimeEvent { expression } => {
            ActivityNodeBuildKind::Action(ActionBuildKind::AcceptTimeEvent {
                expression: expression.clone(),
            })
        }
        ScriptActivityNodeKind::Object {
            object_kind,
            type_ref,
            multiplicity,
            ordering,
            selection,
        } => ActivityNodeBuildKind::Object {
            kind: *object_kind,
            type_ref: type_ref
                .as_deref()
                .map(|value| element_reference(value, project, statement))
                .transpose()?,
            multiplicity: *multiplicity,
            ordering: *ordering,
            selection: selection.clone(),
        },
        ScriptActivityNodeKind::ActivityParameter { parameter } => {
            ActivityNodeBuildKind::ActivityParameter {
                parameter: element_reference(parameter, project, statement)?,
            }
        }
    })
}

fn vertex_kind(
    spec: &ScriptVertexKind,
    statement: usize,
) -> Result<VertexBuildKind, ModelScriptDiagnostic> {
    Ok(match spec {
        ScriptVertexKind::State {
            entry,
            do_activity,
            exit,
            submachine,
        } => VertexBuildKind::State {
            entry: entry.clone(),
            do_activity: do_activity.clone(),
            exit: exit.clone(),
            submachine: submachine
                .as_deref()
                .map(|value| external_reference(value, statement))
                .transpose()?,
        },
        ScriptVertexKind::FinalState => VertexBuildKind::FinalState,
        ScriptVertexKind::Pseudostate { pseudostate } => VertexBuildKind::Pseudostate(*pseudostate),
    })
}

fn trigger(
    spec: &ScriptTrigger,
    project: &Project,
    statement: usize,
) -> Result<TriggerBuild, ModelScriptDiagnostic> {
    Ok(match spec {
        ScriptTrigger::Signal { signal } => {
            TriggerBuild::Signal(element_reference(signal, project, statement)?)
        }
        ScriptTrigger::Call { operation } => {
            TriggerBuild::Call(element_reference(operation, project, statement)?)
        }
        ScriptTrigger::Time {
            expression,
            is_relative,
        } => TriggerBuild::Time {
            expression: expression.clone(),
            is_relative: *is_relative,
        },
        ScriptTrigger::Change { expression } => TriggerBuild::Change {
            expression: expression.clone(),
        },
        ScriptTrigger::AnyReceive => TriggerBuild::AnyReceive,
    })
}

fn signature(
    spec: &ScriptMessageSignature,
    project: &Project,
    statement: usize,
) -> Result<MessageSignatureBuild, ModelScriptDiagnostic> {
    Ok(match spec {
        ScriptMessageSignature::Operation { operation } => {
            MessageSignatureBuild::Operation(element_reference(operation, project, statement)?)
        }
        ScriptMessageSignature::Signal { signal } => {
            MessageSignatureBuild::Signal(element_reference(signal, project, statement)?)
        }
    })
}

fn binding_endpoint(
    spec: &ScriptBindingEndpoint,
    project: &Project,
    statement: usize,
) -> Result<BindingEndpointBuild, ModelScriptDiagnostic> {
    Ok(BindingEndpointBuild {
        role: element_reference(&spec.role, project, statement)?,
        parameter: spec
            .parameter
            .as_deref()
            .map(|value| element_reference(value, project, statement))
            .transpose()?,
    })
}

fn compile_script(
    script_name: &str,
    source: &str,
    project: &Project,
    activities: &systems_modeler_core::ActivityRepository,
    behavior: &systems_modeler_core::BehaviorRepository,
) -> Result<CompiledScript, Vec<ModelScriptDiagnostic>> {
    let document = parse_document(script_name, source).map_err(|item| vec![item])?;
    let namespace = document.source_namespace.trim();
    if namespace.is_empty() {
        return Err(vec![diag(
            script_name,
            None,
            Some("compile".into()),
            None,
            "SOURCE_NAMESPACE_REQUIRED",
            "source_namespace must not be blank",
        )]);
    }

    let mut ordinary = Vec::new();
    let mut ordinary_statements = Vec::new();
    let mut specialized = Vec::new();
    let mut specialized_statements = Vec::new();
    let mut items = Vec::new();
    let mut diagnostics = Vec::new();
    let mut script_ids = HashSet::new();

    macro_rules! push_ordinary {
        ($statement:expr, $operation:expr) => {{
            ordinary.push($operation);
            ordinary_statements.push($statement);
        }};
    }
    macro_rules! push_specialized {
        ($statement:expr, $operation:expr) => {{
            specialized.push($operation);
            specialized_statements.push($statement);
        }};
    }

    for (index, operation) in document.operations.iter().enumerate() {
        let statement = index + 1;
        let external = match operation {
            ScriptOperation::Element { external_id, .. }
            | ScriptOperation::Relationship { external_id, .. }
            | ScriptOperation::Connector { external_id, .. }
            | ScriptOperation::ItemFlow { external_id, .. }
            | ScriptOperation::Activity { external_id, .. }
            | ScriptOperation::ActivityPartition { external_id, .. }
            | ScriptOperation::StructuredActivityNode { external_id, .. }
            | ScriptOperation::ActivityNode { external_id, .. }
            | ScriptOperation::Pin { external_id, .. }
            | ScriptOperation::ActivityEdge { external_id, .. }
            | ScriptOperation::StateMachine { external_id, .. }
            | ScriptOperation::Region { external_id, .. }
            | ScriptOperation::Vertex { external_id, .. }
            | ScriptOperation::Transition { external_id, .. }
            | ScriptOperation::Interaction { external_id, .. }
            | ScriptOperation::Lifeline { external_id, .. }
            | ScriptOperation::Occurrence { external_id, .. }
            | ScriptOperation::Message { external_id, .. }
            | ScriptOperation::Execution { external_id, .. }
            | ScriptOperation::CombinedFragment { external_id, .. }
            | ScriptOperation::Operand { external_id, .. }
            | ScriptOperation::StateInvariant { external_id, .. }
            | ScriptOperation::Binding { external_id, .. } => Some(external_id.as_str()),
            ScriptOperation::ParametricMetadata { .. } => None,
        };
        if let Some(external) = external {
            if external.trim().is_empty() {
                diagnostics.push(diag(
                    script_name,
                    Some(statement),
                    Some("compile".into()),
                    Some(external.into()),
                    "EXTERNAL_ID_REQUIRED",
                    "External ID must not be blank",
                ));
                continue;
            }
            if !script_ids.insert(external.to_owned()) {
                diagnostics.push(diag(
                    script_name,
                    Some(statement),
                    Some("compile".into()),
                    Some(external.into()),
                    "DUPLICATE_EXTERNAL_ID",
                    "External ID is duplicated in this script transaction",
                ));
                continue;
            }
        }

        let compiled: Result<(), ModelScriptDiagnostic> = (|| {
            match operation {
                ScriptOperation::Element {
                    external_id,
                    kind,
                    name,
                    owner,
                    type_ref,
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
                    let existing = element_by_external(project, namespace, external_id);
                    if existing.len() > 1 {
                        return Err(diag(
                            "",
                            Some(statement),
                            Some("element".into()),
                            Some(external_id.clone()),
                            "AMBIGUOUS_REFERENCE",
                            "multiple elements have the same namespaced External ID",
                        ));
                    }
                    if let Some(existing) = existing.first() {
                        if existing.kind != *kind {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("element".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                format!(
                                    "External ID identifies {:?}, not {:?}",
                                    existing.kind, kind
                                ),
                            ));
                        }
                        let optional_fields = documentation.is_some()
                            || visibility.is_some()
                            || requirement_id.is_some()
                            || requirement_text.is_some()
                            || multiplicity.is_some()
                            || default_value.is_some()
                            || parameter_direction.is_some()
                            || flow_direction.is_some()
                            || is_conjugated.is_some()
                            || extension_points.is_some()
                            || type_ref.is_some();
                        if existing.name == *name && !optional_fields {
                            items.push(item(
                                statement,
                                ModelScriptAction::NoChange,
                                format!("Element::{kind:?}"),
                                Some(external_id),
                                Some(name),
                                "stable identity already matches",
                            ));
                        } else {
                            push_ordinary!(
                                statement,
                                ModelBuildOperation::UpdateElementFields {
                                    element: BuildReference::External(external_id.clone()),
                                    name: Some(name.clone()),
                                    owner: Some(element_reference(owner, project, statement)?),
                                    type_ref: type_ref
                                        .as_deref()
                                        .map(|value| element_reference(value, project, statement))
                                        .transpose()?,
                                    external_id: None,
                                    documentation: documentation.clone(),
                                    visibility: *visibility,
                                    requirement_id: requirement_id.clone(),
                                    requirement_text: requirement_text.clone(),
                                    multiplicity: *multiplicity,
                                    default_value: default_value.clone(),
                                    parameter_direction: *parameter_direction,
                                    flow_direction: *flow_direction,
                                    is_conjugated: *is_conjugated,
                                    extension_points: extension_points.clone(),
                                }
                            );
                            items.push(item(
                                statement,
                                ModelScriptAction::Update,
                                format!("Element::{kind:?}"),
                                Some(external_id),
                                Some(name),
                                "update through ModelBuildPlan",
                            ));
                        }
                    } else {
                        push_ordinary!(
                            statement,
                            ModelBuildOperation::CreateElement {
                                external_id: external_id.clone(),
                                kind: kind.clone(),
                                name: name.clone(),
                                owner: element_reference(owner, project, statement)?,
                                type_ref: type_ref
                                    .as_deref()
                                    .map(|value| element_reference(value, project, statement))
                                    .transpose()?,
                            }
                        );
                        if documentation.is_some()
                            || visibility.is_some()
                            || requirement_id.is_some()
                            || requirement_text.is_some()
                            || multiplicity.is_some()
                            || default_value.is_some()
                            || parameter_direction.is_some()
                            || flow_direction.is_some()
                            || is_conjugated.is_some()
                            || extension_points.is_some()
                        {
                            push_ordinary!(
                                statement,
                                ModelBuildOperation::UpdateElementFields {
                                    element: BuildReference::External(external_id.clone()),
                                    name: None,
                                    owner: None,
                                    type_ref: None,
                                    external_id: None,
                                    documentation: documentation.clone(),
                                    visibility: *visibility,
                                    requirement_id: requirement_id.clone(),
                                    requirement_text: requirement_text.clone(),
                                    multiplicity: *multiplicity,
                                    default_value: default_value.clone(),
                                    parameter_direction: *parameter_direction,
                                    flow_direction: *flow_direction,
                                    is_conjugated: *is_conjugated,
                                    extension_points: extension_points.clone(),
                                }
                            );
                        }
                        items.push(item(
                            statement,
                            ModelScriptAction::Create,
                            format!("Element::{kind:?}"),
                            Some(external_id),
                            Some(name),
                            "create through ModelBuildPlan",
                        ));
                    }
                }
                ScriptOperation::Relationship {
                    external_id,
                    kind,
                    source,
                    target,
                    owner,
                    name,
                    documentation,
                    visibility,
                    source_end,
                    target_end,
                    alias,
                    extension_condition,
                    extension_location,
                } => {
                    let existing = relationship_by_external(project, namespace, external_id);
                    if existing.len() > 1 {
                        return Err(diag(
                            "",
                            Some(statement),
                            Some("relationship".into()),
                            Some(external_id.clone()),
                            "AMBIGUOUS_REFERENCE",
                            "multiple relationships have the same namespaced External ID",
                        ));
                    }
                    if let Some(existing) = existing.first() {
                        if existing.kind != *kind {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("relationship".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                format!(
                                    "External ID identifies {:?}, not {:?}",
                                    existing.kind, kind
                                ),
                            ));
                        }
                        let any_fields = name.is_some()
                            || documentation.is_some()
                            || visibility.is_some()
                            || source_end.is_some()
                            || target_end.is_some()
                            || alias.is_some()
                            || extension_condition.is_some()
                            || extension_location.is_some();
                        if !any_fields {
                            items.push(item(
                                statement,
                                ModelScriptAction::NoChange,
                                format!("Relationship::{kind:?}"),
                                Some(external_id),
                                None,
                                "stable identity already exists",
                            ));
                        } else {
                            push_ordinary!(
                                statement,
                                ModelBuildOperation::UpdateRelationshipFields {
                                    relationship: BuildReference::External(external_id.clone()),
                                    name: name.clone(),
                                    owner: owner
                                        .as_deref()
                                        .map(|value| element_reference(value, project, statement))
                                        .transpose()?,
                                    source: Some(element_reference(source, project, statement)?),
                                    target: Some(element_reference(target, project, statement)?),
                                    external_id: None,
                                    documentation: documentation.clone(),
                                    visibility: *visibility,
                                    source_end: source_end.clone().map(Into::into),
                                    target_end: target_end.clone().map(Into::into),
                                    alias: alias.clone().map(Some),
                                    extension_condition: extension_condition.clone().map(Some),
                                    extension_location: extension_location.clone().map(Some),
                                }
                            );
                            items.push(item(
                                statement,
                                ModelScriptAction::Update,
                                format!("Relationship::{kind:?}"),
                                Some(external_id),
                                None,
                                "update through ModelBuildPlan",
                            ));
                        }
                    } else {
                        push_ordinary!(
                            statement,
                            ModelBuildOperation::CreateRelationship {
                                external_id: external_id.clone(),
                                kind: kind.clone(),
                                source: element_reference(source, project, statement)?,
                                target: element_reference(target, project, statement)?,
                                owner: owner
                                    .as_deref()
                                    .map(|value| element_reference(value, project, statement))
                                    .transpose()?,
                            }
                        );
                        if name.is_some()
                            || documentation.is_some()
                            || visibility.is_some()
                            || source_end.is_some()
                            || target_end.is_some()
                            || alias.is_some()
                            || extension_condition.is_some()
                            || extension_location.is_some()
                        {
                            push_ordinary!(
                                statement,
                                ModelBuildOperation::UpdateRelationshipFields {
                                    relationship: BuildReference::External(external_id.clone()),
                                    name: name.clone(),
                                    owner: None,
                                    source: None,
                                    target: None,
                                    external_id: None,
                                    documentation: documentation.clone(),
                                    visibility: *visibility,
                                    source_end: source_end.clone().map(Into::into),
                                    target_end: target_end.clone().map(Into::into),
                                    alias: alias.clone().map(Some),
                                    extension_condition: extension_condition.clone().map(Some),
                                    extension_location: extension_location.clone().map(Some),
                                }
                            );
                        }
                        items.push(item(
                            statement,
                            ModelScriptAction::Create,
                            format!("Relationship::{kind:?}"),
                            Some(external_id),
                            None,
                            "create through ModelBuildPlan",
                        ));
                    }
                }
                ScriptOperation::Connector {
                    external_id,
                    context,
                    kind,
                    source_path,
                    target_path,
                    name,
                    documentation,
                    visibility,
                } => {
                    let existing = relationship_by_external(project, namespace, external_id);
                    if let Some(record) = existing.first() {
                        if record.kind != RelationshipKind::Connector {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("connector".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID does not identify a Connector",
                            ));
                        }
                        push_ordinary!(
                            statement,
                            ModelBuildOperation::UpdateConnectorFields {
                                relationship: BuildReference::External(external_id.clone()),
                                context: element_reference(context, project, statement)?,
                                kind: *kind,
                                source: ConnectorEndBuildSpec {
                                    segments: source_path.clone()
                                },
                                target: ConnectorEndBuildSpec {
                                    segments: target_path.clone()
                                },
                                external_id: None,
                                name: Some(name.clone()),
                                documentation: Some(documentation.clone()),
                                visibility: Some(*visibility)
                            }
                        );
                        items.push(item(
                            statement,
                            ModelScriptAction::Update,
                            "Connector",
                            Some(external_id),
                            Some(name),
                            "update through ModelBuildPlan",
                        ));
                    } else {
                        push_ordinary!(
                            statement,
                            ModelBuildOperation::CreateConnector {
                                external_id: external_id.clone(),
                                context: element_reference(context, project, statement)?,
                                kind: *kind,
                                source: ConnectorEndBuildSpec {
                                    segments: source_path.clone()
                                },
                                target: ConnectorEndBuildSpec {
                                    segments: target_path.clone()
                                },
                                name: name.clone(),
                                documentation: documentation.clone(),
                                visibility: *visibility
                            }
                        );
                        items.push(item(
                            statement,
                            ModelScriptAction::Create,
                            "Connector",
                            Some(external_id),
                            Some(name),
                            "create through ModelBuildPlan",
                        ));
                    }
                }
                ScriptOperation::ItemFlow {
                    external_id,
                    connector,
                    source_path,
                    target_path,
                    conveyed_items,
                    name,
                    documentation,
                    visibility,
                } => {
                    let conveyed = conveyed_items
                        .iter()
                        .map(|value| element_reference(value, project, statement))
                        .collect::<Result<Vec<_>, _>>()?;
                    let existing = relationship_by_external(project, namespace, external_id);
                    if let Some(record) = existing.first() {
                        if record.kind != RelationshipKind::ItemFlow {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("item_flow".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID does not identify an ItemFlow",
                            ));
                        }
                        push_ordinary!(
                            statement,
                            ModelBuildOperation::UpdateItemFlowFields {
                                relationship: BuildReference::External(external_id.clone()),
                                connector: external_reference(connector, statement)?,
                                source: ConnectorEndBuildSpec {
                                    segments: source_path.clone()
                                },
                                target: ConnectorEndBuildSpec {
                                    segments: target_path.clone()
                                },
                                conveyed_items: conveyed,
                                external_id: None,
                                name: Some(name.clone()),
                                documentation: Some(documentation.clone()),
                                visibility: Some(*visibility)
                            }
                        );
                        items.push(item(
                            statement,
                            ModelScriptAction::Update,
                            "ItemFlow",
                            Some(external_id),
                            Some(name),
                            "update through ModelBuildPlan",
                        ));
                    } else {
                        push_ordinary!(
                            statement,
                            ModelBuildOperation::CreateItemFlow {
                                external_id: external_id.clone(),
                                connector: external_reference(connector, statement)?,
                                source: ConnectorEndBuildSpec {
                                    segments: source_path.clone()
                                },
                                target: ConnectorEndBuildSpec {
                                    segments: target_path.clone()
                                },
                                conveyed_items: conveyed,
                                name: name.clone(),
                                documentation: documentation.clone(),
                                visibility: *visibility
                            }
                        );
                        items.push(item(
                            statement,
                            ModelScriptAction::Create,
                            "ItemFlow",
                            Some(external_id),
                            Some(name),
                            "create through ModelBuildPlan",
                        ));
                    }
                }
                ScriptOperation::Activity {
                    external_id,
                    name,
                    owner,
                    context,
                } => {
                    let key = external_key(namespace, external_id);
                    if let Some(record) = activities
                        .activities
                        .values()
                        .find(|record| record.external_id == key)
                    {
                        push_specialized!(
                            statement,
                            ModelBuildOperation::Activity {
                                operation: ActivityBuildOperation::UpdateActivity {
                                    activity: BuildReference::External(external_id.clone()),
                                    name: (record.name != *name).then_some(name.clone()),
                                    owner: Some(element_reference(owner, project, statement)?),
                                    context: Some(
                                        context
                                            .as_deref()
                                            .map(|value| element_reference(
                                                value, project, statement
                                            ))
                                            .transpose()?
                                    )
                                }
                            }
                        );
                        items.push(item(
                            statement,
                            ModelScriptAction::Update,
                            "Activity",
                            Some(external_id),
                            Some(name),
                            "update through unified candidate",
                        ));
                    } else {
                        push_specialized!(
                            statement,
                            ModelBuildOperation::Activity {
                                operation: ActivityBuildOperation::CreateActivity {
                                    external_id: external_id.clone(),
                                    name: name.clone(),
                                    owner: element_reference(owner, project, statement)?,
                                    context: context
                                        .as_deref()
                                        .map(|value| element_reference(value, project, statement))
                                        .transpose()?
                                }
                            }
                        );
                        items.push(item(
                            statement,
                            ModelScriptAction::Create,
                            "Activity",
                            Some(external_id),
                            Some(name),
                            "create through unified candidate",
                        ));
                    }
                }
                ScriptOperation::ActivityPartition {
                    external_id,
                    activity,
                    name,
                    represented_element,
                    is_dimension,
                    is_external,
                } => {
                    let key = external_key(namespace, external_id);
                    let update = activities.external_ids.get(&key).copied();
                    let op = if update.is_some() {
                        if !matches!(update, Some(ActivitySemanticId::Partition(_))) {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("activity_partition".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID has another Activity semantic kind",
                            ));
                        }
                        ActivityBuildOperation::UpdatePartition {
                            partition: external_reference(external_id, statement)?,
                            name: Some(name.clone()),
                            represented_element: Some(
                                represented_element
                                    .as_deref()
                                    .map(|value| element_reference(value, project, statement))
                                    .transpose()?,
                            ),
                            is_dimension: Some(*is_dimension),
                            is_external: Some(*is_external),
                        }
                    } else {
                        ActivityBuildOperation::CreatePartition {
                            external_id: external_id.clone(),
                            activity: external_reference(activity, statement)?,
                            name: name.clone(),
                            represented_element: represented_element
                                .as_deref()
                                .map(|value| element_reference(value, project, statement))
                                .transpose()?,
                            is_dimension: *is_dimension,
                            is_external: *is_external,
                        }
                    };
                    let action = if update.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(statement, ModelBuildOperation::Activity { operation: op });
                    items.push(item(
                        statement,
                        action,
                        "ActivityPartition",
                        Some(external_id),
                        Some(name),
                        "unified Activity repository",
                    ));
                }
                ScriptOperation::StructuredActivityNode {
                    external_id,
                    activity,
                    name,
                    kind,
                    parent,
                } => {
                    let key = external_key(namespace, external_id);
                    let update = activities.external_ids.get(&key).copied();
                    let op = if update.is_some() {
                        if !matches!(update, Some(ActivitySemanticId::StructuredNode(_))) {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("structured_activity_node".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID has another Activity semantic kind",
                            ));
                        }
                        ActivityBuildOperation::UpdateStructuredNode {
                            node: external_reference(external_id, statement)?,
                            name: Some(name.clone()),
                            kind: Some(*kind),
                            parent: Some(
                                parent
                                    .as_deref()
                                    .map(|value| external_reference(value, statement))
                                    .transpose()?,
                            ),
                        }
                    } else {
                        ActivityBuildOperation::CreateStructuredNode {
                            external_id: external_id.clone(),
                            activity: external_reference(activity, statement)?,
                            name: name.clone(),
                            kind: *kind,
                            parent: parent
                                .as_deref()
                                .map(|value| external_reference(value, statement))
                                .transpose()?,
                        }
                    };
                    let action = if update.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(statement, ModelBuildOperation::Activity { operation: op });
                    items.push(item(
                        statement,
                        action,
                        "StructuredActivityNode",
                        Some(external_id),
                        Some(name),
                        "unified Activity repository",
                    ));
                }
                ScriptOperation::ActivityNode {
                    external_id,
                    activity,
                    name,
                    node,
                    partition,
                    structured_node,
                } => {
                    let key = external_key(namespace, external_id);
                    let update = activities.external_ids.get(&key).copied();
                    let native = activity_node_kind(node, project, statement)?;
                    let op = if update.is_some() {
                        if !matches!(update, Some(ActivitySemanticId::Node(_))) {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("activity_node".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID has another Activity semantic kind",
                            ));
                        }
                        ActivityBuildOperation::UpdateNode {
                            node: external_reference(external_id, statement)?,
                            name: Some(name.clone()),
                            kind: Some(native),
                            partition: Some(
                                partition
                                    .as_deref()
                                    .map(|value| external_reference(value, statement))
                                    .transpose()?,
                            ),
                            structured_node: Some(
                                structured_node
                                    .as_deref()
                                    .map(|value| external_reference(value, statement))
                                    .transpose()?,
                            ),
                        }
                    } else {
                        ActivityBuildOperation::CreateNode {
                            external_id: external_id.clone(),
                            activity: external_reference(activity, statement)?,
                            name: name.clone(),
                            kind: native,
                            partition: partition
                                .as_deref()
                                .map(|value| external_reference(value, statement))
                                .transpose()?,
                            structured_node: structured_node
                                .as_deref()
                                .map(|value| external_reference(value, statement))
                                .transpose()?,
                        }
                    };
                    let action = if update.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(statement, ModelBuildOperation::Activity { operation: op });
                    items.push(item(
                        statement,
                        action,
                        "ActivityNode",
                        Some(external_id),
                        Some(name),
                        "unified Activity repository",
                    ));
                }
                ScriptOperation::Pin {
                    external_id,
                    owner_action,
                    name,
                    direction,
                    type_ref,
                    multiplicity,
                    is_ordered,
                    is_unique,
                    value,
                    parameter,
                } => {
                    let key = external_key(namespace, external_id);
                    let update = activities.external_ids.get(&key).copied();
                    let type_native = type_ref
                        .as_deref()
                        .map(|value| element_reference(value, project, statement))
                        .transpose()?;
                    let parameter_native = parameter
                        .as_deref()
                        .map(|value| element_reference(value, project, statement))
                        .transpose()?;
                    let op = if update.is_some() {
                        if !matches!(update, Some(ActivitySemanticId::Pin(_))) {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("pin".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID has another Activity semantic kind",
                            ));
                        }
                        ActivityBuildOperation::UpdatePin {
                            pin: external_reference(external_id, statement)?,
                            name: Some(name.clone()),
                            direction: Some(*direction),
                            type_ref: Some(type_native),
                            multiplicity: Some(*multiplicity),
                            is_ordered: Some(*is_ordered),
                            is_unique: Some(*is_unique),
                            value: Some(value.clone()),
                            parameter: Some(parameter_native),
                        }
                    } else {
                        ActivityBuildOperation::CreatePin {
                            external_id: external_id.clone(),
                            owner_action: external_reference(owner_action, statement)?,
                            name: name.clone(),
                            direction: *direction,
                            type_ref: type_native,
                            multiplicity: *multiplicity,
                            is_ordered: *is_ordered,
                            is_unique: *is_unique,
                            value: value.clone(),
                            parameter: parameter_native,
                        }
                    };
                    let action = if update.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(statement, ModelBuildOperation::Activity { operation: op });
                    items.push(item(
                        statement,
                        action,
                        "Pin",
                        Some(external_id),
                        Some(name),
                        "unified Activity repository",
                    ));
                }
                ScriptOperation::ActivityEdge {
                    external_id,
                    activity,
                    name,
                    kind,
                    source,
                    target,
                    source_is_pin,
                    target_is_pin,
                    guard,
                    weight,
                    selection,
                    transformation,
                    interrupting_region,
                } => {
                    let endpoint = |value: &str, is_pin: bool| -> Result<ActivityEndpointReference, ModelScriptDiagnostic> { Ok(if is_pin { ActivityEndpointReference::Pin(external_reference(value, statement)?) } else { ActivityEndpointReference::Node(external_reference(value, statement)?) }) };
                    let key = external_key(namespace, external_id);
                    let update = activities.external_ids.get(&key).copied();
                    let op = if update.is_some() {
                        if !matches!(update, Some(ActivitySemanticId::Edge(_))) {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("activity_edge".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID has another Activity semantic kind",
                            ));
                        }
                        ActivityBuildOperation::UpdateEdge {
                            edge: external_reference(external_id, statement)?,
                            name: Some(name.clone()),
                            kind: Some(*kind),
                            source: Some(endpoint(source, *source_is_pin)?),
                            target: Some(endpoint(target, *target_is_pin)?),
                            guard: Some(guard.clone()),
                            weight: Some(weight.clone()),
                            selection: Some(selection.clone()),
                            transformation: Some(transformation.clone()),
                            interrupting_region: Some(
                                interrupting_region
                                    .as_deref()
                                    .map(|value| external_reference(value, statement))
                                    .transpose()?,
                            ),
                        }
                    } else {
                        ActivityBuildOperation::CreateEdge {
                            external_id: external_id.clone(),
                            activity: external_reference(activity, statement)?,
                            name: name.clone(),
                            kind: *kind,
                            source: endpoint(source, *source_is_pin)?,
                            target: endpoint(target, *target_is_pin)?,
                            guard: guard.clone(),
                            weight: weight.clone(),
                            selection: selection.clone(),
                            transformation: transformation.clone(),
                            interrupting_region: interrupting_region
                                .as_deref()
                                .map(|value| external_reference(value, statement))
                                .transpose()?,
                        }
                    };
                    let action = if update.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(statement, ModelBuildOperation::Activity { operation: op });
                    items.push(item(
                        statement,
                        action,
                        "ActivityEdge",
                        Some(external_id),
                        Some(name),
                        "unified Activity repository",
                    ));
                }
                ScriptOperation::StateMachine {
                    external_id,
                    name,
                    context,
                } => {
                    let key = external_key(namespace, external_id);
                    let existing = behavior
                        .state_machines
                        .values()
                        .find(|record| record.external_id == key);
                    let op = if existing.is_some() {
                        StateMachineBuildOperation::UpdateStateMachine {
                            state_machine: external_reference(external_id, statement)?,
                            name: Some(name.clone()),
                            context: Some(element_reference(context, project, statement)?),
                        }
                    } else {
                        StateMachineBuildOperation::CreateStateMachine {
                            external_id: external_id.clone(),
                            name: name.clone(),
                            context: element_reference(context, project, statement)?,
                        }
                    };
                    let action = if existing.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(
                        statement,
                        ModelBuildOperation::StateMachine { operation: op }
                    );
                    items.push(item(
                        statement,
                        action,
                        "StateMachine",
                        Some(external_id),
                        Some(name),
                        "unified Behavior repository",
                    ));
                }
                ScriptOperation::Region {
                    external_id,
                    name,
                    state_machine,
                    state,
                } => {
                    if state_machine.is_some() == state.is_some() {
                        return Err(diag(
                            "",
                            Some(statement),
                            Some("region".into()),
                            Some(external_id.clone()),
                            "REGION_PARENT_INVALID",
                            "provide exactly one of state_machine or state",
                        ));
                    }
                    let key = external_key(namespace, external_id);
                    let existing = behavior.external_ids.get(&key).copied();
                    let parent = if let Some(value) = state_machine {
                        RegionParentReference::StateMachine(external_reference(value, statement)?)
                    } else {
                        RegionParentReference::State(external_reference(
                            state.as_deref().unwrap(),
                            statement,
                        )?)
                    };
                    let op = if existing.is_some() {
                        if !matches!(existing, Some(BehaviorSemanticId::Region(_))) {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("region".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID has another Behavior semantic kind",
                            ));
                        }
                        StateMachineBuildOperation::UpdateRegion {
                            region: external_reference(external_id, statement)?,
                            name: Some(name.clone()),
                        }
                    } else {
                        StateMachineBuildOperation::CreateRegion {
                            external_id: external_id.clone(),
                            parent,
                            name: name.clone(),
                        }
                    };
                    let action = if existing.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(
                        statement,
                        ModelBuildOperation::StateMachine { operation: op }
                    );
                    items.push(item(
                        statement,
                        action,
                        "Region",
                        Some(external_id),
                        Some(name),
                        "unified Behavior repository",
                    ));
                }
                ScriptOperation::Vertex {
                    external_id,
                    region,
                    name,
                    vertex,
                } => {
                    let key = external_key(namespace, external_id);
                    let existing = behavior.external_ids.get(&key).copied();
                    let kind = vertex_kind(vertex, statement)?;
                    let op = if existing.is_some() {
                        if !matches!(existing, Some(BehaviorSemanticId::Vertex(_))) {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("vertex".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID has another Behavior semantic kind",
                            ));
                        }
                        StateMachineBuildOperation::UpdateVertex {
                            vertex: external_reference(external_id, statement)?,
                            name: Some(name.clone()),
                            kind: Some(kind),
                        }
                    } else {
                        StateMachineBuildOperation::CreateVertex {
                            external_id: external_id.clone(),
                            region: external_reference(region, statement)?,
                            name: name.clone(),
                            kind,
                        }
                    };
                    let action = if existing.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(
                        statement,
                        ModelBuildOperation::StateMachine { operation: op }
                    );
                    items.push(item(
                        statement,
                        action,
                        "Vertex",
                        Some(external_id),
                        Some(name),
                        "unified Behavior repository",
                    ));
                }
                ScriptOperation::Transition {
                    external_id,
                    region,
                    source,
                    target,
                    kind,
                    trigger: trigger_spec,
                    guard,
                    effect,
                } => {
                    let key = external_key(namespace, external_id);
                    let existing = behavior.external_ids.get(&key).copied();
                    let trigger_native = trigger_spec
                        .as_ref()
                        .map(|value| trigger(value, project, statement))
                        .transpose()?;
                    let op = if existing.is_some() {
                        if !matches!(existing, Some(BehaviorSemanticId::Transition(_))) {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("transition".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID has another Behavior semantic kind",
                            ));
                        }
                        StateMachineBuildOperation::UpdateTransition {
                            transition: external_reference(external_id, statement)?,
                            source: Some(external_reference(source, statement)?),
                            target: Some(external_reference(target, statement)?),
                            kind: Some(*kind),
                            trigger: Some(trigger_native),
                            guard: Some(guard.clone()),
                            effect: Some(effect.clone()),
                        }
                    } else {
                        StateMachineBuildOperation::CreateTransition {
                            external_id: external_id.clone(),
                            region: external_reference(region, statement)?,
                            source: external_reference(source, statement)?,
                            target: external_reference(target, statement)?,
                            kind: *kind,
                            trigger: trigger_native,
                            guard: guard.clone(),
                            effect: effect.clone(),
                        }
                    };
                    let action = if existing.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(
                        statement,
                        ModelBuildOperation::StateMachine { operation: op }
                    );
                    items.push(item(
                        statement,
                        action,
                        "Transition",
                        Some(external_id),
                        None,
                        "unified Behavior repository",
                    ));
                }
                ScriptOperation::Interaction {
                    external_id,
                    name,
                    context,
                } => {
                    let key = external_key(namespace, external_id);
                    let existing = behavior
                        .interactions
                        .values()
                        .find(|record| record.external_id == key);
                    let op = if existing.is_some() {
                        SequenceBuildOperation::UpdateInteraction {
                            interaction: external_reference(external_id, statement)?,
                            name: Some(name.clone()),
                            context: Some(element_reference(context, project, statement)?),
                        }
                    } else {
                        SequenceBuildOperation::CreateInteraction {
                            external_id: external_id.clone(),
                            name: name.clone(),
                            context: element_reference(context, project, statement)?,
                        }
                    };
                    let action = if existing.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(statement, ModelBuildOperation::Sequence { operation: op });
                    items.push(item(
                        statement,
                        action,
                        "Interaction",
                        Some(external_id),
                        Some(name),
                        "unified Behavior repository",
                    ));
                }
                ScriptOperation::Lifeline {
                    external_id,
                    interaction,
                    name,
                    represented_path,
                } => {
                    let key = external_key(namespace, external_id);
                    let existing = behavior.external_ids.get(&key).copied();
                    let path = represented_path
                        .iter()
                        .map(|value| element_reference(value, project, statement))
                        .collect::<Result<Vec<_>, _>>()?;
                    let op = if existing.is_some() {
                        if !matches!(existing, Some(BehaviorSemanticId::Lifeline(_))) {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("lifeline".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID has another Behavior semantic kind",
                            ));
                        }
                        SequenceBuildOperation::UpdateLifeline {
                            lifeline: external_reference(external_id, statement)?,
                            name: Some(name.clone()),
                            represented_path: Some(path),
                        }
                    } else {
                        SequenceBuildOperation::CreateLifeline {
                            external_id: external_id.clone(),
                            interaction: external_reference(interaction, statement)?,
                            name: name.clone(),
                            represented_path: path,
                        }
                    };
                    let action = if existing.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(statement, ModelBuildOperation::Sequence { operation: op });
                    items.push(item(
                        statement,
                        action,
                        "Lifeline",
                        Some(external_id),
                        Some(name),
                        "unified Behavior repository",
                    ));
                }
                ScriptOperation::Occurrence {
                    external_id,
                    interaction,
                    lifeline,
                    order,
                } => {
                    let key = external_key(namespace, external_id);
                    let existing = behavior.external_ids.get(&key).copied();
                    let op = if existing.is_some() {
                        if !matches!(existing, Some(BehaviorSemanticId::Occurrence(_))) {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("occurrence".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID has another Behavior semantic kind",
                            ));
                        }
                        SequenceBuildOperation::UpdateOccurrence {
                            occurrence: external_reference(external_id, statement)?,
                            lifeline: Some(external_reference(lifeline, statement)?),
                            order: Some(*order),
                        }
                    } else {
                        SequenceBuildOperation::CreateOccurrence {
                            external_id: external_id.clone(),
                            interaction: external_reference(interaction, statement)?,
                            lifeline: external_reference(lifeline, statement)?,
                            order: *order,
                        }
                    };
                    let action = if existing.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(statement, ModelBuildOperation::Sequence { operation: op });
                    items.push(item(
                        statement,
                        action,
                        "Occurrence",
                        Some(external_id),
                        None,
                        "unified Behavior repository",
                    ));
                }
                ScriptOperation::Message {
                    external_id,
                    interaction,
                    name,
                    sort,
                    send,
                    receive,
                    signature: signature_spec,
                    arguments,
                } => {
                    let key = external_key(namespace, external_id);
                    let existing = behavior.external_ids.get(&key).copied();
                    let signature_native = signature_spec
                        .as_ref()
                        .map(|value| signature(value, project, statement))
                        .transpose()?;
                    let send_ref = send
                        .as_deref()
                        .map(|value| external_reference(value, statement))
                        .transpose()?;
                    let receive_ref = receive
                        .as_deref()
                        .map(|value| external_reference(value, statement))
                        .transpose()?;
                    let op = if existing.is_some() {
                        if !matches!(existing, Some(BehaviorSemanticId::Message(_))) {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("message".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID has another Behavior semantic kind",
                            ));
                        }
                        SequenceBuildOperation::UpdateMessage {
                            message: external_reference(external_id, statement)?,
                            name: Some(name.clone()),
                            sort: Some(*sort),
                            send: Some(send_ref),
                            receive: Some(receive_ref),
                            signature: Some(signature_native),
                            arguments: Some(arguments.clone()),
                        }
                    } else {
                        SequenceBuildOperation::CreateMessage {
                            external_id: external_id.clone(),
                            interaction: external_reference(interaction, statement)?,
                            name: name.clone(),
                            sort: *sort,
                            send: send_ref,
                            receive: receive_ref,
                            signature: signature_native,
                            arguments: arguments.clone(),
                        }
                    };
                    let action = if existing.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(statement, ModelBuildOperation::Sequence { operation: op });
                    items.push(item(
                        statement,
                        action,
                        "Message",
                        Some(external_id),
                        Some(name),
                        "unified Behavior repository",
                    ));
                }
                ScriptOperation::Execution {
                    external_id,
                    interaction,
                    lifeline,
                    start,
                    finish,
                    behavior: execution_behavior,
                } => {
                    let key = external_key(namespace, external_id);
                    let existing = behavior.external_ids.get(&key).copied();
                    let behavior_ref = execution_behavior
                        .as_deref()
                        .map(|value| element_reference(value, project, statement))
                        .transpose()?;
                    let op = if existing.is_some() {
                        if !matches!(existing, Some(BehaviorSemanticId::Execution(_))) {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("execution".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID has another Behavior semantic kind",
                            ));
                        }
                        SequenceBuildOperation::UpdateExecution {
                            execution: external_reference(external_id, statement)?,
                            lifeline: Some(external_reference(lifeline, statement)?),
                            start: Some(external_reference(start, statement)?),
                            finish: Some(external_reference(finish, statement)?),
                            behavior: Some(behavior_ref),
                        }
                    } else {
                        SequenceBuildOperation::CreateExecution {
                            external_id: external_id.clone(),
                            interaction: external_reference(interaction, statement)?,
                            lifeline: external_reference(lifeline, statement)?,
                            start: external_reference(start, statement)?,
                            finish: external_reference(finish, statement)?,
                            behavior: behavior_ref,
                        }
                    };
                    let action = if existing.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(statement, ModelBuildOperation::Sequence { operation: op });
                    items.push(item(
                        statement,
                        action,
                        "ExecutionSpecification",
                        Some(external_id),
                        None,
                        "unified Behavior repository",
                    ));
                }
                ScriptOperation::CombinedFragment {
                    external_id,
                    interaction,
                    operator,
                    covered_lifelines,
                } => {
                    let key = external_key(namespace, external_id);
                    let existing = behavior.external_ids.get(&key).copied();
                    let covered = covered_lifelines
                        .iter()
                        .map(|value| external_reference(value, statement))
                        .collect::<Result<Vec<_>, _>>()?;
                    let op = if existing.is_some() {
                        if !matches!(existing, Some(BehaviorSemanticId::Fragment(_))) {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("combined_fragment".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID has another Behavior semantic kind",
                            ));
                        }
                        SequenceBuildOperation::UpdateFragment {
                            fragment: external_reference(external_id, statement)?,
                            operator: Some(*operator),
                            covered_lifelines: Some(covered),
                        }
                    } else {
                        SequenceBuildOperation::CreateFragment {
                            external_id: external_id.clone(),
                            interaction: external_reference(interaction, statement)?,
                            operator: *operator,
                            covered_lifelines: covered,
                        }
                    };
                    let action = if existing.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(statement, ModelBuildOperation::Sequence { operation: op });
                    items.push(item(
                        statement,
                        action,
                        "CombinedFragment",
                        Some(external_id),
                        None,
                        "unified Behavior repository",
                    ));
                }
                ScriptOperation::Operand {
                    external_id,
                    fragment,
                    guard,
                    start_order,
                    end_order,
                } => {
                    let key = external_key(namespace, external_id);
                    let existing = behavior.external_ids.get(&key).copied();
                    let op = if existing.is_some() {
                        if !matches!(existing, Some(BehaviorSemanticId::Operand(_))) {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("operand".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID has another Behavior semantic kind",
                            ));
                        }
                        SequenceBuildOperation::UpdateOperand {
                            operand: external_reference(external_id, statement)?,
                            guard: Some(guard.clone()),
                            start_order: Some(*start_order),
                            end_order: Some(*end_order),
                        }
                    } else {
                        SequenceBuildOperation::CreateOperand {
                            external_id: external_id.clone(),
                            fragment: external_reference(fragment, statement)?,
                            guard: guard.clone(),
                            start_order: *start_order,
                            end_order: *end_order,
                        }
                    };
                    let action = if existing.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(statement, ModelBuildOperation::Sequence { operation: op });
                    items.push(item(
                        statement,
                        action,
                        "InteractionOperand",
                        Some(external_id),
                        None,
                        "unified Behavior repository",
                    ));
                }
                ScriptOperation::StateInvariant {
                    external_id,
                    interaction,
                    lifeline,
                    order,
                    constraint,
                } => {
                    let key = external_key(namespace, external_id);
                    let existing = behavior.external_ids.get(&key).copied();
                    let op = if existing.is_some() {
                        if !matches!(existing, Some(BehaviorSemanticId::Invariant(_))) {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("state_invariant".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID has another Behavior semantic kind",
                            ));
                        }
                        SequenceBuildOperation::UpdateInvariant {
                            invariant: external_reference(external_id, statement)?,
                            lifeline: Some(external_reference(lifeline, statement)?),
                            order: Some(*order),
                            constraint: Some(constraint.clone()),
                        }
                    } else {
                        SequenceBuildOperation::CreateInvariant {
                            external_id: external_id.clone(),
                            interaction: external_reference(interaction, statement)?,
                            lifeline: external_reference(lifeline, statement)?,
                            order: *order,
                            constraint: constraint.clone(),
                        }
                    };
                    let action = if existing.is_some() {
                        ModelScriptAction::Update
                    } else {
                        ModelScriptAction::Create
                    };
                    push_specialized!(statement, ModelBuildOperation::Sequence { operation: op });
                    items.push(item(
                        statement,
                        action,
                        "StateInvariant",
                        Some(external_id),
                        None,
                        "unified Behavior repository",
                    ));
                }
                ScriptOperation::ParametricMetadata {
                    element,
                    constraint_expression,
                    quantity_kind_external_id,
                    unit_external_id,
                    quantity_dimension,
                    unit_symbol,
                    unit_scale_to_base,
                } => {
                    push_specialized!(
                        statement,
                        ModelBuildOperation::Parametric {
                            operation: ParametricBuildOperation::UpdateElementSemantics {
                                element: element_reference(element, project, statement)?,
                                constraint_expression: constraint_expression.clone(),
                                quantity_kind_external_id: quantity_kind_external_id
                                    .clone()
                                    .map(Some),
                                unit_external_id: unit_external_id.clone().map(Some),
                                quantity_dimension: quantity_dimension.clone().map(Some),
                                unit_symbol: unit_symbol.clone().map(Some),
                                unit_scale_to_base: *unit_scale_to_base
                            }
                        }
                    );
                    items.push(item(
                        statement,
                        ModelScriptAction::Update,
                        "ParametricMetadata",
                        None,
                        Some(element),
                        "native parametric semantic update",
                    ));
                }
                ScriptOperation::Binding {
                    external_id,
                    name,
                    owner,
                    source,
                    target,
                } => {
                    let existing = relationship_by_external(project, namespace, external_id);
                    let source_native = binding_endpoint(source, project, statement)?;
                    let target_native = binding_endpoint(target, project, statement)?;
                    let op = if let Some(record) = existing.first() {
                        if record.kind != RelationshipKind::BindingConnector {
                            return Err(diag(
                                "",
                                Some(statement),
                                Some("binding".into()),
                                Some(external_id.clone()),
                                "WRONG_KIND_COLLISION",
                                "External ID does not identify a BindingConnector",
                            ));
                        }
                        ParametricBuildOperation::UpdateBinding {
                            relationship: BuildReference::External(external_id.clone()),
                            name: Some(name.clone()),
                            owner: Some(element_reference(owner, project, statement)?),
                            source: Some(source_native),
                            target: Some(target_native),
                        }
                    } else {
                        ParametricBuildOperation::CreateBinding {
                            external_id: external_id.clone(),
                            name: name.clone(),
                            owner: element_reference(owner, project, statement)?,
                            source: source_native,
                            target: target_native,
                        }
                    };
                    let action = if existing.is_empty() {
                        ModelScriptAction::Create
                    } else {
                        ModelScriptAction::Update
                    };
                    push_specialized!(statement, ModelBuildOperation::Parametric { operation: op });
                    items.push(item(
                        statement,
                        action,
                        "BindingConnector",
                        Some(external_id),
                        Some(name),
                        "native parametric build operation",
                    ));
                }
            }
            Ok(())
        })();
        if let Err(mut failure) = compiled {
            failure.script = script_name.into();
            diagnostics.push(failure);
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    let mut operations = ordinary;
    operations.extend(specialized);
    let mut statements = ordinary_statements;
    statements.extend(specialized_statements);
    Ok(CompiledScript {
        document: document.clone(),
        plan: ModelBuildPlan {
            source_namespace: document.source_namespace.clone(),
            operations,
        },
        plan_statements: statements,
        items,
    })
}

fn clone_states(
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> Result<(WorkspaceState, ActivityWorkspaceState), String> {
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone();
    let diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let ibd_diagrams = workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")?
        .clone();
    let behavior = workspace
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?
        .clone();
    let behavior_diagrams = workspace
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .clone();
    let current_file = workspace
        .current_file
        .lock()
        .map_err(|_| "project path lock poisoned")?
        .clone();
    let reqif_exchange = workspace
        .reqif_exchange
        .lock()
        .map_err(|_| "ReqIF exchange lock poisoned")?
        .clone();
    let activity_repository = activity
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?
        .clone();
    let activity_diagrams = activity
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?
        .clone();
    Ok((
        WorkspaceState {
            project: std::sync::Mutex::new(project),
            diagrams: std::sync::Mutex::new(diagrams),
            ibd_diagrams: std::sync::Mutex::new(ibd_diagrams),
            behavior: std::sync::Mutex::new(behavior),
            behavior_diagrams: std::sync::Mutex::new(behavior_diagrams),
            current_file: std::sync::Mutex::new(current_file),
            reqif_exchange: std::sync::Mutex::new(reqif_exchange),
        },
        ActivityWorkspaceState {
            repository: std::sync::Mutex::new(activity_repository),
            diagrams: std::sync::Mutex::new(activity_diagrams),
        },
    ))
}

fn resolve_candidate_element(
    project: &Project,
    namespace: &str,
    token: &str,
) -> Result<ElementId, String> {
    let token = token.trim();
    if token == "$root" {
        return Ok(project.root_id);
    }
    if let Some(qname) = token.strip_prefix("qname:") {
        let ids = project
            .elements
            .values()
            .filter(|element| project.qualified_name(element.id).as_deref() == Ok(qname))
            .map(|element| element.id)
            .collect::<Vec<_>>();
        return match ids.as_slice() {
            [id] => Ok(*id),
            [] => Err(format!("exact qualified name was not found: {qname}")),
            _ => Err(format!("exact qualified name is ambiguous: {qname}")),
        };
    }
    let external = script_external_token(token);
    let key = external_key(namespace, external);
    let ids = project
        .elements
        .values()
        .filter(|element| element.external_id == key)
        .map(|element| element.id)
        .collect::<Vec<_>>();
    match ids.as_slice() {
        [id] => Ok(*id),
        [] => Err(format!("External ID was not found: {external}")),
        _ => Err(format!("External ID is ambiguous: {external}")),
    }
}

fn presentation_family(value: &str) -> Result<&'static str, String> {
    match value.trim().to_ascii_lowercase().replace('_', "-").as_str() {
        "bdd" => Ok("bdd"),
        "ibd" => Ok("ibd"),
        "requirement" | "requirements" => Ok("requirement"),
        "use-case" | "usecase" | "use case" => Ok("use-case"),
        "package" => Ok("package"),
        "activity" => Ok("activity"),
        "state-machine" | "statemachine" | "state machine" => Ok("state-machine"),
        "sequence" => Ok("sequence"),
        "parametric" => Ok("parametric"),
        other => Err(format!("unsupported diagram family: {other}")),
    }
}

fn bdd_family_accepts(family: &str, kind: &ElementKind) -> bool {
    match family {
        "bdd" => !matches!(
            kind,
            ElementKind::Model
                | ElementKind::Package
                | ElementKind::PartProperty
                | ElementKind::ReferenceProperty
                | ElementKind::ValueProperty
                | ElementKind::FlowProperty
                | ElementKind::ConstraintProperty
                | ElementKind::ConstraintParameter
                | ElementKind::ProxyPort
                | ElementKind::FullPort
                | ElementKind::Operation
                | ElementKind::Parameter
                | ElementKind::Reception
                | ElementKind::EnumerationLiteral
                | ElementKind::Slot
        ),
        "requirement" => matches!(
            kind,
            ElementKind::Requirement
                | ElementKind::TestCase
                | ElementKind::Block
                | ElementKind::AssociationBlock
                | ElementKind::InterfaceBlock
                | ElementKind::ConstraintBlock
        ),
        "use-case" => matches!(kind, ElementKind::Actor | ElementKind::UseCase),
        "package" => !matches!(
            kind,
            ElementKind::PartProperty
                | ElementKind::ReferenceProperty
                | ElementKind::ValueProperty
                | ElementKind::FlowProperty
                | ElementKind::ConstraintProperty
                | ElementKind::ConstraintParameter
                | ElementKind::ProxyPort
                | ElementKind::FullPort
                | ElementKind::Operation
                | ElementKind::Parameter
                | ElementKind::Reception
                | ElementKind::EnumerationLiteral
                | ElementKind::Slot
        ),
        "parametric" => matches!(
            kind,
            ElementKind::ConstraintProperty | ElementKind::ValueProperty
        ),
        _ => false,
    }
}

fn relationship_presentable_on_family(family: &str, kind: &RelationshipKind) -> bool {
    match family {
        "package" => matches!(
            kind,
            RelationshipKind::PackageImport
                | RelationshipKind::ElementImport
                | RelationshipKind::Dependency
        ),
        "use-case" => matches!(
            kind,
            RelationshipKind::Association
                | RelationshipKind::Include
                | RelationshipKind::Extend
                | RelationshipKind::Generalization
        ),
        "parametric" => *kind == RelationshipKind::BindingConnector,
        _ => !matches!(
            kind,
            RelationshipKind::Connector
                | RelationshipKind::ItemFlow
                | RelationshipKind::BindingConnector
        ),
    }
}

fn populate_bdd_like(
    diagram: &mut BddDiagram,
    project: &Project,
    family: &str,
    scope: ElementId,
) -> Result<(), String> {
    let mut x = 100.0;
    let mut y = 100.0;
    for element in project
        .children(scope)
        .filter(|element| bdd_family_accepts(family, &element.kind))
    {
        let constraint = family == "parametric" && element.kind == ElementKind::ConstraintProperty;
        let mut node = DiagramNode {
            id: uuid::Uuid::new_v4().to_string(),
            element_id: element.id.to_string(),
            x,
            y,
            width: if constraint {
                260.0
            } else if family == "parametric" {
                220.0
            } else {
                190.0
            },
            height: if constraint {
                170.0
            } else if family == "parametric" {
                72.0
            } else {
                110.0
            },
            actor_notation: None,
            parameter_presentations: Vec::new(),
        };
        if family == "parametric" {
            super::parametrics::sync_parameter_presentations(&mut node, project)?;
        }
        diagram.nodes.push(node);
        x += 260.0;
        if x > 900.0 {
            x = 100.0;
            y += 180.0;
        }
    }

    let node_for = |id: ElementId| {
        diagram
            .nodes
            .iter()
            .find(|node| node.element_id == id.to_string())
            .map(|node| node.id.clone())
    };
    let parametric_endpoint = |role_id: ElementId, parameter_id: Option<ElementId>| {
        let role = diagram
            .nodes
            .iter()
            .find(|node| node.element_id == role_id.to_string())?;
        match parameter_id {
            None => Some(role.id.clone()),
            Some(parameter_id) => role
                .parameter_presentations
                .iter()
                .find(|parameter| parameter.parameter_id == parameter_id.to_string())
                .map(|parameter| parameter.id.clone()),
        }
    };

    for relationship in project
        .relationships
        .values()
        .filter(|relationship| relationship_presentable_on_family(family, &relationship.kind))
    {
        let endpoints = if family == "parametric" {
            let binding = relationship
                .binding
                .as_ref()
                .ok_or("BindingConnector has no semantic endpoint details")?;
            (
                parametric_endpoint(binding.source.role_id, binding.source.parameter_id),
                parametric_endpoint(binding.target.role_id, binding.target.parameter_id),
            )
        } else {
            (
                node_for(relationship.source_id),
                node_for(relationship.target_id),
            )
        };
        if let (Some(source), Some(target)) = endpoints {
            diagram.edges.push(DiagramEdge {
                id: uuid::Uuid::new_v4().to_string(),
                relationship_id: relationship.id.to_string(),
                source_node_id: source,
                target_node_id: target,
                points: vec![
                    DiagramPoint { x: 0.0, y: 0.0 },
                    DiagramPoint { x: 1.0, y: 1.0 },
                ],
                label_anchor: None,
            });
        }
    }
    Ok(())
}

fn stable_script_diagram_id(namespace: &str, external_id: &str) -> String {
    fn hash(seed: u64, bytes: &[u8]) -> u64 {
        bytes.iter().fold(seed, |value, byte| {
            (value ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
    }
    let key = format!("systems-modeler:model-script:{namespace}:{external_id}");
    let mut bytes = [0_u8; 16];
    bytes[..8].copy_from_slice(&hash(0xcbf29ce484222325, key.as_bytes()).to_be_bytes());
    bytes[8..].copy_from_slice(&hash(0x84222325cbf29ce4, key.as_bytes()).to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes).to_string()
}

fn create_script_diagram(
    diagram: &ScriptDiagram,
    namespace: &str,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> Result<String, String> {
    let family = presentation_family(&diagram.family)?;
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let owner = resolve_candidate_element(&project, namespace, &diagram.owner)?;
    let context = diagram
        .context
        .as_deref()
        .map(|value| resolve_candidate_element(&project, namespace, value))
        .transpose()?;
    match family {
        "bdd" | "requirement" | "use-case" | "package" | "parametric" => {
            let id = stable_script_diagram_id(namespace, &diagram.external_id);
            let scope = if family == "parametric" {
                context.unwrap_or(owner)
            } else {
                owner
            };
            let mut native = BddDiagram {
                id: id.clone(),
                name: diagram.name.clone(),
                owner_id: owner.to_string(),
                family: family.into(),
                semantic_context_id: context.map(|value| value.to_string()),
                subject_boundary: None,
                nodes: Vec::new(),
                edges: Vec::new(),
            };
            if diagram.populate {
                populate_bdd_like(&mut native, &project, family, scope)?;
            }
            let mut diagrams = workspace
                .diagrams
                .lock()
                .map_err(|_| "diagram lock poisoned")?;
            diagrams.retain(|existing| existing.id != id);
            diagrams.push(native);
            Ok(id)
        }
        "ibd" => {
            let context = context.ok_or("IBD script diagram requires context")?;
            let id = stable_script_diagram_id(namespace, &diagram.external_id);
            let mut native = IbdDiagram {
                id: id.clone(),
                name: diagram.name.clone(),
                context_block_id: context.to_string(),
                owner_id: owner.to_string(),
                properties: Vec::new(),
                boundary_ports: Vec::new(),
                connectors: Vec::new(),
            };
            if diagram.populate {
                let mut x = 120.0;
                let mut y = 120.0;
                for feature in project.children(context) {
                    match feature.kind {
                        ElementKind::PartProperty | ElementKind::ReferenceProperty => {
                            // A semantic connector end may terminate at a port owned by the
                            // property's type. Populate those native nested-port presentations
                            // up front so presentation endpoints can reconstruct ConnectorEnd
                            // exactly rather than falling back to the owning property box.
                            let typed_ports = feature
                                .type_id
                                .map(|type_id| {
                                    project
                                        .children(type_id)
                                        .filter(|candidate| candidate.is_port())
                                        .map(|candidate| candidate.id)
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default();
                            let port_rows = typed_ports.len().div_ceil(2);
                            let height = 100.0_f64.max(52.0 + port_rows as f64 * 28.0);
                            let ports = typed_ports
                                .into_iter()
                                .enumerate()
                                .map(|(index, port_id)| {
                                    let right_side = index % 2 == 0;
                                    let row = index / 2;
                                    IbdPortPresentation {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        element_id: port_id.to_string(),
                                        property_path: vec![feature.id.to_string()],
                                        x: if right_side { x + 190.0 } else { x },
                                        y: y + 34.0 + row as f64 * 28.0,
                                        size: 16.0,
                                    }
                                })
                                .collect();
                            native.properties.push(IbdPropertyPresentation {
                                id: uuid::Uuid::new_v4().to_string(),
                                element_id: feature.id.to_string(),
                                property_path: vec![feature.id.to_string()],
                                x,
                                y,
                                width: 190.0,
                                height,
                                ports,
                            });
                            x += 240.0;
                            if x > 780.0 {
                                x = 120.0;
                                y += height + 80.0;
                            }
                        }
                        ElementKind::ProxyPort | ElementKind::FullPort => {
                            native.boundary_ports.push(IbdPortPresentation {
                                id: uuid::Uuid::new_v4().to_string(),
                                element_id: feature.id.to_string(),
                                property_path: Vec::new(),
                                x: 55.0,
                                y: 100.0 + native.boundary_ports.len() as f64 * 70.0,
                                size: 16.0,
                            })
                        }
                        _ => {}
                    }
                }
                for relationship in project
                    .relationships
                    .values()
                    .filter(|relationship| relationship.kind == RelationshipKind::Connector)
                {
                    let Some(connector) = relationship
                        .connector
                        .as_ref()
                        .filter(|connector| connector.context_id == context)
                    else {
                        continue;
                    };
                    let endpoint = |end: &systems_modeler_core::ConnectorEnd| -> Option<String> {
                        let path = end
                            .property_path
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>();
                        match end.port_id {
                            Some(port_id) if path.is_empty() => native
                                .boundary_ports
                                .iter()
                                .find(|presentation| {
                                    presentation.element_id == port_id.to_string()
                                        && presentation.property_path.is_empty()
                                })
                                .map(|presentation| presentation.id.clone()),
                            Some(port_id) => native
                                .properties
                                .iter()
                                .flat_map(|property| property.ports.iter())
                                .find(|presentation| {
                                    presentation.element_id == port_id.to_string()
                                        && presentation.property_path == path
                                })
                                .map(|presentation| presentation.id.clone()),
                            None => native
                                .properties
                                .iter()
                                .find(|presentation| {
                                    presentation.element_id == end.role_id.to_string()
                                        && presentation.property_path == path
                                })
                                .map(|presentation| presentation.id.clone()),
                        }
                    };
                    if let (Some(source), Some(target)) =
                        (endpoint(&connector.source), endpoint(&connector.target))
                    {
                        native.connectors.push(IbdConnectorPresentation {
                            id: uuid::Uuid::new_v4().to_string(),
                            relationship_id: relationship.id.to_string(),
                            source_presentation_id: source,
                            target_presentation_id: target,
                            points: vec![
                                DiagramPoint { x: 0.0, y: 0.0 },
                                DiagramPoint { x: 1.0, y: 1.0 },
                            ],
                            label_anchor: None,
                        });
                    }
                }
            }
            let mut diagrams = workspace
                .ibd_diagrams
                .lock()
                .map_err(|_| "IBD lock poisoned")?;
            diagrams.retain(|existing| existing.id != id);
            diagrams.push(native);
            Ok(id)
        }
        "activity" => {
            let semantic = diagram
                .semantic
                .as_deref()
                .ok_or("Activity script diagram requires semantic External ID")?;
            let key = external_key(namespace, script_external_token(semantic));
            let repository = activity
                .repository
                .lock()
                .map_err(|_| "Activity repository lock poisoned")?;
            let activity_record = repository
                .activities
                .values()
                .find(|record| record.external_id == key)
                .ok_or("Activity semantic External ID was not found")?
                .clone();
            drop(repository);
            let id = stable_script_diagram_id(namespace, &diagram.external_id);
            let mut native = ActivityDiagram {
                id: id.clone(),
                name: diagram.name.clone(),
                owner_id: owner.to_string(),
                activity_id: activity_record.id.to_string(),
                nodes: Vec::new(),
                edges: Vec::new(),
            };
            if diagram.populate {
                let mut x = 100.0;
                let mut y = 100.0;
                for node in &activity_record.nodes {
                    let (width, height) = super::activity_workspace::activity_node_size(&node.kind);
                    native.nodes.push(ActivityDiagramNode {
                        id: uuid::Uuid::new_v4().to_string(),
                        activity_node_id: node.id.to_string(),
                        x,
                        y,
                        width,
                        height,
                    });
                    x += 220.0;
                    if x > 900.0 {
                        x = 100.0;
                        y += 160.0;
                    }
                }
                let presentation = |node_id: systems_modeler_core::ActivityNodeId| {
                    native
                        .nodes
                        .iter()
                        .find(|p| p.activity_node_id == node_id.to_string())
                        .map(|p| p.id.clone())
                };
                let owner_for_pin = |pin| {
                    activity_record.nodes.iter().find(|node| matches!(&node.kind, ActivityNodeKind::Action(action) if action.pins.iter().any(|candidate| candidate.id == pin))).map(|node| node.id)
                };
                for edge in &activity_record.edges {
                    let source_node = match edge.source {
                        ActivityEndpoint::Node(id) => Some(id),
                        ActivityEndpoint::Pin(id) => owner_for_pin(id),
                    };
                    let target_node = match edge.target {
                        ActivityEndpoint::Node(id) => Some(id),
                        ActivityEndpoint::Pin(id) => owner_for_pin(id),
                    };
                    if let (Some(source), Some(target)) = (
                        source_node.and_then(presentation),
                        target_node.and_then(presentation),
                    ) {
                        native.edges.push(ActivityDiagramEdge {
                            id: uuid::Uuid::new_v4().to_string(),
                            activity_edge_id: edge.id.to_string(),
                            source_node_id: source,
                            target_node_id: target,
                            points: vec![
                                DiagramPoint { x: 0.0, y: 0.0 },
                                DiagramPoint { x: 1.0, y: 1.0 },
                            ],
                            label_anchor: None,
                        });
                    }
                }
            }
            let mut diagrams = activity
                .diagrams
                .lock()
                .map_err(|_| "Activity diagram lock poisoned")?;
            diagrams.retain(|existing| existing.id != id);
            diagrams.push(native);
            Ok(id)
        }
        "state-machine" | "sequence" => {
            let semantic = diagram
                .semantic
                .as_deref()
                .ok_or("behavior script diagram requires semantic External ID")?;
            let repository = workspace
                .behavior
                .lock()
                .map_err(|_| "behavior lock poisoned")?
                .clone();
            let identity = if family == "state-machine" {
                repository
                    .state_machines
                    .values()
                    .find(|record| {
                        record.external_id
                            == external_key(namespace, script_external_token(semantic))
                    })
                    .map(|record| {
                        (
                            record.id.to_string(),
                            record.context_id,
                            BehaviorDiagramKind::StateMachine,
                        )
                    })
            } else {
                repository
                    .interactions
                    .values()
                    .find(|record| {
                        record.external_id
                            == external_key(namespace, script_external_token(semantic))
                    })
                    .map(|record| {
                        (
                            record.id.to_string(),
                            record.context_id,
                            BehaviorDiagramKind::Sequence,
                        )
                    })
            }
            .ok_or("behavior semantic External ID was not found")?;
            let id = stable_script_diagram_id(namespace, &diagram.external_id);
            let mut native = BehaviorDiagram {
                id: id.clone(),
                name: diagram.name.clone(),
                owner_id: owner.to_string(),
                context_id: identity.1.to_string(),
                kind: identity.2,
                semantic_id: identity.0.clone(),
                state_nodes: Vec::new(),
                lifelines: Vec::new(),
                edge_routes: Vec::new(),
                hidden_semantic_ids: Vec::new(),
                presentation_copies: Vec::new(),
            };
            if diagram.populate {
                if family == "state-machine" {
                    let machine = repository
                        .state_machines
                        .values()
                        .find(|record| record.id.to_string() == identity.0)
                        .unwrap();
                    fn collect(
                        regions: &[systems_modeler_core::behavior::Region],
                        nodes: &mut Vec<StateNodePresentation>,
                        edges: &mut Vec<BehaviorEdgePresentation>,
                        x: &mut f64,
                        y: &mut f64,
                    ) {
                        for region in regions {
                            for vertex in &region.vertices {
                                nodes.push(StateNodePresentation {
                                    vertex_id: vertex.id.to_string(),
                                    x: *x,
                                    y: *y,
                                    width: 160.0,
                                    height: 90.0,
                                });
                                *x += 220.0;
                                if *x > 900.0 {
                                    *x = 100.0;
                                    *y += 150.0;
                                }
                                if let VertexKind::State(state) = &vertex.kind {
                                    collect(&state.regions, nodes, edges, x, y);
                                }
                            }
                            for transition in &region.transitions {
                                edges.push(BehaviorEdgePresentation {
                                    semantic_id: transition.id.to_string(),
                                    points: vec![
                                        DiagramPoint { x: 0.0, y: 0.0 },
                                        DiagramPoint { x: 1.0, y: 1.0 },
                                    ],
                                    label_anchor: None,
                                });
                            }
                        }
                    }
                    let mut x = 100.0;
                    let mut y = 100.0;
                    collect(
                        &machine.regions,
                        &mut native.state_nodes,
                        &mut native.edge_routes,
                        &mut x,
                        &mut y,
                    );
                } else {
                    let interaction = repository
                        .interactions
                        .values()
                        .find(|record| record.id.to_string() == identity.0)
                        .unwrap();
                    native.lifelines = interaction
                        .lifelines
                        .iter()
                        .enumerate()
                        .map(|(index, lifeline)| LifelinePresentation {
                            lifeline_id: lifeline.id.to_string(),
                            x: 140.0 + index as f64 * 220.0,
                            timeline_start_y: 102.0,
                            timeline_end_y: 840.0,
                        })
                        .collect();
                    native.edge_routes = interaction
                        .messages
                        .iter()
                        .map(|message| BehaviorEdgePresentation {
                            semantic_id: message.id.to_string(),
                            points: vec![
                                DiagramPoint { x: 0.0, y: 0.0 },
                                DiagramPoint { x: 1.0, y: 1.0 },
                            ],
                            label_anchor: None,
                        })
                        .collect();
                }
            }
            let mut diagrams = workspace
                .behavior_diagrams
                .lock()
                .map_err(|_| "behavior diagram lock poisoned")?;
            diagrams.retain(|existing| existing.id != id);
            diagrams.push(native);
            Ok(id)
        }
        _ => unreachable!(),
    }
}

fn layout_and_route(
    family: &str,
    diagram_id: &str,
    clean_layout: bool,
    route: bool,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> Result<(), String> {
    if clean_layout {
        match family {
            "bdd" | "requirement" | "use-case" | "package" => {
                super::layout_bdd_with_bounds(diagram_id, workspace, None)?;
            }
            "parametric" => {
                super::parametrics::layout_parametric_with_bounds(diagram_id, workspace, None)?;
            }
            "ibd" => {
                super::ibd::layout_ibd_with_bounds(diagram_id, workspace, None)?;
            }
            "activity" => {
                super::activity_mutation::layout_activity_with_bounds(diagram_id, activity, None)?;
            }
            "state-machine" | "sequence" => {
                super::behavior_workspace::layout_behavior_with_bounds(
                    diagram_id, workspace, None,
                )?;
            }
            _ => {}
        }
    }
    if route {
        match family {
            "bdd" | "requirement" | "use-case" | "package" => {
                super::route_bdd_with_bounds(diagram_id, workspace, None)?;
            }
            "parametric" => {
                super::parametrics::route_parametric_with_bounds(diagram_id, workspace, None)?;
            }
            "ibd" => {
                super::ibd::route_ibd_with_bounds(diagram_id, workspace, None)?;
            }
            "activity" => {
                super::activity_mutation::route_activity_with_bounds(diagram_id, activity, None)?;
            }
            "state-machine" | "sequence" => {
                super::behavior_workspace::route_behavior_with_bounds(diagram_id, workspace, None)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn build_candidate(
    script_name: &str,
    source: &str,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> Result<(CompiledScript, WorkspaceState, ActivityWorkspaceState), ModelScriptPreview> {
    let project = workspace
        .project
        .lock()
        .map_err(|_| ModelScriptPreview {
            host: SCRIPT_HOST,
            applied: false,
            source_namespace: String::new(),
            items: Vec::new(),
            diagnostics: vec![diag(
                script_name,
                None,
                Some("lock".into()),
                None,
                "LOCK_FAILURE",
                "project lock poisoned",
            )],
        })?
        .clone()
        .ok_or_else(|| ModelScriptPreview {
            host: SCRIPT_HOST,
            applied: false,
            source_namespace: String::new(),
            items: Vec::new(),
            diagnostics: vec![diag(
                script_name,
                None,
                Some("run".into()),
                None,
                "NO_PROJECT",
                "no project open",
            )],
        })?;
    let activities = activity
        .repository
        .lock()
        .map_err(|_| ModelScriptPreview {
            host: SCRIPT_HOST,
            applied: false,
            source_namespace: String::new(),
            items: Vec::new(),
            diagnostics: vec![diag(
                script_name,
                None,
                Some("lock".into()),
                None,
                "LOCK_FAILURE",
                "Activity repository lock poisoned",
            )],
        })?
        .clone();
    let behavior = workspace
        .behavior
        .lock()
        .map_err(|_| ModelScriptPreview {
            host: SCRIPT_HOST,
            applied: false,
            source_namespace: String::new(),
            items: Vec::new(),
            diagnostics: vec![diag(
                script_name,
                None,
                Some("lock".into()),
                None,
                "LOCK_FAILURE",
                "behavior lock poisoned",
            )],
        })?
        .clone();
    let compiled = compile_script(script_name, source, &project, &activities, &behavior).map_err(
        |diagnostics| ModelScriptPreview {
            host: SCRIPT_HOST,
            applied: false,
            source_namespace: String::new(),
            items: Vec::new(),
            diagnostics,
        },
    )?;
    let (candidate_workspace, candidate_activity) =
        clone_states(workspace, activity).map_err(|reason| ModelScriptPreview {
            host: SCRIPT_HOST,
            applied: false,
            source_namespace: compiled.document.source_namespace.clone(),
            items: compiled.items.clone(),
            diagnostics: vec![diag(
                script_name,
                None,
                Some("candidate".into()),
                None,
                "LOCK_FAILURE",
                reason,
            )],
        })?;
    if let Err(preview) =
        apply_unified_model_build(&compiled.plan, &candidate_workspace, &candidate_activity)
    {
        let diagnostics = preview
            .diagnostics
            .into_iter()
            .map(|failure| {
                let statement = failure
                    .operation
                    .and_then(|operation| compiled.plan_statements.get(operation).copied());
                let mut d = diag(
                    script_name,
                    statement,
                    Some("ModelBuildPlan".into()),
                    None,
                    failure.code,
                    failure.message,
                );
                if let Some(statement) = statement {
                    d.external_id = compiled
                        .items
                        .iter()
                        .find(|item| item.statement == statement)
                        .and_then(|item| item.external_id.clone());
                    d.semantic_name = compiled
                        .items
                        .iter()
                        .find(|item| item.statement == statement)
                        .and_then(|item| item.semantic_name.clone());
                }
                d
            })
            .collect();
        return Err(ModelScriptPreview {
            host: SCRIPT_HOST,
            applied: false,
            source_namespace: compiled.document.source_namespace.clone(),
            items: compiled.items.clone(),
            diagnostics,
        });
    }
    for (offset, diagram) in compiled.document.diagrams.iter().enumerate() {
        let statement = compiled.document.operations.len() + offset + 1;
        let family = presentation_family(&diagram.family).map_err(|reason| ModelScriptPreview {
            host: SCRIPT_HOST,
            applied: false,
            source_namespace: compiled.document.source_namespace.clone(),
            items: compiled.items.clone(),
            diagnostics: vec![diag(
                script_name,
                Some(statement),
                Some("createDiagram".into()),
                Some(diagram.external_id.clone()),
                "DIAGRAM_FAMILY_INVALID",
                reason,
            )],
        })?;
        let id = create_script_diagram(
            diagram,
            &compiled.document.source_namespace,
            &candidate_workspace,
            &candidate_activity,
        )
        .map_err(|reason| ModelScriptPreview {
            host: SCRIPT_HOST,
            applied: false,
            source_namespace: compiled.document.source_namespace.clone(),
            items: compiled.items.clone(),
            diagnostics: vec![diag(
                script_name,
                Some(statement),
                Some("createDiagram".into()),
                Some(diagram.external_id.clone()),
                "DIAGRAM_BUILD_FAILED",
                reason,
            )],
        })?;
        layout_and_route(
            family,
            &id,
            diagram.clean_layout,
            diagram.route,
            &candidate_workspace,
            &candidate_activity,
        )
        .map_err(|reason| ModelScriptPreview {
            host: SCRIPT_HOST,
            applied: false,
            source_namespace: compiled.document.source_namespace.clone(),
            items: compiled.items.clone(),
            diagnostics: vec![diag(
                script_name,
                Some(statement),
                Some("layout/route".into()),
                Some(diagram.external_id.clone()),
                "DIAGRAM_LAYOUT_FAILED",
                reason,
            )],
        })?;
    }
    super::portable_interchange::portable_from_states(&candidate_workspace, &candidate_activity)
        .map_err(|reason| ModelScriptPreview {
            host: SCRIPT_HOST,
            applied: false,
            source_namespace: compiled.document.source_namespace.clone(),
            items: compiled.items.clone(),
            diagnostics: vec![diag(
                script_name,
                None,
                Some("validate".into()),
                None,
                "AUTHORED_STATE_VALIDATION",
                reason,
            )],
        })?;
    Ok((compiled, candidate_workspace, candidate_activity))
}

fn commit_candidate(
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
    candidate_workspace: &WorkspaceState,
    candidate_activity: &ActivityWorkspaceState,
) -> Result<(), String> {
    let next_project = candidate_workspace
        .project
        .lock()
        .map_err(|_| "candidate project lock poisoned")?
        .clone();
    let next_diagrams = candidate_workspace
        .diagrams
        .lock()
        .map_err(|_| "candidate diagram lock poisoned")?
        .clone();
    let next_ibd = candidate_workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "candidate IBD lock poisoned")?
        .clone();
    let next_behavior = candidate_workspace
        .behavior
        .lock()
        .map_err(|_| "candidate behavior lock poisoned")?
        .clone();
    let next_behavior_diagrams = candidate_workspace
        .behavior_diagrams
        .lock()
        .map_err(|_| "candidate behavior diagram lock poisoned")?
        .clone();
    let next_activities = candidate_activity
        .repository
        .lock()
        .map_err(|_| "candidate Activity repository lock poisoned")?
        .clone();
    let next_activity_diagrams = candidate_activity
        .diagrams
        .lock()
        .map_err(|_| "candidate Activity diagram lock poisoned")?
        .clone();
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?;
    let mut ibd = workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")?;
    let mut behavior = workspace
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let mut behavior_diagrams = workspace
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let mut activities = activity
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let mut activity_diagrams = activity
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    *project = next_project;
    *diagrams = next_diagrams;
    *ibd = next_ibd;
    *behavior = next_behavior;
    *behavior_diagrams = next_behavior_diagrams;
    *activities = next_activities;
    *activity_diagrams = next_activity_diagrams;
    Ok(())
}

fn preview_impl(
    script_name: &str,
    source: &str,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> ModelScriptPreview {
    match build_candidate(script_name, source, workspace, activity) {
        Ok((compiled, _, _)) => {
            let mut items = compiled.items.clone();
            for (offset, diagram) in compiled.document.diagrams.iter().enumerate() {
                items.push(item(
                    compiled.document.operations.len() + offset + 1,
                    ModelScriptAction::Create,
                    format!("Diagram::{}", diagram.family),
                    Some(&diagram.external_id),
                    Some(&diagram.name),
                    "native presentation + populate/Clean Layout/route",
                ));
            }
            ModelScriptPreview {
                host: SCRIPT_HOST,
                applied: false,
                source_namespace: compiled.document.source_namespace,
                items,
                diagnostics: Vec::new(),
            }
        }
        Err(preview) => preview,
    }
}

#[tauri::command]
pub fn preview_model_script(
    script_name: String,
    source: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
) -> ModelScriptPreview {
    preview_impl(&script_name, &source, &workspace, &activity)
}

#[tauri::command]
pub fn apply_model_script(
    script_name: String,
    source: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, super::history::HistoryState>,
) -> ModelScriptPreview {
    let (compiled, candidate_workspace, candidate_activity) =
        match build_candidate(&script_name, &source, &workspace, &activity) {
            Ok(value) => value,
            Err(preview) => return preview,
        };
    if let Err(reason) = super::history::checkpoint_states(&workspace, &activity, &history) {
        return ModelScriptPreview {
            host: SCRIPT_HOST,
            applied: false,
            source_namespace: compiled.document.source_namespace.clone(),
            items: compiled.items.clone(),
            diagnostics: vec![diag(
                &script_name,
                None,
                Some("history".into()),
                None,
                "HISTORY_CHECKPOINT_FAILED",
                reason,
            )],
        };
    }
    if let Err(reason) = commit_candidate(
        &workspace,
        &activity,
        &candidate_workspace,
        &candidate_activity,
    ) {
        return ModelScriptPreview {
            host: SCRIPT_HOST,
            applied: false,
            source_namespace: compiled.document.source_namespace.clone(),
            items: compiled.items.clone(),
            diagnostics: vec![diag(
                &script_name,
                None,
                Some("commit".into()),
                None,
                "ATOMIC_COMMIT_FAILED",
                reason,
            )],
        };
    }
    let mut preview = preview_impl(&script_name, &source, &workspace, &activity);
    preview.applied = preview.valid();
    preview
}

#[cfg(test)]
mod tests {
    use super::*;

    fn states() -> (WorkspaceState, ActivityWorkspaceState) {
        let workspace = WorkspaceState::default();
        let project = Project::new("Script Test");
        *workspace.project.lock().unwrap() = Some(project);
        (workspace, ActivityWorkspaceState::default())
    }

    #[test]
    fn dry_run_is_non_mutating_and_apply_is_atomic() {
        let (workspace, activity) = states();
        let before = workspace
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len();
        let source = r#"{"source_namespace":"script-test","operations":[{"op":"element","external_id":"PKG","kind":"Package","name":"Vehicle","owner":"$root"},{"op":"element","external_id":"VEH","kind":"Block","name":"Vehicle","owner":"ext:PKG"}],"diagrams":[{"external_id":"BDD","family":"BDD","name":"Vehicle Structure","owner":"ext:PKG"}]}"#;
        let preview = preview_impl("test.groovy", source, &workspace, &activity);
        assert!(preview.valid(), "{:?}", preview.diagnostics);
        assert_eq!(
            workspace
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            before
        );
        let (_, candidate_workspace, candidate_activity) =
            build_candidate("test.groovy", source, &workspace, &activity).unwrap();
        commit_candidate(
            &workspace,
            &activity,
            &candidate_workspace,
            &candidate_activity,
        )
        .unwrap();
        assert!(
            workspace
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len()
                > before
        );
        assert_eq!(workspace.diagrams.lock().unwrap().len(), 1);
    }

    #[test]
    fn wrong_kind_collision_is_blocked() {
        let (workspace, activity) = states();
        let source = r#"{"source_namespace":"script-test","operations":[{"op":"element","external_id":"X","kind":"Package","name":"X","owner":"$root"}]}"#;
        let (_, candidate_workspace, candidate_activity) =
            build_candidate("first.groovy", source, &workspace, &activity).unwrap();
        commit_candidate(
            &workspace,
            &activity,
            &candidate_workspace,
            &candidate_activity,
        )
        .unwrap();
        let wrong = r#"{"source_namespace":"script-test","operations":[{"op":"element","external_id":"X","kind":"Block","name":"X","owner":"$root"}]}"#;
        let preview = preview_impl("second.groovy", wrong, &workspace, &activity);
        assert!(!preview.valid());
        assert_eq!(preview.diagnostics[0].code, "WRONG_KIND_COLLISION");
    }

    #[test]
    fn late_invalid_binding_leaves_live_workspace_unchanged() {
        let (workspace, activity) = states();
        let before = workspace
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len();
        let source = r#"{"source_namespace":"rollback","operations":[{"op":"element","external_id":"PKG","kind":"Package","name":"Vehicle","owner":"$root"},{"op":"element","external_id":"B","kind":"Block","name":"Vehicle","owner":"ext:PKG"},{"op":"binding","external_id":"BAD","name":"bad","owner":"ext:B","source":{"role":"ext:B"},"target":{"role":"ext:DOES_NOT_EXIST"}}]}"#;
        let preview = preview_impl("rollback.groovy", source, &workspace, &activity);
        assert!(!preview.valid());
        assert_eq!(
            workspace
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            before
        );
    }

    // PR51_REPRESENTATIVE_QUALIFICATION
    fn representative_vehicle_script() -> &'static str {
        include_str!("../../../../../examples/model-script/vehicle-model.groovy")
    }

    fn applied_vehicle_states() -> (WorkspaceState, ActivityWorkspaceState) {
        let (workspace, activity) = states();
        let (compiled, candidate_workspace, candidate_activity) = build_candidate(
            "vehicle-model.groovy",
            representative_vehicle_script(),
            &workspace,
            &activity,
        )
        .unwrap_or_else(|preview| {
            panic!("representative script failed: {:?}", preview.diagnostics)
        });
        assert_eq!(compiled.document.diagrams.len(), 9);
        commit_candidate(
            &workspace,
            &activity,
            &candidate_workspace,
            &candidate_activity,
        )
        .unwrap();
        (workspace, activity)
    }

    #[test]
    fn representative_script_builds_native_semantics_and_all_nine_diagram_families() {
        let (workspace, activity) = states();
        let before =
            super::super::portable_interchange::export_from_states(&workspace, &activity).unwrap();
        let preview = preview_impl(
            "vehicle-model.groovy",
            representative_vehicle_script(),
            &workspace,
            &activity,
        );
        assert!(preview.valid(), "{:?}", preview.diagnostics);
        let after_preview =
            super::super::portable_interchange::export_from_states(&workspace, &activity).unwrap();
        assert_eq!(before, after_preview, "dry run mutated authored state");

        let (_, candidate_workspace, candidate_activity) = build_candidate(
            "vehicle-model.groovy",
            representative_vehicle_script(),
            &workspace,
            &activity,
        )
        .unwrap();
        super::super::portable_interchange::portable_from_states(
            &candidate_workspace,
            &candidate_activity,
        )
        .unwrap();
        assert_eq!(candidate_workspace.diagrams.lock().unwrap().len(), 5);
        assert_eq!(candidate_workspace.ibd_diagrams.lock().unwrap().len(), 1);
        assert_eq!(candidate_activity.diagrams.lock().unwrap().len(), 1);
        assert_eq!(
            candidate_workspace.behavior_diagrams.lock().unwrap().len(),
            2
        );

        let project = candidate_workspace.project.lock().unwrap().clone().unwrap();
        assert!(
            project
                .elements
                .values()
                .any(|element| element.external_id == "groovy:vehicle-example::VEH")
        );
        assert!(project.relationships.values().any(|relationship| {
            relationship.external_id == "groovy:vehicle-example::BIND"
                && relationship.kind == RelationshipKind::BindingConnector
        }));
        assert!(project.relationships.values().any(|relationship| {
            relationship.external_id == "groovy:vehicle-example::CTRL_LINK"
                && relationship.kind == RelationshipKind::Connector
        }));
        assert!(project.relationships.values().any(|relationship| {
            relationship.external_id == "groovy:vehicle-example::STARTED_FLOW"
                && relationship.kind == RelationshipKind::ItemFlow
        }));
        project.validate().unwrap();

        let activities = candidate_activity.repository.lock().unwrap();
        let activity_record = activities
            .activities
            .values()
            .find(|record| record.external_id == "groovy:vehicle-example::ACT_START")
            .unwrap();
        assert!(activity_record.nodes.len() >= 4);
        assert!(activity_record.edges.len() >= 3);
        activities.validate(&project).unwrap();
        drop(activities);

        let behavior = candidate_workspace.behavior.lock().unwrap();
        assert!(
            behavior
                .state_machines
                .values()
                .any(|record| record.external_id == "groovy:vehicle-example::SM_MODES")
        );
        let interaction = behavior
            .interactions
            .values()
            .find(|record| record.external_id == "groovy:vehicle-example::INT_STARTUP")
            .unwrap();
        assert_eq!(interaction.messages.len(), 1);
        assert_eq!(interaction.executions.len(), 1);
        drop(behavior);

        let parametric = candidate_workspace
            .diagrams
            .lock()
            .unwrap()
            .iter()
            .find(|diagram| diagram.family == "parametric")
            .cloned()
            .unwrap();
        let constraint_node = parametric
            .nodes
            .iter()
            .find(|node| {
                super::super::parse_element_id(&node.element_id)
                    .ok()
                    .and_then(|id| project.element(id).ok())
                    .is_some_and(|element| element.kind == ElementKind::ConstraintProperty)
            })
            .unwrap();
        assert_eq!(constraint_node.parameter_presentations.len(), 1);
        assert_eq!(parametric.edges.len(), 1);
        assert!(parametric.edges[0].points.len() >= 2);

        commit_candidate(
            &workspace,
            &activity,
            &candidate_workspace,
            &candidate_activity,
        )
        .unwrap();
        let first_element_count = workspace
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .len();
        let first_relationship_count = workspace
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationships
            .len();
        let first_diagram_count = workspace.diagrams.lock().unwrap().len()
            + workspace.ibd_diagrams.lock().unwrap().len()
            + activity.diagrams.lock().unwrap().len()
            + workspace.behavior_diagrams.lock().unwrap().len();
        assert_eq!(first_diagram_count, 9);

        let (_, second_workspace, second_activity) = build_candidate(
            "vehicle-model.groovy",
            representative_vehicle_script(),
            &workspace,
            &activity,
        )
        .unwrap();
        commit_candidate(&workspace, &activity, &second_workspace, &second_activity).unwrap();
        assert_eq!(
            workspace
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .elements
                .len(),
            first_element_count
        );
        assert_eq!(
            workspace
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationships
                .len(),
            first_relationship_count
        );
        let second_diagram_count = workspace.diagrams.lock().unwrap().len()
            + workspace.ibd_diagrams.lock().unwrap().len()
            + activity.diagrams.lock().unwrap().len()
            + workspace.behavior_diagrams.lock().unwrap().len();
        assert_eq!(
            second_diagram_count, 9,
            "script reapply duplicated a diagram"
        );
    }

    #[test]
    fn exact_qualified_name_and_explicit_plan_local_handle_references_are_supported() {
        let (workspace, activity) = states();
        let package_qname = {
            let mut guard = workspace.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let package = project
                .create_element(ElementKind::Package, "Existing", project.root_id)
                .unwrap();
            project.qualified_name(package).unwrap()
        };
        let source = format!(
            r#"{{"source_namespace":"qname-test","operations":[{{"op":"element","external_id":"BLOCK","kind":"Block","name":"QualifiedChild","owner":"qname:{package_qname}"}},{{"op":"element","external_id":"OP","kind":"Operation","name":"run","owner":"handle:BLOCK"}}]}}"#
        );
        let preview = preview_impl("qname.groovy", &source, &workspace, &activity);
        assert!(preview.valid(), "{:?}", preview.diagnostics);
        let (_, candidate_workspace, _) =
            build_candidate("qname.groovy", &source, &workspace, &activity).unwrap();
        let guard = candidate_workspace.project.lock().unwrap();
        assert!(
            guard
                .as_ref()
                .unwrap()
                .elements
                .values()
                .any(|element| { element.external_id == "qname-test::OP" })
        );
    }

    #[test]
    fn script_created_model_round_trips_smproj_metadata_and_exports_portable_json_and_xlsx() {
        let (workspace, activity) = applied_vehicle_states();
        let portable_json =
            super::super::portable_interchange::export_from_states(&workspace, &activity).unwrap();
        assert!(portable_json.contains("groovy:vehicle-example::VEH"));
        assert!(portable_json.contains("groovy:vehicle-example::ACT_START"));
        assert!(portable_json.contains("groovy:vehicle-example::INT_STARTUP"));

        let xlsx = std::env::temp_dir().join(format!("pr51-{}.xlsx", uuid::Uuid::new_v4()));
        super::super::spreadsheet_interchange::export_workbook_to_path(
            xlsx.to_str().unwrap(),
            super::super::spreadsheet_interchange::SpreadsheetExportProfile::SystemsModeler,
            &workspace,
            &activity,
        )
        .unwrap();
        assert!(std::fs::metadata(&xlsx).unwrap().len() > 1024);

        let smproj = std::env::temp_dir().join(format!("pr51-{}.smproj", uuid::Uuid::new_v4()));
        let project = workspace.project.lock().unwrap().clone().unwrap();
        let diagrams = workspace.diagrams.lock().unwrap().clone();
        let ibd_diagrams = workspace.ibd_diagrams.lock().unwrap().clone();
        let behavior = workspace.behavior.lock().unwrap().clone();
        let behavior_diagrams = workspace.behavior_diagrams.lock().unwrap().clone();
        let activity_repository = activity.repository.lock().unwrap().clone();
        let activity_diagrams = activity.diagrams.lock().unwrap().clone();
        {
            let mut database = systems_modeler_persistence::ProjectDatabase::open(&smproj).unwrap();
            database.save_project(&project).unwrap();
            database
                .save_metadata(
                    project.id,
                    super::super::BDD_METADATA_KEY,
                    &serde_json::to_string(&diagrams).unwrap(),
                )
                .unwrap();
            super::super::ibd::save_ibd_metadata(&mut database, &project, &ibd_diagrams).unwrap();
            super::super::behavior_workspace::save_behavior_metadata(
                &mut database,
                &project,
                &behavior,
                &behavior_diagrams,
            )
            .unwrap();
            super::super::activity_workspace::save_activity_workspace_metadata(
                &mut database,
                &project,
                &activity_repository,
                &activity_diagrams,
            )
            .unwrap();
        }
        {
            let database = systems_modeler_persistence::ProjectDatabase::open(&smproj).unwrap();
            let reopened = database.load_first_project().unwrap();
            reopened.validate().unwrap();
            assert!(
                reopened
                    .elements
                    .values()
                    .any(|element| element.external_id == "groovy:vehicle-example::VEH")
            );
            let bdd_payload = database
                .load_metadata(reopened.id, super::super::BDD_METADATA_KEY)
                .unwrap()
                .unwrap();
            let reopened_diagrams: Vec<BddDiagram> = serde_json::from_str(&bdd_payload).unwrap();
            assert_eq!(reopened_diagrams.len(), 5);
            let reopened_ibd = super::super::ibd::load_ibd_metadata(&database, &reopened).unwrap();
            assert_eq!(reopened_ibd.len(), 1);
            let (reopened_behavior, reopened_behavior_diagrams) =
                super::super::behavior_workspace::load_behavior_metadata(&database, &reopened)
                    .unwrap();
            assert_eq!(reopened_behavior.state_machines.len(), 1);
            assert_eq!(reopened_behavior.interactions.len(), 1);
            assert_eq!(reopened_behavior_diagrams.len(), 2);
            let (reopened_activity, reopened_activity_diagrams) =
                super::super::activity_workspace::load_activity_workspace_metadata(
                    &database, &reopened,
                )
                .unwrap();
            assert_eq!(reopened_activity.activities.len(), 1);
            assert_eq!(reopened_activity_diagrams.len(), 1);
        }
        let _ = std::fs::remove_file(xlsx);
        let _ = std::fs::remove_file(smproj);
    }
}
