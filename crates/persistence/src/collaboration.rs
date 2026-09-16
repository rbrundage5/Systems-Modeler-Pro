//! Server-side persistence foundation. Caller identities must come from a trusted
//! authentication layer, never from an unverified request field.
use crate::{PersistenceError, ProjectDatabase};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use systems_modeler_core::{
    DiagramId, ElementId, ElementKind, GeometryPoint, ModelError, ProjectId, RelationshipId,
    RelationshipKind,
    routing::{DiagramRouteEdge, RouteRect, route_diagram, route_is_clear},
};
use thiserror::Error;
use uuid::Uuid;

pub const COLLABORATION_PROTOCOL_VERSION: u32 = 1;
pub const COLLABORATION_CAPABILITIES: &[&str] = &[
    "revisioned-operations",
    "shared-bdd",
    "simple-relationships",
    "server-routed-bdd-relationships",
    "shared-requirements-v1",
    "authenticated-actor-v1",
];

const MIN_BDD_NODE_WIDTH: f64 = 48.0;
const MIN_BDD_NODE_HEIGHT: f64 = 32.0;
const BDD_GRID_COLUMNS: usize = 4;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SharedBddNode {
    pub id: Uuid,
    pub element: ElementId,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SharedBddEdge {
    pub id: Uuid,
    pub relationship: RelationshipId,
    pub source_node: Uuid,
    pub target_node: Uuid,
    pub points: Vec<GeometryPoint>,
    pub label_anchor: GeometryPoint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SharedBddDiagram {
    pub id: DiagramId,
    pub name: String,
    pub owner: ElementId,
    pub nodes: Vec<SharedBddNode>,
    #[serde(default)]
    pub edges: Vec<SharedBddEdge>,
}

/// Relationship kinds whose complete semantic payload is source, target and
/// namespace owner. Relationships with additional required structure (such as
/// Association ends, Connectors, ItemFlows and BindingConnectors) use separate
/// collaboration operations so their invariants cannot be bypassed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SharedRelationshipKind {
    Dependency,
    Generalization,
    Realization,
    Allocate,
    DeriveRequirement,
    Satisfy,
    Verify,
    Refine,
    Trace,
    Copy,
    Include,
    Extend,
}

impl SharedRelationshipKind {
    fn model_kind(self) -> RelationshipKind {
        match self {
            Self::Dependency => RelationshipKind::Dependency,
            Self::Generalization => RelationshipKind::Generalization,
            Self::Realization => RelationshipKind::Realization,
            Self::Allocate => RelationshipKind::Allocate,
            Self::DeriveRequirement => RelationshipKind::DeriveRequirement,
            Self::Satisfy => RelationshipKind::Satisfy,
            Self::Verify => RelationshipKind::Verify,
            Self::Refine => RelationshipKind::Refine,
            Self::Trace => RelationshipKind::Trace,
            Self::Copy => RelationshipKind::Copy,
            Self::Include => RelationshipKind::Include,
            Self::Extend => RelationshipKind::Extend,
        }
    }

    fn supports(kind: &RelationshipKind) -> bool {
        matches!(
            kind,
            RelationshipKind::Dependency
                | RelationshipKind::Generalization
                | RelationshipKind::Realization
                | RelationshipKind::Allocate
                | RelationshipKind::DeriveRequirement
                | RelationshipKind::Satisfy
                | RelationshipKind::Verify
                | RelationshipKind::Refine
                | RelationshipKind::Trace
                | RelationshipKind::Copy
                | RelationshipKind::Include
                | RelationshipKind::Extend
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SharedEdit {
    CreateBlock {
        owner: ElementId,
        name: String,
    },
    CreatePackage {
        owner: ElementId,
        name: String,
    },
    RenameElement {
        element: ElementId,
        name: String,
    },
    CreateRequirement {
        owner: ElementId,
        name: String,
        requirement_id: String,
        text: String,
    },
    UpdateRequirement {
        element: ElementId,
        name: String,
        requirement_id: String,
        text: String,
    },
    CreateTestCase {
        owner: ElementId,
        name: String,
    },
    CreateRelationship {
        kind: SharedRelationshipKind,
        source: ElementId,
        target: ElementId,
        owner: ElementId,
    },
    DeleteRelationship {
        relationship: RelationshipId,
    },
    CreateBddDiagram {
        diagram: DiagramId,
        owner: ElementId,
        name: String,
    },
    RenameBddDiagram {
        diagram: DiagramId,
        name: String,
    },
    DeleteBddDiagram {
        diagram: DiagramId,
    },
    PlaceBddElement {
        diagram: DiagramId,
        node: Uuid,
        element: ElementId,
    },
    UpdateBddNodeGeometry {
        diagram: DiagramId,
        node: Uuid,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    RemoveBddNode {
        diagram: DiagramId,
        node: Uuid,
    },
    PresentBddRelationship {
        diagram: DiagramId,
        edge: Uuid,
        relationship: RelationshipId,
    },
    RemoveBddEdge {
        diagram: DiagramId,
        edge: Uuid,
    },
    RouteBddDiagram {
        diagram: DiagramId,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditRequest {
    pub operation_id: Uuid,
    pub expected_revision: i64,
    pub edit: SharedEdit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitReceipt {
    pub revision: i64,
    pub element: ElementId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectRole {
    Viewer,
    Editor,
}

#[derive(Debug, Error)]
pub enum CollaborationError {
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error("project access denied")]
    Forbidden,
    #[error("project revision changed: expected {expected}, current {current}")]
    Conflict { expected: i64, current: i64 },
    #[error("operation ID was already used for a different request or user")]
    OperationIdReused,
    #[error("name must contain between 1 and 1024 bytes of nonblank text")]
    InvalidName,
    #[error("shared BDD diagram was not found")]
    DiagramNotFound,
    #[error("shared BDD diagram edit is invalid: {0}")]
    InvalidDiagram(&'static str),
    #[error("shared relationship edit is invalid: {0}")]
    InvalidRelationship(&'static str),
    #[error("project revision exhausted")]
    RevisionExhausted,
}

fn validate_name(value: &str) -> Result<(), CollaborationError> {
    if value.trim().is_empty() || value.len() > 1024 {
        return Err(CollaborationError::InvalidName);
    }
    Ok(())
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

fn default_bdd_node_size(kind: &ElementKind) -> (f64, f64) {
    match kind {
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
}

fn validate_geometry(node: &SharedBddNode) -> Result<(), CollaborationError> {
    if node.id.is_nil()
        || !node.x.is_finite()
        || !node.y.is_finite()
        || !node.width.is_finite()
        || !node.height.is_finite()
        || node.width < MIN_BDD_NODE_WIDTH
        || node.height < MIN_BDD_NODE_HEIGHT
    {
        return Err(CollaborationError::InvalidDiagram(
            "node geometry or identity is invalid",
        ));
    }
    Ok(())
}

fn bdd_node_rect(node: &SharedBddNode) -> RouteRect {
    RouteRect {
        x: node.x,
        y: node.y,
        width: node.width,
        height: node.height,
    }
}

fn point_on_rect_boundary(point: GeometryPoint, rect: RouteRect) -> bool {
    const EPSILON: f64 = 0.001;
    let right = rect.x + rect.width;
    let bottom = rect.y + rect.height;
    let within_x = point.x >= rect.x - EPSILON && point.x <= right + EPSILON;
    let within_y = point.y >= rect.y - EPSILON && point.y <= bottom + EPSILON;
    let on_vertical = (point.x - rect.x).abs() <= EPSILON || (point.x - right).abs() <= EPSILON;
    let on_horizontal = (point.y - rect.y).abs() <= EPSILON || (point.y - bottom).abs() <= EPSILON;
    within_x && within_y && (on_vertical || on_horizontal)
}

fn validate_bdd_edge_geometry(
    diagram: &SharedBddDiagram,
    edge: &SharedBddEdge,
    source: &SharedBddNode,
    target: &SharedBddNode,
) -> Result<(), CollaborationError> {
    if edge.id.is_nil()
        || edge.points.len() < 2
        || !edge.label_anchor.x.is_finite()
        || !edge.label_anchor.y.is_finite()
        || edge
            .points
            .iter()
            .any(|point| !point.x.is_finite() || !point.y.is_finite())
        || edge.points.windows(2).any(|segment| {
            let horizontal = (segment[0].y - segment[1].y).abs() <= 0.001;
            let vertical = (segment[0].x - segment[1].x).abs() <= 0.001;
            !(horizontal ^ vertical)
        })
        || !point_on_rect_boundary(edge.points[0], bdd_node_rect(source))
        || !point_on_rect_boundary(
            *edge.points.last().expect("length was checked"),
            bdd_node_rect(target),
        )
    {
        return Err(CollaborationError::InvalidDiagram(
            "BDD edge routing geometry or identity is invalid",
        ));
    }
    let obstacles: Vec<_> = diagram
        .nodes
        .iter()
        .filter(|node| node.id != source.id && node.id != target.id)
        .map(bdd_node_rect)
        .collect();
    if !route_is_clear(&edge.points, &obstacles) {
        return Err(CollaborationError::InvalidDiagram(
            "BDD edge route intersects a presented element",
        ));
    }
    Ok(())
}

fn route_shared_bdd(diagram: &mut SharedBddDiagram) -> Result<(), CollaborationError> {
    let obstacles: Vec<_> = diagram.nodes.iter().map(bdd_node_rect).collect();
    let requested = diagram
        .edges
        .iter()
        .map(|edge| {
            let source = diagram
                .nodes
                .iter()
                .find(|node| node.id == edge.source_node)
                .ok_or(CollaborationError::InvalidDiagram(
                    "BDD edge source presentation was not found",
                ))?;
            let target = diagram
                .nodes
                .iter()
                .find(|node| node.id == edge.target_node)
                .ok_or(CollaborationError::InvalidDiagram(
                    "BDD edge target presentation was not found",
                ))?;
            Ok(DiagramRouteEdge {
                id: edge.id.to_string(),
                source_id: source.id.to_string(),
                target_id: target.id.to_string(),
                source: bdd_node_rect(source),
                target: bdd_node_rect(target),
            })
        })
        .collect::<Result<Vec<_>, CollaborationError>>()?;
    let routed = route_diagram(&requested, &obstacles).map_err(|_| {
        CollaborationError::InvalidDiagram(
            "no obstacle-clear orthogonal BDD relationship route is available",
        )
    })?;
    for route in routed {
        let routed_id = Uuid::parse_str(&route.id).map_err(|_| {
            CollaborationError::InvalidDiagram("router returned an invalid BDD edge identity")
        })?;
        let edge = diagram
            .edges
            .iter_mut()
            .find(|edge| edge.id == routed_id)
            .ok_or(CollaborationError::InvalidDiagram(
                "routed BDD edge presentation was not found",
            ))?;
        edge.points = route.points;
        edge.label_anchor = route.label_anchor;
    }
    Ok(())
}

fn validate_shared_bdd(
    project: &systems_modeler_core::Project,
    diagram: &SharedBddDiagram,
) -> Result<(), CollaborationError> {
    validate_name(&diagram.name)?;
    let owner = project.element(diagram.owner)?;
    if !matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
        return Err(CollaborationError::InvalidDiagram(
            "BDD owner must be a Model or Package",
        ));
    }
    let mut nodes = std::collections::HashSet::new();
    let mut elements = std::collections::HashSet::new();
    for node in &diagram.nodes {
        validate_geometry(node)?;
        if !nodes.insert(node.id) || !elements.insert(node.element) {
            return Err(CollaborationError::InvalidDiagram(
                "duplicate BDD node or semantic presentation",
            ));
        }
        let element = project.element(node.element)?;
        if !bdd_presentable(&element.kind) {
            return Err(CollaborationError::InvalidDiagram(
                "semantic element kind is not valid on a BDD",
            ));
        }
    }
    let mut edges = std::collections::HashSet::new();
    let mut relationships = std::collections::HashSet::new();
    for edge in &diagram.edges {
        if !edges.insert(edge.id) || !relationships.insert(edge.relationship) {
            return Err(CollaborationError::InvalidDiagram(
                "duplicate BDD edge or semantic relationship presentation",
            ));
        }
        let relationship = project.relationship(edge.relationship)?;
        if !SharedRelationshipKind::supports(&relationship.kind) {
            return Err(CollaborationError::InvalidDiagram(
                "relationship kind requires a specialized BDD presentation",
            ));
        }
        let source = diagram
            .nodes
            .iter()
            .find(|node| node.id == edge.source_node)
            .ok_or(CollaborationError::InvalidDiagram(
                "BDD edge source presentation was not found",
            ))?;
        let target = diagram
            .nodes
            .iter()
            .find(|node| node.id == edge.target_node)
            .ok_or(CollaborationError::InvalidDiagram(
                "BDD edge target presentation was not found",
            ))?;
        if source.element != relationship.source_id || target.element != relationship.target_id {
            return Err(CollaborationError::InvalidDiagram(
                "BDD edge endpoints do not match the semantic relationship",
            ));
        }
        validate_bdd_edge_geometry(diagram, edge, source, target)?;
    }
    Ok(())
}

impl ProjectDatabase {
    pub(crate) fn migrate_collaboration(&self) -> Result<(), PersistenceError> {
        self.connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS shared_projects (
                project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
                revision INTEGER NOT NULL DEFAULT 0 CHECK(revision >= 0)
            );
            CREATE TABLE IF NOT EXISTS shared_members (
                project_id TEXT NOT NULL REFERENCES shared_projects(project_id) ON DELETE CASCADE,
                actor_id TEXT NOT NULL,
                role TEXT NOT NULL CHECK(role IN ('viewer','editor')),
                PRIMARY KEY(project_id,actor_id)
            );
            CREATE TABLE IF NOT EXISTS shared_operations (
                project_id TEXT NOT NULL REFERENCES shared_projects(project_id) ON DELETE CASCADE,
                operation_id TEXT NOT NULL,
                actor_id TEXT NOT NULL,
                request TEXT NOT NULL,
                revision INTEGER NOT NULL,
                element_id TEXT NOT NULL,
                committed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY(project_id,operation_id),
                UNIQUE(project_id,revision)
            );
            CREATE TABLE IF NOT EXISTS shared_bdd_diagrams (
                project_id TEXT NOT NULL REFERENCES shared_projects(project_id) ON DELETE CASCADE,
                diagram_id TEXT NOT NULL,
                payload TEXT NOT NULL,
                PRIMARY KEY(project_id,diagram_id)
            );",
        )?;
        Ok(())
    }

    /// Trusted administration only. Transport handlers must not expose this
    /// function as a client-authorized membership or ownership claim.
    pub fn provision_shared_member(
        &mut self,
        project: ProjectId,
        actor: Uuid,
        role: ProjectRole,
    ) -> Result<(), CollaborationError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT OR IGNORE INTO shared_projects(project_id) VALUES(?1)",
            [project.to_string()],
        )?;
        let role = match role {
            ProjectRole::Viewer => "viewer",
            ProjectRole::Editor => "editor",
        };
        tx.execute(
            "INSERT INTO shared_members(project_id,actor_id,role) VALUES(?1,?2,?3)
             ON CONFLICT(project_id,actor_id) DO UPDATE SET role=excluded.role",
            params![project.to_string(), actor.to_string(), role],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn load_shared_bdd_diagrams_from(
        connection: &Connection,
        project: ProjectId,
    ) -> Result<Vec<SharedBddDiagram>, CollaborationError> {
        let mut statement = connection.prepare(
            "SELECT payload FROM shared_bdd_diagrams WHERE project_id=?1 ORDER BY diagram_id",
        )?;
        let rows = statement.query_map([project.to_string()], |row| row.get::<_, String>(0))?;
        let mut diagrams = Vec::new();
        for row in rows {
            diagrams.push(serde_json::from_str(&row?)?);
        }
        Ok(diagrams)
    }

    fn load_shared_bdd_diagram_from(
        connection: &Connection,
        project: ProjectId,
        diagram: DiagramId,
    ) -> Result<SharedBddDiagram, CollaborationError> {
        let payload = connection
            .query_row(
                "SELECT payload FROM shared_bdd_diagrams WHERE project_id=?1 AND diagram_id=?2",
                params![project.to_string(), diagram.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(CollaborationError::DiagramNotFound)?;
        Ok(serde_json::from_str(&payload)?)
    }

    fn save_shared_bdd_diagram_to(
        connection: &Connection,
        project: ProjectId,
        diagram: &SharedBddDiagram,
    ) -> Result<(), CollaborationError> {
        connection.execute(
            "INSERT INTO shared_bdd_diagrams(project_id,diagram_id,payload) VALUES(?1,?2,?3)
             ON CONFLICT(project_id,diagram_id) DO UPDATE SET payload=excluded.payload",
            params![
                project.to_string(),
                diagram.id.to_string(),
                serde_json::to_string(diagram)?,
            ],
        )?;
        Ok(())
    }

    /// Returns one transactionally consistent semantic + BDD presentation snapshot
    /// and revision for an authorized member.
    pub fn shared_snapshot(
        &self,
        project: ProjectId,
        actor: Uuid,
    ) -> Result<(systems_modeler_core::Project, Vec<SharedBddDiagram>, i64), CollaborationError>
    {
        let tx = self.connection.unchecked_transaction()?;
        let revision = self.member_revision(project, actor, false)?;
        let snapshot = self.load_project(project)?;
        let diagrams = Self::load_shared_bdd_diagrams_from(&tx, project)?;
        for diagram in &diagrams {
            validate_shared_bdd(&snapshot, diagram)?;
        }
        tx.commit()?;
        Ok((snapshot, diagrams, revision))
    }

    fn member_revision(
        &self,
        project: ProjectId,
        actor: Uuid,
        editing: bool,
    ) -> Result<i64, CollaborationError> {
        self.connection
            .query_row(
                "SELECT p.revision FROM shared_projects p JOIN shared_members m
                 ON p.project_id=m.project_id
                 WHERE p.project_id=?1 AND m.actor_id=?2 AND (?3=0 OR m.role='editor')",
                params![project.to_string(), actor.to_string(), editing],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(CollaborationError::Forbidden)
    }

    /// Commits a semantic or BDD presentation edit, model state, revision and retry
    /// receipt together. Uses a database write lock so separate server connections
    /// cannot both accept the same base revision.
    pub fn commit_shared_edit(
        &self,
        project: ProjectId,
        authenticated_actor: Uuid,
        request: &EditRequest,
    ) -> Result<CommitReceipt, CollaborationError> {
        let tx =
            rusqlite::Transaction::new_unchecked(&self.connection, TransactionBehavior::Immediate)?;
        let current = self.member_revision(project, authenticated_actor, true)?;
        let serialized = serde_json::to_string(request)?;
        let previous: Option<(String, String, i64, String)> = tx
            .query_row(
                "SELECT actor_id,request,revision,element_id FROM shared_operations
                 WHERE project_id=?1 AND operation_id=?2",
                params![project.to_string(), request.operation_id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?;
        if let Some((actor, payload, revision, element)) = previous {
            if actor != authenticated_actor.to_string() || payload != serialized {
                return Err(CollaborationError::OperationIdReused);
            }
            let element =
                Uuid::parse_str(&element).map_err(|_| PersistenceError::InvalidUuid(element))?;
            return Ok(CommitReceipt {
                revision,
                element: ElementId(element),
            });
        }
        if current != request.expected_revision {
            return Err(CollaborationError::Conflict {
                expected: request.expected_revision,
                current,
            });
        }
        match &request.edit {
            SharedEdit::CreateBlock { name, .. }
            | SharedEdit::CreatePackage { name, .. }
            | SharedEdit::RenameElement { name, .. }
            | SharedEdit::CreateRequirement { name, .. }
            | SharedEdit::UpdateRequirement { name, .. }
            | SharedEdit::CreateTestCase { name, .. }
            | SharedEdit::CreateBddDiagram { name, .. }
            | SharedEdit::RenameBddDiagram { name, .. } => validate_name(name)?,
            SharedEdit::DeleteBddDiagram { .. }
            | SharedEdit::CreateRelationship { .. }
            | SharedEdit::DeleteRelationship { .. }
            | SharedEdit::PlaceBddElement { .. }
            | SharedEdit::UpdateBddNodeGeometry { .. }
            | SharedEdit::RemoveBddNode { .. }
            | SharedEdit::PresentBddRelationship { .. }
            | SharedEdit::RemoveBddEdge { .. }
            | SharedEdit::RouteBddDiagram { .. } => {}
        }

        let mut model = self.load_project(project)?;
        let mut semantic_changed = false;
        let element = match &request.edit {
            SharedEdit::CreateBlock { owner, name } => {
                semantic_changed = true;
                model.create_element(ElementKind::Block, name, *owner)?
            }
            SharedEdit::CreatePackage { owner, name } => {
                semantic_changed = true;
                model.create_element(ElementKind::Package, name, *owner)?
            }
            SharedEdit::RenameElement { element, name } => {
                semantic_changed = true;
                model.rename_element(*element, name)?;
                *element
            }
            SharedEdit::CreateRequirement {
                owner,
                name,
                requirement_id,
                text,
            } => {
                semantic_changed = true;
                model.create_requirement(name, requirement_id, text, *owner)?
            }
            SharedEdit::UpdateRequirement {
                element,
                name,
                requirement_id,
                text,
            } => {
                semantic_changed = true;
                model.update_requirement(*element, requirement_id, text)?;
                model.rename_element(*element, name)?;
                *element
            }
            SharedEdit::CreateTestCase { owner, name } => {
                semantic_changed = true;
                model.create_element(ElementKind::TestCase, name, *owner)?
            }
            SharedEdit::CreateRelationship {
                kind,
                source,
                target,
                owner,
            } => {
                semantic_changed = true;
                model.create_relationship(kind.model_kind(), *source, *target, Some(*owner))?;
                *source
            }
            SharedEdit::DeleteRelationship { relationship } => {
                let existing = model.relationship(*relationship)?;
                if !SharedRelationshipKind::supports(&existing.kind) {
                    return Err(CollaborationError::InvalidRelationship(
                        "relationship kind requires a specialized collaboration operation",
                    ));
                }
                let source = existing.source_id;
                model.relationships.remove(relationship);
                for mut diagram in Self::load_shared_bdd_diagrams_from(&tx, project)? {
                    let previous = diagram.edges.len();
                    diagram
                        .edges
                        .retain(|edge| edge.relationship != *relationship);
                    if diagram.edges.len() != previous {
                        route_shared_bdd(&mut diagram)?;
                        validate_shared_bdd(&model, &diagram)?;
                        Self::save_shared_bdd_diagram_to(&tx, project, &diagram)?;
                    }
                }
                semantic_changed = true;
                source
            }
            SharedEdit::CreateBddDiagram {
                diagram,
                owner,
                name,
            } => {
                if tx
                    .query_row(
                        "SELECT 1 FROM shared_bdd_diagrams WHERE project_id=?1 AND diagram_id=?2",
                        params![project.to_string(), diagram.to_string()],
                        |_| Ok(()),
                    )
                    .optional()?
                    .is_some()
                {
                    return Err(CollaborationError::InvalidDiagram(
                        "diagram identity is already in use",
                    ));
                }
                let created = SharedBddDiagram {
                    id: *diagram,
                    name: name.clone(),
                    owner: *owner,
                    nodes: Vec::new(),
                    edges: Vec::new(),
                };
                validate_shared_bdd(&model, &created)?;
                Self::save_shared_bdd_diagram_to(&tx, project, &created)?;
                *owner
            }
            SharedEdit::RenameBddDiagram { diagram, name } => {
                let mut existing = Self::load_shared_bdd_diagram_from(&tx, project, *diagram)?;
                existing.name = name.clone();
                validate_shared_bdd(&model, &existing)?;
                let owner = existing.owner;
                Self::save_shared_bdd_diagram_to(&tx, project, &existing)?;
                owner
            }
            SharedEdit::DeleteBddDiagram { diagram } => {
                let existing = Self::load_shared_bdd_diagram_from(&tx, project, *diagram)?;
                tx.execute(
                    "DELETE FROM shared_bdd_diagrams WHERE project_id=?1 AND diagram_id=?2",
                    params![project.to_string(), diagram.to_string()],
                )?;
                existing.owner
            }
            SharedEdit::PlaceBddElement {
                diagram,
                node,
                element,
            } => {
                if node.is_nil() {
                    return Err(CollaborationError::InvalidDiagram(
                        "node identity must not be nil",
                    ));
                }
                let semantic = model.element(*element)?;
                if !bdd_presentable(&semantic.kind) {
                    return Err(CollaborationError::InvalidDiagram(
                        "semantic element kind is not valid on a BDD",
                    ));
                }
                let mut existing = Self::load_shared_bdd_diagram_from(&tx, project, *diagram)?;
                if existing
                    .nodes
                    .iter()
                    .any(|presented| presented.id == *node || presented.element == *element)
                {
                    return Err(CollaborationError::InvalidDiagram(
                        "element or node is already presented on this BDD",
                    ));
                }
                let index = existing.nodes.len();
                let column = index % BDD_GRID_COLUMNS;
                let row = index / BDD_GRID_COLUMNS;
                let (width, height) = default_bdd_node_size(&semantic.kind);
                existing.nodes.push(SharedBddNode {
                    id: *node,
                    element: *element,
                    x: 72.0 + column as f64 * 240.0,
                    y: 90.0 + row as f64 * 160.0,
                    width,
                    height,
                });
                validate_shared_bdd(&model, &existing)?;
                Self::save_shared_bdd_diagram_to(&tx, project, &existing)?;
                *element
            }
            SharedEdit::UpdateBddNodeGeometry {
                diagram,
                node,
                x,
                y,
                width,
                height,
            } => {
                let mut existing = Self::load_shared_bdd_diagram_from(&tx, project, *diagram)?;
                let presented = existing
                    .nodes
                    .iter_mut()
                    .find(|presented| presented.id == *node)
                    .ok_or(CollaborationError::InvalidDiagram("BDD node was not found"))?;
                presented.x = *x;
                presented.y = *y;
                presented.width = *width;
                presented.height = *height;
                let element = presented.element;
                route_shared_bdd(&mut existing)?;
                validate_shared_bdd(&model, &existing)?;
                Self::save_shared_bdd_diagram_to(&tx, project, &existing)?;
                element
            }
            SharedEdit::RemoveBddNode { diagram, node } => {
                let mut existing = Self::load_shared_bdd_diagram_from(&tx, project, *diagram)?;
                let index = existing
                    .nodes
                    .iter()
                    .position(|presented| presented.id == *node)
                    .ok_or(CollaborationError::InvalidDiagram("BDD node was not found"))?;
                let element = existing.nodes.remove(index).element;
                existing
                    .edges
                    .retain(|edge| edge.source_node != *node && edge.target_node != *node);
                route_shared_bdd(&mut existing)?;
                validate_shared_bdd(&model, &existing)?;
                Self::save_shared_bdd_diagram_to(&tx, project, &existing)?;
                element
            }
            SharedEdit::PresentBddRelationship {
                diagram,
                edge,
                relationship,
            } => {
                if edge.is_nil() {
                    return Err(CollaborationError::InvalidDiagram(
                        "BDD edge identity must not be nil",
                    ));
                }
                let semantic = model.relationship(*relationship)?;
                if !SharedRelationshipKind::supports(&semantic.kind) {
                    return Err(CollaborationError::InvalidDiagram(
                        "relationship kind requires a specialized BDD presentation",
                    ));
                }
                let mut existing = Self::load_shared_bdd_diagram_from(&tx, project, *diagram)?;
                if existing.edges.iter().any(|presented| {
                    presented.id == *edge || presented.relationship == *relationship
                }) {
                    return Err(CollaborationError::InvalidDiagram(
                        "edge or relationship is already presented on this BDD",
                    ));
                }
                let source_node = existing
                    .nodes
                    .iter()
                    .find(|node| node.element == semantic.source_id)
                    .ok_or(CollaborationError::InvalidDiagram(
                        "relationship source must be presented on the selected BDD",
                    ))?
                    .id;
                let target_node = existing
                    .nodes
                    .iter()
                    .find(|node| node.element == semantic.target_id)
                    .ok_or(CollaborationError::InvalidDiagram(
                        "relationship target must be presented on the selected BDD",
                    ))?
                    .id;
                existing.edges.push(SharedBddEdge {
                    id: *edge,
                    relationship: *relationship,
                    source_node,
                    target_node,
                    points: Vec::new(),
                    label_anchor: GeometryPoint::default(),
                });
                route_shared_bdd(&mut existing)?;
                validate_shared_bdd(&model, &existing)?;
                Self::save_shared_bdd_diagram_to(&tx, project, &existing)?;
                semantic.source_id
            }
            SharedEdit::RemoveBddEdge { diagram, edge } => {
                let mut existing = Self::load_shared_bdd_diagram_from(&tx, project, *diagram)?;
                let index = existing
                    .edges
                    .iter()
                    .position(|presented| presented.id == *edge)
                    .ok_or(CollaborationError::InvalidDiagram("BDD edge was not found"))?;
                let relationship = existing.edges.remove(index).relationship;
                let element = model.relationship(relationship)?.source_id;
                route_shared_bdd(&mut existing)?;
                validate_shared_bdd(&model, &existing)?;
                Self::save_shared_bdd_diagram_to(&tx, project, &existing)?;
                element
            }
            SharedEdit::RouteBddDiagram { diagram } => {
                let mut existing = Self::load_shared_bdd_diagram_from(&tx, project, *diagram)?;
                route_shared_bdd(&mut existing)?;
                validate_shared_bdd(&model, &existing)?;
                let owner = existing.owner;
                Self::save_shared_bdd_diagram_to(&tx, project, &existing)?;
                owner
            }
        };
        model.validate()?;
        if semantic_changed {
            Self::write_project(&tx, &model)?;
        }
        let revision = current
            .checked_add(1)
            .ok_or(CollaborationError::RevisionExhausted)?;
        tx.execute(
            "UPDATE shared_projects SET revision=?2 WHERE project_id=?1",
            params![project.to_string(), revision],
        )?;
        tx.execute(
            "INSERT INTO shared_operations
             (project_id,operation_id,actor_id,request,revision,element_id)
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                project.to_string(),
                request.operation_id.to_string(),
                authenticated_actor.to_string(),
                serialized,
                revision,
                element.to_string(),
            ],
        )?;
        tx.commit()?;
        Ok(CommitReceipt { revision, element })
    }
}
