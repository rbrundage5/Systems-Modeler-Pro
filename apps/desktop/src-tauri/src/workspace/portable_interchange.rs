use super::{
    BddDiagram, WorkspaceState,
    activity_workspace::{ActivityDiagram, ActivityWorkspaceState, validate_activity_diagrams},
    behavior_workspace::{BehaviorDiagram, validate_behavior_workspace},
    bulk_model::{
        ModelBuildOperation, ModelBuildPlan, ModelBuildResult, apply_complete_model_build,
    },
    ibd::{IbdDiagram, validate_ibd_diagrams},
    validate_loaded_diagrams,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use systems_modeler_core::behavior::{BehaviorRepository, Interaction, StateMachine};
use systems_modeler_core::{
    Activity, ActivityRepository, Element, Project, ProjectId, Relationship,
};

pub const PORTABLE_SCHEMA: &str = "systems-modeler-interchange";
pub const PORTABLE_VERSION: u32 = 1;
pub const NATIVE_SOURCE_NAMESPACE: &str = "systems-modeler-native";

const DIAGRAM_FAMILIES: [&str; 9] = [
    "bdd",
    "ibd",
    "requirement",
    "use-case",
    "package",
    "activity",
    "state-machine",
    "sequence",
    "parametric",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortableSemanticProjectV1 {
    pub id: ProjectId,
    pub name: String,
    pub root_id: systems_modeler_core::ElementId,
    pub elements: Vec<Element>,
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortableActivityStateV1 {
    pub activities: Vec<Activity>,
    pub diagrams: Vec<ActivityDiagram>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortableBehaviorStateV1 {
    pub state_machines: Vec<StateMachine>,
    pub interactions: Vec<Interaction>,
    pub diagrams: Vec<BehaviorDiagram>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortableProjectV1 {
    pub schema: String,
    pub version: u32,
    pub source_namespace: String,
    pub diagram_families: Vec<String>,
    pub project: PortableSemanticProjectV1,
    pub diagrams: Vec<BddDiagram>,
    pub ibd_diagrams: Vec<IbdDiagram>,
    pub activity: PortableActivityStateV1,
    pub behavior: PortableBehaviorStateV1,
}

#[derive(Debug, Clone)]
pub(crate) struct PortableAuthoredStateV1 {
    pub(super) project: Project,
    pub(super) diagrams: Vec<BddDiagram>,
    pub(super) ibd_diagrams: Vec<IbdDiagram>,
    pub(super) activity_repository: ActivityRepository,
    pub(super) activity_diagrams: Vec<ActivityDiagram>,
    pub(super) behavior_repository: BehaviorRepository,
    pub(super) behavior_diagrams: Vec<BehaviorDiagram>,
}

fn sorted_project(project: &Project) -> PortableSemanticProjectV1 {
    let mut elements: Vec<_> = project.elements.values().cloned().collect();
    elements.sort_by(|left, right| {
        left.external_id
            .cmp(&right.external_id)
            .then_with(|| left.id.to_string().cmp(&right.id.to_string()))
    });
    let mut relationships: Vec<_> = project.relationships.values().cloned().collect();
    relationships.sort_by(|left, right| {
        left.external_id
            .cmp(&right.external_id)
            .then_with(|| left.id.to_string().cmp(&right.id.to_string()))
    });
    PortableSemanticProjectV1 {
        id: project.id,
        name: project.name.clone(),
        root_id: project.root_id,
        elements,
        relationships,
    }
}

fn portable_from_states(
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> Result<PortableProjectV1, String> {
    let project = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?
        .clone()
        .ok_or("no project open")?;
    let mut diagrams = workspace
        .diagrams
        .lock()
        .map_err(|_| "diagram lock poisoned")?
        .clone();
    let mut ibd_diagrams = workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")?
        .clone();
    let behavior = workspace
        .behavior
        .lock()
        .map_err(|_| "behavior lock poisoned")?
        .clone();
    let mut behavior_diagrams = workspace
        .behavior_diagrams
        .lock()
        .map_err(|_| "behavior diagram lock poisoned")?
        .clone();
    let activities = activity
        .repository
        .lock()
        .map_err(|_| "Activity repository lock poisoned")?
        .clone();
    let mut activity_diagrams = activity
        .diagrams
        .lock()
        .map_err(|_| "Activity diagram lock poisoned")?
        .clone();

    validate_authored_content(
        &project,
        &diagrams,
        &ibd_diagrams,
        &activities,
        &activity_diagrams,
        &behavior,
        &behavior_diagrams,
        NATIVE_SOURCE_NAMESPACE,
    )?;

    diagrams.sort_by(|left, right| {
        left.family
            .cmp(&right.family)
            .then_with(|| left.id.cmp(&right.id))
    });
    ibd_diagrams.sort_by(|left, right| left.id.cmp(&right.id));
    activity_diagrams.sort_by(|left, right| left.id.cmp(&right.id));
    behavior_diagrams.sort_by(|left, right| left.id.cmp(&right.id));

    let mut activity_records: Vec<_> = activities.activities.values().cloned().collect();
    activity_records.sort_by(|left, right| {
        left.external_id
            .cmp(&right.external_id)
            .then_with(|| left.id.to_string().cmp(&right.id.to_string()))
    });
    let mut state_machines: Vec<_> = behavior.state_machines.values().cloned().collect();
    state_machines.sort_by(|left, right| {
        left.external_id
            .cmp(&right.external_id)
            .then_with(|| left.id.to_string().cmp(&right.id.to_string()))
    });
    let mut interactions: Vec<_> = behavior.interactions.values().cloned().collect();
    interactions.sort_by(|left, right| {
        left.external_id
            .cmp(&right.external_id)
            .then_with(|| left.id.to_string().cmp(&right.id.to_string()))
    });

    Ok(PortableProjectV1 {
        schema: PORTABLE_SCHEMA.into(),
        version: PORTABLE_VERSION,
        source_namespace: NATIVE_SOURCE_NAMESPACE.into(),
        diagram_families: DIAGRAM_FAMILIES
            .iter()
            .map(|value| (*value).into())
            .collect(),
        project: sorted_project(&project),
        diagrams,
        ibd_diagrams,
        activity: PortableActivityStateV1 {
            activities: activity_records,
            diagrams: activity_diagrams,
        },
        behavior: PortableBehaviorStateV1 {
            state_machines,
            interactions,
            diagrams: behavior_diagrams,
        },
    })
}

impl PortableProjectV1 {
    fn into_build_plan(self) -> Result<ModelBuildPlan, String> {
        if self.schema != PORTABLE_SCHEMA {
            return Err(format!("unsupported portable schema: {}", self.schema));
        }
        if self.version != PORTABLE_VERSION {
            return Err(format!(
                "unsupported portable schema version: {}",
                self.version
            ));
        }
        if self.source_namespace.trim().is_empty() {
            return Err("portable source namespace is required".into());
        }
        let expected: HashSet<_> = DIAGRAM_FAMILIES.iter().copied().collect();
        let actual: HashSet<_> = self.diagram_families.iter().map(String::as_str).collect();
        if actual != expected || self.diagram_families.len() != DIAGRAM_FAMILIES.len() {
            return Err(
                "portable v1 must declare exactly the nine qualified diagram families".into(),
            );
        }

        let mut elements = HashMap::new();
        for element in self.project.elements {
            if elements.insert(element.id, element).is_some() {
                return Err("portable project contains a duplicate semantic element ID".into());
            }
        }
        let mut relationships = HashMap::new();
        for relationship in self.project.relationships {
            if relationships
                .insert(relationship.id, relationship)
                .is_some()
            {
                return Err("portable project contains a duplicate relationship ID".into());
            }
        }
        let project = Project {
            id: self.project.id,
            name: self.project.name,
            root_id: self.project.root_id,
            elements,
            relationships,
        };
        let mut activity_repository = ActivityRepository::default();
        for record in self.activity.activities {
            if activity_repository
                .activities
                .insert(record.id, record)
                .is_some()
            {
                return Err("portable project contains a duplicate Activity ID".into());
            }
        }
        let mut behavior_repository = BehaviorRepository::default();
        for record in self.behavior.state_machines {
            if behavior_repository
                .state_machines
                .insert(record.id, record)
                .is_some()
            {
                return Err("portable project contains a duplicate State Machine ID".into());
            }
        }
        for record in self.behavior.interactions {
            if behavior_repository
                .interactions
                .insert(record.id, record)
                .is_some()
            {
                return Err("portable project contains a duplicate Interaction ID".into());
            }
        }
        let state = PortableAuthoredStateV1 {
            project,
            diagrams: self.diagrams,
            ibd_diagrams: self.ibd_diagrams,
            activity_repository,
            activity_diagrams: self.activity.diagrams,
            behavior_repository,
            behavior_diagrams: self.behavior.diagrams,
        };
        state.validate(&self.source_namespace)?;
        Ok(ModelBuildPlan {
            source_namespace: self.source_namespace,
            operations: vec![ModelBuildOperation::RestorePortableState {
                state: Box::new(state),
            }],
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_authored_content(
    project: &Project,
    diagrams: &[BddDiagram],
    ibd_diagrams: &[IbdDiagram],
    activities: &ActivityRepository,
    activity_diagrams: &[ActivityDiagram],
    behavior: &BehaviorRepository,
    behavior_diagrams: &[BehaviorDiagram],
    source_namespace: &str,
) -> Result<(), String> {
    for relationship in project.relationships.values() {
        if !project.elements.contains_key(&relationship.source_id) {
            return Err(format!(
                "Relationship {} in namespace {source_namespace}: source reference {} could not be resolved",
                relationship.external_id, relationship.source_id
            ));
        }
        if !project.elements.contains_key(&relationship.target_id) {
            return Err(format!(
                "Relationship {} in namespace {source_namespace}: target reference {} could not be resolved",
                relationship.external_id, relationship.target_id
            ));
        }
    }
    project
        .validate()
        .map_err(|error| format!("semantic model validation failed: {error}"))?;
    validate_loaded_diagrams(project, diagrams)
        .map_err(|error| format!("diagram presentation validation failed: {error}"))?;
    validate_ibd_diagrams(project, ibd_diagrams)
        .map_err(|error| format!("IBD presentation validation failed: {error}"))?;
    activities
        .validate(project)
        .map_err(|error| format!("Activity validation failed: {error}"))?;
    validate_activity_diagrams(activities, activity_diagrams)
        .map_err(|error| format!("Activity presentation validation failed: {error}"))?;
    validate_behavior_workspace(project, behavior, behavior_diagrams)
        .map_err(|error| format!("State Machine/Sequence validation failed: {error}"))?;
    Ok(())
}

impl PortableAuthoredStateV1 {
    pub(super) fn validate(&self, source_namespace: &str) -> Result<(), String> {
        validate_authored_content(
            &self.project,
            &self.diagrams,
            &self.ibd_diagrams,
            &self.activity_repository,
            &self.activity_diagrams,
            &self.behavior_repository,
            &self.behavior_diagrams,
            source_namespace,
        )
    }

    pub(super) fn build_result(&self, source_namespace: &str) -> ModelBuildResult {
        ModelBuildResult {
            element_ids: self
                .project
                .elements
                .values()
                .map(|element| {
                    (
                        format!("{source_namespace}::{}", element.external_id),
                        element.id,
                    )
                })
                .collect(),
            relationship_ids: self
                .project
                .relationships
                .values()
                .map(|relationship| {
                    (
                        format!("{source_namespace}::{}", relationship.external_id),
                        relationship.id,
                    )
                })
                .collect(),
            diagram_ids: self
                .diagrams
                .iter()
                .map(|diagram| {
                    let id = uuid::Uuid::parse_str(&diagram.id)
                        .map(systems_modeler_core::DiagramId)
                        .expect("portable diagram IDs were validated");
                    (format!("{source_namespace}::{}", diagram.id), id)
                })
                .chain(self.ibd_diagrams.iter().map(|diagram| {
                    let id = uuid::Uuid::parse_str(&diagram.id)
                        .map(systems_modeler_core::DiagramId)
                        .expect("portable IBD IDs were validated");
                    (format!("{source_namespace}::{}", diagram.id), id)
                }))
                .collect(),
        }
    }
}

fn export_from_states(
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> Result<String, String> {
    let portable = portable_from_states(workspace, activity)?;
    serde_json::to_string_pretty(&portable).map_err(|error| error.to_string())
}

fn import_into_states(
    json: &str,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> Result<ModelBuildResult, String> {
    let portable: PortableProjectV1 =
        serde_json::from_str(json).map_err(|error| format!("invalid portable JSON: {error}"))?;
    let plan = portable.into_build_plan()?;
    apply_complete_model_build(&plan, workspace, activity).map_err(|preview| {
        preview
            .diagnostics
            .iter()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .collect::<Vec<_>>()
            .join("; ")
    })
}

#[tauri::command]
pub fn export_portable_project_json(
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
) -> Result<String, String> {
    export_from_states(&workspace, &activity)
}

#[tauri::command]
pub fn import_portable_project_json(
    json: String,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
) -> Result<(), String> {
    import_into_states(&json, &workspace, &activity).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{DiagramEdge, DiagramNode, DiagramPoint};
    use systems_modeler_core::behavior::{
        BehaviorRepository, Lifeline, LifelineId, Message, MessageId, MessageSort, Occurrence,
        OccurrenceId, State, Vertex, VertexId, VertexKind,
    };
    use systems_modeler_core::{
        Action, ActionKind, ActivityNode, ActivityNodeId, ActivityNodeKind, ElementId, ElementKind,
        Multiplicity, RelationshipKind,
    };

    fn diagram(
        family: &str,
        name: &str,
        owner: ElementId,
        context: Option<ElementId>,
    ) -> BddDiagram {
        BddDiagram {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            owner_id: owner.to_string(),
            family: family.into(),
            semantic_context_id: context.map(|id| id.to_string()),
            subject_boundary: None,
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    fn representative_states() -> (WorkspaceState, ActivityWorkspaceState) {
        let workspace = WorkspaceState::default();
        let activity_state = ActivityWorkspaceState::default();
        let mut project = Project::new("Round Trip Example");
        let package = project
            .create_element(ElementKind::Package, "RoundTripExample", project.root_id)
            .unwrap();
        let block_a = project
            .create_element(ElementKind::Block, "Block A", package)
            .unwrap();
        let block_b = project
            .create_element(ElementKind::Block, "Block B", package)
            .unwrap();
        let value_type = project
            .create_element(ElementKind::ValueType, "Real", package)
            .unwrap();
        let value = project
            .create_typed_feature(
                ElementKind::ValueProperty,
                "mass",
                block_a,
                value_type,
                Multiplicity::ONE,
            )
            .unwrap();
        project.element_mut(value).unwrap().default_value = Some("42".into());
        let part = project
            .create_typed_feature(
                ElementKind::PartProperty,
                "b",
                block_a,
                block_b,
                Multiplicity::ONE,
            )
            .unwrap();
        let second_part = project
            .create_typed_feature(
                ElementKind::PartProperty,
                "b2",
                block_a,
                block_b,
                Multiplicity::ONE,
            )
            .unwrap();
        let requirement = project
            .create_element(ElementKind::Requirement, "Mass Requirement", package)
            .unwrap();
        {
            let record = project.element_mut(requirement).unwrap();
            record.requirement_id = Some("REQ-1".into());
            record.requirement_text = Some("The system shall expose mass.".into());
        }
        project
            .create_element(ElementKind::TestCase, "Mass Test", package)
            .unwrap();
        project
            .create_element(ElementKind::Actor, "Operator", package)
            .unwrap();
        project
            .create_element(ElementKind::UseCase, "Operate", package)
            .unwrap();
        let relationship = project
            .create_relationship(
                RelationshipKind::Dependency,
                block_a,
                block_b,
                Some(package),
            )
            .unwrap();

        let mut bdd = diagram("bdd", "Structure", package, None);
        let node_a = DiagramNode {
            id: uuid::Uuid::new_v4().to_string(),
            element_id: block_a.to_string(),
            x: 80.0,
            y: 100.0,
            width: 190.0,
            height: 115.0,
            actor_notation: None,
            parameter_presentations: Vec::new(),
        };
        let node_b = DiagramNode {
            id: uuid::Uuid::new_v4().to_string(),
            element_id: block_b.to_string(),
            x: 380.0,
            y: 100.0,
            width: 190.0,
            height: 115.0,
            actor_notation: None,
            parameter_presentations: Vec::new(),
        };
        bdd.edges.push(DiagramEdge {
            id: uuid::Uuid::new_v4().to_string(),
            relationship_id: relationship.to_string(),
            source_node_id: node_a.id.clone(),
            target_node_id: node_b.id.clone(),
            points: vec![
                DiagramPoint { x: 270.0, y: 157.0 },
                DiagramPoint { x: 380.0, y: 157.0 },
            ],
            label_anchor: Some(DiagramPoint { x: 325.0, y: 157.0 }),
        });
        bdd.nodes.extend([node_a, node_b]);
        let diagrams = vec![
            bdd,
            diagram("requirement", "Requirements", package, None),
            diagram("use-case", "Use Cases", package, Some(block_a)),
            diagram("package", "Packages", package, None),
            diagram("parametric", "Mass Analysis", package, Some(block_a)),
        ];

        let ibd_diagrams = vec![IbdDiagram {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Block A Internal".into(),
            context_block_id: block_a.to_string(),
            owner_id: package.to_string(),
            properties: vec![super::super::ibd::IbdPropertyPresentation {
                id: uuid::Uuid::new_v4().to_string(),
                element_id: part.to_string(),
                property_path: vec![part.to_string()],
                x: 100.0,
                y: 100.0,
                width: 180.0,
                height: 105.0,
                ports: Vec::new(),
            }],
            boundary_ports: Vec::new(),
            connectors: Vec::new(),
        }];

        let mut activities = ActivityRepository::default();
        let activity_id = activities
            .create_activity(&project, package, Some(block_a), "Operate Activity")
            .unwrap();
        let activity_node_id = ActivityNodeId::new();
        activities
            .activities
            .get_mut(&activity_id)
            .unwrap()
            .nodes
            .push(ActivityNode {
                id: activity_node_id,
                name: "Calculate".into(),
                kind: ActivityNodeKind::Action(Action {
                    kind: ActionKind::Opaque {
                        body: "mass = 42".into(),
                    },
                    pins: Vec::new(),
                }),
                partition_id: None,
                structured_node_id: None,
            });
        let activity_diagrams = vec![ActivityDiagram {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Operate Activity".into(),
            owner_id: package.to_string(),
            activity_id: activity_id.to_string(),
            nodes: vec![super::super::activity_workspace::ActivityDiagramNode {
                id: uuid::Uuid::new_v4().to_string(),
                activity_node_id: activity_node_id.to_string(),
                x: 100.0,
                y: 100.0,
                width: 150.0,
                height: 72.0,
            }],
            edges: Vec::new(),
        }];

        let mut behavior = BehaviorRepository::default();
        let machine_id = behavior
            .create_state_machine(&project, block_a, "Lifecycle")
            .unwrap();
        let vertex_id = VertexId::new();
        behavior
            .state_machines
            .get_mut(&machine_id)
            .unwrap()
            .regions[0]
            .vertices
            .push(Vertex {
                id: vertex_id,
                name: "Ready".into(),
                kind: VertexKind::State(State::default()),
            });
        let interaction_id = behavior
            .create_interaction(&project, block_a, "Exchange")
            .unwrap();
        let first_lifeline = LifelineId::new();
        let second_lifeline = LifelineId::new();
        let interaction = behavior.interactions.get_mut(&interaction_id).unwrap();
        interaction.lifelines.extend([
            Lifeline {
                id: first_lifeline,
                name: "b".into(),
                represented_path: vec![part],
            },
            Lifeline {
                id: second_lifeline,
                name: "b2".into(),
                represented_path: vec![second_part],
            },
        ]);
        interaction.messages.push(Message {
            id: MessageId::new(),
            name: "result".into(),
            sort: MessageSort::Reply,
            send_event: Some(Occurrence {
                id: OccurrenceId::new(),
                lifeline_id: first_lifeline,
                order: 1,
            }),
            receive_event: Some(Occurrence {
                id: OccurrenceId::new(),
                lifeline_id: second_lifeline,
                order: 2,
            }),
            signature: None,
            arguments: vec!["42".into()],
        });
        let behavior_diagrams = vec![
            BehaviorDiagram {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Lifecycle".into(),
                owner_id: package.to_string(),
                context_id: block_a.to_string(),
                kind: super::super::behavior_workspace::BehaviorDiagramKind::StateMachine,
                semantic_id: machine_id.to_string(),
                state_nodes: vec![super::super::behavior_workspace::StateNodePresentation {
                    vertex_id: vertex_id.to_string(),
                    x: 100.0,
                    y: 100.0,
                    width: 160.0,
                    height: 90.0,
                }],
                lifelines: Vec::new(),
                edge_routes: Vec::new(),
                hidden_semantic_ids: Vec::new(),
                presentation_copies: Vec::new(),
            },
            BehaviorDiagram {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Exchange".into(),
                owner_id: package.to_string(),
                context_id: block_a.to_string(),
                kind: super::super::behavior_workspace::BehaviorDiagramKind::Sequence,
                semantic_id: interaction_id.to_string(),
                state_nodes: Vec::new(),
                lifelines: vec![
                    super::super::behavior_workspace::LifelinePresentation {
                        lifeline_id: first_lifeline.to_string(),
                        x: 120.0,
                        timeline_start_y: 102.0,
                        timeline_end_y: 840.0,
                    },
                    super::super::behavior_workspace::LifelinePresentation {
                        lifeline_id: second_lifeline.to_string(),
                        x: 420.0,
                        timeline_start_y: 102.0,
                        timeline_end_y: 840.0,
                    },
                ],
                edge_routes: Vec::new(),
                hidden_semantic_ids: Vec::new(),
                presentation_copies: Vec::new(),
            },
        ];

        *workspace.project.lock().unwrap() = Some(project);
        *workspace.diagrams.lock().unwrap() = diagrams;
        *workspace.ibd_diagrams.lock().unwrap() = ibd_diagrams;
        *workspace.behavior.lock().unwrap() = behavior;
        *workspace.behavior_diagrams.lock().unwrap() = behavior_diagrams;
        *activity_state.repository.lock().unwrap() = activities;
        *activity_state.diagrams.lock().unwrap() = activity_diagrams;
        (workspace, activity_state)
    }

    #[test]
    fn pr37_round_trip_preserves_all_nine_authored_families() {
        let (source, source_activity) = representative_states();
        let json = export_from_states(&source, &source_activity).unwrap();
        let target = WorkspaceState::default();
        let target_activity = ActivityWorkspaceState::default();
        import_into_states(&json, &target, &target_activity).unwrap();
        let reconstructed = export_from_states(&target, &target_activity).unwrap();
        assert_eq!(json, reconstructed);
        let portable: PortableProjectV1 = serde_json::from_str(&reconstructed).unwrap();
        assert_eq!(portable.diagram_families.len(), 9);
        assert_eq!(portable.project.relationships.len(), 1);
        assert_eq!(portable.activity.activities.len(), 1);
        assert_eq!(portable.behavior.state_machines.len(), 1);
        assert_eq!(portable.behavior.interactions.len(), 1);
    }

    #[test]
    fn pr37_export_is_deterministic_and_excludes_runtime_state() {
        let (source, activity) = representative_states();
        let first = export_from_states(&source, &activity).unwrap();
        let second = export_from_states(&source, &activity).unwrap();
        assert_eq!(first, second);
        for excluded in [
            "ExecutionSession",
            "RuntimeInstance",
            "SimulationTime",
            "undo",
            "current_file",
        ] {
            assert!(!first.contains(excluded));
        }
    }

    #[test]
    fn pr37_invalid_late_reference_is_atomic_and_contextual() {
        let (source, activity) = representative_states();
        let json = export_from_states(&source, &activity).unwrap();
        let mut portable: PortableProjectV1 = serde_json::from_str(&json).unwrap();
        let missing = ElementId::new();
        portable.project.relationships[0].target_id = missing;
        let invalid_json = serde_json::to_string(&portable).unwrap();
        let target = WorkspaceState::default();
        let target_activity = ActivityWorkspaceState::default();
        let before_project = target.project.lock().unwrap().clone();
        let error = import_into_states(&invalid_json, &target, &target_activity).unwrap_err();
        assert!(error.contains(&portable.project.relationships[0].external_id));
        assert!(error.contains("target reference"));
        assert_eq!(
            target.project.lock().unwrap().is_some(),
            before_project.is_some()
        );
        assert!(target.diagrams.lock().unwrap().is_empty());
        assert!(target.ibd_diagrams.lock().unwrap().is_empty());
        assert!(
            target_activity
                .repository
                .lock()
                .unwrap()
                .activities
                .is_empty()
        );
    }
}

#[cfg(test)]
mod pr42_allocation_tests {
    use super::*;
    use systems_modeler_core::{ElementKind, RelationshipKind};

    #[test]
    fn pr42_portable_json_round_trip_preserves_native_allocate() {
        let source = WorkspaceState::default();
        let activity = ActivityWorkspaceState::default();
        let mut project = Project::new("PR42 Portable Source");
        let package = project
            .create_element(ElementKind::Package, "Architecture", project.root_id)
            .unwrap();
        let logical = project
            .create_element(ElementKind::Block, "LogicalController", package)
            .unwrap();
        let physical = project
            .create_element(ElementKind::Block, "PhysicalController", package)
            .unwrap();
        let allocation = project
            .create_relationship(RelationshipKind::Allocate, logical, physical, Some(package))
            .unwrap();
        {
            let relationship = project.relationships.get_mut(&allocation).unwrap();
            relationship.external_id = "catia:pr42::ALLOC-PORTABLE".into();
            relationship.name = "Portable allocation".into();
            relationship.documentation = "Portable round trip".into();
        }
        *source.project.lock().unwrap() = Some(project);

        let json = export_from_states(&source, &activity).unwrap();
        assert!(json.contains("Allocate"));

        let target = WorkspaceState::default();
        let target_activity = ActivityWorkspaceState::default();
        import_into_states(&json, &target, &target_activity).unwrap();
        let guard = target.project.lock().unwrap();
        let restored = guard.as_ref().unwrap();
        let relationship = restored.relationship(allocation).unwrap();
        assert_eq!(relationship.kind, RelationshipKind::Allocate);
        assert_eq!(relationship.external_id, "catia:pr42::ALLOC-PORTABLE");
        assert_eq!(relationship.source_id, logical);
        assert_eq!(relationship.target_id, physical);
        assert_eq!(relationship.owner_id, Some(package));
        assert_eq!(relationship.name, "Portable allocation");
        assert_eq!(relationship.documentation, "Portable round trip");
    }
}

#[cfg(test)]
mod pr43_port_tests;
#[cfg(test)]
mod pr44_connector_tests;
#[cfg(test)]
mod pr45_item_flow_tests;
#[cfg(test)]
mod pr46_operation_parameter_reception_tests;
