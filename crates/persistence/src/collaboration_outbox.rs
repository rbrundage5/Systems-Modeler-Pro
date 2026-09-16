//! Client-side recovery journal, separate from native/shared project databases.
//! Credentials are never stored. Reservations are scoped by authenticated actor
//! and canonical server origin, and cannot replace a different pending operation.
use crate::collaboration::EditRequest;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use std::{path::Path, sync::Mutex, time::Duration};
use systems_modeler_core::ProjectId;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum OutboxError {
    #[error("recovery storage is unavailable")]
    Storage(#[from] rusqlite::Error),
    #[error("recovery record is invalid")]
    Invalid(#[from] serde_json::Error),
    #[error("another unresolved edit is already recorded for this account and server")]
    Occupied,
    #[error("recovery record changed; it was not removed")]
    Changed,
    #[error("recovery storage lock is unavailable")]
    Unavailable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PendingSharedEdit {
    pub project: ProjectId,
    pub request: EditRequest,
}

pub struct CollaborationOutbox(Mutex<Connection>);

impl CollaborationOutbox {
    pub fn open(path: &Path) -> Result<Self, OutboxError> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(2))?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS pending_shared_edits (
                server TEXT NOT NULL,
                actor TEXT NOT NULL,
                project TEXT NOT NULL,
                request_json TEXT NOT NULL,
                PRIMARY KEY (server, actor)
            );",
        )?;
        Ok(Self(Mutex::new(connection)))
    }

    pub fn load(
        &self,
        server: &str,
        actor: Uuid,
    ) -> Result<Option<PendingSharedEdit>, OutboxError> {
        let connection = self.0.lock().map_err(|_| OutboxError::Unavailable)?;
        let row: Option<(String, String)> = connection.query_row(
            "SELECT project, request_json FROM pending_shared_edits WHERE server=?1 AND actor=?2",
            params![server, actor.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?;
        row.map(|(project, request)| {
            Ok(PendingSharedEdit {
                project: serde_json::from_str(&project)?,
                request: serde_json::from_str(&request)?,
            })
        }).transpose()
    }

    pub fn reserve(
        &mut self,
        server: &str,
        actor: Uuid,
        pending: &PendingSharedEdit,
    ) -> Result<(), OutboxError> {
        let project = serde_json::to_string(&pending.project)?;
        let request = serde_json::to_string(&pending.request)?;
        let mut connection = self.0.lock().map_err(|_| OutboxError::Unavailable)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<(String, String)> = transaction.query_row(
            "SELECT project, request_json FROM pending_shared_edits WHERE server=?1 AND actor=?2",
            params![server, actor.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?;
        if let Some(existing) = existing {
            if existing != (project, request) {
                return Err(OutboxError::Occupied);
            }
        } else {
            transaction.execute(
                "INSERT INTO pending_shared_edits(server, actor, project, request_json) VALUES (?1, ?2, ?3, ?4)",
                params![server, actor.to_string(), project, request],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn complete(
        &mut self,
        server: &str,
        actor: Uuid,
        pending: &PendingSharedEdit,
    ) -> Result<(), OutboxError> {
        let project = serde_json::to_string(&pending.project)?;
        let request = serde_json::to_string(&pending.request)?;
        let mut connection = self.0.lock().map_err(|_| OutboxError::Unavailable)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "DELETE FROM pending_shared_edits WHERE server=?1 AND actor=?2 AND project=?3 AND request_json=?4",
            params![server, actor.to_string(), project, request],
        )?;
        // Another process may already have acknowledged this exact operation.
        // A different reservation must never be removed by a delayed response.
        let remains: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM pending_shared_edits WHERE server=?1 AND actor=?2)",
            params![server, actor.to_string()],
            |row| row.get(0),
        )?;
        if remains {
            return Err(OutboxError::Changed);
        }
        transaction.commit()?;
        Ok(())
    }
}
