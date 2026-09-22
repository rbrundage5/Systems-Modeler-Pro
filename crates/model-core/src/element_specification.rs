//! The structural Properties form is one validated semantic edit.
use crate::{Element, ElementId, ElementKind, Multiplicity, Project};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ElementSpecificationEdit {
    pub name: String,
    pub documentation: Option<String>,
    pub type_id: Option<ElementId>,
    pub default_value: Option<String>,
    pub multiplicity: Option<String>,
    pub aggregation: Option<String>,
    pub is_derived: Option<bool>,
    pub is_read_only: Option<bool>,
    pub is_conjugated: Option<bool>,
    pub parameter_direction: Option<String>,
    pub flow_direction: Option<String>,
    pub quantity_kind_external_id: Option<String>,
    pub unit_external_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElementTypeChoice {
    pub id: ElementId,
    pub external_id: String,
    pub qualified_name: String,
    pub kind: ElementKind,
}

fn multiplicity_supported(element: &Element) -> bool {
    element.is_property() || element.is_port() || element.kind == ElementKind::Parameter
}

fn type_supported(element: &Element) -> bool {
    multiplicity_supported(element)
        || matches!(
            element.kind,
            ElementKind::Reception | ElementKind::InstanceSpecification
        )
}

/// Text parsing and bound validation stay in Rust, including the u32 range.
pub fn parse_specification_multiplicity(text: &str) -> Result<Multiplicity, String> {
    let text = text.trim();
    let invalid = || {
        "Multiplicity must be a nonnegative integer, *, or lower..upper (for example 0..1 or 1..*)."
            .to_string()
    };
    let bound = |value: &str| {
        if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(invalid());
        }
        value.parse::<u32>().map_err(|_| invalid())
    };
    if text == "*" {
        return Multiplicity::new(0, None).map_err(|error| error.to_string());
    }
    let (lower, upper) = if let Some((lower, upper)) = text.split_once("..") {
        (
            bound(lower)?,
            if upper == "*" {
                None
            } else {
                Some(bound(upper)?)
            },
        )
    } else {
        let value = bound(text)?;
        (value, Some(value))
    };
    Multiplicity::new(lower, upper).map_err(|error| error.to_string())
}

impl Project {
    /// Candidates use the same type-kind rules as set_element_type, never a UI copy.
    /// Dependent connector/presentation compatibility is checked when applying.
    pub fn element_type_choices(&self, id: ElementId) -> Result<Vec<ElementTypeChoice>, String> {
        let element = self.element(id).map_err(|error| error.to_string())?;
        if !type_supported(element) {
            return Ok(Vec::new());
        }
        let mut choices: Vec<_> = self
            .elements
            .values()
            .filter(|candidate| crate::validate_type_kind(&element.kind, &candidate.kind).is_ok())
            .map(|candidate| ElementTypeChoice {
                id: candidate.id,
                external_id: candidate.external_id.clone(),
                qualified_name: self
                    .qualified_name(candidate.id)
                    .unwrap_or_else(|_| candidate.name.clone()),
                kind: candidate.kind.clone(),
            })
            .collect();
        choices.sort_by(|a, b| {
            a.qualified_name
                .cmp(&b.qualified_name)
                .then(a.external_id.cmp(&b.external_id))
        });
        Ok(choices)
    }

