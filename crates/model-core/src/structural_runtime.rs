use crate::execution::validate_runtime_assignment;
use crate::{
    ConnectorEnd, ConnectorKind, DiagnosticSeverity, Element, ElementId, ElementKind,
    FlowDirection, Multiplicity, Project, RelationshipId, RelationshipKind, RuntimeInstance,
    RuntimeInstanceId, RuntimeValue, RuntimeValueKey, evaluate_execution_expression,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePopulationDecision {
    /// Restricts a decision to one occurrence of a classifier. `None` applies
    /// to every occurrence that owns the same semantic PartProperty.
    pub owner_runtime_path: Option<String>,
    pub part_property_id: ElementId,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeReferenceBindingDecision {
    pub owner_runtime_path: String,
    pub reference_property_id: ElementId,
    pub target_runtime_paths: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralRuntimeConfiguration {
    /// Optional engineer-facing name for a Block root occurrence. A Block
    /// remains a classifier; this names the configured root occurrence only.
    pub root_instance_name: Option<String>,
    #[serde(default)]
    pub populations: Vec<RuntimePopulationDecision>,
    #[serde(default)]
    pub reference_bindings: Vec<RuntimeReferenceBindingDecision>,
    /// Additional authored InstanceSpecifications that exist outside the
    /// composite root but can be reference targets.
    #[serde(default)]
    pub configured_instance_specification_ids: Vec<ElementId>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionRuntimeSelection {
    /// Optional structural execution root. `None` preserves the behavior's
    /// existing context root.
    pub root_semantic_id: Option<ElementId>,
    #[serde(default)]
    pub structural_configuration: StructuralRuntimeConfiguration,
    /// Qualified runtime occurrence path. Required when the behavior classifier
    /// appears more than once below the selected structural root.
    pub runtime_instance_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionRuntimePreview {
    pub root_semantic_id: ElementId,
    pub structural_runtime: Option<StructuralRuntimeSnapshot>,
    pub compatible_runtime_instance_paths: Vec<String>,
    pub selected_runtime_instance_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuntimePortKey {
    pub instance_id: RuntimeInstanceId,
    pub semantic_port_id: ElementId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimePortKind {
    Proxy,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeFlowContract {
    pub flow_property_id: ElementId,
    pub name: String,
    pub direction: FlowDirection,
    pub type_id: ElementId,
    pub type_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePort {
    pub key: RuntimePortKey,
    pub owner_instance_id: RuntimeInstanceId,
    pub semantic_port_id: ElementId,
    pub kind: RuntimePortKind,
    pub type_id: ElementId,
    pub type_name: String,
    pub is_conjugated: bool,
    pub qualified_path: String,
    pub flow_contracts: Vec<RuntimeFlowContract>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeReferenceBinding {
    pub owner_instance_id: RuntimeInstanceId,
    pub reference_property_id: ElementId,
    pub target_instance_ids: Vec<RuntimeInstanceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeEndpoint {
    pub instance_id: RuntimeInstanceId,
    pub semantic_port_id: Option<ElementId>,
    pub property_path: Vec<ElementId>,
    pub qualified_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeItemFlow {
    pub item_flow_id: RelationshipId,
    pub conveyed_item_ids: Vec<ElementId>,
    /// True when the authored ItemFlow source is the Connector source.
    pub connector_source_to_target: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeConnectorLink {
    pub id: Uuid,
    pub semantic_connector_id: RelationshipId,
    pub context_instance_id: RuntimeInstanceId,
    pub kind: ConnectorKind,
    pub source: RuntimeEndpoint,
    pub target: RuntimeEndpoint,
    pub item_flows: Vec<RuntimeItemFlow>,
    pub route_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralRuntimeDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub semantic_element_id: Option<ElementId>,
    pub runtime_path: Option<String>,
    pub message: String,
    pub expected: Option<String>,
    pub found: Option<String>,
    pub remedy: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeValueDefinition {
    pub semantic_property_id: ElementId,
    pub name: String,
    pub type_id: Option<ElementId>,
    pub type_name: Option<String>,
    pub unit_symbol: Option<String>,
}

/// JSON-safe, deterministically ordered view of the transient runtime. The
/// authoritative runtime keeps keyed maps for routing performance; this DTO
/// deliberately exposes arrays because JSON object keys cannot represent the
/// composite RuntimePortKey/RuntimeValueKey types without lossy conversion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuralRuntimeSnapshot {
    pub root_instance_ids: Vec<RuntimeInstanceId>,
    pub instances: Vec<RuntimeInstance>,
    pub ports: Vec<RuntimePort>,
    pub value_definitions: Vec<RuntimeValueDefinition>,
    pub references: Vec<RuntimeReferenceBinding>,
    pub connector_links: Vec<RuntimeConnectorLink>,
    pub diagnostics: Vec<StructuralRuntimeDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructuralRuntime {
    pub root_instance_ids: Vec<RuntimeInstanceId>,
    pub instances: HashMap<RuntimeInstanceId, RuntimeInstance>,
    pub ports: HashMap<RuntimePortKey, RuntimePort>,
    pub value_definitions: Vec<RuntimeValueDefinition>,
    pub references: Vec<RuntimeReferenceBinding>,
    pub connector_links: Vec<RuntimeConnectorLink>,
    pub initial_values: HashMap<RuntimeValueKey, RuntimeValue>,
    pub diagnostics: Vec<StructuralRuntimeDiagnostic>,
    path_index: HashMap<String, RuntimeInstanceId>,
    usage_index: HashMap<ElementId, Vec<RuntimeInstanceId>>,
    classifier_index: HashMap<ElementId, Vec<RuntimeInstanceId>>,
    outgoing_links: HashMap<RuntimePortKey, Vec<usize>>,
    incoming_links: HashMap<RuntimePortKey, Vec<usize>>,
}

impl StructuralRuntime {
    pub fn build(
        project: &Project,
        root_semantic_id: ElementId,
        configuration: &StructuralRuntimeConfiguration,
    ) -> Result<Self, StructuralRuntimeError> {
        StructuralRuntimeBuilder::new(project, configuration).build(root_semantic_id)
    }

    pub fn instance_by_path(&self, path: &str) -> Option<&RuntimeInstance> {
        self.path_index
            .get(path)
            .and_then(|id| self.instances.get(id))
    }

    pub fn instances_for_usage(&self, usage_id: ElementId) -> Vec<&RuntimeInstance> {
        self.usage_index
            .get(&usage_id)
            .into_iter()
            .flatten()
            .filter_map(|id| self.instances.get(id))
            .collect()
    }

    pub fn instances_for_classifier(&self, classifier_id: ElementId) -> Vec<&RuntimeInstance> {
        self.classifier_index
            .get(&classifier_id)
            .into_iter()
            .flatten()
            .filter_map(|id| self.instances.get(id))
            .collect()
    }

    pub fn root_instance_id(&self) -> Option<RuntimeInstanceId> {
        self.root_instance_ids.first().copied()
    }

    pub fn instance_conforms_to(
        &self,
        project: &Project,
        instance_id: RuntimeInstanceId,
        expected_classifier_id: ElementId,
    ) -> bool {
        self.instances
            .get(&instance_id)
            .and_then(|instance| instance.classifier_id)
            .is_some_and(|actual| classifier_conforms(project, actual, expected_classifier_id))
    }

    pub fn compatible_instance_paths(
        &self,
        project: &Project,
        expected_classifier_id: ElementId,
    ) -> Vec<String> {
        let mut paths: Vec<_> = self
            .instances
            .values()
            .filter(|instance| {
                self.instance_conforms_to(project, instance.id, expected_classifier_id)
            })
            .map(|instance| instance.qualified_path.clone())
            .collect();
        paths.sort();
        paths.dedup();
        paths
    }

    pub fn snapshot(&self) -> StructuralRuntimeSnapshot {
        let mut instances: Vec<_> = self.instances.values().cloned().collect();
        instances.sort_by(|left, right| left.qualified_path.cmp(&right.qualified_path));

        let mut ports: Vec<_> = self.ports.values().cloned().collect();
        ports.sort_by(|left, right| left.qualified_path.cmp(&right.qualified_path));

        StructuralRuntimeSnapshot {
            root_instance_ids: self.root_instance_ids.clone(),
            instances,
            ports,
            value_definitions: self.value_definitions.clone(),
            references: self.references.clone(),
            connector_links: self.connector_links.clone(),
            diagnostics: self.diagnostics.clone(),
        }
    }

    pub fn port(
        &self,
        instance_id: RuntimeInstanceId,
        semantic_port_id: ElementId,
    ) -> Option<&RuntimePort> {
        self.ports.get(&RuntimePortKey {
            instance_id,
            semantic_port_id,
        })
    }

    pub fn outgoing_links(
        &self,
        instance_id: RuntimeInstanceId,
        semantic_port_id: ElementId,
    ) -> Vec<&RuntimeConnectorLink> {
        self.outgoing_links
            .get(&RuntimePortKey {
                instance_id,
                semantic_port_id,
            })
            .into_iter()
            .flatten()
            .filter_map(|index| self.connector_links.get(*index))
            .collect()
    }

    pub fn incoming_links(
        &self,
        instance_id: RuntimeInstanceId,
        semantic_port_id: ElementId,
    ) -> Vec<&RuntimeConnectorLink> {
        self.incoming_links
            .get(&RuntimePortKey {
                instance_id,
                semantic_port_id,
            })
            .into_iter()
            .flatten()
            .filter_map(|index| self.connector_links.get(*index))
            .collect()
    }

    /// Resolves every structurally connected receiver in deterministic runtime
    /// path order. ItemFlow semantics constrain transport when authored.
    pub fn signal_destinations(
        &self,
        project: &Project,
        source_instance_id: RuntimeInstanceId,
        source_port_id: ElementId,
        signal_id: ElementId,
    ) -> Result<Vec<RuntimeEndpoint>, StructuralRuntimeError> {
        let signal =
            project
                .element(signal_id)
                .map_err(|_| StructuralRuntimeError::InvalidSignal {
                    signal: readable_element(project, signal_id),
                })?;
        if signal.kind != ElementKind::Signal {
            return Err(StructuralRuntimeError::InvalidSignal {
                signal: readable_element(project, signal_id),
            });
        }
        let key = RuntimePortKey {
            instance_id: source_instance_id,
            semantic_port_id: source_port_id,
        };
        let source_port =
            self.ports
                .get(&key)
                .ok_or_else(|| StructuralRuntimeError::RuntimePortNotFound {
                    path: self
                        .instances
                        .get(&source_instance_id)
                        .map(|instance| instance.qualified_path.clone())
                        .unwrap_or_else(|| source_instance_id.to_string()),
                    port: readable_element(project, source_port_id),
                })?;
        let mut destinations = Vec::new();
        for index in self.outgoing_links.get(&key).into_iter().flatten().copied() {
            let link = &self.connector_links[index];
            let (destination, forward) = if endpoint_key(&link.source) == Some(key) {
                (&link.target, true)
            } else {
                (&link.source, false)
            };
            if !link_allows_item(project, link, signal_id, forward) {
                continue;
            }
            validate_source_flow(project, source_port, signal_id)?;
            if let Some(port_id) = destination.semantic_port_id {
                let target_port = self.port(destination.instance_id, port_id).ok_or_else(|| {
                    StructuralRuntimeError::RuntimePortNotFound {
                        path: destination.qualified_path.clone(),
                        port: readable_element(project, port_id),
                    }
                })?;
                validate_target_flow(project, target_port, signal_id)?;
            }
            validate_reception(project, self, destination, signal_id)?;
            destinations.push(destination.clone());
        }
        destinations.sort_by(|left, right| left.qualified_path.cmp(&right.qualified_path));
        destinations.dedup_by(|left, right| {
            left.instance_id == right.instance_id && left.semantic_port_id == right.semantic_port_id
        });
        if destinations.is_empty() {
            return Err(StructuralRuntimeError::NoSignalRoute {
                signal: signal.name.clone(),
                source_path: source_port.qualified_path.clone(),
            });
        }
        Ok(destinations)
    }

    /// Returns the exact structurally valid source-Port/destination pairs for
    /// one intended runtime receiver. This lets Sequence semantics select a
    /// modeled participant without treating same-typed occurrences as a
    /// broadcast group.
    pub fn signal_routes_between(
        &self,
        project: &Project,
        source_instance_id: RuntimeInstanceId,
        target_instance_id: RuntimeInstanceId,
        signal_id: ElementId,
    ) -> Vec<(ElementId, RuntimeEndpoint)> {
        let mut source_ports: Vec<_> = self
            .ports
            .keys()
            .filter(|key| key.instance_id == source_instance_id)
            .map(|key| key.semantic_port_id)
            .collect();
        source_ports.sort_by_key(ToString::to_string);
        source_ports.dedup();
        let mut routes = Vec::new();
        for source_port_id in source_ports {
            let Ok(destinations) =
                self.signal_destinations(project, source_instance_id, source_port_id, signal_id)
            else {
                continue;
            };
            routes.extend(
                destinations
                    .into_iter()
                    .filter(|destination| destination.instance_id == target_instance_id)
                    .map(|destination| (source_port_id, destination)),
            );
        }
        routes.sort_by(|left, right| {
            left.0
                .to_string()
                .cmp(&right.0.to_string())
                .then_with(|| left.1.qualified_path.cmp(&right.1.qualified_path))
        });
        routes.dedup_by(|left, right| {
            left.0 == right.0
                && left.1.instance_id == right.1.instance_id
                && left.1.semantic_port_id == right.1.semantic_port_id
        });
        routes
    }

    pub fn signal_source_ports(
        &self,
        project: &Project,
        source_instance_id: RuntimeInstanceId,
        signal_id: ElementId,
    ) -> Vec<ElementId> {
        let mut source_ports: Vec<_> = self
            .ports
            .keys()
            .filter(|key| key.instance_id == source_instance_id)
            .filter_map(|key| {
                self.signal_destinations(
                    project,
                    source_instance_id,
                    key.semantic_port_id,
                    signal_id,
                )
                .is_ok()
                .then_some(key.semantic_port_id)
            })
            .collect();
        source_ports.sort_by_key(ToString::to_string);
        source_ports.dedup();
        source_ports
    }

    fn rebuild_indices(&mut self) {
        self.path_index.clear();
        self.usage_index.clear();
        self.classifier_index.clear();
        for instance in self.instances.values() {
            self.path_index
                .insert(instance.qualified_path.clone(), instance.id);
            if let Some(usage_id) = instance.semantic_usage_id {
                self.usage_index
                    .entry(usage_id)
                    .or_default()
                    .push(instance.id);
            }
            if let Some(classifier_id) = instance.classifier_id {
                self.classifier_index
                    .entry(classifier_id)
                    .or_default()
                    .push(instance.id);
            }
        }
        for ids in self.usage_index.values_mut() {
            ids.sort_by_key(|id| self.instances[id].qualified_path.clone());
        }
        for ids in self.classifier_index.values_mut() {
            ids.sort_by_key(|id| self.instances[id].qualified_path.clone());
        }
        self.outgoing_links.clear();
        self.incoming_links.clear();
        for (index, link) in self.connector_links.iter().enumerate() {
            if let Some(key) = endpoint_key(&link.source) {
                self.outgoing_links.entry(key).or_default().push(index);
                self.incoming_links.entry(key).or_default().push(index);
            }
            if let Some(key) = endpoint_key(&link.target) {
                self.outgoing_links.entry(key).or_default().push(index);
                self.incoming_links.entry(key).or_default().push(index);
            }
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum StructuralRuntimeError {
    #[error(
        "Cannot construct structural runtime from {element}. Select a Block, PartProperty, or typed InstanceSpecification as the execution root."
    )]
    InvalidRoot { element: String },
    #[error("{property} has no Block type. Assign a compatible Block classifier before execution.")]
    MissingPartType { property: String },
    #[error(
        "{property} population {count} violates authored multiplicity {multiplicity}. Choose a count inside the authored range."
    )]
    PopulationOutsideMultiplicity {
        property: String,
        count: u32,
        multiplicity: String,
    },
    #[error(
        "{path} -> recursive composite instantiation. Remove or replace a PartProperty in this ownership cycle."
    )]
    RecursiveComposition { path: String },
    #[error(
        "Runtime occurrence path '{path}' is defined more than once. Give configured/root occurrences unique names or remove the conflicting configuration."
    )]
    DuplicateRuntimePath { path: String },
    #[error(
        "Runtime identity collision at '{path}' ({runtime_id}). Runtime construction stopped rather than aliasing two engineering occurrences."
    )]
    RuntimeIdentityCollision {
        path: String,
        runtime_id: RuntimeInstanceId,
    },
    #[error(
        "PartProperty {property} at {owner_path} has multiple equally scoped population decisions. Keep only one population decision for this occurrence."
    )]
    DuplicatePopulationDecision {
        property: String,
        owner_path: String,
    },
    #[error(
        "ReferenceProperty {reference} at {owner_path} has multiple runtime binding decisions. Keep exactly one binding decision for this occurrence/reference pair."
    )]
    DuplicateReferenceBindingDecision {
        reference: String,
        owner_path: String,
    },
    #[error(
        "{classifier} inherits conflicting structural features named '{feature}'. The current metamodel has no explicit redefinition/subsetting link to disambiguate them."
    )]
    AmbiguousInheritedFeature { classifier: String, feature: String },
    #[error(
        "{port} is a ProxyPort typed by {found}. ProxyPorts require an InterfaceBlock type in the supported SysML profile. Select or create an InterfaceBlock describing the exposed contract."
    )]
    ProxyPortRequiresInterfaceBlock { port: String, found: String },
    #[error(
        "Required reference {reference} at {owner_path} is unresolved. Configure one or more existing compatible runtime target paths; ReferenceProperty does not create owned structure."
    )]
    RequiredReferenceUnresolved {
        reference: String,
        owner_path: String,
    },
    #[error(
        "Reference {reference} at {owner_path} has {count} target(s), outside multiplicity {multiplicity}. Update the explicit reference binding."
    )]
    ReferenceMultiplicity {
        reference: String,
        owner_path: String,
        count: usize,
        multiplicity: String,
    },
    #[error(
        "Reference {reference} at {owner_path} targets unknown runtime path '{target_path}'. Bind it to an existing configured or composite occurrence."
    )]
    ReferenceTargetNotFound {
        reference: String,
        owner_path: String,
        target_path: String,
    },
    #[error(
        "Reference {reference} at {owner_path} expects {expected}, but target {target_path} is {found}. Select a compatible runtime occurrence."
    )]
    ReferenceTypeMismatch {
        reference: String,
        owner_path: String,
        expected: String,
        target_path: String,
        found: String,
    },
    #[error("Connector {connector} endpoint path is invalid at runtime: {details}")]
    InvalidConnectorEndpoint { connector: String, details: String },
    #[error(
        "Connector {connector} has no runtime endpoints in context {context}. Check PartProperty population and ReferenceProperty bindings."
    )]
    MissingConnectorEndpoint { connector: String, context: String },
    #[error(
        "{path} does not contain runtime port {port}. Check the semantic nested property path and port owner."
    )]
    RuntimePortNotFound { path: String, port: String },
    #[error(
        "ItemFlow {item_flow} conveys {conveyed}, but {port} has no compatible FlowProperty. Add a compatible typed FlowProperty or correct the conveyed item."
    )]
    ItemFlowTypeMismatch {
        item_flow: String,
        conveyed: String,
        port: String,
    },
    #[error(
        "{port} cannot supply {conveyed}; its compatible FlowProperty direction is not out/inout after conjugation."
    )]
    SourceFlowDirection { port: String, conveyed: String },
    #[error(
        "{port} cannot receive {conveyed}; its compatible FlowProperty direction is not in/inout after conjugation."
    )]
    TargetFlowDirection { port: String, conveyed: String },
    #[error("{signal} is not a modeled Signal and cannot be transported structurally.")]
    InvalidSignal { signal: String },
    #[error(
        "No compatible structural route carries Signal '{signal}' from {source_path}. Check Connector, ItemFlow, FlowProperty, and Reception semantics."
    )]
    NoSignalRoute { signal: String, source_path: String },
    #[error(
        "{target} has modeled Receptions, but none accepts Signal {signal}. Add a compatible Reception or correct the route."
    )]
    ReceptionMismatch { target: String, signal: String },
    #[error(
        "Configured element {element} is not a typed InstanceSpecification. Select a modeled InstanceSpecification with a Block classifier."
    )]
    InvalidConfiguredInstance { element: String },
    #[error("Authored default for {property} is incompatible: {details}")]
    InvalidDefault { property: String, details: String },
    #[error("Structural model validation failed: {0}")]
    Model(String),
}

