const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const source = fs.readFileSync('apps/desktop/frontend/updater.js', 'utf8');
const tick = () => new Promise(resolve => setImmediate(resolve));

function harness(handler) {
  const nodes = Object.fromEntries(['status','install','open','retry'].map(id => [id, {
    hidden: id === 'install' || id === 'retry', disabled: false, textContent: '',
    listeners: {}, addEventListener(type, callback) { this.listeners[type] = callback; },
  }]));
  const calls = [];
  vm.runInNewContext(source, {
    document: { getElementById: id => nodes[id] },
    window: { __TAURI__: { core: { invoke(command) {
      calls.push(command);
      return handler(command);
    } } } },
  });
  return { nodes, calls, click: id => nodes[id].listeners.click() };
}

test('available update is displayed as text and installation is explicit', async () => {
  const h = harness(async cmd => cmd === 'check_app_update'
    ? {current_version:'0.1.0', version:'<new version>'} : undefined);
  await tick();
  assert.equal(h.nodes.install.hidden, false);
  assert.match(h.nodes.status.textContent, /<new version>/);
  assert.deepEqual(h.calls, ['check_app_update']);
  await h.click('install');
  assert.deepEqual(h.calls, ['check_app_update','install_app_update']);
  assert.equal(h.nodes.open.disabled, true);
});

test('up-to-date application remains available without installation', async () => {
  const h = harness(async () => ({current_version:'0.1.0',version:null}));
  await tick();
  assert.equal(h.nodes.install.hidden, true);
  assert.match(h.nodes.status.textContent, /up to date/);
  await h.click('open');
  assert.deepEqual(h.calls, ['check_app_update','open_application']);
});

test('offline checking still permits opening and retrying', async () => {
  const h = harness(async cmd => {
    if(cmd === 'check_app_update') throw new Error('Offline');
  });
  await tick();
  assert.equal(h.nodes.open.disabled, false);
  assert.equal(h.nodes.retry.hidden, false);
  assert.match(h.nodes.status.textContent, /Offline/);
  await h.click('open');
  assert.deepEqual(h.calls, ['check_app_update','open_application']);
});

test('late check does not offer installation after application opens', async () => {
  let finish;
  const h = harness(cmd => cmd === 'check_app_update'
    ? new Promise(resolve => { finish = resolve; }) : Promise.resolve());
  await h.click('open');
  finish({current_version:'0.1.0',version:'0.1.1'});
  await tick();
  assert.equal(h.nodes.install.hidden, true);
  assert.deepEqual(h.calls, ['check_app_update','open_application']);
});

test('failed download or signature validation leaves existing version usable', async () => {
  const h = harness(async cmd => {
    if(cmd === 'check_app_update') return {current_version:'0.1.0',version:'0.1.1'};
    if(cmd === 'install_app_update') throw new Error('Signature verification failed');
  });
  await tick();
  await h.click('install');
  assert.equal(h.nodes.install.hidden, true);
  assert.equal(h.nodes.open.disabled, false);
  assert.equal(h.nodes.retry.disabled, false);
  await h.click('open');
  assert.deepEqual(h.calls, ['check_app_update','install_app_update','open_application']);
});

test('repeated install clicks cannot submit another installation', async () => {
  let finish;
  const h = harness(cmd => cmd === 'check_app_update'
    ? Promise.resolve({current_version:'0.1.0',version:'0.1.1'})
    : new Promise(resolve => { finish = resolve; }));
  await tick();
  const pending = h.click('install');
  await h.click('install');
  assert.equal(h.calls.filter(cmd => cmd === 'install_app_update').length, 1);
  finish();
  await pending;
});
