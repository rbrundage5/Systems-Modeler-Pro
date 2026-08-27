(() => {
  function activeStateMachineDiagram() {
    return state.behaviorSnapshot?.diagrams?.find(
      (diagram) => String(diagram.id) === String(state.selectedBehaviorDiagramId)
        && diagram.kind === 'StateMachine',
    ) || null;
  }

  function stateMachines() {
    return state.behaviorSnapshot?.repository?.state_machines || {};
  }

  function activities() {
    return state.activitySnapshot?.repository?.activities || {};
  }

  function flattenVertices(regions, output = []) {
    for (const region of regions || []) {
      for (const vertex of region.vertices || []) {
        output.push(vertex);
        if (vertex.kind?.State?.regions) flattenVertices(vertex.kind.State.regions, output);
      }
    }
    return output;
  }

  function chooseSubmachine(currentSemanticId) {
    const candidates = Object.entries(stateMachines())
      .filter(([id]) => String(id) !== String(currentSemanticId))
      .map(([id, machine]) => ({ id, name: machine.name || id }));
    if (!candidates.length) {
      alert('Create another State Machine first. A Submachine State must reference an existing different State Machine.');
      return null;
    }
    const menu = candidates.map((item, index) => `${index + 1}. ${item.name}`).join('\n');
    const answer = prompt(`Choose referenced State Machine:\n${menu}`, '1');
    if (answer === null) return null;
    return candidates[Number(answer) - 1] || null;
  }

  async function createSubmachineState(frame, diagram, event) {
    const candidate = chooseSubmachine(diagram.semantic_id);
    if (!candidate) return;
    const name = prompt('Submachine State name', candidate.name);
    if (!name) return;
    const rect = frame.getBoundingClientRect();
    const x = Math.max(12, event.clientX - rect.left - 95);
    const y = Math.max(42, event.clientY - rect.top - 45);
    await runCommand('Creating Submachine State…', () => requireInvoke()('add_submachine_state', {
      diagramId: diagram.id,
      regionIdValue: state.behaviorTargetRegionId || null,
      name,
      submachineIdValue: candidate.id,
      x,
      y,
    }));
    state.behaviorTool = null;
    await refresh();
  }

  function decorateSubmachineStates(diagram) {
    const machine = stateMachines()[String(diagram.semantic_id)];
    if (!machine) return;
    const vertices = flattenVertices(machine.regions);
    const nodes = [...document.querySelectorAll('.state-machine-frame .state-vertex')];
    nodes.forEach((node, index) => {
      const vertex = vertices[index];
      const submachineId = vertex?.kind?.State?.submachine;
      if (!submachineId) return;
      const referenced = stateMachines()[String(submachineId)];
      node.classList.add('submachine-state');
      if (node.querySelector('.submachine-reference')) return;
      const label = document.createElement('span');
      label.className = 'submachine-reference';
      label.textContent = `«submachine» ${referenced?.name || submachineId}`;
      node.appendChild(label);
    });
  }

  function activityOptions(selected) {
    const choices = Object.entries(activities())
      .sort((left, right) => String(left[1]?.name || left[0]).localeCompare(String(right[1]?.name || right[0])));
    const options = ['<option value="">(none)</option>'];
    for (const [id, activity] of choices) {
      const isSelected = String(id) === String(selected || '');
      options.push(`<option value="${escapeAttr(id)}"${isSelected ? ' selected' : ''}>${escapeHtml(activity?.name || id)}</option>`);
    }
    return options.join('');
  }

  function appendExecutableStateBehaviorProperties(panel, diagram, vertex) {
    const stateSemantic = vertex?.kind?.State;
    if (!stateSemantic || !panel) return;
    const section = document.createElement('div');
    section.className = 'state-executable-behavior-editor';
    section.innerHTML = `<div class="property-heading">Executable State Behaviors</div>
      <label>Entry Activity<select id="state-entry-activity">${activityOptions(stateSemantic.entry)}</select></label>
      <label>Do Activity<select id="state-do-activity">${activityOptions(stateSemantic.do_activity)}</select></label>
      <label>Exit Activity<select id="state-exit-activity">${activityOptions(stateSemantic.exit)}</select></label>
      <button id="state-behavior-apply" class="primary">Apply State Behaviors</button>
      <div class="muted">These fields store stable Activity IDs. Runtime execution never interprets arbitrary state text as code.</div>`;
    panel.appendChild(section);
    const apply = document.getElementById('state-behavior-apply');
    if (!apply) return;
    apply.onclick = async () => {
      await runCommand('Updating State behaviors…', () => requireInvoke()('update_state_behaviors', {
        diagramId: diagram.id,
        stateVertexId: vertex.id,
        entry: document.getElementById('state-entry-activity')?.value || null,
        doActivity: document.getElementById('state-do-activity')?.value || null,
        exit: document.getElementById('state-exit-activity')?.value || null,
      }));
      await refresh();
    };
  }

  const baseRenderPalette = renderPalette;
  renderPalette = function renderPaletteWithSubmachineState() {
    const result = baseRenderPalette();
    const button = document.querySelector('[data-behavior-tool="SubmachineState"]');
    if (button) {
      button.onclick = () => {
        state.selectedBehaviorItem = null;
        state.behaviorPending = null;
        state.behaviorTool = 'SubmachineState';
        render();
      };
    }
    return result;
  };

  const baseRenderProperties = renderProperties;
  renderProperties = function renderPropertiesWithExecutableStateBehaviors() {
    const result = baseRenderProperties();
    const diagram = activeStateMachineDiagram();
    const item = state.selectedBehaviorItem;
    if (!diagram || item?.type !== 'Vertex' || item.semantic?.kind?.State == null) return result;
    appendExecutableStateBehaviorProperties(document.getElementById('properties'), diagram, item.semantic);
    return result;
  };

  // Wrap the final render lifecycle, not renderCanvas. The authoritative behavior
  // renderer owns canvas construction and this adapter only decorates/forwards input.
  const baseRender = render;
  render = function renderWithSubmachineState() {
    const result = baseRender();
    const diagram = activeStateMachineDiagram();
    if (!diagram) return result;
    decorateSubmachineStates(diagram);
    const frame = document.querySelector('.state-machine-frame');
    if (!frame) return result;
    frame.addEventListener('click', async (event) => {
      if (state.behaviorTool !== 'SubmachineState' || event.target !== frame) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      await createSubmachineState(frame, diagram, event);
    }, true);
    return result;
  };
})();
