//! Rust-authoritative Package Diagram semantics and presentations.

use super::*;
use systems_modeler_core::{ElementKind, Project, RelationshipKind, VisibilityKind};

const PACKAGE_MIN_WIDTH: f64 = 120.0;
const PACKAGE_MIN_HEIGHT: f64 = 70.0;

fn checkpoint(
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &history::HistoryState,
) -> Result<(), String> {
    history::checkpoint_states(workspace, activity, history)
}

fn candidate_states(workspace: &WorkspaceState) -> Result<(Project, Vec<BddDiagram>), String> {
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    Ok((project, diagrams))
}

fn commit_states(
    project: Project,
    diagrams: Vec<BddDiagram>,
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &history::HistoryState,
) -> Result<(), String> {
    project.validate().map_err(|error| error.to_string())?;
    for diagram in diagrams.iter().filter(|diagram| diagram.family == "package") {
        validate_package_diagram(&project, diagram)?;
    }
    checkpoint(workspace, activity, history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(())
}

fn parse_visibility(value: &str) -> Result<VisibilityKind, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "public" => Ok(VisibilityKind::Public),
        "private" => Ok(VisibilityKind::Private),
        _ => Err("visibility must be public or private".into()),
    }
}

fn package_node_kind(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Package | ElementKind::ModelLibrary | ElementKind::Comment
    )
}

fn package_presentable(element: &systems_modeler_core::Element) -> bool {
    element.is_packageable() || element.kind == ElementKind::Comment
}

fn package_endpoint_kind(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Model | ElementKind::Package | ElementKind::ModelLibrary
    )
}

fn package_relationship_kind(kind: &RelationshipKind) -> bool {
    matches!(
        kind,
        RelationshipKind::PackageImport
            | RelationshipKind::ElementImport
            | RelationshipKind::PackageMerge
            | RelationshipKind::Dependency
    )
}

fn package_relationship_name(kind: &RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::PackageImport => "PackageImport",
        RelationshipKind::ElementImport => "ElementImport",
        RelationshipKind::PackageMerge => "PackageMerge",
        RelationshipKind::Dependency => "Dependency",
        _ => "Unsupported",
    }
}

fn package_element_kind(value: &str) -> Result<ElementKind, String> {
    match value {
        "Package" => Ok(ElementKind::Package),
        "ModelLibrary" => Ok(ElementKind::ModelLibrary),
        "Comment" => Ok(ElementKind::Comment),
        _ => Err(format!("{value} is not creatable from a Package Diagram")),
    }
}

fn package_relationship_semantic_kind(value: &str) -> Result<RelationshipKind, String> {
    match value {
        "PackageImport" => Ok(RelationshipKind::PackageImport),
        "ElementImport" => Ok(RelationshipKind::ElementImport),
        "PackageMerge" => Ok(RelationshipKind::PackageMerge),
        "Dependency" => Ok(RelationshipKind::Dependency),
        _ => Err(format!("unsupported Package Diagram relationship: {value}")),
    }
}

fn diagram_index(diagrams: &[BddDiagram], diagram_id: &str) -> Result<usize, String> {
    diagrams
        .iter()
        .position(|diagram| diagram.id == diagram_id && diagram.family == "package")
        .ok_or_else(|| "Package Diagram not found".into())
}

fn node_for_element<'a>(
    diagram: &'a BddDiagram,
    element_id: &str,
) -> Result<&'a DiagramNode, String> {
    diagram
        .nodes
        .iter()
        .find(|node| node.element_id == element_id)
        .ok_or_else(|| "relationship endpoint must be presented on the Package Diagram".into())
}

fn dependency_endpoints(
    project: &Project,
    source_id: ElementId,
    target_id: ElementId,
) -> Result<(), String> {
    let source = project
        .element(source_id)
        .map_err(|error| error.to_string())?;
    let target = project
        .element(target_id)
        .map_err(|error| error.to_string())?;
    if !package_endpoint_kind(&source.kind) || !package_endpoint_kind(&target.kind) {
        return Err(format!(
            "Package-level Dependency requires Package, Model, or ModelLibrary endpoints; received '{}' ({:?}) -> '{}' ({:?})",
            source.name, source.kind, target.name, target.kind
        ));
    }
    if source_id == target_id {
        return Err(format!("Dependency cannot connect '{}' to itself", source.name));
    }
    Ok(())
}

