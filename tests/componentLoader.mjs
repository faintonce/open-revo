import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';

export function componentLoader(mockImport, globals = { console }) {
  const modules = new Map();
  return function load(name) {
    if (modules.has(name)) return modules.get(name).exports;
    const path = new URL(`../src/components/${name}`, import.meta.url);
    const { outputText } = ts.transpileModule(readFileSync(path, 'utf8'), {
      compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.React, esModuleInterop: true },
    });
    const module = { exports: {} };
    modules.set(name, module);
    const require = (id) => mockImport(id, load);
    vm.runInNewContext(`(function(require, module, exports) { ${outputText}\n})`, globals)(require, module, module.exports);
    return module.exports;
  };
}
