import test from 'node:test';
import assert from 'node:assert/strict';
import React from 'react';
import { create, act } from 'react-test-renderer';
import { componentLoader } from './componentLoader.mjs';

const curves = {
  fan0: [{ temp: 40, rpm: 2200 }, { temp: 90, rpm: 7500 }],
  fan1: [{ temp: 40, rpm: 2200 }, { temp: 90, rpm: 7500 }],
  linked: true,
  smoothing: { enabled: true, rise_rpm_per_sec: 1500, fall_rpm_per_sec: 300, fall_delay_secs: 3 },
};
const marker = name => props => React.createElement(name, props);

function timers() {
  const pending = new Set();
  return {
    pending,
    setTimeout: callback => { pending.add(callback); return callback; },
    clearTimeout: callback => pending.delete(callback),
    setInterval: () => 0,
    clearInterval() {},
  };
}

test('RPM fan controls send fixed targets, curves and smoothing through the original tuning column', async (t) => {
  const requests = [], saved = [], clock = timers();
  let rejectSmoothing = false;
  let control = { mode: 'manual', requested_mode: 'manual', target_rpm: [1200, 1000],
    rpm_limits: { minimum: [1200, 1000], maximum: 7500, step: 100, startup_rpm: 2000, startup_secs: 2 } };
  const load = componentLoader(name => {
    if (name === 'react') return React;
    if (name === '@tauri-apps/api/core') return { invoke: async (command, args) => {
      requests.push({ command, args });
      if (command === 'get_fan_control_state') return structuredClone(control);
      if (command === 'get_rpm_fan_curves') return structuredClone(curves);
      if (command === 'get_custom_profile') return { fan_curve_enabled: control.requested_mode === 'curve' };
      if (command === 'set_manual_fan_rpm') control = { ...control, mode: 'manual', requested_mode: 'manual', target_rpm: [args.cpuRpm, args.gpuRpm] };
      if (command === 'set_rpm_fan_curves') control = { ...control, mode: 'curve', requested_mode: 'curve' };
      if (command === 'restore_fan_auto') control = { ...control, mode: 'auto', requested_mode: 'auto' };
      if (command === 'set_rpm_fan_smoothing' && rejectSmoothing) throw new Error('save failed');
    } };
    if (name === './RpmFanCurveEditor') return { RpmFanCurveEditor: marker('rpm-editor') };
    if (name === '../ui/RevSwitch') return { RevSwitch: marker('rev-switch') };
    throw new Error(`Unexpected import: ${name}`);
  }, { console, ...clock });
  const { RpmFanControlPanel } = load('tuning/RpmFanControlPanel.tsx');
  let tree;
  await act(async () => { tree = create(React.createElement(RpmFanControlPanel, { fan0: 1190, fan1: 990, onPolicySaved: value => saved.push(value) })); });
  t.after(() => act(() => tree.unmount()));
  const sliders = () => tree.root.findAllByType('input').filter(input => input.props.type === 'range');
  assert.deepEqual(sliders().map(input => [input.props.min, input.props.value]), [[1200, 1200], [1000, 1000]]);
  const button = text => tree.root.findAllByType('button').find(node => node.children.join('') === text);
  act(() => sliders()[0].props.onChange({ target: { value: '1700' } }));
  await act(async () => { await button('应用固定转速').props.onClick(); });
  assert.deepEqual({ ...requests.find(request => request.command === 'set_manual_fan_rpm').args }, { cpuRpm: 1700, gpuRpm: 1000 });
  assert.equal(saved.length, 1);

  // Failed persistence must restore the displayed smoothing value.
  rejectSmoothing = true;
  await act(async () => { tree.root.findByType('rev-switch').props.onChange(false); });
  assert.equal(tree.root.findByType('rev-switch').props.checked, true);
  assert.match(tree.root.findByProps({ role: 'status' }).children.join(''), /save failed/);
  await act(async () => { await button('转速曲线').props.onClick(); });
  assert.equal(button('转速曲线').props.className.includes('active-indigo'), true);
  assert.equal(saved.at(-1).fan_curve_enabled, true);
  const editor = tree.root.findByType('rpm-editor');
  assert.equal(editor.props.minRpm, 1200);
  act(() => editor.props.onChange([{ temp: 40, rpm: 2400 }, { temp: 90, rpm: 7500 }]));
  await act(async () => {
    for (const callback of [...clock.pending]) { clock.pending.delete(callback); await callback(); }
  });
  assert.equal(requests.filter(request => request.command === 'set_rpm_fan_curves').at(-1).args.curves.fan0[0].rpm, 2400);
  await act(async () => { await button('原厂自动').props.onClick(); });
  assert.equal(button('原厂自动').props.className.includes('active-indigo'), true);
});