struct StructuralRuntimeBuilder<'a> {
    project: &'a Project,
    configuration: &'a StructuralRuntimeConfiguration,
    runtime: StructuralRuntime,
    children_by_usage: HashMap<(RuntimeInstanceId, ElementId), Vec<RuntimeInstanceId>>,
}

impl<'a> StructuralRuntimeBuilder<'a> {
    fn new(project: &'a Project, configuration: &'a StructuralRuntimeConfiguration) -> Self {
        Self {
            project,
            configuration,
            runtime: StructuralRuntime {
                root_instance_ids: Vec::new(),
                instances: HashMap::new(),
                ports: HashMap::new(),
                value_definitions: Vec::new(),
                references: Vec::new(),
                connector_links: Vec::new(),
                initial_values: HashMap::new(),
                diagnostics: Vec::new(),
                path_index: HashMap::new(),
                usage_index: HashMap::new(),
                classifier_index: HashMap::new(),
                outgoing_links: HashMap::new(),
                incoming_links: HashMap::new(),
            },
            children_by_usage: HashMap::new(),
        }
    }

    fn build(
        mut self,
        root_semantic_id: ElementId,
    ) -> Result<StructuralRuntime, StructuralRuntimeError> {
        self.project
            .validate()
            .map_err(|error| StructuralRuntimeError::Model(error.to_string()))?;
        let root = self.project.element(root_semantic_id).map_err(|_| {
            StructuralRuntimeError::InvalidRoot {
                element: readable_element(self.project, root_semantic_id),
            }
        })?;
        let (usage_id, classifier_id, authored_instance_id, default_name) = match root.kind {
            ElementKind::Block | ElementKind::AssociationBlock => {
                (None, root.id, None, root.name.clone())
            }
            ElementKind::PartProperty => (
                Some(root.id),
                root.type_id
                    .ok_or_else(|| StructuralRuntimeError::MissingPartType {
                        property: readable_element(self.project, root.id),
                    })?,
                None,
                root.name.clone(),
            ),
            ElementKind::InstanceSpecification => (
                None,
                root.type_id
                    .ok_or_else(|| StructuralRuntimeError::InvalidConfiguredInstance {
                        element: readable_element(self.project, root.id),
                    })?,
                Some(root.id),
                root.name.clone(),
            ),
            _ => {
                return Err(StructuralRuntimeError::InvalidRoot {
                    element: readable_element(self.project, root.id),
                });
            }
        };
        let root_name = self
            .configuration
            .root_instance_name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(&default_name)
            .to_string();
        let root_id = self.instantiate(
            usage_id,
            classifier_id,
            None,
            root_name,
            0,
            authored_instance_id,
            &[],
        )?;
        self.runtime.root_instance_ids.push(root_id);

        let mut configured = self
            .configuration
            .configured_instance_specification_ids
            .clone();
        configured.sort_by_key(ToString::to_string);
        configured.dedup();
        for instance_specification_id in configured {
            if instance_specification_id == root_semantic_id {
                continue;
            }
            let element = self
                .project
                .element(instance_specification_id)
                .map_err(|_| StructuralRuntimeError::InvalidConfiguredInstance {
                    element: readable_element(self.project, instance_specification_id),
                })?;
            if element.kind != ElementKind::InstanceSpecification {
                return Err(StructuralRuntimeError::InvalidConfiguredInstance {
                    element: readable_element(self.project, element.id),
                });
            }
            let classifier_id = element.type_id.ok_or_else(|| {
                StructuralRuntimeError::InvalidConfiguredInstance {
                    element: readable_element(self.project, element.id),
                }
            })?;
            let id = self.instantiate(
                None,
                classifier_id,
                None,
                element.name.clone(),
                0,
                Some(element.id),
                &[],
            )?;
            self.runtime.root_instance_ids.push(id);
        }

        self.runtime.rebuild_indices();
        self.resolve_references()?;
        self.build_connectors()?;
        self.runtime.rebuild_indices();
        self.runtime
            .root_instance_ids
            .sort_by_key(|id| self.runtime.instances[id].qualified_path.clone());
        self.runtime.connector_links.sort_by(|left, right| {
            left.source
                .qualified_path
                .cmp(&right.source.qualified_path)
                .then_with(|| left.target.qualified_path.cmp(&right.target.qualified_path))
                .then_with(|| {
                    left.semantic_connector_id
                        .to_string()
                        .cmp(&right.semantic_connector_id.to_string())
                })
        });
        self.runtime.rebuild_indices();
        Ok(self.runtime)
    }

