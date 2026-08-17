use super::*;

#[derive(Debug, Clone, Serialize)]
pub struct CompleteElementSnapshot {
    pub id: String,
    pub external_id: String,
    pub kind: String,
    pub name: String,
    pub owner_id: Option<String>,
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
    pub current_file: Option<String>,
}

fn bdd_presentable(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Block
            | ElementKind::InterfaceBlock
            | ElementKind::ValueType
            | ElementKind::DataType
            | ElementKind::Enumeration
            | ElementKind::ConstraintBlock
    )
}

fn parse_kind(value: &str) -> Result<ElementKind, String> {
    match value {
        "Block" => Ok(ElementKind::Block),
        "InterfaceBlock" => Ok(ElementKind::InterfaceBlock),
        "ValueType" => Ok(ElementKind::ValueType),
        "DataType" => Ok(ElementKind::DataType),
        "Enumeration" => Ok(ElementKind::Enumeration),
        "ConstraintBlock" => Ok(ElementKind::ConstraintBlock),
        "EnumerationLiteral" => Ok(ElementKind::EnumerationLiteral),
        "PartProperty" => Ok(ElementKind::PartProperty),
        "ReferenceProperty" => Ok(ElementKind::ReferenceProperty),
        "ValueProperty" => Ok(ElementKind::ValueProperty),
        "ConstraintProperty" => Ok(ElementKind::ConstraintProperty),
        "ProxyPort" => Ok(ElementKind::ProxyPort),
        "FullPort" => Ok(ElementKind::FullPort),
        "Operation" => Ok(ElementKind::Operation),
        "Parameter" => Ok(ElementKind::Parameter),
        "Reception" => Ok(ElementKind::Reception),
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
            parameter_direction: element.parameter_direction.map(direction_name).map(str::to_string),
            literal_value: element.literal_value.clone(),
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
            source_id: relationship.source_id.to_string(),
            target_id: relationship.target_id.to_string(),
            association_ends: relationship
                .association_ends
                .iter()
                .map(|end| AssociationEndSnapshot {
                    id: end.id.to_string(),
                    classifier_id: end.classifier_id.to_string(),
                    role_name: end.role_name.clone(),
                    multiplicity: end.multiplicity.notation(),
                    navigable: end.navigable,
                    aggregation: aggregation_name(end.aggregation).to_string(),
                })
                .collect(),
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
        let owner = project.element(owner_id).map_err(|error| error.to_string())?;
        if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
            return Err(format!("BDD owner is not a Model or Package: {}", diagram.owner_id));
        }
        for node in &diagram.nodes {
            if uuid::Uuid::parse_str(&node.id).is_err() {
                return Err(format!("invalid diagram node id: {}", node.id));
            }
            if !node_ids.insert(&node.id) {
                return Err(format!("duplicate diagram node id: {}", node.id));
            }
            let element_id = parse_element_id(&node.element_id)?;
            let element = project.element(element_id).map_err(|error| error.to_string())?;
            if !bdd_presentable(&element.kind) {
                return Err(format!("element kind {:?} is not valid as a BDD node", element.kind));
            }
        }
        for edge in &diagram.edges {
            if uuid::Uuid::parse_str(&edge.id).is_err() {
                return Err(format!("invalid diagram edge id: {}", edge.id));
            }
            if !edge_ids.insert(&edge.id) {
                return Err(format!("duplicate diagram edge id: {}", edge.id));
            }
            let relationship_id = parse_relationship_id(&edge.relationship_id)?;
            let relationship = project.relationship(relationship_id).map_err(|error| error.to_string())?;
            let source = diagram.nodes.iter().find(|node| node.id == edge.source_node_id)
                .ok_or_else(|| format!("edge source node not found: {}", edge.source_node_id))?;
            let target = diagram.nodes.iter().find(|node| node.id == edge.target_node_id)
                .ok_or_else(|| format!("edge target node not found: {}", edge.target_node_id))?;
            if source.element_id != relationship.source_id.to_string()
                || target.element_id != relationship.target_id.to_string()
            {
                return Err(format!("diagram edge endpoints do not match semantic relationship: {}", edge.relationship_id));
            }
            if edge.points.len() < 2 {
                return Err(format!("diagram edge has no usable route: {}", edge.id));
            }
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
    let current_file = state.current_file.lock().map_err(|_| "project path lock poisoned")?;
    Ok(CompleteWorkspaceSnapshot {
        project: project.as_ref().map(snapshot_complete),
        diagrams: diagrams.clone(),
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
        return Err(format!("{kind:?} is an owned feature, not a top-level BDD classifier"));
    }
    create_element(kind, owner_id, name, state)
}

fn parse_multiplicity(lower: u32, upper: Option<u32>) -> Result<Multiplicity, String> {
    Multiplicity::new(lower, upper).map_err(|error| error.to_string())
}

#[tauri::command]
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
        ElementKind::EnumerationLiteral => project
            .create_element(ElementKind::EnumerationLiteral, name, owner_id)
            .map_err(|error| error.to_string())?,
        ElementKind::Operation | ElementKind::Reception => project
            .create_element(kind, name, owner_id)
            .map_err(|error| error.to_string())?,
        ElementKind::PartProperty
        | ElementKind::ReferenceProperty
        | ElementKind::ValueProperty
        | ElementKind::ConstraintProperty
        | ElementKind::ProxyPort
        | ElementKind::FullPort
        | ElementKind::Parameter => {
            let type_id = type_id.ok_or_else(|| format!("{kind:?} requires a compatible stable type ID"))?;
            let type_id = parse_element_id(&type_id)?;
            let multiplicity = parse_multiplicity(lower.unwrap_or(1), upper.or(Some(1)))?;
            project
                .create_typed_feature(kind, name, owner_id, type_id, multiplicity)
                .map_err(|error| error.to_string())?
        }
        _ => return Err(format!("{kind:?} is not an owned BDD feature")),
    };

    if let Some(default_value) = default_value {
        project.element_mut(id).map_err(|error| error.to_string())?.default_value = Some(default_value);
    }
    project.validate_element(id).map_err(|error| error.to_string())?;
    Ok(id.to_string())
}

