include!("model.rs");

// Keep Activity topology validation as an explicit one-branch-per-node-kind table.
// Collapsing guarded node cases makes the UML/SysML control-node rules less auditable.
#[allow(clippy::collapsible_match)]
pub mod activity;
pub use activity::*;