    #[allow(clippy::too_many_arguments)]
    fn instantiate(
        &mut self,
        usage_id: Option<ElementId>,
        classifier_id: ElementId,
        owner_instance_id: Option<RuntimeInstanceId>,
        qualified_path: String,
        ordinal: u32,
        authored_instance_specification_id: Option<ElementId>,
        classifier_stack: &[ElementId],
    ) -> Result<RuntimeInstanceId, StructuralRuntimeError> {
        let classifier = self.project.element(classifier_id).map_err(|_| {
            StructuralRuntimeError::MissingPartType {
                property: usage_id
                    .map(|id| readable_element(self.project, id))
                    .unwrap_or_else(|| readable_element(self.project, classifier_id)),
            }
        })?;
        if !matches!(
            classifier.kind,
            ElementKind::Block | ElementKind::AssociationBlock
        ) {
            return Err(StructuralRuntimeError::MissingPartType {
                property: usage_id
                    .map(|id| readable_element(self.project, id))
                    .unwrap_or_else(|| readable_element(self.project, classifier_id)),
            });
        }
        if classifier_stack.contains(&classifier_id) {
            let mut path = classifier_stack
                .iter()
                .map(|id| {
                    self.project
                        .element(*id)
                        .map(|e| e.name.clone())
                        .unwrap_or_else(|_| id.to_string())
                })
                .collect::<Vec<_>>();
            path.push(classifier.name.clone());
            return Err(StructuralRuntimeError::RecursiveComposition {
                path: format!("{qualified_path} ({})", path.join(" -> ")),
            });
        }
        let semantic_element_id = usage_id
            .or(authored_instance_specification_id)
            .unwrap_or(classifier_id);
        if self
            .runtime
            .instances
            .values()
            .any(|instance| instance.qualified_path == qualified_path)
        {
            return Err(StructuralRuntimeError::DuplicateRuntimePath {
                path: qualified_path.clone(),
            });
        }
        let id = deterministic_instance_id(
            self.project,
            semantic_element_id,
            classifier_id,
            &qualified_path,
            ordinal,
        );
        if self.runtime.instances.contains_key(&id) {
            return Err(StructuralRuntimeError::RuntimeIdentityCollision {
                path: qualified_path.clone(),
                runtime_id: id,
            });
        }
        let name = usage_id
            .and_then(|id| self.project.element(id).ok())
            .or_else(|| {
                authored_instance_specification_id.and_then(|id| self.project.element(id).ok())
            })
            .unwrap_or(classifier)
            .name
            .clone();
        self.runtime.instances.insert(
            id,
            RuntimeInstance {
                id,
                semantic_element_id,
                semantic_usage_id: usage_id,
                classifier_id: Some(classifier_id),
                classifier_name: classifier.name.clone(),
                owner_runtime_instance_id: owner_instance_id,
                qualified_path: qualified_path.clone(),
                ordinal,
                authored_instance_specification_id,
                name,
            },
        );
        self.initialize_values_and_ports(id, classifier_id)?;

        let mut next_stack = classifier_stack.to_vec();
        next_stack.push(classifier_id);
        let parts = self.effective_features(classifier_id, ElementKind::PartProperty)?;
        for part in parts {
            let part_type =
                part.type_id
                    .ok_or_else(|| StructuralRuntimeError::MissingPartType {
                        property: readable_element(self.project, part.id),
                    })?;
            let multiplicity = part.multiplicity.unwrap_or(Multiplicity::ONE);
            let count = self.population(&qualified_path, part, multiplicity)?;
            let mut child_ids = Vec::new();
            for child_ordinal in 0..count {
                let segment = if count > 1 {
                    format!("{}[{child_ordinal}]", part.name)
                } else {
                    part.name.clone()
                };
                let child_path = format!("{qualified_path}.{segment}");
                let child_id = self.instantiate(
                    Some(part.id),
                    part_type,
                    Some(id),
                    child_path,
                    child_ordinal,
                    None,
                    &next_stack,
                )?;
                child_ids.push(child_id);
            }
            self.children_by_usage.insert((id, part.id), child_ids);
        }
        Ok(id)
    }

