use super::*;

#[derive(Debug, Clone, Serialize)]
pub struct CompleteElementSnapshot {
    pub id: String,
    pub external_id: String,
    pub kind: String,
    pub name: String,
    pub owner_id: Option<String>,
    pub qualified_name: String,
    pub packageable: bool,
    pub namespace: bool,
    pub visibility: String,
    pub documentation: String,
    pub type_id: Option<String>,
    pub multiplicity: Option<String>,
    pub aggregation: String,
    pub default_value: Option<String>,
    pub is_derived: bool,
    pub is_read_only: bool,
    pub is_conjugated: bool,
    pub quantity_kind_external_id: Option<String>,
    pub unit_external_id: Option<String>,
    pub parameter_direction: Option<String>,
    pub literal_value: Option<String>,
    pub flow_direction: Option<String>,
    pub requirement_id: Option<String>,
    pub requirement_text: Option<String>,
    pub extension_points: Vec<String>,
    pub use_case_specification: String,
    pub represented_classifier_id: Option<String>,
    pub constraint_expression: String,
    pub quantity_dimension: Option<String>,
    pub unit_symbol: Option<String>,
    pub unit_scale_to_base: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompleteProjectSnapshot {
    pub id: String,
    pub name: String,
    pub root_id: String,
    pub elements: Vec<CompleteElementSnapshot>,
    pub relationships: Vec<RelationshipSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompleteWorkspaceSnapshot {
    pub project: Option<CompleteProjectSnapshot>,
    pub diagrams: Vec<BddDiagram>,
    pub ibd_diagrams: Vec<ibd::IbdDiagram>,
    pub current_file: Option<String>,
}

fn bdd_presentable(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Block
            | ElementKind::AssociationBlock
            | ElementKind::InterfaceBlock
            | ElementKind::ConstraintBlock
            | ElementKind::ValueType
            | ElementKind::DataType
            | ElementKind::PrimitiveType
            | ElementKind::Enumeration
            | ElementKind::Signal
            | ElementKind::Unit
            | ElementKind::QuantityKind
            | ElementKind::InstanceSpecification
            | ElementKind::Comment
            | ElementKind::Requirement
            | ElementKind::TestCase
            | ElementKind::Actor
            | ElementKind::UseCase
    )
}

fn parse_kind(value: &str) -> Result<ElementKind, String> {
    match value {
        "Block" => Ok(ElementKind::Block),
        "AssociationBlock" => Ok(ElementKind::AssociationBlock),
        "InterfaceBlock" => Ok(ElementKind::InterfaceBlock),
        "ConstraintBlock" => Ok(ElementKind::ConstraintBlock),
        "ValueType" => Ok(ElementKind::ValueType),
        "DataType" => Ok(ElementKind::DataType),
        "PrimitiveType" => Ok(ElementKind::PrimitiveType),
        "Enumeration" => Ok(ElementKind::Enumeration),
        "EnumerationLiteral" => Ok(ElementKind::EnumerationLiteral),
        "Signal" => Ok(ElementKind::Signal),
        "Unit" => Ok(ElementKind::Unit),
        "QuantityKind" => Ok(ElementKind::QuantityKind),
        "InstanceSpecification" => Ok(ElementKind::InstanceSpecification),
        "Slot" => Ok(ElementKind::Slot),
        "PartProperty" => Ok(ElementKind::PartProperty),
        "ReferenceProperty" => Ok(ElementKind::ReferenceProperty),
        "ValueProperty" => Ok(ElementKind::ValueProperty),
        "FlowProperty" => Ok(ElementKind::FlowProperty),
        "ConstraintProperty" => Ok(ElementKind::ConstraintProperty),
        "ConstraintParameter" => Ok(ElementKind::ConstraintParameter),
        "ProxyPort" => Ok(ElementKind::ProxyPort),
        "FullPort" => Ok(ElementKind::FullPort),
        "Operation" => Ok(ElementKind::Operation),
        "Parameter" => Ok(ElementKind::Parameter),
        "Reception" => Ok(ElementKind::Reception),
        "Comment" => Ok(ElementKind::Comment),
        "Requirement" => Ok(ElementKind::Requirement),
        "TestCase" => Ok(ElementKind::TestCase),
        "Actor" => Ok(ElementKind::Actor),
        "UseCase" => Ok(ElementKind::UseCase),
        _ => Err(format!("unsupported BDD semantic kind: {value}")),
    }
}

fn direction_name(value: systems_modeler_core::ParameterDirection) -> &'static str {
    match value {
        systems_modeler_core::ParameterDirection::In => "in",
        systems_modeler_core::ParameterDirection::Out => "out",
        systems_modeler_core::ParameterDirection::InOut => "inout",
        systems_modeler_core::ParameterDirection::Return => "return",
    }
}

fn flow_direction_name(value: systems_modeler_core::FlowDirection) -> &'static str {
    match value {
        systems_modeler_core::FlowDirection::In => "in",
        systems_modeler_core::FlowDirection::Out => "out",
        systems_modeler_core::FlowDirection::InOut => "inout",
    }
}

