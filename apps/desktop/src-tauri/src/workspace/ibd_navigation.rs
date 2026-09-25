use super::*;

fn open_type_ibd(
    element_id: ElementId,
    workspace: &WorkspaceState,
    activity: &ActivityWorkspaceState,
    history: &HistoryState,
) -> Result<String, String> {
    let mut result = String::new();
    history::apply_structural_specification(workspace, activity, history, |project, diagrams| {
        let selected = project
            .element(element_id)
            .map_err(|error| error.to_string())?;
        let context = if matches!(
            selected.kind,
            ElementKind::PartProperty | ElementKind::ReferenceProperty
        ) {
            project
                .element(
                    selected
                        .type_id
                        .ok_or("The selected property has no type")?,
                )
                .map_err(|error| error.to_string())?
        } else {
            selected
        };
        if !matches!(
            context.kind,
            ElementKind::Block | ElementKind::AssociationBlock
        ) {
            return Err("Select a Block or a Block-typed part/reference property".into());
        }
        if let Some(existing) = diagrams
            .iter()
            .find(|d| d.context_block_id == context.id.to_string())
        {
            result = existing.id.clone();
            return Ok((project.clone(), diagrams.to_vec()));
        }
        let mut owner_id = context.owner_id.ok_or("Block has no repository owner")?;
        loop {
            let owner = project
                .element(owner_id)
                .map_err(|error| error.to_string())?;
            if matches!(owner.kind, ElementKind::Model | ElementKind::Package) {
                break;
            }
            owner_id = owner.owner_id.ok_or("No package owner for type IBD")?;
        }
        let mut diagram = ibd::IbdDiagram {
            id: DiagramId::new().to_string(),
            name: format!("{} Internal Structure", context.name),
            context_block_id: context.id.to_string(),
            owner_id: owner_id.to_string(),
            context_frame: Some(super::ibd_geometry::default_context_frame()),
            properties: Vec::new(),
            boundary_ports: Vec::new(),
            connectors: Vec::new(),
        };
        ibd::populate_ibd_diagram_from_context(project, &mut diagram)?;
        for index in 0..diagram.properties.len() {
            super::ibd_structure::add_ports(project, &mut diagram, index)?;
        }
        super::ibd_structure::fit_ancestors(&mut diagram)?;
        super::ibd_projection::show_existing_connectors(project, &mut diagram)?;
        result = diagram.id.clone();
        let mut diagrams = diagrams.to_vec();
        diagrams.push(diagram);
        Ok((project.clone(), diagrams))
    })?;
    Ok(result)
}

#[tauri::command]
pub fn open_or_create_type_ibd(
    element_id: String,
    state: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, ActivityWorkspaceState>,
    history: tauri::State<'_, HistoryState>,
) -> Result<String, String> {
    open_type_ibd(parse_element_id(&element_id)?, &state, &activity, &history)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_navigation_reuses_definition_and_diagram_with_one_history_entry() {
        let workspace = WorkspaceState::default();
        let activity = ActivityWorkspaceState::default();
        let history = HistoryState::default();
        let mut project = Project::new("Type navigation");
        let root = project.root_id;
        let system = project
            .create_element(ElementKind::Block, "Vehicle", root)
            .unwrap();
        let engine = project
            .create_element(ElementKind::Block, "Engine", root)
            .unwrap();
        let part = project
            .create_typed_feature(
                ElementKind::PartProperty,
                "engine",
                system,
                engine,
                Multiplicity::ONE,
            )
            .unwrap();
        let original = serde_json::to_value(&project).unwrap();
        *workspace.project.lock().unwrap() = Some(project);
        let id = open_type_ibd(part, &workspace, &activity, &history).unwrap();
        assert_eq!(
            workspace.ibd_diagrams.lock().unwrap()[0].context_block_id,
            engine.to_string()
        );
        assert_eq!(history::undo_len(&history), 1);
        assert_eq!(
            open_type_ibd(engine, &workspace, &activity, &history).unwrap(),
            id
        );
        assert_eq!(history::undo_len(&history), 1);
        assert!(history::undo_states(&workspace, &activity, &history).unwrap());
        assert!(open_type_ibd(root, &workspace, &activity, &history).is_err());
        assert!(history::redo_states(&workspace, &activity, &history).unwrap());
        assert_eq!(workspace.ibd_diagrams.lock().unwrap()[0].id, id);
        assert_eq!(
            serde_json::to_value(workspace.project.lock().unwrap().as_ref().unwrap()).unwrap(),
            original
        );
    }
}