#[tauri::command]
pub fn update_bdd_element_details(
    element_id: String,
    documentation: Option<String>,
    default_value: Option<String>,
    quantity_kind_external_id: Option<String>,
    unit_external_id: Option<String>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let element_id = parse_element_id(&element_id)?;
    let mut project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_mut().ok_or("no project open")?;
    let element = project.element_mut(element_id).map_err(|error| error.to_string())?;
    if let Some(value) = documentation { element.documentation = value; }
    if let Some(value) = default_value { element.default_value = if value.is_empty() { None } else { Some(value) }; }
    if let Some(value) = quantity_kind_external_id { element.quantity_kind_external_id = if value.is_empty() { None } else { Some(value) }; }
    if let Some(value) = unit_external_id { element.unit_external_id = if value.is_empty() { None } else { Some(value) }; }
    project.validate_element(element_id).map_err(|error| error.to_string())
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
        let element = project.as_ref().ok_or("no project open")?.element(element_id)
            .map_err(|error| error.to_string())?;
        if !bdd_presentable(&element.kind) {
            return Err(format!("{:?} is not valid as a BDD node", element.kind));
        }
        match element.kind {
            ElementKind::Enumeration => (190.0, 125.0),
            ElementKind::ConstraintBlock => (200.0, 120.0),
            ElementKind::ValueType | ElementKind::DataType => (185.0, 100.0),
            _ => (190.0, 115.0),
        }
    };

    let mut diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let diagram = diagrams.iter_mut().find(|diagram| diagram.id == diagram_id.to_string())
        .ok_or("diagram not found")?;
    if diagram.nodes.iter().any(|node| node.element_id == element_id.to_string()) {
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
    let kind = supported_relationship_kind(&kind)?;
    let source_id = parse_element_id(&source_element_id)?;
    let target_id = parse_element_id(&target_element_id)?;
    if source_id == target_id {
        return Err(format!("{kind} cannot connect an element to itself"));
    }

    let mut project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_mut().ok_or("no project open")?;
    let source = project.element(source_id).map_err(|error| error.to_string())?;
    let target = project.element(target_id).map_err(|error| error.to_string())?;
    if !source.is_classifier() || !target.is_classifier() {
        return Err(format!("{kind} requires classifier endpoints on a BDD"));
    }
    if semantic_duplicate(project, kind, source_id, target_id) {
        return Err(format!("an equivalent {kind} already exists"));
    }

    let mut diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let diagram = diagrams.iter_mut().find(|diagram| diagram.id == diagram_id).ok_or("diagram not found")?;
    let source_node = diagram.nodes.iter().find(|node| node.element_id == source_element_id).cloned()
        .ok_or("source classifier must be presented on the selected BDD")?;
    let target_node = diagram.nodes.iter().find(|node| node.element_id == target_element_id).cloned()
        .ok_or("target classifier must be presented on the selected BDD")?;
    let owner_id = Some(parse_element_id(&diagram.owner_id)?);

    let relationship_id = match kind {
        "Association" | "Aggregation" | "Composition" => {
            let aggregation = match kind {
                "Aggregation" => AggregationKind::Shared,
                "Composition" => AggregationKind::Composite,
                _ => AggregationKind::None,
            };
            project.create_association(
                owner_id,
                vec![
                    Project::association_end(source_id, "", Multiplicity::ONE, true, aggregation),
                    Project::association_end(target_id, "", Multiplicity::ONE, true, AggregationKind::None),
                ],
            ).map_err(|error| error.to_string())?
        }
        "Generalization" => project.create_relationship(RelationshipKind::Generalization, source_id, target_id, owner_id)
            .map_err(|error| error.to_string())?,
        "Dependency" => project.create_relationship(RelationshipKind::Dependency, source_id, target_id, owner_id)
            .map_err(|error| error.to_string())?,
        "Realization" => project.create_relationship(RelationshipKind::Realization, source_id, target_id, owner_id)
            .map_err(|error| error.to_string())?,
        _ => unreachable!(),
    };

    let points = route_relationship(&source_node, &target_node, &diagram.nodes);
    diagram.edges.push(DiagramEdge {
        id: uuid::Uuid::new_v4().to_string(),
        relationship_id: relationship_id.to_string(),
        source_node_id: source_node.id,
        target_node_id: target_node.id,
        points,
    });
    Ok(relationship_id.to_string())
}

