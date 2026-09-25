//! Contextual presentation of reusable structural definitions. No model creation.
use super::ibd::{self, IbdDiagram, IbdPortPresentation, IbdPropertyPresentation};
use super::*;

const MAX_PATH_DEPTH: usize = 32;
const MAX_OCCURRENCES: usize = 4096;

pub(super) fn is_descendant(path: &[String], parent: &[String]) -> bool {
    path.len() > parent.len() && path.starts_with(parent)
}

pub(super) fn property_visible(diagram: &IbdDiagram, property: &IbdPropertyPresentation) -> bool {
    !diagram.properties.iter().any(|ancestor| {
        ancestor.collapsed && is_descendant(&property.property_path, &ancestor.property_path)
    })
}

pub(super) fn endpoint_visible(diagram: &IbdDiagram, id: &str) -> bool {
    diagram.boundary_ports.iter().any(|port| port.id == id)
        || diagram.properties.iter().any(|property| {
            (property.id == id || property.ports.iter().any(|port| port.id == id))
                && property_visible(diagram, property)
        })
}

fn add_ports(project: &Project, diagram: &mut IbdDiagram, index: usize) -> Result<(), String> {
    let property = &mut diagram.properties[index];
    let path = property
        .property_path
        .iter()
        .map(|id| parse_element_id(id))
        .collect::<Result<Vec<_>, _>>()?;
    let classifier = project
        .resolve_structural_path(parse_element_id(&diagram.context_block_id)?, &path)
        .map_err(|error| error.to_string())?;
    for port in project
        .classifier_features(classifier)
        .map_err(|error| error.to_string())?
    {
        if !port.is_port()
            || property
                .ports
                .iter()
                .any(|p| p.element_id == port.id.to_string())
        {
            continue;
        }
        let offset = 50.0 + property.ports.len() as f64 * 30.0;
        property.height = property.height.max(offset + 24.0);
        property.ports.push(IbdPortPresentation {
            id: uuid::Uuid::new_v4().to_string(),
            element_id: port.id.to_string(),
            property_path: property.property_path.clone(),
            x: property.x,
            y: property.y + offset,
            size: 16.0,
        });
    }
    Ok(())
}

/// Fit every enclosing presentation without shrinking or discarding authored geometry.
pub(super) fn fit_ancestors(diagram: &mut IbdDiagram) -> Result<(), String> {
    let mut order: Vec<_> = (0..diagram.properties.len()).collect();
    order.sort_by_key(|index| std::cmp::Reverse(diagram.properties[*index].property_path.len()));
    for index in order {
        let path = diagram.properties[index].property_path.clone();
        let old = super::ibd_geometry::property_rect(&diagram.properties[index]);
        let mut width = old.width;
        let mut height = old.height;
        for child in &diagram.properties {
            if is_descendant(&child.property_path, &path) {
                width = width.max(child.x + child.width + 24.0 - old.x);
                height = height.max(child.y + child.height + 24.0 - old.y);
            }
        }
        let new = super::routing::RouteRect {
            width,
            height,
            ..old
        };
        for port in &mut diagram.properties[index].ports {
            super::ibd_geometry::reanchor_port(port, old, new)?;
        }
        diagram.properties[index].width = width;
        diagram.properties[index].height = height;
    }
    if let Some(mut frame) = diagram.context_frame.clone() {
        for property in &diagram.properties {
            frame.width = frame
                .width
                .max(property.x + property.width + 32.0 - frame.x);
            frame.height = frame
                .height
                .max(property.y + property.height + 32.0 - frame.y);
        }
        super::ibd_geometry::apply_context_frame(diagram, frame)?;
    }
    Ok(())
}

