//! Rust-authoritative Parametric diagram commands.
//!
//! The diagram reuses BDD presentation storage and all shared workspace
//! services. This module contributes only Parametric endpoint geometry,
//! semantic eligibility, BindingConnector mutation, and explicit evaluation.

use super::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use systems_modeler_core::{
    BindingEndpoint, ElementKind, Multiplicity, ParametricEvaluationScope, RelationshipKind,
    evaluate_parametrics,
};

const PARAMETER_SIZE: f64 = 14.0;
const CONSTRAINT_WIDTH: f64 = 260.0;
const CONSTRAINT_HEIGHT: f64 = 170.0;
const VALUE_WIDTH: f64 = 220.0;
const VALUE_HEIGHT: f64 = 72.0;

#[derive(Debug, Clone, Serialize)]
pub struct ParametricEvaluationSnapshot {
    pub evaluated_constraints: usize,
    pub changed_values: usize,
    pub updates: Vec<ParametricValueSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ParametricValueSnapshot {
    pub element_id: String,
    pub previous_value: Option<String>,
    pub value: String,
}

fn checkpoint(
    workspace: &WorkspaceState,
    activity: &activity_workspace::ActivityWorkspaceState,
    history: &history::HistoryState,
) -> Result<(), String> {
    history::checkpoint_states(workspace, activity, history)
}

pub(super) fn parse_multiplicity(value: &str) -> Result<Multiplicity, String> {
    let value = value.trim();
    if value == "*" {
        return Multiplicity::new(0, None).map_err(|error| error.to_string());
    }
    if let Some((lower, upper)) = value.split_once("..") {
        let lower = lower
            .trim()
            .parse::<u32>()
            .map_err(|_| "invalid multiplicity lower bound")?;
        let upper = match upper.trim() {
            "*" => None,
            value => Some(
                value
                    .parse::<u32>()
                    .map_err(|_| "invalid multiplicity upper bound")?,
            ),
        };
        return Multiplicity::new(lower, upper).map_err(|error| error.to_string());
    }
    let exact = value
        .parse::<u32>()
        .map_err(|_| "multiplicity must be N, N..M, N..*, or *")?;
    Multiplicity::new(exact, Some(exact)).map_err(|error| error.to_string())
}

pub(super) fn diagram_context(diagram: &BddDiagram) -> Result<ElementId, String> {
    if diagram.family != "parametric" {
        return Err("diagram is not a Parametric Diagram".into());
    }
    parse_element_id(
        diagram
            .semantic_context_id
            .as_deref()
            .ok_or("Parametric Diagram has no semantic context")?,
    )
}

pub(super) fn evaluation_scope(
    diagram: &BddDiagram,
    project: &Project,
) -> Result<ParametricEvaluationScope, String> {
    Ok(ParametricEvaluationScope {
        context_id: diagram_context(diagram)?,
        constraint_property_ids: diagram
            .nodes
            .iter()
            .filter_map(|node| {
                let id = parse_element_id(&node.element_id).ok()?;
                (project.element(id).ok()?.kind == ElementKind::ConstraintProperty).then_some(id)
            })
            .collect(),
        value_property_ids: diagram
            .nodes
            .iter()
            .filter_map(|node| {
                let id = parse_element_id(&node.element_id).ok()?;
                (project.element(id).ok()?.kind == ElementKind::ValueProperty).then_some(id)
            })
            .collect(),
        binding_relationship_ids: diagram
            .edges
            .iter()
            .map(|edge| parse_relationship_id(&edge.relationship_id))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn parameter_layout(
    parameters: &[&systems_modeler_core::Element],
    width: f64,
    height: f64,
) -> Vec<ConstraintParameterPresentation> {
    let rows = parameters.len().div_ceil(2).max(1);
    parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            let right = index % 2 == 1;
            let row = index / 2;
            let center_y = (row + 1) as f64 * height / (rows + 1) as f64;
            ConstraintParameterPresentation {
                id: uuid::Uuid::new_v4().to_string(),
                parameter_id: parameter.id.to_string(),
                offset_x: if right { width - PARAMETER_SIZE } else { 0.0 },
                offset_y: (center_y - PARAMETER_SIZE / 2.0).clamp(0.0, height - PARAMETER_SIZE),
                size: PARAMETER_SIZE,
            }
        })
        .collect()
}

pub(super) fn sync_parameter_presentations(
    node: &mut DiagramNode,
    project: &Project,
) -> Result<(), String> {
    let property = project
        .element(parse_element_id(&node.element_id)?)
        .map_err(|error| error.to_string())?;
    if property.kind != ElementKind::ConstraintProperty {
        node.parameter_presentations.clear();
        return Ok(());
    }
    let block_id = property
        .type_id
        .ok_or("ConstraintProperty requires a ConstraintBlock type")?;
    let mut parameters: Vec<_> = project
        .children(block_id)
        .filter(|parameter| parameter.kind == ElementKind::ConstraintParameter)
        .collect();
    parameters.sort_by(|left, right| left.name.cmp(&right.name));
    node.height = node
        .height
        .max((parameters.len().div_ceil(2) as f64 * 34.0 + 92.0).max(CONSTRAINT_HEIGHT));
    let previous: HashMap<_, _> = node
        .parameter_presentations
        .drain(..)
        .map(|presentation| (presentation.parameter_id.clone(), presentation))
        .collect();
    let defaults = parameter_layout(&parameters, node.width, node.height);
    node.parameter_presentations = defaults
        .into_iter()
        .map(|mut default| {
            if let Some(existing) = previous.get(&default.parameter_id) {
                default.id = existing.id.clone();
                default.size = existing.size.max(10.0);
                default.offset_x = existing
                    .offset_x
                    .clamp(0.0, (node.width - default.size).max(0.0));
                default.offset_y = existing
                    .offset_y
                    .clamp(0.0, (node.height - default.size).max(0.0));
            }
            default
        })
        .collect();
    Ok(())
}

fn endpoint_from_presentation(
    diagram: &BddDiagram,
    project: &Project,
    presentation_id: &str,
) -> Result<BindingEndpoint, String> {
    if let Some(node) = diagram.nodes.iter().find(|node| node.id == presentation_id) {
        let element_id = parse_element_id(&node.element_id)?;
        let element = project
            .element(element_id)
            .map_err(|error| error.to_string())?;
        if element.kind != ElementKind::ValueProperty {
            return Err(
                "BindingConnector must attach to a ValueProperty or constraint parameter".into(),
            );
        }
        return Ok(BindingEndpoint {
            role_id: element_id,
            parameter_id: None,
        });
    }
    for node in &diagram.nodes {
        if let Some(parameter) = node
            .parameter_presentations
            .iter()
            .find(|parameter| parameter.id == presentation_id)
        {
            return Ok(BindingEndpoint {
                role_id: parse_element_id(&node.element_id)?,
                parameter_id: Some(parse_element_id(&parameter.parameter_id)?),
            });
        }
    }
    Err("Parametric endpoint presentation not found".into())
}

fn route_rect(node: &DiagramNode) -> routing::RouteRect {
    routing::RouteRect {
        x: node.x,
        y: node.y,
        width: node.width,
        height: node.height,
    }
}

fn endpoint_geometry(
    diagram: &BddDiagram,
    presentation_id: &str,
) -> Result<(routing::RouteRect, String), String> {
    if let Some(node) = diagram.nodes.iter().find(|node| node.id == presentation_id) {
        return Ok((route_rect(node), node.id.clone()));
    }
    for node in &diagram.nodes {
        if let Some(parameter) = node
            .parameter_presentations
            .iter()
            .find(|parameter| parameter.id == presentation_id)
        {
            return Ok((
                routing::RouteRect {
                    x: node.x + parameter.offset_x,
                    y: node.y + parameter.offset_y,
                    width: parameter.size,
                    height: parameter.size,
                },
                node.id.clone(),
            ));
        }
    }
    Err(format!("Parametric endpoint not found: {presentation_id}"))
}

fn parameter_rectangles(node: &DiagramNode) -> impl Iterator<Item = routing::RouteRect> + '_ {
    node.parameter_presentations
        .iter()
        .map(|parameter| routing::RouteRect {
            x: node.x + parameter.offset_x,
            y: node.y + parameter.offset_y,
            width: parameter.size,
            height: parameter.size,
        })
}

pub(super) fn routed_edges(
    diagram: &BddDiagram,
    bounds: Option<routing::RouteRect>,
) -> Result<Vec<DiagramEdge>, String> {
    let mut result = diagram.edges.clone();
    let mut reserved = Vec::new();
    for (index, edge) in diagram.edges.iter().enumerate() {
        let (source, source_role) = endpoint_geometry(diagram, &edge.source_node_id)?;
        let (target, target_role) = endpoint_geometry(diagram, &edge.target_node_id)?;
        let mut obstacles: Vec<_> = diagram
            .nodes
            .iter()
            .filter(|node| node.id != source_role && node.id != target_role)
            .map(route_rect)
            .collect();
        obstacles.extend(
            diagram
                .nodes
                .iter()
                .flat_map(parameter_rectangles)
                .filter(|rect| *rect != source && *rect != target),
        );
        let same_source = diagram.edges[..index]
            .iter()
            .filter(|candidate| candidate.source_node_id == edge.source_node_id)
            .count();
        let points = routing::orthogonal_route(routing::RouteRequest {
            source,
            target,
            obstacles: &obstacles,
            lane_index: same_source,
            reserved_routes: &reserved,
            allow_shared_departure: same_source > 0,
            bounds,
        })?;
        let candidate = result
            .iter_mut()
            .find(|candidate| candidate.id == edge.id)
            .ok_or("BindingConnector presentation not found")?;
        candidate.points = points.clone();
        candidate.label_anchor = None;
        reserved.push(points);
    }
    Ok(result)
}

fn reroute_incident_edges(
    diagram: &mut BddDiagram,
    affected_presentation_ids: &HashSet<String>,
) -> Result<(), String> {
    let mut routing_diagram = diagram.clone();
    routing_diagram.edges.retain(|edge| {
        affected_presentation_ids.contains(&edge.source_node_id)
            || affected_presentation_ids.contains(&edge.target_node_id)
    });
    if routing_diagram.edges.is_empty() {
        return Ok(());
    }
    let routed = routed_edges(&routing_diagram, None)?;
    for routed_edge in routed {
        if let Some(edge) = diagram
            .edges
            .iter_mut()
            .find(|edge| edge.id == routed_edge.id)
        {
            *edge = routed_edge;
        }
    }
    Ok(())
}

pub(super) fn route_parametric_with_bounds(
    diagram_id: &str,
    workspace: &WorkspaceState,
    bounds: Option<routing::RouteRect>,
) -> Result<bool, String> {
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id && diagram.family == "parametric")
        .ok_or("Parametric Diagram not found")?;
    let routed = routed_edges(diagram, bounds)?;
    let changed = diagram
        .edges
        .iter()
        .zip(&routed)
        .any(|(left, right)| left.points != right.points);
    if changed {
        diagram.edges = routed;
    }
    Ok(changed)
}

