use super::{parse_diagram_id, parse_element_id, parse_relationship_id, relationship_display_kind, route_relationship, WorkspaceState};
use systems_modeler_core::{AggregationKind, ElementId, ElementKind, Multiplicity, RelationshipKind};

fn parse_multiplicity(value: &str) -> Result<Multiplicity, String> {
    let trimmed = value.trim();
    if trimmed == "*" {
        return Multiplicity::new(0, None).map_err(|error| error.to_string());
    }
    if let Some((lower, upper)) = trimmed.split_once("..") {
        let lower = lower.trim().parse::<u32>().map_err(|_| "invalid multiplicity lower bound")?;
        let upper = match upper.trim() {
            "*" => None,
            value => Some(value.parse::<u32>().map_err(|_| "invalid multiplicity upper bound")?),
        };
        return Multiplicity::new(lower, upper).map_err(|error| error.to_string());
    }
    let exact = trimmed.parse::<u32>().map_err(|_| "multiplicity must be N, N..M, N..*, or *")?;
    Multiplicity::new(exact, Some(exact)).map_err(|error| error.to_string())
}

fn parse_aggregation(value: &str) -> Result<AggregationKind, String> {
    match value {
        "none" => Ok(AggregationKind::None),
        "shared" => Ok(AggregationKind::Shared),
        "composite" => Ok(AggregationKind::Composite),
        _ => Err("aggregation must be none, shared, or composite".into()),
    }
}

fn generalization_cycle(project: &systems_modeler_core::Project, relationship_id: systems_modeler_core::RelationshipId, source_id: ElementId, target_id: ElementId) -> bool {
    let mut current = target_id;
    let mut visited = std::collections::HashSet::new();
    while visited.insert(current) {
        if current == source_id {
            return true;
        }
        let Some(next) = project.relationships.values().find(|relationship| {
            relationship.id != relationship_id
                && relationship.kind == RelationshipKind::Generalization
                && relationship.source_id == current
        }) else {
            return false;
        };
        current = next.target_id;
    }
    false
}

fn duplicate_after_reconnect(
    project: &systems_modeler_core::Project,
    relationship_id: systems_modeler_core::RelationshipId,
    kind: &str,
    source_id: ElementId,
    target_id: ElementId,
) -> bool {
    project.relationships.values().any(|relationship| {
        relationship.id != relationship_id
            && relationship.source_id == source_id
            && relationship.target_id == target_id
            && relationship_display_kind(relationship) == kind
    })
}

#[tauri::command]
pub fn update_association_end(
    relationship_id: String,
    end_id: String,
    role_name: String,
    multiplicity: String,
    navigable: bool,
    aggregation: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let relationship_id = parse_relationship_id(&relationship_id)?;
    let multiplicity = parse_multiplicity(&multiplicity)?;
    let aggregation = parse_aggregation(&aggregation)?;

    let mut project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_mut().ok_or("no project open")?;
    let relationship = project.relationships.get_mut(&relationship_id).ok_or("relationship not found")?;
    if relationship.kind != RelationshipKind::Association || relationship.association_ends.len() != 2 {
        return Err("association-end editing requires a binary Association-family relationship".into());
    }
    let end_index = relationship.association_ends.iter().position(|end| end.id.to_string() == end_id).ok_or("association end not found")?;

    let original = relationship.association_ends[end_index].clone();
    relationship.association_ends[end_index].role_name = role_name.trim().to_string();
    relationship.association_ends[end_index].multiplicity = multiplicity;
    relationship.association_ends[end_index].navigable = navigable;
    relationship.association_ends[end_index].aggregation = aggregation;

    let decorated = relationship.association_ends.iter().filter(|end| end.aggregation != AggregationKind::None).count();
    if decorated > 1 {
        relationship.association_ends[end_index] = original;
        return Err("a binary association can have aggregation/composition on only one end".into());
    }
    project.validate().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn reconnect_bdd_relationship(
    diagram_id: String,
    relationship_id: String,
    side: String,
    element_id: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let diagram_id = parse_diagram_id(&diagram_id)?;
    let relationship_id = parse_relationship_id(&relationship_id)?;
    let element_id = parse_element_id(&element_id)?;
    if side != "source" && side != "target" {
        return Err("relationship side must be source or target".into());
    }

    let mut project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_mut().ok_or("no project open")?;
    let replacement = project.element(element_id).map_err(|error| error.to_string())?;
    if replacement.kind != ElementKind::Block {
        return Err("BDD relationship endpoints must be Blocks".into());
    }

    let current = project.relationship(relationship_id).map_err(|error| error.to_string())?.clone();
    let mut new_source = current.source_id;
    let mut new_target = current.target_id;
    if side == "source" { new_source = element_id; } else { new_target = element_id; }
    if new_source == new_target {
        return Err("a BDD relationship cannot connect a Block to itself".into());
    }
    let display_kind = relationship_display_kind(&current);
    if duplicate_after_reconnect(project, relationship_id, display_kind, new_source, new_target) {
        return Err(format!("an equivalent {display_kind} already exists"));
    }
    if current.kind == RelationshipKind::Generalization {
        if generalization_cycle(project, relationship_id, new_source, new_target) {
            return Err("generalization would create an inheritance cycle".into());
        }
    }

    {
        let relationship = project.relationships.get_mut(&relationship_id).ok_or("relationship not found")?;
        relationship.source_id = new_source;
        relationship.target_id = new_target;
        if relationship.kind == RelationshipKind::Association && relationship.association_ends.len() == 2 {
            relationship.association_ends[0].classifier_id = new_source;
            relationship.association_ends[1].classifier_id = new_target;
        }
    }
    project.validate().map_err(|error| error.to_string())?;

    let mut diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let diagram = diagrams.iter_mut().find(|diagram| diagram.id == diagram_id.to_string()).ok_or("diagram not found")?;
    let source_node = diagram.nodes.iter().find(|node| node.element_id == new_source.to_string()).cloned().ok_or("new source Block must be presented on the BDD")?;
    let target_node = diagram.nodes.iter().find(|node| node.element_id == new_target.to_string()).cloned().ok_or("new target Block must be presented on the BDD")?;
    let edge = diagram.edges.iter_mut().find(|edge| edge.relationship_id == relationship_id.to_string()).ok_or("diagram edge not found")?;
    edge.source_node_id = source_node.id.clone();
    edge.target_node_id = target_node.id.clone();
    edge.points = route_relationship(&source_node, &target_node, &diagram.nodes);
    Ok(())
}

#[tauri::command]
pub fn delete_bdd_relationship(
    diagram_id: String,
    relationship_id: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let diagram_id = parse_diagram_id(&diagram_id)?;
    let relationship_id = parse_relationship_id(&relationship_id)?;
    let mut project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_mut().ok_or("no project open")?;
    if project.relationships.remove(&relationship_id).is_none() {
        return Err("relationship not found".into());
    }
    let mut diagrams = state.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    for diagram in diagrams.iter_mut() {
        diagram.edges.retain(|edge| edge.relationship_id != relationship_id.to_string());
    }
    if !diagrams.iter().any(|diagram| diagram.id == diagram_id.to_string()) {
        return Err("diagram not found".into());
    }
    Ok(())
}