pub(super) fn expand_property(
    project: &Project,
    diagram: &mut IbdDiagram,
    presentation_id: &str,
    expand: bool,
) -> Result<(), String> {
    let index = diagram
        .properties
        .iter()
        .position(|p| p.id == presentation_id)
        .ok_or("IBD property occurrence not found")?;
    let parent = diagram.properties[index].clone();
    if !expand {
        diagram.properties[index].collapsed = true;
        return Ok(());
    }
    if parent.property_path.len() >= MAX_PATH_DEPTH {
        return Err(format!(
            "Expansion is limited to {MAX_PATH_DEPTH} levels. Open the type-level IBD to continue; the model is unchanged."
        ));
    }
    let path = parent
        .property_path
        .iter()
        .map(|id| parse_element_id(id))
        .collect::<Result<Vec<_>, _>>()?;
    let classifier = project
        .resolve_structural_path(parse_element_id(&diagram.context_block_id)?, &path)
        .map_err(|error| error.to_string())?;
    let features = project
        .classifier_features(classifier)
        .map_err(|error| error.to_string())?;
    let children: Vec<_> = features
        .into_iter()
        .filter(|feature| {
            matches!(
                feature.kind,
                ElementKind::PartProperty | ElementKind::ReferenceProperty
            )
        })
        .collect();
    let missing = children
        .iter()
        .filter(|feature| {
            let mut path = parent.property_path.clone();
            path.push(feature.id.to_string());
            !diagram.properties.iter().any(|p| p.property_path == path)
        })
        .count();
    if diagram.properties.len() + missing > MAX_OCCURRENCES {
        return Err(format!(
            "This view is limited to {MAX_OCCURRENCES} property occurrences. Open a type-level IBD for further detail."
        ));
    }
    diagram.properties[index].collapsed = false;
    add_ports(project, diagram, index)?;
    let mut y = diagram
        .properties
        .iter()
        .filter(|p| is_descendant(&p.property_path, &parent.property_path))
        .map(|p| p.y + p.height + 24.0)
        .fold(parent.y + 48.0, f64::max);
    for feature in children {
        let mut path = parent.property_path.clone();
        path.push(feature.id.to_string());
        if diagram.properties.iter().any(|p| p.property_path == path) {
            continue;
        }
        diagram.properties.push(IbdPropertyPresentation {
            collapsed: false,
            id: uuid::Uuid::new_v4().to_string(),
            element_id: feature.id.to_string(),
            property_path: path,
            x: parent.x + 24.0,
            y,
            width: 220.0,
            height: 100.0,
            ports: Vec::new(),
        });
        let child_index = diagram.properties.len() - 1;
        add_ports(project, diagram, child_index)?;
        y += diagram.properties[child_index].height + 24.0;
    }
    fit_ancestors(diagram)?;
    diagram.connectors = ibd::routed_ibd_connectors(diagram, None)?;
    Ok(())
}