pub(super) fn layout_parametric_with_bounds(
    diagram_id: &str,
    workspace: &WorkspaceState,
    bounds: Option<routing::RouteRect>,
) -> Result<bool, String> {
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?;
    let index = diagrams
        .iter()
        .position(|diagram| diagram.id == diagram_id && diagram.family == "parametric")
        .ok_or("Parametric Diagram not found")?;
    let original = diagrams[index].clone();
    let mut candidate = original.clone();
    let edges = candidate
        .edges
        .iter()
        .map(|edge| {
            Ok((
                endpoint_geometry(&candidate, &edge.source_node_id)?.1,
                endpoint_geometry(&candidate, &edge.target_node_id)?.1,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let positions = layout::hierarchical_positions_sized(
        candidate.nodes.iter().map(|node| layout::LayoutNode {
            id: node.id.clone(),
            width: node.width,
            height: node.height,
        }),
        &edges,
        systems_modeler_core::PreferredFlowDirection::LeftToRight,
    );
    for node in &mut candidate.nodes {
        if let Some((x, y)) = positions.get(&node.id) {
            node.x = *x;
            node.y = *y;
        }
    }
    candidate.edges = routed_edges(&candidate, bounds)?;
    let changed = original
        .nodes
        .iter()
        .zip(&candidate.nodes)
        .any(|(left, right)| {
            left.x != right.x
                || left.y != right.y
                || left.width != right.width
                || left.height != right.height
        })
        || original
            .edges
            .iter()
            .zip(&candidate.edges)
            .any(|(left, right)| left.points != right.points);
    if changed {
        diagrams[index] = candidate;
    }
    Ok(changed)
}

#[tauri::command]
pub fn create_parametric_diagram(
    owner_id: String,
    name: String,
    semantic_context_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let owner_id = parse_element_id(&owner_id)?;
    let context_id = parse_element_id(&semantic_context_id)?;
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let owner = project
        .element(owner_id)
        .map_err(|error| error.to_string())?;
    if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
        return Err("Parametric Diagram owner must be a Model or Package".into());
    }
    let context = project
        .element(context_id)
        .map_err(|error| error.to_string())?;
    if !matches!(
        context.kind,
        ElementKind::Block | ElementKind::AssociationBlock | ElementKind::ConstraintBlock
    ) {
        return Err("Parametric Diagram context must be a Block or ConstraintBlock".into());
    }
    let id = DiagramId::new().to_string();
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    diagrams.push(BddDiagram {
        id: id.clone(),
        name,
        owner_id: owner_id.to_string(),
        family: "parametric".into(),
        semantic_context_id: Some(context_id.to_string()),
        subject_boundary: None,
        nodes: Vec::new(),
        edges: Vec::new(),
    });
    validate_loaded_diagrams(&project, &diagrams)?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(id)
}

#[tauri::command]
pub fn place_on_parametric_diagram(
    diagram_id: String,
    element_id: String,
    x: f64,
    y: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    if !x.is_finite() || !y.is_finite() {
        return Err("Parametric presentation coordinates must be finite".into());
    }
    parse_diagram_id(&diagram_id)?;
    let element_id = parse_element_id(&element_id)?;
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let element = project
        .element(element_id)
        .map_err(|error| error.to_string())?;
    if !matches!(
        element.kind,
        ElementKind::ConstraintProperty | ElementKind::ValueProperty
    ) {
        return Err(
            "only ConstraintProperties and ValueProperties can be placed on a Parametric Diagram"
                .into(),
        );
    }
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id && diagram.family == "parametric")
        .ok_or("Parametric Diagram not found")?;
    if element.owner_id != Some(diagram_context(diagram)?) {
        return Err("Parametric element must be owned by the diagram context".into());
    }
    if diagram
        .nodes
        .iter()
        .any(|node| node.element_id == element_id.to_string())
    {
        return Err("this semantic property is already presented on the Parametric Diagram".into());
    }
    let constraint = element.kind == ElementKind::ConstraintProperty;
    let mut node = DiagramNode {
        id: uuid::Uuid::new_v4().to_string(),
        element_id: element_id.to_string(),
        x: x.max(0.0),
        y: y.max(42.0),
        width: if constraint {
            CONSTRAINT_WIDTH
        } else {
            VALUE_WIDTH
        },
        height: if constraint {
            CONSTRAINT_HEIGHT
        } else {
            VALUE_HEIGHT
        },
        actor_notation: None,
        parameter_presentations: Vec::new(),
    };
    sync_parameter_presentations(&mut node, &project)?;
    let presentation_id = node.id.clone();
    diagram.nodes.push(node);
    validate_loaded_diagrams(&project, &diagrams)?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(presentation_id)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_parametric_constraint_property(
    diagram_id: String,
    name: String,
    constraint_block_id: String,
    x: f64,
    y: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let constraint_block_id = parse_element_id(&constraint_block_id)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id && diagram.family == "parametric")
        .ok_or("Parametric Diagram not found")?;
    let context_id = diagram_context(diagram)?;
    let element_id = project
        .create_typed_feature(
            ElementKind::ConstraintProperty,
            name,
            context_id,
            constraint_block_id,
            Multiplicity::ONE,
        )
        .map_err(|error| error.to_string())?;
    let mut node = DiagramNode {
        id: uuid::Uuid::new_v4().to_string(),
        element_id: element_id.to_string(),
        x: x.max(0.0),
        y: y.max(42.0),
        width: CONSTRAINT_WIDTH,
        height: CONSTRAINT_HEIGHT,
        actor_notation: None,
        parameter_presentations: Vec::new(),
    };
    sync_parameter_presentations(&mut node, &project)?;
    diagram.nodes.push(node);
    project.validate().map_err(|error| error.to_string())?;
    validate_loaded_diagrams(&project, &diagrams)?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(element_id.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_parametric_constraint_property(
    element_id: String,
    name: String,
    constraint_block_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let element_id = parse_element_id(&element_id)?;
    let constraint_block_id = parse_element_id(&constraint_block_id)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    if project
        .element(element_id)
        .map_err(|error| error.to_string())?
        .kind
        != ElementKind::ConstraintProperty
    {
        return Err("constraint property editing requires a ConstraintProperty".into());
    }
    project
        .rename_element(element_id, name)
        .map_err(|error| error.to_string())?;
    project
        .set_element_type(element_id, constraint_block_id)
        .map_err(|error| error.to_string())?;

    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    for diagram in diagrams
        .iter_mut()
        .filter(|diagram| diagram.family == "parametric")
    {
        for node in diagram
            .nodes
            .iter_mut()
            .filter(|node| node.element_id == element_id.to_string())
        {
            sync_parameter_presentations(node, &project)?;
        }
        diagram.edges = routed_edges(diagram, None)?;
    }
    project.validate().map_err(|error| error.to_string())?;
    validate_loaded_diagrams(&project, &diagrams)?;
    checkpoint(&workspace, &activity, &history)?;
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

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_parametric_value_property(
    diagram_id: String,
    name: String,
    value_type_id: Option<String>,
    value: Option<String>,
    multiplicity: String,
    is_derived: bool,
    is_read_only: bool,
    x: f64,
    y: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id && diagram.family == "parametric")
        .ok_or("Parametric Diagram not found")?;
    let context_id = diagram_context(diagram)?;
    let value_type_id =
        resolve_parametric_value_type(&mut project, context_id, value_type_id.as_deref())?;
    let element_id = project
        .create_typed_feature(
            ElementKind::ValueProperty,
            name,
            context_id,
            value_type_id,
            parse_multiplicity(&multiplicity)?,
        )
        .map_err(|error| error.to_string())?;
    {
        let property = project
            .element_mut(element_id)
            .map_err(|error| error.to_string())?;
        property.default_value = value.filter(|value| !value.trim().is_empty());
        property.is_derived = is_derived;
        property.is_read_only = is_read_only;
    }
    diagram.nodes.push(DiagramNode {
        id: uuid::Uuid::new_v4().to_string(),
        element_id: element_id.to_string(),
        x: x.max(0.0),
        y: y.max(42.0),
        width: VALUE_WIDTH,
        height: VALUE_HEIGHT,
        actor_notation: None,
        parameter_presentations: Vec::new(),
    });
    project.validate().map_err(|error| error.to_string())?;
    validate_loaded_diagrams(&project, &diagrams)?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(element_id.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_constraint_block_details(
    element_id: String,
    name: String,
    documentation: String,
    expression: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let element_id = parse_element_id(&element_id)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    if project
        .element(element_id)
        .map_err(|error| error.to_string())?
        .kind
        != ElementKind::ConstraintBlock
    {
        return Err("constraint expression can only update a ConstraintBlock".into());
    }
    project
        .rename_element(element_id, name)
        .map_err(|error| error.to_string())?;
    {
        let block = project
            .element_mut(element_id)
            .map_err(|error| error.to_string())?;
        block.documentation = documentation;
        block.constraint_expression = expression.trim().to_owned();
    }
    project.validate().map_err(|error| error.to_string())?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn create_constraint_parameter(
    constraint_block_id: String,
    name: String,
    type_id: Option<String>,
    multiplicity: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let constraint_block_id = parse_element_id(&constraint_block_id)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let type_id =
        resolve_constraint_parameter_type(&mut project, constraint_block_id, type_id.as_deref())?;
    let parameter_id = project
        .create_typed_feature(
            ElementKind::ConstraintParameter,
            name,
            constraint_block_id,
            type_id,
            parse_multiplicity(&multiplicity)?,
        )
        .map_err(|error| error.to_string())?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    for diagram in diagrams
        .iter_mut()
        .filter(|diagram| diagram.family == "parametric")
    {
        for node in &mut diagram.nodes {
            let property = project
                .element(parse_element_id(&node.element_id)?)
                .map_err(|error| error.to_string())?;
            if property.kind == ElementKind::ConstraintProperty
                && property.type_id == Some(constraint_block_id)
            {
                sync_parameter_presentations(node, &project)?;
            }
        }
        diagram.edges = routed_edges(diagram, None)?;
    }
    project.validate().map_err(|error| error.to_string())?;
    validate_loaded_diagrams(&project, &diagrams)?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(parameter_id.to_string())
}

fn resolve_constraint_parameter_type(
    project: &mut Project,
    constraint_block_id: ElementId,
    type_id: Option<&str>,
) -> Result<ElementId, String> {
    if let Some(type_id) = type_id {
        return parse_element_id(type_id);
    }

    let constraint_block = project
        .element(constraint_block_id)
        .map_err(|error| error.to_string())?;
    if constraint_block.kind != ElementKind::ConstraintBlock {
        return Err("constraint parameters require a ConstraintBlock owner".into());
    }
    resolve_parametric_value_type(project, constraint_block_id, None)
}

fn resolve_parametric_value_type(
    project: &mut Project,
    owner_id: ElementId,
    type_id: Option<&str>,
) -> Result<ElementId, String> {
    if let Some(type_id) = type_id {
        return parse_element_id(type_id);
    }
    let namespace_id = project
        .element(owner_id)
        .map_err(|error| error.to_string())?
        .owner_id
        .ok_or("parametric element has no owning Model or Package")?;
    if let Some(existing) = project
        .children(namespace_id)
        .find(|element| element.name == "Real" && element.kind == ElementKind::PrimitiveType)
    {
        return Ok(existing.id);
    }
    project
        .create_element(ElementKind::PrimitiveType, "Real", namespace_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_parametric_value_property(
    element_id: String,
    name: String,
    type_id: String,
    value: Option<String>,
    multiplicity: String,
    is_derived: bool,
    is_read_only: bool,
    quantity_kind_id: Option<String>,
    unit_id: Option<String>,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let element_id = parse_element_id(&element_id)?;
    let type_id = parse_element_id(&type_id)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    if project
        .element(element_id)
        .map_err(|error| error.to_string())?
        .kind
        != ElementKind::ValueProperty
    {
        return Err("value editing requires a ValueProperty".into());
    }
    let quantity_external = quantity_kind_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(parse_element_id)
        .transpose()?
        .map(|id| {
            project
                .element(id)
                .map(|element| element.external_id.clone())
                .map_err(|error| error.to_string())
        })
        .transpose()?;
    let unit_external = unit_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(parse_element_id)
        .transpose()?
        .map(|id| {
            project
                .element(id)
                .map(|element| element.external_id.clone())
                .map_err(|error| error.to_string())
        })
        .transpose()?;
    project
        .rename_element(element_id, name)
        .map_err(|error| error.to_string())?;
    project
        .set_multiplicity(element_id, parse_multiplicity(&multiplicity)?)
        .map_err(|error| error.to_string())?;
    {
        let property = project
            .element_mut(element_id)
            .map_err(|error| error.to_string())?;
        property.type_id = Some(type_id);
        property.default_value = value.filter(|value| !value.trim().is_empty());
        property.is_derived = is_derived;
        property.is_read_only = is_read_only;
        property.quantity_kind_external_id = quantity_external;
        property.unit_external_id = unit_external;
    }
    project.validate().map_err(|error| error.to_string())?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_constraint_parameter(
    element_id: String,
    name: String,
    type_id: String,
    multiplicity: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let element_id = parse_element_id(&element_id)?;
    let type_id = parse_element_id(&type_id)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    if project
        .element(element_id)
        .map_err(|error| error.to_string())?
        .kind
        != ElementKind::ConstraintParameter
    {
        return Err("parameter editing requires a ConstraintParameter".into());
    }
    project
        .rename_element(element_id, name)
        .map_err(|error| error.to_string())?;
    project
        .set_multiplicity(element_id, parse_multiplicity(&multiplicity)?)
        .map_err(|error| error.to_string())?;
    project
        .element_mut(element_id)
        .map_err(|error| error.to_string())?
        .type_id = Some(type_id);
    project.validate().map_err(|error| error.to_string())?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_value_type_details(
    element_id: String,
    name: String,
    documentation: String,
    quantity_kind_id: Option<String>,
    unit_id: Option<String>,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let element_id = parse_element_id(&element_id)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    if project
        .element(element_id)
        .map_err(|error| error.to_string())?
        .kind
        != ElementKind::ValueType
    {
        return Err("unit and QuantityKind editing requires a ValueType".into());
    }
    let resolve_external =
        |value: Option<String>, kind: ElementKind| -> Result<Option<String>, String> {
            value
                .as_deref()
                .filter(|value| !value.trim().is_empty())
                .map(parse_element_id)
                .transpose()?
                .map(|id| {
                    let element = project.element(id).map_err(|error| error.to_string())?;
                    if element.kind != kind {
                        return Err(format!("reference must target a {kind:?}"));
                    }
                    Ok(element.external_id.clone())
                })
                .transpose()
        };
    let quantity_external = resolve_external(quantity_kind_id, ElementKind::QuantityKind)?;
    let unit_external = resolve_external(unit_id, ElementKind::Unit)?;
    project
        .rename_element(element_id, name)
        .map_err(|error| error.to_string())?;
    {
        let value_type = project
            .element_mut(element_id)
            .map_err(|error| error.to_string())?;
        value_type.documentation = documentation;
        value_type.quantity_kind_external_id = quantity_external;
        value_type.unit_external_id = unit_external;
    }
    project.validate().map_err(|error| error.to_string())?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_quantity_kind_details(
    element_id: String,
    name: String,
    documentation: String,
    dimension: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let element_id = parse_element_id(&element_id)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    if project
        .element(element_id)
        .map_err(|error| error.to_string())?
        .kind
        != ElementKind::QuantityKind
    {
        return Err("dimension editing requires a QuantityKind".into());
    }
    project
        .rename_element(element_id, name)
        .map_err(|error| error.to_string())?;
    {
        let quantity = project
            .element_mut(element_id)
            .map_err(|error| error.to_string())?;
        quantity.documentation = documentation;
        quantity.quantity_dimension = Some(dimension.trim().to_owned());
    }
    project.validate().map_err(|error| error.to_string())?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_unit_details(
    element_id: String,
    name: String,
    documentation: String,
    symbol: String,
    scale_to_base: f64,
    quantity_kind_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let element_id = parse_element_id(&element_id)?;
    let quantity_kind_id = parse_element_id(&quantity_kind_id)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    if project
        .element(element_id)
        .map_err(|error| error.to_string())?
        .kind
        != ElementKind::Unit
    {
        return Err("unit editing requires a Unit".into());
    }
    let quantity = project
        .element(quantity_kind_id)
        .map_err(|error| error.to_string())?;
    if quantity.kind != ElementKind::QuantityKind {
        return Err("Unit quantity kind must reference a QuantityKind".into());
    }
    let quantity_external = quantity.external_id.clone();
    project
        .rename_element(element_id, name)
        .map_err(|error| error.to_string())?;
    {
        let unit = project
            .element_mut(element_id)
            .map_err(|error| error.to_string())?;
        unit.documentation = documentation;
        unit.unit_symbol = Some(symbol.trim().to_owned());
        unit.unit_scale_to_base = scale_to_base;
        unit.quantity_kind_external_id = Some(quantity_external);
    }
    project.validate().map_err(|error| error.to_string())?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    Ok(())
}

#[tauri::command]
pub fn create_binding_connector(
    diagram_id: String,
    source_presentation_id: String,
    target_presentation_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<String, String> {
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id && diagram.family == "parametric")
        .ok_or("Parametric Diagram not found")?;
    let source = endpoint_from_presentation(diagram, &project, &source_presentation_id)?;
    let target = endpoint_from_presentation(diagram, &project, &target_presentation_id)?;
    let relationship_id = project
        .create_binding_connector(diagram_context(diagram)?, source, target)
        .map_err(|error| error.to_string())?;
    diagram.edges.push(DiagramEdge {
        id: uuid::Uuid::new_v4().to_string(),
        relationship_id: relationship_id.to_string(),
        source_node_id: source_presentation_id,
        target_node_id: target_presentation_id,
        points: Vec::new(),
        label_anchor: None,
    });
    diagram.edges = routed_edges(diagram, None)?;
    project.validate().map_err(|error| error.to_string())?;
    validate_loaded_diagrams(&project, &diagrams)?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")? = Some(project);
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(relationship_id.to_string())
}

#[tauri::command]
pub fn reconnect_binding_connector(
    diagram_id: String,
    relationship_id: String,
    side: String,
    presentation_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    if !matches!(side.as_str(), "source" | "target") {
        return Err("BindingConnector side must be source or target".into());
    }
    let relationship_id = parse_relationship_id(&relationship_id)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id && diagram.family == "parametric")
        .ok_or("Parametric Diagram not found")?;
    let endpoint = endpoint_from_presentation(diagram, &project, &presentation_id)?;
    let relationship = project
        .relationships
        .get_mut(&relationship_id)
        .ok_or("BindingConnector not found")?;
    if relationship.kind != RelationshipKind::BindingConnector {
        return Err("relationship is not a BindingConnector".into());
    }
    let binding = relationship
        .binding
        .as_mut()
        .ok_or("BindingConnector endpoint details are missing")?;
    if side == "source" {
        binding.source = endpoint.clone();
        relationship.source_id = endpoint.role_id;
    } else {
        binding.target = endpoint.clone();
        relationship.target_id = endpoint.role_id;
    }
    project.validate().map_err(|error| error.to_string())?;
    let edge = diagram
        .edges
        .iter_mut()
        .find(|edge| edge.relationship_id == relationship_id.to_string())
        .ok_or("BindingConnector presentation not found")?;
    if side == "source" {
        edge.source_node_id = presentation_id;
    } else {
        edge.target_node_id = presentation_id;
    }
    diagram.edges = routed_edges(diagram, None)?;
    validate_loaded_diagrams(&project, &diagrams)?;
    checkpoint(&workspace, &activity, &history)?;
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

#[tauri::command]
pub fn delete_binding_connector(
    relationship_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    let relationship_id = parse_relationship_id(&relationship_id)?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    if project
        .relationship(relationship_id)
        .map_err(|error| error.to_string())?
        .kind
        != RelationshipKind::BindingConnector
    {
        return Err("relationship is not a BindingConnector".into());
    }
    project.relationships.remove(&relationship_id);
    let id = relationship_id.to_string();
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    for diagram in &mut diagrams {
        diagram.edges.retain(|edge| edge.relationship_id != id);
    }
    validate_loaded_diagrams(&project, &diagrams)?;
    checkpoint(&workspace, &activity, &history)?;
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

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_parametric_presentation_geometry(
    diagram_id: String,
    presentation_id: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    if ![x, y, width, height].iter().all(|value| value.is_finite()) || width < 80.0 || height < 50.0
    {
        return Err("Parametric presentation geometry is invalid".into());
    }
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id && diagram.family == "parametric")
        .ok_or("Parametric Diagram not found")?;
    let affected = {
        let node = diagram
            .nodes
            .iter_mut()
            .find(|node| node.id == presentation_id)
            .ok_or("Parametric presentation not found")?;
        node.x = x.max(0.0);
        node.y = y.max(42.0);
        node.width = width;
        node.height = height;
        sync_parameter_presentations(node, &project)?;
        let mut affected = HashSet::from([node.id.clone()]);
        affected.extend(
            node.parameter_presentations
                .iter()
                .map(|parameter| parameter.id.clone()),
        );
        affected
    };
    reroute_incident_edges(diagram, &affected)?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn update_constraint_parameter_presentation(
    diagram_id: String,
    presentation_id: String,
    offset_x: f64,
    offset_y: f64,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<(), String> {
    if !offset_x.is_finite() || !offset_y.is_finite() {
        return Err("parameter position must be finite".into());
    }
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id && diagram.family == "parametric")
        .ok_or("Parametric Diagram not found")?;
    {
        let node = diagram
            .nodes
            .iter_mut()
            .find(|node| {
                node.parameter_presentations
                    .iter()
                    .any(|parameter| parameter.id == presentation_id)
            })
            .ok_or("ConstraintParameter presentation not found")?;
        let parameter = node
            .parameter_presentations
            .iter_mut()
            .find(|parameter| parameter.id == presentation_id)
            .ok_or("ConstraintParameter presentation not found")?;
        let max_x = (node.width - parameter.size).max(0.0);
        let max_y = (node.height - parameter.size).max(0.0);
        let x = offset_x.clamp(0.0, max_x);
        let y = offset_y.clamp(0.0, max_y);
        let distances = [x, max_x - x, y, max_y - y];
        match distances
            .iter()
            .enumerate()
            .min_by(|left, right| left.1.total_cmp(right.1))
            .map(|(index, _)| index)
            .unwrap_or(0)
        {
            0 => {
                parameter.offset_x = 0.0;
                parameter.offset_y = y;
            }
            1 => {
                parameter.offset_x = max_x;
                parameter.offset_y = y;
            }
            2 => {
                parameter.offset_x = x;
                parameter.offset_y = 0.0;
            }
            _ => {
                parameter.offset_x = x;
                parameter.offset_y = max_y;
            }
        }
    }
    reroute_incident_edges(diagram, &HashSet::from([presentation_id]))?;
    checkpoint(&workspace, &activity, &history)?;
    *workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")? = diagrams;
    Ok(())
}

#[tauri::command]
pub fn evaluate_parametric_diagram(
    diagram_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, history::HistoryState>,
) -> Result<ParametricEvaluationSnapshot, String> {
    let diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id && diagram.family == "parametric")
        .ok_or("Parametric Diagram not found")?;
    let mut project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let scope = ParametricEvaluationScope {
        context_id: diagram_context(diagram)?,
        constraint_property_ids: diagram
            .nodes
            .iter()
            .filter_map(|node| {
                let id = parse_element_id(&node.element_id).ok()?;
                (project.element(id).ok()?.kind == ElementKind::ConstraintProperty).then_some(id)
            })
            .collect(),
        value_property_ids: diagram
            .nodes
            .iter()
            .filter_map(|node| {
                let id = parse_element_id(&node.element_id).ok()?;
                (project.element(id).ok()?.kind == ElementKind::ValueProperty).then_some(id)
            })
            .collect(),
        binding_relationship_ids: diagram
            .edges
            .iter()
            .map(|edge| parse_relationship_id(&edge.relationship_id))
            .collect::<Result<Vec<_>, _>>()?,
    };
    let report = evaluate_parametrics(&mut project, &scope).map_err(|error| error.to_string())?;
    // PR35 keeps this legacy command preview-only. Runtime values are owned by
    // the shared ExecutionSession path and never overwrite authored defaults.
    let _ = (&workspace, &activity, &history);
    Ok(ParametricEvaluationSnapshot {
        evaluated_constraints: report.evaluated_constraints,
        changed_values: report.updates.len(),
        updates: report
            .updates
            .into_iter()
            .map(|update| ParametricValueSnapshot {
                element_id: update.element_id.to_string(),
                previous_value: update.previous_value,
                value: update.value,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_constraint_parameter_gets_reusable_real_primitive_type() {
        let mut project = Project::new("Fresh Parametrics");
        let package = project
            .create_element(ElementKind::Package, "Analysis", project.root_id)
            .unwrap();
        let block = project
            .create_element(ElementKind::ConstraintBlock, "Equation", package)
            .unwrap();
        let context = project
            .create_element(ElementKind::Block, "System", package)
            .unwrap();

        let first = resolve_constraint_parameter_type(&mut project, block, None).unwrap();
        let second = resolve_constraint_parameter_type(&mut project, block, None).unwrap();
        let value_type = resolve_parametric_value_type(&mut project, context, None).unwrap();

        assert_eq!(first, second);
        assert_eq!(first, value_type);
        let real = project.element(first).unwrap();
        assert_eq!(real.kind, ElementKind::PrimitiveType);
        assert_eq!(real.name, "Real");
        assert_eq!(real.owner_id, Some(package));
        project
            .create_typed_feature(
                ElementKind::ConstraintParameter,
                "input",
                block,
                first,
                Multiplicity::ONE,
            )
            .unwrap();
        project
            .create_typed_feature(
                ElementKind::ValueProperty,
                "inputValue",
                context,
                value_type,
                Multiplicity::ONE,
            )
            .unwrap();
        project.validate().unwrap();
    }

    #[test]
    fn route_and_clean_layout_use_parameter_aware_shared_geometry() {
        let mut project = Project::new("Parametric geometry");
        let package = project
            .create_element(ElementKind::Package, "Analysis", project.root_id)
            .unwrap();
        let context = project
            .create_element(ElementKind::Block, "System", package)
            .unwrap();
        let value_type = project
            .create_element(ElementKind::ValueType, "Real", package)
            .unwrap();
        let first = project
            .create_typed_feature(
                ElementKind::ValueProperty,
                "first",
                context,
                value_type,
                Multiplicity::ONE,
            )
            .unwrap();
        let second = project
            .create_typed_feature(
                ElementKind::ValueProperty,
                "second",
                context,
                value_type,
                Multiplicity::ONE,
            )
            .unwrap();
        let obstacle = project
            .create_typed_feature(
                ElementKind::ValueProperty,
                "obstacle",
                context,
                value_type,
                Multiplicity::ONE,
            )
            .unwrap();
        let relationship = project
            .create_binding_connector(
                context,
                BindingEndpoint {
                    role_id: first,
                    parameter_id: None,
                },
                BindingEndpoint {
                    role_id: second,
                    parameter_id: None,
                },
            )
            .unwrap();
        let diagram_id = DiagramId::new().to_string();
        let nodes = vec![
            DiagramNode {
                id: "first".into(),
                element_id: first.to_string(),
                x: 720.0,
                y: 540.0,
                width: VALUE_WIDTH,
                height: VALUE_HEIGHT,
                actor_notation: None,
                parameter_presentations: Vec::new(),
            },
            DiagramNode {
                id: "second".into(),
                element_id: second.to_string(),
                x: 80.0,
                y: 90.0,
                width: VALUE_WIDTH,
                height: VALUE_HEIGHT,
                actor_notation: None,
                parameter_presentations: Vec::new(),
            },
            DiagramNode {
                id: "obstacle".into(),
                element_id: obstacle.to_string(),
                x: 390.0,
                y: 300.0,
                width: VALUE_WIDTH,
                height: VALUE_HEIGHT,
                actor_notation: None,
                parameter_presentations: Vec::new(),
            },
        ];
        let diagram = BddDiagram {
            id: diagram_id.clone(),
            name: "Analysis".into(),
            owner_id: package.to_string(),
            family: "parametric".into(),
            semantic_context_id: Some(context.to_string()),
            subject_boundary: None,
            nodes,
            edges: vec![DiagramEdge {
                id: "binding".into(),
                relationship_id: relationship.to_string(),
                source_node_id: "first".into(),
                target_node_id: "second".into(),
                points: Vec::new(),
                label_anchor: None,
            }],
        };
        let workspace = WorkspaceState::default();
        *workspace.project.lock().unwrap() = Some(project);
        *workspace.diagrams.lock().unwrap() = vec![diagram];

        assert!(route_parametric_with_bounds(&diagram_id, &workspace, None).unwrap());
        let routed = workspace.diagrams.lock().unwrap()[0].edges[0]
            .points
            .clone();
        assert!(routed.len() >= 2);
        assert!(layout_parametric_with_bounds(&diagram_id, &workspace, None).unwrap());
        let diagrams = workspace.diagrams.lock().unwrap();
        assert!(diagrams[0].edges[0].points.len() >= 2);
        assert!(
            diagrams[0]
                .nodes
                .iter()
                .all(|node| node.x >= 0.0 && node.y >= 0.0)
        );
    }

    #[test]
    fn definition_owned_parameters_receive_distinct_movable_presentations() {
        let mut project = Project::new("Parameter presentations");
        let package = project
            .create_element(ElementKind::Package, "Analysis", project.root_id)
            .unwrap();
        let context = project
            .create_element(ElementKind::Block, "System", package)
            .unwrap();
        let value_type = project
            .create_element(ElementKind::ValueType, "Real", package)
            .unwrap();
        let block = project
            .create_element(ElementKind::ConstraintBlock, "Equation", package)
            .unwrap();
        let parameter = project
            .create_typed_feature(
                ElementKind::ConstraintParameter,
                "result",
                block,
                value_type,
                Multiplicity::ONE,
            )
            .unwrap();
        let property = project
            .create_typed_feature(
                ElementKind::ConstraintProperty,
                "equation",
                context,
                block,
                Multiplicity::ONE,
            )
            .unwrap();
        let mut first = DiagramNode {
            id: "first".into(),
            element_id: property.to_string(),
            x: 100.0,
            y: 100.0,
            width: CONSTRAINT_WIDTH,
            height: CONSTRAINT_HEIGHT,
            actor_notation: None,
            parameter_presentations: Vec::new(),
        };
        let mut second = first.clone();
        second.id = "second".into();
        sync_parameter_presentations(&mut first, &project).unwrap();
        sync_parameter_presentations(&mut second, &project).unwrap();
        assert_eq!(
            first.parameter_presentations[0].parameter_id,
            parameter.to_string()
        );
        assert_ne!(
            first.parameter_presentations[0].id,
            second.parameter_presentations[0].id
        );
        let presentation = &first.parameter_presentations[0];
        assert!(presentation.offset_x == 0.0 || presentation.offset_y == 0.0);
    }
}
