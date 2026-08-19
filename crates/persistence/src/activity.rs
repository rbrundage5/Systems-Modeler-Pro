use crate::{PersistenceError, ProjectDatabase};
use systems_modeler_core::{ActivityRepository, Project};
use thiserror::Error;

pub const ACTIVITY_METADATA_KEY: &str = "activity-repository";

#[derive(Debug, Error)]
pub enum ActivityPersistenceError {
    #[error(transparent)]
    Persistence(#[from] PersistenceError),
    #[error("activity repository validation failed: {0}")]
    Validation(String),
}

pub fn save_activity_repository(
    database: &mut ProjectDatabase,
    project: &Project,
    repository: &ActivityRepository,
) -> Result<(), ActivityPersistenceError> {
    repository
        .validate(project)
        .map_err(|error| ActivityPersistenceError::Validation(error.to_string()))?;
    let payload = serde_json::to_string(repository)?;
    database.save_metadata(project.id, ACTIVITY_METADATA_KEY, &payload)?;
    Ok(())
}

pub fn load_activity_repository(
    database: &ProjectDatabase,
    project: &Project,
) -> Result<ActivityRepository, ActivityPersistenceError> {
    let repository = match database.load_metadata(project.id, ACTIVITY_METADATA_KEY)? {
        Some(payload) => serde_json::from_str::<ActivityRepository>(&payload)?,
        None => ActivityRepository::default(),
    };
    repository
        .validate(project)
        .map_err(|error| ActivityPersistenceError::Validation(error.to_string()))?;
    Ok(repository)
}