#[tauri::command]
pub fn set_ibd_structure_expanded(
    diagram_id: String,
    presentation_id: String,
    expanded: bool,
    state: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<(), String> {
    edit_structure(
        &diagram_id,
        Some((&presentation_id, expanded)),
        &state,
        &activity,
        &history,
    )
}

#[tauri::command]
pub fn show_ibd_existing_parts(
    diagram_id: String,
    state: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<(), String> {
    edit_structure(&diagram_id, None, &state, &activity, &history)
}

fn edit_structure(
    diagram_id: &str,
    occurrence: Option<(&str, bool)>,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
    history: &HistoryState,
) -> Result<(), String> {
    history::apply_structural_specification(workspace, activity, history, |project, diagrams| {
        let mut diagrams = diagrams.to_vec();
        let diagram = diagrams
            .iter_mut()
            .find(|d| d.id == diagram_id)
            .ok_or("IBD not found")?;
        if let Some((id, expanded)) = occurrence {
            expand_property(project, diagram, id, expanded)?;
        } else {
            ibd::populate_ibd_diagram_from_context(project, diagram)?;
            for index in 0..diagram.properties.len() {
                add_ports(project, diagram, index)?;
            }
            fit_ancestors(diagram)?;
        }
        Ok((project.clone(), diagrams))
    })
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vehicle() -> (Project, IbdDiagram, [ElementId; 4], [ElementId; 3]) {
        let mut project = Project::new("Vehicle decomposition");
        let package = project
            .create_element(ElementKind::Package, "VehicleDefinitions", project.root_id)
            .unwrap();
        let blocks = ["Vehicle", "Engine", "Piston", "Ring"].map(|name| {
            project
                .create_element(ElementKind::Block, name, package)
                .unwrap()
        });
        let mut parts = Vec::new();
        for (index, (name, multiplicity)) in [("engine", 1), ("pistons", 4), ("rings", 3)]
            .into_iter()
            .enumerate()
        {
            parts.push(
                project
                    .create_typed_feature(
                        ElementKind::PartProperty,
                        name,
                        blocks[index],
                        blocks[index + 1],
                        Multiplicity::new(multiplicity, Some(multiplicity)).unwrap(),
                    )
                    .unwrap(),
            );
        }
        let mut diagram = IbdDiagram {
            id: uuid::Uuid::new_v4().to_string(),
            name: "Vehicle internal structure".into(),
            context_block_id: blocks[0].to_string(),
            owner_id: package.to_string(),
            context_frame: Some(super::super::ibd_geometry::default_context_frame()),
            properties: Vec::new(),
            boundary_ports: Vec::new(),
            connectors: Vec::new(),
        };
        ibd::populate_ibd_diagram_from_context(&project, &mut diagram).unwrap();
        (project, diagram, blocks, parts.try_into().unwrap())
    }

    #[test]
    fn vehicle_expansion_reuses_definitions_and_persists_contextual_occurrences() {
        let (mut project, mut diagram, blocks, parts) = vehicle();
        let second = project
            .create_typed_feature(
                ElementKind::PartProperty,
                "rearEngine",
                blocks[0],
                blocks[1],
                Multiplicity::ONE,
            )
            .unwrap();
        ibd::populate_ibd_diagram_from_context(&project, &mut diagram).unwrap();
        let original = serde_json::to_value(&project).unwrap();
        let engines: Vec<_> = diagram.properties.iter().map(|p| p.id.clone()).collect();
        for id in &engines {
            expand_property(&project, &mut diagram, id, true).unwrap();
        }
        let pistons: Vec<_> = diagram
            .properties
            .iter()
            .filter(|p| p.element_id == parts[1].to_string())
            .map(|p| p.id.clone())
            .collect();
        assert_eq!(pistons.len(), 2);
        for id in &pistons {
            expand_property(&project, &mut diagram, id, true).unwrap();
        }
        assert_eq!(diagram.properties.len(), 6);
        assert!(diagram.properties.iter().any(|p| p.property_path
            == vec![
                second.to_string(),
                parts[1].to_string(),
                parts[2].to_string()
            ]));
        let expanded = serde_json::to_value(&diagram).unwrap();
        for id in &engines {
            expand_property(&project, &mut diagram, id, true).unwrap();
        }
        assert_eq!(serde_json::to_value(&diagram).unwrap(), expanded);
        assert_eq!(serde_json::to_value(&project).unwrap(), original);
        expand_property(&project, &mut diagram, &engines[0], false).unwrap();
        assert_eq!(
            diagram
                .properties
                .iter()
                .filter(|p| property_visible(&diagram, p))
                .count(),
            4
        );
        let collapsed = serde_json::to_value(&diagram).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("vehicle.smproj");
        let mut database = ProjectDatabase::open(&path).unwrap();
        database.save_project(&project).unwrap();
        ibd::save_ibd_metadata(&mut database, &project, &[diagram]).unwrap();
        drop(database);
        let database = ProjectDatabase::open(&path).unwrap();
        let reopened = database.load_first_project().unwrap();
        let mut diagrams = ibd::load_ibd_metadata(&database, &reopened).unwrap();
        assert_eq!(serde_json::to_value(&reopened).unwrap(), original);
        assert_eq!(serde_json::to_value(&diagrams[0]).unwrap(), collapsed);
        expand_property(&reopened, &mut diagrams[0], &engines[0], true).unwrap();
        assert_eq!(serde_json::to_value(&diagrams[0]).unwrap(), expanded);
        for (index, part) in parts.iter().enumerate() {
            assert_eq!(
                reopened.element(*part).unwrap().owner_id,
                Some(blocks[index])
            );
            assert_eq!(
                reopened.element(*part).unwrap().type_id,
                Some(blocks[index + 1])
            );
        }
    }

    #[test]
    fn expansion_is_one_transaction_with_noop_rejection_and_legacy_defaults() {
        let (project, diagram, _, _) = vehicle();
        let workspace = WorkspaceState::default();
        let activity = ActivityWorkspaceState::default();
        let history = HistoryState::default();
        let id = diagram.id.clone();
        let part = diagram.properties[0].id.clone();
        let mut legacy = serde_json::to_value(&diagram).unwrap();
        legacy["properties"][0]
            .as_object_mut()
            .unwrap()
            .remove("collapsed");
        let legacy: IbdDiagram = serde_json::from_value(legacy).unwrap();
        assert!(!legacy.properties[0].collapsed);
        *workspace.project.lock().unwrap() = Some(project);
        *workspace.ibd_diagrams.lock().unwrap() = vec![legacy];
        edit_structure(&id, Some((&part, true)), &workspace, &activity, &history).unwrap();
        assert_eq!(history::undo_len(&history), 1);
        edit_structure(&id, Some((&part, true)), &workspace, &activity, &history).unwrap();
        assert_eq!(history::undo_len(&history), 1);
        assert!(history::undo_states(&workspace, &activity, &history).unwrap());
        assert!(
            edit_structure(
                &id,
                Some(("missing", true)),
                &workspace,
                &activity,
                &history
            )
            .is_err()
        );
        assert!(history::redo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(
            workspace.ibd_diagrams.lock().unwrap()[0].properties.len(),
            2
        );
    }

    #[test]
    fn recursive_types_allow_bounded_lazy_expansion_without_semantic_cycles() {
        let (mut project, mut diagram, blocks, _) = vehicle();
        project
            .create_typed_feature(
                ElementKind::PartProperty,
                "nested",
                blocks[1],
                blocks[1],
                Multiplicity::new(0, Some(1)).unwrap(),
            )
            .unwrap();
        project.validate().unwrap();
        let original = serde_json::to_value(&project).unwrap();
        let mut current = diagram.properties[0].id.clone();
        for _ in 1..MAX_PATH_DEPTH {
            expand_property(&project, &mut diagram, &current, true).unwrap();
            current = diagram
                .properties
                .iter()
                .filter(|p| {
                    project
                        .element(parse_element_id(&p.element_id).unwrap())
                        .unwrap()
                        .name
                        == "nested"
                })
                .max_by_key(|p| p.property_path.len())
                .unwrap()
                .id
                .clone();
        }
        assert!(expand_property(&project, &mut diagram, &current, true).is_err());
        assert_eq!(serde_json::to_value(&project).unwrap(), original);
        assert!(diagram.properties.len() < 100);
        ibd::validate_ibd_diagrams(&project, &[diagram]).unwrap();
    }
}
