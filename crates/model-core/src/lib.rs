include!("model.rs");

// Keep Activity topology validation as an explicit one-branch-per-node-kind table.
// Collapsing guarded node cases makes the UML/SysML control-node rules less auditable.
#[allow(clippy::collapsible_match)]
pub mod activity;
pub use activity::*;

pub mod execution;
pub use execution::*;

pub mod execution_expression;
pub use execution_expression::*;

pub mod activity_execution;
pub use activity_execution::*;

pub mod namespace;
pub use namespace::*;

pub mod diagram_family;
pub use diagram_family::{
    DiagramCapability, DiagramFamilyDescriptor, DiagramFamilyId, DiagramFamilyRegistry,
    DiagramGeometry, DiagramGeometrySnapshot, GeometryPoint, GeometryRect, PanelPreference,
    PreferredFlowDirection, RelationshipGeometry, ViewportPreference, fit_viewport,
    zoom_viewport_at,
};

mod package_registry;
pub use package_registry::supported_diagram_families;
