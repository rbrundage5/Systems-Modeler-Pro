//! Explicit presentation allocation of a reusable property's population.
use super::ibd::{IbdDiagram, IbdPropertyPresentation};
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OccurrenceRange {
    /// One-based display position in the population; not a semantic element ID.
    pub first: u32,
    pub count: u32,
}

pub(super) fn range_at(
    property: &IbdPropertyPresentation,
    index: usize,
) -> Option<OccurrenceRange> {
    property.occurrence_path.get(index).copied().flatten()
}

pub(super) fn ranges_equal(
    a: &[Option<OccurrenceRange>],
    b: &[Option<OccurrenceRange>],
    depth: usize,
) -> bool {
    (0..depth).all(|index| a.get(index).copied().flatten() == b.get(index).copied().flatten())
}

pub(super) fn same_prefix(
    a: &IbdPropertyPresentation,
    b: &IbdPropertyPresentation,
    depth: usize,
) -> bool {
    a.property_path.len() >= depth
        && b.property_path.len() >= depth
        && a.property_path[..depth] == b.property_path[..depth]
        && (0..depth).all(|index| range_at(a, index) == range_at(b, index))
}

pub(super) fn endpoint_property<'a>(
    diagram: &'a IbdDiagram,
    id: &str,
) -> Option<&'a IbdPropertyPresentation> {
    diagram
        .properties
        .iter()
        .find(|property| property.id == id || property.ports.iter().any(|port| port.id == id))
}

pub(super) fn endpoint_matches_context(
    diagram: &IbdDiagram,
    id: &str,
    ranges: &[Option<OccurrenceRange>],
) -> bool {
    let Some(property) = endpoint_property(diagram, id) else {
        return ranges.iter().all(Option::is_none);
    };
    (0..property.property_path.len())
        .all(|index| range_at(property, index) == ranges.get(index).copied().flatten())
}

