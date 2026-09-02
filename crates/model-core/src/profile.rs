use crate::{ElementId, ElementKind, Project, RelationshipId, RelationshipKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

macro_rules! profile_id_type {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

profile_id_type!(ProfileId);
profile_id_type!(StereotypeId);
profile_id_type!(TagDefinitionId);
profile_id_type!(ProfileApplicationId);
profile_id_type!(StereotypeApplicationId);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticTarget {
    Element(ElementId),
    Relationship(RelationshipId),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StereotypeTargetKind {
    Element(ElementKind),
    Relationship(RelationshipKind),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TagValueType {
    String,
    Boolean,
    Integer,
    Real,
    Enumeration { literals: Vec<String> },
    SemanticReference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
pub enum TagValue {
    String(String),
    Boolean(bool),
    Integer(i64),
    Real(f64),
    Enumeration(String),
    SemanticReference(SemanticTarget),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileDefinition {
    pub id: ProfileId,
    pub external_id: String,
    pub name: String,
    pub uri: Option<String>,
    #[serde(default)]
    pub documentation: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StereotypeDefinition {
    pub id: StereotypeId,
    pub external_id: String,
    pub profile_id: ProfileId,
    pub name: String,
    #[serde(default)]
    pub documentation: String,
    pub extends: Vec<StereotypeTargetKind>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TagDefinition {
    pub id: TagDefinitionId,
    pub external_id: String,
    pub stereotype_id: StereotypeId,
    pub name: String,
    pub value_type: TagValueType,
    pub lower: u32,
    pub upper: Option<u32>,
    pub default: Option<TagValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileApplication {
    pub id: ProfileApplicationId,
    pub external_id: String,
    pub profile_id: ProfileId,
    pub scope_id: ElementId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StereotypeApplication {
    pub id: StereotypeApplicationId,
    pub external_id: String,
    pub stereotype_id: StereotypeId,
    pub target: SemanticTarget,
    #[serde(default)]
    pub tagged_values: BTreeMap<TagDefinitionId, Vec<TagValue>>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProfileRepository {
    #[serde(default)]
    pub profiles: BTreeMap<ProfileId, ProfileDefinition>,
    #[serde(default)]
    pub stereotypes: BTreeMap<StereotypeId, StereotypeDefinition>,
    #[serde(default)]
    pub tag_definitions: BTreeMap<TagDefinitionId, TagDefinition>,
    #[serde(default)]
    pub profile_applications: BTreeMap<ProfileApplicationId, ProfileApplication>,
    #[serde(default)]
    pub stereotype_applications: BTreeMap<StereotypeApplicationId, StereotypeApplication>,
    /// Producer-specific XML that is preserved for loss-minimized XMI re-export but is never
    /// interpreted as native model semantics.
    #[serde(default)]
    pub interchange_extensions: BTreeMap<String, String>,
}

impl ProfileRepository {
    pub fn validate(&self, project: &Project) -> Result<(), String> {
        let mut external_ids = HashSet::new();
        for profile in self.profiles.values() {
            require_identity(&profile.external_id, &profile.name, &mut external_ids)?;
        }
        for stereotype in self.stereotypes.values() {
            require_identity(&stereotype.external_id, &stereotype.name, &mut external_ids)?;
            if !self.profiles.contains_key(&stereotype.profile_id) {
                return Err(format!(
                    "stereotype '{}' references missing profile {}",
                    stereotype.name, stereotype.profile_id
                ));
            }
            if stereotype.extends.is_empty() {
                return Err(format!(
                    "stereotype '{}' must declare at least one applicable semantic kind",
                    stereotype.name
                ));
            }
        }
        for definition in self.tag_definitions.values() {
            require_identity(&definition.external_id, &definition.name, &mut external_ids)?;
            if !self.stereotypes.contains_key(&definition.stereotype_id) {
                return Err(format!(
                    "tag '{}' references missing stereotype {}",
                    definition.name, definition.stereotype_id
                ));
            }
            if definition.upper.is_some_and(|upper| definition.lower > upper) {
                return Err(format!("tag '{}' has invalid multiplicity", definition.name));
            }
            if let Some(default) = &definition.default {
                validate_tag_value(project, definition, default)?;
            }
        }
        for application in self.profile_applications.values() {
            require_external_id(&application.external_id, &mut external_ids)?;
            if !self.profiles.contains_key(&application.profile_id) {
                return Err(format!(
                    "profile application {} references missing profile {}",
                    application.external_id, application.profile_id
                ));
            }
            let scope = project
                .elements
                .get(&application.scope_id)
                .ok_or_else(|| format!("profile application scope not found: {}", application.scope_id))?;
            if !scope.is_namespace() {
                return Err(format!(
                    "profile application scope '{}' must be a Model, Package, or ModelLibrary",
                    scope.name
                ));
            }
        }
        for application in self.stereotype_applications.values() {
            require_external_id(&application.external_id, &mut external_ids)?;
            self.validate_stereotype_application(project, application)?;
        }
        Ok(())
    }

    pub fn validate_stereotype_application(
        &self,
        project: &Project,
        application: &StereotypeApplication,
    ) -> Result<(), String> {
        let stereotype = self
            .stereotypes
            .get(&application.stereotype_id)
            .ok_or_else(|| format!("stereotype not found: {}", application.stereotype_id))?;
        let actual_kind = match application.target {
            SemanticTarget::Element(id) => StereotypeTargetKind::Element(
                project
                    .elements
                    .get(&id)
                    .ok_or_else(|| format!("stereotype target element not found: {id}"))?
                    .kind
                    .clone(),
            ),
            SemanticTarget::Relationship(id) => StereotypeTargetKind::Relationship(
                project
                    .relationships
                    .get(&id)
                    .ok_or_else(|| format!("stereotype target relationship not found: {id}"))?
                    .kind
                    .clone(),
            ),
        };
        if !stereotype.extends.contains(&actual_kind) {
            return Err(format!(
                "stereotype '{}' cannot extend {actual_kind:?}",
                stereotype.name
            ));
        }
        if !self.profile_is_applied_to_target(project, stereotype.profile_id, &application.target) {
            return Err(format!(
                "profile for stereotype '{}' is not applied to the target scope",
                stereotype.name
            ));
        }
        for (definition_id, values) in &application.tagged_values {
            let definition = self
                .tag_definitions
                .get(definition_id)
                .ok_or_else(|| format!("tag definition not found: {definition_id}"))?;
            if definition.stereotype_id != stereotype.id {
                return Err(format!(
                    "tag '{}' does not belong to stereotype '{}'",
                    definition.name, stereotype.name
                ));
            }
            validate_value_count(definition, values.len())?;
            for value in values {
                validate_tag_value(project, definition, value)?;
            }
        }
        for definition in self
            .tag_definitions
            .values()
            .filter(|definition| definition.stereotype_id == stereotype.id)
        {
            let count = application
                .tagged_values
                .get(&definition.id)
                .map_or(usize::from(definition.default.is_some()), Vec::len);
            validate_value_count(definition, count)?;
        }
        Ok(())
    }

    fn profile_is_applied_to_target(
        &self,
        project: &Project,
        profile_id: ProfileId,
        target: &SemanticTarget,
    ) -> bool {
        let owner = match *target {
            SemanticTarget::Element(id) => Some(id),
            SemanticTarget::Relationship(id) => project
                .relationships
                .get(&id)
                .and_then(|relationship| relationship.owner_id)
                .or_else(|| project.relationships.get(&id).map(|relationship| relationship.source_id)),
        };
        self.profile_applications.values().any(|application| {
            application.profile_id == profile_id
                && owner.is_some_and(|target_id| scope_contains(project, application.scope_id, target_id))
        })
    }
}

impl Project {
    pub fn create_profile(
        &mut self,
        external_id: impl Into<String>,
        name: impl Into<String>,
        uri: Option<String>,
    ) -> Result<ProfileId, String> {
        let id = ProfileId::new();
        let mut candidate = self.profiles.clone();
        candidate.profiles.insert(
            id,
            ProfileDefinition {
                id,
                external_id: external_id.into(),
                name: name.into(),
                uri,
                documentation: String::new(),
            },
        );
        candidate.validate(self)?;
        self.profiles = candidate;
        Ok(id)
    }

    pub fn create_stereotype(
        &mut self,
        profile_id: ProfileId,
        external_id: impl Into<String>,
        name: impl Into<String>,
        extends: Vec<StereotypeTargetKind>,
    ) -> Result<StereotypeId, String> {
        let id = StereotypeId::new();
        let mut candidate = self.profiles.clone();
        candidate.stereotypes.insert(
            id,
            StereotypeDefinition {
                id,
                external_id: external_id.into(),
                profile_id,
                name: name.into(),
                documentation: String::new(),
                extends,
            },
        );
        candidate.validate(self)?;
        self.profiles = candidate;
        Ok(id)
    }

    pub fn create_tag_definition(
        &mut self,
        stereotype_id: StereotypeId,
        external_id: impl Into<String>,
        name: impl Into<String>,
        value_type: TagValueType,
        multiplicity: (u32, Option<u32>),
        default: Option<TagValue>,
    ) -> Result<TagDefinitionId, String> {
        let id = TagDefinitionId::new();
        let mut candidate = self.profiles.clone();
        candidate.tag_definitions.insert(
            id,
            TagDefinition {
                id,
                external_id: external_id.into(),
                stereotype_id,
                name: name.into(),
                value_type,
                lower: multiplicity.0,
                upper: multiplicity.1,
                default,
            },
        );
        candidate.validate(self)?;
        self.profiles = candidate;
        Ok(id)
    }

    pub fn apply_profile(
        &mut self,
        profile_id: ProfileId,
        scope_id: ElementId,
        external_id: impl Into<String>,
    ) -> Result<ProfileApplicationId, String> {
        if let Some(existing) = self.profiles.profile_applications.values().find(|application| {
            application.profile_id == profile_id && application.scope_id == scope_id
        }) {
            return Ok(existing.id);
        }
        let id = ProfileApplicationId::new();
        let mut candidate = self.profiles.clone();
        candidate.profile_applications.insert(
            id,
            ProfileApplication {
                id,
                external_id: external_id.into(),
                profile_id,
                scope_id,
            },
        );
        candidate.validate(self)?;
        self.profiles = candidate;
        Ok(id)
    }

    pub fn apply_stereotype(
        &mut self,
        stereotype_id: StereotypeId,
        target: SemanticTarget,
        external_id: impl Into<String>,
    ) -> Result<StereotypeApplicationId, String> {
        if let Some(existing) = self
            .profiles
            .stereotype_applications
            .values()
            .find(|application| {
                application.stereotype_id == stereotype_id && application.target == target
            })
        {
            return Ok(existing.id);
        }
        let id = StereotypeApplicationId::new();
        let mut candidate = self.profiles.clone();
        candidate.stereotype_applications.insert(
            id,
            StereotypeApplication {
                id,
                external_id: external_id.into(),
                stereotype_id,
                target: target.clone(),
                tagged_values: BTreeMap::new(),
            },
        );
        candidate.validate(self)?;
        let label = candidate
            .stereotypes
            .get(&stereotype_id)
            .expect("validated stereotype")
            .name
            .clone();
        self.profiles = candidate;
        self.add_legacy_compatible_label(&target, &label)?;
        Ok(id)
    }

    pub fn set_tagged_values(
        &mut self,
        application_id: StereotypeApplicationId,
        definition_id: TagDefinitionId,
        values: Vec<TagValue>,
    ) -> Result<(), String> {
        let mut candidate = self.profiles.clone();
        let application = candidate
            .stereotype_applications
            .get_mut(&application_id)
            .ok_or_else(|| format!("stereotype application not found: {application_id}"))?;
        if values.is_empty() {
            application.tagged_values.remove(&definition_id);
        } else {
            application.tagged_values.insert(definition_id, values);
        }
        candidate.validate(self)?;
        self.profiles = candidate;
        Ok(())
    }

    pub fn remove_stereotype_application(
        &mut self,
        application_id: StereotypeApplicationId,
    ) -> Result<(), String> {
        let mut candidate = self.profiles.clone();
        let application = candidate
            .stereotype_applications
            .remove(&application_id)
            .ok_or_else(|| format!("stereotype application not found: {application_id}"))?;
        let label = candidate
            .stereotypes
            .get(&application.stereotype_id)
            .map(|stereotype| stereotype.name.clone());
        candidate.validate(self)?;
        self.profiles = candidate;
        if let Some(label) = label
            && !self.profiles.stereotype_applications.values().any(|other| {
                other.target == application.target
                    && self
                        .profiles
                        .stereotypes
                        .get(&other.stereotype_id)
                        .is_some_and(|stereotype| stereotype.name == label)
            })
        {
            self.remove_legacy_compatible_label(&application.target, &label)?;
        }
        Ok(())
    }

    fn add_legacy_compatible_label(
        &mut self,
        target: &SemanticTarget,
        label: &str,
    ) -> Result<(), String> {
        let labels = match *target {
            SemanticTarget::Element(id) => &mut self
                .elements
                .get_mut(&id)
                .ok_or_else(|| format!("stereotype target element not found: {id}"))?
                .applied_stereotypes,
            SemanticTarget::Relationship(id) => &mut self
                .relationships
                .get_mut(&id)
                .ok_or_else(|| format!("stereotype target relationship not found: {id}"))?
                .applied_stereotypes,
        };
        if !labels.iter().any(|candidate| candidate == label) {
            labels.push(label.to_owned());
        }
        Ok(())
    }

    fn remove_legacy_compatible_label(
        &mut self,
        target: &SemanticTarget,
        label: &str,
    ) -> Result<(), String> {
        let labels = match *target {
            SemanticTarget::Element(id) => &mut self
                .elements
                .get_mut(&id)
                .ok_or_else(|| format!("stereotype target element not found: {id}"))?
                .applied_stereotypes,
            SemanticTarget::Relationship(id) => &mut self
                .relationships
                .get_mut(&id)
                .ok_or_else(|| format!("stereotype target relationship not found: {id}"))?
                .applied_stereotypes,
        };
        labels.retain(|candidate| candidate != label);
        Ok(())
    }
}

fn require_identity(
    external_id: &str,
    name: &str,
    external_ids: &mut HashSet<String>,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("profile definition name cannot be empty".into());
    }
    require_external_id(external_id, external_ids)
}

fn require_external_id(external_id: &str, external_ids: &mut HashSet<String>) -> Result<(), String> {
    if external_id.trim().is_empty() {
        return Err("profile external identity cannot be empty".into());
    }
    if !external_ids.insert(external_id.to_owned()) {
        return Err(format!("duplicate profile external identity: {external_id}"));
    }
    Ok(())
}

fn scope_contains(project: &Project, scope_id: ElementId, mut candidate: ElementId) -> bool {
    let mut visited = HashSet::new();
    loop {
        if candidate == scope_id {
            return true;
        }
        if !visited.insert(candidate) {
            return false;
        }
        let Some(owner) = project.elements.get(&candidate).and_then(|element| element.owner_id) else {
            return false;
        };
        candidate = owner;
    }
}

fn validate_value_count(definition: &TagDefinition, count: usize) -> Result<(), String> {
    if count < definition.lower as usize || definition.upper.is_some_and(|upper| count > upper as usize) {
        return Err(format!(
            "tag '{}' value count {count} violates multiplicity {}..{}",
            definition.name,
            definition.lower,
            definition.upper.map_or_else(|| "*".into(), |value| value.to_string())
        ));
    }
    Ok(())
}

fn validate_tag_value(
    project: &Project,
    definition: &TagDefinition,
    value: &TagValue,
) -> Result<(), String> {
    let valid = match (&definition.value_type, value) {
        (TagValueType::String, TagValue::String(_))
        | (TagValueType::Boolean, TagValue::Boolean(_))
        | (TagValueType::Integer, TagValue::Integer(_)) => true,
        (TagValueType::Real, TagValue::Real(value)) => value.is_finite(),
        (TagValueType::Enumeration { literals }, TagValue::Enumeration(value)) => {
            literals.contains(value)
        }
        (TagValueType::SemanticReference, TagValue::SemanticReference(target)) => match target {
            SemanticTarget::Element(id) => project.elements.contains_key(id),
            SemanticTarget::Relationship(id) => project.relationships.contains_key(id),
        },
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(format!("tag '{}' has a value of the wrong type", definition.name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ElementKind, RelationshipKind};

    fn safety_profile() -> (Project, ElementId, StereotypeId, TagDefinitionId) {
        let mut project = Project::new("Vehicle");
        let block = project
            .create_element(ElementKind::Block, "Controller", project.root_id)
            .unwrap();
        let profile = project
            .create_profile("profile:safety", "Safety", Some("urn:example:safety".into()))
            .unwrap();
        let stereotype = project
            .create_stereotype(
                profile,
                "profile:safety:critical",
                "SafetyCritical",
                vec![
                    StereotypeTargetKind::Element(ElementKind::Block),
                    StereotypeTargetKind::Relationship(RelationshipKind::Dependency),
                ],
            )
            .unwrap();
        let tag = project
            .create_tag_definition(
                stereotype,
                "profile:safety:level",
                "level",
                TagValueType::Enumeration {
                    literals: vec!["A".into(), "B".into()],
                },
                (1, Some(1)),
                Some(TagValue::Enumeration("B".into())),
            )
            .unwrap();
        project
            .apply_profile(profile, project.root_id, "profile-app:safety")
            .unwrap();
        (project, block, stereotype, tag)
    }

    #[test]
    fn profile_application_and_typed_tags_are_first_class() {
        let (mut project, block, stereotype, tag) = safety_profile();
        let application = project
            .apply_stereotype(
                stereotype,
                SemanticTarget::Element(block),
                "stereotype-app:controller:safety",
            )
            .unwrap();
        project
            .set_tagged_values(application, tag, vec![TagValue::Enumeration("A".into())])
            .unwrap();

        project.validate().unwrap();
        assert_eq!(
            project.profiles.stereotype_applications[&application].tagged_values[&tag],
            vec![TagValue::Enumeration("A".into())]
        );
        assert_eq!(project.element(block).unwrap().applied_stereotypes, ["SafetyCritical"]);
    }

    #[test]
    fn incompatible_stereotype_and_invalid_tag_value_are_blocked_without_mutation() {
        let (mut project, _, stereotype, tag) = safety_profile();
        let package = project
            .create_element(ElementKind::Package, "Subsystem", project.root_id)
            .unwrap();
        assert!(
            project
                .apply_stereotype(
                    stereotype,
                    SemanticTarget::Element(package),
                    "stereotype-app:invalid",
                )
                .unwrap_err()
                .contains("cannot extend")
        );
        assert!(project.profiles.stereotype_applications.is_empty());

        let block = project
            .elements
            .values()
            .find(|element| element.kind == ElementKind::Block)
            .unwrap()
            .id;
        let application = project
            .apply_stereotype(
                stereotype,
                SemanticTarget::Element(block),
                "stereotype-app:valid",
            )
            .unwrap();
        assert!(
            project
                .set_tagged_values(application, tag, vec![TagValue::Enumeration("Z".into())])
                .unwrap_err()
                .contains("wrong type")
        );
        assert!(project.profiles.stereotype_applications[&application]
            .tagged_values
            .is_empty());
    }

    #[test]
    fn unresolved_legacy_labels_are_preserved() {
        let mut project = Project::new("Legacy");
        let block = project
            .create_element(ElementKind::Block, "Legacy Block", project.root_id)
            .unwrap();
        project.element_mut(block).unwrap().applied_stereotypes = vec!["SupplierLabel".into()];
        project.validate().unwrap();
        assert_eq!(project.element(block).unwrap().applied_stereotypes, ["SupplierLabel"]);
        assert!(project.profiles.stereotypes.is_empty());
    }
}
