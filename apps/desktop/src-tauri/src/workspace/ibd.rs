use super::routing::{RouteRect, RouteRequest, orthogonal_route};
use super::*;
use serde::{Deserialize, Serialize};
use systems_modeler_core::{Connector, ConnectorEnd, ConnectorKind, ItemFlow};

pub const IBD_METADATA_KEY: &str = "ibd-diagrams";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IbdPortPresentation {
    pub id: String,
    pub element_id: String,
    #[serde(default)]
    pub property_path: Vec<String>,
    pub x: f64,
    pub y: f64,
    pub size: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IbdPropertyPresentation {
    pub id: String,
    pub element_id: String,
    #[serde(default)]
    pub property_path: Vec<String>,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    #[serde(default)]
    pub ports: Vec<IbdPortPresentation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IbdConnectorPresentation {
    pub id: String,
    pub relationship_id: String,
    pub source_presentation_id: String,
    pub target_presentation_id: String,
    #[serde(default)]
    pub points: Vec<DiagramPoint>,
    #[serde(default)]
    pub label_anchor: Option<DiagramPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IbdDiagram {
    pub id: String,
    pub name: String,
    /// The semantic Block whose internal structure is shown.
    pub context_block_id: String,
    /// Model/package owner for repository organization.
    pub owner_id: String,
    #[serde(default)]
    pub properties: Vec<IbdPropertyPresentation>,
    #[serde(default)]
    pub boundary_ports: Vec<IbdPortPresentation>,
    #[serde(default)]
    pub connectors: Vec<IbdConnectorPresentation>,
}

fn parse_path(values: &[String]) -> Result<Vec<ElementId>, String> {
    values.iter().map(|value| parse_element_id(value)).collect()
}

fn property_rect(value: &IbdPropertyPresentation) -> RouteRect {
    RouteRect {
        x: value.x,
        y: value.y,
        width: value.width,
        height: value.height,
    }
}

fn port_rect(value: &IbdPortPresentation) -> RouteRect {
    RouteRect {
        x: value.x - value.size / 2.0,
        y: value.y - value.size / 2.0,
        width: value.size,
        height: value.size,
    }
}

fn ibd_end_for_presentation(
    diagram: &IbdDiagram,
    presentation_id: &str,
) -> Result<(ConnectorEnd, RouteRect), String> {
    if let Some(port) = diagram
        .boundary_ports
        .iter()
        .find(|port| port.id == presentation_id)
    {
        return Ok((
            ConnectorEnd::boundary(parse_element_id(&port.element_id)?),
            port_rect(port),
        ));
    }

    for property in &diagram.properties {
        if property.id == presentation_id {
            let path = parse_path(&property.property_path)?;
            let role_id = parse_element_id(&property.element_id)?;
            let end = if path.is_empty() {
                ConnectorEnd::role(role_id)
            } else {
                ConnectorEnd {
                    property_path: path,
                    role_id,
                    port_id: None,
                }
            };
            return Ok((end, property_rect(property)));
        }

        if let Some(port) = property
            .ports
            .iter()
            .find(|port| port.id == presentation_id)
        {
            let path = parse_path(&port.property_path)?;
            return Ok((
                ConnectorEnd::nested_port(path, parse_element_id(&port.element_id)?),
                port_rect(port),
            ));
        }
    }

    Err(format!(
        "IBD endpoint presentation not found: {presentation_id}"
    ))
}

fn routing_obstacles(diagram: &IbdDiagram, source_id: &str, target_id: &str) -> Vec<RouteRect> {
    let mut obstacles = Vec::new();
    for property in &diagram.properties {
        let owns_source =
            property.id == source_id || property.ports.iter().any(|port| port.id == source_id);
        let owns_target =
            property.id == target_id || property.ports.iter().any(|port| port.id == target_id);
        if !owns_source && !owns_target {
            obstacles.push(property_rect(property));
        }
        for port in &property.ports {
            if port.id != source_id && port.id != target_id {
                obstacles.push(port_rect(port));
            }
        }
    }
    for port in &diagram.boundary_ports {
        if port.id != source_id && port.id != target_id {
            obstacles.push(port_rect(port));
        }
    }
    obstacles
}

fn all_routing_obstacles(diagram: &IbdDiagram) -> Vec<RouteRect> {
    let mut obstacles = Vec::new();
    for property in &diagram.properties {
        obstacles.push(property_rect(property));
        obstacles.extend(property.ports.iter().map(port_rect));
    }
    obstacles.extend(diagram.boundary_ports.iter().map(port_rect));
    obstacles
}

fn lane_index(diagram: &IbdDiagram, source_id: &str, target_id: &str) -> usize {
    diagram
        .connectors
        .iter()
        .filter(|edge| {
            (edge.source_presentation_id == source_id && edge.target_presentation_id == target_id)
                || (edge.source_presentation_id == target_id
                    && edge.target_presentation_id == source_id)
        })
        .count()
}

pub fn route_ibd_edge(
    diagram: &IbdDiagram,
    source_id: &str,
    target_id: &str,
) -> Result<Vec<DiagramPoint>, String> {
    route_ibd_edge_avoiding(
        diagram,
        source_id,
        target_id,
        lane_index(diagram, source_id, target_id),
        &[],
        false,
        &[],
        None,
    )
}

fn route_ibd_edge_avoiding(
    diagram: &IbdDiagram,
    source_id: &str,
    target_id: &str,
    lane_index: usize,
    reserved_routes: &[Vec<DiagramPoint>],
    allow_shared_departure: bool,
    additional_obstacles: &[RouteRect],
    bounds: Option<RouteRect>,
) -> Result<Vec<DiagramPoint>, String> {
    let (_, source_rect) = ibd_end_for_presentation(diagram, source_id)?;
    let (_, target_rect) = ibd_end_for_presentation(diagram, target_id)?;
    let obstacles: Vec<_> = routing_obstacles(diagram, source_id, target_id)
        .into_iter()
        .chain(additional_obstacles.iter().copied())
        .collect();
    orthogonal_route(RouteRequest {
        source: source_rect,
        target: target_rect,
        obstacles: &obstacles,
        lane_index,
        reserved_routes,
        allow_shared_departure,
        bounds,
    })
}

pub fn validate_ibd_diagrams(project: &Project, diagrams: &[IbdDiagram]) -> Result<(), String> {
    let mut diagram_ids = HashSet::new();
    let mut presentation_ids = HashSet::new();

    for diagram in diagrams {
        parse_diagram_id(&diagram.id)?;
        if !diagram_ids.insert(&diagram.id) {
            return Err(format!("duplicate IBD diagram id: {}", diagram.id));
        }

        let context_id = parse_element_id(&diagram.context_block_id)?;
        let context = project
            .element(context_id)
            .map_err(|error| error.to_string())?;
        if !matches!(
            context.kind,
            ElementKind::Block | ElementKind::AssociationBlock
        ) {
            return Err(format!(
                "IBD context must be a Block or AssociationBlock: {}",
                diagram.context_block_id
            ));
        }

        let owner = project
            .element(parse_element_id(&diagram.owner_id)?)
            .map_err(|error| error.to_string())?;
        if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
            return Err("IBD repository owner must be Model or Package".into());
        }

        for property in &diagram.properties {
            if !presentation_ids.insert(&property.id) {
                return Err(format!("duplicate IBD presentation id: {}", property.id));
            }

            let element = project
                .element(parse_element_id(&property.element_id)?)
                .map_err(|error| error.to_string())?;
            if !matches!(
                element.kind,
                ElementKind::PartProperty | ElementKind::ReferenceProperty
            ) {
                return Err(format!(
                    "IBD structural node must reference PartProperty or ReferenceProperty: {}",
                    property.element_id
                ));
            }

            let path = parse_path(&property.property_path)?;
            if path.last().copied() != Some(element.id) {
                return Err(format!(
                    "IBD property path must terminate at the presented property: {}",
                    property.element_id
                ));
            }
            project
                .resolve_structural_path(context_id, &path)
                .map_err(|error| error.to_string())?;

            for port in &property.ports {
                if !presentation_ids.insert(&port.id) {
                    return Err(format!("duplicate IBD port presentation id: {}", port.id));
                }
                let port_element = project
                    .element(parse_element_id(&port.element_id)?)
                    .map_err(|error| error.to_string())?;
                if !port_element.is_port() {
                    return Err(format!(
                        "IBD port presentation does not reference a port: {}",
                        port.element_id
                    ));
                }
                let end =
                    ConnectorEnd::nested_port(parse_path(&port.property_path)?, port_element.id);
                project
                    .validate_connector_end(context_id, &end)
                    .map_err(|error| error.to_string())?;
            }
        }

        for port in &diagram.boundary_ports {
            if !presentation_ids.insert(&port.id) {
                return Err(format!("duplicate IBD port presentation id: {}", port.id));
            }
            let end = ConnectorEnd::boundary(parse_element_id(&port.element_id)?);
            project
                .validate_connector_end(context_id, &end)
                .map_err(|error| error.to_string())?;
        }

        for edge in &diagram.connectors {
            if !presentation_ids.insert(&edge.id) {
                return Err(format!(
                    "duplicate IBD connector presentation id: {}",
                    edge.id
                ));
            }
            let relationship = project
                .relationship(parse_relationship_id(&edge.relationship_id)?)
                .map_err(|error| error.to_string())?;
            if relationship.kind != RelationshipKind::Connector {
                return Err(format!(
                    "IBD connector edge does not reference a semantic Connector: {}",
                    edge.relationship_id
                ));
            }
            let (source, _) = ibd_end_for_presentation(diagram, &edge.source_presentation_id)?;
            let (target, _) = ibd_end_for_presentation(diagram, &edge.target_presentation_id)?;
            let semantic = relationship
                .connector
                .as_ref()
                .ok_or("Connector semantics missing")?;
            if semantic.source != source || semantic.target != target {
                return Err(format!(
                    "IBD presentation endpoints do not match semantic Connector: {}",
                    edge.relationship_id
                ));
            }
            if edge.points.len() < 2 {
                return Err(format!("IBD connector has no usable route: {}", edge.id));
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub fn create_ibd(
    context_block_id: String,
    owner_id: String,
    name: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let context_id = parse_element_id(&context_block_id)?;
    let owner_id_parsed = parse_element_id(&owner_id)?;
    {
        let project = state.project.lock().map_err(|_| "project lock poisoned")?;
        let project = project.as_ref().ok_or("no project open")?;
        let context = project
            .element(context_id)
            .map_err(|error| error.to_string())?;
        if !matches!(
            context.kind,
            ElementKind::Block | ElementKind::AssociationBlock
        ) {
            return Err("IBD context must be a Block or AssociationBlock".into());
        }
        let owner = project
            .element(owner_id_parsed)
            .map_err(|error| error.to_string())?;
        if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
            return Err("IBD repository owner must be a Model or Package".into());
        }
    }

    if state
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")?
        .iter()
        .any(|d| d.context_block_id == context_block_id)
    {
        return Err("this Block already has an Internal Block Diagram".into());
    }

    let id = DiagramId::new().to_string();
    state
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")?
        .push(IbdDiagram {
            id: id.clone(),
            name,
            context_block_id,
            owner_id,
            properties: Vec::new(),
            boundary_ports: Vec::new(),
            connectors: Vec::new(),
        });
    Ok(id)
}

#[tauri::command]
pub fn populate_ibd_from_context(
    diagram_id: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;
    let mut diagrams = state.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|d| d.id == diagram_id)
        .ok_or("IBD not found")?;
    let context = parse_element_id(&diagram.context_block_id)?;
    let mut x = 120.0;
    let mut y = 120.0;

    for feature in project.children(context) {
        match feature.kind {
            ElementKind::PartProperty | ElementKind::ReferenceProperty => {
                if diagram
                    .properties
                    .iter()
                    .any(|p| p.element_id == feature.id.to_string())
                {
                    continue;
                }
                diagram.properties.push(IbdPropertyPresentation {
                    id: uuid::Uuid::new_v4().to_string(),
                    element_id: feature.id.to_string(),
                    property_path: vec![feature.id.to_string()],
                    x,
                    y,
                    width: 190.0,
                    height: 100.0,
                    ports: Vec::new(),
                });
                x += 240.0;
                if x > 780.0 {
                    x = 120.0;
                    y += 180.0;
                }
            }
            ElementKind::ProxyPort | ElementKind::FullPort => {
                if diagram
                    .boundary_ports
                    .iter()
                    .any(|p| p.element_id == feature.id.to_string())
                {
                    continue;
                }
                diagram.boundary_ports.push(IbdPortPresentation {
                    id: uuid::Uuid::new_v4().to_string(),
                    element_id: feature.id.to_string(),
                    property_path: Vec::new(),
                    x: 55.0,
                    y: 100.0 + diagram.boundary_ports.len() as f64 * 70.0,
                    size: 16.0,
                });
            }
            _ => {}
        }
    }

    Ok(())
}

#[tauri::command]
pub fn add_nested_port_to_ibd(
    diagram_id: String,
    property_presentation_id: String,
    port_id: String,
    side: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let port_id_parsed = parse_element_id(&port_id)?;
    let project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_ref().ok_or("no project open")?;
    let mut diagrams = state.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|d| d.id == diagram_id)
        .ok_or("IBD not found")?;
    let context_id = parse_element_id(&diagram.context_block_id)?;
    let property = diagram
        .properties
        .iter_mut()
        .find(|p| p.id == property_presentation_id)
        .ok_or("property presentation not found")?;
    let end = ConnectorEnd::nested_port(parse_path(&property.property_path)?, port_id_parsed);
    project
        .validate_connector_end(context_id, &end)
        .map_err(|error| error.to_string())?;

    if property.ports.iter().any(|p| p.element_id == port_id) {
        return Err("that port is already presented on this property presentation".into());
    }

    let id = uuid::Uuid::new_v4().to_string();
    let (x, y) = match side.as_str() {
        "left" => (property.x, property.y + property.height / 2.0),
        "right" => (
            property.x + property.width,
            property.y + property.height / 2.0,
        ),
        "top" => (property.x + property.width / 2.0, property.y),
        "bottom" => (
            property.x + property.width / 2.0,
            property.y + property.height,
        ),
        _ => return Err("port side must be left, right, top, or bottom".into()),
    };

    property.ports.push(IbdPortPresentation {
        id: id.clone(),
        element_id: port_id,
        property_path: property.property_path.clone(),
        x,
        y,
        size: 16.0,
    });
    Ok(id)
}

#[tauri::command]
pub fn create_ibd_connector(
    diagram_id: String,
    kind: String,
    source_presentation_id: String,
    target_presentation_id: String,
    name: Option<String>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let mut project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_mut().ok_or("no project open")?;
    let mut diagrams = state.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|d| d.id == diagram_id)
        .ok_or("IBD not found")?;
    let (source, _) = ibd_end_for_presentation(diagram, &source_presentation_id)?;
    let (target, _) = ibd_end_for_presentation(diagram, &target_presentation_id)?;
    let connector_kind = match kind.as_str() {
        "Assembly" => ConnectorKind::Assembly,
        "Delegation" => ConnectorKind::Delegation,
        _ => return Err("connector kind must be Assembly or Delegation".into()),
    };
    let semantic_id = project
        .create_connector(Connector {
            context_id: parse_element_id(&diagram.context_block_id)?,
            kind: connector_kind,
            source,
            target,
        })
        .map_err(|error| error.to_string())?;

    if let Some(name) = name {
        project.relationships.get_mut(&semantic_id).unwrap().name = name.trim().to_string();
    }

    let points = route_ibd_edge(diagram, &source_presentation_id, &target_presentation_id)?;
    diagram.connectors.push(IbdConnectorPresentation {
        id: uuid::Uuid::new_v4().to_string(),
        relationship_id: semantic_id.to_string(),
        source_presentation_id,
        target_presentation_id,
        label_anchor: Some(super::routing::route_label_anchor(&points)),
        points,
    });
    Ok(semantic_id.to_string())
}

#[tauri::command]
pub fn add_item_flow_to_connector(
    relationship_id: String,
    conveyed_item_ids: Vec<String>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<String, String> {
    let connector_id = parse_relationship_id(&relationship_id)?;
    let conveyed: Vec<_> = conveyed_item_ids
        .iter()
        .map(|id| parse_element_id(id))
        .collect::<Result<_, _>>()?;
    let mut project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_mut().ok_or("no project open")?;
    let connector = project
        .relationship(connector_id)
        .map_err(|error| error.to_string())?
        .connector
        .clone()
        .ok_or("relationship is not a Connector")?;
    project
        .create_item_flow(ItemFlow {
            connector_id,
            source: connector.source,
            target: connector.target,
            conveyed_item_ids: conveyed,
        })
        .map(|id| id.to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn route_ibd(
    diagram_id: String,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    route_ibd_with_bounds(&diagram_id, &state, None)?;
    Ok(())
}

fn routed_ibd_connectors(
    diagram: &IbdDiagram,
    bounds: Option<RouteRect>,
) -> Result<Vec<IbdConnectorPresentation>, String> {
    let snapshot = diagram.clone();
    let all_obstacles = all_routing_obstacles(&snapshot);
    let mut routes = Vec::new();
    let mut label_obstacles = Vec::new();
    let mut connectors = snapshot.connectors.clone();
    for (index, edge) in snapshot.connectors.iter().enumerate() {
        let same_source_count = snapshot.connectors[..index]
            .iter()
            .filter(|candidate| candidate.source_presentation_id == edge.source_presentation_id)
            .count();
        let points = route_ibd_edge_avoiding(
            &snapshot,
            &edge.source_presentation_id,
            &edge.target_presentation_id,
            same_source_count,
            &routes,
            same_source_count > 0,
            &label_obstacles,
            bounds,
        )?;
        let obstacles: Vec<_> = all_obstacles
            .iter()
            .copied()
            .chain(label_obstacles.iter().copied())
            .collect();
        let label_anchor =
            super::routing::route_label_anchor_avoiding(&points, &obstacles, &routes, bounds)?;
        let connector = connectors
            .iter_mut()
            .find(|candidate| candidate.id == edge.id)
            .ok_or("IBD connector presentation not found")?;
        connector.points = points.clone();
        connector.label_anchor = Some(label_anchor);
        routes.push(points);
        label_obstacles.push(super::routing::label_rect(label_anchor));
    }
    Ok(connectors)
}

fn ibd_presentation_changed(left: &IbdDiagram, right: &IbdDiagram) -> bool {
    left.properties.len() != right.properties.len()
        || left.connectors.len() != right.connectors.len()
        || left
            .properties
            .iter()
            .zip(&right.properties)
            .any(|(left, right)| {
                left.id != right.id
                    || left.x != right.x
                    || left.y != right.y
                    || left.width != right.width
                    || left.height != right.height
                    || left.ports.iter().zip(&right.ports).any(|(left, right)| {
                        left.id != right.id || left.x != right.x || left.y != right.y
                    })
            })
        || left
            .connectors
            .iter()
            .zip(&right.connectors)
            .any(|(left, right)| {
                left.id != right.id
                    || left.points != right.points
                    || left.label_anchor != right.label_anchor
            })
}

pub(super) fn route_ibd_with_bounds(
    diagram_id: &str,
    state: &WorkspaceState,
    bounds: Option<RouteRect>,
) -> Result<bool, String> {
    let mut diagrams = state.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")?;
    let diagram = diagrams
        .iter_mut()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("IBD not found")?;
    let connectors = routed_ibd_connectors(diagram, bounds)?;
    let changed = diagram
        .connectors
        .iter()
        .zip(&connectors)
        .any(|(left, right)| {
            left.points != right.points || left.label_anchor != right.label_anchor
        });
    if changed {
        diagram.connectors = connectors;
    }
    Ok(changed)
}

pub(super) fn layout_ibd_with_bounds(
    diagram_id: &str,
    state: &WorkspaceState,
    bounds: Option<RouteRect>,
) -> Result<bool, String> {
    let mut diagrams = state.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")?;
    let index = diagrams
        .iter()
        .position(|diagram| diagram.id == diagram_id)
        .ok_or("IBD not found")?;
    let original = diagrams[index].clone();
    let mut candidate = original.clone();
    let owner = |presentation_id: &str| {
        candidate.properties.iter().find_map(|property| {
            (property.id == presentation_id
                || property.ports.iter().any(|port| port.id == presentation_id))
            .then(|| property.id.clone())
        })
    };
    let edges: Vec<_> = candidate
        .connectors
        .iter()
        .filter_map(|edge| {
            Some((
                owner(&edge.source_presentation_id)?,
                owner(&edge.target_presentation_id)?,
            ))
        })
        .collect();
    let positions = super::layout::hierarchical_positions_sized(
        candidate
            .properties
            .iter()
            .map(|property| super::layout::LayoutNode {
                id: property.id.clone(),
                width: property.width,
                height: property.height,
            }),
        &edges,
        systems_modeler_core::PreferredFlowDirection::LeftToRight,
    );
    for property in &mut candidate.properties {
        if let Some((x, y)) = positions.get(&property.id) {
            let dx = *x - property.x;
            let dy = *y - property.y;
            property.x = *x;
            property.y = *y;
            for port in &mut property.ports {
                port.x += dx;
                port.y += dy;
            }
        }
    }
    candidate.connectors = routed_ibd_connectors(&candidate, bounds)?;
    let changed = ibd_presentation_changed(&original, &candidate);
    if changed {
        diagrams[index] = candidate;
    }
    Ok(changed)
}

pub fn save_ibd_metadata(
    database: &mut ProjectDatabase,
    project: &Project,
    diagrams: &[IbdDiagram],
) -> Result<(), String> {
    validate_ibd_diagrams(project, diagrams)?;
    let payload = serde_json::to_string(diagrams).map_err(|error| error.to_string())?;
    database
        .save_metadata(project.id, IBD_METADATA_KEY, &payload)
        .map_err(|error| error.to_string())
}

pub fn load_ibd_metadata(
    database: &ProjectDatabase,
    project: &Project,
) -> Result<Vec<IbdDiagram>, String> {
    let diagrams = match database
        .load_metadata(project.id, IBD_METADATA_KEY)
        .map_err(|error| error.to_string())?
    {
        Some(payload) => serde_json::from_str::<Vec<IbdDiagram>>(&payload)
            .map_err(|error| format!("invalid saved IBD presentation data: {error}"))?,
        None => Vec::new(),
    };
    validate_ibd_diagrams(project, &diagrams)?;
    Ok(diagrams)
}
