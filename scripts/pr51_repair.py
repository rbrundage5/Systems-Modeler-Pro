from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old in text:
        return text.replace(old, new, 1)
    if new in text:
        return text
    raise SystemExit(f"{label} source was not found")


behavior_path = Path("apps/desktop/src-tauri/src/workspace/behavior_workspace.rs")
behavior = behavior_path.read_text(encoding="utf-8")
behavior = replace_once(
    behavior,
    "        let top = 110.0 + f64::from(execution.start.order) * 4.0;\n",
    "        let top = 130.0 + f64::from(execution.start.order) * 4.0;\n",
    "sequence execution start position",
)
behavior = replace_once(
    behavior,
    "        let bottom = 110.0 + f64::from(execution.finish.order) * 4.0;\n",
    "        let bottom = 130.0 + f64::from(execution.finish.order) * 4.0;\n",
    "sequence execution finish position",
)
behavior = replace_once(
    behavior,
    "        let y = 110.0 + f64::from(order) * 4.0;\n",
    """        // Keep the earliest message below the lifeline header plus the shared
        // routing clearance so Route/Clean Layout cannot generate geometry
        // that the obstacle-safe router must immediately reject.
        let y = 130.0 + f64::from(order) * 4.0;
""",
    "sequence message position",
)
behavior_path.write_text(behavior, encoding="utf-8")


script_path = Path("apps/desktop/src-tauri/src/workspace/model_script.rs")
script = script_path.read_text(encoding="utf-8")

old_property = """                        ElementKind::PartProperty | ElementKind::ReferenceProperty => {
                            native.properties.push(IbdPropertyPresentation {
                                id: uuid::Uuid::new_v4().to_string(),
                                element_id: feature.id.to_string(),
                                property_path: vec![feature.id.to_string()],
                                x,
                                y,
                                width: 190.0,
                                height: 100.0,
                                ports: Vec::new(),
                            });
                            x += 240.0;
                            if x > 780.0 {
                                x = 120.0;
                                y += 180.0;
                            }
                        }
"""
new_property = """                        ElementKind::PartProperty | ElementKind::ReferenceProperty => {
                            // A semantic connector end may terminate at a port owned by the
                            // property's type. Populate those native nested-port presentations
                            // up front so presentation endpoints can reconstruct ConnectorEnd
                            // exactly rather than falling back to the owning property box.
                            let typed_ports = feature
                                .type_id
                                .map(|type_id| {
                                    project
                                        .children(type_id)
                                        .filter(|candidate| candidate.is_port())
                                        .map(|candidate| candidate.id)
                                        .collect::<Vec<_>>()
                                })
                                .unwrap_or_default();
                            let port_rows = typed_ports.len().div_ceil(2);
                            let height = 100.0_f64.max(52.0 + port_rows as f64 * 28.0);
                            let ports = typed_ports
                                .into_iter()
                                .enumerate()
                                .map(|(index, port_id)| {
                                    let right_side = index % 2 == 0;
                                    let row = index / 2;
                                    IbdPortPresentation {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        element_id: port_id.to_string(),
                                        property_path: vec![feature.id.to_string()],
                                        x: if right_side { x + 190.0 } else { x },
                                        y: y + 34.0 + row as f64 * 28.0,
                                        size: 16.0,
                                    }
                                })
                                .collect();
                            native.properties.push(IbdPropertyPresentation {
                                id: uuid::Uuid::new_v4().to_string(),
                                element_id: feature.id.to_string(),
                                property_path: vec![feature.id.to_string()],
                                x,
                                y,
                                width: 190.0,
                                height,
                                ports,
                            });
                            x += 240.0;
                            if x > 780.0 {
                                x = 120.0;
                                y += height + 80.0;
                            }
                        }
"""
script = replace_once(script, old_property, new_property, "IBD property population")

