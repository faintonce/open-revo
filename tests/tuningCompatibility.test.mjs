import test from 'node:test';
import assert from 'node:assert/strict';
import React from 'react';
import { create, act } from 'react-test-renderer';
import { componentLoader } from './componentLoader.mjs';

const marker = name => props => React.createElement(name, props);

async function fixture(t, mode = 4, overrides = {}) {
  const requests = [], pending = new Set();
  let profile, failRamp = false;
  const load = componentLoader((name, load) => {
    if (name === 'react') return React;
    if (name === 'lucide-react') return new Proxy({}, { get: () => () => null });
    if (name === '../ui/RevSwitch') return { RevSwitch: marker('rev-switch') };
    if (name === '@tauri-apps/api/event') return { listen: async () => () => {} };
    if (name === '@tauri-apps/api/core') return { invoke: async (command, args) => {
      requests.push({ command, args });
      const preset = { id: 'preset_silent_beast', name: 'Test', file_name: 'test.json', profile };
      if (command === 'get_profiles_list' || command === 'get_default_builtin_presets') return [preset];
      if (command === 'get_active_profile_id') return preset.id;
      if (command === 'get_app_config') return { cpu_temp_target_min: 85, cpu_temp_target_max: 95 };
      if (command === 'get_gpu_tuning_capabilities') return { supports_gpu_tuning: false };
      if (command === 'get_cpu_power_status') return { min_watts: 5, max_watts: 50 };
      if (command === 'toggle_fan_ramp_rate' && failRamp) throw new Error('save failed');
      if (['toggle_fan_ramp_rate', 'toggle_fan_isolated_output', 'apply_fan_curve_live', 'save_profile_preset'].includes(command)) return true;
      throw new Error(`Unexpected command: ${command}`);
    } };
    if (name === './tuning/FanCurveEditor') return load('tuning/FanCurveEditor.tsx');
    const children = {
      './tuning/TuningToolbar': ['TuningToolbar', 'toolbar'],
      './tuning/CpuTuningSection': ['CpuTuningSection', 'cpu-section'],
      './tuning/GpuTuningSection': ['GpuTuningSection', 'gpu-section'],
      './tuning/BldFanStrategyPanel': ['BldFanStrategyPanel', 'bld-fan'],
    };
    if (children[name]) return { [children[name][0]]: marker(children[name][1]) };
    throw new Error(`Unexpected import: ${name}`);
  }, { console,
    setTimeout: callback => { pending.add(callback); return callback; },
    clearTimeout: callback => pending.delete(callback),
    setInterval: () => 0, clearInterval() {},
  });
  const { CustomTuningPanel, DEFAULT_8_POINT_CURVE } = load('CustomTuningPanel.tsx');
  const { FanCurveEditor } = load('tuning/FanCurveEditor.tsx');
  profile = { cpu_pl1_watts: 15, cpu_pl2_watts: 20, cpu_pl4_watts: 30, cpu_temp_target: 95,
    cpu_fan_curve: DEFAULT_8_POINT_CURVE, gpu_fan_curve: DEFAULT_8_POINT_CURVE,
    fan_curve_enabled: false, fan_isolated_output: true, fan_ramp_rate: true, fan_ramp_speed_ms: 300, ...overrides };
  let tree;
  await act(async () => { tree = create(React.createElement(CustomTuningPanel, {
    status: { power_mode: mode, cpu_fan_rpm: 2400, gpu_fan_rpm: 2300, gpu_name: 'Intel Arc' },
    capabilities: { cpu_power_backend: 'intel_rapl', cpu_power_max_watts: 50,
      fan_control_mode: 'pmc_rpm_curve', fan_independent_channels: true,
      fan_ramp_rate: false, fan_software_smoothing: true, gpu_power_tuning_mode: 'none',
      power_modes: ['office', 'balance', 'turbo', 'custom'] },
  })); });
  t.after(() => act(() => tree.unmount()));
  return {
    requests, get root() { return tree.root; },
    get fan() { return tree.root.findByType(FanCurveEditor); },
    rejectRamp() { failRamp = true; },
    async flush() { await act(async () => {
      for (const callback of [...pending]) { pending.delete(callback); await callback(); }
    }); },
  };
}

