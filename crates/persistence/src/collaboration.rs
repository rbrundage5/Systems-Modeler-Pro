//! Server-side persistence foundation. Caller identities must come from a trusted
//! authentication layer, never from an unverified request field.
use crate::{PersistenceError, ProjectDatabase};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use systems_modeler_core::{ElementId, ElementKind, ModelError, ProjectId};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SharedEdit {
    CreateBlock { owner: ElementId, name: String },
    CreatePackage { owner: ElementId, name: String },
    RenameElement { element: ElementId, name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    #[error("project revision exhausted")]
    RevisionExhausted,
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

    /// Returns an atomic reconnect snapshot and revision for an authorized member.
    pub fn shared_snapshot(
        &self,
        project: ProjectId,
        actor: Uuid,
    ) -> Result<(systems_modeler_core::Project, i64), CollaborationError> {
        let tx = self.connection.unchecked_transaction()?;
        let revision = self.member_revision(project, actor, false)?;
        let snapshot = self.load_project(project)?;
        tx.commit()?;
        Ok((snapshot, revision))
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

    /// Commits a semantic edit, model state, revision and retry receipt together.
    /// Uses a database write lock so separate server connections cannot both
    /// accept the same base revision. No in-memory model is published on failure.
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
        let name = match &request.edit {
            SharedEdit::CreateBlock { name, .. }
            | SharedEdit::CreatePackage { name, .. }
            | SharedEdit::RenameElement { name, .. } => name,
        };
        if name.trim().is_empty() || name.len() > 1024 {
            return Err(CollaborationError::InvalidName);
        }
        let mut model = self.load_project(project)?;
        let element = match &request.edit {
            SharedEdit::CreateBlock { owner, name } => {
                model.create_element(ElementKind::Block, name, *owner)?
            }
            SharedEdit::CreatePackage { owner, name } => {
                model.create_element(ElementKind::Package, name, *owner)?
            }
            SharedEdit::RenameElement { element, name } => {
                model.rename_element(*element, name)?;
                *element
            }
        };
        model.validate()?;
        let revision = current
            .checked_add(1)
            .ok_or(CollaborationError::RevisionExhausted)?;
        Self::write_project(&tx, &model)?;
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
