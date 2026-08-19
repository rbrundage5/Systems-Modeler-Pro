use crate::{PersistenceError, ProjectDatabase};
use systems_modeler_core::{ActivityRepository, Project};

pub const ACTIVITY_METADATA_KEY: &str = "activity-repository";

pub fn save_activity_repository(
    database: &mut ProjectDatabase,
    project: &Project,
    repository: &ActivityRepository,
) -> Result<(), PersistenceError> {
    repository
        .validate(project)
        .map_err(|error| PersistenceError::InvalidActivity(error.to_string()))?;
    let payload = serde_json::to_string(repository)?;
    database.save_metadata(project.id, ACTIVITY_METADATA_KEY, &payload)
}

pub fn load_activity_repository(
    database: &ProjectDatabase,
    project: &Project,
) -> Result<ActivityRepository, PersistenceError> {
    let repository = match database.load_metadata(project.id, ACTIVITY_METADATA_KEY)? {
        Some(payload) => serde_json::from_str::<ActivityRepository>(&payload)?,
        None => ActivityRepository::default(),
    };
    repository
        .validate(project)
        .map_err(|error| PersistenceError::InvalidActivity(error.to_string()))?;
    Ok(repository)
}
