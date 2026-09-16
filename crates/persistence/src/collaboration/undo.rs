//! Typed inverse deltas. Preconditions are checked against current records so
//! reversal cannot overwrite a later editor's affected objects.
use super::{CollaborationError, SharedBddDiagram};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};
use systems_modeler_core::{DiagramId, Element, ElementId, Project, Relationship, RelationshipId};

#[derive(Serialize, Deserialize)]
struct Change<K, T> {
    id: K,
    before: Option<T>,
    after: Option<T>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct UndoDelta {
    elements: Vec<Change<ElementId, Element>>,
    relationships: Vec<Change<RelationshipId, Relationship>>,
    diagrams: Vec<Change<DiagramId, SharedBddDiagram>>,
}

fn changes<K: Copy + Eq + Hash, T: Clone + Serialize>(
    before: &HashMap<K, T>,
    after: &HashMap<K, T>,
) -> Result<Vec<Change<K, T>>, CollaborationError> {
    let keys: HashSet<_> = before.keys().chain(after.keys()).copied().collect();
    let mut result = Vec::new();
    for id in keys {
        if serde_json::to_value(before.get(&id))? != serde_json::to_value(after.get(&id))? {
            result.push(Change {
                id,
                before: before.get(&id).cloned(),
                after: after.get(&id).cloned(),
            });
        }
    }
    Ok(result)
}

fn reverse<K: Copy + Eq + Hash, T: Clone + Serialize>(
    records: &mut HashMap<K, T>,
    changes: &[Change<K, T>],
) -> Result<(), CollaborationError> {
    for change in changes {
        if serde_json::to_value(records.get(&change.id))? != serde_json::to_value(&change.after)? {
            return Err(CollaborationError::UndoConflict);
        }
    }
    for change in changes {
        if let Some(before) = &change.before {
            records.insert(change.id, before.clone());
        } else {
            records.remove(&change.id);
        }
    }
    Ok(())
}

impl UndoDelta {
    pub(super) fn capture(
        before: &Project,
        after: &Project,
        before_diagrams: &[SharedBddDiagram],
        after_diagrams: &[SharedBddDiagram],
    ) -> Result<Self, CollaborationError> {
        // Shared commands currently mutate only these typed records. Any future
        // metadata/profile operation must explicitly extend inverse support.
        if before.id != after.id
            || before.root_id != after.root_id
            || before.name != after.name
            || serde_json::to_value(&before.profiles)? != serde_json::to_value(&after.profiles)?
        {
            return Err(CollaborationError::UndoUnavailable);
        }
        Ok(Self {
            elements: changes(&before.elements, &after.elements)?,
            relationships: changes(&before.relationships, &after.relationships)?,
            diagrams: changes(
                &before_diagrams
                    .iter()
                    .map(|value| (value.id, value.clone()))
                    .collect(),
                &after_diagrams
                    .iter()
                    .map(|value| (value.id, value.clone()))
                    .collect(),
            )?,
        })
    }

    pub(super) fn is_empty(&self) -> bool {
        self.elements.is_empty() && self.relationships.is_empty() && self.diagrams.is_empty()
    }

    pub(super) fn apply(
        &self,
        model: &mut Project,
        diagrams: Vec<SharedBddDiagram>,
    ) -> Result<Vec<SharedBddDiagram>, CollaborationError> {
        let mut diagram_map: HashMap<_, _> = diagrams
            .into_iter()
            .map(|diagram| (diagram.id, diagram))
            .collect();
        reverse(&mut model.elements, &self.elements)?;
        reverse(&mut model.relationships, &self.relationships)?;
        reverse(&mut diagram_map, &self.diagrams)?;
        model
            .validate()
            .map_err(|_| CollaborationError::UndoConflict)?;
        let result: Vec<_> = diagram_map.into_values().collect();
        for diagram in &result {
            super::validate_shared_bdd(model, diagram)
                .map_err(|_| CollaborationError::UndoConflict)?;
        }
        Ok(result)
    }
}
