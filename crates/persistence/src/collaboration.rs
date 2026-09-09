//! Server-side persistence foundation. Caller identities must come from a trusted
//! authentication layer, never from an unverified request field.
use crate::{PersistenceError, ProjectDatabase};
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use systems_modeler_core::{DiagramId, ElementId, ElementKind, ModelError, ProjectId};
use thiserror::Error;
use uuid::Uuid;

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
pub struct SharedBddDiagram {
    pub id: DiagramId,
    pub name: String,
    pub owner: ElementId,
    pub nodes: Vec<SharedBddNode>,
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
            | SharedEdit::CreateBddDiagram { name, .. }
            | SharedEdit::RenameBddDiagram { name, .. } => validate_name(name)?,
            SharedEdit::DeleteBddDiagram { .. }
            | SharedEdit::PlaceBddElement { .. }
            | SharedEdit::UpdateBddNodeGeometry { .. }
            | SharedEdit::RemoveBddNode { .. } => {}
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
                validate_shared_bdd(&model, &existing)?;
                Self::save_shared_bdd_diagram_to(&tx, project, &existing)?;
                element
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
