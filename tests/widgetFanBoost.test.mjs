import test from 'node:test';
import assert from 'node:assert/strict';
import React from 'react';
import { create, act } from 'react-test-renderer';
import { componentLoader } from './componentLoader.mjs';

async function builtInWidget(t, available = false) {
  const requests = [], listeners = new Map();
  let tree;
  const load = componentLoader((name) => {
    if (name === 'react') return React;
    if (name === '@tauri-apps/api/core') return { invoke: (command, args) => {
      if (command === 'set_fan_boost') return new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }));
      if (command === 'get_hardware_status') return Promise.resolve({ power_mode: 2, fan_boost: false, support_flags: { has_fan_boost: available } });
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
  const button = () => tree.root.findByProps({ title: '一键全速强冷 (Fan Boost)' });
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

test('built-in fan control waits for confirmation, rejects duplicates and reports failure', async (t) => {
  const state = await builtInWidget(t, true);
  const initial = state.label;
  state.choose(); state.choose();
  assert.equal(state.requests.length, 1);
  assert.equal(state.disabled, true);
  assert.equal(state.label, initial);
  await state.complete(state.requests[0], 'hardware failed', true);
  assert.match(state.error, /hardware failed/);
  assert.equal(state.label, initial);
  state.choose(); await state.complete(state.requests[1]);
  assert.notEqual(state.label, initial);
});

test('built-in fan control checks support and preserves newer telemetry after a round trip', async (t) => {
  const state = await builtInWidget(t, false);
  assert.equal(state.disabled, true);
  state.choose(); assert.equal(state.requests.length, 0);
  state.snapshot({ fan_boost: false, support_flags: { has_fan_boost: true } });
  const initial = state.label;
  state.choose();
  state.snapshot({ fan_boost: true }); state.snapshot({ fan_boost: false });
  await state.complete(state.requests[0]);
  assert.equal(state.label, initial);
});
