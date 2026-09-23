use crate::{ElementId, ElementKind, ModelError, Project, RelationshipId};
use serde::{Deserialize, Serialize};

/// SysML/UML connector classification for a Block's internal structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectorKind {
    /// Connects internal roles/ports within the same structured classifier.
    Assembly,
    /// Delegates a boundary port of the context Block to an internal role/port.
    Delegation,
}

/// A connector end is not merely an element ID. `property_path` identifies the
/// nested property chain used to reach the terminal port/property from the IBD
/// context. An empty path means the terminal feature is owned directly by the
/// context Block (for example a boundary port).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorEnd {
    #[serde(default)]
    pub property_path: Vec<ElementId>,
    pub role_id: ElementId,
    pub port_id: Option<ElementId>,
}

impl ConnectorEnd {
    pub fn boundary(feature_id: ElementId) -> Self {
        Self {
            property_path: Vec::new(),
            role_id: feature_id,
            port_id: Some(feature_id),
        }
    }

    pub fn role(role_id: ElementId) -> Self {
        Self {
            property_path: vec![role_id],
            role_id,
            port_id: None,
        }
    }

    pub fn nested_port(role_path: Vec<ElementId>, port_id: ElementId) -> Self {
        let role_id = role_path.last().copied().unwrap_or(port_id);
        Self {
            property_path: role_path,
            role_id,
            port_id: Some(port_id),
        }
    }
}

/// First-class SysML Connector semantics. This deliberately lives beside the
/// generic relationship store rather than being encoded as an Association.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Connector {
    pub context_id: ElementId,
    pub kind: ConnectorKind,
    pub source: ConnectorEnd,
    pub target: ConnectorEnd,
}

/// SysML ItemFlow semantics realized by a Connector. `conveyed_item_ids`
/// references classifiers (Signal/Block/ValueType/DataType/etc.) rather than
/// duplicating conveyed items as presentation-only strings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemFlow {
    pub connector_id: RelationshipId,
    pub source: ConnectorEnd,
    pub target: ConnectorEnd,
    #[serde(default)]
    pub conveyed_item_ids: Vec<ElementId>,
}

impl Project {
    /// Stage an existing ItemFlow edit without changing its identity or connector.
    pub fn stage_item_flow_specification(
        &self,
        id: RelationshipId,
        name: &str,
        flow: ItemFlow,
    ) -> Result<Project, String> {
        let original = self.relationship(id).map_err(|error| error.to_string())?;
        let old = original.item_flow.as_ref().ok_or("relationship is not an ItemFlow")?;
        if original.kind != crate::RelationshipKind::ItemFlow || old.connector_id != flow.connector_id {
            return Err("ItemFlow edits must retain their existing Connector".into());
        }
        self.validate_item_flow(&flow).map_err(|error| error.to_string())?;
        let mut candidate = self.clone();
        let relationship = candidate.relationships.get_mut(&id).ok_or("ItemFlow not found")?;
        relationship.name = name.trim().to_owned();
        relationship.source_id = flow.source.port_id.unwrap_or(flow.source.role_id);
        relationship.target_id = flow.target.port_id.unwrap_or(flow.target.role_id);
        relationship.item_flow = Some(flow);
        candidate.validate().map_err(|error| error.to_string())?;
        Ok(candidate)
    }

