// Drive production gestures; Rust projection is an IPC reply, never recreated here.
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { fixture, flush } = require('./test_cancelled_presentation_gesture.cjs');

const frame = { x: 54, y: 70, width: 1018, height: 662, manuallySized: true };
const deferred = () => { let resolve, reject; const promise = new Promise((yes, no) => { resolve = yes; reject = no; }); return { promise, resolve, reject }; };

test('port preview uses Rust projection, visible start coordinates and zoom', async () => {
  const ui = fixture('ibd-port', 2, { frame, invoke: () => ({ x: 54, y: 188, size: 16 }) });
  // A legacy renderer can display a point different from stored coordinates.
  ui.node.style.left = '46px';
  const gesture = ui.start(); gesture.move(50, 60); await flush();
  const { command, args } = ui.commands[0];
  assert.equal(command, 'preview_ibd_port_geometry');
  assert.deepEqual({ x: args.x, y: args.y, size: args.size }, { x: 74, y: 148, size: 16 });
  assert.deepEqual(args.framePreference, frame);
  assert.equal(ui.node.style.left, '46px'); assert.equal(ui.node.style.top, '180px');
  assert.equal(ui.commands.filter(item => item.command === 'update_ibd_port_geometry').length, 0);
});

test('rapid port movement coalesces requests and discards stale projection replies', async () => {
  const pending = [];
  const ui = fixture('ibd-port', 1, { frame, invoke: () => { const reply = deferred(); pending.push(reply); return reply.promise; } });
  const gesture = ui.start();
  gesture.move(30, 40); gesture.move(50, 60); gesture.move(70, 80);
  assert.equal(ui.commands.length, 1, 'only one preview may be in flight');
  pending[0].resolve({ x: 54, y: 148, size: 16 }); await flush();
  assert.equal(ui.node.style.left, '100px', 'an obsolete reply must not flash onscreen');
  assert.equal(ui.commands.length, 2);
  assert.equal(ui.commands[1].args.x, 168); assert.equal(ui.commands[1].args.y, 188);
  pending[1].resolve({ x: 54, y: 188, size: 16 }); await flush();
  assert.equal(ui.node.style.top, '180px');
  gesture.end('pointercancel');
});

for (const cancellation of ['pointercancel', 'lostpointercapture']) {
  test(`port ${cancellation} restores geometry and ignores an outstanding IPC reply`, async () => {
    const reply = deferred();
    const ui = fixture('ibd-port', 1, { frame, invoke: () => reply.promise });
    const gesture = ui.start(); gesture.move(50, 60); gesture.end(cancellation);
    reply.resolve({ x: 900, y: 500, size: 32 }); await flush();
    assert.equal(ui.node.style.left, '100px'); assert.equal(ui.node.style.top, '120px');
    assert.equal(ui.node.style.width, '16px');
    assert.equal(ui.commands.length, 1); assert.equal(ui.refreshes, 0);
  });
}

test('port release commits the latest requested point once, without waiting for a stale preview', async () => {
  const reply = deferred();
  const ui = fixture('ibd-port', 1, { frame, invoke: (command) => command.startsWith('preview_') ? reply.promise : undefined });
  const gesture = ui.start(); gesture.move(50, 60); gesture.move(90, 100); gesture.end('pointerup'); await flush();
  const commits = ui.commands.filter(item => item.command === 'update_ibd_port_geometry');
  assert.equal(commits.length, 1);
  assert.deepEqual({ x: commits[0].args.x, y: commits[0].args.y }, { x: 188, y: 208 });
  reply.resolve({ x: 54, y: 168, size: 16 }); await flush();
  assert.equal(ui.commands.length, 2); assert.equal(ui.refreshes, 1);
  assert.equal(ui.node.style.left, '100px', 'a late preview cannot overwrite the post-commit render');
});

test('port resizing requests square geometry and invalid preview leaves the last valid view', async () => {
  const ui = fixture('ibd-port', 1, { frame, invoke: async () => { throw new Error('port exceeds owner'); } });
  const gesture = ui.start(true); gesture.move(50, 30); await flush();
  assert.equal(ui.commands[0].args.size, 56);
  assert.equal(ui.node.style.width, '16px'); assert.equal(ui.node.style.height, '16px');
  gesture.end('pointercancel');
});

test('native connector and label previews follow the port and cancel restores the rendered routes', async () => {
  const edge = { id: 'edge', relationship_id: 'rel', source_presentation_id: 'presentation', target_presentation_id: 'target',
    points: [{ x: 54, y: 128 }, { x: 200, y: 128 }], label_anchor: { x: 127, y: 128 } };
  const projected = { ...edge, points: [{ x: 1072, y: 400 }, { x: 200, y: 400 }], label_anchor: { x: 636, y: 400 } };
  const ui = fixture('ibd-port', 1, { frame, connectors: [edge], invoke: () => ({ x: 1072, y: 400, size: 16, connectors: [projected] }) });
  const line = ui.document.querySelector('#canvas .ibd-connector[data-presentation-id="edge"]');
  const label = ui.document.querySelector('#canvas .relationship-label[data-presentation-id="edge"]');
  const before = JSON.stringify(ui.diagram);
  const gesture = ui.start(); gesture.move(500, 400); await flush();
  assert.equal(line.getAttribute('points'), '1072,400 200,400');
  assert.equal(label.getAttribute('x'), '641'); assert.equal(label.getAttribute('y'), '394');
  assert.equal(JSON.stringify(ui.diagram), before, 'preview cannot change authored snapshots');
  gesture.end('pointercancel');
  assert.equal(line.getAttribute('points'), '54,128 200,128');
  assert.equal(label.getAttribute('x'), '132'); assert.equal(label.getAttribute('y'), '122');
  assert.equal(ui.commands.length, 1); assert.equal(ui.refreshes, 0);
});

test('an outstanding native route preview cannot repaint after switching diagrams', async () => {
  const reply = deferred();
  const edge = { id: 'edge', relationship_id: 'rel', source_presentation_id: 'presentation', target_presentation_id: 'target',
    points: [{ x: 54, y: 128 }, { x: 200, y: 128 }], label_anchor: null };
  const ui = fixture('ibd-port', 1, { frame, connectors: [edge], invoke: () => reply.promise });
  const line = ui.document.querySelector('#canvas .ibd-connector[data-presentation-id="edge"]');
  const gesture = ui.start(); gesture.move(500, 400); ui.state.selectedDiagramId = 'another';
  reply.resolve({ x: 1072, y: 400, size: 16, connectors: [{ ...edge, points: [{ x: 1, y: 2 }] }] }); await flush();
  assert.equal(line.getAttribute('points'), '54,128 200,128');
  gesture.end('pointercancel');
});
