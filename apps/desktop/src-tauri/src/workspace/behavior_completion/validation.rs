use std::collections::{HashMap, HashSet};

use systems_modeler_core::behavior::{
    BehaviorRepository, Event, PseudostateKind, Region, StateMachine, StateMachineId, Trigger,
    Vertex, VertexId, VertexKind,
};
use systems_modeler_core::{ElementKind, Project};

fn collect_vertices<'a>(regions: &'a [Region], out: &mut HashMap<VertexId, &'a Vertex>) {
    for region in regions {
        for vertex in &region.vertices {
            out.insert(vertex.id, vertex);
            if let VertexKind::State(state) = &vertex.kind {
                collect_vertices(&state.regions, out);
            }
        }
    }
}

fn validate_trigger(project: &Project, trigger: Option<&Trigger>) -> Result<(), String> {
    let Some(trigger) = trigger else {
        return Ok(());
    };
    match &trigger.event {
        Event::Signal { signal_id } => {
            if project
                .element(*signal_id)
                .map_err(|error| error.to_string())?
                .kind
                != ElementKind::Signal
            {
                return Err(format!(
                    "Signal trigger must reference a Signal: {signal_id}"
                ));
            }
        }
        Event::Call { operation_id } => {
            if project
                .element(*operation_id)
                .map_err(|error| error.to_string())?
                .kind
                != ElementKind::Operation
            {
                return Err(format!(
                    "Call trigger must reference an Operation: {operation_id}"
                ));
            }
        }
        Event::Time { expression, .. } => {
            if expression.trim().is_empty() {
                return Err("Time trigger requires a non-empty expression".into());
            }
        }
        Event::Change { expression } => {
            if expression.trim().is_empty() {
                return Err("Change trigger requires a non-empty expression".into());
            }
        }
        Event::AnyReceive => {}
    }
    Ok(())
}

