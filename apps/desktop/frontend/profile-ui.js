(function () {
  'use strict';

  const invoke = window.__TAURI__?.core?.invoke;
  const notify = (message) => window.modelerNotify ? window.modelerNotify(message) : console.info(message);
  const dialogs = () => window.smpDialogs;
  const values = (object) => Object.values(object || {});

  function selectedElement(project) {
    const id = window.__MODEL_SELECTION__?.elementId;
    return project.elements.find((element) => element.id === id) || project.elements.find((element) => element.id === project.root_id);
  }

  function render(dialog, project) {
    const repository = project.profiles || {};
    const profiles = values(repository.profiles);
    const stereotypes = values(repository.stereotypes);
    const definitions = values(repository.tag_definitions);
    const applications = values(repository.stereotype_applications);
    const target = selectedElement(project);
    dialog.innerHTML = `<form method="dialog" style="min-width:42rem;max-width:70vw">
      <h2>Profiles and stereotypes</h2>
      <p>Selected semantic target: <strong>${target?.name || 'project root'}</strong> (${target?.kind || 'Model'})</p>
      <div style="display:grid;grid-template-columns:1fr 1fr;gap:1rem;max-height:55vh;overflow:auto">
        <section><h3>Profiles</h3>${profiles.map((profile) => `<div><strong>${profile.name}</strong><br><small>${profile.external_id}</small></div>`).join('') || '<p>No profiles defined.</p>'}</section>
        <section><h3>Stereotypes</h3>${stereotypes.map((stereotype) => `<div><strong>«${stereotype.name}»</strong><br><small>${stereotype.external_id}</small></div>`).join('') || '<p>No stereotypes defined.</p>'}</section>
        <section style="grid-column:1 / -1"><h3>Applications</h3>${applications.map((application) => {
          const stereotype = stereotypes.find((candidate) => candidate.id === application.stereotype_id);
          const tags = definitions.filter((definition) => definition.stereotype_id === application.stereotype_id).map((definition) => `<button type="button" data-set-tag="${application.id}" data-definition="${definition.id}">Edit ${definition.name}</button>`).join(' ');
          return `<div style="display:flex;justify-content:space-between;gap:1rem"><span>«${stereotype?.name || application.stereotype_id}» — ${application.external_id}<br>${tags}</span><button type="button" data-remove="${application.id}">Remove</button></div>`;
        }).join('') || '<p>No stereotype applications.</p>'}</section>
      </div>
      <menu style="display:flex;gap:.5rem;justify-content:flex-end"><button type="button" data-create-profile>Create profile</button><button type="button" data-create-stereotype>Create stereotype</button><button type="button" data-apply>Apply stereotype</button><button>Close</button></menu>
    </form>`;
    dialog.querySelector('[data-create-profile]').addEventListener('click', async () => {
      const result = await dialogs().edit({ title: 'Create profile', fields: [{ id: 'name', label: 'Profile name', required: true }, { id: 'externalId', label: 'Stable external identity', required: true }, { id: 'uri', label: 'Profile URI (optional)' }], confirmLabel: 'Create' });
      if (!result) return;
      await invoke('create_profile_definition', { externalId: result.values.externalId, name: result.values.name, uri: result.values.uri || null });
      await openEditor(dialog);
    });
    dialog.querySelector('[data-create-stereotype]').addEventListener('click', async () => {
      if (!profiles.length) throw new Error('Create a profile first.');
      const choice = profiles.length === 1 ? { selectedId: profiles[0].id } : await dialogs().choose({ title: 'Choose profile', candidates: profiles.map((profile) => ({ id: profile.id, label: profile.name })), confirmLabel: 'Continue' });
      const profile = profiles.find((candidate) => candidate.id === choice?.selectedId);
      if (!profile) return;
      const result = await dialogs().edit({ title: 'Create stereotype', description: `Applicable to ${target.kind}`, fields: [{ id: 'name', label: 'Stereotype name', required: true }, { id: 'externalId', label: 'Stable external identity', required: true }], confirmLabel: 'Create' });
      if (!result) return;
      await invoke('create_stereotype_definition', { profileId: profile.id, externalId: result.values.externalId, name: result.values.name, extends: [{ Element: target.kind }] });
      await invoke('apply_profile_definition', { profileId: profile.id, scopeId: project.root_id, externalId: `${profile.external_id}:application` });
      await openEditor(dialog);
    });
    dialog.querySelector('[data-apply]').addEventListener('click', async () => {
      if (!stereotypes.length) throw new Error('Create a stereotype first.');
      const choice = stereotypes.length === 1 ? { selectedId: stereotypes[0].id } : await dialogs().choose({ title: 'Apply stereotype', candidates: stereotypes.map((stereotype) => ({ id: stereotype.id, label: `«${stereotype.name}»` })), confirmLabel: 'Apply' });
      const stereotype = stereotypes.find((candidate) => candidate.id === choice?.selectedId);
      if (!stereotype) return;
      await invoke('apply_stereotype_definition', { stereotypeId: stereotype.id, target: { Element: target.id }, externalId: `application:${stereotype.external_id}:${target.external_id}` });
      notify(`Applied «${stereotype.name}» to ${target.name}.`);
      await openEditor(dialog);
    });
    dialog.querySelectorAll('[data-remove]').forEach((button) => button.addEventListener('click', async () => {
      await invoke('remove_stereotype_application', { applicationId: button.dataset.remove });
      await openEditor(dialog);
    }));
    dialog.querySelectorAll('[data-set-tag]').forEach((button) => button.addEventListener('click', async () => {
      const definition = definitions.find((candidate) => candidate.id === button.dataset.definition);
      const result = await dialogs().edit({ title: `Edit ${definition.name}`, description: `Type: ${typeof definition.value_type === 'string' ? definition.value_type : 'Enumeration'}`, fields: [{ id: 'value', label: 'Value', required: definition.lower > 0 }], confirmLabel: 'Set value' });
      if (!result) return;
      const raw = result.values.value;
      const kind = typeof definition.value_type === 'string' ? definition.value_type.toLowerCase() : 'enumeration';
      let value = raw;
      if (kind === 'boolean') value = raw.toLowerCase() === 'true';
      if (kind === 'integer') value = Number.parseInt(raw, 10);
      if (kind === 'real') value = Number.parseFloat(raw);
      await invoke('set_stereotype_tag_values', { applicationId: button.dataset.setTag, definitionId: definition.id, values: [{ kind, value }] });
      await openEditor(dialog);
    }));
  }

  async function openEditor(existing) {
    if (!invoke) throw new Error('Profile editing is available in the desktop application.');
    const snapshot = await invoke('workspace_snapshot');
    if (!snapshot.project) throw new Error('Create or open a project first.');
    const dialog = existing || document.body.appendChild(document.createElement('dialog'));
    render(dialog, snapshot.project);
    if (!dialog.open) dialog.showModal();
  }

  function install() {
    const ribbon = document.querySelector('.ribbon-content, .ribbon-groups');
    if (!ribbon || ribbon.querySelector('[data-profile-editing]')) return;
    const group = document.createElement('section');
    group.className = 'ribbon-group';
    group.dataset.profileEditing = 'true';
    group.innerHTML = '<div class="ribbon-actions"><button class="ribbon-command" data-open-profiles><span class="command-icon">«P»</span><span>Profiles</span></button></div><div class="ribbon-label">Semantics</div>';
    group.querySelector('[data-open-profiles]').addEventListener('click', () => openEditor().catch((error) => notify(`Profile editing failed: ${error.message || error}`)));
    ribbon.appendChild(group);
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', install, { once: true });
  else install();
})();
