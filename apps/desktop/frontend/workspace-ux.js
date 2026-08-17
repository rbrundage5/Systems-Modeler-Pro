(() => {
  let modelerDragActive = false;

  function isModelerDragSource(target) {
    return target instanceof Element && (
      target.matches('.palette-item[draggable="true"]') ||
      target.matches('.tree-row[draggable="true"]') ||
      target.closest('.palette-item[draggable="true"], .tree-row[draggable="true"]')
    );
  }

  function diagramFrameFromEvent(event) {
    const target = event.target;
    return target instanceof Element ? target.closest('.diagram-frame') : null;
  }

  document.addEventListener('dragstart', (event) => {
    if (!isModelerDragSource(event.target)) return;
    modelerDragActive = true;
    try {
      // WebView2 reliably recognizes text/plain even when custom MIME types are
      // omitted from dataTransfer.types. The authoritative payload remains the
      // custom type written by app.js; this fallback only makes the OS accept
      // the drag as an application drag.
      if (event.dataTransfer && !event.dataTransfer.getData('text/plain')) {
        event.dataTransfer.setData('text/plain', 'systems-modeler-pro');
      }
    } catch (_) {
      // The existing custom payload is sufficient if the fallback is blocked.
    }
  }, true);

  document.addEventListener('dragover', (event) => {
    if (!modelerDragActive) return;
    const frame = diagramFrameFromEvent(event);
    if (!frame) return;
    // Required by HTML5 DnD and, specifically, WebView2 to switch from the
    // prohibited cursor to an allowed copy/drop cursor.
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
    frame.classList.add('palette-target');
  }, true);

  document.addEventListener('dragenter', (event) => {
    if (!modelerDragActive) return;
    const frame = diagramFrameFromEvent(event);
    if (!frame) return;
    event.preventDefault();
    frame.classList.add('palette-target');
  }, true);

  document.addEventListener('dragleave', (event) => {
    const frame = diagramFrameFromEvent(event);
    if (!frame) return;
    const next = event.relatedTarget;
    if (!(next instanceof Node) || !frame.contains(next)) frame.classList.remove('palette-target');
  }, true);

  document.addEventListener('drop', (event) => {
    if (!modelerDragActive) return;
    const frame = diagramFrameFromEvent(event);
    if (!frame) return;
    // app.js owns the semantic drop operation. This capture handler only makes
    // the frame a valid Windows/WebView2 drop target.
    event.preventDefault();
    frame.classList.remove('palette-target');
    modelerDragActive = false;
  }, true);

  document.addEventListener('dragend', () => {
    modelerDragActive = false;
    document.querySelectorAll('.diagram-frame.palette-target')
      .forEach((frame) => frame.classList.remove('palette-target'));
  }, true);
})();
