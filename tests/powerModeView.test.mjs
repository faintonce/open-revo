import test from 'node:test';
import assert from 'node:assert/strict';
import React from 'react';
import { create, act } from 'react-test-renderer';
import { componentLoader } from './componentLoader.mjs';

async function mainWindow(t, deferConfig = false) {
  const requests = [], alerts = [], listeners = new Map();
  let props, tree;
  const listen = async (name, handler) => {
    listeners.set(name, handler);
    return () => listeners.delete(name);
  };
  const win = { listen, innerSize: async () => ({ width: 400 }), scaleFactor: async () => 1 };
  const load = componentLoader((name) => {
    if (name === 'react') return React;
    if (name === '@tauri-apps/api/core') return { invoke: (command, args) => {
      if (command === 'get_app_config' && !deferConfig) return Promise.resolve({ power_mode: 2 });
      if (command === 'get_model_capabilities') return Promise.resolve({});
      if (['set_power_mode', 'get_hardware_status', 'get_app_config'].includes(command)) {
        return new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }));
      }
      return Promise.resolve();
    } };
    if (name === '@tauri-apps/api/event') return { listen };
    if (name === '@tauri-apps/api/window') return { getCurrentWindow: () => win };
    if (name === './components/MiniDrawer') return { MiniDrawer: (next) => { props = next; return null; } };
    if (name === './components/FullDashboard') return { FullDashboard: () => null };
    throw new Error(`Unexpected import: ${name}`);
  }, {
    React, URLSearchParams, console: { ...console, error() {}, log() {} },
    window: { __TAURI_INTERNALS__: {}, location: { search: '' }, innerWidth: 400,
      addEventListener() {}, removeEventListener() {} },
    localStorage: { getItem: () => null },
    document: { documentElement: { setAttribute() {}, removeAttribute() {} } },
    setInterval: () => 1, clearInterval() {}, setTimeout: () => 1, clearTimeout() {},
    alert: (message) => alerts.push(message),
  }, {
    // Load the real boot-authority helpers instead of stubbing them.
    loadRelative: ['./lib/powerModeAuthority'],
  });
  const { default: App } = load('../App.tsx');
  await act(async () => { tree = create(React.createElement(App)); });
  t.after(() => act(() => tree.unmount()));
  return {
    requests, alerts,
    get mode() { return props.status.power_mode; },
    choose(mode) { act(() => { props.onSetPowerMode(mode); }); },
    snapshot(mode) { act(() => listeners.get('hardware-state-updated')({ payload: { ...props.status, power_mode: mode } })); },
    physical(mode) { act(() => listeners.get('physical-mode-switched')({ payload: mode })); },
    async complete(request, value, failed = false) {
      await act(async () => { request[failed ? 'reject' : 'resolve'](value); });
    },
  };
}

test('main mode control waits for confirmation, blocks duplicates and reports failure', async (t) => {
  const state = await mainWindow(t);
  state.choose(3);
  state.choose(4);
  const pending = state.requests.filter(({ command }) => command === 'set_power_mode');
  assert.equal(pending.length, 1);
  assert.equal(state.mode, 2);
  await state.complete(pending[0], 'hardware failed', true);
  assert.equal(state.mode, 2);
  assert.match(state.alerts[0], /hardware failed/);
  state.choose(4);
  await state.complete(state.requests.filter(({ command }) => command === 'set_power_mode')[1]);
  assert.equal(state.mode, 4);
});

test('late main mode confirmation preserves newer physical events and telemetry', async (t) => {
  const state = await mainWindow(t);
  state.choose(3);
  state.physical(1);
  await state.complete(state.requests.find(({ command }) => command === 'set_power_mode'));
  assert.equal(state.mode, 1);
  state.choose(4);
  state.snapshot(2);
  state.snapshot(1);
  await state.complete(state.requests.filter(({ command }) => command === 'set_power_mode')[1]);
  assert.equal(state.mode, 1);
});

test('late startup configuration preserves a confirmed mode selection', async (t) => {
  const state = await mainWindow(t, true);
  state.choose(3);
  await state.complete(state.requests.find(({ command }) => command === 'set_power_mode'));
  await state.complete(state.requests.find(({ command }) => command === 'get_app_config'), { power_mode: 1 });
  assert.equal(state.mode, 3);
});