    /// Stage one connector edit and preserve its dependent ItemFlow directions.
    pub fn stage_connector_specification(
        &self,
        id: RelationshipId,
        name: &str,
        connector: Connector,
    ) -> Result<Project, String> {
        let original = self.relationship(id).map_err(|error| error.to_string())?;
        let old = original.connector.as_ref().ok_or("relationship is not a Connector")?;
        if original.kind != crate::RelationshipKind::Connector || old.context_id != connector.context_id {
            return Err("connector edits must retain their existing Block context".into());
        }
        self.validate_connector(&connector).map_err(|error| error.to_string())?;
        let mut candidate = self.clone();
        let relationship = candidate.relationships.get_mut(&id).ok_or("Connector not found")?;
        relationship.name = name.trim().to_owned();
        relationship.source_id = connector.source.port_id.unwrap_or(connector.source.role_id);
        relationship.target_id = connector.target.port_id.unwrap_or(connector.target.role_id);
        relationship.connector = Some(connector.clone());
        for relationship in candidate.relationships.values_mut() {
            let Some(flow) = &mut relationship.item_flow else { continue; };
            if flow.connector_id != id { continue; }
            let forward = flow.source == old.source && flow.target == old.target;
            let reverse = flow.source == old.target && flow.target == old.source;
            if !forward && !reverse {
                return Err("dependent ItemFlow does not match the original connector ends".into());
            }
            // Reversing the connector's presentation order does not reverse flow.
            if !((flow.source == connector.source && flow.target == connector.target)
                || (flow.source == connector.target && flow.target == connector.source))
            {
                (flow.source, flow.target) = if forward {
                    (connector.source.clone(), connector.target.clone())
                } else {
                    (connector.target.clone(), connector.source.clone())
                };
            }
            relationship.source_id = flow.source.port_id.unwrap_or(flow.source.role_id);
            relationship.target_id = flow.target.port_id.unwrap_or(flow.target.role_id);
        }
        candidate.validate().map_err(|error| error.to_string())?;
        Ok(candidate)
    }

    /// Resolve the classifier reached by a connector role path. Every property
    /// in the path must be a PartProperty or ReferenceProperty whose type is a
    /// Block-like structured classifier. The path is evaluated from the IBD
    /// context outward, making nested-port semantics deterministic.
    pub fn resolve_structural_path(
        &self,
        context_id: ElementId,
        path: &[ElementId],
    ) -> Result<ElementId, ModelError> {
        let context = self.element(context_id)?;
        if !matches!(
            context.kind,
            ElementKind::Block | ElementKind::AssociationBlock
        ) {
            return Err(ModelError::InvalidIbdContext(context_id));
        }
        let mut classifier_id = context_id;
        for property_id in path {
            let property = self.element(*property_id)?;
            if !matches!(
                property.kind,
                ElementKind::PartProperty | ElementKind::ReferenceProperty
            ) || !self.has_classifier_feature(classifier_id, *property_id)?
            {
                return Err(ModelError::InvalidConnectorPath(*property_id));
            }
            let type_id = property
                .type_id
                .ok_or(ModelError::TypeRequired(*property_id))?;
            let type_element = self.element(type_id)?;
            if !matches!(
                type_element.kind,
                ElementKind::Block | ElementKind::AssociationBlock | ElementKind::InterfaceBlock
            ) {
                return Err(ModelError::InvalidConnectorPath(*property_id));
            }
            classifier_id = type_id;
        }
        Ok(classifier_id)
    }

    pub fn validate_connector_end(
        &self,
        context_id: ElementId,
        end: &ConnectorEnd,
    ) -> Result<(), ModelError> {
        let reached_classifier = self.resolve_structural_path(context_id, &end.property_path)?;

        if let Some(port_id) = end.port_id {
            let port = self.element(port_id)?;
            if !port.is_port() {
                return Err(ModelError::ConnectorEndpointMustBePortOrProperty(port_id));
            }
            if !self.has_classifier_feature(reached_classifier, port_id)? {
                return Err(ModelError::InvalidConnectorPath(port_id));
            }
            // Boundary ports have an empty role path and identify themselves as
            // the role; nested ports identify the owning property as the role.
            let expected_role = end.property_path.last().copied().unwrap_or(port_id);
            if end.role_id != expected_role {
                return Err(ModelError::InvalidConnectorPath(end.role_id));
            }
        } else {
            let role = self.element(end.role_id)?;
            if !matches!(
                role.kind,
                ElementKind::PartProperty | ElementKind::ReferenceProperty
            ) {
                return Err(ModelError::ConnectorEndpointMustBePortOrProperty(
                    end.role_id,
                ));
            }
            if end.property_path.last().copied() != Some(end.role_id) {
                return Err(ModelError::InvalidConnectorPath(end.role_id));
            }
        }
        Ok(())
    }