fn semantic_duplicate(
    project: &Project,
    ignored_id: Option<RelationshipId>,
    kind: &RelationshipKind,
    source_id: ElementId,
    target_id: ElementId,
) -> bool {
    project.relationships.values().any(|relationship| {
        Some(relationship.id) != ignored_id
            && relationship.kind == *kind
            && relationship.source_id == source_id
            && relationship.target_id == target_id
    })
}

fn reroute(diagram: &mut BddDiagram) -> Result<(), String> {
    diagram.edges = routed_bdd_edges(diagram, None)?;
    Ok(())
}

fn reconnect_candidate(
    project: &mut Project,
    diagram: &mut BddDiagram,
    relationship_id: RelationshipId,
    source_id: ElementId,
    target_id: ElementId,
) -> Result<(), String> {
    let source_node = node_for_element(diagram, &source_id.to_string())?.clone();
    let target_node = node_for_element(diagram, &target_id.to_string())?.clone();
    let kind = project
        .relationship(relationship_id)
        .map_err(|error| error.to_string())?
        .kind
        .clone();
    if !package_relationship_kind(&kind) {
        return Err("selected relationship is not a Package Diagram relationship".into());
    }
    if semantic_duplicate(project, Some(relationship_id), &kind, source_id, target_id) {
        let source = project
            .element(source_id)
            .map_err(|error| error.to_string())?;
        let target = project
            .element(target_id)
            .map_err(|error| error.to_string())?;
        return Err(format!(
            "an equivalent {} already exists: '{}' -> '{}'",
            package_relationship_name(&kind),
            source.name,
            target.name,
        ));
    }
    if kind == RelationshipKind::Dependency {
        dependency_endpoints(project, source_id, target_id)?;
    }
    {
        let relationship = project
            .relationships
            .get_mut(&relationship_id)
            .ok_or("Package relationship not found")?;
        relationship.source_id = source_id;
        relationship.target_id = target_id;
        relationship.owner_id = Some(source_id);
    }
    project.validate().map_err(|error| error.to_string())?;
    let edge = diagram
        .edges
        .iter_mut()
        .find(|edge| edge.relationship_id == relationship_id.to_string())
        .ok_or("Package relationship presentation not found")?;
    edge.source_node_id = source_node.id;
    edge.target_node_id = target_node.id;
    reroute(diagram)
}

fn delete_relationship_candidate(
    project: &mut Project,
    diagrams: &mut [BddDiagram],
    relationship_id: RelationshipId,
) -> Result<(), String> {
    let relationship = project
        .relationship(relationship_id)
        .map_err(|error| error.to_string())?;
    if !package_relationship_kind(&relationship.kind) {
        return Err("selected relationship is not a Package Diagram relationship".into());
    }
    project.relationships.remove(&relationship_id);
    for diagram in diagrams {
        diagram
            .edges
            .retain(|edge| edge.relationship_id != relationship_id.to_string());
    }
    Ok(())
}

