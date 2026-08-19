(() => {
  const REGION_VERTEX_TOOLS = new Set([
    'State', 'CompositeState', 'OrthogonalState', 'Initial', 'FinalState',
    'Choice', 'Junction', 'Fork', 'Join', 'ShallowHistory', 'DeepHistory',
    'EntryPoint', 'ExitPoint', 'Terminate',
  ]);

  function activeStateMachineDiagram() {
    return state.behaviorSnapshot?.diagrams?.find(
      (diagram) => diagram.id === state.selectedBehaviorDiagramId && diagram.kind === 'StateMachine',
    ) || null;
  }

  function machine(diagram) {
    return diagram
      ? state.behaviorSnapshot?.repository?.state_machines?.[diagram.semantic_id] || null
      : null;
  }

  function nestedRegions(regions, output = []) {
    for (const region of regions || []) {
      for (const vertex of region.vertices || []) {
        const children = vertex.kind?.State?.regions || [];
        for (const child of children) output.push(child);
        nestedRegions(children, output);
      }
    }
    return output;
  }

  async function createInRegion(diagram, region, frame, event) {
    const tool = state.behaviorTool;
    if (!REGION_VERTEX_TOOLS.has(tool)) return;
    const rect = frame.getBoundingClientRect();
    const x = Math.max(12, event.clientX - rect.left - 70);
    const y = Math.max(42, event.clientY - rect.top - 35);
    const needsName = ['State', 'CompositeState', 'OrthogonalState'].includes(tool);
    const name = needsName
      ? prompt(`${tool === 'State' ? 'State' : tool.replace('State', ' State')} name`, 'State')
      : '';
    if (needsName && !name) return;

    state.behaviorTargetRegionId = region.id;
    if (tool === 'CompositeState' || tool === 'OrthogonalState') {
      await runCommand(`Creating ${tool} in ${region.name || 'Region'}…`, () => requireInvoke()(
        'add_composite_state',
        {
          diagramId: diagram.id,
          regionIdValue: region.id,
          name,
          orthogonal: tool === 'OrthogonalState',
          x,
          y,
        },
      ));
    } else {
      await runCommand(`Creating ${tool} in ${region.name || 'Region'}…`, () => requireInvoke()(
        'add_state_vertex',
        {
          diagramId: diagram.id,
          regionIdValue: region.id,
          kind: tool,
          name: name || '',
          x,
          y,
        },
      ));
    }
    state.behaviorTool = null;
    await refresh();
  }

  function bindRegionCells() {
    const diagram = activeStateMachineDiagram();
    const semantic = machine(diagram);
    const frame = document.querySelector('.state-machine-frame');
    if (!diagram || !semantic || !frame) return;

    const regions = nestedRegions(semantic.regions);
    const cells = [...frame.querySelectorAll('.state-region-cell')];
    cells.forEach((cell, index) => {
      const region = regions[index];
      if (!region) return;
      cell.dataset.regionId = region.id;
      cell.title = `Region: ${region.name || 'Region'}`;
      cell.addEventListener('click', async (event) => {
        if (!REGION_VERTEX_TOOLS.has(state.behaviorTool)) {
          state.behaviorTargetRegionId = region.id;
          return;
        }
        event.preventDefault();
        event.stopImmediatePropagation();
        await createInRegion(diagram, region, frame, event);
      }, true);
    });
  }

  const previousRender = render;
  render = function renderWithRegionPlacement() {
    previousRender();
    bindRegionCells();
  };

  bindRegionCells();
})();
