import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

const source = readFileSync(new URL('../src/lib/powerModeAuthority.ts', import.meta.url), 'utf8');
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS },
});
const module = { exports: {} };
vm.runInNewContext(`(function(module, exports) { ${outputText}\n})`, {})(module, module.exports);
const { configIsAuthoritative } = module.exports;

test('boot config only wins when nothing newer arrived during the request', () => {
  // Steady state: capture == current, no pending selection.
  assert.equal(configIsAuthoritative(3, 3, undefined, 1000), true);
  // A telemetry snapshot landed while `get_app_config` was in flight.
  assert.equal(configIsAuthoritative(3, 4, undefined, 1000), false);
  // The user changed this value after the request started.
  assert.equal(configIsAuthoritative(3, 3, { expiry: 1500 }, 1000), false);
  // That protection has expired, so the config is authoritative again.
  assert.equal(configIsAuthoritative(3, 3, { expiry: 900 }, 1000), true);
});

test('a non-zero revision at capture does not by itself block the config', () => {
  // Regression: the previous check was `revision > 0`, which is permanently true after the
  // first poll and silently stopped `cfg.power_mode` from ever being applied.
  assert.equal(configIsAuthoritative(1, 1, undefined, 5000), true);
  assert.equal(configIsAuthoritative(0, 1, undefined, 5000), false);
});