    /// Build a replacement without mutating the caller, including on late failure.
    pub fn stage_element_specification(
        &self,
        id: ElementId,
        edit: &ElementSpecificationEdit,
    ) -> Result<Self, String> {
        let element = self.element(id).map_err(|error| error.to_string())?;
        let kind = element.kind.clone();
        if edit.name.trim().is_empty() {
            return Err("Name cannot be empty.".into());
        }
        let require = |supported: bool, field: &str| {
            if supported {
                Ok(())
            } else {
                Err(format!("{kind:?} does not support {field}"))
            }
        };
        if edit.type_id.is_some() {
            require(type_supported(element), "a type")?;
        }
        if edit.multiplicity.is_some() || edit.is_derived.is_some() || edit.is_read_only.is_some() {
            require(
                multiplicity_supported(element),
                "multiplicity/derived/read-only properties",
            )?;
        }
        if edit.aggregation.is_some() {
            require(
                matches!(
                    kind,
                    ElementKind::PartProperty | ElementKind::ReferenceProperty
                ),
                "aggregation",
            )?;
        }
        if edit.is_conjugated.is_some() {
            require(element.is_port(), "port conjugation")?;
        }
        if edit.parameter_direction.is_some() {
            require(kind == ElementKind::Parameter, "parameter direction")?;
        }
        if edit.flow_direction.is_some() {
            require(kind == ElementKind::FlowProperty, "flow direction")?;
        }
        if edit.quantity_kind_external_id.is_some() || edit.unit_external_id.is_some() {
            require(
                matches!(
                    kind,
                    ElementKind::ValueType
                        | ElementKind::ValueProperty
                        | ElementKind::ConstraintParameter
                        | ElementKind::Unit
                ),
                "quantity/unit metadata",
            )?;
        }

        let mut candidate = self.clone();
        candidate
            .rename_element(id, edit.name.trim())
            .map_err(|error| error.to_string())?;
        if let Some(type_id) = edit.type_id {
            candidate
                .set_element_type(id, type_id)
                .map_err(|error| error.to_string())?;
        }
        if let Some(text) = &edit.multiplicity {
            candidate
                .set_multiplicity(id, parse_specification_multiplicity(text)?)
                .map_err(|error| error.to_string())?;
        }
        if let Some(text) = &edit.aggregation {
            let aggregation = match text.as_str() {
                "none" => crate::AggregationKind::None,
                "shared" => crate::AggregationKind::Shared,
                "composite" => crate::AggregationKind::Composite,
                _ => return Err(format!("Invalid aggregation: {text}")),
            };
            candidate
                .set_aggregation(id, aggregation)
                .map_err(|error| error.to_string())?;
        }
        let updated = candidate
            .element_mut(id)
            .map_err(|error| error.to_string())?;
        if let Some(value) = &edit.documentation {
            updated.documentation = value.clone();
        }
        if let Some(value) = &edit.default_value {
            updated.default_value = nonempty(value);
        }
        if let Some(value) = edit.is_derived {
            updated.is_derived = value;
        }
        if let Some(value) = edit.is_read_only {
            updated.is_read_only = value;
        }
        if let Some(value) = edit.is_conjugated {
            updated.is_conjugated = value;
        }
        if let Some(value) = &edit.quantity_kind_external_id {
            updated.quantity_kind_external_id = nonempty(value);
        }
        if let Some(value) = &edit.unit_external_id {
            updated.unit_external_id = nonempty(value);
        }
        if let Some(value) = &edit.parameter_direction {
            updated.parameter_direction = Some(match value.as_str() {
                "in" => crate::ParameterDirection::In,
                "out" => crate::ParameterDirection::Out,
                "inout" => crate::ParameterDirection::InOut,
                "return" => crate::ParameterDirection::Return,
                _ => return Err(format!("Invalid parameter direction: {value}")),
            });
        }
        if let Some(value) = &edit.flow_direction {
            updated.flow_direction = Some(match value.as_str() {
                "in" => crate::FlowDirection::In,
                "out" => crate::FlowDirection::Out,
                "inout" => crate::FlowDirection::InOut,
                _ => return Err(format!("Invalid flow direction: {value}")),
            });
        }
        candidate.validate().map_err(|error| error.to_string())?;
        Ok(candidate)
    }
}

