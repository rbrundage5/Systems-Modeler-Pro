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