pub(super) fn validate_package_diagram(
    project: &Project,
    diagram: &BddDiagram,
) -> Result<(), String> {
    if diagram.family != "package" {
        return Err("target diagram is not a Package Diagram".into());
    }
    let owner = project
        .element(parse_element_id(&diagram.owner_id)?)
        .map_err(|error| error.to_string())?;
    if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
        return Err(format!(
            "Package Diagram owner '{}' must be a Model or Package",
            owner.name
        ));
    }

    let mut presentation_ids = HashSet::new();
    for node in &diagram.nodes {
        if !presentation_ids.insert(&node.id) {
            return Err(format!("duplicate Package presentation id: {}", node.id));
        }
        let element = project
            .element(parse_element_id(&node.element_id)?)
            .map_err(|error| error.to_string())?;
        if !package_presentable(element) {
            return Err(format!(
                "'{}' ({:?}) cannot be presented on a Package Diagram",
                element.name, element.kind
            ));
        }
        if !node.x.is_finite()
            || !node.y.is_finite()
            || !node.width.is_finite()
            || !node.height.is_finite()
            || node.x < 0.0
            || node.y < 42.0
            || node.width < PACKAGE_MIN_WIDTH
            || node.height < PACKAGE_MIN_HEIGHT
        {
            return Err(format!(
                "invalid Package presentation geometry for '{}'",
                element.name
            ));
        }
    }

    for edge in &diagram.edges {
        if !presentation_ids.insert(&edge.id) {
            return Err(format!(
                "duplicate Package relationship presentation id: {}",
                edge.id
            ));
        }
        let relationship = project
            .relationship(parse_relationship_id(&edge.relationship_id)?)
            .map_err(|error| error.to_string())?;
        if !package_relationship_kind(&relationship.kind) {
            return Err(format!(
                "{} is not valid on a Package Diagram",
                relationship_display_kind(relationship)
            ));
        }
        let source = diagram
            .nodes
            .iter()
            .find(|node| node.id == edge.source_node_id)
            .ok_or(
                "Package relationship presentation references a missing source presentation",
            )?;
        let target = diagram
            .nodes
            .iter()
            .find(|node| node.id == edge.target_node_id)
            .ok_or(
                "Package relationship presentation references a missing target presentation",
            )?;
        if source.element_id != relationship.source_id.to_string()
            || target.element_id != relationship.target_id.to_string()
        {
            let semantic_source = project
                .element(relationship.source_id)
                .map_err(|error| error.to_string())?;
            let semantic_target = project
                .element(relationship.target_id)
                .map_err(|error| error.to_string())?;
            return Err(format!(
                "{} presentation endpoints do not match its Rust semantic endpoints '{} -> {}'",
                package_relationship_name(&relationship.kind),
                semantic_source.name,
                semantic_target.name,
            ));
        }
        if relationship.kind == RelationshipKind::Dependency {
            dependency_endpoints(project, relationship.source_id, relationship.target_id)?;
        }
        if edge.points.len() < 2
            || edge
                .points
                .iter()
                .any(|point| !point.x.is_finite() || !point.y.is_finite())
            || edge.label_anchor.is_none()
        {
            return Err(format!(
                "{} presentation has invalid route or label geometry",
                package_relationship_name(&relationship.kind)
            ));
        }
    }
    Ok(())
}

