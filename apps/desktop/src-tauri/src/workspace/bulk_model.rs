use super::*;
use std::collections::{HashMap, HashSet};
use systems_modeler_core::{DiagramFamilyId, supported_diagram_families};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildReference<T> {
    External(String),
    Existing(T),
}

pub type ElementReference = BuildReference<ElementId>;
pub type RelationshipReference = BuildReference<RelationshipId>;
pub type DiagramReference = BuildReference<DiagramId>;

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
    CreateRelationship {
        external_id: String,
        kind: RelationshipKind,
        source: ElementReference,
        target: ElementReference,
        owner: Option<ElementReference>,
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

fn external_key(namespace: &str, external_id: &str) -> String {
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
        ModelBuildOperation::CreateRelationship { external_id, .. } => {
            format!("CREATE relationship {external_id}")
        }
        ModelBuildOperation::CreateDiagram {
            external_id, name, ..
        } => format!("CREATE diagram {external_id} ({name})"),
        ModelBuildOperation::PresentElement { .. } => "PRESENT element".into(),
        ModelBuildOperation::PresentRelationship { .. } => "PRESENT relationship".into(),
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
            | ModelBuildOperation::CreateDiagram { external_id, .. } => Some(external_id),
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
                        project.create_typed_feature(
                            kind.clone(),
                            name,
                            owner_id,
                            type_id,
                            Multiplicity::ONE,
                        )
                    } else {
                        project.create_element(kind.clone(), name, owner_id)
                    }
                    .map_err(|cause| {
                        error("SEMANTIC_VALIDATION", Some(index), cause.to_string())
                    })?;
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
