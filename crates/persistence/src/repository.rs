use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use systems_modeler_core::{Element, ElementId, ProfileRepository, Project, ProjectId, Relationship};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PersistenceError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("invalid UUID stored in database: {0}")]
    InvalidUuid(String),
    #[error("project not found: {0}")]
    ProjectNotFound(ProjectId),
    #[error("project database contains no project")]
    NoProject,
}

pub struct ProjectDatabase {
    connection: Connection,
}

const PROFILE_METADATA_KEY: &str = "profile-repository-v1";

impl ProjectDatabase {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, PersistenceError> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        let database = Self { connection };
        database.migrate()?;
        Ok(database)
    }

    pub fn open_in_memory() -> Result<Self, PersistenceError> {
        let connection = Connection::open_in_memory()?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let database = Self { connection };
        database.migrate()?;
        Ok(database)
    }

    fn migrate(&self) -> Result<(), PersistenceError> {
        self.connection.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                root_id TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS elements (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                external_id TEXT NOT NULL,
                owner_id TEXT,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                payload TEXT NOT NULL,
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_elements_project_owner ON elements(project_id, owner_id);
            CREATE INDEX IF NOT EXISTS idx_elements_project_kind ON elements(project_id, kind);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_elements_project_external ON elements(project_id, external_id);
            CREATE TABLE IF NOT EXISTS relationships (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                external_id TEXT NOT NULL,
                owner_id TEXT,
                source_id TEXT NOT NULL,
                target_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                payload TEXT NOT NULL,
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_relationships_project_source ON relationships(project_id, source_id);
            CREATE INDEX IF NOT EXISTS idx_relationships_project_target ON relationships(project_id, target_id);
            CREATE INDEX IF NOT EXISTS idx_relationships_project_owner ON relationships(project_id, owner_id);
            CREATE UNIQUE INDEX IF NOT EXISTS idx_relationships_project_external ON relationships(project_id, external_id);
            CREATE TABLE IF NOT EXISTS project_metadata (
                project_id TEXT NOT NULL,
                key TEXT NOT NULL,
                payload TEXT NOT NULL,
                PRIMARY KEY(project_id, key),
                FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            INSERT OR IGNORE INTO schema_migrations(version) VALUES(1);
            INSERT OR IGNORE INTO schema_migrations(version) VALUES(2);
            ",
        )?;
        Ok(())
    }

    pub fn save_project(&mut self, project: &Project) -> Result<(), PersistenceError> {
        let tx = self.connection.transaction()?;
        tx.execute(
            "INSERT INTO projects(id,name,root_id,updated_at) VALUES(?1,?2,?3,CURRENT_TIMESTAMP)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name,root_id=excluded.root_id,updated_at=CURRENT_TIMESTAMP",
            params![project.id.to_string(), project.name, project.root_id.to_string()],
        )?;
        tx.execute(
            "DELETE FROM relationships WHERE project_id=?1",
            params![project.id.to_string()],
        )?;
        tx.execute(
            "DELETE FROM elements WHERE project_id=?1",
            params![project.id.to_string()],
        )?;

        {
            let mut statement = tx.prepare(
                "INSERT INTO elements(id,project_id,external_id,owner_id,kind,name,payload) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            )?;
            for element in project.elements.values() {
                statement.execute(params![
                    element.id.to_string(),
                    project.id.to_string(),
                    element.external_id,
                    element.owner_id.map(|id| id.to_string()),
                    format!("{:?}", element.kind),
                    element.name,
                    serde_json::to_string(element)?,
                ])?;
            }
        }
        {
            let mut statement = tx.prepare(
                "INSERT INTO relationships(id,project_id,external_id,owner_id,source_id,target_id,kind,payload) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
            )?;
            for relationship in project.relationships.values() {
                statement.execute(params![
                    relationship.id.to_string(),
                    project.id.to_string(),
                    relationship.external_id,
                    relationship.owner_id.map(|id| id.to_string()),
                    relationship.source_id.to_string(),
                    relationship.target_id.to_string(),
                    format!("{:?}", relationship.kind),
                    serde_json::to_string(relationship)?,
                ])?;
            }
        }
        tx.execute(
            "INSERT INTO project_metadata(project_id,key,payload) VALUES(?1,?2,?3)
             ON CONFLICT(project_id,key) DO UPDATE SET payload=excluded.payload",
            params![
                project.id.to_string(),
                PROFILE_METADATA_KEY,
                serde_json::to_string(&project.profiles)?,
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn save_metadata(
        &mut self,
        project_id: ProjectId,
        key: &str,
        payload: &str,
    ) -> Result<(), PersistenceError> {
        self.connection.execute(
            "INSERT INTO project_metadata(project_id,key,payload) VALUES(?1,?2,?3)
             ON CONFLICT(project_id,key) DO UPDATE SET payload=excluded.payload",
            params![project_id.to_string(), key, payload],
        )?;
        Ok(())
    }

    pub fn load_metadata(
        &self,
        project_id: ProjectId,
        key: &str,
    ) -> Result<Option<String>, PersistenceError> {
        self.connection
            .query_row(
                "SELECT payload FROM project_metadata WHERE project_id=?1 AND key=?2",
                params![project_id.to_string(), key],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(PersistenceError::from)
    }

    pub fn load_first_project(&self) -> Result<Project, PersistenceError> {
        let id = self
            .connection
            .query_row(
                "SELECT id FROM projects ORDER BY updated_at DESC, id LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(PersistenceError::NoProject)?;
        self.load_project(ProjectId(parse_uuid(&id)?))
    }

    pub fn load_project(&self, id: ProjectId) -> Result<Project, PersistenceError> {
        let project_row = self
            .connection
            .query_row(
                "SELECT name,root_id FROM projects WHERE id=?1",
                params![id.to_string()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        let (name, root_id) = project_row.ok_or(PersistenceError::ProjectNotFound(id))?;
        let root_id = ElementId(parse_uuid(&root_id)?);

        let mut elements = std::collections::HashMap::new();
        let mut statement = self
            .connection
            .prepare("SELECT payload FROM elements WHERE project_id=?1")?;
        let rows = statement.query_map(params![id.to_string()], |row| row.get::<_, String>(0))?;
        for row in rows {
            let element: Element = serde_json::from_str(&row?)?;
            elements.insert(element.id, element);
        }

        let mut relationships = std::collections::HashMap::new();
        let mut statement = self
            .connection
            .prepare("SELECT payload FROM relationships WHERE project_id=?1")?;
        let rows = statement.query_map(params![id.to_string()], |row| row.get::<_, String>(0))?;
        for row in rows {
            let relationship: Relationship = serde_json::from_str(&row?)?;
            relationships.insert(relationship.id, relationship);
        }

        let profiles = self
            .load_metadata(id, PROFILE_METADATA_KEY)?
            .map(|payload| serde_json::from_str::<ProfileRepository>(&payload))
            .transpose()?
            .unwrap_or_default();

        Ok(Project {
            id,
            name,
            root_id,
            elements,
            relationships,
            profiles,
        })
    }
}

fn parse_uuid(value: &str) -> Result<Uuid, PersistenceError> {
    Uuid::parse_str(value).map_err(|_| PersistenceError::InvalidUuid(value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use systems_modeler_core::{ElementKind, RelationshipKind};

    #[test]
    fn project_round_trips_through_sqlite() {
        let mut project = Project::new("Vehicle");
        let package = project
            .create_element(ElementKind::Package, "Structure", project.root_id)
            .unwrap();
        let vehicle = project
            .create_element(ElementKind::Block, "Vehicle", package)
            .unwrap();
        let powertrain = project
            .create_element(ElementKind::Block, "Powertrain", package)
            .unwrap();
        project
            .create_relationship(
                RelationshipKind::Composition,
                vehicle,
                powertrain,
                Some(package),
            )
            .unwrap();

        let mut db = ProjectDatabase::open_in_memory().unwrap();
        db.save_project(&project).unwrap();
        let restored = db.load_project(project.id).unwrap();

        assert_eq!(restored.id, project.id);
        assert_eq!(restored.elements.len(), project.elements.len());
        assert_eq!(restored.relationships.len(), project.relationships.len());
        assert_eq!(restored.children(package).count(), 2);
    }

    #[test]
    fn project_metadata_round_trips_with_first_project_lookup() {
        let project = Project::new("Vehicle");
        let mut db = ProjectDatabase::open_in_memory().unwrap();
        db.save_project(&project).unwrap();
        db.save_metadata(project.id, "bdd-diagrams", "[{}]")
            .unwrap();

        let restored = db.load_first_project().unwrap();
        assert_eq!(restored.id, project.id);
        assert_eq!(
            db.load_metadata(project.id, "bdd-diagrams").unwrap(),
            Some("[{}]".to_owned())
        );
    }
}
