from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def write(path: str, text: str) -> None:
    (ROOT / path).write_text(text, encoding="utf-8")


def replace_between(text: str, start: str, end: str, replacement: str) -> str:
    left = text.find(start)
    if left < 0:
        raise SystemExit(f"missing start marker: {start}")
    right = text.find(end, left)
    if right < 0:
        raise SystemExit(f"missing end marker: {end}")
    return text[:left] + replacement + text[right:]


path = "apps/desktop/frontend/diagram-interaction.js"
text = read(path)

new_begin = r'''function beginPointerGesture(event, options) {
    if (event.button !== 0) return false;
    const pointerId = event.pointerId;
    const startX = event.clientX;
    const startY = event.clientY;
    const owner = options.owner;
    if (!owner) return false;
    const scale = options.scale || { x: 1, y: 1 };
    let started = false;
    let finished = false;

    const cleanup = () => {
      owner.removeEventListener('pointermove', onMove, true);
      owner.removeEventListener('pointerup', onUp, true);
      owner.removeEventListener('pointercancel', onCancel, true);
      owner.removeEventListener('lostpointercapture', onLostCapture, true);
    };

    const startIfNeeded = (move) => {
      if (started) return true;
      const dx = move.clientX - startX;
      const dy = move.clientY - startY;
      if (Math.hypot(dx, dy) < DRAG_THRESHOLD_PX) return false;
      options.prepare?.();
      if (options.disabled?.()) {
        finished = true;
        cleanup();
        options.onCancel?.();
        return false;
      }
      started = true;
      options.onStart?.();
      return true;
    };

    const onMove = (move) => {
      if (finished || move.pointerId !== pointerId || !startIfNeeded(move)) return;
      move.preventDefault();
      move.stopPropagation();
      options.onMove?.(
        (move.clientX - startX) / Math.max(scale.x || 1, 0.0001),
        (move.clientY - startY) / Math.max(scale.y || 1, 0.0001),
        move,
      );
    };

    const finish = async (up, cancelled) => {
      if (finished || up.pointerId !== pointerId) return;
      finished = true;
      cleanup();
      if (!started) return;
      up.preventDefault?.();
      up.stopPropagation?.();
      if (cancelled) options.onCancel?.();
      else await options.onCommit?.();
    };
    const onUp = (up) => { void finish(up, false); };
    const onCancel = (cancel) => { void finish(cancel, true); };
    const onLostCapture = (lost) => {
      if (!finished && lost.pointerId === pointerId) void finish(lost, true);
    };

    // Pointer capture is owned by the presentation/handle itself. WebView2 then
    // continues delivering the complete drag lifecycle to the same owner even
    // when the pointer leaves its original bounds. Do not depend on window-level
    // pointermove delivery for core diagram editing.
    owner.addEventListener('pointermove', onMove, true);
    owner.addEventListener('pointerup', onUp, true);
    owner.addEventListener('pointercancel', onCancel, true);
    owner.addEventListener('lostpointercapture', onLostCapture, true);
    try { owner.setPointerCapture?.(pointerId); } catch (_) {}
    return true;
  }

  window.smpBeginPresentationGesture = beginPointerGesture;

  '''
text = replace_between(
    text,
    "function beginPointerGesture(event, options) {",
    "function bindHtmlGeometry(node, config) {",
    new_begin,
)