    fn population(
        &self,
        owner_path: &str,
        part: &Element,
        multiplicity: Multiplicity,
    ) -> Result<u32, StructuralRuntimeError> {
        let exact: Vec<_> = self
            .configuration
            .populations
            .iter()
            .filter(|decision| {
                decision.part_property_id == part.id
                    && decision.owner_runtime_path.as_deref() == Some(owner_path)
            })
            .collect();
        let generic: Vec<_> = self
            .configuration
            .populations
            .iter()
            .filter(|decision| {
                decision.part_property_id == part.id && decision.owner_runtime_path.is_none()
            })
            .collect();
        if exact.len() > 1 || (exact.is_empty() && generic.len() > 1) {
            return Err(StructuralRuntimeError::DuplicatePopulationDecision {
                property: readable_element(self.project, part.id),
                owner_path: owner_path.to_string(),
            });
        }
        let count = exact
            .first()
            .or_else(|| generic.first())
            .map(|decision| decision.count)
            .unwrap_or(multiplicity.lower);
        if count < multiplicity.lower || multiplicity.upper.is_some_and(|upper| count > upper) {
            return Err(StructuralRuntimeError::PopulationOutsideMultiplicity {
                property: readable_element(self.project, part.id),
                count,
                multiplicity: multiplicity.notation(),
            });
        }
        Ok(count)
    }

