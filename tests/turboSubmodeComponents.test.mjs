import test from 'node:test';
import assert from 'node:assert/strict';
import React from 'react';
import { create, act } from 'react-test-renderer';
import { componentLoader } from './componentLoader.mjs';

function setup(t, name, props, hookState) {
  const invoked = [];
  const applied = [];
  const load = componentLoader((id) => {
    if (id === 'react') return React;
    if (id === '@tauri-apps/api/core') {
      return {
        invoke: (command, args) => {
          invoked.push({ command, args });
          if (command === 'get_app_config') return Promise.resolve({});
          if (command === 'get_gpu_oc_state') return Promise.resolve([0, 0]);
          return Promise.resolve({ supported: true, silent: false });
        },
      };
    }
    if (id === '@tauri-apps/api/event') return { listen: () => Promise.resolve(() => {}) };
    if (id === 'lucide-react') return new Proxy({}, { get: () => () => null });
    if (id === '../types/hardware' || id === '../../types/hardware') {
      return {
        POWER_MODES: [
          { id: 1, name: '办公模式', nameEn: 'Office', desc: 'Office' },
          { id: 2, name: '均衡模式', nameEn: 'Balance', desc: 'Balance' },
          { id: 3, name: '狂暴模式', nameEn: 'Turbo', desc: 'Turbo' },
          { id: 4, name: '极客模式', nameEn: 'Custom', desc: 'Custom' },
        ],
      };
    }
    if (id === '../hooks/useTurboSubmode' || id === '../../hooks/useTurboSubmode') {
      return {
        useTurboSubmode: () => ({
          supported: true,
          silent: false,
          busy: false,
          error: null,
          apply: async (silent) => { applied.push(silent); },
          ...hookState,
        }),
      };
    }
    throw new Error(`Unexpected import: ${id}`);
  }, { console: { ...console, warn() {}, error() {} } }); // deliberately no setTimeout/setInterval

  const Component = load(name)[name.split('/').pop().replace('.tsx', '')];
  let tree;
  act(() => { tree = create(React.createElement(Component, props)); });
  t.after(() => act(() => tree.unmount()));

  const textOf = (node) => {
    if (typeof node === 'string') return node;
    if (Array.isArray(node)) return node.map(textOf).join('');
    if (node && typeof node === 'object' && node.props) return textOf(node.props.children);
    return '';
  };
  const buttons = () => tree.root.findAll((node) => typeof node.type === 'string' && node.type === 'button');
  const buttonWithText = (text) => buttons().find((node) => textOf(node.props.children).includes(text));

  return {
    invoked,
    applied,
    click(text) {
      const button = buttonWithText(text);
      assert.ok(button, `button containing ${text} not rendered`);
      act(() => { button.props.onClick({ stopPropagation() {} }); });
    },
  };
}

test('ModeSelector delegates turbo clicks to the shared hook exactly once', async (t) => {
  const onSelectMode = [];
  const state = setup(t, 'ModeSelector.tsx', {
    currentMode: 2,
    onSelectMode: (mode) => onSelectMode.push(mode),
    capabilities: { power_modes: ['office', 'balance', 'turbo', 'custom'], form_factor: 'standard' },
  });

  await act(async () => {});
  state.click('静音狂暴');
  await act(async () => {});

  assert.deepEqual(state.applied, [true]);
  assert.deepEqual(onSelectMode, []);
  assert.equal(state.invoked.some(({ command }) => command === 'set_power_mode'), false);
  assert.equal(state.invoked.some(({ command }) => command === 'set_turbo_submode'), false);
});

test('DrawerPowerModeCard delegates turbo clicks to the shared hook without set_power_mode', async (t) => {
  const onSetPowerMode = [];
  const state = setup(t, 'drawer/DrawerPowerModeCard.tsx', {
    status: {
      power_mode: 2,
      is_dc_online: true,
      unlock_battery_power_limit: false,
      cpu_temp: 50,
      cpu_fan_rpm: 1200,
    },
    onSetPowerMode: (mode) => onSetPowerMode.push(mode),
    checkOemGuard: (action) => action(),
    onExpandDashboard: () => {},
  });

  await act(async () => {});
  state.click('高能狂暴');
  await act(async () => {});

  assert.deepEqual(state.applied, [false]);
  assert.deepEqual(onSetPowerMode, []);
  assert.equal(state.invoked.some(({ command }) => command === 'set_power_mode'), false);
  assert.equal(state.invoked.some(({ command }) => command === 'set_turbo_submode'), false);
});