new_bind = r'''const htmlGeometryConfigs = new WeakMap();
  const suppressGeometryClicks = new WeakSet();
  const geometryCanvas = document.getElementById('canvas');

  function bindHtmlGeometry(node, config) {
    if (!node) return;
    node.dataset.smpGeometryBound = '1';
    node.classList.add('smp-interactive-presentation');
    if (getComputedStyle(node).position === 'static') node.style.position = 'absolute';
    // Refresh the adapter every install. Some family renderers preserve DOM nodes
    // while replacing Rust snapshots, so a one-time closure can become stale.
    htmlGeometryConfigs.set(node, config);

    let handle = [...node.children].find((child) => child.classList?.contains('smp-resize-handle'));
    if (!handle) {
      handle = document.createElement('span');
      handle.className = 'smp-resize-handle';
      handle.title = 'Drag to resize';
      handle.setAttribute('aria-label', 'Resize element');
      node.appendChild(handle);
    }
  }

  function htmlGeometryNode(target) {
    if (!(target instanceof Element)) return null;
    return target.closest('.smp-interactive-presentation, .bdd-block, .ibd-property, .ibd-port, .use-case-subject-boundary, .state-vertex');
  }

  function startHtmlGeometryGesture(event) {
    if (event.button !== 0
      || event.ctrlKey
      || event.metaKey
      || geometryCanvas?.classList.contains('space-pan')
      || geometryCanvas?.classList.contains('pan-active')
      || geometryCanvas?.classList.contains('is-panning')) return;

    const node = htmlGeometryNode(event.target);
    if (!node || !geometryCanvas?.contains(node)) return;

    // Rendering is layered across legacy family adapters. If a renderer replaced
    // its DOM between the mutation observer and this pointerdown, synchronously
    // recover the shared adapter before starting the gesture.
    if (!htmlGeometryConfigs.has(node)) install();
    const config = htmlGeometryConfigs.get(node);
    if (!config) return;

    const handle = event.target.closest?.('.smp-resize-handle');
    if (!handle && event.target.closest?.('.constraint-parameter, input, select, textarea, a, [contenteditable="true"]')) return;

    // This one capture listener is the HTML presentation gesture authority for
    // BDD, IBD, Requirement, Use Case, Parametric, Package and State Machine.
    // It prevents older family-local pointerdown handlers from starting a second
    // drag while preserving the later click event for ordinary selection.
    event.stopImmediatePropagation();
    config.select?.();
    const original = config.geometry();
    let next = { ...original };
    const resizing = Boolean(handle);
    const owner = handle || node;

    beginPointerGesture(event, {
      owner,
      scale: surfaceScale(node),
      prepare: cancelTransientAuthoring,
      disabled: config.disabled,
      onStart: () => {
        suppressGeometryClicks.add(node);
        node.classList.toggle('smp-dragging', !resizing);
      },
      onMove: (dx, dy) => {
        if (resizing) {
          next.width = Math.max(config.minWidth, original.width + dx);
          next.height = Math.max(config.minHeight, original.height + dy);
          node.style.width = `${next.width}px`;
          node.style.height = `${next.height}px`;
        } else {
          next.x = Math.max(0, original.x + dx);
          next.y = Math.max(42, original.y + dy);
          node.style.left = `${next.x}px`;
          node.style.top = `${next.y}px`;
        }
        config.preview?.(next);
      },
      onCancel: () => node.classList.remove('smp-dragging'),
      onCommit: async () => {
        node.classList.remove('smp-dragging');
        await config.commit(next);
      },
    });
  }

  geometryCanvas?.addEventListener('pointerdown', startHtmlGeometryGesture, true);
  geometryCanvas?.addEventListener('click', (event) => {
    const node = htmlGeometryNode(event.target);
    if (!node || !suppressGeometryClicks.has(node)) return;
    suppressGeometryClicks.delete(node);
    event.preventDefault();
    event.stopImmediatePropagation();
  }, true);

  '''
text = replace_between(
    text,
    "function bindHtmlGeometry(node, config) {",
    "function installBdd() {",
    new_bind,
)

