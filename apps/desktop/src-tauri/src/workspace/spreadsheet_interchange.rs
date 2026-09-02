//! Deterministic XLSX interchange built on the same authored-state authority as
//! portable JSON. The extended profile embeds the validated portable payload
//! and also emits normalized, inspectable semantic/presentation sheets.

use super::{
    BddDiagram, WorkspaceState,
    activity_workspace::{ActivityDiagram, ActivityWorkspaceState},
    behavior_workspace::{BehaviorDiagram, BehaviorDiagramKind},
    ibd::IbdDiagram,
    portable_interchange::{
        PortableAuthoredStateV1, PortableProjectV1, export_from_states, portable_from_states,
    },
};
use calamine::{Reader, open_workbook_auto};
use rust_xlsxwriter::{Color, Format, FormatAlign, Workbook};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use systems_modeler_core::{Element, ElementKind, Project, Relationship, RelationshipKind};

const STATE_SHEET: &str = "SystemsModelerState";
const MANIFEST_SHEET: &str = "Manifest";
const STATE_CHUNK_SIZE: usize = 30_000;
type SpreadsheetRows = Vec<Vec<String>>;
type DiagramRows = (SpreadsheetRows, SpreadsheetRows, SpreadsheetRows);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpreadsheetExportProfile {
    CatiaSemantic,
    SystemsModeler,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpreadsheetSynchronizationPolicy {
    Additive,
    AuthoritativeMappedScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpreadsheetInterchangeAction {
    Create,
    Update,
    NoChange,
    Remove,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpreadsheetInterchangePreviewItem {
    pub action: SpreadsheetInterchangeAction,
    pub kind: String,
    pub external_id: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpreadsheetInterchangePreview {
    pub applied: bool,
    pub source_namespace: String,
    pub items: Vec<SpreadsheetInterchangePreviewItem>,
    pub diagnostics: Vec<String>,
}

impl SpreadsheetInterchangePreview {
    pub fn is_valid(&self) -> bool {
        !self
            .items
            .iter()
            .any(|item| item.action == SpreadsheetInterchangeAction::Blocked)
            && self.diagnostics.is_empty()
    }
}

fn header_format() -> Format {
    Format::new()
        .set_bold()
        .set_font_color(Color::White)
        .set_background_color(Color::RGB(0x1F4E78))
        .set_align(FormatAlign::Center)
}

fn write_rows(
    workbook: &mut Workbook,
    name: &str,
    headers: &[&str],
    rows: &[Vec<String>],
) -> Result<(), String> {
    let worksheet = workbook
        .add_worksheet()
        .set_name(name)
        .map_err(|error| error.to_string())?;
    let format = header_format();
    for (column, header) in headers.iter().enumerate() {
        worksheet
            .write_string_with_format(0, column as u16, *header, &format)
            .map_err(|error| error.to_string())?;
        worksheet
            .set_column_width(column as u16, if *header == "Record JSON" { 72 } else { 24 })
            .map_err(|error| error.to_string())?;
    }
    for (row_index, row) in rows.iter().enumerate() {
        for (column, value) in row.iter().enumerate() {
            worksheet
                .write_string((row_index + 1) as u32, column as u16, value)
                .map_err(|error| error.to_string())?;
        }
    }
    worksheet
        .autofilter(0, 0, rows.len() as u32, headers.len().saturating_sub(1) as u16)
        .map_err(|error| error.to_string())?;
    worksheet
        .set_freeze_panes(1, 0)
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn chunks(value: &str) -> Vec<String> {
    let mut output = Vec::new();
    let mut start = 0;
    while start < value.len() {
        let mut end = (start + STATE_CHUNK_SIZE).min(value.len());
        while !value.is_char_boundary(end) {
            end -= 1;
        }
        output.push(value[start..end].to_owned());
        start = end;
    }
    output
}

fn element_owner_external(project: &Project, element: &Element) -> String {
    element
        .owner_id
        .and_then(|id| project.elements.get(&id))
        .map(|owner| owner.external_id.clone())
        .unwrap_or_default()
}

fn element_type_external(project: &Project, element: &Element) -> String {
    element
        .type_id
        .and_then(|id| project.elements.get(&id))
        .map(|record| record.external_id.clone())
        .unwrap_or_default()
}

fn element_row(project: &Project, element: &Element) -> Result<Vec<String>, String> {
    Ok(vec![
        element.external_id.clone(),
        format!("{:?}", element.kind),
        element.name.clone(),
        element_owner_external(project, element),
        element_type_external(project, element),
        project.qualified_name(element.id).unwrap_or_default(),
        serde_json::to_string(element).map_err(|error| error.to_string())?,
    ])
}

fn relationship_row(
    project: &Project,
    relationship: &Relationship,
) -> Result<Vec<String>, String> {
    let external = |id| {
        project
            .elements
            .get(&id)
            .map(|element| element.external_id.clone())
            .unwrap_or_default()
    };
    Ok(vec![
        relationship.external_id.clone(),
        format!("{:?}", relationship.kind),
        relationship.name.clone(),
        relationship.owner_id.map(external).unwrap_or_default(),
        external(relationship.source_id),
        external(relationship.target_id),
        serde_json::to_string(relationship).map_err(|error| error.to_string())?,
    ])
}

fn sorted_elements(project: &Project) -> Vec<&Element> {
    let mut records = project.elements.values().collect::<Vec<_>>();
    records.sort_by(|left, right| {
        project
            .qualified_name(left.id)
            .unwrap_or_default()
            .cmp(&project.qualified_name(right.id).unwrap_or_default())
            .then_with(|| left.external_id.cmp(&right.external_id))
    });
    records
}

fn sorted_relationships(project: &Project) -> Vec<&Relationship> {
    let mut records = project.relationships.values().collect::<Vec<_>>();
    records.sort_by(|left, right| {
        left.external_id
            .cmp(&right.external_id)
            .then_with(|| left.id.to_string().cmp(&right.id.to_string()))
    });
    records
}

fn element_sheet(
    workbook: &mut Workbook,
    name: &str,
    project: &Project,
    include: impl Fn(&Element) -> bool,
) -> Result<(), String> {
    let rows = sorted_elements(project)
        .into_iter()
        .filter(|element| include(element))
        .map(|element| element_row(project, element))
        .collect::<Result<Vec<_>, _>>()?;
    write_rows(
        workbook,
        name,
        &[
            "External ID",
            "Kind",
            "Name",
            "Owner External ID",
            "Type External ID",
            "Qualified Name",
            "Record JSON",
        ],
        &rows,
    )
}

fn relationship_sheet(
    workbook: &mut Workbook,
    name: &str,
    project: &Project,
    include: impl Fn(&Relationship) -> bool,
) -> Result<(), String> {
    let rows = sorted_relationships(project)
        .into_iter()
        .filter(|relationship| include(relationship))
        .map(|relationship| relationship_row(project, relationship))
        .collect::<Result<Vec<_>, _>>()?;
    write_rows(
        workbook,
        name,
        &[
            "External ID",
            "Kind",
            "Name",
            "Owner External ID",
            "Source External ID",
            "Target External ID",
            "Record JSON",
        ],
        &rows,
    )
}

fn write_semantic_sheets(
    workbook: &mut Workbook,
    portable: &PortableProjectV1,
) -> Result<(), String> {
    let project = Project {
        id: portable.project.id,
        name: portable.project.name.clone(),
        root_id: portable.project.root_id,
        elements: portable
            .project
            .elements
            .iter()
            .cloned()
            .map(|element| (element.id, element))
            .collect(),
        relationships: portable
            .project
            .relationships
            .iter()
            .cloned()
            .map(|relationship| (relationship.id, relationship))
            .collect(),
    };
    element_sheet(workbook, "Packages", &project, |element| {
        matches!(element.kind, ElementKind::Model | ElementKind::Package | ElementKind::ModelLibrary)
    })?;
    element_sheet(workbook, "Elements", &project, |_| true)?;
    element_sheet(workbook, "Properties", &project, Element::is_property)?;
    element_sheet(workbook, "Ports", &project, Element::is_port)?;
    element_sheet(workbook, "Operations", &project, |element| element.kind == ElementKind::Operation)?;
    element_sheet(workbook, "Parameters", &project, |element| {
        matches!(element.kind, ElementKind::Parameter | ElementKind::ConstraintParameter)
    })?;
    let requirement_rows = sorted_elements(&project)
        .into_iter()
        .filter(|element| element.kind == ElementKind::Requirement)
        .map(|element| {
            Ok(vec![
                element.external_id.clone(),
                format!("{:?}", element.kind),
                element.name.clone(),
                element.requirement_id.clone().unwrap_or_default(),
                element.requirement_text.clone().unwrap_or_default(),
                element_owner_external(&project, element),
                project.qualified_name(element.id).unwrap_or_default(),
                serde_json::to_string(element).map_err(|error| error.to_string())?,
            ])
        })
        .collect::<Result<Vec<_>, String>>()?;
    write_rows(
        workbook,
        "Requirements",
        &[
            "External ID",
            "Kind",
            "Name",
            "Requirement ID",
            "Requirement Text",
            "Owner External ID",
            "Qualified Name",
            "Record JSON",
        ],
        &requirement_rows,
    )?;
    element_sheet(workbook, "UseCases", &project, |element| {
        matches!(element.kind, ElementKind::UseCase | ElementKind::Actor)
    })?;
    element_sheet(workbook, "Parametrics", &project, |element| {
        matches!(element.kind, ElementKind::ConstraintBlock | ElementKind::ConstraintProperty
            | ElementKind::ConstraintParameter | ElementKind::ValueProperty
            | ElementKind::ValueType | ElementKind::QuantityKind | ElementKind::Unit)
    })?;
    relationship_sheet(workbook, "Relationships", &project, |_| true)?;
    relationship_sheet(workbook, "Connectors", &project, |relationship| {
        relationship.kind == RelationshipKind::Connector
    })?;
    relationship_sheet(workbook, "ItemFlows", &project, |relationship| {
        relationship.kind == RelationshipKind::ItemFlow
    })?;
    relationship_sheet(workbook, "Bindings", &project, |relationship| {
        relationship.kind == RelationshipKind::BindingConnector
    })?;

    let activity_rows = portable
        .activity
        .activities
        .iter()
        .map(|activity| {
            vec![
                activity.external_id.clone(),
                activity.name.clone(),
                activity.owner_id.to_string(),
                serde_json::to_string(activity).unwrap_or_default(),
            ]
        })
        .collect::<Vec<_>>();
    write_rows(
        workbook,
        "Activities",
        &["External ID", "Name", "Owner ID", "Record JSON"],
        &activity_rows,
    )?;
    let mut activity_nodes = Vec::new();
    let mut activity_flows = Vec::new();
    for activity in &portable.activity.activities {
        activity_nodes.extend(activity.nodes.iter().map(|node| {
            vec![
                activity.external_id.clone(),
                node.id.to_string(),
                node.name.clone(),
                format!("{:?}", node.kind),
                serde_json::to_string(node).unwrap_or_default(),
            ]
        }));
        activity_flows.extend(activity.edges.iter().map(|edge| {
            vec![
                activity.external_id.clone(),
                edge.id.to_string(),
                edge.name.clone(),
                format!("{:?}", edge.kind),
                serde_json::to_string(edge).unwrap_or_default(),
            ]
        }));
    }
    activity_nodes.sort();
    activity_flows.sort();
    write_rows(workbook, "ActivityNodes", &["Activity External ID", "Semantic ID", "Name", "Kind", "Record JSON"], &activity_nodes)?;
    write_rows(workbook, "ActivityFlows", &["Activity External ID", "Semantic ID", "Name", "Kind", "Record JSON"], &activity_flows)?;

    let state_machine_rows = portable.behavior.state_machines.iter().map(|machine| vec![
        machine.external_id.clone(), machine.name.clone(), machine.context_id.to_string(),
        serde_json::to_string(machine).unwrap_or_default(),
    ]).collect::<Vec<_>>();
    write_rows(workbook, "StateMachines", &["External ID", "Name", "Context ID", "Record JSON"], &state_machine_rows)?;
    let mut states = Vec::new();
    let mut transitions = Vec::new();
    fn collect_regions(
        machine: &str,
        regions: &[systems_modeler_core::behavior::Region],
        states: &mut Vec<Vec<String>>,
        transitions: &mut Vec<Vec<String>>,
    ) {
        for region in regions {
            for vertex in &region.vertices {
                states.push(vec![machine.into(), region.id.to_string(), vertex.id.to_string(), vertex.name.clone(),
                    serde_json::to_string(vertex).unwrap_or_default()]);
                if let systems_modeler_core::behavior::VertexKind::State(state) = &vertex.kind {
                    collect_regions(machine, &state.regions, states, transitions);
                }
            }
            transitions.extend(region.transitions.iter().map(|transition| vec![
                machine.into(), region.id.to_string(), transition.id.to_string(),
                transition.source_id.to_string(), transition.target_id.to_string(),
                serde_json::to_string(transition).unwrap_or_default(),
            ]));
        }
    }
    for machine in &portable.behavior.state_machines {
        collect_regions(&machine.external_id, &machine.regions, &mut states, &mut transitions);
    }
    states.sort();
    transitions.sort();
    write_rows(workbook, "States", &["State Machine", "Region ID", "Semantic ID", "Name", "Record JSON"], &states)?;
    write_rows(workbook, "Transitions", &["State Machine", "Region ID", "Semantic ID", "Source ID", "Target ID", "Record JSON"], &transitions)?;

    let interaction_rows = portable.behavior.interactions.iter().map(|interaction| vec![
        interaction.external_id.clone(), interaction.name.clone(), interaction.context_id.to_string(),
        serde_json::to_string(interaction).unwrap_or_default(),
    ]).collect::<Vec<_>>();
    write_rows(workbook, "Interactions", &["External ID", "Name", "Context ID", "Record JSON"], &interaction_rows)?;
    let mut lifelines = Vec::new();
    let mut messages = Vec::new();
    for interaction in &portable.behavior.interactions {
        lifelines.extend(interaction.lifelines.iter().map(|lifeline| vec![
            interaction.external_id.clone(), lifeline.id.to_string(), lifeline.name.clone(),
            lifeline.represented_path.iter().map(ToString::to_string).collect::<Vec<_>>().join("/"),
            serde_json::to_string(lifeline).unwrap_or_default(),
        ]));
        messages.extend(interaction.messages.iter().map(|message| vec![
            interaction.external_id.clone(), message.id.to_string(), message.name.clone(),
            format!("{:?}", message.sort), serde_json::to_string(message).unwrap_or_default(),
        ]));
    }
    lifelines.sort();
    messages.sort();
    write_rows(workbook, "Lifelines", &["Interaction", "Semantic ID", "Name", "Represented Path", "Record JSON"], &lifelines)?;
    write_rows(workbook, "Messages", &["Interaction", "Semantic ID", "Name", "Sort", "Record JSON"], &messages)?;
    Ok(())
}

fn diagram_rows(
    bdd: &[BddDiagram],
    ibd: &[IbdDiagram],
    activity: &[ActivityDiagram],
    behavior: &[BehaviorDiagram],
) -> Result<DiagramRows, String> {
    let mut diagrams = Vec::new();
    let mut presentations = Vec::new();
    let mut relationships = Vec::new();
    for diagram in bdd {
        diagrams.push(vec![diagram.id.clone(), diagram.name.clone(), diagram.family.clone(), diagram.owner_id.clone(),
            diagram.semantic_context_id.clone().unwrap_or_default(), serde_json::to_string(diagram).map_err(|error| error.to_string())?]);
        presentations.extend(diagram.nodes.iter().map(|node| vec![diagram.id.clone(), node.id.clone(), node.element_id.clone(), node.x.to_string(), node.y.to_string(), node.width.to_string(), node.height.to_string(), serde_json::to_string(node).unwrap_or_default()]));
        relationships.extend(diagram.edges.iter().map(|edge| vec![diagram.id.clone(), edge.id.clone(), edge.relationship_id.clone(), serde_json::to_string(edge).unwrap_or_default()]));
    }
    for diagram in ibd {
        diagrams.push(vec![diagram.id.clone(), diagram.name.clone(), "ibd".into(), diagram.owner_id.clone(), diagram.context_block_id.clone(), serde_json::to_string(diagram).map_err(|error| error.to_string())?]);
        presentations.extend(diagram.properties.iter().map(|node| vec![diagram.id.clone(), node.id.clone(), node.element_id.clone(), node.x.to_string(), node.y.to_string(), node.width.to_string(), node.height.to_string(), serde_json::to_string(node).unwrap_or_default()]));
        relationships.extend(diagram.connectors.iter().map(|edge| vec![diagram.id.clone(), edge.id.clone(), edge.relationship_id.clone(), serde_json::to_string(edge).unwrap_or_default()]));
    }
    for diagram in activity {
        diagrams.push(vec![diagram.id.clone(), diagram.name.clone(), "activity".into(), diagram.owner_id.clone(), diagram.activity_id.clone(), serde_json::to_string(diagram).map_err(|error| error.to_string())?]);
        presentations.extend(diagram.nodes.iter().map(|node| vec![diagram.id.clone(), node.id.clone(), node.activity_node_id.clone(), node.x.to_string(), node.y.to_string(), node.width.to_string(), node.height.to_string(), serde_json::to_string(node).unwrap_or_default()]));
        relationships.extend(diagram.edges.iter().map(|edge| vec![diagram.id.clone(), edge.id.clone(), edge.activity_edge_id.clone(), serde_json::to_string(edge).unwrap_or_default()]));
    }
    for diagram in behavior {
        let family = match diagram.kind { BehaviorDiagramKind::StateMachine => "state-machine", BehaviorDiagramKind::Sequence => "sequence" };
        diagrams.push(vec![diagram.id.clone(), diagram.name.clone(), family.into(), diagram.owner_id.clone(), diagram.context_id.clone(), serde_json::to_string(diagram).map_err(|error| error.to_string())?]);
        presentations.extend(diagram.state_nodes.iter().map(|node| vec![diagram.id.clone(), node.vertex_id.clone(), node.vertex_id.clone(), node.x.to_string(), node.y.to_string(), node.width.to_string(), node.height.to_string(), serde_json::to_string(node).unwrap_or_default()]));
        presentations.extend(diagram.lifelines.iter().map(|node| vec![diagram.id.clone(), node.lifeline_id.clone(), node.lifeline_id.clone(), node.x.to_string(), node.timeline_start_y.to_string(), "0".into(), (node.timeline_end_y - node.timeline_start_y).to_string(), serde_json::to_string(node).unwrap_or_default()]));
        relationships.extend(diagram.edge_routes.iter().map(|edge| vec![diagram.id.clone(), edge.semantic_id.clone(), edge.semantic_id.clone(), serde_json::to_string(edge).unwrap_or_default()]));
    }
    diagrams.sort();
    presentations.sort();
    relationships.sort();
    Ok((diagrams, presentations, relationships))
}

pub(super) fn export_workbook_to_path(
    path: &str,
    profile: SpreadsheetExportProfile,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> Result<(), String> {
    let portable = portable_from_states(workspace, activity)?;
    let mut workbook = Workbook::new();
    write_rows(&mut workbook, MANIFEST_SHEET, &["Key", "Value"], &[
        vec!["Schema".into(), "systems-modeler-xlsx".into()],
        vec!["Version".into(), "1".into()],
        vec!["Profile".into(), format!("{profile:?}")],
        vec!["Source Namespace".into(), portable.source_namespace.clone()],
        vec!["Project ID".into(), portable.project.id.to_string()],
    ])?;
    write_semantic_sheets(&mut workbook, &portable)?;
    if profile == SpreadsheetExportProfile::SystemsModeler {
        let (diagrams, presentations, relationships) = diagram_rows(
            &portable.diagrams,
            &portable.ibd_diagrams,
            &portable.activity.diagrams,
            &portable.behavior.diagrams,
        )?;
        write_rows(&mut workbook, "Diagrams", &["Diagram ID", "Name", "Family", "Owner ID", "Semantic Context ID", "Record JSON"], &diagrams)?;
        write_rows(&mut workbook, "DiagramPresentations", &["Diagram ID", "Presentation ID", "Semantic ID", "X", "Y", "Width", "Height", "Record JSON"], &presentations)?;
        write_rows(&mut workbook, "DiagramRelationships", &["Diagram ID", "Presentation ID", "Semantic ID", "Record JSON"], &relationships)?;
        let json = export_from_states(workspace, activity)?;
        let rows = chunks(&json).into_iter().enumerate().map(|(index, chunk)| vec![(index + 1).to_string(), chunk]).collect::<Vec<_>>();
        write_rows(&mut workbook, STATE_SHEET, &["Chunk", "Portable JSON"], &rows)?;
    }
    workbook.save(path).map_err(|error| error.to_string())
}

fn read_extended_workbook(path: &str) -> Result<(BTreeMap<String, String>, PortableProjectV1), String> {
    let mut workbook = open_workbook_auto(path).map_err(|error| error.to_string())?;
    let manifest_range = workbook.worksheet_range(MANIFEST_SHEET).map_err(|error| format!("Manifest sheet is required: {error}"))?;
    let mut manifest = BTreeMap::new();
    for row in manifest_range.rows().skip(1) {
        if row.len() >= 2 {
            manifest.insert(row[0].to_string(), row[1].to_string());
        }
    }
    if manifest.get("Schema").map(String::as_str) != Some("systems-modeler-xlsx") {
        return Err("workbook is not a Systems-Modeler extended workbook".into());
    }
    let state_range = workbook.worksheet_range(STATE_SHEET).map_err(|error| format!("{STATE_SHEET} sheet is required: {error}"))?;
    let mut parts = state_range
        .rows()
        .skip(1)
        .filter(|row| row.len() >= 2)
        .map(|row| {
            (
                row[0].to_string().parse::<usize>().unwrap_or(usize::MAX),
                row[1].to_string(),
            )
        })
        .collect::<Vec<_>>();
    parts.sort_by_key(|(index, _)| *index);
    if parts.is_empty() || parts.iter().any(|(index, _)| *index == usize::MAX) {
        return Err("SystemsModelerState chunks are missing or invalid".into());
    }
    let json = parts.into_iter().map(|(_, part)| part).collect::<String>();
    let portable = serde_json::from_str(&json).map_err(|error| format!("invalid embedded authored state: {error}"))?;
    Ok((manifest, portable))
}

fn current_is_blank(workspace: &WorkspaceState, activity: &ActivityWorkspaceState) -> Result<bool, String> {
    let project = workspace.project.lock().map_err(|_| "project lock poisoned")?;
    let semantic_blank = project.as_ref().is_none_or(|project| project.elements.len() == 1 && project.relationships.is_empty());
    let diagrams = workspace.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let ibd_diagrams = workspace.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")?;
    let behavior = workspace.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let behavior_diagrams = workspace.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?;
    let activities = activity.repository.lock().map_err(|_| "Activity repository lock poisoned")?;
    let activity_diagrams = activity.diagrams.lock().map_err(|_| "Activity diagrams lock poisoned")?;
    Ok(semantic_blank
        && diagrams.is_empty()
        && ibd_diagrams.is_empty()
        && behavior.state_machines.is_empty()
        && behavior.interactions.is_empty()
        && behavior_diagrams.is_empty()
        && activities.activities.is_empty()
        && activity_diagrams.is_empty())
}

fn authored_ids(portable: &PortableProjectV1) -> BTreeSet<String> {
    portable.project.elements.iter().map(|record| record.external_id.clone())
        .chain(portable.project.relationships.iter().map(|record| record.external_id.clone()))
        .chain(portable.activity.activities.iter().map(|record| record.external_id.clone()))
        .chain(portable.behavior.state_machines.iter().map(|record| record.external_id.clone()))
        .chain(portable.behavior.interactions.iter().map(|record| record.external_id.clone()))
        .collect()
}

fn imported_namespaces(portable: &PortableProjectV1) -> BTreeSet<String> {
    authored_ids(portable)
        .into_iter()
        .filter_map(|external_id| {
            external_id
                .split_once("::")
                .map(|(namespace, _)| namespace.to_owned())
        })
        .collect()
}

pub(super) fn preview_workbook_import(
    path: &str,
    policy: SpreadsheetSynchronizationPolicy,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> SpreadsheetInterchangePreview {
    let (manifest, incoming) = match read_extended_workbook(path) {
        Ok(value) => value,
        Err(error) => return SpreadsheetInterchangePreview { diagnostics: vec![error], ..Default::default() },
    };
    let namespace = manifest.get("Source Namespace").cloned().unwrap_or_default();
    let mut preview = SpreadsheetInterchangePreview { source_namespace: namespace.clone(), ..Default::default() };
    if let Err(error) = incoming.clone().into_build_plan() {
        preview.items.push(SpreadsheetInterchangePreviewItem {
            action: SpreadsheetInterchangeAction::Blocked,
            kind: "Workbook".into(),
            external_id: incoming.project.id.to_string(),
            detail: "authored semantic, behavior, activity, or diagram references are invalid".into(),
        });
        preview.diagnostics.push(error);
        return preview;
    }
    if current_is_blank(workspace, activity).unwrap_or(false) {
        for record in incoming.project.elements.iter().filter(|record| record.id != incoming.project.root_id) {
            preview.items.push(SpreadsheetInterchangePreviewItem { action: SpreadsheetInterchangeAction::Create,
                kind: format!("{:?}", record.kind), external_id: record.external_id.clone(), detail: record.name.clone() });
        }
        for record in &incoming.project.relationships {
            preview.items.push(SpreadsheetInterchangePreviewItem { action: SpreadsheetInterchangeAction::Create,
                kind: format!("{:?}", record.kind), external_id: record.external_id.clone(), detail: record.name.clone() });
        }
        for diagram in incoming.diagrams.iter().map(|record| (&record.id, &record.name))
            .chain(incoming.ibd_diagrams.iter().map(|record| (&record.id, &record.name)))
            .chain(incoming.activity.diagrams.iter().map(|record| (&record.id, &record.name)))
            .chain(incoming.behavior.diagrams.iter().map(|record| (&record.id, &record.name))) {
            preview.items.push(SpreadsheetInterchangePreviewItem { action: SpreadsheetInterchangeAction::Create,
                kind: "Diagram".into(), external_id: diagram.0.clone(), detail: diagram.1.clone() });
        }
        return preview;
    }
    let current_json = match export_from_states(workspace, activity) {
        Ok(value) => value,
        Err(error) => { preview.diagnostics.push(error); return preview; }
    };
    let incoming_json = serde_json::to_string_pretty(&incoming).unwrap_or_default();
    if current_json == incoming_json {
        preview.items.push(SpreadsheetInterchangePreviewItem { action: SpreadsheetInterchangeAction::NoChange,
            kind: "Workbook".into(), external_id: incoming.project.id.to_string(), detail: "authored state is identical".into() });
        return preview;
    }
    if policy == SpreadsheetSynchronizationPolicy::Additive {
        preview.items.push(SpreadsheetInterchangePreviewItem { action: SpreadsheetInterchangeAction::Blocked,
            kind: "Workbook".into(), external_id: incoming.project.id.to_string(),
            detail: "additive full-fidelity import cannot overwrite a non-blank target; use ordinary mapped import for additive updates or explicitly select authoritative mapped scope".into() });
        return preview;
    }
    let current = match portable_from_states(workspace, activity) {
        Ok(value) => value,
        Err(error) => { preview.diagnostics.push(error); return preview; }
    };
    if current.project.id != incoming.project.id {
        preview.items.push(SpreadsheetInterchangePreviewItem { action: SpreadsheetInterchangeAction::Blocked,
            kind: "Project".into(), external_id: incoming.project.id.to_string(),
            detail: "authoritative synchronization requires the same Project identity".into() });
        return preview;
    }
    let incoming_ids = authored_ids(&incoming);
    let incoming_namespaces = imported_namespaces(&incoming);
    for external_id in authored_ids(&current).difference(&incoming_ids) {
        let proven_namespace = external_id
            .split_once("::")
            .map(|(candidate, _)| incoming_namespaces.contains(candidate))
            .unwrap_or(false);
        if proven_namespace {
            preview.items.push(SpreadsheetInterchangePreviewItem { action: SpreadsheetInterchangeAction::Remove,
                kind: "Imported semantic".into(), external_id: external_id.clone(),
                detail: "absent from authoritative workbook scope".into() });
        } else {
            preview.items.push(SpreadsheetInterchangePreviewItem { action: SpreadsheetInterchangeAction::Blocked,
                kind: "Manual or unproven semantic".into(), external_id: external_id.clone(),
                detail: "removal origin cannot be proven for this source namespace".into() });
        }
    }
    if preview.is_valid() {
        preview.items.push(SpreadsheetInterchangePreviewItem { action: SpreadsheetInterchangeAction::Update,
            kind: "Project".into(), external_id: incoming.project.id.to_string(),
            detail: "validated authored state will be synchronized atomically".into() });
    }
    preview
}

fn replace_states_atomically(
    portable: PortableProjectV1,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> Result<(), String> {
    let plan = portable.into_build_plan()?;
    let state = match plan.operations.into_iter().next() {
        Some(super::bulk_model::ModelBuildOperation::RestorePortableState { state }) => state,
        _ => return Err("extended workbook did not produce one authored-state candidate".into()),
    };
    state.validate("systems-modeler-xlsx")?;
    let PortableAuthoredStateV1 {
        project: next_project,
        diagrams: next_diagrams,
        ibd_diagrams: next_ibd,
        activity_repository: next_activity,
        activity_diagrams: next_activity_diagrams,
        behavior_repository: next_behavior,
        behavior_diagrams: next_behavior_diagrams,
    } = *state;
    let mut project = workspace.project.lock().map_err(|_| "project lock poisoned")?;
    let mut diagrams = workspace.diagrams.lock().map_err(|_| "diagram lock poisoned")?;
    let mut ibd = workspace.ibd_diagrams.lock().map_err(|_| "IBD lock poisoned")?;
    let mut behavior = workspace.behavior.lock().map_err(|_| "behavior lock poisoned")?;
    let mut behavior_diagrams = workspace.behavior_diagrams.lock().map_err(|_| "behavior diagram lock poisoned")?;
    let mut activities = activity.repository.lock().map_err(|_| "Activity repository lock poisoned")?;
    let mut activity_diagrams = activity.diagrams.lock().map_err(|_| "Activity diagrams lock poisoned")?;
    *project = Some(next_project);
    *diagrams = next_diagrams;
    *ibd = next_ibd;
    *behavior = next_behavior;
    *behavior_diagrams = next_behavior_diagrams;
    *activities = next_activity;
    *activity_diagrams = next_activity_diagrams;
    Ok(())
}

pub(super) fn apply_workbook_import(
    path: &str,
    policy: SpreadsheetSynchronizationPolicy,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
) -> SpreadsheetInterchangePreview {
    let mut preview = preview_workbook_import(path, policy, workspace, activity);
    if !preview.is_valid() || preview.items.iter().all(|item| item.action == SpreadsheetInterchangeAction::NoChange) {
        return preview;
    }
    let (_, portable) = match read_extended_workbook(path) {
        Ok(value) => value,
        Err(error) => { preview.diagnostics.push(error); return preview; }
    };
    match replace_states_atomically(portable, workspace, activity) {
        Ok(()) => preview.applied = true,
        Err(error) => preview.diagnostics.push(error),
    }
    preview
}

#[tauri::command]
pub fn export_spreadsheet_workbook(
    path: String,
    profile: SpreadsheetExportProfile,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
) -> Result<(), String> {
    export_workbook_to_path(&path, profile, &workspace, &activity)
}

#[tauri::command]
pub fn stage_spreadsheet_upload(file_name: String, bytes: Vec<u8>) -> Result<String, String> {
    const MAX_UPLOAD_BYTES: usize = 64 * 1024 * 1024;
    if bytes.is_empty() || bytes.len() > MAX_UPLOAD_BYTES {
        return Err("spreadsheet upload must be between 1 byte and 64 MiB".into());
    }
    let extension = std::path::Path::new(&file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or("spreadsheet file extension is required")?;
    if !matches!(extension.as_str(), "xlsx" | "csv") {
        return Err("only XLSX and CSV spreadsheet uploads are supported".into());
    }
    let path = std::env::temp_dir().join(format!(
        "systems-modeler-import-{}.{}",
        uuid::Uuid::new_v4(),
        extension
    ));
    std::fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn discard_staged_spreadsheet(path: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(path);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("invalid staged spreadsheet path")?;
    if path.parent() != Some(std::env::temp_dir().as_path())
        || !file_name.starts_with("systems-modeler-import-")
    {
        return Err("only staged spreadsheet uploads can be discarded".into());
    }
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command]
pub fn preview_spreadsheet_workbook_import(
    path: String,
    policy: SpreadsheetSynchronizationPolicy,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
) -> SpreadsheetInterchangePreview {
    preview_workbook_import(&path, policy, &workspace, &activity)
}

#[tauri::command]
pub fn apply_spreadsheet_workbook_import(
    path: String,
    policy: SpreadsheetSynchronizationPolicy,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
) -> SpreadsheetInterchangePreview {
    apply_workbook_import(&path, policy, &workspace, &activity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::{
        portable_interchange::tests::representative_states,
        spreadsheet_import::{
            SpreadsheetColumnMapping, SpreadsheetIdentificationProperty, SpreadsheetImportMap,
            SpreadsheetImportMapGroup, SpreadsheetSearchScope, SpreadsheetSemanticProperty,
            apply_spreadsheet_import_group, preview_spreadsheet_import_group,
        },
    };

    fn workbook_path(label: &str) -> String {
        std::env::temp_dir()
            .join(format!("systems-modeler-{label}-{}.xlsx", uuid::Uuid::new_v4()))
            .to_string_lossy()
            .into_owned()
    }

    fn workbook_cells(path: &str) -> BTreeMap<String, Vec<Vec<String>>> {
        let mut workbook = open_workbook_auto(path).unwrap();
        let mut sheets = BTreeMap::new();
        let names = workbook.sheet_names().to_vec();
        for name in names {
            let rows = workbook
                .worksheet_range(&name)
                .unwrap()
                .rows()
                .map(|row| row.iter().map(ToString::to_string).collect())
                .collect();
            sheets.insert(name, rows);
        }
        sheets
    }

    #[test]
    fn systems_modeler_xlsx_round_trip_preserves_all_nine_authored_families() {
        let (source, source_activity) = representative_states();
        let authored = portable_from_states(&source, &source_activity).unwrap();
        assert_eq!(
            authored.diagrams.len()
                + authored.ibd_diagrams.len()
                + authored.activity.diagrams.len()
                + authored.behavior.diagrams.len(),
            9
        );
        let path = workbook_path("round-trip");
        export_workbook_to_path(
            &path,
            SpreadsheetExportProfile::SystemsModeler,
            &source,
            &source_activity,
        )
        .unwrap();

        let target = WorkspaceState::default();
        let target_activity = ActivityWorkspaceState::default();
        let preview = preview_workbook_import(
            &path,
            SpreadsheetSynchronizationPolicy::Additive,
            &target,
            &target_activity,
        );
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert!(preview.items.iter().any(|item| {
            item.kind == "Diagram" && item.action == SpreadsheetInterchangeAction::Create
        }));
        let applied = apply_workbook_import(
            &path,
            SpreadsheetSynchronizationPolicy::Additive,
            &target,
            &target_activity,
        );
        assert!(applied.applied, "{:?}", applied.diagnostics);
        assert_eq!(
            export_from_states(&source, &source_activity).unwrap(),
            export_from_states(&target, &target_activity).unwrap()
        );
        let second = preview_workbook_import(
            &path,
            SpreadsheetSynchronizationPolicy::Additive,
            &target,
            &target_activity,
        );
        assert_eq!(second.items.len(), 1);
        assert_eq!(second.items[0].action, SpreadsheetInterchangeAction::NoChange);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn repeated_export_has_deterministic_workbook_content() {
        let (workspace, activity) = representative_states();
        let first = workbook_path("deterministic-a");
        let second = workbook_path("deterministic-b");
        export_workbook_to_path(
            &first,
            SpreadsheetExportProfile::SystemsModeler,
            &workspace,
            &activity,
        )
        .unwrap();
        export_workbook_to_path(
            &second,
            SpreadsheetExportProfile::SystemsModeler,
            &workspace,
            &activity,
        )
        .unwrap();
        assert_eq!(workbook_cells(&first), workbook_cells(&second));
        std::fs::remove_file(first).unwrap();
        std::fs::remove_file(second).unwrap();
    }

    #[test]
    fn catia_semantic_profile_round_trips_through_mapping_importer() {
        let (workspace, activity) = representative_states();
        let path = workbook_path("catia-semantic");
        export_workbook_to_path(
            &path,
            SpreadsheetExportProfile::CatiaSemantic,
            &workspace,
            &activity,
        )
        .unwrap();
        let sheets = workbook_cells(&path);
        assert!(sheets.contains_key("Requirements"));
        assert!(!sheets.contains_key(STATE_SHEET));

        let target = WorkspaceState::default();
        let root = {
            let mut guard = target.project.lock().unwrap();
            let project = Project::new("CATIA semantic target");
            let root = project.root_id;
            *guard = Some(project);
            root
        };
        let group = SpreadsheetImportMapGroup {
            mappings: vec![SpreadsheetImportMap {
                name: "Exported requirements".into(),
                source: path.clone(),
                worksheet: Some("Requirements".into()),
                header_row: 1,
                element_kind: ElementKind::Requirement,
                relationship_kind: None,
                relationship_identity: Default::default(),
                target_scope: root,
                identification_property: SpreadsheetIdentificationProperty::ExternalId,
                search_scope: SpreadsheetSearchScope::TargetRecursive,
                source_namespace: "catia-export-qualification".into(),
                mapping_version: "1".into(),
                column_mappings: vec![
                    SpreadsheetColumnMapping {
                        source_column: "External ID".into(),
                        property: SpreadsheetSemanticProperty::ExternalId,
                    },
                    SpreadsheetColumnMapping {
                        source_column: "Name".into(),
                        property: SpreadsheetSemanticProperty::Name,
                    },
                    SpreadsheetColumnMapping {
                        source_column: "Requirement ID".into(),
                        property: SpreadsheetSemanticProperty::RequirementId,
                    },
                    SpreadsheetColumnMapping {
                        source_column: "Requirement Text".into(),
                        property: SpreadsheetSemanticProperty::RequirementText,
                    },
                ],
            }],
        };
        let preview = preview_spreadsheet_import_group(&group, &target);
        assert!(preview.is_valid(), "{:?}", preview.diagnostics);
        assert!(preview.totals.create > 0);
        apply_spreadsheet_import_group(&group, &target).unwrap();
        assert!(target
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .elements
            .values()
            .any(|element| element.kind == ElementKind::Requirement));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn invalid_workbook_preview_is_non_mutating() {
        let (workspace, activity) = representative_states();
        let before = export_from_states(&workspace, &activity).unwrap();
        let preview = preview_workbook_import(
            "missing-workbook.xlsx",
            SpreadsheetSynchronizationPolicy::AuthoritativeMappedScope,
            &workspace,
            &activity,
        );
        assert!(!preview.is_valid());
        assert_eq!(before, export_from_states(&workspace, &activity).unwrap());
    }

    #[test]
    fn authoritative_sync_previews_proven_removal_and_protects_manual_content() {
        let (seed, seed_activity) = representative_states();
        let mut current_portable = portable_from_states(&seed, &seed_activity).unwrap();
        let requirement_id = current_portable
            .project
            .elements
            .iter_mut()
            .find(|element| element.kind == ElementKind::Requirement)
            .map(|element| {
                element.external_id = "supplier-a::REQ-1".into();
                element.id
            })
            .unwrap();
        current_portable
            .project
            .elements
            .iter_mut()
            .find(|element| element.kind == ElementKind::TestCase)
            .unwrap()
            .external_id = "supplier-a::TEST-1".into();
        let mut incoming_portable = current_portable.clone();
        incoming_portable
            .project
            .elements
            .retain(|element| element.id != requirement_id);

        let current = WorkspaceState::default();
        let current_activity = ActivityWorkspaceState::default();
        replace_states_atomically(current_portable, &current, &current_activity).unwrap();
        let incoming = WorkspaceState::default();
        let incoming_activity = ActivityWorkspaceState::default();
        replace_states_atomically(incoming_portable, &incoming, &incoming_activity).unwrap();
        let path = workbook_path("authoritative");
        export_workbook_to_path(
            &path,
            SpreadsheetExportProfile::SystemsModeler,
            &incoming,
            &incoming_activity,
        )
        .unwrap();

        let additive = preview_workbook_import(
            &path,
            SpreadsheetSynchronizationPolicy::Additive,
            &current,
            &current_activity,
        );
        assert!(!additive.is_valid());
        let authoritative = preview_workbook_import(
            &path,
            SpreadsheetSynchronizationPolicy::AuthoritativeMappedScope,
            &current,
            &current_activity,
        );
        assert!(authoritative.is_valid(), "{:?}", authoritative.diagnostics);
        assert!(authoritative.items.iter().any(|item| {
            item.action == SpreadsheetInterchangeAction::Remove
                && item.external_id == "supplier-a::REQ-1"
        }));
        assert!(apply_workbook_import(
            &path,
            SpreadsheetSynchronizationPolicy::AuthoritativeMappedScope,
            &current,
            &current_activity,
        )
        .applied);

        let manual = {
            let mut guard = current.project.lock().unwrap();
            let project = guard.as_mut().unwrap();
            let root_id = project.root_id;
            project
                .create_element(ElementKind::Package, "Manual package", root_id)
                .unwrap()
        };
        let protected = preview_workbook_import(
            &path,
            SpreadsheetSynchronizationPolicy::AuthoritativeMappedScope,
            &current,
            &current_activity,
        );
        assert!(!protected.is_valid());
        assert!(protected.items.iter().any(|item| {
            item.action == SpreadsheetInterchangeAction::Blocked
                && item.external_id
                    == current
                        .project
                        .lock()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .element(manual)
                        .unwrap()
                        .external_id
        }));
        std::fs::remove_file(path).unwrap();
    }
}
