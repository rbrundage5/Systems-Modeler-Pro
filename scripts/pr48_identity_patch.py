from pathlib import Path


def replace_once(path, old, new):
    p = Path(path)
    text = p.read_text()
    if old not in text:
        raise SystemExit(f"missing anchor in {path}: {old[:160]!r}")
    p.write_text(text.replace(old, new, 1))

# Activity repository: stable external identity for nested authored semantic records.
activity = "crates/model-core/src/activity.rs"
replace_once(
    activity,
    "activity_id_type!(StructuredNodeId);\n\n#[derive(Debug, Clone, Serialize, Deserialize, Default)]\npub struct ActivityRepository {\n    pub activities: HashMap<ActivityId, Activity>,\n}\n",
    "activity_id_type!(StructuredNodeId);\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\npub enum ActivitySemanticId {\n    Node(ActivityNodeId),\n    Pin(PinId),\n    Edge(ActivityEdgeId),\n    Partition(ActivityPartitionId),\n    StructuredNode(StructuredNodeId),\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize, Default)]\npub struct ActivityRepository {\n    pub activities: HashMap<ActivityId, Activity>,\n    /// Stable source identities for specialized authored Activity records.\n    /// Internal typed UUIDs remain implementation identity and are never workbook identity.\n    #[serde(default)]\n    pub external_ids: HashMap<String, ActivitySemanticId>,\n}\n",
)
replace_once(
    activity,
    "    #[error(\"interrupting edge must reference an InterruptibleActivityRegion\")]\n    InvalidInterruptingRegion,\n    #[error(transparent)]",
    "    #[error(\"interrupting edge must reference an InterruptibleActivityRegion\")]\n    InvalidInterruptingRegion,\n    #[error(\"activity external identity must be non-empty\")]\n    EmptyExternalIdentity,\n    #[error(\"activity external identity is duplicated or collides with another semantic record: {0}\")]\n    DuplicateExternalIdentity(String),\n    #[error(\"activity external identity references a missing specialized semantic record: {0}\")]\n    UnknownExternalIdentity(String),\n    #[error(transparent)]",
)
replace_once(
    activity,
    "    pub fn validate(&self, project: &Project) -> Result<(), ActivityError> {\n        for activity in self.activities.values() {\n            validate_activity(self, project, activity)?;\n        }\n        Ok(())\n    }\n}",
    "    pub fn validate(&self, project: &Project) -> Result<(), ActivityError> {\n        for activity in self.activities.values() {\n            validate_activity(self, project, activity)?;\n        }\n        self.validate_external_identities()?;\n        Ok(())\n    }\n\n    fn semantic_identity_exists(&self, identity: ActivitySemanticId) -> bool {\n        match identity {\n            ActivitySemanticId::Node(id) => self\n                .activities\n                .values()\n                .any(|activity| activity.nodes.iter().any(|node| node.id == id)),\n            ActivitySemanticId::Pin(id) => self.activities.values().any(|activity| {\n                activity.nodes.iter().any(|node| match &node.kind {\n                    ActivityNodeKind::Action(action) => action.pins.iter().any(|pin| pin.id == id),\n                    _ => false,\n                })\n            }),\n            ActivitySemanticId::Edge(id) => self\n                .activities\n                .values()\n                .any(|activity| activity.edges.iter().any(|edge| edge.id == id)),\n            ActivitySemanticId::Partition(id) => self.activities.values().any(|activity| {\n                activity.partitions.iter().any(|partition| partition.id == id)\n            }),\n            ActivitySemanticId::StructuredNode(id) => self.activities.values().any(|activity| {\n                activity\n                    .structured_nodes\n                    .iter()\n                    .any(|structured| structured.id == id)\n            }),\n        }\n    }\n\n    fn validate_external_identities(&self) -> Result<(), ActivityError> {\n        let mut keys = HashSet::new();\n        for activity in self.activities.values() {\n            if activity.external_id.trim().is_empty() {\n                return Err(ActivityError::EmptyExternalIdentity);\n            }\n            if !keys.insert(activity.external_id.clone()) {\n                return Err(ActivityError::DuplicateExternalIdentity(\n                    activity.external_id.clone(),\n                ));\n            }\n        }\n        let mut targets = HashSet::new();\n        for (external_id, identity) in &self.external_ids {\n            if external_id.trim().is_empty() {\n                return Err(ActivityError::EmptyExternalIdentity);\n            }\n            if !keys.insert(external_id.clone()) || !targets.insert(*identity) {\n                return Err(ActivityError::DuplicateExternalIdentity(external_id.clone()));\n            }\n            if !self.semantic_identity_exists(*identity) {\n                return Err(ActivityError::UnknownExternalIdentity(external_id.clone()));\n            }\n        }\n        Ok(())\n    }\n}\n",
)

