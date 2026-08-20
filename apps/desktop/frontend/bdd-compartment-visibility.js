(() => {
  const STORAGE_KEY = 'systems-modeler-pro.bdd-compartment-visibility.v1';

  function loadVisibility() {
    try {
      const parsed = JSON.parse(localStorage.getItem(STORAGE_KEY) || '{}');
      return parsed && typeof parsed === 'object' ? parsed : {};
    } catch {
      return {};
    }
  }

  function saveVisibility(value) {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
  }

  function activeBdd() {
    return state.snapshot?.diagrams?.find((diagram) => String(diagram.id) === String(state.selectedDiagramId)) || null;
  }

  function presentationForElement(elementId) {
    const diagram = activeBdd();
    if (!diagram) return null;
    return diagram.nodes?.find((node) => String(node.element_id) === String(elementId)) || null;
  }

  function visibilityKey(diagram, presentation) {
    const projectId = state.snapshot?.project?.id || 'project';
    return `${projectId}:${diagram.id}:${presentation.id}`;
  }

  function hiddenCompartments(diagram, presentation) {
    const stored = loadVisibility()[visibilityKey(diagram, presentation)];
    return new Set(Array.isArray(stored) ? stored : []);
  }

  function setCompartmentHidden(diagram, presentation, label, hidden) {
    const all = loadVisibility();
    const key = visibilityKey(diagram, presentation);
    const values = new Set(Array.isArray(all[key]) ? all[key] : []);
    if (hidden) values.add(label);
    else values.delete(label);
    if (values.size) all[key] = [...values].sort();
    else delete all[key];
    saveVisibility(all);
  }

  function applyCompartmentPresentation() {
    const diagram = activeBdd();
    if (!diagram) return;
    const boxes = [...document.querySelectorAll('#canvas .bdd-block')];
    boxes.forEach((box, index) => {
      const presentation = diagram.nodes?.[index];
      if (!presentation) return;
      box.dataset.presentationId = presentation.id;
      box.style.height = `${presentation.height}px`;
      box.style.minHeight = '0';
      box.style.overflow = 'hidden';
      box.style.boxSizing = 'border-box';

      const hidden = hiddenCompartments(diagram, presentation);
      for (const compartment of box.querySelectorAll('.compartment')) {
        const label = compartment.querySelector('.compartment-title')?.textContent?.trim() || '';
        compartment.hidden = hidden.has(label);
      }

      const header = box.querySelector('.stereotype');
      const name = box.querySelector('.block-name');
      if (header) header.style.flexShrink = '0';
      if (name) name.style.flexShrink = '0';
    });
  }

  function appendCompartmentControls() {
    const diagram = activeBdd();
    const panel = $('properties');
    const elementId = state.selectedElementId;
    if (!diagram || !panel || !elementId) return;
    const presentation = presentationForElement(elementId);
    if (!presentation) return;
    const box = [...document.querySelectorAll('#canvas .bdd-block')]
      .find((candidate) => candidate.dataset.presentationId === presentation.id);
    if (!box) return;
    const compartments = [...box.querySelectorAll('.compartment')]
      .map((compartment) => compartment.querySelector('.compartment-title')?.textContent?.trim())
      .filter(Boolean);
    if (!compartments.length || panel.querySelector('.bdd-compartment-controls')) return;

    const hidden = hiddenCompartments(diagram, presentation);
    const section = document.createElement('section');
    section.className = 'bdd-compartment-controls';
    section.innerHTML = '<div class="property-heading">Compartments</div><div class="muted">Visibility affects this diagram presentation only. Owned model features remain unchanged.</div>';
    for (const label of compartments) {
      const row = document.createElement('label');
      row.className = 'compartment-visibility-toggle';
      const checkbox = document.createElement('input');
      checkbox.type = 'checkbox';
      checkbox.checked = !hidden.has(label);
      checkbox.onchange = () => {
        setCompartmentHidden(diagram, presentation, label, !checkbox.checked);
        applyCompartmentPresentation();
      };
      const text = document.createElement('span');
      text.textContent = `Show ${label}`;
      row.append(checkbox, text);
      section.appendChild(row);
    }
    panel.appendChild(section);
  }

  const baseRender = render;
  render = function renderWithCompartmentVisibility() {
    baseRender();
    applyCompartmentPresentation();
    appendCompartmentControls();
  };

  const observer = new MutationObserver(() => {
    applyCompartmentPresentation();
    appendCompartmentControls();
  });
  const canvas = $('canvas');
  if (canvas) observer.observe(canvas, { childList: true, subtree: true });

  queueMicrotask(() => {
    applyCompartmentPresentation();
    appendCompartmentControls();
  });
})();