fn validate_region_editing(
    project: &Project,
    region: &Region,
    vertices: &HashMap<VertexId, &Vertex>,
) -> Result<(), String> {
    let initial_count = region
        .vertices
        .iter()
        .filter(|vertex| {
            matches!(
                vertex.kind,
                VertexKind::Pseudostate(PseudostateKind::Initial)
            )
        })
        .count();
    if initial_count > 1 {
        return Err("State Machine Region may contain at most one Initial pseudostate".into());
    }

    for transition in &region.transitions {
        let source = vertices
            .get(&transition.source_id)
            .ok_or("Transition source does not exist in this State Machine")?;
        vertices
            .get(&transition.target_id)
            .ok_or("Transition target does not exist in this State Machine")?;
        validate_trigger(project, transition.trigger.as_ref())?;
        if matches!(
            source.kind,
            VertexKind::Pseudostate(PseudostateKind::Initial)
        ) && (transition.trigger.is_some()
            || transition
                .guard
                .as_ref()
                .is_some_and(|guard| !guard.trim().is_empty()))
        {
            return Err("Initial transition must be triggerless and guardless".into());
        }
    }

    for vertex in &region.vertices {
        let incoming = region
            .transitions
            .iter()
            .filter(|transition| transition.target_id == vertex.id)
            .count();
        let outgoing = region
            .transitions
            .iter()
            .filter(|transition| transition.source_id == vertex.id)
            .count();

        match &vertex.kind {
            VertexKind::Pseudostate(PseudostateKind::Initial) => {
                if incoming != 0 {
                    return Err("Initial pseudostate cannot have an incoming Transition".into());
                }
                if outgoing > 1 {
                    return Err("Initial pseudostate may have only one outgoing Transition".into());
                }
            }
            VertexKind::Pseudostate(PseudostateKind::Fork) if incoming > 1 => {
                return Err("Fork may have at most one incoming Transition while editing".into());
            }
            VertexKind::Pseudostate(PseudostateKind::Join) if outgoing > 1 => {
                return Err("Join may have at most one outgoing Transition while editing".into());
            }
            VertexKind::FinalState if outgoing != 0 => {
                return Err("Final State cannot have outgoing Transitions".into());
            }
            VertexKind::State(state) => {
                for child in &state.regions {
                    validate_region_editing(project, child, vertices)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

pub(super) fn validate_state_machine_editing(
    project: &Project,
    machine: &StateMachine,
) -> Result<(), String> {
    let context = project
        .element(machine.context_id)
        .map_err(|error| error.to_string())?;
    if !context.is_classifier() {
        return Err("State Machine context must be a classifier".into());
    }
    let mut vertices = HashMap::new();
    collect_vertices(&machine.regions, &mut vertices);
    for region in &machine.regions {
        validate_region_editing(project, region, &vertices)?;
    }
    Ok(())
}

fn collect_submachines(regions: &[Region], output: &mut Vec<StateMachineId>) {
    for region in regions {
        for vertex in &region.vertices {
            if let VertexKind::State(state) = &vertex.kind {
                if let Some(submachine) = state.submachine {
                    output.push(submachine);
                }
                collect_submachines(&state.regions, output);
            }
        }
    }
}

fn validate_cycle(
    machine_id: StateMachineId,
    repository: &BehaviorRepository,
    visiting: &mut HashSet<StateMachineId>,
    complete: &mut HashSet<StateMachineId>,
) -> Result<(), String> {
    if complete.contains(&machine_id) {
        return Ok(());
    }
    if !visiting.insert(machine_id) {
        return Err(format!(
            "Submachine State references form a cycle involving State Machine {machine_id}"
        ));
    }
    let machine = repository
        .state_machines
        .get(&machine_id)
        .ok_or_else(|| format!("Unknown State Machine: {machine_id}"))?;
    let mut references = Vec::new();
    collect_submachines(&machine.regions, &mut references);
    for referenced in references {
        if referenced == machine_id {
            return Err("A State Machine cannot reference itself as a Submachine State".into());
        }
        if !repository.state_machines.contains_key(&referenced) {
            return Err(format!(
                "Submachine State references an unknown State Machine: {referenced}"
            ));
        }
        validate_cycle(referenced, repository, visiting, complete)?;
    }
    visiting.remove(&machine_id);
    complete.insert(machine_id);
    Ok(())
}

pub(super) fn validate_repository_state_machines_editing(
    project: &Project,
    repository: &BehaviorRepository,
) -> Result<(), String> {
    for machine in repository.state_machines.values() {
        validate_state_machine_editing(project, machine)?;
    }
    let mut complete = HashSet::new();
    for machine_id in repository.state_machines.keys().copied() {
        validate_cycle(machine_id, repository, &mut HashSet::new(), &mut complete)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use systems_modeler_core::behavior::{
        BehaviorRepository, PseudostateKind, State, Transition, TransitionId, TransitionKind,
        Vertex, VertexId, VertexKind,
    };
    use systems_modeler_core::{ElementKind, Project};

    fn fixture() -> (Project, BehaviorRepository, StateMachineId) {
        let mut project = Project::new("Editing validation");
        let package = project
            .create_element(ElementKind::Package, "Behavior", project.root_id)
            .unwrap();
        let block = project
            .create_element(ElementKind::Block, "System", package)
            .unwrap();
        let mut repository = BehaviorRepository::default();
        let machine_id = repository
            .create_state_machine(&project, block, "System States")
            .unwrap();
        (project, repository, machine_id)
    }

    #[test]
    fn incomplete_fork_does_not_block_initial_to_state_edit() {
        let (project, mut repository, machine_id) = fixture();
        let machine = repository.state_machines.get_mut(&machine_id).unwrap();
        let region = machine.regions.first_mut().unwrap();
        let initial = VertexId::new();
        let state = VertexId::new();
        let fork = VertexId::new();
        region.vertices.extend([
            Vertex {
                id: initial,
                name: String::new(),
                kind: VertexKind::Pseudostate(PseudostateKind::Initial),
            },
            Vertex {
                id: state,
                name: "Ready".into(),
                kind: VertexKind::State(State::default()),
            },
            Vertex {
                id: fork,
                name: String::new(),
                kind: VertexKind::Pseudostate(PseudostateKind::Fork),
            },
        ]);
        region.transitions.push(Transition {
            id: TransitionId::new(),
            source_id: initial,
            target_id: state,
            kind: TransitionKind::External,
            trigger: None,
            guard: None,
            effect: None,
        });

        validate_state_machine_editing(&project, machine).unwrap();
        assert!(systems_modeler_core::behavior::validate_state_machine(&project, machine).is_err());
    }

    #[test]
    fn editing_rejects_impossible_initial_final_fork_join_topology() {
        let (project, mut repository, machine_id) = fixture();
        let machine = repository.state_machines.get_mut(&machine_id).unwrap();
        let region = machine.regions.first_mut().unwrap();
        let initial = VertexId::new();
        let a = VertexId::new();
        let b = VertexId::new();
        region.vertices.extend([
            Vertex {
                id: initial,
                name: String::new(),
                kind: VertexKind::Pseudostate(PseudostateKind::Initial),
            },
            Vertex {
                id: a,
                name: "A".into(),
                kind: VertexKind::State(State::default()),
            },
            Vertex {
                id: b,
                name: "B".into(),
                kind: VertexKind::State(State::default()),
            },
        ]);
        region.transitions.extend([
            Transition {
                id: TransitionId::new(),
                source_id: initial,
                target_id: a,
                kind: TransitionKind::External,
                trigger: None,
                guard: None,
                effect: None,
            },
            Transition {
                id: TransitionId::new(),
                source_id: initial,
                target_id: b,
                kind: TransitionKind::External,
                trigger: None,
                guard: None,
                effect: None,
            },
        ]);
        assert!(
            validate_state_machine_editing(&project, machine)
                .unwrap_err()
                .contains("only one outgoing")
        );
    }
}
