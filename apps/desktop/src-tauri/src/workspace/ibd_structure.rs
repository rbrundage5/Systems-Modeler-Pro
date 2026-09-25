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
        separate_children(diagram, Some(&path));
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
    separate_children(diagram, None);
    if let Some(mut frame) = diagram.context_frame.clone() {
        for property in &diagram.properties {
            frame.width = frame.width.max(property.x + property.width + 32.0 - frame.x);
            frame.height = frame.height.max(property.y + property.height + 32.0 - frame.y);
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

/// Translate a presentation subtree; shared definitions and other usages are untouched.
fn translate_subtree(diagram: &mut IbdDiagram, id: &str, dx: f64, dy: f64) {
    let Some(path) = diagram.properties.iter().find(|p| p.id == id).map(|p| p.property_path.clone()) else { return; };
    for property in &mut diagram.properties {
        if property.id == id || is_descendant(&property.property_path, &path) {
            property.x += dx;
            property.y += dy;
            for port in &mut property.ports { port.x += dx; port.y += dy; }
        }
    }
}

pub(super) fn apply_property_geometry(
    diagram: &mut IbdDiagram,
    id: &str,
    rect: super::routing::RouteRect,
) -> Result<(), String> {
    if ![rect.x, rect.y, rect.width, rect.height].iter().all(|v| v.is_finite() && v.abs() <= 100_000.0)
        || rect.width < 60.0 || rect.height < 40.0 {
        return Err("IBD property geometry must be finite and within the canvas range".into());
    }
    let index = diagram.properties.iter().position(|p| p.id == id).ok_or("IBD property occurrence not found")?;
    let property = &diagram.properties[index];
    let old = super::ibd_geometry::property_rect(property);
    let path = property.property_path.clone();
    let dx = rect.x - old.x;
    let dy = rect.y - old.y;
    for child in &diagram.properties {
        if is_descendant(&child.property_path, &path)
            && (child.x + child.width + 16.0 > old.x + rect.width
                || child.y + child.height + 16.0 > old.y + rect.height) {
            return Err("Expanded property must remain large enough for its internal parts".into());
        }
        if is_descendant(&path, &child.property_path)
            && (rect.x < child.x + 16.0 || rect.y < child.y + 40.0) {
            return Err("Nested parts must stay inside their enclosing property's content area".into());
        }
    }
    translate_subtree(diagram, id, dx, dy);
    let translated = super::ibd_geometry::property_rect(&diagram.properties[index]);
    for port in &mut diagram.properties[index].ports {
        super::ibd_geometry::reanchor_port(port, translated, rect)?;
    }
    diagram.properties[index].width = rect.width;
    diagram.properties[index].height = rect.height;
    fit_ancestors(diagram)?;
    diagram.connectors = ibd::routed_ibd_connectors(diagram, None)?;
    Ok(())
}

pub(super) fn remove_property_presentation(diagram: &mut IbdDiagram, id: &str) -> bool {
    let Some(path) = diagram.properties.iter().find(|p| p.id == id).map(|p| p.property_path.clone()) else { return false; };
    let mut removed = HashSet::new();
    diagram.properties.retain(|p| {
        if p.id == id || is_descendant(&p.property_path, &path) {
            removed.insert(p.id.clone());
            removed.extend(p.ports.iter().map(|port| port.id.clone()));
            false
        } else { true }
    });
    diagram.connectors.retain(|edge| !removed.contains(&edge.source_presentation_id) && !removed.contains(&edge.target_presentation_id));
    true
}

fn separate_children(diagram: &mut IbdDiagram, parent: Option<&[String]>) {
    let mut children: Vec<_> = diagram.properties.iter().filter(|p| {
        if let Some(path) = parent {
            p.property_path.len() == path.len() + 1 && p.property_path.starts_with(path)
        } else { p.property_path.len() == 1 }
    }).cloned().collect();
    children.sort_by(|a, b| a.y.total_cmp(&b.y).then_with(|| a.x.total_cmp(&b.x)).then_with(|| a.id.cmp(&b.id)));
    let mut placed: Vec<super::routing::RouteRect> = Vec::new();
    for child in children {
        let mut y = child.y;
        for previous in &placed {
            if child.x < previous.x + previous.width + 16.0 && child.x + child.width + 16.0 > previous.x
                && y < previous.y + previous.height + 16.0 && y + child.height + 16.0 > previous.y {
                y = previous.y + previous.height + 24.0;
            }
        }
        translate_subtree(diagram, &child.id, 0.0, y - child.y);
        placed.push(super::routing::RouteRect { y, ..super::ibd_geometry::property_rect(&child) });
    }
}

pub(super) fn clean_groups(diagram: &mut IbdDiagram) -> Result<(), String> {
    let roots: Vec<_> = diagram.properties.iter().filter(|p| {
        !diagram.properties.iter().any(|parent| is_descendant(&p.property_path, &parent.property_path))
    }).cloned().collect();
    let root_for = |id: &str| diagram.properties.iter().find(|p| p.id == id || p.ports.iter().any(|port| port.id == id))
        .and_then(|p| roots.iter().find(|root| p.id == root.id || is_descendant(&p.property_path, &root.property_path)))
        .map(|root| root.id.clone());
    let edges: Vec<_> = diagram.connectors.iter().filter_map(|edge| {
        let source = root_for(&edge.source_presentation_id)?;
        let target = root_for(&edge.target_presentation_id)?;
        (source != target).then_some((source, target))
    }).collect();
    let positions = super::layout::hierarchical_positions_sized(
        roots.iter().map(|property| super::layout::LayoutNode { id: property.id.clone(), width: property.width, height: property.height }),
        &edges, systems_modeler_core::PreferredFlowDirection::LeftToRight,
    );
    for property in roots {
        if let Some((x, y)) = positions.get(&property.id) {
            // The context header and its ports remain outside the content area.
            let frame = diagram.context_frame.as_ref();
            let x = *x + frame.map_or(80.0, |f| f.x + 32.0);
            let y = *y + frame.map_or(110.0, |f| f.y + 48.0);
            translate_subtree(diagram, &property.id, x - property.x, y - property.y);
        }
    }
    fit_ancestors(diagram)
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
    #[test]
    fn contained_geometry_and_symbol_removal_preserve_definitions() {
        let (project, mut diagram, _, _) = vehicle();
        let original = serde_json::to_value(&project).unwrap();
        let engine = diagram.properties[0].id.clone();
        expand_property(&project, &mut diagram, &engine, true).unwrap();
        let piston = diagram.properties[1].id.clone();
        expand_property(&project, &mut diagram, &piston, true).unwrap();
        let positions: Vec<_> = diagram.properties.iter().map(|p| (p.x, p.y)).collect();
        let mut rect = super::super::ibd_geometry::property_rect(&diagram.properties[0]);
        rect.x += 160.0;
        rect.y += 90.0;
        apply_property_geometry(&mut diagram, &engine, rect).unwrap();
        for (p, (x, y)) in diagram.properties.iter().zip(positions) {
            assert_eq!((p.x, p.y), (x + 160.0, y + 90.0));
        }
        let before = serde_json::to_value(&diagram).unwrap();
        rect.width = 60.0;
        assert!(apply_property_geometry(&mut diagram, &engine, rect).is_err());
        assert_eq!(serde_json::to_value(&diagram).unwrap(), before);
        let offsets: Vec<_> = diagram.properties.iter().map(|p| (p.x - diagram.properties[0].x, p.y - diagram.properties[0].y)).collect();
        clean_groups(&mut diagram).unwrap();
        for (p, offset) in diagram.properties.iter().zip(offsets) {
            assert_eq!((p.x - diagram.properties[0].x, p.y - diagram.properties[0].y), offset);
        }
        assert!(remove_property_presentation(&mut diagram, &engine));
        assert!(diagram.properties.is_empty());
        assert_eq!(serde_json::to_value(&project).unwrap(), original);
    }

}