    /// Validate assembly/delegation topology independently of presentation.
    pub fn validate_connector(&self, connector: &Connector) -> Result<(), ModelError> {
        self.validate_connector_end(connector.context_id, &connector.source)?;
        self.validate_connector_end(connector.context_id, &connector.target)?;
        if connector.source == connector.target {
            return Err(ModelError::ConnectorSelfConnection);
        }

        let source_boundary = connector.source.property_path.is_empty();
        let target_boundary = connector.target.property_path.is_empty();
        match connector.kind {
            ConnectorKind::Assembly if source_boundary || target_boundary => {
                return Err(ModelError::AssemblyRequiresInternalEnds);
            }
            ConnectorKind::Delegation if source_boundary == target_boundary => {
                return Err(ModelError::DelegationRequiresBoundaryAndInternal);
            }
            _ => {}
        }
        self.validate_connector_compatibility(&connector.source, &connector.target)
    }

    /// Port compatibility foundation. The current SysML slice requires port
    /// types to be equal or related by generalization. Conjugation is applied
    /// to ProxyPorts; FullPorts cannot be conjugated. Untyped role-to-role
    /// connectors remain legal when both property types are compatible.
    pub fn validate_connector_compatibility(
        &self,
        source: &ConnectorEnd,
        target: &ConnectorEnd,
    ) -> Result<(), ModelError> {
        let source_element = self.element(source.port_id.unwrap_or(source.role_id))?;
        let target_element = self.element(target.port_id.unwrap_or(target.role_id))?;
        let Some(source_type) = source_element.type_id else {
            return Err(ModelError::TypeRequired(source_element.id));
        };
        let Some(target_type) = target_element.type_id else {
            return Err(ModelError::TypeRequired(target_element.id));
        };
        let compatible = source_type == target_type
            || self.is_generalization_related(source_type, target_type)
            || self.is_generalization_related(target_type, source_type);
        if !compatible {
            return Err(ModelError::IncompatibleConnectorTypes {
                source_id: source_type,
                target_id: target_type,
            });
        }
        if source_element.kind == ElementKind::FullPort && source_element.is_conjugated {
            return Err(ModelError::FullPortCannotBeConjugated(source_element.id));
        }
        if target_element.kind == ElementKind::FullPort && target_element.is_conjugated {
            return Err(ModelError::FullPortCannotBeConjugated(target_element.id));
        }
        Ok(())
    }

    pub fn validate_item_flow(&self, flow: &ItemFlow) -> Result<(), ModelError> {
        if flow.conveyed_item_ids.is_empty() {
            return Err(ModelError::ItemFlowRequiresConveyedItem);
        }
        let mut conveyed = std::collections::HashSet::new();
        for item_id in &flow.conveyed_item_ids {
            if !conveyed.insert(*item_id) {
                return Err(ModelError::DuplicateConveyedItem(*item_id));
            }
            let item = self.element(*item_id)?;
            if !item.is_classifier() {
                return Err(ModelError::InvalidConveyedItem(*item_id));
            }
        }
        let relationship = self.relationship(flow.connector_id)?;
        let connector = relationship
            .connector
            .as_ref()
            .ok_or(ModelError::RelationshipIsNotConnector(flow.connector_id))?;
        self.validate_connector_end(connector.context_id, &flow.source)?;
        self.validate_connector_end(connector.context_id, &flow.target)?;
        if !((flow.source == connector.source && flow.target == connector.target)
            || (flow.source == connector.target && flow.target == connector.source))
        {
            return Err(ModelError::ItemFlowEndpointsDoNotMatchConnector(
                flow.connector_id,
            ));
        }
        Ok(())
    }

    fn is_generalization_related(&self, specific: ElementId, general: ElementId) -> bool {
        let mut stack = vec![specific];
        let mut visited = std::collections::HashSet::new();
        while let Some(current) = stack.pop() {
            if !visited.insert(current) {
                continue;
            }
            if current == general {
                return true;
            }
            for relationship in self.relationships.values().filter(|relationship| {
                relationship.kind == crate::RelationshipKind::Generalization
                    && relationship.source_id == current
            }) {
                stack.push(relationship.target_id);
            }
        }
        false
    }
}