fn snapshot_complete(project: &Project) -> CompleteProjectSnapshot {
    let mut elements: Vec<_> = project
        .elements
        .values()
        .map(|element| CompleteElementSnapshot {
            id: element.id.to_string(),
            external_id: element.external_id.clone(),
            kind: format!("{:?}", element.kind),
            name: element.name.clone(),
            owner_id: element.owner_id.map(|id| id.to_string()),
            qualified_name: qualified_element_name(project, element.id),
            packageable: element.is_packageable(),
            namespace: element.is_namespace(),
            visibility: visibility_name(element.visibility).to_string(),
            documentation: element.documentation.clone(),
            type_id: element.type_id.map(|id| id.to_string()),
            multiplicity: element.multiplicity.map(|value| value.notation()),
            aggregation: aggregation_name(element.aggregation).to_string(),
            default_value: element.default_value.clone(),
            is_derived: element.is_derived,
            is_read_only: element.is_read_only,
            is_conjugated: element.is_conjugated,
            quantity_kind_external_id: element.quantity_kind_external_id.clone(),
            unit_external_id: element.unit_external_id.clone(),
            parameter_direction: element
                .parameter_direction
                .map(direction_name)
                .map(str::to_string),
            literal_value: element.literal_value.clone(),
            flow_direction: element
                .flow_direction
                .map(flow_direction_name)
                .map(str::to_string),
            requirement_id: element.requirement_id.clone(),
            requirement_text: element.requirement_text.clone(),
            extension_points: element.extension_points.clone(),
            use_case_specification: element.use_case_specification.clone(),
            represented_classifier_id: element.represented_classifier_id.map(|id| id.to_string()),
            constraint_expression: element.constraint_expression.clone(),
            quantity_dimension: element.quantity_dimension.clone(),
            unit_symbol: element.unit_symbol.clone(),
            unit_scale_to_base: element.unit_scale_to_base,
        })
        .collect();
    elements.sort_by(|a, b| a.name.cmp(&b.name));

    let mut relationships: Vec<_> = project
        .relationships
        .values()
        .map(|relationship| RelationshipSnapshot {
            id: relationship.id.to_string(),
            external_id: relationship.external_id.clone(),
            kind: relationship_display_kind(relationship).to_string(),
            name: relationship.name.clone(),
            owner_id: relationship.owner_id.map(|id| id.to_string()),
            source_id: relationship.source_id.to_string(),
            target_id: relationship.target_id.to_string(),
            documentation: relationship.documentation.clone(),
            visibility: visibility_name(relationship.visibility).to_string(),
            alias: relationship.alias.clone(),
            association_ends: relationship
                .association_ends
                .iter()
                .enumerate()
                .map(|(index, end)| association_end_snapshot(relationship, index, end))
                .collect(),
            extension_condition: relationship.extension_condition.clone(),
            extension_location: relationship.extension_location.clone(),
            binding: relationship
                .binding
                .as_ref()
                .map(|binding| BindingConnectorSnapshot {
                    source: BindingEndpointSnapshot {
                        role_id: binding.source.role_id.to_string(),
                        parameter_id: binding.source.parameter_id.map(|id| id.to_string()),
                    },
                    target: BindingEndpointSnapshot {
                        role_id: binding.target.role_id.to_string(),
                        parameter_id: binding.target.parameter_id.map(|id| id.to_string()),
                    },
                }),
            applied_stereotypes: relationship.applied_stereotypes.clone(),
        })
        .collect();
    relationships.sort_by(|a, b| a.id.cmp(&b.id));

    CompleteProjectSnapshot {
        id: project.id.to_string(),
        name: project.name.clone(),
        root_id: project.root_id.to_string(),
        elements,
        relationships,
    }
}