# Behavior repository: stable identities for nested State Machine records only.
behavior = "crates/model-core/src/behavior.rs"
replace_once(
    behavior,
    "behavior_id_type!(InvariantId);\n\n#[derive(Debug, Clone, Serialize, Deserialize, Default)]\npub struct BehaviorRepository {\n    pub state_machines: HashMap<StateMachineId, StateMachine>,\n    pub interactions: HashMap<InteractionId, Interaction>,\n}\n",
    "behavior_id_type!(InvariantId);\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\npub enum BehaviorSemanticId {\n    Region(RegionId),\n    Vertex(VertexId),\n    Transition(TransitionId),\n}\n\n#[derive(Debug, Clone, Serialize, Deserialize, Default)]\npub struct BehaviorRepository {\n    pub state_machines: HashMap<StateMachineId, StateMachine>,\n    pub interactions: HashMap<InteractionId, Interaction>,\n    /// Stable source identities for specialized State Machine records.\n    #[serde(default)]\n    pub external_ids: HashMap<String, BehaviorSemanticId>,\n}\n",
)
replace_once(
    behavior,
    "    #[error(\"state invariant constraint cannot be empty\")]\n    EmptyStateInvariant,\n    #[error(transparent)]",
    "    #[error(\"state invariant constraint cannot be empty\")]\n    EmptyStateInvariant,\n    #[error(\"behavior external identity must be non-empty\")]\n    EmptyExternalIdentity,\n    #[error(\"behavior external identity is duplicated or collides with another semantic record: {0}\")]\n    DuplicateExternalIdentity(String),\n    #[error(\"behavior external identity references a missing specialized semantic record: {0}\")]\n    UnknownExternalIdentity(String),\n    #[error(transparent)]",
)
replace_once(
    behavior,
    "        for interaction in self.interactions.values() {\n            validate_interaction(project, interaction)?;\n        }\n        Ok(())\n    }\n}\n",
    "        for interaction in self.interactions.values() {\n            validate_interaction(project, interaction)?;\n        }\n        self.validate_external_identities()?;\n        Ok(())\n    }\n\n    fn semantic_identity_exists(&self, identity: BehaviorSemanticId) -> bool {\n        fn region_contains(regions: &[Region], identity: BehaviorSemanticId) -> bool {\n            for region in regions {\n                if identity == BehaviorSemanticId::Region(region.id) {\n                    return true;\n                }\n                if region\n                    .transitions\n                    .iter()\n                    .any(|transition| identity == BehaviorSemanticId::Transition(transition.id))\n                {\n                    return true;\n                }\n                for vertex in &region.vertices {\n                    if identity == BehaviorSemanticId::Vertex(vertex.id) {\n                        return true;\n                    }\n                    if let VertexKind::State(state) = &vertex.kind\n                        && region_contains(&state.regions, identity)\n                    {\n                        return true;\n                    }\n                }\n            }\n            false\n        }\n        self.state_machines\n            .values()\n            .any(|machine| region_contains(&machine.regions, identity))\n    }\n\n    fn validate_external_identities(&self) -> Result<(), BehaviorError> {\n        let mut keys = HashSet::new();\n        for machine in self.state_machines.values() {\n            if machine.external_id.trim().is_empty() {\n                return Err(BehaviorError::EmptyExternalIdentity);\n            }\n            if !keys.insert(machine.external_id.clone()) {\n                return Err(BehaviorError::DuplicateExternalIdentity(\n                    machine.external_id.clone(),\n                ));\n            }\n        }\n        for interaction in self.interactions.values() {\n            if interaction.external_id.trim().is_empty() {\n                return Err(BehaviorError::EmptyExternalIdentity);\n            }\n            if !keys.insert(interaction.external_id.clone()) {\n                return Err(BehaviorError::DuplicateExternalIdentity(\n                    interaction.external_id.clone(),\n                ));\n            }\n        }\n        let mut targets = HashSet::new();\n        for (external_id, identity) in &self.external_ids {\n            if external_id.trim().is_empty() {\n                return Err(BehaviorError::EmptyExternalIdentity);\n            }\n            if !keys.insert(external_id.clone()) || !targets.insert(*identity) {\n                return Err(BehaviorError::DuplicateExternalIdentity(external_id.clone()));\n            }\n            if !self.semantic_identity_exists(*identity) {\n                return Err(BehaviorError::UnknownExternalIdentity(external_id.clone()));\n            }\n        }\n        Ok(())\n    }\n}\n",
)

