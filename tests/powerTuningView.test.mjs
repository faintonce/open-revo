import test from 'node:test';
import assert from 'node:assert/strict';
import React from 'react';
import { create, act } from 'react-test-renderer';
import { componentLoader } from './componentLoader.mjs';

const initial = { battery_percent: 75, is_ac_online: true, battery_mode: 'balance', battery_limit: 80 };

function fixture(t) {
  const requests = [], timers = new Set();
  const load = componentLoader((name) => {
    if (name === 'react') return React;
    if (name === '@tauri-apps/api/core') return { invoke: (command, args) => new Promise((resolve, reject) => {
      requests.push({ command, args, resolve, reject });
    }) };
    if (name === 'lucide-react') return new Proxy({}, { get: () => () => null });
    if (name === './ui') return {
      RevBadge: ({ children }) => React.createElement('span', null, children),
      RevSwitch: ({ checked, onChange }) => React.createElement('input', { type: 'checkbox', checked, onChange }),
    };
    throw new Error(`Unexpected import: ${name}`);
  }, { console: { ...console, error() {} },
    setInterval: (callback) => { timers.add(callback); return callback; },
    clearInterval: (callback) => timers.delete(callback) });
  const { PowerTuningView } = load('PowerTuningView.tsx');
  let tree;
  act(() => { tree = create(React.createElement(PowerTuningView)); });
  t.after(() => act(() => tree.unmount()));
  const state = {
    requests, timers,
    get root() { return tree.root; },
    button(mode) { return tree.root.findAll((element) => typeof element.type === 'string' && element.props.className?.includes(`battery-mode-${mode}`))[0]; },
    active(mode) { return state.button(mode).props.className.split(/\s+/).includes('active'); },
    async complete(request, value, reject = false) { await act(async () => { request[reject ? 'reject' : 'resolve'](value); }); },
    click(mode) { act(() => { state.button(mode).props.onClick(); }); },
    poll() { act(() => { for (const callback of timers) callback(); }); },
  };
  return state;
}

test('battery mode waits for confirmation and rejects duplicate clicks without hiding failure', async (t) => {
  const state = fixture(t);
  await state.complete(state.requests[0], initial);
  const click = state.button('station').props.onClick;
  act(() => { click(); click(); });
  const commands = state.requests.filter((request) => request.command === 'set_battery_mode');
  assert.equal(commands.length, 1);
  assert.equal(state.active('balance'), true);
  assert.equal(state.button('station').props.disabled, true);
  state.poll();
  assert.equal(state.requests.filter((request) => request.command === 'get_power_settings').length, 1);
  await state.complete(commands[0], 'hardware rejected the mode', true);
  assert.equal(state.active('balance'), true);
  assert.match(state.root.findByProps({ role: 'alert' }).children.join(''), /hardware rejected the mode/);
  state.click('station');
  await state.complete(state.requests.at(-1), { ...initial, battery_mode: 'station', battery_limit: 60 });
  assert.equal(state.active('station'), true);
});

test('late initial and polling settings cannot replace a newer battery operation', async (t) => {
  const state = fixture(t);
  const oldInitial = state.requests[0];
  state.click('station');
  await state.complete(state.requests.at(-1), { ...initial, battery_mode: 'station', battery_limit: 60 });
  await state.complete(oldInitial, initial);
  assert.equal(state.active('station'), true);
  state.poll();
  const oldPoll = state.requests.at(-1);
  state.click('long-life');
  await state.complete(state.requests.at(-1), 'write failed', true);
  await state.complete(oldPoll, initial);
  assert.equal(state.active('station'), true);
});
