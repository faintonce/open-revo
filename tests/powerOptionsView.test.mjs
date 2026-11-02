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

test('power options keep their IPC arguments and ignore duplicate switch events', async (t) => {
  const state = fixture(t);
  const values = {
    ...initial, sleep_auto_office: false, sleep_guard_enabled: false,
    sleep_disconnect_net: false, sleep_block_away: false, sleep_ignore_sysreq: false,
    sleep_disable_wake_timers: false, sleep_deep_sleep: false, sleep_hibernate_30m: false,
    usb_charge_enabled: false, ac_recovery_enabled: false, high_perf_scheme: false,
  };
  await state.complete(state.requests[0], values);
  const cases = [
    ...['usb_charge', 'ac_recovery', 'high_perf']
      .map((option) => ['set_power_option', { option, enable: true }, option === 'high_perf' ? 'high_perf_scheme' : `${option}_enabled`]),
  ];
  for (const [index, [command, args, key]] of cases.entries()) {
    const toggle = state.root.findAllByType('input')[index + 8];
    assert.equal(toggle.props.checked, false);
    const before = state.requests.length;
    act(() => { toggle.props.onChange(); toggle.props.onChange(); });
    assert.equal(state.requests.length, before + 1);
    const request = state.requests.at(-1);
    assert.equal(request.command, command);
    assert.deepEqual({ ...request.args }, args);
    assert.equal(state.root.findAllByType('input')[index + 8].props.checked, false);
    await state.complete(request, { ...values, [key]: true });
    assert.equal(state.root.findAllByType('input')[index + 8].props.checked, true);
  }
  act(() => { state.root.findAllByType('input').at(-1).props.onChange(); });
  await state.complete(state.requests.at(-1), 'power option failed', true);
  assert.equal(state.root.findAllByType('input').at(-1).props.checked, true);
  assert.match(state.root.findByProps({ role: 'alert' }).children.join(''), /power option failed/);
});
