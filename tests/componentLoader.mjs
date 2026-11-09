import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath, URL } from 'node:url';
import vm from 'node:vm';
import ts from 'typescript';

/**
 * Loads TS/TSX modules for tests by transpiling and evaluating them in a VM context.
 *
 * Every import id is delegated to `mockImport`. Ids listed in `options.loadRelative` are instead
 * loaded for real through this same pipeline, so a module extracted out of a component (e.g.
 * `src/lib/*.ts`) can still be exercised as production code instead of being stubbed.
 */
export function componentLoader(mockImport, globals = { console }, options = {}) {
  const loadRelative = new Set(options.loadRelative ?? []);
  const modules = new Map();

  function resolve(fromUrl, id) {
    for (const candidate of [id, `${id}.ts`, `${id}.tsx`, `${id}/index.ts`, `${id}/index.tsx`]) {
      const url = new URL(candidate, fromUrl);
      if (existsSync(fileURLToPath(url))) return url;
    }
    throw new Error(`Cannot resolve relative import ${id} from ${fileURLToPath(fromUrl)}`);
  }

  function loadPath(path) {
    // Key by the resolved URL: two different specifiers can point at the same module, and a
    // single specifier can resolve differently depending on the importing file.
    const key = path.href;
    if (modules.has(key)) return modules.get(key).exports;
    const { outputText } = ts.transpileModule(readFileSync(fileURLToPath(path), 'utf8'), {
      compilerOptions: {
        target: ts.ScriptTarget.ES2020,
        module: ts.ModuleKind.CommonJS,
        jsx: ts.JsxEmit.React,
        esModuleInterop: true,
      },
    });
    const module = { exports: {} };
    modules.set(key, module);
    const require = (id) =>
      loadRelative.has(id) ? loadPath(resolve(path, id)) : mockImport(id, load);
    vm.runInNewContext(`(function(require, module, exports) { ${outputText}\n})`, globals)(
      require,
      module,
      module.exports,
    );
    return module.exports;
  }

  function load(name) {
    return loadPath(new URL(`../src/components/${name}`, import.meta.url));
  }

  return load;
}