#[tauri::command]
pub fn save_project_file_complete(
    path: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let path = normalize_project_path(&path)?;
    let project = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project.as_ref().ok_or("no project open")?;
    let diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    project.validate().map_err(|error| format!("project validation failed: {error}"))?;
    validate_complete_diagrams(project, &diagrams)?;
    let mut database = ProjectDatabase::open(&path).map_err(|error| error.to_string())?;
    database.save_project(project).map_err(|error| error.to_string())?;
    let diagram_payload = serde_json::to_string(&*diagrams).map_err(|error| error.to_string())?;
    database.save_metadata(project.id, BDD_METADATA_KEY, &diagram_payload).map_err(|error| error.to_string())?;
    let saved_path = path.to_string_lossy().into_owned();
    *state.current_file.lock().map_err(|_| "project path lock poisoned")? = Some(saved_path.clone());
    Ok(saved_path)
}

#[tauri::command]
pub fn save_current_project_complete(
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let path = state.current_file.lock().map_err(|_| "project path lock poisoned")?.clone()
        .ok_or("project has not been saved yet; use Save As")?;
    save_project_file_complete(path, state)
}

#[tauri::command]
pub fn open_project_file_complete(
    path: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let path = normalize_project_path(&path)?;
    if !path.exists() { return Err(format!("project file does not exist: {}", path.display())); }
    let database = ProjectDatabase::open(&path).map_err(|error| error.to_string())?;
    let project = database.load_first_project().map_err(|error| error.to_string())?;
    project.validate().map_err(|error| format!("saved project validation failed: {error}"))?;
    let diagrams = match database.load_metadata(project.id, BDD_METADATA_KEY).map_err(|error| error.to_string())? {
        Some(payload) => serde_json::from_str::<Vec<BddDiagram>>(&payload)
            .map_err(|error| format!("invalid saved BDD presentation data: {error}"))?,
        None => Vec::new(),
    };
    validate_complete_diagrams(&project, &diagrams)?;
    let opened_path = path.to_string_lossy().into_owned();
    *state.project.lock().map_err(|_| "project lock poisoned")? = Some(project);
    *state.diagrams.lock().map_err(|_| "diagram lock poisoned")? = diagrams;
    *state.current_file.lock().map_err(|_| "project path lock poisoned")? = Some(opened_path.clone());
    Ok(opened_path)
}
