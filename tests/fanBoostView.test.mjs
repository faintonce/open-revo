import test from 'node:test';
import assert from 'node:assert/strict';
import React from 'react';
import { create, act } from 'react-test-renderer';
import { componentLoader } from './componentLoader.mjs';

async function fixture(t, fanMode = 'hardware_toggle', snapshot = true, deferConfig = false) {
  const requests = [], alerts = [], listeners = new Map();
  let props, telemetry, tree;
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
    if (name === './components/widgetTelemetry') return { subscribeWidgetTelemetry: (_, handler) => {
      telemetry = handler;
      return () => {};
    } };
    if (name === './components/mainVisibility') return { mainVisibility: {} };
    if (name === './components/pluginAppearance') return { createPluginAppearance: () => ({ restore: async () => {}, apply: async () => {} }) };
    if (name === './components/pluginChanges') return { subscribePluginChanges: () => ({ ready: Promise.resolve(), stop() {} }) };
    throw new Error(`Unexpected import: ${name}`);
  }, { React, window, URLSearchParams, console: { ...console, error() {}, log() {} },
    localStorage: { getItem: () => null }, document: { documentElement: { setAttribute() {}, removeAttribute() {} } },
    setInterval: () => 1, clearInterval() {}, setTimeout: () => 1, clearTimeout() {},
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


test('main fan control waits for acknowledgement, blocks duplicates and exposes failure', async (t) => {
  const state = await fixture(t);
  state.click();
  state.click();
  assert.equal(state.requests.length, 1);
  assert.equal(state.props.status.fan_boost, false);
  await state.complete(state.requests[0], 'hardware write failed', true);
  assert.equal(state.props.status.fan_boost, false);
  assert.match(state.alerts[0], /hardware write failed/);
  state.status(true);
  state.status(false);
  assert.equal(state.props.status.fan_boost, false, 'failure must not leave a telemetry override');
  state.click();
  await state.complete(state.requests[1]);
  assert.equal(state.props.status.fan_boost, true);
});

test('a late fan acknowledgement preserves newer physical fan events', async (t) => {
  const state = await fixture(t);
  state.click();
  state.physical(true);
  state.physical(false);
  await state.complete(state.requests[0]);
  assert.equal(state.props.status.fan_boost, false);
});

test('BLD fan controls retain RPM and automatic-mode arguments', async (t) => {
  const state = await fixture(t, 'direct_rpm_clamp');
  state.click(4200);
  assert.equal(state.requests[0].command, 'set_bld_fan_speed_cap');
  assert.equal(state.requests[0].args.rpm, 4200);
  assert.equal(state.props.status.fan_boost, false);
  await state.complete(state.requests[0]);
  state.click(4200);
  assert.equal(state.requests[1].args.rpm, 0);
  await state.complete(state.requests[1], 'settings failed', true);
  assert.equal(state.props.status.fan_boost, true);
  assert.match(state.alerts[0], /settings failed/);
});

test('a native host without its first snapshot still submits fan commands', async (t) => {
  const state = await fixture(t, 'hardware_toggle', false);
  state.click();
  assert.equal(state.requests.length, 1);
  assert.equal(state.props.status.fan_boost, false);
  await state.complete(state.requests[0], 'host not ready', true);
  assert.equal(state.props.status.fan_boost, false);
});

test('a late boot configuration preserves acknowledged fan changes before the first snapshot', async (t) => {
  const state = await fixture(t, 'hardware_toggle', false, true);
  state.click();
  await state.complete(state.requests.find(({ command }) => command === 'set_fan_boost'));
  await state.complete(state.requests.find(({ command }) => command === 'get_app_config'), {
    power_mode: 2, fan_boost: false, battery_limit: 80,
  });
  assert.equal(state.props.status.fan_boost, true);
});

test('a late boot configuration preserves newer fan telemetry', async (t) => {
  const state = await fixture(t, 'hardware_toggle', true, true);
  state.snapshot({ fan_boost: true, battery_limit: 70 });
  await state.complete(state.requests.find(({ command }) => command === 'get_app_config'), {
    power_mode: 2, fan_boost: false, battery_limit: 80,
  });
  assert.equal(state.props.status.fan_boost, true);
});

test('boot configuration still supplies fan defaults before telemetry or user actions', async (t) => {
  const state = await fixture(t, 'hardware_toggle', false, true);
  await state.complete(state.requests.find(({ command }) => command === 'get_app_config'), {
    power_mode: 1, fan_boost: true, battery_limit: 60,
  });
  assert.equal(state.props.status.power_mode, 1);
  assert.equal(state.props.status.fan_boost, true);
});