pub(super) fn validate(diagram: &IbdDiagram, project: &Project) -> Result<(), String> {
    for property in &diagram.properties {
        if property.occurrence_path.len() > property.property_path.len() {
            return Err("Occurrence selections must follow the semantic property path".into());
        }
        for (index, range) in property.occurrence_path.iter().enumerate() {
            let Some(range) = range else {
                continue;
            };
            let definition = project
                .element(parse_element_id(&property.property_path[index])?)
                .map_err(|error| error.to_string())?;
            let upper = definition.multiplicity.unwrap_or(Multiplicity::ONE).upper;
            let end = u64::from(range.first) + u64::from(range.count);
            if range.first == 0
                || range.count == 0
                || upper.is_some_and(|upper| end - 1 > u64::from(upper))
            {
                return Err(format!(
                    "Occurrence allocation for '{}' exceeds multiplicity [{}]; adjust its displayed groups first",
                    definition.name,
                    definition
                        .multiplicity
                        .unwrap_or(Multiplicity::ONE)
                        .notation()
                ));
            }
        }
        let depth = property.property_path.len();
        if depth == 0 {
            continue;
        }
        let Some(range) = range_at(property, depth - 1) else {
            continue;
        };
        let name = property.occurrence_name.as_deref().unwrap_or("").trim();
        if name.is_empty() {
            return Err("Allocated occurrences require a display name".into());
        }
        for other in &diagram.properties {
            if property.id == other.id
                || other.property_path != property.property_path
                || !same_prefix(property, other, depth - 1)
            {
                continue;
            }
            let Some(other_range) = range_at(other, depth - 1) else {
                continue;
            };
            if range == other_range && property.occurrence_name == other.occurrence_name {
                continue;
            }
            if property.occurrence_name == other.occurrence_name {
                return Err("Different occurrence groups require distinct display names".into());
            }
            if u64::from(range.first) < u64::from(other_range.first) + u64::from(other_range.count)
                && u64::from(other_range.first) < u64::from(range.first) + u64::from(range.count)
            {
                return Err("Occurrence groups overlap; repeating a symbol does not allocate another instance".into());
            }
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct OccurrenceCoverage {
    pub displayed: Option<u64>,
    pub constraint: String,
    pub status: String,
}

pub(super) fn coverage(
    project: &Project,
    diagram: &IbdDiagram,
    property: &IbdPropertyPresentation,
) -> Result<OccurrenceCoverage, String> {
    let definition = project
        .element(parse_element_id(&property.element_id)?)
        .map_err(|error| error.to_string())?;
    let multiplicity = definition.multiplicity.unwrap_or(Multiplicity::ONE);
    let depth = property.property_path.len();
    if depth == 0 {
        return Err("Occurrence coverage requires a resolved property path".into());
    }
    let mut ranges = HashSet::new();
    let mut compact = false;
    for peer in &diagram.properties {
        if peer.property_path != property.property_path
            || !same_prefix(property, peer, depth.saturating_sub(1))
        {
            continue;
        }
        if let Some(range) = range_at(peer, depth - 1) {
            ranges.insert(range);
        } else {
            compact = true;
        }
    }
    let total: u64 = ranges.iter().map(|range| u64::from(range.count)).sum();
    let status = if compact {
        "Compact population"
    } else if multiplicity.upper == Some(multiplicity.lower)
        && total == u64::from(multiplicity.lower)
    {
        "Complete population"
    } else if multiplicity
        .upper
        .is_some_and(|upper| total > u64::from(upper))
    {
        "Exceeds definition"
    } else {
        "Partial population view"
    };
    Ok(OccurrenceCoverage {
        displayed: (!compact).then_some(total),
        constraint: multiplicity.notation(),
        status: status.into(),
    })
}

type OccurrenceGroup = (Option<String>, Option<OccurrenceRange>);

fn parse_groups(text: Option<&str>) -> Result<Vec<OccurrenceGroup>, String> {
    let Some(text) = text else {
        return Ok(vec![(None, None)]);
    };
    let mut result = Vec::new();
    let mut first = 1u32;
    let mut names = HashSet::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let (name, count) = line
            .rsplit_once('=')
            .ok_or("Use one occurrence group per line: display name = count")?;
        let name = name.trim();
        let count = count
            .trim()
            .parse::<u32>()
            .map_err(|_| "Occurrence counts must be positive whole numbers")?;
        if name.is_empty() || count == 0 || !names.insert(name.to_owned()) {
            return Err(
                "Occurrence groups require distinct nonempty names and positive counts".into(),
            );
        }
        result.push((
            Some(name.to_owned()),
            Some(OccurrenceRange { first, count }),
        ));
        first = first
            .checked_add(count)
            .ok_or("Occurrence count exceeds the supported range")?;
        if result.len() > 4096 {
            return Err("An occurrence view supports at most 4096 displayed groups".into());
        }
    }
    if result.is_empty() {
        return Err("Enter at least one occurrence group or choose Compact".into());
    }
    Ok(result)
}

pub(super) fn set_groups(
    project: &Project,
    diagram: &mut IbdDiagram,
    id: &str,
    text: Option<&str>,
) -> Result<(), String> {
    set_allocations(project, diagram, id, parse_groups(text)?)
}

fn set_allocations(
    project: &Project,
    diagram: &mut IbdDiagram,
    id: &str,
    requested: Vec<OccurrenceGroup>,
) -> Result<(), String> {
    let selected = diagram
        .properties
        .iter()
        .find(|property| property.id == id)
        .cloned()
        .ok_or("IBD property occurrence not found")?;
    let depth = selected.property_path.len();
    if depth == 0 {
        return Err("Occurrence view requires a resolved property path".into());
    }
    let peers: Vec<_> = diagram
        .properties
        .iter()
        .filter(|property| {
            property.property_path == selected.property_path
                && same_prefix(property, &selected, depth - 1)
        })
        .cloned()
        .collect();
    let original = diagram.clone();
    let unchanged_ranges = requested.len() == peers.len()
        && requested
            .iter()
            .all(|(_, range)| peers.iter().any(|peer| range_at(peer, depth - 1) == *range));
    if unchanged_ranges {
        for property in &mut diagram.properties {
            if peers.iter().any(|peer| peer.id == property.id) {
                property.occurrence_name = requested
                    .iter()
                    .find(|(_, range)| *range == range_at(property, depth - 1))
                    .unwrap()
                    .0
                    .clone();
            }
        }
        return validate(diagram, project);
    }
    let belongs = |property: &IbdPropertyPresentation| {
        peers.iter().any(|peer| {
            property.id == peer.id || super::ibd_structure::is_descendant(property, peer)
        })
    };
    diagram.properties.retain(|property| !belongs(property));
    let mut next_y = original
        .properties
        .iter()
        .filter(|property| {
            property.property_path.len() == depth && same_prefix(property, &selected, depth - 1)
        })
        .map(|property| property.y + property.height + 32.0)
        .fold(selected.y, f64::max);
    let mut reused = HashSet::new();
    for (index, (name, range)) in requested.into_iter().enumerate() {
        let existing = peers
            .iter()
            .find(|peer| !reused.contains(&peer.id) && range_at(peer, depth - 1) == range)
            .or_else(|| (index == 0).then_some(&selected));
        let template = existing.unwrap_or(&selected);
        let keep_ids = existing.is_some() && reused.insert(template.id.clone());
        let dy = if keep_ids {
            0.0
        } else {
            let offset = next_y - template.y;
            next_y += template.height + 32.0;
            offset
        };
        for source in original.properties.iter().filter(|property| {
            property.id == template.id || super::ibd_structure::is_descendant(property, template)
        }) {
            let mut property = source.clone();
            property
                .occurrence_path
                .resize(property.property_path.len(), None);
            property.occurrence_path[depth - 1] = range;
            if property.id == template.id {
                property.occurrence_name = name.clone();
            }
            if !keep_ids {
                property.id = uuid::Uuid::new_v4().to_string();
            }
            property.y += dy;
            for port in &mut property.ports {
                if !keep_ids {
                    port.id = uuid::Uuid::new_v4().to_string();
                }
                port.y += dy;
            }
            diagram.properties.push(property);
            if diagram.properties.len() > 4096 {
                return Err("This view is limited to 4096 property occurrences".into());
            }
        }
    }
    validate(diagram, project)?;
    // Keep connector presentations whose endpoint occurrences survived intact.
    // Aggregate endpoints replaced by subsets must not be silently redirected.
    diagram.connectors = original
        .connectors
        .into_iter()
        .filter(|edge| {
            [&edge.source_presentation_id, &edge.target_presentation_id]
                .iter()
                .all(|id| {
                    super::ibd::ibd_end_for_presentation(diagram, id).is_ok()
                        && endpoint_matches_context(diagram, id, &edge.context_occurrence_path)
                })
        })
        .collect();
    super::ibd_structure::fit_ancestors(diagram)?;
    super::ibd_projection::show_existing_connectors(project, diagram)?;
    diagram.connectors = super::ibd::routed_ibd_connectors(diagram, None)?;
    Ok(())
}

#[tauri::command]
pub fn set_ibd_occurrence_groups(
    diagram_id: String,
    presentation_id: String,
    groups: Option<String>,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, super::activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, super::history::HistoryState>,
) -> Result<bool, String> {
    super::history::apply_structural_specification(
        &workspace,
        &activity,
        &history,
        |project, diagrams| {
            let mut staged = diagrams.to_vec();
            let diagram = staged
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("IBD not found")?;
            set_groups(project, diagram, &presentation_id, groups.as_deref())?;
            super::ibd::validate_ibd_diagrams(project, &staged)?;
            Ok((project.clone(), staged))
        },
    )
}

#[derive(Debug, Serialize)]
pub struct OccurrenceSpecification {
    pub groups: String,
    pub suggested_groups: String,
    pub coverage: OccurrenceCoverage,
    pub definition_id: String,
    pub type_id: Option<String>,
}

#[tauri::command]
pub fn ibd_occurrence_specification(
    diagram_id: String,
    presentation_id: String,
    workspace: tauri::State<'_, WorkspaceState>,
) -> Result<OccurrenceSpecification, String> {
    let guard = workspace
        .project
        .lock()
        .map_err(|_| "project lock poisoned")?;
    let project = guard.as_ref().ok_or("no project open")?;
    let diagrams = workspace
        .ibd_diagrams
        .lock()
        .map_err(|_| "IBD lock poisoned")?;
    let diagram = diagrams
        .iter()
        .find(|diagram| diagram.id == diagram_id)
        .ok_or("IBD not found")?;
    let property = diagram
        .properties
        .iter()
        .find(|property| property.id == presentation_id)
        .ok_or("IBD occurrence not found")?;
    let definition = project
        .element(parse_element_id(&property.element_id)?)
        .map_err(|error| error.to_string())?;
    let depth = property.property_path.len();
    let mut peers: Vec<_> = diagram
        .properties
        .iter()
        .filter(|peer| {
            peer.property_path == property.property_path
                && same_prefix(peer, property, depth.saturating_sub(1))
        })
        .collect();
    peers.sort_by_key(|peer| range_at(peer, depth.saturating_sub(1)).map(|range| range.first));
    let mut seen = HashSet::new();
    let groups = peers
        .into_iter()
        .filter_map(|peer| {
            let range = range_at(peer, depth.saturating_sub(1))?;
            seen.insert(range).then(|| {
                format!(
                    "{} = {}",
                    peer.occurrence_name.as_deref().unwrap_or(&definition.name),
                    range.count
                )
            })
        })
        .collect::<Vec<_>>()
        .join("\n");
    let multiplicity = definition.multiplicity.unwrap_or(Multiplicity::ONE);
    let count = if multiplicity.upper == Some(0) {
        0
    } else {
        multiplicity.lower.max(1)
    };
    let suggested_groups = if count <= 128 {
        (1..=count)
            .map(|index| format!("{}{} = 1", definition.name, index))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        format!("{} group = {}", definition.name, count)
    };
    Ok(OccurrenceSpecification {
        groups,
        suggested_groups,
        coverage: coverage(project, diagram, property)?,
        definition_id: definition.id.to_string(),
        type_id: definition.type_id.map(|id| id.to_string()),
    })
}

#[derive(Debug, Deserialize)]
pub struct OccurrenceDrop {
    pub element_id: String,
    pub parent_presentation_id: Option<String>,
    pub name: String,
    pub count: String,
    pub x: f64,
    pub y: f64,
}

fn append_group(
    project: &Project,
    diagram: &mut IbdDiagram,
    request: &OccurrenceDrop,
) -> Result<String, String> {
    if ![request.x, request.y]
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0 && *value <= 100_000.0)
    {
        return Err("Occurrence drop coordinates must be within the canvas".into());
    }
    let property_id = parse_element_id(&request.element_id)?;
    let definition = project
        .element(property_id)
        .map_err(|error| error.to_string())?;
    if !matches!(
        definition.kind,
        ElementKind::PartProperty | ElementKind::ReferenceProperty
    ) {
        return Err("Drag an existing part/reference property. Its Block type remains a reusable definition.".into());
    }
    let mut parent = request
        .parent_presentation_id
        .as_ref()
        .map(|id| {
            diagram
                .properties
                .iter()
                .find(|property| &property.id == id)
                .cloned()
                .ok_or("Containing occurrence not found")
        })
        .transpose()?;
    if let Some(target) = parent
        .as_ref()
        .filter(|target| target.element_id == request.element_id)
    {
        let depth = target.property_path.len();
        parent = if depth <= 1 {
            None
        } else {
            Some(
                diagram
                    .properties
                    .iter()
                    .find(|candidate| {
                        candidate.property_path.len() == depth - 1
                            && same_prefix(candidate, target, depth - 1)
                    })
                    .cloned()
                    .ok_or("Enclosing occurrence is not presented")?,
            )
        };
    }
    let mut path = parent
        .as_ref()
        .map(|property| property.property_path.clone())
        .unwrap_or_default();
    let parsed = path
        .iter()
        .map(|id| parse_element_id(id))
        .collect::<Result<Vec<_>, _>>()?;
    let context = project
        .resolve_structural_path(parse_element_id(&diagram.context_block_id)?, &parsed)
        .map_err(|error| error.to_string())?;
    if !project
        .has_classifier_feature(context, property_id)
        .map_err(|error| error.to_string())?
    {
        return Err("This property is not owned or inherited by the drop context. Drop onto an occurrence of its owning type, or open that type's IBD.".into());
    }
    path.push(request.element_id.clone());
    if path.len() > 32 {
        return Err("Occurrence expansion is limited to 32 levels".into());
    }
    let mut ranges = parent
        .as_ref()
        .map(|property| property.occurrence_path.clone())
        .unwrap_or_default();
    ranges.resize(path.len(), None);
    let mut peers: Vec<_> = diagram
        .properties
        .iter()
        .filter(|property| {
            property.property_path == path
                && ranges_equal(&property.occurrence_path, &ranges, path.len() - 1)
        })
        .cloned()
        .collect();
    peers.sort_by_key(|property| range_at(property, path.len() - 1).map(|range| range.first));
    let id = if let Some(property) = peers.first() {
        property.id.clone()
    } else {
        let id = uuid::Uuid::new_v4().to_string();
        diagram.properties.push(IbdPropertyPresentation {
            id: id.clone(),
            element_id: request.element_id.clone(),
            property_path: path.clone(),
            occurrence_path: ranges.clone(),
            occurrence_name: None,
            collapsed: false,
            x: request
                .x
                .max(parent.as_ref().map_or(0.0, |property| property.x + 24.0)),
            y: request
                .y
                .max(parent.as_ref().map_or(48.0, |property| property.y + 48.0)),
            width: 220.0,
            height: 100.0,
            ports: Vec::new(),
        });
        let index = diagram.properties.len() - 1;
        super::ibd_structure::add_ports(project, diagram, index)?;
        id
    };
    let mut seen = HashSet::new();
    let mut groups: Vec<_> = peers
        .iter()
        .filter_map(|property| {
            let range = range_at(property, path.len() - 1)?;
            seen.insert(range)
                .then(|| (property.occurrence_name.clone(), Some(range)))
        })
        .collect();
    if request.name.contains(['\n', '\r', '=']) || request.count.contains(['\n', '\r', '=']) {
        return Err("Enter one occurrence name and one count".into());
    }
    let parsed_group = parse_groups(Some(&format!("{} = {}", request.name, request.count)))?;
    let count = parsed_group[0].1.unwrap().count;
    let mut first = 1u32;
    for (_, range) in &groups {
        let range = range.unwrap();
        if u64::from(first) + u64::from(count) <= u64::from(range.first) {
            break;
        }
        first = range
            .first
            .checked_add(range.count)
            .ok_or("Occurrence allocation exceeds the supported range")?;
    }
    groups.push((
        Some(request.name.trim().into()),
        Some(OccurrenceRange { first, count }),
    ));
    set_allocations(project, diagram, &id, groups)?;
    diagram
        .properties
        .iter()
        .find(|property| {
            property.property_path == path
                && ranges_equal(&property.occurrence_path, &ranges, path.len() - 1)
                && property.occurrence_name.as_deref() == Some(request.name.trim())
        })
        .map(|property| property.id.clone())
        .ok_or("New occurrence was not presented".into())
}

#[tauri::command]
pub fn append_ibd_occurrence(
    diagram_id: String,
    request: OccurrenceDrop,
    workspace: tauri::State<'_, WorkspaceState>,
    activity: tauri::State<'_, super::activity_workspace::ActivityWorkspaceState>,
    history: tauri::State<'_, super::history::HistoryState>,
) -> Result<String, String> {
    let mut created = None;
    super::history::apply_structural_specification(
        &workspace,
        &activity,
        &history,
        |project, diagrams| {
            let mut staged = diagrams.to_vec();
            let diagram = staged
                .iter_mut()
                .find(|diagram| diagram.id == diagram_id)
                .ok_or("IBD not found")?;
            created = Some(append_group(project, diagram, &request)?);
            super::ibd::validate_ibd_diagrams(project, &staged)?;
            Ok((project.clone(), staged))
        },
    )?;
    created.ok_or("Occurrence was not created".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use systems_modeler_core::{Connector, ConnectorEnd, ConnectorKind};

    fn fixture() -> (Project, IbdDiagram) {
        let (mut project, mut diagram) = super::super::ibd_geometry::tests::fixture();
        let property = parse_element_id(&diagram.properties[0].element_id).unwrap();
        project.element_mut(property).unwrap().multiplicity =
            Some(Multiplicity::new(4, Some(4)).unwrap());
        let component = project.element(property).unwrap().type_id.unwrap();
        let port = parse_element_id(&diagram.properties[0].ports[0].element_id).unwrap();
        let interface = project.element(port).unwrap().type_id.unwrap();
        let leaf = project
            .create_element(ElementKind::Block, "Channel", project.root_id)
            .unwrap();
        let child = project
            .create_typed_feature(
                ElementKind::PartProperty,
                "channels",
                component,
                leaf,
                Multiplicity::new(2, Some(2)).unwrap(),
            )
            .unwrap();
        let child_port = project
            .create_typed_feature(
                ElementKind::ProxyPort,
                "input",
                leaf,
                interface,
                Multiplicity::ONE,
            )
            .unwrap();
        project
            .create_connector(Connector {
                context_id: component,
                kind: ConnectorKind::Delegation,
                source: ConnectorEnd::boundary(port),
                target: ConnectorEnd::nested_port(vec![child], child_port),
                association_type_id: None,
                end_multiplicities: Default::default(),
            })
            .unwrap();
        super::super::ibd_structure::expand_property(&project, &mut diagram, "part", true).unwrap();
        (project, diagram)
    }

    #[test]
    fn repeated_drops_reuse_the_property_and_preserve_existing_allocations() {
        let (project, mut diagram) = fixture();
        let semantic = serde_json::to_value(&project).unwrap();
        let element_id = diagram.properties[0].element_id.clone();
        let mut request = OccurrenceDrop {
            element_id,
            parent_presentation_id: Some("part".into()),
            name: "north".into(),
            count: "1".into(),
            x: 140.0,
            y: 180.0,
        };
        let mut ids = Vec::new();
        let mut first_connector = None;
        for name in ["north", "south", "east", "west"] {
            request.name = name.into();
            ids.push(append_group(&project, &mut diagram, &request).unwrap());
            super::super::ibd::validate_ibd_diagrams(&project, &[diagram.clone()]).unwrap();
            if let Some(id) = &first_connector {
                assert!(diagram.connectors.iter().any(|edge| &edge.id == id));
            } else {
                first_connector = Some(diagram.connectors[0].id.clone());
            }
        }
        assert_eq!(ids.iter().collect::<HashSet<_>>().len(), 4);
        let north = diagram
            .properties
            .iter()
            .find(|property| property.id == ids[0])
            .unwrap();
        assert_eq!(
            coverage(&project, &diagram, north).unwrap().displayed,
            Some(4)
        );
        assert_eq!(
            range_at(north, 0),
            Some(OccurrenceRange { first: 1, count: 1 })
        );
        let before = serde_json::to_value(&diagram).unwrap();
        let mut staged = diagram.clone();
        request.name = "excess".into();
        assert!(
            append_group(&project, &mut staged, &request)
                .unwrap_err()
                .contains("exceeds")
        );
        assert_eq!(serde_json::to_value(&diagram).unwrap(), before);
        // Removing a view leaves a hole; a later drop fills it without renumbering peers.
        assert!(super::super::ibd_structure::remove_property_presentation(
            &mut diagram,
            &ids[1]
        ));
        request.name = "replacement".into();
        let id = append_group(&project, &mut diagram, &request).unwrap();
        let replacement = diagram
            .properties
            .iter()
            .find(|property| property.id == id)
            .unwrap();
        let west = diagram
            .properties
            .iter()
            .find(|property| property.id == ids[3])
            .unwrap();
        assert!(replacement.y >= west.y + west.height + 32.0);
        assert_eq!(
            range_at(
                diagram
                    .properties
                    .iter()
                    .find(|property| property.id == id)
                    .unwrap(),
                0
            ),
            Some(OccurrenceRange { first: 2, count: 1 })
        );
        assert_eq!(
            range_at(
                diagram
                    .properties
                    .iter()
                    .find(|property| property.id == ids[2])
                    .unwrap(),
                0
            ),
            Some(OccurrenceRange { first: 3, count: 1 })
        );
        assert_eq!(serde_json::to_value(&project).unwrap(), semantic);
    }

    #[test]
    fn nested_repeat_drops_resolve_the_containing_population_and_reject_wrong_context() {
        let (project, mut diagram) = fixture();
        set_groups(
            &project,
            &mut diagram,
            "part",
            Some("first = 1\nsecond = 1"),
        )
        .unwrap();
        let first = diagram
            .properties
            .iter()
            .find(|property| property.occurrence_name.as_deref() == Some("first"))
            .unwrap()
            .clone();
        let second = diagram
            .properties
            .iter()
            .find(|property| property.occurrence_name.as_deref() == Some("second"))
            .unwrap()
            .clone();
        let child_id = diagram
            .properties
            .iter()
            .find(|property| super::super::ibd_structure::is_descendant(property, &first))
            .unwrap()
            .element_id
            .clone();
        let mut request = OccurrenceDrop {
            element_id: child_id,
            parent_presentation_id: Some(first.id),
            name: "channelA".into(),
            count: "1".into(),
            x: 160.0,
            y: 200.0,
        };
        let first_child = append_group(&project, &mut diagram, &request).unwrap();
        request.parent_presentation_id = Some(second.id);
        let second_child = append_group(&project, &mut diagram, &request).unwrap();
        assert_ne!(first_child, second_child);
        assert_eq!(
            range_at(
                diagram
                    .properties
                    .iter()
                    .find(|property| property.id == second_child)
                    .unwrap(),
                0
            ),
            Some(OccurrenceRange { first: 2, count: 1 })
        );
        // Dropping the same property on its child repeats it in that child's enclosing group.
        request.parent_presentation_id = Some(second_child);
        request.name = "channelB".into();
        let sibling = append_group(&project, &mut diagram, &request).unwrap();
        let sibling = diagram
            .properties
            .iter()
            .find(|property| property.id == sibling)
            .unwrap();
        assert_eq!(
            coverage(&project, &diagram, sibling).unwrap().displayed,
            Some(2)
        );
        request.parent_presentation_id = None;
        assert!(
            append_group(&project, &mut diagram, &request)
                .unwrap_err()
                .contains("not owned or inherited")
        );
    }

    #[test]
    fn four_occurrences_keep_definitions_and_nested_ports_connectors_and_movement_distinct() {
        let (project, mut diagram) = fixture();
        let semantic = serde_json::to_value(&project).unwrap();
        set_groups(
            &project,
            &mut diagram,
            "part",
            Some("north = 1\nsouth = 1\neast = 1\nwest = 1"),
        )
        .unwrap();
        super::super::ibd::validate_ibd_diagrams(&project, &[diagram.clone()]).unwrap();
        assert_eq!(diagram.properties.len(), 8);
        // Shared internal delegation is projected four times; aggregate external
        // connection is retained semantically, without substituting the first slot.
        assert_eq!(diagram.connectors.len(), 4);
        assert!(
            diagram
                .connectors
                .iter()
                .all(|edge| edge.context_occurrence_path[0].is_some())
        );
        let first = diagram
            .properties
            .iter()
            .find(|property| property.id == "part")
            .unwrap()
            .clone();
        let report = coverage(&project, &diagram, &first).unwrap();
        assert_eq!(report.displayed, Some(4));
        assert_eq!(report.status, "Complete population");
        let own_child = diagram
            .properties
            .iter()
            .find(|property| super::super::ibd_structure::is_descendant(property, &first))
            .unwrap()
            .clone();
        let other = diagram
            .properties
            .iter()
            .find(|property| property.occurrence_name.as_deref() == Some("south"))
            .unwrap()
            .clone();
        let rect = super::super::routing::RouteRect {
            x: first.x + 40.0,
            y: first.y,
            width: first.width,
            height: first.height,
        };
        super::super::ibd_structure::apply_property_geometry(&mut diagram, &first.id, rect)
            .unwrap();
        assert_eq!(
            diagram
                .properties
                .iter()
                .find(|property| property.id == own_child.id)
                .unwrap()
                .x,
            own_child.x + 40.0
        );
        assert_eq!(
            diagram
                .properties
                .iter()
                .find(|property| property.id == other.id)
                .unwrap()
                .x,
            other.x
        );
        super::super::ibd_structure::expand_property(&project, &mut diagram, &first.id, false)
            .unwrap();
        assert!(!super::super::ibd_structure::property_visible(
            &diagram,
            diagram
                .properties
                .iter()
                .find(|property| property.id == own_child.id)
                .unwrap()
        ));
        assert!(super::super::ibd_structure::property_visible(
            &diagram,
            diagram
                .properties
                .iter()
                .find(|property| property.id == other.id)
                .unwrap()
        ));
        super::super::ibd_structure::clean_groups(&mut diagram).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let mut database = systems_modeler_persistence::ProjectDatabase::open(
            directory.path().join("occurrences.smproj"),
        )
        .unwrap();
        database.save_project(&project).unwrap();
        super::super::ibd::save_ibd_metadata(&mut database, &project, &[diagram.clone()]).unwrap();
        let reopened = database.load_first_project().unwrap();
        let views = super::super::ibd::load_ibd_metadata(&database, &reopened).unwrap();
        assert_eq!(
            serde_json::to_value(&views[0]).unwrap(),
            serde_json::to_value(&diagram).unwrap()
        );
        set_groups(&project, &mut diagram, &first.id, None).unwrap();
        assert_eq!(diagram.properties.len(), 2);
        assert_eq!(
            coverage(&project, &diagram, &diagram.properties[0])
                .unwrap()
                .displayed,
            None
        );
        assert_eq!(serde_json::to_value(&project).unwrap(), semantic);
    }

    #[test]
    fn coverage_counts_distinct_ranges_and_invalid_edits_preserve_history() {
        let (project, diagram) = fixture();
        let workspace = WorkspaceState::default();
        let activity = super::super::activity_workspace::ActivityWorkspaceState::default();
        let history = super::super::history::HistoryState::default();
        *workspace.project.lock().unwrap() = Some(project);
        *workspace.ibd_diagrams.lock().unwrap() = vec![diagram];
        let apply = |text: &str| {
            super::super::history::apply_structural_specification(
                &workspace,
                &activity,
                &history,
                |project, diagrams| {
                    let mut staged = diagrams.to_vec();
                    set_groups(project, &mut staged[0], "part", Some(text))?;
                    super::super::ibd::validate_ibd_diagrams(project, &staged)?;
                    Ok((project.clone(), staged))
                },
            )
        };
        assert!(apply("first pair = 2\nthird = 1").unwrap());
        {
            let guard = workspace.project.lock().unwrap();
            let project = guard.as_ref().unwrap();
            let diagrams = workspace.ibd_diagrams.lock().unwrap();
            let mut view = diagrams[0].clone();
            let property = view
                .properties
                .iter()
                .find(|property| property.id == "part")
                .unwrap()
                .clone();
            assert_eq!(
                coverage(project, &view, &property).unwrap().displayed,
                Some(3)
            );
            let mut repeated_symbol = property.clone();
            repeated_symbol.id = "repeated-symbol".into();
            repeated_symbol.ports.clear();
            view.properties.push(repeated_symbol);
            assert_eq!(
                coverage(project, &view, &property).unwrap().displayed,
                Some(3)
            );
        }
        assert!(super::super::history::undo_states(&workspace, &activity, &history).unwrap());
        let before = serde_json::to_value(&*workspace.ibd_diagrams.lock().unwrap()).unwrap();
        for invalid in ["too many = 5", "same = 1\nsame = 1", "none = 0"] {
            assert!(apply(invalid).is_err());
            assert_eq!(
                serde_json::to_value(&*workspace.ibd_diagrams.lock().unwrap()).unwrap(),
                before
            );
        }
        assert!(super::super::history::redo_states(&workspace, &activity, &history).unwrap());
    }
}