# Portable interchange carries the repository identity indexes explicitly while
# remaining backward-compatible with existing v1 payloads.
portable = "apps/desktop/src-tauri/src/workspace/portable_interchange.rs"
replace_once(
    portable,
    "use systems_modeler_core::behavior::{BehaviorRepository, Interaction, StateMachine};\nuse systems_modeler_core::{\n    Activity, ActivityRepository, Element, Project, ProjectId, Relationship,\n};",
    "use systems_modeler_core::behavior::{\n    BehaviorRepository, BehaviorSemanticId, Interaction, StateMachine,\n};\nuse systems_modeler_core::{\n    Activity, ActivityRepository, ActivitySemanticId, Element, Project, ProjectId, Relationship,\n};",
)
replace_once(
    portable,
    "pub struct PortableActivityStateV1 {\n    pub activities: Vec<Activity>,\n    pub diagrams: Vec<ActivityDiagram>,\n}",
    "pub struct PortableActivityStateV1 {\n    pub activities: Vec<Activity>,\n    #[serde(default)]\n    pub external_ids: HashMap<String, ActivitySemanticId>,\n    pub diagrams: Vec<ActivityDiagram>,\n}",
)
replace_once(
    portable,
    "pub struct PortableBehaviorStateV1 {\n    pub state_machines: Vec<StateMachine>,\n    pub interactions: Vec<Interaction>,\n    pub diagrams: Vec<BehaviorDiagram>,\n}",
    "pub struct PortableBehaviorStateV1 {\n    pub state_machines: Vec<StateMachine>,\n    pub interactions: Vec<Interaction>,\n    #[serde(default)]\n    pub external_ids: HashMap<String, BehaviorSemanticId>,\n    pub diagrams: Vec<BehaviorDiagram>,\n}",
)
replace_once(
    portable,
    "        activity: PortableActivityStateV1 {\n            activities: activity_records,\n            diagrams: activity_diagrams,\n        },\n        behavior: PortableBehaviorStateV1 {\n            state_machines,\n            interactions,\n            diagrams: behavior_diagrams,\n        },",
    "        activity: PortableActivityStateV1 {\n            activities: activity_records,\n            external_ids: activities.external_ids.clone(),\n            diagrams: activity_diagrams,\n        },\n        behavior: PortableBehaviorStateV1 {\n            state_machines,\n            interactions,\n            external_ids: behavior.external_ids.clone(),\n            diagrams: behavior_diagrams,\n        },",
)
replace_once(
    portable,
    "        let mut activity_repository = ActivityRepository::default();\n        for record in self.activity.activities {",
    "        let mut activity_repository = ActivityRepository {\n            external_ids: self.activity.external_ids,\n            ..ActivityRepository::default()\n        };\n        for record in self.activity.activities {",
)
replace_once(
    portable,
    "        let mut behavior_repository = BehaviorRepository::default();\n        for record in self.behavior.state_machines {",
    "        let mut behavior_repository = BehaviorRepository {\n            external_ids: self.behavior.external_ids,\n            ..BehaviorRepository::default()\n        };\n        for record in self.behavior.state_machines {",
)

