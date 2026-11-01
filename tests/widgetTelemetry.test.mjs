import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import ts from 'typescript';

const source = readFileSync(new URL('../src/components/widgetTelemetry.ts', import.meta.url), 'utf8');
const { outputText } = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.ESNext } });
const { subscribeWidgetTelemetry } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`);
const flush = () => new Promise((resolve) => setImmediate(resolve));

test('visible page polling uses its own interval and stops while hidden', async (t) => {
  t.mock.timers.enable({ apis: ['setInterval', 'Date'], now: 0 });
  const state = fixture();
  const stop = subscribeWidgetTelemetry(state.source, () => {}, 1500);
  t.after(stop);
  await flush();
  assert.equal(state.reads, 1);
  t.mock.timers.tick(1500);
  await flush();
  assert.equal(state.reads, 2);
  state.visibility(false);
  t.mock.timers.tick(30000);
  await flush();
  assert.equal(state.reads, 2);
});

function fixture(visible = true) {
  const state = { reads: 0, received: [], unlistened: 0, telemetry: undefined, visibility: undefined };
  state.source = {
    readSnapshot: async () => ({ cpu_temp: ++state.reads }),
    listenTelemetry: async (handler) => { state.telemetry = handler; return () => state.unlistened++; },
    listenVisibility: async (handler) => { state.visibility = handler; return () => state.unlistened++; },
    isVisible: async () => visible,
  };
  return state;
}

test('receives events without polling until the heartbeat stops', async (t) => {
  t.mock.timers.enable({ apis: ['setInterval', 'Date'], now: 0 });
  const state = fixture();
  const stop = subscribeWidgetTelemetry(state.source, (snapshot) => state.received.push(snapshot));
  t.after(stop);
  await flush();
  assert.equal(state.reads, 1);
  t.mock.timers.tick(4000);
  state.telemetry({ cpu_temp: 42 });
  t.mock.timers.tick(1000);
  await flush();
  assert.equal(state.reads, 1);
  assert.equal(state.received.at(-1).cpu_temp, 42);
  t.mock.timers.tick(5000);
  await flush();
  assert.equal(state.reads, 2);
});

test('hidden widgets stop polling and resume on reopening', async (t) => {
  t.mock.timers.enable({ apis: ['setInterval', 'Date'], now: 0 });
  const state = fixture(false);
  const stop = subscribeWidgetTelemetry(state.source, (snapshot) => state.received.push(snapshot));
  await flush();
  assert.equal(state.reads, 0);
  state.visibility(true);
  await flush();
  assert.equal(state.reads, 1);
  state.visibility(false);
  state.telemetry({ cpu_temp: 99 });
  t.mock.timers.tick(20000);
  await flush();
  assert.equal(state.reads, 1);
  assert.equal(state.received.length, 1);
  state.visibility(true);
  await flush();
  assert.equal(state.reads, 2);
  stop();
  assert.equal(state.unlistened, 2);
  t.mock.timers.tick(10000);
  assert.equal(state.reads, 2);
});

test('slow requests do not overlap or update a hidden widget', async (t) => {
  t.mock.timers.enable({ apis: ['setInterval', 'Date'], now: 0 });
  const state = fixture();
  let finish;
  state.source.readSnapshot = () => { state.reads++; return new Promise((resolve) => { finish = resolve; }); };
  const stop = subscribeWidgetTelemetry(state.source, (snapshot) => state.received.push(snapshot));
  t.after(stop);
  await flush();
  t.mock.timers.tick(15000);
  assert.equal(state.reads, 1);
  state.visibility(false);
  finish({ cpu_temp: 99 });
  await flush();
  assert.equal(state.received.length, 0);
});

test('late subscription registration is cleaned up after unmount', async () => {
  const state = fixture();
  let finish;
  state.source.listenVisibility = () => new Promise((resolve) => { finish = resolve; });
  const stop = subscribeWidgetTelemetry(state.source, () => assert.fail('Unmounted widget received a snapshot'));
  stop();
  finish(() => state.unlistened++);
  await flush();
  assert.equal(state.unlistened, 1);
  assert.equal(state.reads, 0);
});

test('a stale visibility query does not override a newer show event', async (t) => {
  t.mock.timers.enable({ apis: ['setInterval', 'Date'], now: 0 });
  const state = fixture(false);
  let finish;
  state.source.isVisible = () => new Promise((resolve) => { finish = resolve; });
  const stop = subscribeWidgetTelemetry(state.source, () => {});
  t.after(stop);
  await flush();
  state.visibility(true);
  finish(false);
  await flush();
  t.mock.timers.tick(5000);
  await flush();
  assert.equal(state.reads, 2);
});

test('a stale visibility query does not override a newer hide event', async (t) => {
  t.mock.timers.enable({ apis: ['setInterval', 'Date'], now: 0 });
  const state = fixture();
  let finish;
  state.source.isVisible = () => new Promise((resolve) => { finish = resolve; });
  const stop = subscribeWidgetTelemetry(state.source, () => assert.fail('Hidden widget received a snapshot'));
  t.after(stop);
  await flush();
  state.visibility(false);
  finish(true);
  await flush();
  t.mock.timers.tick(10000);
  await flush();
  assert.equal(state.reads, 0);
});