fn validate_complete_diagrams(project: &Project, diagrams: &[BddDiagram]) -> Result<(), String> {
    let mut diagram_ids = HashSet::new();
    let mut node_ids = HashSet::new();
    let mut edge_ids = HashSet::new();
    for diagram in diagrams {
        parse_diagram_id(&diagram.id)?;
        if !diagram_ids.insert(&diagram.id) {
            return Err(format!("duplicate diagram id: {}", diagram.id));
        }
        let owner_id = parse_element_id(&diagram.owner_id)?;
        let owner = project
            .element(owner_id)
            .map_err(|error| error.to_string())?;
        if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
            return Err(format!(
                "BDD owner is not a Model or Package: {}",
                diagram.owner_id
            ));
        }
        if let Some(context_id) = diagram.semantic_context_id.as_deref() {
            let context = project
                .element(parse_element_id(context_id)?)
                .map_err(|error| error.to_string())?;
            if diagram.family == "use-case"
                && (!context.is_classifier()
                    || matches!(context.kind, ElementKind::Actor | ElementKind::UseCase))
            {
                return Err(
                    "Use Case diagram context is not a represented system classifier".into(),
                );
            }
            if diagram.family == "parametric"
                && !matches!(
                    context.kind,
                    ElementKind::Block
                        | ElementKind::AssociationBlock
                        | ElementKind::ConstraintBlock
                )
            {
                return Err("Parametric diagram context must be a Block or ConstraintBlock".into());
            }
        } else if diagram.family == "parametric" {
            return Err("Parametric Diagram requires a semantic context".into());
        }
        if diagram.family != "use-case" && diagram.subject_boundary.is_some() {
            return Err("subject boundaries are only valid on Use Case Diagrams".into());
        }
        if diagram.semantic_context_id.is_none() && diagram.subject_boundary.is_some() {
            return Err("Use Case subject boundary requires a semantic context".into());
        }
        if let Some(boundary) = diagram.subject_boundary.as_ref() {
            if uuid::Uuid::parse_str(&boundary.id).is_err() || !node_ids.insert(&boundary.id) {
                return Err(format!(
                    "invalid or duplicate Use Case subject-boundary id: {}",
                    boundary.id
                ));
            }
            if !boundary.x.is_finite()
                || !boundary.y.is_finite()
                || !boundary.width.is_finite()
                || !boundary.height.is_finite()
                || boundary.x < 0.0
                || boundary.y < 42.0
                || boundary.width < 280.0
                || boundary.height < 220.0
            {
                return Err("invalid Use Case subject-boundary geometry".into());
            }
        }
        for node in &diagram.nodes {
            if uuid::Uuid::parse_str(&node.id).is_err() {
                return Err(format!("invalid diagram node id: {}", node.id));
            }
            if !node_ids.insert(&node.id) {
                return Err(format!("duplicate diagram node id: {}", node.id));
            }
            let element = project
                .element(parse_element_id(&node.element_id)?)
                .map_err(|error| error.to_string())?;
            if diagram.family == "package"
                && !element.is_packageable()
                && element.kind != ElementKind::Comment
            {
                return Err(format!(
                    "element kind {:?} is not valid on a Package Diagram",
                    element.kind
                ));
            }
            if diagram.family != "parametric"
                && diagram.family != "package"
                && !bdd_presentable(&element.kind)
            {
                return Err(format!(
                    "element kind {:?} is not valid as a BDD node",
                    element.kind
                ));
            }
            if diagram.family == "use-case"
                && !matches!(element.kind, ElementKind::Actor | ElementKind::UseCase)
            {
                return Err(format!(
                    "element kind {:?} is not valid on a Use Case Diagram",
                    element.kind
                ));
            }
            if diagram.family == "parametric" {
                if !matches!(
                    element.kind,
                    ElementKind::ConstraintProperty | ElementKind::ValueProperty
                ) {
                    return Err(format!(
                        "element kind {:?} is not valid on a Parametric Diagram",
                        element.kind
                    ));
                }
                project
                    .validate_parametric_role(parametrics::diagram_context(diagram)?, element.id)
                    .map_err(|error| error.to_string())?;
                if element.kind == ElementKind::ValueProperty
                    && !node.parameter_presentations.is_empty()
                {
                    return Err("ValueProperty presentations cannot own parameter endpoints".into());
                }
                if element.kind == ElementKind::ConstraintProperty {
                    let constraint_block_id =
                        element.type_id.ok_or("ConstraintProperty has no type")?;
                    let expected_parameters: HashSet<_> = project
                        .children(constraint_block_id)
                        .filter(|parameter| parameter.kind == ElementKind::ConstraintParameter)
                        .map(|parameter| parameter.id.to_string())
                        .collect();
                    let presented_parameters: HashSet<_> = node
                        .parameter_presentations
                        .iter()
                        .map(|parameter| parameter.parameter_id.clone())
                        .collect();
                    if presented_parameters != expected_parameters {
                        return Err("ConstraintProperty presentation must expose every definition-owned parameter exactly once".into());
                    }
                    for parameter in &node.parameter_presentations {
                        let max_x = (node.width - parameter.size).max(0.0);
                        let max_y = (node.height - parameter.size).max(0.0);
                        let on_boundary = parameter.offset_x.abs() < f64::EPSILON
                            || (parameter.offset_x - max_x).abs() < f64::EPSILON
                            || parameter.offset_y.abs() < f64::EPSILON
                            || (parameter.offset_y - max_y).abs() < f64::EPSILON;
                        if uuid::Uuid::parse_str(&parameter.id).is_err()
                            || !node_ids.insert(&parameter.id)
                            || !parameter.offset_x.is_finite()
                            || !parameter.offset_y.is_finite()
                            || !parameter.size.is_finite()
                            || parameter.size < 10.0
                            || parameter.offset_x < 0.0
                            || parameter.offset_x > max_x
                            || parameter.offset_y < 0.0
                            || parameter.offset_y > max_y
                            || !on_boundary
                        {
                            return Err("invalid ConstraintParameter presentation geometry".into());
                        }
                        let semantic = project
                            .element(parse_element_id(&parameter.parameter_id)?)
                            .map_err(|error| error.to_string())?;
                        if semantic.kind != ElementKind::ConstraintParameter
                            || semantic.owner_id != Some(constraint_block_id)
                        {
                            return Err("ConstraintParameter presentation does not match its reusable definition".into());
                        }
                    }
                }
            }
            if let Some(notation) = node.actor_notation.as_deref()
                && (element.kind != ElementKind::Actor
                    || !matches!(notation, "stick" | "rectangle"))
            {
                return Err(format!(
                    "invalid Actor notation for presentation {}",
                    node.id
                ));
            }
            if element.kind == ElementKind::UseCase
                && let Some(boundary) = diagram.subject_boundary.as_ref()
                && (node.x < boundary.x
                    || node.y < boundary.y
                    || node.x + node.width > boundary.x + boundary.width
                    || node.y + node.height > boundary.y + boundary.height)
            {
                return Err(format!(
                    "Use Case presentation {} is outside its subject boundary",
                    node.id
                ));
            }
        }
        for edge in &diagram.edges {
            if uuid::Uuid::parse_str(&edge.id).is_err() {
                return Err(format!("invalid diagram edge id: {}", edge.id));
            }
            if !edge_ids.insert(&edge.id) {
                return Err(format!("duplicate diagram edge id: {}", edge.id));
            }
            let relationship = project
                .relationship(parse_relationship_id(&edge.relationship_id)?)
                .map_err(|error| error.to_string())?;
            if matches!(
                relationship.kind,
                RelationshipKind::Connector | RelationshipKind::ItemFlow
            ) {
                return Err(
                    "Connector and ItemFlow presentations belong on an IBD, not a BDD".into(),
                );
            }
            if diagram.family == "use-case"
                && !matches!(
                    relationship.kind,
                    RelationshipKind::Association
                        | RelationshipKind::Include
                        | RelationshipKind::Extend
                        | RelationshipKind::Generalization
                )
            {
                return Err(format!(
                    "relationship kind {:?} is not valid on a Use Case Diagram",
                    relationship.kind
                ));
            }
            if diagram.family == "parametric"
                && relationship.kind != RelationshipKind::BindingConnector
            {
                return Err("only BindingConnectors are valid on a Parametric Diagram".into());
            }
            if diagram.family != "parametric"
                && relationship.kind == RelationshipKind::BindingConnector
            {
                return Err("BindingConnector presentations belong on a Parametric Diagram".into());
            }
            if diagram.family == "parametric" {
                project
                    .validate_binding_in_context(
                        relationship,
                        parametrics::diagram_context(diagram)?,
                    )
                    .map_err(|error| error.to_string())?;
                let binding = relationship
                    .binding
                    .as_ref()
                    .ok_or("BindingConnector has no semantic endpoint details")?;
                if !parametric_endpoint_matches(diagram, &edge.source_node_id, &binding.source)
                    || !parametric_endpoint_matches(diagram, &edge.target_node_id, &binding.target)
                {
                    return Err(format!(
                        "diagram binding endpoints do not match semantic relationship: {}",
                        edge.relationship_id
                    ));
                }
            } else {
                let source = diagram
                    .nodes
                    .iter()
                    .find(|node| node.id == edge.source_node_id)
                    .ok_or_else(|| {
                        format!("edge source node not found: {}", edge.source_node_id)
                    })?;
                let target = diagram
                    .nodes
                    .iter()
                    .find(|node| node.id == edge.target_node_id)
                    .ok_or_else(|| {
                        format!("edge target node not found: {}", edge.target_node_id)
                    })?;
                if source.element_id != relationship.source_id.to_string()
                    || target.element_id != relationship.target_id.to_string()
                {
                    return Err(format!(
                        "diagram edge endpoints do not match semantic relationship: {}",
                        edge.relationship_id
                    ));
                }
            }
            if edge.points.len() < 2 {
                return Err(format!("diagram edge has no usable route: {}", edge.id));
            }
        }
        if diagram.family == "package" {
            package_diagrams::validate_package_diagram(project, diagram)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn workspace_snapshot_complete(
    state: tauri::State<'_, WorkspaceState>,
) -> Result<CompleteWorkspaceSnapshot, String> {
    let project = state.project.lock().map_err(|_| "project lock poisoned")?;
    let diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let ibd_diagrams = state.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")?;
    if let Some(project) = project.as_ref() {
        validate_complete_diagrams(project, &diagrams)?;
    }
    let current_file = state
        .current_file
        .lock()
        .map_err(|_| "project path lock poisoned")?;
    Ok(CompleteWorkspaceSnapshot {
        project: project.as_ref().map(snapshot_complete),
        diagrams: diagrams.clone(),
        ibd_diagrams: ibd_diagrams.clone(),
        current_file: current_file.clone(),
    })
}

#[tauri::command]
pub fn create_bdd_element(
    kind: String,
    owner_id: String,
    name: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let kind = parse_kind(&kind)?;
    if !bdd_presentable(&kind) {
        return Err(format!(
            "{kind:?} is an owned feature, not a top-level BDD element"
        ));
    }
    if kind == ElementKind::Requirement {
        return create_element(kind, owner_id, name, state);
    }
    use systems_modeler_core::structural_presentation::creation::{
        BddElementKind, CreateBddElement,
    };
    let command = CreateBddElement {
        kind: BddElementKind::from_model_kind(&kind)
            .ok_or("unsupported BDD classifier creation")?,
        owner: parse_element_id(&owner_id)?,
        name,
    };
    let mut project = state.project.lock().map_err(|_| "project lock poisoned")?;
    command
        .apply(project.as_mut().ok_or("no project open")?)
        .map(|id| id.to_string())
        .map_err(|error| error.to_string())
}

fn parse_multiplicity(lower: u32, upper: Option<u32>) -> Result<Multiplicity, String> {
    Multiplicity::new(lower, upper).map_err(|error| error.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable Tauri IPC contract; frontend sends named fields.
pub fn create_bdd_feature(
    kind: String,
    owner_id: String,
    name: String,
    type_id: Option<String>,
    lower: Option<u32>,
    upper: Option<u32>,
    default_value: Option<String>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let kind = parse_kind(&kind)?;
    let owner_id = parse_element_id(&owner_id)?;
    let mut project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_mut().ok_or("no project open")?;
    let id = match kind {
        ElementKind::EnumerationLiteral | ElementKind::Slot | ElementKind::Operation => project
            .create_element(kind, name, owner_id)
            .map_err(|error| error.to_string())?,
        ElementKind::Reception => {
            let type_id = type_id
                .ok_or_else(|| "Reception requires a modeled Signal stable type ID".to_string())?;
            let id = project
                .create_element(ElementKind::Reception, name, owner_id)
                .map_err(|error| error.to_string())?;
            project
                .set_element_type(id, parse_element_id(&type_id)?)
                .map_err(|error| error.to_string())?;
            id
        }
        ElementKind::PartProperty
        | ElementKind::ReferenceProperty
        | ElementKind::ValueProperty
        | ElementKind::FlowProperty
        | ElementKind::ConstraintProperty
        | ElementKind::ProxyPort
        | ElementKind::FullPort
        | ElementKind::Parameter => {
            let type_id =
                type_id.ok_or_else(|| format!("{kind:?} requires a compatible stable type ID"))?;
            project
                .create_typed_feature(
                    kind,
                    name,
                    owner_id,
                    parse_element_id(&type_id)?,
                    parse_multiplicity(lower.unwrap_or(1), upper)?,
                )
                .map_err(|error| error.to_string())?
        }
        _ => return Err(format!("{kind:?} is not an owned BDD feature")),
    };
    if let Some(default_value) = default_value {
        project
            .element_mut(id)
            .map_err(|error| error.to_string())?
            .default_value = Some(default_value);
    }
    project
        .validate_element(id)
        .map_err(|error| error.to_string())?;
    Ok(id.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Preserve named fields on the existing IPC command.
pub fn update_bdd_element_details(
    element_id: String,
    documentation: Option<String>,
    default_value: Option<String>,
    quantity_kind_external_id: Option<String>,
    unit_external_id: Option<String>,
    type_id: Option<String>,
    state: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    apply_legacy_details(
        parse_element_id(&element_id)?,
        systems_modeler_core::ElementSpecificationEdit {
            documentation,
            default_value,
            quantity_kind_external_id,
            unit_external_id,
            type_id: type_id.as_deref().map(parse_element_id).transpose()?,
            ..Default::default()
        },
        &state,
        &activity,
        &history,
    )
    .map(|_| ())
}

fn apply_legacy_details(
    element_id: ElementId,
    mut edit: systems_modeler_core::ElementSpecificationEdit,
    state: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &history::HistoryState,
) -> Result<bool, String> {
    history::apply_structural_specification(state, activity, history, |project, ibds| {
        // The legacy command has no rename field. Resolve its current name under
        // the transaction guard rather than racing a separate snapshot/read.
        edit.name = project
            .element(element_id)
            .map_err(|error| error.to_string())?
            .name
            .clone();
        Ok((
            project.stage_element_specification(element_id, &edit)?,
            ibds.to_vec(),
        ))
    })
}

#[tauri::command]
pub fn place_bdd_element(
    diagram_id: String,
    element_id: String,
    x: f64,
    y: f64,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let diagram_id = parse_diagram_id(&diagram_id)?;
    let element_id = parse_element_id(&element_id)?;
    let (width, height) = {
        let project = state.project.lock().map_err(|_| "project lock poisoned")?;
        let element = project
            .as_ref()
            .ok_or("no project open")?
            .element(element_id)
            .map_err(|error| error.to_string())?;
        if !bdd_presentable(&element.kind) {
            return Err(format!("{:?} is not valid as a BDD node", element.kind));
        }
        match element.kind {
            ElementKind::Enumeration => (190.0, 125.0),
            ElementKind::ConstraintBlock | ElementKind::AssociationBlock => (205.0, 120.0),
            ElementKind::ValueType
            | ElementKind::DataType
            | ElementKind::PrimitiveType
            | ElementKind::Unit
            | ElementKind::QuantityKind => (185.0, 100.0),
            ElementKind::Comment => (210.0, 90.0),
            _ => (190.0, 115.0),
        }
    };
    let mut diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id.to_string())
        .ok_or("diagram not found")?;
    if diagram
        .nodes
        .iter()
        .any(|node| node.element_id == element_id.to_string())
    {
        return Err("this semantic element is already presented on the BDD".into());
    }
    let node_id = uuid::Uuid::new_v4().to_string();
    diagram.nodes.push(DiagramNode {
        id: node_id.clone(),
        element_id: element_id.to_string(),
        x,
        y,
        width,
        height,
        actor_notation: None,
        parameter_presentations: Vec::new(),
    });
    Ok(node_id)
}

#[tauri::command]
pub fn create_bdd_relationship_complete(
    diagram_id: String,
    kind: String,
    source_element_id: String,
    target_element_id: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    create_bdd_relationship_in_state(
        diagram_id,
        kind,
        source_element_id,
        target_element_id,
        &state,
    )
}

pub(super) fn create_bdd_relationship_in_state(
    diagram_id: String,
    kind: String,
    source_element_id: String,
    target_element_id: String,
    state: &WorkspaceState,
) -> Result<String, String> {
    let kind = supported_relationship_kind(&kind)?;
    let source_id = parse_element_id(&source_element_id)?;
    let target_id = parse_element_id(&target_element_id)?;
    if source_id == target_id {
        return Err(format!("{kind} cannot connect an element to itself"));
    }
    let mut project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_mut().ok_or("no project open")?;
    let source = project
        .element(source_id)
        .map_err(|error| error.to_string())?;
    let target = project
        .element(target_id)
        .map_err(|error| error.to_string())?;
    if !source.is_classifier() || !target.is_classifier() {
        return Err(format!("{kind} requires classifier endpoints on a BDD"));
    }
    if kind != "Composition" && semantic_duplicate(project, kind, source_id, target_id) {
        return Err(format!("an equivalent {kind} already exists"));
    }
    let mut diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("diagram not found")?;
    let source_node = diagram
        .nodes
        .iter()
        .find(|node| node.element_id == source_element_id)
        .cloned()
        .ok_or("source classifier must be presented on the selected BDD")?;
    let target_node = diagram
        .nodes
        .iter()
        .find(|node| node.element_id == target_element_id)
        .cloned()
        .ok_or("target classifier must be presented on the selected BDD")?;
    let owner_id = Some(parse_element_id(&diagram.owner_id)?);
    // A route failure must not publish either half of a new composition.
    let lane_index = diagram
        .edges
        .iter()
        .filter(|edge| {
            edge.source_node_id == source_node.id && edge.target_node_id == target_node.id
        })
        .count();
    let points =
        route_relationship_at_lane(&source_node, &target_node, &diagram.nodes, lane_index)?;
    let relationship_id = match kind {
        "Composition" => {
            let type_name = project
                .element(target_id)
                .map_err(|error| error.to_string())?
                .name
                .clone();
            let mut characters = type_name.chars();
            let base_name = characters
                .next()
                .map(|first| first.to_lowercase().collect::<String>() + characters.as_str())
                .unwrap_or_else(|| "part".into());
            let names: std::collections::HashSet<_> = project
                .owned_features(source_id)
                .map(|element| element.name.clone())
                .collect();
            let mut name = base_name.clone();
            let mut suffix = 2;
            while names.contains(&name) {
                name = format!("{base_name}{suffix}");
                suffix += 1;
            }
            project
                .create_composition(source_id, target_id, name, Multiplicity::ONE, owner_id)
                .map_err(|error| error.to_string())?
                .0
        }
        "Association" | "Aggregation" => {
            let aggregation = match kind {
                "Aggregation" => AggregationKind::Shared,
                _ => AggregationKind::None,
            };
            project
                .create_association(
                    owner_id,
                    vec![
                        Project::association_end(
                            source_id,
                            "",
                            Multiplicity::ONE,
                            true,
                            aggregation,
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
                .map_err(|error| error.to_string())?
        }
        "Generalization" => project
            .create_relationship(
                RelationshipKind::Generalization,
                source_id,
                target_id,
                owner_id,
            )
            .map_err(|error| error.to_string())?,
        "Dependency" => project
            .create_relationship(RelationshipKind::Dependency, source_id, target_id, owner_id)
            .map_err(|error| error.to_string())?,
        "Realization" => project
            .create_relationship(
                RelationshipKind::Realization,
                source_id,
                target_id,
                owner_id,
            )
            .map_err(|error| error.to_string())?,
        _ => unreachable!(),
    };
    diagram.edges.push(DiagramEdge {
        id: uuid::Uuid::new_v4().to_string(),
        relationship_id: relationship_id.to_string(),
        source_node_id: source_node.id,
        target_node_id: target_node.id,
        label_anchor: Some(routing::route_label_anchor(&points)),
        points,
    });
    Ok(relationship_id.to_string())
}

struct CompleteWorkspaceRef<'a> {
    project: &'a Project,
    diagrams: &'a [BddDiagram],
    ibd_diagrams: &'a [ibd::IbdDiagram],
    behavior: &'a BehaviorRepository,
    behavior_diagrams: &'a [behavior_workspace::BehaviorDiagram],
    activity_repository: &'a systems_modeler_core::ActivityRepository,
    activity_diagrams: &'a [activity_workspace::ActivityDiagram],
    reqif_exchange: &'a reqif_interchange::ReqifExchangeState,
}

fn prepare_complete_project_metadata(
    workspace: CompleteWorkspaceRef<'_>,
) -> Result<Vec<(&'static str, String)>, String> {
    let CompleteWorkspaceRef {
        project,
        diagrams,
        ibd_diagrams,
        behavior,
        behavior_diagrams,
        activity_repository,
        activity_diagrams,
        reqif_exchange,
    } = workspace;
    project
        .validate()
        .map_err(|error| format!("project validation failed: {error}"))?;
    validate_loaded_diagrams(project, diagrams)?;
    ibd::validate_ibd_diagrams(project, ibd_diagrams)?;
    behavior_workspace::validate_behavior_workspace(project, behavior, behavior_diagrams)?;
    activity_repository
        .validate(project)
        .map_err(|error| format!("Activity repository validation failed: {error}"))?;
    activity_workspace::validate_activity_diagrams(activity_repository, activity_diagrams)?;

    Ok(vec![
        (
            BDD_METADATA_KEY,
            serde_json::to_string(diagrams).map_err(|error| error.to_string())?,
        ),
        (
            ibd::IBD_METADATA_KEY,
            serde_json::to_string(ibd_diagrams).map_err(|error| error.to_string())?,
        ),
        (
            behavior_workspace::BEHAVIOR_METADATA_KEY,
            serde_json::to_string(behavior).map_err(|error| error.to_string())?,
        ),
        (
            behavior_workspace::BEHAVIOR_DIAGRAM_METADATA_KEY,
            serde_json::to_string(behavior_diagrams).map_err(|error| error.to_string())?,
        ),
        (
            systems_modeler_persistence::ACTIVITY_METADATA_KEY,
            serde_json::to_string(activity_repository).map_err(|error| error.to_string())?,
        ),
        (
            activity_workspace::ACTIVITY_DIAGRAM_METADATA_KEY,
            serde_json::to_string(activity_diagrams).map_err(|error| error.to_string())?,
        ),
        (
            reqif_interchange::REQIF_METADATA_KEY,
            serde_json::to_string(reqif_exchange).map_err(|error| error.to_string())?,
        ),
    ])
}

#[tauri::command]
pub fn save_project_file_complete(
    path: String,
    state: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
) -> Result<String, String> {
    save_complete_workspace(Some(&path), &state, &activity_state)
}

fn save_complete_workspace(
    requested_path: Option<&str>,
    state: &WorkspaceState,
    activity_state: &activity_workspace::ActivityWorkspaceState,
) -> Result<String, String> {
    let project = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project.as_ref().ok_or("no project open")?;
    let diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let ibd_diagrams = state.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")?;
    let behavior = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let behavior_diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let reqif_exchange = state
        .reqif_exchange
        .lock()
        .map_err(|_| "ReqIF exchange lock poisoned")?;
    let activity_repository = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let activity_diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let mut current_file = state
        .current_file
        .lock()
        .map_err(|_| "project path lock poisoned")?;
    // Select the destination from the same locked session as the authored
    // snapshot. All fallible session locks must precede any database write.
    let path = normalize_project_path(
        requested_path
            .or(current_file.as_deref())
            .ok_or("project has not been saved yet; use Save As")?,
    )?;

    // Prepare and validate the complete authored snapshot before opening a
    // database transaction. No file or current-path state changes on failure.
    let metadata = prepare_complete_project_metadata(CompleteWorkspaceRef {
        project,
        diagrams: &diagrams,
        ibd_diagrams: &ibd_diagrams,
        behavior: &behavior,
        behavior_diagrams: &behavior_diagrams,
        activity_repository: &activity_repository,
        activity_diagrams: &activity_diagrams,
        reqif_exchange: &reqif_exchange,
    })?;
    let metadata_refs = metadata
        .iter()
        .map(|(key, payload)| (*key, payload.as_str()))
        .collect::<Vec<_>>();
    let mut database = ProjectDatabase::open(&path).map_err(|error| error.to_string())?;
    database
        .save_project_with_metadata(project, &metadata_refs)
        .map_err(|error| error.to_string())?;

    let saved_path = path.to_string_lossy().into_owned();
    *current_file = Some(saved_path.clone());
    Ok(saved_path)
}

#[tauri::command]
pub fn save_current_project_complete(
    state: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
) -> Result<String, String> {
    save_complete_workspace(None, &state, &activity_state)
}

#[tauri::command]
pub fn open_project_file_complete(
    path: String,
    state: tauri::State<'_, WorkspaceState>,
    activity_state: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    open_project_file_in_state(path, &state, &activity_state, &history)
}

pub(super) fn open_project_file_in_state(
    path: String,
    state: &WorkspaceState,
    activity_state: &activity_workspace::ActivityWorkspaceState,
    history: &history::HistoryState,
) -> Result<String, String> {
    let path = normalize_project_path(&path)?;
    if !path.exists() {
        return Err(format!("project file does not exist: {}", path.display()));
    }
    let database = ProjectDatabase::open(&path).map_err(|error| error.to_string())?;

    // Load and validate every repository before acquiring publication locks.
    let loaded = load_complete_project(&database)?;
    let CompleteProjectLoad {
        project,
        diagrams,
        ibd_diagrams,
        behavior,
        behavior_diagrams,
        reqif_exchange,
        activity_repository,
        activity_diagrams,
    } = loaded;
    let opened_path = path.to_string_lossy().into_owned();

    // Acquire every guard before changing the first field. Poisoned-lock
    // failures cannot publish a mixture of old and new repositories.
    let mut current_project = state.project.lock().map_err(|_| "project lock poisoned")?;
    let mut current_diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let mut current_ibd = state.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")?;
    let mut current_behavior = state
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?;
    let mut current_behavior_diagrams = state
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?;
    let mut current_reqif = state
        .reqif_exchange
        .lock()
        .map_err(|_| "ReqIF exchange lock poisoned")?;
    let mut current_activity = activity_state
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?;
    let mut current_activity_diagrams = activity_state
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?;
    let mut current_file = state
        .current_file
        .lock()
        .map_err(|_| "project path lock poisoned")?;

    // History belongs to this session, not a later frontend refresh. Acquiring
    // both history guards can still fail; do so before publishing any field.
    history::reset_states(history)?;
    *current_project = Some(project);
    *current_diagrams = diagrams;
    *current_ibd = ibd_diagrams;
    *current_behavior = behavior;
    *current_behavior_diagrams = behavior_diagrams;
    *current_reqif = reqif_exchange;
    *current_activity = activity_repository;
    *current_activity_diagrams = activity_diagrams;
    *current_file = Some(opened_path.clone());
    Ok(opened_path)
}

struct CompleteProjectLoad {
    project: Project,
    diagrams: Vec<BddDiagram>,
    ibd_diagrams: Vec<ibd::IbdDiagram>,
    behavior: BehaviorRepository,
    behavior_diagrams: Vec<behavior_workspace::BehaviorDiagram>,
    reqif_exchange: reqif_interchange::ReqifExchangeState,
    activity_repository: systems_modeler_core::ActivityRepository,
    activity_diagrams: Vec<activity_workspace::ActivityDiagram>,
}

fn load_complete_project(database: &ProjectDatabase) -> Result<CompleteProjectLoad, String> {
    let project = database
        .load_first_project()
        .map_err(|error| error.to_string())?;
    project
        .validate()
        .map_err(|error| format!("saved project validation failed: {error}"))?;
    let diagrams = match database
        .load_metadata(project.id, BDD_METADATA_KEY)
        .map_err(|error| error.to_string())?
    {
        Some(payload) => serde_json::from_str::<Vec<BddDiagram>>(&payload)
            .map_err(|error| format!("invalid saved BDD presentation data: {error}"))?,
        None => Vec::new(),
    };
    validate_loaded_diagrams(&project, &diagrams)?;
    let ibd_diagrams = ibd::load_ibd_metadata(database, &project)?;
    let (behavior, behavior_diagrams) =
        behavior_workspace::load_behavior_metadata(database, &project)?;
    let reqif_exchange = reqif_runtime::load_reqif_metadata(database, &project)?;
    let (activity_repository, activity_diagrams) =
        activity_workspace::load_activity_workspace_metadata(database, &project)?;
    Ok(CompleteProjectLoad {
        project,
        diagrams,
        ibd_diagrams,
        behavior,
        behavior_diagrams,
        reqif_exchange,
        activity_repository,
        activity_diagrams,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejected_legacy_details_preserve_model_and_redo() {
        let state = WorkspaceState::default();
        let activity = activity_workspace::ActivityWorkspaceState::default();
        let history = history::HistoryState::default();
        let mut project = Project::new("Details");
        let id = project
            .create_element(ElementKind::ValueType, "Speed", project.root_id)
            .unwrap();
        *state.project.lock().unwrap() = Some(project);
        history::checkpoint_states(&state, &activity, &history).unwrap();
        state.project.lock().unwrap().as_mut().unwrap().name = "Redo revision".into();
        assert!(history::undo_states(&state, &activity, &history).unwrap());
        let before = serde_json::to_value(&*state.project.lock().unwrap()).unwrap();

        for edit in [
            systems_modeler_core::ElementSpecificationEdit {
                documentation: Some("must not leak".into()),
                unit_external_id: Some("missing-unit".into()),
                ..Default::default()
            },
            systems_modeler_core::ElementSpecificationEdit {
                documentation: Some("must not leak".into()),
                type_id: Some(ElementId::new()),
                ..Default::default()
            },
        ] {
            assert!(apply_legacy_details(id, edit, &state, &activity, &history).is_err());
            assert_eq!(
                serde_json::to_value(&*state.project.lock().unwrap()).unwrap(),
                before
            );
            assert_eq!(history::undo_len(&history), 0);
        }
        assert!(history::redo_states(&state, &activity, &history).unwrap());
        assert_eq!(
            state.project.lock().unwrap().as_ref().unwrap().name,
            "Redo revision"
        );
    }

    #[test]
    fn legacy_details_commit_once_skip_noop_and_support_undo_redo() {
        let (state, activity) = save_session_fixture();
        let history = history::HistoryState::default();
        let id = state.project.lock().unwrap().as_ref().unwrap().root_id;
        let before = serde_json::to_value(&*state.project.lock().unwrap()).unwrap();
        let edit = systems_modeler_core::ElementSpecificationEdit {
            documentation: Some("authored documentation".into()),
            ..Default::default()
        };
        assert!(apply_legacy_details(id, edit.clone(), &state, &activity, &history).unwrap());
        let after = serde_json::to_value(&*state.project.lock().unwrap()).unwrap();
        assert_ne!(before, after);
        assert_eq!(history::undo_len(&history), 1);
        assert!(!apply_legacy_details(id, edit, &state, &activity, &history).unwrap());
        assert_eq!(history::undo_len(&history), 1);
        assert!(history::undo_states(&state, &activity, &history).unwrap());
        assert_eq!(
            serde_json::to_value(&*state.project.lock().unwrap()).unwrap(),
            before
        );
        assert!(history::redo_states(&state, &activity, &history).unwrap());
        assert_eq!(
            serde_json::to_value(&*state.project.lock().unwrap()).unwrap(),
            after
        );
    }

    fn save_session_fixture() -> (WorkspaceState, activity_workspace::ActivityWorkspaceState) {
        let state = WorkspaceState::default();
        let activity = activity_workspace::ActivityWorkspaceState::default();
        let mut project = Project::new("Authored project");
        project
            .create_element(ElementKind::Block, "System", project.root_id)
            .unwrap();
        activity
            .repository
            .lock()
            .unwrap()
            .create_activity(&project, project.root_id, None, "Operate")
            .unwrap();
        *state.project.lock().unwrap() = Some(project);
        (state, activity)
    }

    #[test]
    fn complete_save_poisoned_path_preserves_existing_target_and_creates_no_new_file() {
        let directory = tempfile::tempdir().unwrap();
        for existing in [false, true] {
            let (state, activity) = save_session_fixture();
            let path = directory.path().join(format!("target-{existing}.smproj"));
            let previous = Project::new("Previous revision");
            if existing {
                ProjectDatabase::open(&path)
                    .unwrap()
                    .save_project(&previous)
                    .unwrap();
            }
            let before = std::fs::read(&path).ok();
            let poisoned = std::panic::catch_unwind(|| {
                let _guard = state.current_file.lock().unwrap();
                panic!("injected path-lock failure");
            });
            assert!(poisoned.is_err());

            let error = save_complete_workspace(Some(path.to_str().unwrap()), &state, &activity)
                .unwrap_err();

            assert_eq!(error, "project path lock poisoned");
            assert_eq!(std::fs::read(&path).ok(), before);
            assert_eq!(path.exists(), existing);
            assert_eq!(
                state.project.lock().unwrap().as_ref().unwrap().name,
                "Authored project"
            );
        }
    }

    #[test]
    fn complete_save_filesystem_failure_preserves_current_path_and_previous_revision() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("original.smproj");
        let (state, activity) = save_session_fixture();
        let saved =
            save_complete_workspace(Some(original.to_str().unwrap()), &state, &activity).unwrap();
        state.project.lock().unwrap().as_mut().unwrap().name = "Unsaved changes".into();
        let invalid = directory.path().join("missing-parent/target.smproj");

        assert!(
            save_complete_workspace(Some(invalid.to_str().unwrap()), &state, &activity).is_err()
        );

        assert_eq!(
            state.current_file.lock().unwrap().as_deref(),
            Some(saved.as_str())
        );
        assert!(!invalid.exists());
        assert_eq!(
            ProjectDatabase::open(&original)
                .unwrap()
                .load_first_project()
                .unwrap()
                .name,
            "Authored project"
        );
        assert_eq!(
            state.project.lock().unwrap().as_ref().unwrap().name,
            "Unsaved changes"
        );
    }

    #[test]
    fn complete_save_current_preserves_identity_and_round_trips_activity_with_core() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("complete.smproj");
        let (state, activity) = save_session_fixture();
        let id = state.project.lock().unwrap().as_ref().unwrap().id;
        let saved =
            save_complete_workspace(Some(path.to_str().unwrap()), &state, &activity).unwrap();
        state.project.lock().unwrap().as_mut().unwrap().name = "Updated project".into();
        activity
            .repository
            .lock()
            .unwrap()
            .activities
            .values_mut()
            .next()
            .unwrap()
            .name = "Updated operation".into();

        assert_eq!(
            save_complete_workspace(None, &state, &activity).unwrap(),
            saved
        );

        let database = ProjectDatabase::open(&path).unwrap();
        let loaded = load_complete_project(&database).unwrap();
        assert_eq!(loaded.project.id, id);
        assert_eq!(loaded.project.name, "Updated project");
        assert_eq!(loaded.project.elements.len(), 2);
        assert_eq!(loaded.activity_repository.activities.len(), 1);
        assert_eq!(
            loaded
                .activity_repository
                .activities
                .values()
                .next()
                .unwrap()
                .name,
            "Updated operation"
        );
        assert_eq!(
            state.current_file.lock().unwrap().as_deref(),
            Some(saved.as_str())
        );
    }

    #[test]
    fn complete_save_invalid_activity_preserves_destination_and_session_path() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("complete.smproj");
        let (state, activity) = save_session_fixture();
        let saved =
            save_complete_workspace(Some(path.to_str().unwrap()), &state, &activity).unwrap();
        let before = std::fs::read(&path).unwrap();
        activity
            .repository
            .lock()
            .unwrap()
            .activities
            .values_mut()
            .next()
            .unwrap()
            .owner_id = ElementId::new();

        assert!(save_complete_workspace(None, &state, &activity).is_err());

        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(
            state.current_file.lock().unwrap().as_deref(),
            Some(saved.as_str())
        );
    }

    #[test]
    fn complete_save_prepares_every_authored_repository_before_commit() {
        let project = Project::new("Vehicle");
        let behavior = BehaviorRepository::default();
        let activity = systems_modeler_core::ActivityRepository::default();
        let reqif = reqif_interchange::ReqifExchangeState::default();
        let metadata = prepare_complete_project_metadata(CompleteWorkspaceRef {
            project: &project,
            diagrams: &[],
            ibd_diagrams: &[],
            behavior: &behavior,
            behavior_diagrams: &[],
            activity_repository: &activity,
            activity_diagrams: &[],
            reqif_exchange: &reqif,
        })
        .unwrap();
        let keys = metadata.iter().map(|(key, _)| *key).collect::<HashSet<_>>();

        assert_eq!(keys.len(), 7);
        for key in [
            BDD_METADATA_KEY,
            ibd::IBD_METADATA_KEY,
            behavior_workspace::BEHAVIOR_METADATA_KEY,
            behavior_workspace::BEHAVIOR_DIAGRAM_METADATA_KEY,
            systems_modeler_persistence::ACTIVITY_METADATA_KEY,
            activity_workspace::ACTIVITY_DIAGRAM_METADATA_KEY,
            reqif_interchange::REQIF_METADATA_KEY,
        ] {
            assert!(keys.contains(key), "complete Save omitted {key}");
        }
    }

    #[test]
    fn complete_open_rejects_malformed_activity_before_publication() {
        let project = Project::new("Candidate");
        let mut database = ProjectDatabase::open_in_memory().unwrap();
        database
            .save_project_with_metadata(
                &project,
                &[(
                    systems_modeler_persistence::ACTIVITY_METADATA_KEY,
                    "not-json",
                )],
            )
            .unwrap();

        let error = load_complete_project(&database).err().unwrap();
        assert!(error.contains("expected ident") || error.contains("expected value"));
    }

    #[test]
    fn complete_snapshot_preserves_namespace_endpoint_capabilities() {
        let mut project = Project::new("Vehicle");
        let package = project
            .create_element(ElementKind::Package, "Structure", project.root_id)
            .unwrap();
        let library = project
            .create_element(ElementKind::ModelLibrary, "Common Library", project.root_id)
            .unwrap();
        let block = project
            .create_element(ElementKind::Block, "Vehicle Type", project.root_id)
            .unwrap();

        let snapshot = snapshot_complete(&project);
        for namespace_id in [package, library] {
            let element = snapshot
                .elements
                .iter()
                .find(|element| element.id == namespace_id.to_string())
                .unwrap();
            assert!(element.namespace);
            assert!(element.packageable);
        }
        let block = snapshot
            .elements
            .iter()
            .find(|element| element.id == block.to_string())
            .unwrap();
        assert!(!block.namespace);
        assert!(block.packageable);
    }
}

#[cfg(test)]
mod composition_authoring_tests {
    use super::*;

    fn fixture() -> (WorkspaceState, String, ElementId, ElementId) {
        let state = WorkspaceState::default();
        let mut project = Project::new("Composition authoring");
        let root = project.root_id;
        let whole = project
            .create_element(ElementKind::Block, "Vehicle", root)
            .unwrap();
        let part = project
            .create_element(ElementKind::Block, "Wheel", root)
            .unwrap();
        let diagram_id = DiagramId::new().to_string();
        let nodes = [whole, part]
            .iter()
            .enumerate()
            .map(|(index, id)| DiagramNode {
                id: uuid::Uuid::new_v4().to_string(),
                element_id: id.to_string(),
                x: 100.0 + index as f64 * 350.0,
                y: 100.0 + index as f64 * 150.0,
                width: 180.0,
                height: 100.0,
                actor_notation: None,
                parameter_presentations: Vec::new(),
            })
            .collect();
        state.diagrams.lock().unwrap().push(BddDiagram {
            id: diagram_id.clone(),
            name: "Vehicle structure".into(),
            owner_id: root.to_string(),
            family: "bdd".into(),
            semantic_context_id: None,
            subject_boundary: None,
            nodes,
            edges: Vec::new(),
        });
        *state.project.lock().unwrap() = Some(project);
        (state, diagram_id, whole, part)
    }

    #[test]
    fn composition_creation_populates_ibd_with_distinct_part_property_identities() {
        let (state, diagram_id, whole, part) = fixture();
        let mut relationships = Vec::new();
        for _ in 0..2 {
            relationships.push(
                create_bdd_relationship_in_state(
                    diagram_id.clone(),
                    "Composition".into(),
                    whole.to_string(),
                    part.to_string(),
                    &state,
                )
                .unwrap(),
            );
        }
        let project_guard = state.project.lock().unwrap();
        let project = project_guard.as_ref().unwrap();
        project.validate().unwrap();
        let properties: Vec<_> = project.owned_features(whole).collect();
        assert_eq!(properties.len(), 2);
        assert!(properties.iter().any(|property| property.name == "wheel"));
        assert!(properties.iter().any(|property| property.name == "wheel2"));
        let mut ibd = ibd::IbdDiagram {
            id: DiagramId::new().to_string(),
            name: "Vehicle internals".into(),
            owner_id: project.root_id.to_string(),
            context_block_id: whole.to_string(),
            context_frame: None,
            properties: Vec::new(),
            boundary_ports: Vec::new(),
            connectors: Vec::new(),
        };
        ibd::populate_ibd_diagram_from_context(project, &mut ibd).unwrap();
        assert_eq!(ibd.properties.len(), 2);
        for relationship_id in relationships {
            let relation = project
                .relationship(parse_relationship_id(&relationship_id).unwrap())
                .unwrap();
            let property = relation.association_ends[1].property_id.unwrap();
            assert!(
                ibd.properties
                    .iter()
                    .any(|node| node.element_id == property.to_string())
            );
            let snapshot = association_end_snapshot(relation, 1, &relation.association_ends[1]);
            assert_eq!(snapshot.property_id, Some(property.to_string()));
            assert_eq!(snapshot.decoration_side.as_deref(), Some("source"));
        }
        let diagrams = state.diagrams.lock().unwrap();
        validate_loaded_diagrams(project, &diagrams).unwrap();
        assert_eq!(diagrams[0].edges.len(), 2);
        assert_ne!(diagrams[0].edges[0].points, diagrams[0].edges[1].points);
    }

    #[test]
    fn composition_route_failure_does_not_publish_a_part_or_relationship() {
        let (state, diagram_id, whole, part) = fixture();
        state.diagrams.lock().unwrap()[0].nodes[1].x = f64::NAN;
        let before = serde_json::to_value(&*state.project.lock().unwrap()).unwrap();
        assert!(
            create_bdd_relationship_in_state(
                diagram_id,
                "Composition".into(),
                whole.to_string(),
                part.to_string(),
                &state,
            )
            .is_err()
        );
        assert_eq!(
            serde_json::to_value(&*state.project.lock().unwrap()).unwrap(),
            before
        );
        assert!(state.diagrams.lock().unwrap()[0].edges.is_empty());
    }
}
