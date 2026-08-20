//! Extensible, renderer-independent contracts for diagram workspace families.
//!
//! A family contributes semantic and geometry capabilities. The desktop workspace
//! consumes this data without branching on a closed list of diagram names.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DiagramFamilyId(pub String);

impl DiagramFamilyId {
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if value.is_empty()
            || !value
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
        {
            return Err("diagram family identifier must be non-empty ASCII kebab-case".into());
        }
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagramCapability {
    NodePlacement,
    Relationships,
    Frames,
    Move,
    Resize,
    Delete,
    Clipboard,
    Routing,
    CleanLayout,
    DrillDown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreferredFlowDirection {
    LeftToRight,
    TopToBottom,
    Freeform,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramFamilyDescriptor {
    pub id: DiagramFamilyId,
    pub display_name: String,
    pub renderer_id: String,
    pub permitted_owner_kinds: Vec<String>,
    pub capabilities: BTreeSet<DiagramCapability>,
    pub preferred_flow: PreferredFlowDirection,
    pub accessibility_name: String,
    pub empty_message: String,
}

impl DiagramFamilyDescriptor {
    pub fn supports(&self, capability: DiagramCapability) -> bool {
        self.capabilities.contains(&capability)
    }
}

#[derive(Debug, Clone, Default)]
pub struct DiagramFamilyRegistry {
    families: BTreeMap<DiagramFamilyId, DiagramFamilyDescriptor>,
}

impl DiagramFamilyRegistry {
    pub fn register(&mut self, descriptor: DiagramFamilyDescriptor) -> Result<(), String> {
        if descriptor.renderer_id.trim().is_empty() {
            return Err("diagram renderer identifier is required".into());
        }
        if descriptor.permitted_owner_kinds.is_empty() {
            return Err("diagram family must declare permitted semantic owners".into());
        }
        if self.families.contains_key(&descriptor.id) {
            return Err(format!(
                "diagram family is already registered: {}",
                descriptor.id.0
            ));
        }
        self.families.insert(descriptor.id.clone(), descriptor);
        Ok(())
    }

    pub fn get(&self, id: &DiagramFamilyId) -> Option<&DiagramFamilyDescriptor> {
        self.families.get(id)
    }

    pub fn descriptors(&self) -> Vec<DiagramFamilyDescriptor> {
        self.families.values().cloned().collect()
    }
}

fn capabilities(values: &[DiagramCapability]) -> BTreeSet<DiagramCapability> {
    values.iter().copied().collect()
}

pub fn supported_diagram_families() -> DiagramFamilyRegistry {
    use DiagramCapability as C;
    let mut registry = DiagramFamilyRegistry::default();
    for descriptor in [
        descriptor(
            "bdd",
            "Block Definition Diagram",
            "bdd",
            &["Model", "Package"],
            &[
                C::NodePlacement,
                C::Relationships,
                C::Move,
                C::Resize,
                C::Delete,
                C::Clipboard,
                C::Routing,
                C::CleanLayout,
            ],
            PreferredFlowDirection::TopToBottom,
        ),
        descriptor(
            "ibd",
            "Internal Block Diagram",
            "ibd",
            &["Block", "AssociationBlock", "InterfaceBlock"],
            &[
                C::NodePlacement,
                C::Relationships,
                C::Frames,
                C::Move,
                C::Resize,
                C::Delete,
                C::Clipboard,
                C::Routing,
                C::CleanLayout,
                C::DrillDown,
            ],
            PreferredFlowDirection::LeftToRight,
        ),
        descriptor(
            "state-machine",
            "State Machine Diagram",
            "state-machine",
            &["Block", "AssociationBlock", "InterfaceBlock"],
            &[
                C::NodePlacement,
                C::Relationships,
                C::Frames,
                C::Move,
                C::Resize,
                C::Delete,
                C::Clipboard,
                C::Routing,
                C::CleanLayout,
                C::DrillDown,
            ],
            PreferredFlowDirection::TopToBottom,
        ),
        descriptor(
            "sequence",
            "Sequence Diagram",
            "sequence",
            &["Block", "AssociationBlock", "InterfaceBlock"],
            &[
                C::NodePlacement,
                C::Relationships,
                C::Frames,
                C::Move,
                C::Resize,
                C::Delete,
                C::Clipboard,
                C::Routing,
                C::CleanLayout,
                C::DrillDown,
            ],
            PreferredFlowDirection::LeftToRight,
        ),
        descriptor(
            "activity",
            "Activity Diagram",
            "activity",
            &["Model", "Package", "Block", "AssociationBlock"],
            &[
                C::NodePlacement,
                C::Relationships,
                C::Frames,
                C::Move,
                C::Resize,
                C::Delete,
                C::Clipboard,
                C::Routing,
                C::CleanLayout,
                C::DrillDown,
            ],
            PreferredFlowDirection::TopToBottom,
        ),
    ] {
        registry
            .register(descriptor)
            .expect("built-in diagram family must be valid");
    }
    registry
}

fn descriptor(
    id: &str,
    name: &str,
    renderer: &str,
    owners: &[&str],
    supported: &[DiagramCapability],
    flow: PreferredFlowDirection,
) -> DiagramFamilyDescriptor {
    DiagramFamilyDescriptor {
        id: DiagramFamilyId::new(id).expect("static family id is valid"),
        display_name: name.into(),
        renderer_id: renderer.into(),
        permitted_owner_kinds: owners.iter().map(|value| (*value).into()).collect(),
        capabilities: capabilities(supported),
        preferred_flow: flow,
        accessibility_name: format!("{name} engineering workspace"),
        empty_message: format!("This {name} has no presented elements."),
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeometryPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeometryRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl GeometryRect {
    pub fn right(self) -> f64 {
        self.x + self.width
    }
    pub fn bottom(self) -> f64 {
        self.y + self.height
    }
    pub fn union(self, other: Self) -> Self {
        let left = self.x.min(other.x);
        let top = self.y.min(other.y);
        Self {
            x: left,
            y: top,
            width: self.right().max(other.right()) - left,
            height: self.bottom().max(other.bottom()) - top,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramGeometry {
    pub id: String,
    pub bounds: GeometryRect,
    #[serde(default)]
    pub movable: bool,
    #[serde(default)]
    pub fixed: bool,
    #[serde(default)]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationshipGeometry {
    pub id: String,
    pub points: Vec<GeometryPoint>,
    #[serde(default)]
    pub label_bounds: Option<GeometryRect>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagramGeometrySnapshot {
    pub nodes: Vec<DiagramGeometry>,
    pub ports: Vec<DiagramGeometry>,
    pub frames: Vec<DiagramGeometry>,
    pub relationships: Vec<RelationshipGeometry>,
}

impl DiagramGeometrySnapshot {
    pub fn content_bounds(&self) -> GeometryRect {
        let mut all = self
            .nodes
            .iter()
            .chain(&self.ports)
            .chain(&self.frames)
            .map(|item| item.bounds)
            .chain(self.relationships.iter().flat_map(|edge| {
                edge.points
                    .iter()
                    .map(|point| GeometryRect {
                        x: point.x,
                        y: point.y,
                        width: 0.0,
                        height: 0.0,
                    })
                    .chain(edge.label_bounds)
            }));
        let Some(first) = all.next() else {
            return GeometryRect::default();
        };
        all.fold(first, GeometryRect::union)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewportPreference {
    pub zoom: f64,
    pub pan_x: f64,
    pub pan_y: f64,
    pub grid_visible: bool,
    pub snap_to_grid: bool,
}

impl Default for ViewportPreference {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            grid_visible: true,
            snap_to_grid: true,
        }
    }
}

impl ViewportPreference {
    pub fn validate(&self) -> Result<(), String> {
        if !(0.25..=4.0).contains(&self.zoom) || !self.pan_x.is_finite() || !self.pan_y.is_finite()
        {
            return Err("viewport preference contains invalid zoom or pan values".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanelPreference {
    pub repository_width: u16,
    pub elements_width: u16,
    pub properties_width: u16,
    pub repository_visible: bool,
    pub elements_visible: bool,
    pub properties_visible: bool,
}

impl Default for PanelPreference {
    fn default() -> Self {
        Self {
            repository_width: 250,
            elements_width: 220,
            properties_width: 290,
            repository_visible: true,
            elements_visible: true,
            properties_visible: true,
        }
    }
}

impl PanelPreference {
    pub fn validate(&self) -> Result<(), String> {
        for width in [
            self.repository_width,
            self.elements_width,
            self.properties_width,
        ] {
            if !(150..=480).contains(&width) {
                return Err("panel width must be between 150 and 480 pixels".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_is_extensible_and_rejects_duplicates() {
        let mut registry = supported_diagram_families();
        assert_eq!(registry.descriptors().len(), 5);
        let future = descriptor(
            "requirement",
            "Requirement Diagram",
            "requirement",
            &["Model", "Package"],
            &[DiagramCapability::NodePlacement],
            PreferredFlowDirection::TopToBottom,
        );
        assert!(registry.register(future.clone()).is_ok());
        assert!(registry.register(future).is_err());
    }
    #[test]
    fn capabilities_are_family_owned() {
        let registry = supported_diagram_families();
        let sequence = registry
            .get(&DiagramFamilyId::new("sequence").unwrap())
            .unwrap();
        assert!(sequence.supports(DiagramCapability::Routing));
        assert!(sequence.supports(DiagramCapability::CleanLayout));
    }
    #[test]
    fn bounds_include_nodes_ports_frames_routes_and_labels() {
        let snapshot = DiagramGeometrySnapshot {
            nodes: vec![DiagramGeometry {
                id: "n".into(),
                bounds: GeometryRect {
                    x: 10.0,
                    y: 20.0,
                    width: 100.0,
                    height: 80.0,
                },
                movable: true,
                fixed: false,
                parent_id: None,
            }],
            ports: vec![],
            frames: vec![],
            relationships: vec![RelationshipGeometry {
                id: "e".into(),
                points: vec![
                    GeometryPoint { x: 0.0, y: 50.0 },
                    GeometryPoint { x: 200.0, y: 50.0 },
                ],
                label_bounds: Some(GeometryRect {
                    x: 90.0,
                    y: 110.0,
                    width: 40.0,
                    height: 20.0,
                }),
            }],
        };
        assert_eq!(
            snapshot.content_bounds(),
            GeometryRect {
                x: 0.0,
                y: 20.0,
                width: 200.0,
                height: 110.0
            }
        );
    }
    #[test]
    fn viewport_round_trip_is_isolated_from_geometry() {
        let geometry = GeometryRect {
            x: 4.0,
            y: 8.0,
            width: 10.0,
            height: 20.0,
        };
        let value = ViewportPreference {
            zoom: 2.0,
            pan_x: 30.0,
            pan_y: -5.0,
            ..Default::default()
        };
        let json = serde_json::to_string(&value).unwrap();
        assert_eq!(
            serde_json::from_str::<ViewportPreference>(&json).unwrap(),
            value
        );
        assert_eq!(
            geometry,
            GeometryRect {
                x: 4.0,
                y: 8.0,
                width: 10.0,
                height: 20.0
            }
        );
    }
}
