import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import ts from 'typescript';

// Use the project's TypeScript compiler so this also runs on the release CI's Node 20.
const source = await readFile(new URL('../src/components/tuning/cpuPowerLimits.ts', import.meta.url), 'utf8');
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 },
});
const { resolveCpuPowerLimits } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`);

const current = { pl1: 75, pl2: 95, pl4: 150 };

test('raising sustained power to 165W previews all linked increases', () => {
  assert.deepEqual(resolveCpuPowerLimits(current, 'pl1', 165, 20), { pl1: 165, pl2: 165, pl4: 185 });
});

test('lowering sustained power to 15W preserves the other limits', () => {
  assert.deepEqual(resolveCpuPowerLimits(current, 'pl1', 15, 20), { pl1: 15, pl2: 95, pl4: 150 });
});

test('lowering burst power also lowers sustained power', () => {
  assert.deepEqual(resolveCpuPowerLimits(current, 'pl2', 15, 20), { pl1: 15, pl2: 15, pl4: 150 });
});

test('raising burst power preserves sustained power and reserves peak headroom', () => {
  assert.deepEqual(resolveCpuPowerLimits(current, 'pl2', 165, 20), { pl1: 75, pl2: 165, pl4: 185 });
});

test('lowering peak power preserves the configured transient buffer', () => {
  assert.deepEqual(resolveCpuPowerLimits(current, 'pl4', 35, 20), { pl1: 15, pl2: 15, pl4: 35 });
});

test('raising peak power leaves the other limits unchanged', () => {
  assert.deepEqual(resolveCpuPowerLimits(current, 'pl4', 200, 20), { pl1: 75, pl2: 95, pl4: 200 });
});

test('preview does not mutate the active profile', () => {
  const original = Object.freeze({ ...current });
  resolveCpuPowerLimits(original, 'pl1', 165, 20);
  assert.deepEqual(original, current);
});

test('Intel and AMD limits preserve ordering across low, normal and high targets', () => {
  for (const margin of [0, 10, 20, 30]) {
    for (const limit of ['pl1', 'pl2', 'pl4']) {
      for (const target of [15, 35, 65, 75, 95, 115, 165, 200]) {
        if (limit === 'pl4' && target < 15 + margin) continue;
        const next = resolveCpuPowerLimits(current, limit, target, margin);
        assert.equal(next[limit], target);
        assert.ok(next.pl1 >= 15);
        assert.ok(next.pl1 <= next.pl2);
        assert.ok(next.pl2 + margin <= next.pl4);
      }
    }
  }
});