test('PTL keeps the original tuning controls and millisecond selector with a software backend', async t => {
  const state = await fixture(t, 2);
  assert.equal(state.root.findAllByType('cpu-section').length, 1);
  assert.equal(state.root.findAllByType('gpu-section').length, 1);
  assert.equal(state.fan.props.supportFanRampRate, true);
  assert.equal(state.fan.props.fanIsolated, false);
  assert.deepEqual(state.root.findAllByType('option').map(option => [option.props.value, option.children.join('')]),
    [['100', '100ms'], ['300', '300ms'], ['500', '500ms'], ['1000', '1000ms']]);
  await act(async () => { await state.fan.props.onChangeFanRampSpeed(500); });
  const command = state.requests.find(request => request.command === 'toggle_fan_ramp_rate');
  assert.deepEqual({ ...command.args }, { enable: true, speedMs: 500 });
  assert.equal(state.fan.props.fanRampSpeedMs, 500);
  await act(async () => { await state.fan.props.onToggleFanRampRate(false); });
  assert.equal(state.fan.props.fanRampRate, false);
  await state.flush();
  assert.equal(state.requests.some(request => request.command === 'apply_custom_profile' || request.command === 'heal_gpu_power_contract'), false);
});

test('curve and software ramp changes preserve pending CPU edits in the original page', async t => {
  const state = await fixture(t);
  act(() => state.root.findByType('cpu-section').props.onChangePl1(18));
  await act(async () => { await state.fan.props.onToggleFanIsolated(true); });
  const next = state.fan.props.cpuCurve.map((point, index) => ({ ...point, duty: index === 0 ? 25 : point.duty }));
  act(() => state.fan.props.onCurveChange('cpu', next));
  await act(async () => { await state.fan.props.onChangeFanRampSpeed(1000); });
  await state.flush();
  const saved = state.requests.filter(request => request.command === 'save_profile_preset').at(-1);
  assert.equal(saved.args.preset.profile.cpu_pl1_watts, 18);
  assert.equal(saved.args.preset.profile.fan_curve_enabled, true);
  assert.equal(saved.args.preset.profile.cpu_fan_curve[0].duty, 25);
  assert.equal(saved.args.preset.profile.fan_ramp_speed_ms, 1000);
  assert.equal(saved.args.legacyFanCurveChanged, true);
  assert.equal(state.requests.some(request => request.command === 'apply_fan_curve_live'), true);
});

test('a failed software smoothing save keeps the displayed switch and interval unchanged', async t => {
  const state = await fixture(t);
  state.rejectRamp();
  await act(async () => { await state.fan.props.onToggleFanRampRate(false); });
  assert.equal(state.fan.props.fanRampRate, true);
  await act(async () => { await state.fan.props.onChangeFanRampSpeed(100); });
  assert.equal(state.fan.props.fanRampSpeedMs, 300);
  assert.match(state.root.findByType('cpu-section').props.controlError, /save failed/);
  await state.flush();
  assert.equal(state.requests.some(request => request.command === 'save_profile_preset'), false);
});

test('existing RPM policies populate the original controls from their saved software settings', async t => {
  const state = await fixture(t, 4, {
    fan_ramp_rate: false, fan_ramp_speed_ms: 300,
    ptl_rpm_fan_curves: { smoothing: { enabled: true, step_interval_ms: 1000 }, fan0: [], fan1: [], linked: true },
  });
  assert.equal(state.fan.props.fanRampRate, true);
  assert.equal(state.fan.props.fanRampSpeedMs, 1000);
  await state.flush();
  assert.equal(state.requests.some(request => request.command === 'save_profile_preset'), false);
});
