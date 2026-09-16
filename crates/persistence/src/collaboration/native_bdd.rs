//! Compatibility adapter: existing shared BDD records keep their storage and
//! protocol shape while executing the native core's typed geometry command.
use super::{CollaborationError, SharedBddDiagram};
use systems_modeler_core::structural_presentation::geometry::{
    BddGeometryCommand, BddRoutingScope, apply_bdd_geometry,
};
use systems_modeler_core::structural_presentation::{BddDiagram, DiagramEdge, DiagramNode};

impl SharedBddDiagram {
    /// IDs and authored geometry are preserved for use by the normal workspace.
    /// Shared BDD v1 has no subject, actor notation or constraint parameters.
    pub fn native_presentation(&self) -> BddDiagram {
        BddDiagram {
            id: self.id.to_string(),
            name: self.name.clone(),
            owner_id: self.owner.to_string(),
            family: "bdd".into(),
            semantic_context_id: None,
            subject_boundary: None,
            nodes: self
                .nodes
                .iter()
                .map(|node| DiagramNode {
                    id: node.id.to_string(),
                    element_id: node.element.to_string(),
                    x: node.x,
                    y: node.y,
                    width: node.width,
                    height: node.height,
                    actor_notation: None,
                    parameter_presentations: Vec::new(),
                })
                .collect(),
            edges: self
                .edges
                .iter()
                .map(|edge| DiagramEdge {
                    id: edge.id.to_string(),
                    relationship_id: edge.relationship.to_string(),
                    source_node_id: edge.source_node.to_string(),
                    target_node_id: edge.target_node.to_string(),
                    points: edge.points.clone(),
                    label_anchor: Some(edge.label_anchor),
                })
                .collect(),
        }
    }

    pub(super) fn apply_native_geometry(
        &mut self,
        command: &BddGeometryCommand,
    ) -> Result<(), CollaborationError> {
        let mut native = self.native_presentation();
        apply_bdd_geometry(&mut native, command, BddRoutingScope::AllEdges, None).map_err(
            |_| {
                CollaborationError::InvalidDiagram(
                    "native BDD geometry command or obstacle-clear route is invalid",
                )
            },
        )?;
        let mut candidate = self.clone();
        for node in &mut candidate.nodes {
            let changed = native
                .nodes
                .iter()
                .find(|value| value.id == node.id.to_string())
                .ok_or(CollaborationError::InvalidDiagram(
                    "native BDD node was not found",
                ))?;
            node.x = changed.x;
            node.y = changed.y;
            node.width = changed.width;
            node.height = changed.height;
        }
        for edge in &mut candidate.edges {
            let changed = native
                .edges
                .iter()
                .find(|value| value.id == edge.id.to_string())
                .ok_or(CollaborationError::InvalidDiagram(
                    "native BDD edge was not found",
                ))?;
            edge.points = changed.points.clone();
            edge.label_anchor = changed
                .label_anchor
                .ok_or(CollaborationError::InvalidDiagram(
                    "native BDD label was not found",
                ))?;
        }
        *self = candidate;
        Ok(())
    }
}
