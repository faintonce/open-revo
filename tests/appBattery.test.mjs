import test from 'node:test';
import assert from 'node:assert/strict';
import React from 'react';
import { create, act } from 'react-test-renderer';
import { componentLoader } from './componentLoader.mjs';

async function fixture(t, fanMode = 'hardware_toggle', snapshot = true, deferConfig = false) {
  const requests = [], alerts = [], listeners = new Map();
  let props, tree;
  const listen = async (event, handler) => {
    listeners.set(event, handler);
    return () => listeners.delete(event);
  };
  const win = {
    listen, innerSize: async () => ({ width: 400 }), scaleFactor: async () => 1,
    isVisible: async () => true, isMinimized: async () => false,
  };
  const window = { __TAURI_INTERNALS__: {}, location: { search: '' }, innerWidth: 400,
    addEventListener() {}, removeEventListener() {} };
  const load = componentLoader((name) => {
    if (name === 'react') return React;
    if (name === '@tauri-apps/api/core') return { invoke: (command, args) => {
      if (command === 'get_hardware_status') return new Promise(() => {});
      if (command === 'get_model_capabilities') return Promise.resolve({ fan_boost_mode: fanMode });
      if (command === 'get_app_config' && deferConfig) return new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }));
      if (command === 'get_app_config') return Promise.resolve({ power_mode: 2, fan_boost: false, battery_limit: 80 });
      if (['set_fan_boost', 'set_bld_fan_speed_cap', 'set_battery_limit', 'set_power_mode', 'toggle_oem_service'].includes(command)) {
        return new Promise((resolve, reject) => requests.push({ command, args, resolve, reject }));
      }
      return Promise.resolve();
    } };
    if (name === '@tauri-apps/api/event') return { listen };
    if (name === '@tauri-apps/api/window') return { getCurrentWindow: () => win };
    if (name === './components/FullDashboard') return { FullDashboard: () => null };
    if (name === './components/MiniDrawer') return { MiniDrawer: (next) => { props = next; return null; } };
    if (name === './components/ModuleErrorBoundary') return { ModuleErrorBoundary: ({ children }) => children };
    if (name === './components/pluginAppearance') return { createPluginAppearance: () => ({ restore: async () => {}, apply: async () => {} }) };
    if (name === './components/pluginChanges') return { subscribePluginChanges: () => ({ ready: Promise.resolve(), stop() {} }) };
    throw new Error(`Unexpected import: ${name}`);
  }, { React, window, URLSearchParams, setInterval: () => 0, clearInterval() {}, console: { ...console, error() {}, log() {} },
    localStorage: { getItem: () => null }, document: { documentElement: { setAttribute() {}, removeAttribute() {} } },
    alert: (message) => alerts.push(message) });
  const { default: App } = load('../App.tsx');
  await act(async () => { tree = create(React.createElement(App)); });
  t.after(() => act(() => tree.unmount()));
  const state = {
    requests, alerts,
    get props() { return props; },
    status(active) { state.snapshot({ fan_boost: active, support_flags: { has_fan_boost: true } }); },
    snapshot(patch) { act(() => listeners.get('hardware-state-updated')({ payload: { ...props.status, ...patch } })); },
    physical(active) { act(() => listeners.get('physical-fan-boost-switched')({ payload: active })); },
    click(rpm) { act(() => { props.onToggleFanBoost(rpm); }); },
    battery(limit) { act(() => { props.onToggleBatteryLimit(limit); }); },
    mode(mode) { act(() => { props.onSetPowerMode(mode); }); },
    physicalMode(mode) { act(() => listeners.get('physical-mode-switched')({ payload: mode })); },
    async complete(request, value, reject = false) { await act(async () => { request[reject ? 'reject' : 'resolve'](value); }); },
  };
  if (snapshot) state.status(false);
  return state;
}

test('main battery control waits for acknowledgement, blocks duplicates and keeps failure visible', async (t) => {
  const state = await fixture(t);
  state.battery();
  state.battery();
  assert.equal(state.requests.length, 1);
  assert.equal(state.requests[0].args.limit, 60);
  assert.equal(state.props.status.battery_limit, 80);
  await state.complete(state.requests[0], 'battery write failed', true);
  assert.equal(state.props.status.battery_limit, 80);
  assert.match(state.alerts[0], /battery write failed/);
  state.snapshot({ battery_limit: 90 });
  assert.equal(state.props.status.battery_limit, 90, 'failure must not leave a stale limit override');
  for (const limit of [60, 100, 80]) {
    state.battery(limit);
    const request = state.requests.at(-1);
    assert.equal(request.args.limit, limit);
    await state.complete(request);
    assert.equal(state.props.status.battery_limit, limit);
  }
});

test('late battery acknowledgements preserve newer telemetry', async (t) => {
  const state = await fixture(t);
  state.battery(60);
  state.snapshot({ battery_limit: 70 });
  await state.complete(state.requests[0]);
  assert.equal(state.props.status.battery_limit, 70);
  state.snapshot({ battery_limit: 90 });
  assert.equal(state.props.status.battery_limit, 90);
});

test('a native host without its first snapshot still submits battery commands', async (t) => {
  const state = await fixture(t, 'hardware_toggle', false);
  state.battery(60);
  assert.equal(state.requests.length, 1);
  assert.equal(state.props.status.battery_limit, 80);
  await state.complete(state.requests[0], 'host not ready', true);
  assert.equal(state.props.status.battery_limit, 80);
});

test('a late boot configuration preserves an acknowledged battery change before the first snapshot', async (t) => {
  const state = await fixture(t, 'hardware_toggle', false, true);
  state.battery(60);
  await state.complete(state.requests.find(({ command }) => command === 'set_battery_limit'));
  await state.complete(state.requests.find(({ command }) => command === 'get_app_config'), {
    power_mode: 2, fan_boost: false, battery_limit: 80,
  });
  assert.equal(state.props.status.battery_limit, 60);
});

test('a late boot configuration preserves newer battery telemetry', async (t) => {
  const state = await fixture(t, 'hardware_toggle', true, true);
  state.snapshot({ fan_boost: true, battery_limit: 70 });
  await state.complete(state.requests.find(({ command }) => command === 'get_app_config'), {
    power_mode: 2, fan_boost: false, battery_limit: 80,
  });
  assert.equal(state.props.status.battery_limit, 70);
});

test('boot configuration still supplies battery defaults before telemetry or user actions', async (t) => {
  const state = await fixture(t, 'hardware_toggle', false, true);
  await state.complete(state.requests.find(({ command }) => command === 'get_app_config'), {
    power_mode: 1, fan_boost: true, battery_limit: 60,
  });
  assert.equal(state.props.status.power_mode, 1);
  assert.equal(state.props.status.battery_limit, 60);
});