# Focused native identity regression.
Path("crates/model-core/tests/pr48_specialized_external_identity.rs").write_text(r'''use systems_modeler_core::behavior::{BehaviorRepository, BehaviorSemanticId, PseudostateKind, Transition, TransitionId, TransitionKind, Vertex, VertexId, VertexKind};
use systems_modeler_core::{ActivityEdge, ActivityEdgeId, ActivityEdgeKind, ActivityEndpoint, ActivityNode, ActivityNodeId, ActivityNodeKind, ActivityRepository, ActivitySemanticId, ElementKind, Project};

#[test]
fn pr48_specialized_external_identity_is_native_validated_authored_state() {
    let mut project = Project::new("PR48 Identity");
    let block = project.create_element(ElementKind::Block, "Controller", project.root_id).unwrap();

    let mut activities = ActivityRepository::default();
    let activity_id = activities.create_activity(&project, block, Some(block), "Operate").unwrap();
    let activity = activities.activities.get_mut(&activity_id).unwrap();
    activity.external_id = "catia:pr48::ACT".into();
    let initial = ActivityNode { id: ActivityNodeId::new(), name: "Start".into(), kind: ActivityNodeKind::Initial, partition_id: None, structured_node_id: None };
    let final_node = ActivityNode { id: ActivityNodeId::new(), name: "Done".into(), kind: ActivityNodeKind::ActivityFinal, partition_id: None, structured_node_id: None };
    let edge = ActivityEdge { id: ActivityEdgeId::new(), name: "flow".into(), kind: ActivityEdgeKind::ControlFlow, source: ActivityEndpoint::Node(initial.id), target: ActivityEndpoint::Node(final_node.id), guard: None, weight: None, selection: None, transformation: None, interrupting_region_id: None };
    activity.nodes.extend([initial.clone(), final_node]);
    activity.edges.push(edge.clone());
    activities.external_ids.insert("catia:pr48::NODE-START".into(), ActivitySemanticId::Node(initial.id));
    activities.external_ids.insert("catia:pr48::EDGE-1".into(), ActivitySemanticId::Edge(edge.id));
    activities.validate(&project).unwrap();

    let mut behavior = BehaviorRepository::default();
    let sm = behavior.create_state_machine(&project, block, "Lifecycle").unwrap();
    let machine = behavior.state_machines.get_mut(&sm).unwrap();
    machine.external_id = "catia:pr48::SM".into();
    let region = &mut machine.regions[0];
    let initial = Vertex { id: VertexId::new(), name: "Initial".into(), kind: VertexKind::Pseudostate(PseudostateKind::Initial) };
    let final_state = Vertex { id: VertexId::new(), name: "Final".into(), kind: VertexKind::FinalState };
    let transition = Transition { id: TransitionId::new(), source_id: initial.id, target_id: final_state.id, kind: TransitionKind::External, trigger: None, guard: None, effect: None };
    region.vertices.extend([initial.clone(), final_state]);
    region.transitions.push(transition.clone());
    behavior.external_ids.insert("catia:pr48::REGION".into(), BehaviorSemanticId::Region(region.id));
    behavior.external_ids.insert("catia:pr48::VERTEX-INIT".into(), BehaviorSemanticId::Vertex(initial.id));
    behavior.external_ids.insert("catia:pr48::TRANS-1".into(), BehaviorSemanticId::Transition(transition.id));
    behavior.validate(&project).unwrap();

    let missing = ActivitySemanticId::Node(ActivityNodeId::new());
    activities.external_ids.insert("catia:pr48::MISSING".into(), missing);
    assert!(activities.validate(&project).is_err());
}
''')
