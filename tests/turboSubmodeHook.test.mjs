import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import React from 'react';
import ts from 'typescript';
import { create, act } from 'react-test-renderer';

function loadHook({ invoke, listen }) {
  const path = new URL('../src/hooks/useTurboSubmode.ts', import.meta.url);
  const { outputText } = ts.transpileModule(readFileSync(path, 'utf8'), {
    compilerOptions: {
      target: ts.ScriptTarget.ES2020,
      module: ts.ModuleKind.CommonJS,
      jsx: ts.JsxEmit.React,
      esModuleInterop: true,
    },
  });
  const module = { exports: {} };
  const require = (id) => {
    if (id === 'react') return React;
    if (id === '@tauri-apps/api/core') return { invoke };
    if (id === '@tauri-apps/api/event') return { listen };
    throw new Error(`Unexpected import: ${id}`);
  };
  vm.runInNewContext(`(function(require, module, exports) { ${outputText}\n})`, { console })(
    require,
    module,
    module.exports,
  );
  return module.exports.useTurboSubmode;
}

function fixture(t, overrides = {}) {
  const requests = [];
  const listeners = new Map();
  const invoke = async (command, args) => {
    requests.push({ command, args });
    if (overrides.invoke) return overrides.invoke(command, args);
    if (command === 'get_turbo_submode_state') return { supported: true, silent: false };
    if (command === 'apply_turbo_profile') return { supported: true, silent: args.silent };
    throw new Error(`Unexpected invoke: ${command}`);
  };
  const listen = (name, callback) => {
    listeners.set(name, callback);
    return Promise.resolve(() => listeners.delete(name));
  };
  const useTurboSubmode = loadHook({ invoke, listen });

  function Harness() {
    const state = useTurboSubmode();
    return React.createElement('div', {
      supported: state.supported,
      silent: state.silent,
      busy: state.busy,
      error: state.error,
      apply: state.apply,
    });
  }

  let tree;
  act(() => { tree = create(React.createElement(Harness)); });
  t.after(() => act(() => tree.unmount()));
  return {
    requests,
    listeners,
    get node() { return tree.root.findByType('div'); },
    get props() { return this.node.props; },
    async flush() { await act(async () => {}); },
  };
}

test('turbo submode applies through one profile IPC and never sends set_power_mode', async (t) => {
  const state = fixture(t);
  await state.flush();
  assert.equal(state.props.supported, true);
  assert.equal(state.props.silent, false);

  await act(async () => { await state.props.apply(true); });

  assert.deepEqual(
    state.requests.map((request) => request.command),
    ['get_turbo_submode_state', 'apply_turbo_profile'],
  );
  assert.deepEqual({ ...state.requests[1].args }, { silent: true });
  assert.equal(state.requests.some((request) => request.command === 'set_power_mode'), false);
  assert.equal(state.props.silent, true);
  assert.equal(state.props.busy, false);
  assert.equal(state.props.error, null);
});

test('turbo submode follows applied events and refreshes immediately after physical mode 3', async (t) => {
  const state = fixture(t);
  await state.flush();

  act(() => {
    state.listeners.get('turbo-submode-changed')({ payload: { supported: true, silent: true } });
  });
  assert.equal(state.props.silent, true);

  const before = state.requests.length;
  act(() => {
    state.listeners.get('physical-mode-switched')({ payload: 3 });
  });
  await state.flush();
  assert.equal(state.requests.length, before + 1);
  assert.equal(state.requests.at(-1).command, 'get_turbo_submode_state');
});

test('turbo submode exposes apply failures and keeps the last confirmed state', async (t) => {
  const state = fixture(t, {
    invoke: async (command, args) => {
      if (command === 'get_turbo_submode_state') return { supported: true, silent: false };
      if (command === 'apply_turbo_profile') throw 'EC write failed';
      throw new Error(`Unexpected invoke: ${command} ${args}`);
    },
  });
  await state.flush();

  await act(async () => { await state.props.apply(false); });

  assert.equal(state.props.error, 'EC write failed');
  assert.equal(state.props.silent, false);
  assert.equal(state.props.busy, false);
});

test('turbo submode ignores concurrent apply requests while one is in flight', async (t) => {
  let resolveApply;
  const state = fixture(t, {
    invoke: (command) => {
      if (command === 'get_turbo_submode_state') return Promise.resolve({ supported: true, silent: false });
      if (command === 'apply_turbo_profile') {
        return new Promise((resolve) => { resolveApply = resolve; });
      }
      throw new Error(`Unexpected invoke: ${command}`);
    },
  });
  await state.flush();

  let first;
  act(() => { first = state.props.apply(true); });
  assert.equal(state.props.busy, true);
  await act(async () => { await state.props.apply(true); });
  const applies = state.requests.filter((request) => request.command === 'apply_turbo_profile');
  assert.equal(applies.length, 1);

  await act(async () => { resolveApply({ supported: true, silent: true }); await first; });
  assert.equal(state.props.busy, false);
});
