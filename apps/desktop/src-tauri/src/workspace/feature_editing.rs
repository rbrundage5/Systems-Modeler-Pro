use super::*;

fn parse_aggregation(value: &str) -> Result<AggregationKind, String> {
    match value {
        "none" => Ok(AggregationKind::None),
        "shared" => Ok(AggregationKind::Shared),
        "composite" => Ok(AggregationKind::Composite),
        _ => Err(format!("invalid aggregation: {value}")),
    }
}

pub(super) fn parse_parameter_direction(
    value: &str,
) -> Result<systems_modeler_core::ParameterDirection, String> {
    match value {
        "in" => Ok(systems_modeler_core::ParameterDirection::In),
        "out" => Ok(systems_modeler_core::ParameterDirection::Out),
        "inout" => Ok(systems_modeler_core::ParameterDirection::InOut),
        "return" => Ok(systems_modeler_core::ParameterDirection::Return),
        _ => Err(format!("invalid parameter direction: {value}")),
    }
}

pub(super) fn parse_flow_direction(
    value: &str,
) -> Result<systems_modeler_core::FlowDirection, String> {
    match value {
        "in" => Ok(systems_modeler_core::FlowDirection::In),
        "out" => Ok(systems_modeler_core::FlowDirection::Out),
        "inout" => Ok(systems_modeler_core::FlowDirection::InOut),
        _ => Err(format!("invalid flow direction: {value}")),
    }
}

#[tauri::command]
#[allow(clippy::too_many_arguments)] // Stable Tauri IPC contract; frontend sends named fields.
pub fn update_bdd_feature_semantics(
    element_id: String,
    lower: Option<u32>,
    upper: Option<u32>,
    aggregation: Option<String>,
    is_derived: Option<bool>,
    is_read_only: Option<bool>,
    is_conjugated: Option<bool>,
    parameter_direction: Option<String>,
    flow_direction: Option<String>,
    state: tauri::State<'_, WorkspaceState>,
) -> Result<(), String> {
    let element_id = parse_element_id(&element_id)?;
    let mut project_guard = state.project.lock().map_err(|_| "project lock poisoned")?;
    let project = project_guard.as_mut().ok_or("no project open")?;
    let kind = project
        .element(element_id)
        .map_err(|error| error.to_string())?
        .kind
        .clone();

    if lower.is_some() || upper.is_some() {
        if !matches!(
            kind,
            ElementKind::PartProperty
                | ElementKind::ReferenceProperty
                | ElementKind::ValueProperty
                | ElementKind::FlowProperty
                | ElementKind::ConstraintProperty
                | ElementKind::ConstraintParameter
                | ElementKind::ProxyPort
                | ElementKind::FullPort
                | ElementKind::Parameter
        ) {
            return Err(format!("{kind:?} does not support multiplicity"));
        }
        let current = project
            .element(element_id)
            .map_err(|error| error.to_string())?
            .multiplicity
            .unwrap_or(Multiplicity::ONE);
        let multiplicity = Multiplicity::new(lower.unwrap_or(current.lower), upper)
            .map_err(|error| error.to_string())?;
        project
            .set_multiplicity(element_id, multiplicity)
            .map_err(|error| error.to_string())?;
    }

    if let Some(aggregation) = aggregation {
        if !matches!(
            kind,
            ElementKind::PartProperty | ElementKind::ReferenceProperty
        ) {
            return Err(format!("{kind:?} does not support aggregation"));
        }
        project
            .set_aggregation(element_id, parse_aggregation(&aggregation)?)
            .map_err(|error| error.to_string())?;
    }

    {
        let element = project
            .element_mut(element_id)
            .map_err(|error| error.to_string())?;
        if let Some(value) = is_derived {
            element.is_derived = value;
        }
        if let Some(value) = is_read_only {
            element.is_read_only = value;
        }
        if let Some(value) = is_conjugated {
            if !matches!(kind, ElementKind::ProxyPort | ElementKind::FullPort) {
                return Err(format!("{kind:?} does not support port conjugation"));
            }
            element.is_conjugated = value;
        }
        if let Some(value) = parameter_direction {
            if kind != ElementKind::Parameter {
                return Err(format!("{kind:?} does not support parameter direction"));
            }
            element.parameter_direction = Some(parse_parameter_direction(&value)?);
        }
        if let Some(value) = flow_direction {
            if kind != ElementKind::FlowProperty {
                return Err(format!("{kind:?} does not support flow direction"));
            }
            element.flow_direction = Some(parse_flow_direction(&value)?);
        }
    }

    project
        .validate_element(element_id)
        .map_err(|error| error.to_string())
}