fn nonempty(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AggregationKind, BindingEndpoint};

    fn fixture(kind: ElementKind) -> (Project, ElementId, ElementId, ElementId) {
        let mut project = Project::new("Specification");
        let block = project
            .create_element(ElementKind::Block, "System", project.root_id)
            .unwrap();
        let first = project
            .create_element(ElementKind::Block, "Unit", project.root_id)
            .unwrap();
        let second = project
            .create_element(ElementKind::Block, "Replacement", project.root_id)
            .unwrap();
        let feature = project
            .create_typed_feature(kind, "unit", block, first, Multiplicity::ONE)
            .unwrap();
        (project, feature, first, second)
    }

    fn edit(name: &str) -> ElementSpecificationEdit {
        ElementSpecificationEdit {
            name: name.into(),
            ..Default::default()
        }
    }

    #[test]
    fn specification_applies_all_fields_without_changing_identity_or_source() {
        let (project, id, _, replacement) = fixture(ElementKind::PartProperty);
        let before = serde_json::to_value(&project).unwrap();
        let original = project.element(id).unwrap();
        let edit = ElementSpecificationEdit {
            name: " updated ".into(),
            documentation: Some("engineering notes".into()),
            type_id: Some(replacement),
            default_value: Some("initial".into()),
            multiplicity: Some("0..*".into()),
            aggregation: Some("composite".into()),
            is_derived: Some(true),
            is_read_only: Some(true),
            ..Default::default()
        };
        let candidate = project.stage_element_specification(id, &edit).unwrap();
        let changed = candidate.element(id).unwrap();
        assert_eq!(changed.id, original.id);
        assert_eq!(changed.external_id, original.external_id);
        assert_eq!(changed.owner_id, original.owner_id);
        assert_eq!(changed.name, "updated");
        assert_eq!(changed.type_id, Some(replacement));
        assert_eq!(
            changed.multiplicity,
            Some(Multiplicity {
                lower: 0,
                upper: None
            })
        );
        assert_eq!(changed.documentation, "engineering notes");
        assert_eq!(changed.default_value.as_deref(), Some("initial"));
        assert!(changed.is_derived && changed.is_read_only);
        assert_eq!(changed.aggregation, AggregationKind::Composite);
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
    }

    #[test]
    fn specification_rejections_never_partially_rename_or_retype() {
        let (project, id, _, replacement) = fixture(ElementKind::PartProperty);
        let before = serde_json::to_value(&project).unwrap();
        for invalid in [
            "2..1",
            "-1",
            "+1",
            "1.5",
            "1..",
            "1..2..3",
            "4294967296",
            "",
        ] {
            let change = ElementSpecificationEdit {
                name: "changed".into(),
                type_id: Some(replacement),
                multiplicity: Some(invalid.into()),
                ..Default::default()
            };
            assert!(
                project.stage_element_specification(id, &change).is_err(),
                "{invalid}"
            );
            assert_eq!(serde_json::to_value(&project).unwrap(), before);
        }
        for change in [
            edit(" "),
            ElementSpecificationEdit {
                aggregation: Some("none".into()),
                ..edit("changed")
            },
            ElementSpecificationEdit {
                type_id: Some(project.root_id),
                ..edit("changed")
            },
            ElementSpecificationEdit {
                type_id: Some(ElementId::new()),
                ..edit("changed")
            },
            ElementSpecificationEdit {
                parameter_direction: Some("in".into()),
                ..edit("changed")
            },
            ElementSpecificationEdit {
                unit_external_id: Some("not-a-unit".into()),
                ..edit("changed")
            },
        ] {
            assert!(project.stage_element_specification(id, &change).is_err());
            assert_eq!(serde_json::to_value(&project).unwrap(), before);
        }
    }

    #[test]
    fn specification_multiplicity_parsing_preserves_zero_and_unbounded() {
        for (text, expected) in [
            ("0", (0, Some(0))),
            ("0..1", (0, Some(1))),
            ("*", (0, None)),
            ("1..*", (1, None)),
            (" 4 ", (4, Some(4))),
        ] {
            let value = parse_specification_multiplicity(text).unwrap();
            assert_eq!((value.lower, value.upper), expected);
        }
    }

    #[test]
    fn specification_type_choices_keep_duplicate_names_distinguishable() {
        let (mut project, id, first, second) = fixture(ElementKind::PartProperty);
        project.rename_element(second, "Unit").unwrap();
        let package = project
            .create_element(ElementKind::Package, "Library", project.root_id)
            .unwrap();
        project.move_element(second, package).unwrap();
        let primitive = project
            .create_element(ElementKind::PrimitiveType, "Real", package)
            .unwrap();
        let choices = project.element_type_choices(id).unwrap();
        assert!(
            choices
                .iter()
                .any(|choice| choice.id == first && choice.qualified_name == "Specification::Unit")
        );
        assert!(
            choices.iter().any(|choice| choice.id == second
                && choice.qualified_name == "Specification::Library::Unit")
        );
        assert!(
            !choices
                .iter()
                .any(|choice| choice.id == primitive || choice.id == id)
        );
    }

    #[test]
    fn specification_conjugation_rejects_full_port_but_accepts_proxy_port() {
        for (kind, valid) in [
            (ElementKind::FullPort, false),
            (ElementKind::ProxyPort, true),
        ] {
            let (project, id, _, replacement) = fixture(kind);
            let before = serde_json::to_value(&project).unwrap();
            let change = ElementSpecificationEdit {
                type_id: Some(replacement),
                is_conjugated: Some(true),
                ..edit("updated")
            };
            assert_eq!(
                project.stage_element_specification(id, &change).is_ok(),
                valid
            );
            assert_eq!(serde_json::to_value(&project).unwrap(), before);
        }
    }

    #[test]
    fn specification_retype_rejects_a_broken_existing_binding() {
        let mut project = Project::new("Binding");
        let root = project.root_id;
        let block = project
            .create_element(ElementKind::Block, "System", root)
            .unwrap();
        let real = project
            .create_element(ElementKind::PrimitiveType, "Real", root)
            .unwrap();
        let integer = project
            .create_element(ElementKind::PrimitiveType, "Integer", root)
            .unwrap();
        let a = project
            .create_typed_feature(
                ElementKind::ValueProperty,
                "a",
                block,
                real,
                Multiplicity::ONE,
            )
            .unwrap();
        let b = project
            .create_typed_feature(
                ElementKind::ValueProperty,
                "b",
                block,
                real,
                Multiplicity::ONE,
            )
            .unwrap();
        project
            .create_binding_connector(
                block,
                BindingEndpoint {
                    role_id: a,
                    parameter_id: None,
                },
                BindingEndpoint {
                    role_id: b,
                    parameter_id: None,
                },
            )
            .unwrap();
        let before = serde_json::to_value(&project).unwrap();
        assert!(
            project
                .stage_element_specification(
                    a,
                    &ElementSpecificationEdit {
                        type_id: Some(integer),
                        ..edit("retyped")
                    }
                )
                .is_err()
        );
        assert_eq!(serde_json::to_value(&project).unwrap(), before);
    }

    #[test]
    fn specification_empty_default_clears_but_omitted_default_preserves() {
        let (mut project, id, _, _) = fixture(ElementKind::ReferenceProperty);
        project.element_mut(id).unwrap().default_value = Some("existing".into());
        let preserved = project
            .stage_element_specification(id, &edit("reference"))
            .unwrap();
        assert_eq!(
            preserved.element(id).unwrap().default_value.as_deref(),
            Some("existing")
        );
        let cleared = project
            .stage_element_specification(
                id,
                &ElementSpecificationEdit {
                    default_value: Some(String::new()),
                    ..edit("reference")
                },
            )
            .unwrap();
        assert_eq!(cleared.element(id).unwrap().default_value, None);
    }
}