old_tail = r'''  const baseRender = render;
  render = function renderWithSharedPresentationInteraction() {
    baseRender();
    install();
  };
  queueMicrotask(install);
  const canvas = document.getElementById('canvas');
  if (canvas) {
    let installQueued = false;
    new MutationObserver(() => {
      if (installQueued) return;
      installQueued = true;
      queueMicrotask(() => {
        installQueued = false;
        install();
      });
    }).observe(canvas, { childList: true, subtree: true });
  }
})();
'''
new_tail = r'''  let installQueued = false;
  function scheduleInstall() {
    if (installQueued) return;
    installQueued = true;
    const run = () => {
      installQueued = false;
      install();
    };
    if (typeof requestAnimationFrame === 'function') requestAnimationFrame(run);
    else queueMicrotask(run);
  }

  window.smpInstallPresentationGeometry = install;
  const baseRender = render;
  render = function renderWithSharedPresentationInteraction() {
    baseRender();
    install();
    scheduleInstall();
  };
  queueMicrotask(() => {
    install();
    scheduleInstall();
  });
  if (geometryCanvas) {
    new MutationObserver(scheduleInstall).observe(geometryCanvas, { childList: true, subtree: true });
  }
})();
'''
if old_tail not in text:
    raise SystemExit("diagram interaction tail changed")
text = text.replace(old_tail, new_tail, 1)
write(path, text)

# Presentation-only geometry edits must not be rejected because some other BDD
# in the project has unrelated stale metadata. Geometry validation and incident
# rerouting are already performed before the candidate snapshot is committed.
path = "apps/desktop/src-tauri/src/workspace/presentation_interaction.rs"
text = read(path)
text = text.replace(
    "    WorkspaceState, behavior_workspace, ibd, routed_bdd_edges, use_cases, validate_loaded_diagrams,\n",
    "    WorkspaceState, behavior_workspace, ibd, routed_bdd_edges, use_cases,\n",
    1,
)
text = text.replace(
    "    reroute_connected_bdd_edges(diagram, presentation_id)?;\n    validate_loaded_diagrams(project, diagrams)\n",
    "    reroute_connected_bdd_edges(diagram, presentation_id)?;\n    Ok(())\n",
    1,
)
write(path, text)

# Strengthen the integration contract around the actual common gesture authority,
# not merely the presence of family selectors.
path = "scripts/validate_presentation_interaction.py"
text = read(path)
old = '''assert "window.smpBeginPresentationGesture = beginPointerGesture" in frontend\nassert "window.addEventListener('pointermove', onMove, true)" in frontend\nassert "window.addEventListener('pointerup', onUp, true)" in frontend\nassert "window.addEventListener('pointercancel', onCancel, true)" in frontend\nassert "addEventListener('pointerdown'" in frontend\n'''
new = '''assert "window.smpBeginPresentationGesture = beginPointerGesture" in frontend\nassert "owner.addEventListener('pointermove', onMove, true)" in frontend\nassert "owner.addEventListener('pointerup', onUp, true)" in frontend\nassert "owner.addEventListener('pointercancel', onCancel, true)" in frontend\nassert "owner.addEventListener('lostpointercapture', onLostCapture, true)" in frontend\nassert "const htmlGeometryConfigs = new WeakMap()" in frontend\nassert "geometryCanvas?.addEventListener('pointerdown', startHtmlGeometryGesture, true)" in frontend\nassert "if (!htmlGeometryConfigs.has(node)) install();" in frontend\nassert "event.stopImmediatePropagation();" in frontend\nassert "window.smpInstallPresentationGeometry = install" in frontend\n'''
if old not in text:
    raise SystemExit("presentation interaction contract gesture block changed")
text = text.replace(old, new, 1)
text = text.replace(
    'assert "reroute_connected_bdd_edges" in interaction_rs, "BDD geometry must reroute incident edges without making unrelated routes block editing"\n',
    'assert "reroute_connected_bdd_edges" in interaction_rs, "BDD geometry must reroute incident edges without making unrelated routes block editing"\nassert "validate_loaded_diagrams(project, diagrams)" not in interaction_rs, "Presentation-only BDD geometry must not be blocked by unrelated diagram validation"\n',
    1,
)
write(path, text)

print("PR35 unified geometry kernel repair applied")