    fn initialize_values_and_ports(
        &mut self,
        instance_id: RuntimeInstanceId,
        classifier_id: ElementId,
    ) -> Result<(), StructuralRuntimeError> {
        for property in self.effective_features(classifier_id, ElementKind::ValueProperty)? {
            if !self
                .runtime
                .value_definitions
                .iter()
                .any(|definition| definition.semantic_property_id == property.id)
            {
                let property_type = property
                    .type_id
                    .and_then(|type_id| self.project.element(type_id).ok());
                let unit_symbol = property_type
                    .and_then(|element| element.unit_external_id.as_deref())
                    .and_then(|external_id| {
                        self.project.elements.values().find(|candidate| {
                            candidate.external_id == external_id
                                && candidate.kind == ElementKind::Unit
                        })
                    })
                    .and_then(|unit| unit.unit_symbol.clone());
                self.runtime.value_definitions.push(RuntimeValueDefinition {
                    semantic_property_id: property.id,
                    name: property.name.clone(),
                    type_id: property.type_id,
                    type_name: property_type.map(|element| element.name.clone()),
                    unit_symbol,
                });
            }
            if let Some(authored) = property.default_value.as_deref() {
                let value = parse_authored_runtime_default(authored);
                validate_runtime_assignment(self.project, property, &value).map_err(|error| {
                    StructuralRuntimeError::InvalidDefault {
                        property: readable_element(self.project, property.id),
                        details: error.to_string(),
                    }
                })?;
                self.runtime.initial_values.insert(
                    RuntimeValueKey {
                        instance_id: Some(instance_id),
                        semantic_element_id: property.id,
                    },
                    value,
                );
            }
        }
        for port in self
            .effective_features(classifier_id, ElementKind::ProxyPort)?
            .into_iter()
            .chain(self.effective_features(classifier_id, ElementKind::FullPort)?)
        {
            let type_id = port.type_id.ok_or_else(|| {
                StructuralRuntimeError::ProxyPortRequiresInterfaceBlock {
                    port: readable_element(self.project, port.id),
                    found: "no type".into(),
                }
            })?;
            let port_type = self.project.element(type_id).map_err(|_| {
                StructuralRuntimeError::ProxyPortRequiresInterfaceBlock {
                    port: readable_element(self.project, port.id),
                    found: readable_element(self.project, type_id),
                }
            })?;
            let kind = match port.kind {
                ElementKind::ProxyPort => {
                    if port_type.kind != ElementKind::InterfaceBlock {
                        return Err(StructuralRuntimeError::ProxyPortRequiresInterfaceBlock {
                            port: readable_element(self.project, port.id),
                            found: readable_element(self.project, type_id),
                        });
                    }
                    RuntimePortKind::Proxy
                }
                ElementKind::FullPort => RuntimePortKind::Full,
                _ => unreachable!("filtered to runtime ports"),
            };
            let instance_path = self.runtime.instances[&instance_id].qualified_path.clone();
            let mut flow_contracts = Vec::new();
            for flow in self.effective_features(type_id, ElementKind::FlowProperty)? {
                if let (Some(direction), Some(flow_type)) = (flow.flow_direction, flow.type_id) {
                    flow_contracts.push(RuntimeFlowContract {
                        flow_property_id: flow.id,
                        name: flow.name.clone(),
                        direction,
                        type_id: flow_type,
                        type_name: readable_element_name(self.project, flow_type),
                    });
                }
            }
            flow_contracts.sort_by(|left, right| {
                left.name.cmp(&right.name).then_with(|| {
                    left.flow_property_id
                        .to_string()
                        .cmp(&right.flow_property_id.to_string())
                })
            });
            let key = RuntimePortKey {
                instance_id,
                semantic_port_id: port.id,
            };
            self.runtime.ports.insert(
                key,
                RuntimePort {
                    key,
                    owner_instance_id: instance_id,
                    semantic_port_id: port.id,
                    kind,
                    type_id,
                    type_name: port_type.name.clone(),
                    is_conjugated: port.is_conjugated,
                    qualified_path: format!("{instance_path}.{}", port.name),
                    flow_contracts,
                },
            );
        }
        self.runtime.value_definitions.sort_by(|left, right| {
            left.name.cmp(&right.name).then_with(|| {
                left.semantic_property_id
                    .to_string()
                    .cmp(&right.semantic_property_id.to_string())
            })
        });
        Ok(())
    }

