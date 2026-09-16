use std::{collections::HashMap, time::{Duration, Instant}};
use systems_modeler_core::ProjectId;
use systems_modeler_persistence::collaboration::{PresenceParticipant, PresenceRequest, ProjectPresence};
use uuid::Uuid;

const TTL: Duration = Duration::from_secs(45);
const MAX_SESSIONS: usize = 1024;
const MAX_ACTOR_SESSIONS: usize = 8;

struct ActiveSession {
    participant: PresenceParticipant,
    last_seen: Instant,
}

#[derive(Default)]
pub(crate) struct PresenceRegistry {
    sessions: HashMap<(ProjectId, Uuid, Uuid), ActiveSession>,
}

impl PresenceRegistry {
    fn expire(&mut self, now: Instant) {
        self.sessions.retain(|_, value| now.saturating_duration_since(value.last_seen) < TTL);
    }

    pub(crate) fn heartbeat(&mut self, project: ProjectId, actor: Uuid, role: &str, request: PresenceRequest, now: Instant) -> Result<ProjectPresence, u16> {
        let name = request.name.trim();
        if request.session.is_nil() || name.is_empty() || name.len() > 256 || name.chars().count() > 64 || name.chars().any(char::is_control) {
            return Err(400);
        }
        self.expire(now);
        let key = (project, actor, request.session);
        if !self.sessions.contains_key(&key) && (self.sessions.len() >= MAX_SESSIONS || self.sessions.keys().filter(|(_, owner, _)| *owner == actor).count() >= MAX_ACTOR_SESSIONS) {
            return Err(429);
        }
        self.sessions.insert(key, ActiveSession {
            participant: PresenceParticipant { actor, session: request.session, name: name.into(), role: role.into() },
            last_seen: now,
        });
        Ok(self.snapshot(project, now))
    }

    pub(crate) fn leave(&mut self, project: ProjectId, actor: Uuid, session: Uuid, now: Instant) -> ProjectPresence {
        self.sessions.remove(&(project, actor, session));
        self.snapshot(project, now)
    }

    pub(crate) fn snapshot(&mut self, project: ProjectId, now: Instant) -> ProjectPresence {
        self.expire(now);
        let mut participants: Vec<_> = self.sessions.iter().filter(|((id, _, _), _)| *id == project).map(|(_, value)| value.participant.clone()).collect();
        participants.sort_by_key(|value| (value.actor, value.session));
        ProjectPresence { participants }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use systems_modeler_core::Project;

    #[test]
    fn presence_is_scoped_expires_and_cannot_be_removed_by_another_actor() {
        let project = Project::new("Shared").id;
        let actor = Uuid::new_v4();
        let session = Uuid::new_v4();
        let now = Instant::now();
        let mut registry = PresenceRegistry::default();
        assert_eq!(registry.heartbeat(project, actor, "editor", PresenceRequest { session, name: "Engineer".into() }, now).unwrap().participants.len(), 1);
        assert!(registry.snapshot(Project::new("Other").id, now).participants.is_empty());
        assert_eq!(registry.leave(project, Uuid::new_v4(), session, now).participants.len(), 1);
        assert_eq!(registry.snapshot(project, now + TTL - Duration::from_millis(1)).participants.len(), 1);
        assert!(registry.snapshot(project, now + TTL).participants.is_empty());
    }

    #[test]
    fn presence_is_bounded_and_existing_sessions_can_renew_at_capacity() {
        let project = Project::new("Shared").id;
        let actor = Uuid::new_v4();
        let now = Instant::now();
        let mut registry = PresenceRegistry::default();
        let mut session = Uuid::nil();
        for _ in 0..MAX_ACTOR_SESSIONS {
            session = Uuid::new_v4();
            assert!(registry.heartbeat(project, actor, "viewer", PresenceRequest { session, name: "Viewer".into() }, now).is_ok());
        }
        assert!(registry.heartbeat(project, actor, "viewer", PresenceRequest { session: Uuid::new_v4(), name: "Too many".into() }, now).is_err());
        assert!(registry.heartbeat(project, actor, "viewer", PresenceRequest { session, name: "Still active".into() }, now).is_ok());
        assert_eq!(registry.leave(project, actor, session, now).participants.len(), MAX_ACTOR_SESSIONS - 1);
        assert!(registry.heartbeat(project, actor, "viewer", PresenceRequest { session, name: "New\nline".into() }, now).is_err());
    }
}