test('saving RPM policy preserves pending CPU edits and keeps the classic three-column tuning page', async (t) => {
  const requests = [], clock = timers();
  let profile;
  const load = componentLoader(name => {
    if (name === 'react') return React;
    if (name === '@tauri-apps/api/event') return { listen: async () => () => {} };
    if (name === '@tauri-apps/api/core') return { invoke: async (command, args) => {
      requests.push({ command, args });
      const preset = { id: 'preset_silent_beast', name: 'Test', file_name: 'test.json', profile };
      if (command === 'get_profiles_list' || command === 'get_default_builtin_presets') return [preset];
      if (command === 'get_active_profile_id') return preset.id;
      if (command === 'get_app_config') return { cpu_temp_target_min: 85, cpu_temp_target_max: 95 };
      if (command === 'get_gpu_tuning_capabilities') return { supports_gpu_tuning: false };
      if (command === 'get_cpu_power_status') return { max_watts: 50 };
      if (command === 'save_profile_preset') return;
      throw new Error(`Unexpected command: ${command}`);
    } };
    const children = {
      './tuning/FanCurveEditor': ['FanCurveEditor', 'legacy-fan'],
      './tuning/RpmFanControlPanel': ['RpmFanControlPanel', 'rpm-fan'],
      './tuning/TuningToolbar': ['TuningToolbar', 'toolbar'],
      './tuning/CpuTuningSection': ['CpuTuningSection', 'cpu-section'],
      './tuning/GpuTuningSection': ['GpuTuningSection', 'gpu-section'],
      './tuning/BldFanStrategyPanel': ['BldFanStrategyPanel', 'bld-fan'],
    };
    if (children[name]) return { [children[name][0]]: marker(children[name][1]) };
    throw new Error(`Unexpected import: ${name}`);
  }, { console, ...clock });
  const { CustomTuningPanel, DEFAULT_8_POINT_CURVE } = load('CustomTuningPanel.tsx');
  profile = { cpu_pl1_watts: 15, cpu_pl2_watts: 20, cpu_pl4_watts: 30, cpu_temp_target: 95,
    cpu_fan_curve: DEFAULT_8_POINT_CURVE, gpu_fan_curve: DEFAULT_8_POINT_CURVE,
    fan_curve_enabled: false, fan_isolated_output: false };
  let tree;
  await act(async () => { tree = create(React.createElement(CustomTuningPanel, {
    status: { power_mode: 4, cpu_fan_rpm: 2400, gpu_fan_rpm: 2300, gpu_name: 'Intel Arc' },
    capabilities: { cpu_power_backend: 'intel_rapl', cpu_power_max_watts: 50, fan_control_mode: 'pmc_rpm_curve', gpu_power_tuning_mode: 'none', power_modes: ['office', 'balance', 'turbo', 'custom'] },
  })); });
  t.after(() => act(() => tree.unmount()));
  assert.equal(tree.root.findAllByType('cpu-section').length, 1);
  assert.equal(tree.root.findAllByType('gpu-section').length, 1);
  assert.equal(tree.root.findAllByType('legacy-fan').length, 0);
  assert.equal(tree.root.findAllByType('rpm-fan').length, 1);
  act(() => tree.root.findByType('cpu-section').props.onChangePl1(18));
  act(() => tree.root.findByType('rpm-fan').props.onPolicySaved({ ...profile, ptl_rpm_fan_curves: curves, fan_curve_enabled: true, fan_isolated_output: true }));
  assert.equal(tree.root.findByType('cpu-section').props.pl1, 18);
  await act(async () => {
    for (const callback of [...clock.pending]) { clock.pending.delete(callback); await callback(); }
  });
  const writes = requests.filter(request => request.command === 'save_profile_preset');
  assert.equal(writes.length, 1);
  assert.equal(writes[0].args.preset.profile.cpu_pl1_watts, 18);
  assert.equal(writes[0].args.preset.profile.fan_curve_enabled, true);
  assert.equal(writes[0].args.preset.profile.ptl_rpm_fan_curves.fan0[0].rpm, 2200);
  assert.equal(requests.some(request => request.command === 'apply_custom_profile' || request.command === 'heal_gpu_power_contract'), false);

  await act(async () => { tree.update(React.createElement(CustomTuningPanel, {
    status: { power_mode: 4, cpu_fan_rpm: 2400, gpu_fan_rpm: 2300 },
    capabilities: { cpu_power_backend: 'intel_rapl', fan_control_mode: 'ec_curve', power_modes: ['custom'] },
  })); });
  assert.equal(tree.root.findAllByType('rpm-fan').length, 0);
  assert.equal(tree.root.findAllByType('legacy-fan').length, 1);
});
