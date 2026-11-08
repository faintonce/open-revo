import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

// Rust cannot be compiled in this environment, so these tests pin the three
// reviewed backend invariants structurally: one process-wide apply guard, one
// shared event channel for both writers, and no fake `silent=false` broadcast
// when the register read fails.

const read = (path) => readFileSync(new URL(`../src-tauri/src/${path}`, import.meta.url), 'utf8');

const turbo = read('system/power/turbo_submode.rs');
const coreTurbo = readFileSync(
  new URL('../src-tauri/crates/openrevo-core/src/system/power/turbo_submode.rs', import.meta.url),
  'utf8',
);
const bridge = read('driver/ec_bridge.rs');
const command = read('commands/power_cmd.rs');
const events = read('system/turbo_events.rs');

/** Return the body of `fn <name>(` by brace matching, ignoring strings/comments. */
function fnBody(source, name) {
  const start = source.indexOf(`fn ${name}(`);
  assert.ok(start >= 0, `fn ${name} not found`);
  const open = source.indexOf('{', start);
  assert.ok(open > start, `fn ${name} has no body`);
  let depth = 0;
  for (let i = open; i < source.length; i += 1) {
    const ch = source[i];
    if (ch === '{') depth += 1;
    else if (ch === '}') {
      depth -= 1;
      if (depth === 0) return source.slice(open + 1, i);
    }
  }
  throw new Error(`unbalanced body for fn ${name}`);
}

test('apply_silent serializes the whole hardware sequence behind one process-wide guard', () => {
  const body = fnBody(turbo, 'apply_silent');

  const guardAt = body.indexOf('lock_apply()');
  assert.ok(guardAt >= 0, 'apply_silent must take the shared apply guard');

  // The guard has to cover read, write, WMI and rollback.
  for (const step of ['wmi_read_ec(addr)', 'wmi_write_ec(addr, target)', 'mulong_exec_raw', 'wmi_write_ec(addr, prev)']) {
    const at = body.indexOf(step);
    assert.ok(at > guardAt, `${step} must run while the guard is held`);
  }
  assert.ok(body.indexOf('current_silent()') > guardAt, 'readback must run while the guard is held');

  // A static (not per-instance) lock, so every caller shares it.
  assert.match(turbo, /static APPLY_LOCK: Mutex<\(\)> = Mutex::new\(\(\)\);/);
  assert.match(turbo, /Err\(poisoned\) => poisoned\.into_inner\(\)/);
});

test('apply_silent keeps the upstream WMI-only contract (no registry mirror)', () => {
  const body = fnBody(turbo, 'apply_silent');

  // PR #169 deleted the OEM-control-center registry mirror entirely: the WMI
  // channel is the only success signal, so no registry write or readback is
  // rolled back on failure.
  assert.match(body, /if !wmi_ok \{/, 'only the WMI result may gate success');
  assert.doesNotMatch(body, /write_registry_switch|read_registry_flag/, 'registry mirror must stay deleted');
  assert.doesNotMatch(body, /!wmi_ok \|\| !reg_ok/, 'registry failure must not roll back the EC bit');
  assert.doesNotMatch(body, /注册表=/, 'the user-facing error must not mention the registry mirror');
  assert.match(body, /WMI 通道不可用/, 'error text must describe the WMI failure only');
});

test('core turbo_submode copy keeps the same upstream WMI-only contract', () => {
  const core = coreTurbo;
  const body = fnBody(core, 'apply_silent');

  assert.match(body, /if !wmi_ok \{/);
  assert.doesNotMatch(body, /write_registry_switch|read_registry_flag/);
  assert.doesNotMatch(body, /!wmi_ok \|\| !reg_ok/);
  assert.match(body, /WMI 通道不可用/);
  // The support probe stays as-is so the UI entry point cannot disappear.
  assert.match(core, /\(wmi_ok \|\| nextgen, detail\)/);
});

test('command and physical paths publish through one shared event helper', () => {
  assert.match(command, /crate::system::turbo_events::publish_turbo_submode_changed/);
  assert.match(bridge, /crate::system::turbo_events::publish_turbo_submode_changed/);
  assert.doesNotMatch(command, /app\.emit\(\s*"turbo-submode-changed"/);
  assert.doesNotMatch(bridge, /emit_global\(\s*"turbo-submode-changed"/);

  // The helper fans out to the runtime sink and native wry exactly once each.
  assert.equal(events.match(/emit_global\(/g)?.length, 1);
  assert.equal(events.match(/broadcast_event\(/g)?.length, 1);
  assert.match(events, /pub const TURBO_SUBMODE_CHANGED: &str = "turbo-submode-changed";/);
});

test('a failed sub-mode register read never broadcasts silent=false', () => {
  assert.doesNotMatch(bridge, /current_silent\(\)\.unwrap_or\(false\)/);
  assert.doesNotMatch(turbo, /current_silent\(\)[\s\S]{0,80}unwrap_or/);

  const body = fnBody(bridge, 'apply_power_mode');
  const confirmedAt = body.indexOf('if confirmed');
  const publishAt = body.indexOf('publish_turbo_submode_changed');
  const warnAt = body.indexOf('"turbo-submode-changed skipped');
  assert.ok(confirmedAt >= 0 && publishAt > confirmedAt, 'physical path must wait for hardware confirmation');
  assert.ok(publishAt >= 0 && warnAt > publishAt, 'physical path must skip the event on read failure');
  assert.match(body, /match crate::system::power::turbo_submode::current_silent\(\)/);
});

test('legacy set_turbo_submode keeps its silent-only IPC shape', () => {
  assert.match(command, /pub fn set_turbo_submode\(silent: bool\) -> Result<TurboSubmodeState, String>/);
  assert.match(read('lib.rs'), /commands::power_cmd::set_turbo_submode/);
});