#[tauri::command]
pub fn create_package_diagram(
    owner_id: String,
    name: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let owner_id = parse_element_id(&owner_id)?;
    let name = name.trim();
    if name.is_empty() {
        return Err("Package Diagram name cannot be empty".into());
    }
    if name.chars().count() > 256 {
        return Err("Package Diagram name cannot exceed 256 characters".into());
    }
    let (project, mut diagrams) = candidate_states(&workspace)?;
    let owner = project
        .element(owner_id)
        .map_err(|error| error.to_string())?;
    if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
        return Err(format!(
            "Package Diagram owner '{}' must be a Model or Package",
            owner.name
        ));
    }
    let id = DiagramId::new().to_string();
    diagrams.push(BddDiagram {
        id: id.clone(),
        name: name.into(),
        owner_id: owner_id.to_string(),
        family: "package".into(),
        semantic_context_id: None,
        subject_boundary: None,
        nodes: Vec::new(),
        edges: Vec::new(),
    });
    commit_states(project, diagrams, &workspace, &activity, &history)?;
    Ok(id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_package_element(
    diagram_id: String,
    kind: String,
    name: String,
    x: f64,
    y: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    parse_diagram_id(&diagram_id)?;
    let kind = package_element_kind(&kind)?;
    let name = name.trim();
    if name.is_empty() {
        return Err(format!("{kind:?} name cannot be empty"));
    }
    if !x.is_finite() || !y.is_finite() {
        return Err("Package presentation coordinates must be finite".into());
    }
    let (mut project, mut diagrams) = candidate_states(&workspace)?;
    let index = diagram_index(&diagrams, &diagram_id)?;
    let owner_id = parse_element_id(&diagrams[index].owner_id)?;
    let element_id = project
        .create_element(kind.clone(), name, owner_id)
        .map_err(|error| error.to_string())?;
    let (width, height) = if kind == ElementKind::Comment {
        (210.0, 90.0)
    } else {
        (220.0, 120.0)
    };
    diagrams[index].nodes.push(DiagramNode {
        id: uuid::Uuid::new_v4().to_string(),
        element_id: element_id.to_string(),
        x: x.max(0.0),
        y: y.max(42.0),
        width,
        height,
        actor_notation: None,
        parameter_presentations: Vec::new(),
    });
    commit_states(project, diagrams, &workspace, &activity, &history)?;
    Ok(element_id.to_string())
}

#[tauri::command]
pub fn place_on_package_diagram(
    diagram_id: String,
    element_id: String,
    x: f64,
    y: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    parse_diagram_id(&diagram_id)?;
    let element_id = parse_element_id(&element_id)?;
    if !x.is_finite() || !y.is_finite() {
        return Err("Package presentation coordinates must be finite".into());
    }
    let (project, mut diagrams) = candidate_states(&workspace)?;
    let element = project
        .element(element_id)
        .map_err(|error| error.to_string())?;
    if !package_presentable(element) {
        return Err(format!(
            "'{}' ({:?}) cannot be presented on a Package Diagram",
            element.name, element.kind
        ));
    }
    let index = diagram_index(&diagrams, &diagram_id)?;
    if diagrams[index]
        .nodes
        .iter()
        .any(|node| node.element_id == element_id.to_string())
    {
        return Err(format!(
            "'{}' is already presented on this Package Diagram",
            element.name
        ));
    }
    let presentation_id = uuid::Uuid::new_v4().to_string();
    let (width, height) = if element.kind == ElementKind::Comment {
        (210.0, 90.0)
    } else {
        (220.0, 120.0)
    };
    diagrams[index].nodes.push(DiagramNode {
        id: presentation_id.clone(),
        element_id: element_id.to_string(),
        x: x.max(0.0),
        y: y.max(42.0),
        width,
        height,
        actor_notation: None,
        parameter_presentations: Vec::new(),
    });
    commit_states(project, diagrams, &workspace, &activity, &history)?;
    Ok(presentation_id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_package_relationship(
    diagram_id: String,
    kind: String,
    source_element_id: String,
    target_element_id: String,
    visibility: Option<String>,
    alias: Option<String>,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    parse_diagram_id(&diagram_id)?;
    let kind = package_relationship_semantic_kind(&kind)?;
    let source_id = parse_element_id(&source_element_id)?;
    let target_id = parse_element_id(&target_element_id)?;
    let visibility = parse_visibility(visibility.as_deref().unwrap_or("public"))?;
    let (mut project, mut diagrams) = candidate_states(&workspace)?;
    let index = diagram_index(&diagrams, &diagram_id)?;
    let source_node = node_for_element(&diagrams[index], &source_element_id)?.clone();
    let target_node = node_for_element(&diagrams[index], &target_element_id)?.clone();
    if semantic_duplicate(&project, None, &kind, source_id, target_id) {
        let source = project
            .element(source_id)
            .map_err(|error| error.to_string())?;
        let target = project
            .element(target_id)
            .map_err(|error| error.to_string())?;
        return Err(format!(
            "an equivalent {} already exists: '{}' -> '{}'",
            package_relationship_name(&kind),
            source.name,
            target.name
        ));
    }
    let relationship_id = match kind {
        RelationshipKind::PackageImport => {
            project.create_package_import(source_id, target_id, visibility)
        }
        RelationshipKind::ElementImport => {
            project.create_element_import(source_id, target_id, visibility, alias)
        }
        RelationshipKind::PackageMerge => project.create_package_merge(source_id, target_id),
        RelationshipKind::Dependency => {
            dependency_endpoints(&project, source_id, target_id)?;
            project.create_relationship(kind, source_id, target_id, Some(source_id))
        }
        _ => unreachable!(),
    }
    .map_err(|error| error.to_string())?;
    let points = route_relationship(&source_node, &target_node, &diagrams[index].nodes)?;
    diagrams[index].edges.push(DiagramEdge {
        id: uuid::Uuid::new_v4().to_string(),
        relationship_id: relationship_id.to_string(),
        source_node_id: source_node.id,
        target_node_id: target_node.id,
        points,
        label_anchor: None,
    });
    reroute(&mut diagrams[index])?;
    commit_states(project, diagrams, &workspace, &activity, &history)?;
    Ok(relationship_id.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn reconnect_package_relationship(
    diagram_id: String,
    relationship_id: String,
    source_element_id: String,
    target_element_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    parse_diagram_id(&diagram_id)?;
    let relationship_id = parse_relationship_id(&relationship_id)?;
    let source_id = parse_element_id(&source_element_id)?;
    let target_id = parse_element_id(&target_element_id)?;
    let (mut project, mut diagrams) = candidate_states(&workspace)?;
    let index = diagram_index(&diagrams, &diagram_id)?;
    reconnect_candidate(
        &mut project,
        &mut diagrams[index],
        relationship_id,
        source_id,
        target_id,
    )?;
    commit_states(project, diagrams, &workspace, &activity, &history)
}

#[tauri::command]
pub fn delete_package_relationship(
    relationship_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let relationship_id = parse_relationship_id(&relationship_id)?;
    let (mut project, mut diagrams) = candidate_states(&workspace)?;
    delete_relationship_candidate(&mut project, &mut diagrams, relationship_id)?;
    commit_states(project, diagrams, &workspace, &activity, &history)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_package_element(
    element_id: String,
    name: String,
    documentation: String,
    visibility: String,
    owner_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let element_id = parse_element_id(&element_id)?;
    let owner_id = parse_element_id(&owner_id)?;
    let name = name.trim();
    if name.is_empty() {
        return Err("Package Diagram element name cannot be empty".into());
    }
    let visibility = parse_visibility(&visibility)?;
    let (mut project, diagrams) = candidate_states(&workspace)?;
    let element = project
        .element(element_id)
        .map_err(|error| error.to_string())?;
    if !package_node_kind(&element.kind) {
        return Err(format!(
            "{:?} is not editable as a Package Diagram element",
            element.kind
        ));
    }
    if element.owner_id != Some(owner_id) {
        project
            .move_element(element_id, owner_id)
            .map_err(|error| error.to_string())?;
    }
    let element = project
        .element_mut(element_id)
        .map_err(|error| error.to_string())?;
    element.name = name.into();
    element.documentation = documentation;
    element.visibility = visibility;
    commit_states(project, diagrams, &workspace, &activity, &history)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_package_relationship(
    diagram_id: String,
    relationship_id: String,
    source_element_id: String,
    target_element_id: String,
    name: String,
    documentation: String,
    visibility: String,
    alias: Option<String>,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    parse_diagram_id(&diagram_id)?;
    let relationship_id = parse_relationship_id(&relationship_id)?;
    let source_id = parse_element_id(&source_element_id)?;
    let target_id = parse_element_id(&target_element_id)?;
    let visibility = parse_visibility(&visibility)?;
    let (mut project, mut diagrams) = candidate_states(&workspace)?;
    let index = diagram_index(&diagrams, &diagram_id)?;
    let kind = project
        .relationship(relationship_id)
        .map_err(|error| error.to_string())?
        .kind
        .clone();
    if !package_relationship_kind(&kind) {
        return Err("selected relationship is not a Package Diagram relationship".into());
    }
    reconnect_candidate(
        &mut project,
        &mut diagrams[index],
        relationship_id,
        source_id,
        target_id,
    )?;
    let relationship = project
        .relationships
        .get_mut(&relationship_id)
        .ok_or("Package relationship not found")?;
    relationship.name = name.trim().to_owned();
    relationship.documentation = documentation;
    relationship.visibility = visibility;
    relationship.alias = if kind == RelationshipKind::ElementImport {
        alias
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    } else {
        None
    };
    project.validate().map_err(|error| error.to_string())?;
    commit_states(project, diagrams, &workspace, &activity, &history)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, element_id: ElementId, x: f64, y: f64) -> DiagramNode {
        DiagramNode {
            id: id.into(),
            element_id: element_id.to_string(),
            x,
            y,
            width: 220.0,
            height: 120.0,
            actor_notation: None,
            parameter_presentations: Vec::new(),
        }
    }

    #[test]
    fn package_palette_semantics_are_typed_and_validate_names() {
        assert_eq!(
            package_element_kind("Package").unwrap(),
            ElementKind::Package
        );
        assert_eq!(
            package_element_kind("ModelLibrary").unwrap(),
            ElementKind::ModelLibrary
        );
        assert_eq!(
            package_relationship_semantic_kind("ElementImport").unwrap(),
            RelationshipKind::ElementImport
        );
        assert!(package_element_kind("Block").is_err());
        assert!(package_relationship_semantic_kind("Association").is_err());
    }

    #[test]
    fn dependency_validation_reports_engineering_names() {
        let mut project = Project::new("Vehicle");
        let root = project.root_id;
        let package = project
            .create_element(ElementKind::Package, "Architecture", root)
            .unwrap();
        let comment = project
            .create_element(ElementKind::Comment, "Note", root)
            .unwrap();
        let error = dependency_endpoints(&project, package, comment).unwrap_err();
        assert!(error.contains("Architecture"));
        assert!(error.contains("Note"));
        assert!(!error.contains(&comment.to_string()));
    }

    #[test]
    fn package_routes_reserve_tabs_labels_and_parallel_lanes() {
        let mut project = Project::new("Vehicle");
        let source = project
            .create_element(ElementKind::Package, "Vehicle", project.root_id)
            .unwrap();
        let target = project
            .create_element(ElementKind::Package, "Powertrain", project.root_id)
            .unwrap();
        let obstacle = project
            .create_element(ElementKind::Package, "Shared Types", project.root_id)
            .unwrap();
        let first = project
            .create_package_import(source, target, VisibilityKind::Public)
            .unwrap();
        let second = project
            .create_element_import(source, target, VisibilityKind::Private, Some("Drive".into()))
            .unwrap();
        let diagram = BddDiagram {
            id: DiagramId::new().to_string(),
            name: "Packages".into(),
            owner_id: project.root_id.to_string(),
            family: "package".into(),
            semantic_context_id: None,
            subject_boundary: None,
            nodes: vec![
                node("source", source, 80.0, 180.0),
                node("target", target, 720.0, 180.0),
                node("obstacle", obstacle, 400.0, 140.0),
            ],
            edges: vec![
                DiagramEdge {
                    id: "edge-a".into(),
                    relationship_id: first.to_string(),
                    source_node_id: "source".into(),
                    target_node_id: "target".into(),
                    points: Vec::new(),
                    label_anchor: None,
                },
                DiagramEdge {
                    id: "edge-b".into(),
                    relationship_id: second.to_string(),
                    source_node_id: "source".into(),
                    target_node_id: "target".into(),
                    points: Vec::new(),
                    label_anchor: None,
                },
            ],
        };

        let routed = routed_bdd_edges(&diagram, None).unwrap();
        assert_ne!(routed[0].points, routed[1].points);
        assert!(routed.iter().all(|edge| edge.label_anchor.is_some()));
        let obstacle_rect = diagram_node_route_rect(&diagram, &diagram.nodes[2]);
        assert_eq!(obstacle_rect.y, diagram.nodes[2].y - 20.0);
        assert!(routed
            .iter()
            .all(|edge| routing::route_is_clear(&edge.points, &[obstacle_rect])));
    }

    #[test]
    fn package_validation_rejects_mismatched_relationship_presentations() {
        let mut project = Project::new("Vehicle");
        let source = project
            .create_element(ElementKind::Package, "Vehicle", project.root_id)
            .unwrap();
        let target = project
            .create_element(ElementKind::Package, "Powertrain", project.root_id)
            .unwrap();
        let wrong_target = project
            .create_element(ElementKind::Package, "Safety", project.root_id)
            .unwrap();
        let relationship = project
            .create_package_merge(source, target)
            .unwrap();
        let diagram = BddDiagram {
            id: DiagramId::new().to_string(),
            name: "Packages".into(),
            owner_id: project.root_id.to_string(),
            family: "package".into(),
            semantic_context_id: None,
            subject_boundary: None,
            nodes: vec![
                node("source", source, 80.0, 180.0),
                node("wrong-target", wrong_target, 520.0, 180.0),
            ],
            edges: vec![DiagramEdge {
                id: "merge".into(),
                relationship_id: relationship.to_string(),
                source_node_id: "source".into(),
                target_node_id: "wrong-target".into(),
                points: vec![
                    DiagramPoint { x: 300.0, y: 240.0 },
                    DiagramPoint { x: 520.0, y: 240.0 },
                ],
                label_anchor: Some(DiagramPoint { x: 410.0, y: 220.0 }),
            }],
        };
        let error = validate_package_diagram(&project, &diagram).unwrap_err();
        assert!(error.contains("PackageMerge"));
        assert!(error.contains("endpoint"));
    }

    #[test]
    fn reconnect_delete_and_history_keep_semantics_and_presentations_together() {
        let mut project = Project::new("Vehicle");
        let source = project
            .create_element(ElementKind::Package, "Vehicle", project.root_id)
            .unwrap();
        let original_target = project
            .create_element(ElementKind::Package, "Powertrain", project.root_id)
            .unwrap();
        let replacement_target = project
            .create_element(ElementKind::Package, "Electrical", project.root_id)
            .unwrap();
        let relationship = project
            .create_package_import(source, original_target, VisibilityKind::Public)
            .unwrap();
        let mut diagram = BddDiagram {
            id: DiagramId::new().to_string(),
            name: "Packages".into(),
            owner_id: project.root_id.to_string(),
            family: "package".into(),
            semantic_context_id: None,
            subject_boundary: None,
            nodes: vec![
                node("source", source, 80.0, 180.0),
                node("original", original_target, 520.0, 100.0),
                node("replacement", replacement_target, 520.0, 300.0),
            ],
            edges: vec![DiagramEdge {
                id: "import".into(),
                relationship_id: relationship.to_string(),
                source_node_id: "source".into(),
                target_node_id: "original".into(),
                points: Vec::new(),
                label_anchor: None,
            }],
        };
        reroute(&mut diagram).unwrap();

        let workspace = WorkspaceState::default();
        *workspace.project.lock().unwrap() = Some(project);
        *workspace.diagrams.lock().unwrap() = vec![diagram];
        let activity = activity_workspace::ActivityWorkspaceState::default();
        let history = history::HistoryState::default();

        let (mut candidate_project, mut candidate_diagrams) =
            candidate_states(&workspace).unwrap();
        reconnect_candidate(
            &mut candidate_project,
            &mut candidate_diagrams[0],
            relationship,
            source,
            replacement_target,
        )
        .unwrap();
        commit_states(
            candidate_project,
            candidate_diagrams,
            &workspace,
            &activity,
            &history,
        )
        .unwrap();
        assert_eq!(
            workspace
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationship(relationship)
                .unwrap()
                .target_id,
            replacement_target
        );
        assert_eq!(
            workspace.diagrams.lock().unwrap()[0].edges[0].target_node_id,
            "replacement"
        );

        assert!(history::undo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(
            workspace
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .relationship(relationship)
                .unwrap()
                .target_id,
            original_target
        );
        assert!(history::redo_states(&workspace, &activity, &history).unwrap());

        let (mut candidate_project, mut candidate_diagrams) =
            candidate_states(&workspace).unwrap();
        delete_relationship_candidate(
            &mut candidate_project,
            &mut candidate_diagrams,
            relationship,
        )
        .unwrap();
        commit_states(
            candidate_project,
            candidate_diagrams,
            &workspace,
            &activity,
            &history,
        )
        .unwrap();
        assert!(workspace
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationship(relationship)
            .is_err());
        assert!(workspace.diagrams.lock().unwrap()[0].edges.is_empty());
        assert!(history::undo_states(&workspace, &activity, &history).unwrap());
        assert!(workspace
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .relationship(relationship)
            .is_ok());
        assert_eq!(workspace.diagrams.lock().unwrap()[0].edges.len(), 1);
    }
}