test('startup configuration supplies the initial mode but preserves newer telemetry', async (t) => {
  const state = await mainWindow(t, true);
  state.snapshot(4);
  await state.complete(state.requests.find(({ command }) => command === 'get_app_config'), { power_mode: 1 });
  assert.equal(state.mode, 4);
  const initial = await mainWindow(t, true);
  await initial.complete(initial.requests.find(({ command }) => command === 'get_app_config'), { power_mode: 1 });
  assert.equal(initial.mode, 1);
});

async function builtInWidget(t, available = false) {
  const requests = [], listeners = new Map();
  let tree;
  const load = componentLoader((name) => {
    if (name === 'react') return React;
    if (name === '@tauri-apps/api/core') return { invoke: (command, args) => {
      if (command === 'set_power_mode') return new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }));
      if (command === 'get_hardware_status') return Promise.resolve({ power_mode: 2, high_power_modes_available: available });
      if (command === 'load_plugin_html') return Promise.resolve('');
      return Promise.resolve();
    } };
    if (name === '@tauri-apps/api/event') return {
      emit: async () => {},
      listen: async (name, handler) => { listeners.set(name, handler); return () => listeners.delete(name); },
    };
    if (name === '@tauri-apps/api/window') return { getCurrentWindow: () => ({}) };
    if (name === '@tauri-apps/api/webviewWindow') return { getCurrentWebviewWindow: () => ({}) };
    if (name === '@tauri-apps/api/dpi') return {};
    if (name === 'lucide-react') return new Proxy({}, { get: () => () => null });
    throw new Error(`Unexpected import: ${name}`);
  }, { window: { addEventListener() {}, removeEventListener() {} }, console,
    setInterval: () => 1, clearInterval() {} });
  const { WidgetHost } = load('WidgetHost.tsx');
  await act(async () => { tree = create(React.createElement(WidgetHost, { pluginId: 'core' })); });
  t.after(() => act(() => tree.unmount()));
  const button = () => tree.root.findByProps({ title: '点击轮转性能模式' });
  return {
    requests,
    get label() { return button().findByType('span').children[0]; },
    get disabled() { return button().props.disabled; },
    get error() { return tree.root.findByProps({ role: 'alert' }).children.join(''); },
    choose() { act(() => { button().props.onClick({ stopPropagation() {} }); }); },
    snapshot(payload) { act(() => listeners.get('hardware-state-updated')({ payload })); },
    async complete(request, value, failed = false) {
      await act(async () => { request[failed ? 'reject' : 'resolve'](value); });
    },
  };
}

test('built-in widget waits for mode confirmation and exposes failures', async (t) => {
  const state = await builtInWidget(t);
  state.choose();
  state.choose();
  assert.equal(state.requests.length, 1);
  assert.equal(state.disabled, true);
  assert.equal(state.label, '均衡');
  await state.complete(state.requests[0], 'hardware failed', true);
  assert.equal(state.disabled, false);
  assert.equal(state.label, '均衡');
  assert.match(state.error, /hardware failed/);
});

test('built-in widget cycling follows current high-power availability', async (t) => {
  const state = await builtInWidget(t);
  state.choose();
  assert.equal(state.requests[0].args.mode, 1);
  await state.complete(state.requests[0]);
  state.choose();
  assert.equal(state.requests[1].args.mode, 2);
  await state.complete(state.requests[1]);
  state.snapshot({ power_mode: 2, high_power_modes_available: true });
  state.choose();
  assert.equal(state.requests[2].args.mode, 3);
  await state.complete(state.requests[2]);
  assert.equal(state.label, '狂暴');
});

test('late widget mode confirmation preserves telemetry even after a state round trip', async (t) => {
  const state = await builtInWidget(t, true);
  state.choose();
  state.snapshot({ power_mode: 1 });
  state.snapshot({ power_mode: 2 });
  await state.complete(state.requests[0]);
  assert.equal(state.label, '均衡');
});
