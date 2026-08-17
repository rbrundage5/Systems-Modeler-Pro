(() => {
  const FRAME = '.diagram-frame';
  const ACTIVE = 'palette-target';

  function frameFor(event) {
    const target = event.target instanceof Element ? event.target : null;
    return target?.closest(FRAME) || null;
  }

  // WebView2 can omit custom MIME types from dataTransfer.types during dragover.
  // Accept modeler drags based on the actual diagram hit target, then let app.js
  // consume the semantic payload on drop and forward the operation to Rust.
  document.addEventListener('dragenter', (event) => {
    const frame = frameFor(event);
    if (!frame) return;
    event.preventDefault();
    frame.classList.add(ACTIVE);
  }, true);

  document.addEventListener('dragover', (event) => {
    const frame = frameFor(event);
    if (!frame) return;
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = 'copy';
    frame.classList.add(ACTIVE);
  }, true);

  document.addEventListener('dragleave', (event) => {
    const frame = frameFor(event);
    if (!frame) return;
    const next = event.relatedTarget instanceof Node ? event.relatedTarget : null;
    if (!next || !frame.contains(next)) frame.classList.remove(ACTIVE);
  }, true);

  document.addEventListener('drop', (event) => {
    const frame = frameFor(event);
    if (!frame) return;
    // Do not stop propagation: the frame's existing drop handler owns the
    // semantic operation and invokes the Rust commands.
    event.preventDefault();
    frame.classList.remove(ACTIVE);
  }, true);

  document.addEventListener('dragend', () => {
    document.querySelectorAll(`${FRAME}.${ACTIVE}`).forEach((node) => node.classList.remove(ACTIVE));
  }, true);
})();