    fn effective_features(
        &self,
        classifier_id: ElementId,
        kind: ElementKind,
    ) -> Result<Vec<&'a Element>, StructuralRuntimeError> {
        let mut classifiers = Vec::new();
        collect_general_classifiers(
            self.project,
            classifier_id,
            &mut HashSet::new(),
            &mut classifiers,
        );
        classifiers.push(classifier_id);
        let mut result = Vec::new();
        let mut names = HashMap::<String, ElementId>::new();
        for owner_id in classifiers {
            let mut owned: Vec<_> = self
                .project
                .elements
                .values()
                .filter(|element| element.owner_id == Some(owner_id) && element.kind == kind)
                .collect();
            owned.sort_by(|left, right| {
                left.name
                    .cmp(&right.name)
                    .then_with(|| left.id.to_string().cmp(&right.id.to_string()))
            });
            for feature in owned {
                if names.insert(feature.name.clone(), feature.id).is_some() {
                    return Err(StructuralRuntimeError::AmbiguousInheritedFeature {
                        classifier: readable_element(self.project, classifier_id),
                        feature: feature.name.clone(),
                    });
                }
                result.push(feature);
            }
        }
        Ok(result)
    }

    fn resolve_references(&mut self) -> Result<(), StructuralRuntimeError> {
        let mut instance_ids: Vec<_> = self.runtime.instances.keys().copied().collect();
        instance_ids.sort_by_key(|id| self.runtime.instances[id].qualified_path.clone());
        for owner_instance_id in instance_ids {
            let owner = self.runtime.instances[&owner_instance_id].clone();
            let classifier_id = owner.classifier_id.expect("structural instances are typed");
            for reference in
                self.effective_features(classifier_id, ElementKind::ReferenceProperty)?
            {
                let decisions: Vec<_> = self
                    .configuration
                    .reference_bindings
                    .iter()
                    .filter(|decision| {
                        decision.owner_runtime_path == owner.qualified_path
                            && decision.reference_property_id == reference.id
                    })
                    .collect();
                if decisions.len() > 1 {
                    return Err(StructuralRuntimeError::DuplicateReferenceBindingDecision {
                        reference: readable_element(self.project, reference.id),
                        owner_path: owner.qualified_path.clone(),
                    });
                }
                let decision = decisions.first().copied();
                let mut target_ids = Vec::new();
                if let Some(decision) = decision {
                    let mut target_paths = decision.target_runtime_paths.clone();
                    target_paths.sort();
                    target_paths.dedup();
                    for target_path in target_paths {
                        let target_id = self
                            .runtime
                            .path_index
                            .get(&target_path)
                            .copied()
                            .ok_or_else(|| StructuralRuntimeError::ReferenceTargetNotFound {
                                reference: readable_element(self.project, reference.id),
                                owner_path: owner.qualified_path.clone(),
                                target_path: target_path.clone(),
                            })?;
                        let target_classifier = self.runtime.instances[&target_id]
                            .classifier_id
                            .expect("structural instances are typed");
                        let expected = reference.type_id.ok_or_else(|| {
                            StructuralRuntimeError::MissingPartType {
                                property: readable_element(self.project, reference.id),
                            }
                        })?;
                        if !classifier_conforms(self.project, target_classifier, expected) {
                            return Err(StructuralRuntimeError::ReferenceTypeMismatch {
                                reference: readable_element(self.project, reference.id),
                                owner_path: owner.qualified_path.clone(),
                                expected: readable_element(self.project, expected),
                                target_path,
                                found: readable_element(self.project, target_classifier),
                            });
                        }
                        target_ids.push(target_id);
                    }
                }
                let multiplicity = reference.multiplicity.unwrap_or(Multiplicity::ONE);
                if target_ids.is_empty() && multiplicity.lower > 0 {
                    return Err(StructuralRuntimeError::RequiredReferenceUnresolved {
                        reference: readable_element(self.project, reference.id),
                        owner_path: owner.qualified_path.clone(),
                    });
                }
                if target_ids.len() < multiplicity.lower as usize
                    || multiplicity
                        .upper
                        .is_some_and(|upper| target_ids.len() > upper as usize)
                {
                    return Err(StructuralRuntimeError::ReferenceMultiplicity {
                        reference: readable_element(self.project, reference.id),
                        owner_path: owner.qualified_path.clone(),
                        count: target_ids.len(),
                        multiplicity: multiplicity.notation(),
                    });
                }
                target_ids.sort_by_key(|id| self.runtime.instances[id].qualified_path.clone());
                self.runtime.references.push(RuntimeReferenceBinding {
                    owner_instance_id,
                    reference_property_id: reference.id,
                    target_instance_ids: target_ids,
                });
            }
        }
        self.runtime.references.sort_by(|left, right| {
            self.runtime.instances[&left.owner_instance_id]
                .qualified_path
                .cmp(&self.runtime.instances[&right.owner_instance_id].qualified_path)
                .then_with(|| {
                    left.reference_property_id
                        .to_string()
                        .cmp(&right.reference_property_id.to_string())
                })
        });
        Ok(())
    }

    fn build_connectors(&mut self) -> Result<(), StructuralRuntimeError> {
        let mut connectors: Vec<_> = self
            .project
            .relationships
            .values()
            .filter(|relationship| relationship.kind == RelationshipKind::Connector)
            .collect();
        connectors.sort_by_key(|relationship| relationship.id.to_string());
        for relationship in connectors {
            let connector = relationship.connector.as_ref().ok_or_else(|| {
                StructuralRuntimeError::InvalidConnectorEndpoint {
                    connector: relationship.name.clone(),
                    details: "semantic Connector payload is missing".into(),
                }
            })?;
            let mut contexts: Vec<_> = self
                .runtime
                .instances
                .values()
                .filter(|instance| {
                    instance.classifier_id.is_some_and(|classifier_id| {
                        classifier_conforms(self.project, classifier_id, connector.context_id)
                    })
                })
                .map(|instance| instance.id)
                .collect();
            contexts.sort_by_key(|id| self.runtime.instances[id].qualified_path.clone());
            for context_id in contexts {
                let sources = self
                    .resolve_endpoint(context_id, &connector.source)
                    .map_err(|details| StructuralRuntimeError::InvalidConnectorEndpoint {
                        connector: relationship.name.clone(),
                        details,
                    })?;
                let targets = self
                    .resolve_endpoint(context_id, &connector.target)
                    .map_err(|details| StructuralRuntimeError::InvalidConnectorEndpoint {
                        connector: relationship.name.clone(),
                        details,
                    })?;
                if sources.is_empty() || targets.is_empty() {
                    return Err(StructuralRuntimeError::MissingConnectorEndpoint {
                        connector: relationship.name.clone(),
                        context: self.runtime.instances[&context_id].qualified_path.clone(),
                    });
                }
                let item_flows =
                    self.item_flows(relationship.id, &connector.source, &connector.target)?;
                for source in &sources {
                    for target in &targets {
                        self.validate_item_flow_endpoints(&item_flows, source, target)?;
                        let seed = format!(
                            "{}:{}:{}:{}",
                            relationship.id,
                            context_id,
                            source.qualified_path,
                            target.qualified_path
                        );
                        self.runtime.connector_links.push(RuntimeConnectorLink {
                            id: deterministic_uuid(self.project.id.0, seed.as_bytes()),
                            semantic_connector_id: relationship.id,
                            context_instance_id: context_id,
                            kind: connector.kind,
                            source: source.clone(),
                            target: target.clone(),
                            item_flows: item_flows.clone(),
                            route_reason: format!(
                                "Connector '{}' resolves {} to {} in runtime context {}",
                                relationship.name,
                                source.qualified_path,
                                target.qualified_path,
                                self.runtime.instances[&context_id].qualified_path
                            ),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    fn item_flows(
        &self,
        connector_id: RelationshipId,
        connector_source: &ConnectorEnd,
        connector_target: &ConnectorEnd,
    ) -> Result<Vec<RuntimeItemFlow>, StructuralRuntimeError> {
        let mut flows = Vec::new();
        let mut relationships: Vec<_> = self
            .project
            .relationships
            .values()
            .filter(|relationship| {
                relationship.kind == RelationshipKind::ItemFlow
                    && relationship
                        .item_flow
                        .as_ref()
                        .is_some_and(|flow| flow.connector_id == connector_id)
            })
            .collect();
        relationships.sort_by_key(|relationship| relationship.id.to_string());
        for relationship in relationships {
            let flow = relationship
                .item_flow
                .as_ref()
                .expect("filtered item flows");
            let connector_source_to_target =
                if flow.source == *connector_source && flow.target == *connector_target {
                    true
                } else if flow.source == *connector_target && flow.target == *connector_source {
                    false
                } else {
                    return Err(StructuralRuntimeError::InvalidConnectorEndpoint {
                    connector: relationship.name.clone(),
                    details:
                        "ItemFlow ends do not match either orientation of the realizing Connector"
                            .into(),
                });
                };
            flows.push(RuntimeItemFlow {
                item_flow_id: relationship.id,
                conveyed_item_ids: flow.conveyed_item_ids.clone(),
                connector_source_to_target,
            });
        }
        Ok(flows)
    }

    fn validate_item_flow_endpoints(
        &self,
        item_flows: &[RuntimeItemFlow],
        source: &RuntimeEndpoint,
        target: &RuntimeEndpoint,
    ) -> Result<(), StructuralRuntimeError> {
        for item_flow in item_flows {
            let (flow_source, flow_target) = if item_flow.connector_source_to_target {
                (source, target)
            } else {
                (target, source)
            };
            for conveyed in &item_flow.conveyed_item_ids {
                if let Some(port_id) = flow_source.semantic_port_id {
                    let port = self
                        .runtime
                        .port(flow_source.instance_id, port_id)
                        .ok_or_else(|| StructuralRuntimeError::RuntimePortNotFound {
                            path: flow_source.qualified_path.clone(),
                            port: readable_element(self.project, port_id),
                        })?;
                    validate_item_flow_contract(self.project, item_flow, port, *conveyed, true)?;
                }
                if let Some(port_id) = flow_target.semantic_port_id {
                    let port = self
                        .runtime
                        .port(flow_target.instance_id, port_id)
                        .ok_or_else(|| StructuralRuntimeError::RuntimePortNotFound {
                            path: flow_target.qualified_path.clone(),
                            port: readable_element(self.project, port_id),
                        })?;
                    validate_item_flow_contract(self.project, item_flow, port, *conveyed, false)?;
                }
            }
        }
        Ok(())
    }

    fn resolve_endpoint(
        &self,
        context_instance_id: RuntimeInstanceId,
        end: &ConnectorEnd,
    ) -> Result<Vec<RuntimeEndpoint>, String> {
        let mut current = vec![context_instance_id];
        for property_id in &end.property_path {
            let property = self
                .project
                .element(*property_id)
                .map_err(|error| error.to_string())?;
            let mut next = Vec::new();
            for instance_id in current {
                match property.kind {
                    ElementKind::PartProperty => {
                        next.extend(
                            self.children_by_usage
                                .get(&(instance_id, *property_id))
                                .into_iter()
                                .flatten()
                                .copied(),
                        );
                    }
                    ElementKind::ReferenceProperty => {
                        next.extend(
                            self.runtime
                                .references
                                .iter()
                                .find(|binding| {
                                    binding.owner_instance_id == instance_id
                                        && binding.reference_property_id == *property_id
                                })
                                .into_iter()
                                .flat_map(|binding| binding.target_instance_ids.iter().copied()),
                        );
                    }
                    _ => {
                        return Err(format!(
                            "{} is not a PartProperty or ReferenceProperty",
                            readable_element(self.project, *property_id)
                        ));
                    }
                }
            }
            current = next;
        }
        let mut endpoints = Vec::new();
        for instance_id in current {
            let instance = &self.runtime.instances[&instance_id];
            if let Some(port_id) = end.port_id {
                let port = self.runtime.port(instance_id, port_id).ok_or_else(|| {
                    format!(
                        "{} does not own runtime port {}",
                        instance.qualified_path,
                        readable_element(self.project, port_id)
                    )
                })?;
                endpoints.push(RuntimeEndpoint {
                    instance_id,
                    semantic_port_id: Some(port_id),
                    property_path: end.property_path.clone(),
                    qualified_path: port.qualified_path.clone(),
                });
            } else {
                endpoints.push(RuntimeEndpoint {
                    instance_id,
                    semantic_port_id: None,
                    property_path: end.property_path.clone(),
                    qualified_path: instance.qualified_path.clone(),
                });
            }
        }
        endpoints.sort_by(|left, right| left.qualified_path.cmp(&right.qualified_path));
        Ok(endpoints)
    }
}

fn collect_general_classifiers(
    project: &Project,
    classifier_id: ElementId,
    visited: &mut HashSet<ElementId>,
    output: &mut Vec<ElementId>,
) {
    if !visited.insert(classifier_id) {
        return;
    }
    let mut generals: Vec<_> = project
        .relationships
        .values()
        .filter(|relationship| {
            relationship.kind == RelationshipKind::Generalization
                && relationship.source_id == classifier_id
        })
        .map(|relationship| relationship.target_id)
        .collect();
    generals.sort_by_key(ToString::to_string);
    for general in generals {
        collect_general_classifiers(project, general, visited, output);
        if !output.contains(&general) {
            output.push(general);
        }
    }
}

pub(crate) fn classifier_conforms(
    project: &Project,
    actual: ElementId,
    expected: ElementId,
) -> bool {
    if actual == expected {
        return true;
    }
    let mut stack = vec![actual];
    let mut visited = HashSet::new();
    while let Some(current) = stack.pop() {
        if !visited.insert(current) {
            continue;
        }
        for relationship in project.relationships.values().filter(|relationship| {
            relationship.kind == RelationshipKind::Generalization
                && relationship.source_id == current
        }) {
            if relationship.target_id == expected {
                return true;
            }
            stack.push(relationship.target_id);
        }
    }
    false
}

fn deterministic_instance_id(
    project: &Project,
    semantic_element_id: ElementId,
    classifier_id: ElementId,
    qualified_path: &str,
    ordinal: u32,
) -> RuntimeInstanceId {
    let seed = format!(
        "pr33-runtime-instance:{semantic_element_id}:{classifier_id}:{qualified_path}:{ordinal}"
    );
    RuntimeInstanceId(deterministic_uuid(project.id.0, seed.as_bytes()))
}

fn deterministic_uuid(namespace: Uuid, seed: &[u8]) -> Uuid {
    // Stable, dependency-free FNV-1a derivation. Runtime identity must remain
    // stable for reset/replay; it is not a cryptographic identity boundary.
    fn fnv64(bytes: impl IntoIterator<Item = u8>, offset: u64) -> u64 {
        bytes.into_iter().fold(offset, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        })
    }
    let namespace_bytes = namespace.as_bytes();
    let first = fnv64(
        namespace_bytes.iter().copied().chain(seed.iter().copied()),
        0xcbf2_9ce4_8422_2325,
    );
    let second = fnv64(
        seed.iter().copied().chain(namespace_bytes.iter().copied()),
        0x8422_2325_cbf2_9ce4,
    );
    Uuid::from_u128((u128::from(first) << 64) | u128::from(second))
}

fn readable_element(project: &Project, id: ElementId) -> String {
    project
        .element(id)
        .map(|element| format!("'{}' ({:?}, {})", element.name, element.kind, element.id))
        .unwrap_or_else(|_| format!("missing element {id}"))
}

fn readable_element_name(project: &Project, id: ElementId) -> String {
    project
        .element(id)
        .map(|element| element.name.clone())
        .unwrap_or_else(|_| id.to_string())
}

fn parse_authored_runtime_default(authored: &str) -> RuntimeValue {
    if let Ok(value) = evaluate_execution_expression(authored, |_| None) {
        return value;
    }
    if let Ok(text) = serde_json::from_str::<String>(authored) {
        return RuntimeValue::Text(text);
    }
    RuntimeValue::Text(authored.to_string())
}

fn endpoint_key(endpoint: &RuntimeEndpoint) -> Option<RuntimePortKey> {
    endpoint
        .semantic_port_id
        .map(|semantic_port_id| RuntimePortKey {
            instance_id: endpoint.instance_id,
            semantic_port_id,
        })
}

fn link_allows_item(
    project: &Project,
    link: &RuntimeConnectorLink,
    conveyed_id: ElementId,
    connector_source_to_target: bool,
) -> bool {
    link.item_flows.is_empty()
        || link.item_flows.iter().any(|flow| {
            flow.connector_source_to_target == connector_source_to_target
                && flow
                    .conveyed_item_ids
                    .iter()
                    .any(|authored| classifier_conforms(project, conveyed_id, *authored))
        })
}

fn effective_flow_direction(port: &RuntimePort, direction: FlowDirection) -> FlowDirection {
    if port.kind == RuntimePortKind::Proxy && port.is_conjugated {
        match direction {
            FlowDirection::In => FlowDirection::Out,
            FlowDirection::Out => FlowDirection::In,
            FlowDirection::InOut => FlowDirection::InOut,
        }
    } else {
        direction
    }
}

fn compatible_flow_contracts<'a>(
    project: &Project,
    port: &'a RuntimePort,
    conveyed_id: ElementId,
) -> Vec<&'a RuntimeFlowContract> {
    port.flow_contracts
        .iter()
        .filter(|contract| classifier_conforms(project, conveyed_id, contract.type_id))
        .collect()
}

fn validate_item_flow_contract(
    project: &Project,
    item_flow: &RuntimeItemFlow,
    port: &RuntimePort,
    conveyed_id: ElementId,
    source: bool,
) -> Result<(), StructuralRuntimeError> {
    if port.flow_contracts.is_empty() {
        return Ok(());
    }
    let contracts = compatible_flow_contracts(project, port, conveyed_id);
    if contracts.is_empty() {
        return Err(StructuralRuntimeError::ItemFlowTypeMismatch {
            item_flow: item_flow.item_flow_id.to_string(),
            conveyed: readable_element(project, conveyed_id),
            port: port.qualified_path.clone(),
        });
    }
    let compatible_direction = contracts.iter().any(|contract| {
        let direction = effective_flow_direction(port, contract.direction);
        if source {
            matches!(direction, FlowDirection::Out | FlowDirection::InOut)
        } else {
            matches!(direction, FlowDirection::In | FlowDirection::InOut)
        }
    });
    if compatible_direction {
        Ok(())
    } else if source {
        Err(StructuralRuntimeError::SourceFlowDirection {
            port: port.qualified_path.clone(),
            conveyed: readable_element(project, conveyed_id),
        })
    } else {
        Err(StructuralRuntimeError::TargetFlowDirection {
            port: port.qualified_path.clone(),
            conveyed: readable_element(project, conveyed_id),
        })
    }
}

fn validate_source_flow(
    project: &Project,
    port: &RuntimePort,
    conveyed_id: ElementId,
) -> Result<(), StructuralRuntimeError> {
    if port.flow_contracts.is_empty() {
        return Ok(());
    }
    let contracts = compatible_flow_contracts(project, port, conveyed_id);
    if contracts.iter().any(|contract| {
        matches!(
            effective_flow_direction(port, contract.direction),
            FlowDirection::Out | FlowDirection::InOut
        )
    }) {
        Ok(())
    } else {
        Err(StructuralRuntimeError::SourceFlowDirection {
            port: port.qualified_path.clone(),
            conveyed: readable_element(project, conveyed_id),
        })
    }
}

fn validate_target_flow(
    project: &Project,
    port: &RuntimePort,
    conveyed_id: ElementId,
) -> Result<(), StructuralRuntimeError> {
    if port.flow_contracts.is_empty() {
        return Ok(());
    }
    let contracts = compatible_flow_contracts(project, port, conveyed_id);
    if contracts.iter().any(|contract| {
        matches!(
            effective_flow_direction(port, contract.direction),
            FlowDirection::In | FlowDirection::InOut
        )
    }) {
        Ok(())
    } else {
        Err(StructuralRuntimeError::TargetFlowDirection {
            port: port.qualified_path.clone(),
            conveyed: readable_element(project, conveyed_id),
        })
    }
}

fn validate_reception(
    project: &Project,
    runtime: &StructuralRuntime,
    target: &RuntimeEndpoint,
    signal_id: ElementId,
) -> Result<(), StructuralRuntimeError> {
    let classifier_id = runtime.instances[&target.instance_id]
        .classifier_id
        .expect("structural instances are typed");
    let mut classifiers = Vec::new();
    collect_general_classifiers(
        project,
        classifier_id,
        &mut HashSet::new(),
        &mut classifiers,
    );
    classifiers.push(classifier_id);
    let receptions: Vec<_> = project
        .elements
        .values()
        .filter(|element| {
            element.kind == ElementKind::Reception
                && element
                    .owner_id
                    .is_some_and(|owner| classifiers.contains(&owner))
        })
        .collect();
    if receptions.is_empty()
        || receptions.iter().any(|reception| {
            reception
                .type_id
                .is_some_and(|accepted| classifier_conforms(project, signal_id, accepted))
        })
    {
        Ok(())
    } else {
        Err(StructuralRuntimeError::ReceptionMismatch {
            target: target.qualified_path.clone(),
            signal: readable_element(project, signal_id),
        })
    }
}
