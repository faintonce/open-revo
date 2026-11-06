import test from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync, mkdirSync, mkdtempSync } from 'node:fs';
import { join, resolve } from 'node:path';

const project = resolve(import.meta.dirname, '..');
const python = join(project, 'scripts/check_vault_leaks.py');
const wrapper = join(project, 'scripts/check_vault_leaks.ps1');
const targets = join(project, 'src-tauri/target');
mkdirSync(targets, { recursive: true });

function fixture() {
  const root = mkdtempSync(join(targets, 'vault-gate-'));
  for (const path of ['src-tauri/config', 'src-tauri/src/hal', 'src-tauri/crates/openrevo-core/src/hal', 'src']) mkdirSync(join(root, path), { recursive: true });
  writeFileSync(join(root, 'src-tauri/config/ec_registry.json'), JSON.stringify({ ec: { ADDR_TEST: 1856 }, driver: { IOCTL_TEST: 2621482120 } }));
  writeFileSync(join(root, 'src-tauri/src/hal/vault.rs'), 'pub const ADDR_TEST: u16 = 1856;');
  return root;
}

function run(root) {
  return spawnSync('python', ['-B', python, '--root', root], { encoding: 'utf8', windowsHide: true });
}

test('vault gate accepts symbolic HAL references and ordinary time, size, and text values', () => {
  const root = fixture();
  writeFileSync(join(root, 'src-tauri/src/control.rs'), 'let name = "server 1856";\nlet delay = Duration::from_millis(1856);\ncurves.fan0[0].rpm = 1856;\nassert_eq!(loaded.fan0[0].rpm, 1856);\nlet settings = RpmSmoothing { rise_rpm_per_sec: 1856 };\nwmi_write_ec(crate::hal::vault::ec::ADDR_TEST, 1);');
  writeFileSync(join(root, 'src/control.ts'), 'const rates = [500, 1856, curves.smoothing.fall_rpm_per_sec];\nconst ticks = [minRpm, 1856, maxRpm];');
  const result = run(root);
  assert.equal(result.status, 0, result.stdout + result.stderr);
});

test('vault gate rejects bare addresses, aliases, IOCTLs, protocol frames, and address-based names', () => {
  const cases = [
    'wmi_write_ec(0x0999, 1);',
    'wmi_write_ec(1856, 1); // Duration::from_millis(1856)',
    'wmi_write_ec(1856, rpm);',
    'wmi_write_ec_batch(&[(2718, 1)]);',
    'const ADDR_OTHER: u16 = 2718;',
    'let anything: u16 = 0x0740;',
    'const IOCTL_OTHER: u32 = 0x12345678;',
    'const FRAME_HEADER: u8 = 0xAA;',
    'const PACKET_PING: [u8; 2] = [0xAA, 0x55];',
    'let cur_1856 = data;',
    '// use pre_1856 for the last value',
    'RawEcWriteRequest { address: 2718, value: 1 }',
  ];
  const root = fixture();
  for (const source of cases) {
    writeFileSync(join(root, 'src-tauri/src/control.rs'), source);
    const result = run(root);
    assert.equal(result.status, 1, source + '\n' + result.stdout + result.stderr);
  }
  writeFileSync(join(root, 'src-tauri/src/control.rs'), '');
  writeFileSync(join(root, 'src/control.ts'), 'const request = { address: 2718 };');
  assert.equal(run(root).status, 1);
});