old_endpoint = """                    let endpoint = |role: ElementId, port: Option<ElementId>| -> Option<String> {
                        let wanted = port.unwrap_or(role).to_string();
                        native
                            .boundary_ports
                            .iter()
                            .find(|p| p.element_id == wanted)
                            .map(|p| p.id.clone())
                            .or_else(|| {
                                native
                                    .properties
                                    .iter()
                                    .find(|p| p.element_id == role.to_string())
                                    .map(|p| p.id.clone())
                            })
                    };
                    if let (Some(source), Some(target)) = (
                        endpoint(connector.source.role_id, connector.source.port_id),
                        endpoint(connector.target.role_id, connector.target.port_id),
                    ) {
"""
new_endpoint = """                    let endpoint = |end: &systems_modeler_core::ConnectorEnd| -> Option<String> {
                        let path = end
                            .property_path
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>();
                        match end.port_id {
                            Some(port_id) if path.is_empty() => native
                                .boundary_ports
                                .iter()
                                .find(|presentation| {
                                    presentation.element_id == port_id.to_string()
                                        && presentation.property_path.is_empty()
                                })
                                .map(|presentation| presentation.id.clone()),
                            Some(port_id) => native
                                .properties
                                .iter()
                                .flat_map(|property| property.ports.iter())
                                .find(|presentation| {
                                    presentation.element_id == port_id.to_string()
                                        && presentation.property_path == path
                                })
                                .map(|presentation| presentation.id.clone()),
                            None => native
                                .properties
                                .iter()
                                .find(|presentation| {
                                    presentation.element_id == end.role_id.to_string()
                                        && presentation.property_path == path
                                })
                                .map(|presentation| presentation.id.clone()),
                        }
                    };
                    if let (Some(source), Some(target)) = (
                        endpoint(&connector.source),
                        endpoint(&connector.target),
                    ) {
"""
script = replace_once(script, old_endpoint, new_endpoint, "IBD connector endpoint population")

old_bdd_push = """            workspace
                .diagrams
                .lock()
                .map_err(|_| "diagram lock poisoned")?
                .push(native);
            Ok(id)
"""
new_bdd_push = """            let mut diagrams = workspace
                .diagrams
                .lock()
                .map_err(|_| "diagram lock poisoned")?;
            diagrams.retain(|existing| existing.id != id);
            diagrams.push(native);
            Ok(id)
"""
script = replace_once(script, old_bdd_push, new_bdd_push, "BDD-like stable diagram replacement")

old_ibd_push = """            workspace
                .ibd_diagrams
                .lock()
                .map_err(|_| "IBD lock poisoned")?
                .push(native);
            Ok(id)
"""
new_ibd_push = """            let mut diagrams = workspace
                .ibd_diagrams
                .lock()
                .map_err(|_| "IBD lock poisoned")?;
            diagrams.retain(|existing| existing.id != id);
            diagrams.push(native);
            Ok(id)
"""
script = replace_once(script, old_ibd_push, new_ibd_push, "IBD stable diagram replacement")

old_activity_push = """            activity
                .diagrams
                .lock()
                .map_err(|_| "Activity diagram lock poisoned")?
                .push(native);
            Ok(id)
"""
new_activity_push = """            let mut diagrams = activity
                .diagrams
                .lock()
                .map_err(|_| "Activity diagram lock poisoned")?;
            diagrams.retain(|existing| existing.id != id);
            diagrams.push(native);
            Ok(id)
"""
script = replace_once(
    script,
    old_activity_push,
    new_activity_push,
    "Activity stable diagram replacement",
)

old_behavior_push = """            workspace
                .behavior_diagrams
                .lock()
                .map_err(|_| "behavior diagram lock poisoned")?
                .push(native);
            Ok(id)
"""
new_behavior_push = """            let mut diagrams = workspace
                .behavior_diagrams
                .lock()
                .map_err(|_| "behavior diagram lock poisoned")?;
            diagrams.retain(|existing| existing.id != id);
            diagrams.push(native);
            Ok(id)
"""
script = replace_once(
    script,
    old_behavior_push,
    new_behavior_push,
    "Behavior stable diagram replacement",
)

script_path.write_text(script, encoding="utf-8")
