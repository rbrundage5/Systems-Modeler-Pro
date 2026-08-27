(() => {
  'use strict';

  const html = (value) => String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#039;');

  const idText = (value) => String(value ?? '');

  function runtimeValueText(value) {
    if (value == null || value === 'Unset') return 'unset';
    if (typeof value !== 'object') return String(value);
    const [kind, content] = Object.entries(value)[0] || ['Unset', 'unset'];
    if (kind === 'ElementReference') return `element ${content}`;
    return String(content);
  }

  function renderBehavior(snapshot, instanceId) {
    if (idText(snapshot.runtime_instance_id) !== instanceId) return '';
    if (Array.isArray(snapshot.call_frames)) {
      const frames = snapshot.call_frames.map((frame) => html(frame.activity_name)).join(' → ');
      return frames
        ? `<div class="structural-runtime-behavior"><b>Activity</b><span>${frames}</span></div>`
        : '';
    }
    if (Array.isArray(snapshot.active_states)) {
      const states = snapshot.active_states.map((state) => html(state.state_name)).join(', ');
      return `<div class="structural-runtime-behavior"><b>State</b><span>${states || 'No active state'}</span></div>`;
    }
    return '';
  }

  function renderInstance(snapshot, runtime, instance, children, valueDefinitions, depth) {
    const execution = snapshot.execution;
    const instanceId = idText(instance.id);
    const values = (execution.runtime_values || [])
      .filter((entry) => idText(entry.key?.instance_id) === instanceId)
      .map((entry) => {
        const definition = valueDefinitions.get(idText(entry.key?.semantic_element_id));
        const unit = definition?.unit_symbol ? ` ${html(definition.unit_symbol)}` : '';
        return `<li><b>${html(definition?.name || entry.key?.semantic_element_id)}</b> = ${html(runtimeValueText(entry.value))}${unit}</li>`;
      });
    const ports = (runtime.ports || [])
      .filter((port) => idText(port.owner_instance_id) === instanceId)
      .map((port) => {
        const contracts = (port.flow_contracts || [])
          .map((flow) => `${html(String(flow.direction).toLowerCase())} ${html(flow.name)} : ${html(flow.type_name)}`)
          .join('; ');
        return `<li title="Port ${html(port.semantic_port_id)}"><b>${html(port.qualified_path.split('.').at(-1))}</b> : ${html(port.type_name)} <span class="structural-runtime-tag">${html(port.kind)}</span>${port.is_conjugated ? ' <span class="structural-runtime-tag">conjugated</span>' : ''}${contracts ? `<small>${contracts}</small>` : ''}</li>`;
      });
    const links = (runtime.connector_links || [])
      .filter((link) => idText(link.source?.instance_id) === instanceId || idText(link.target?.instance_id) === instanceId)
      .map((link) => {
        const outgoing = idText(link.source?.instance_id) === instanceId;
        const peer = outgoing ? link.target : link.source;
        return `<li title="Connector ${html(link.semantic_connector_id)}">${outgoing ? '→' : '←'} ${html(peer?.qualified_path)} <span class="structural-runtime-tag">${html(link.kind)}</span></li>`;
      });
    const events = (execution.scheduled_events || [])
      .filter((scheduled) => idText(scheduled.event?.target_runtime_instance_id) === instanceId)
      .map((scheduled) => `<li><b>${html(scheduled.event?.name)}</b> at ${html(scheduled.due_time)} ns${scheduled.event?.target_port_id ? ` → Port ${html(scheduled.event.target_port_id)}` : ''}</li>`);
    const nested = (children.get(instanceId) || [])
      .map((child) => renderInstance(snapshot, runtime, child, children, valueDefinitions, depth + 1))
      .join('');
    const usage = instance.semantic_usage_id
      ? `<span>usage ${html(instance.name)} · ${html(instance.semantic_usage_id)}</span>`
      : '<span>configured/root occurrence</span>';
    return `<details class="structural-runtime-instance" data-depth="${depth}" ${depth < 2 ? 'open' : ''}>
      <summary><span class="structural-runtime-path">${html(instance.qualified_path)}</span><span>: ${html(instance.classifier_name || instance.classifier_id)}</span></summary>
      <div class="structural-runtime-identity" title="Runtime instance ${html(instanceId)}"><span>runtime ${html(instanceId)}</span>${usage}<span>classifier ${html(instance.classifier_id)}</span></div>
      ${renderBehavior(snapshot, instanceId)}
      ${values.length ? `<div class="structural-runtime-section"><b>Values</b><ul>${values.join('')}</ul></div>` : ''}
      ${ports.length ? `<div class="structural-runtime-section"><b>Ports</b><ul>${ports.join('')}</ul></div>` : ''}
      ${links.length ? `<div class="structural-runtime-section"><b>Connector endpoints</b><ul>${links.join('')}</ul></div>` : ''}
      ${events.length ? `<div class="structural-runtime-section"><b>Pending addressed events</b><ul>${events.join('')}</ul></div>` : ''}
      ${nested ? `<div class="structural-runtime-children">${nested}</div>` : ''}
    </details>`;
  }

  window.renderStructuralRuntimeInspector = function renderStructuralRuntimeInspector(snapshot) {
    const runtime = snapshot?.execution?.structural_runtime;
    if (!runtime) return '';
    const instances = runtime.instances || [];
    const byId = new Map(instances.map((instance) => [idText(instance.id), instance]));
    const children = new Map();
    for (const instance of instances) {
      const ownerId = idText(instance.owner_runtime_instance_id);
      if (!ownerId) continue;
      const owned = children.get(ownerId) || [];
      owned.push(instance);
      children.set(ownerId, owned);
    }
    for (const owned of children.values()) {
      owned.sort((left, right) => String(left.qualified_path).localeCompare(String(right.qualified_path)));
    }
    const valueDefinitions = new Map((runtime.value_definitions || [])
      .map((definition) => [idText(definition.semantic_property_id), definition]));
    const roots = (runtime.root_instance_ids || [])
      .map((id) => byId.get(idText(id)))
      .filter(Boolean);
    return `<section class="structural-runtime-inspector" aria-label="System runtime inspection">
      <div class="structural-runtime-heading"><strong>System Runtime</strong><span>${instances.length} instance(s) · ${(runtime.connector_links || []).length} link(s)</span></div>
      <div class="structural-runtime-tree">${roots.map((instance) => renderInstance(snapshot, runtime, instance, children, valueDefinitions, 0)).join('')}</div>
    </section>`;
  };
})();
