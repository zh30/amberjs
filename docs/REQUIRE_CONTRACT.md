# CommonJS `require` contract

This is the user-facing contract for Stable CommonJS `require` and Node-compatible module resolution in the default Amber runtime. It is derived from `src/runtime_minimal.rs` (`setup_module_system`, `cjs_require_builtin_module`, `cjs_require_user_module`), `src/nodejs_core/commonjs_resolver.rs`, and `tests/require_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

The CLI path is `runtime_minimal`. The older helper in `src/nodejs_core/require.rs` is not what `amber run` / `amber eval` install.

This is not the full Node.js Module loader. Builtin module **APIs** stay on their own Stable or Preview contracts (for example `fs` is G8, `path` is G17, Node `events` / EventEmitter is G18, `buffer` is G19, `os` is G20, `zlib` sync is G21, `util`/`process` basics are G22, Node `crypto` is G23, stream is G24, `child_process` sync is G25, Node `url` fileURL helpers are G26, and `dns` getaddrinfo is provisional G27; HTTP, networking, readline, and other `src/nodejs_core/` surfaces stay Preview unless another Stable contract names them). This page graduates the loader and resolution surface only (**provisional G28**; merge last among wave-2). G24 is stream, never require.

## Reachability

`amber run` and `amber eval` install:

| Binding | Behavior |
| :--- | :--- |
| `require` | Function. Loads a builtin or a file module. |
| `module` | Object for the current entry. `module.exports` starts as the same object as `exports`. |
| `exports` | The initial exports object for the entry. |
| `__dirname` / `__filename` | Strings for the entry directory and path. Nested file modules get their own values inside the wrapper; sibling `require("./…")` uses the requiring module's directory even if script code mutates the global `__dirname`. |
| `require.main` | The entry `module` object. |
| `require.resolve` | Resolves a specifier from the current `__dirname` and returns a string (absolute file path, or the builtin name). |
| `module.createRequire` / `require('module').createRequire` | Same function. Given a filename string or a `{ href }` file URL, returns a require that resolves relative to that file's directory. The returned function has no `.resolve` and no `.main`. |

The contract is not feature-gated. Library users that construct `MinimalRuntime` get the same installer.

## Builtin loading

`require('fs')`, `require('node:fs')`, and the other names in the resolver builtin list return the object already installed on the runtime (or throw when that object is missing). `require.resolve('fs')` and `require.resolve('node:path')` return the bare names `"fs"` and `"path"`.

An unknown `node:…` or `amber:…` specifier throws `Error` whose message is `Cannot find module '…'`. There is no `error.code`.

Loading a builtin through `require` does **not** promote that module's API to Stable. Callers must use the module's own contract (or treat it as Preview).

## File loading

After resolution, a file module is loaded as follows:

| Kind | How it is chosen | What `require` returns |
| :--- | :--- | :--- |
| CommonJS | `.cjs`, or `.js` when the nearest `package.json` `"type"` is not `"module"` | `module.exports` after the wrapper runs |
| JSON | `.json` | The parsed JSON value |
| TypeScript | `.ts` | `module.exports` after oxc transpile |
| TSX / JSX | `.tsx` / `.jsx` | `module.exports` after oxc transpile |
| ES module | `.mjs`, or `.js` when nearest `"type"` is `"module"` | The ESM namespace object (named exports as properties) |

CommonJS wrappers receive `(module, exports, __dirname, __filename)` and a local `require` / `require.resolve` bound to that module directory. Assigning `module.exports = value` replaces what later `require` calls receive. Property writes on `exports` work when `module.exports` was not replaced.

The same absolute path returns the same exports object on later `require` calls (identity cache). `require.cache` is not defined.

Circular CommonJS graphs see the partial `exports` object of a module that is still evaluating. Two-way `require` between file modules completes without throwing.

`module.children` exists as an array on the entry module and stays empty. It is not a Node-compatible children list.

## Resolution

`resolve_commonjs_module(specifier, parentDir)` (and therefore `require` / `require.resolve`) applies these rules:

1. **Builtins** — bare names and `node:` / `amber:` / `wintertc:` forms that match the builtin list resolve to `Builtin(name)` without reading the filesystem.
2. **`#` imports** — package `"imports"` from the nearest enclosing `package.json`, including exact keys and `*` patterns, with the CommonJS condition set.
3. **Relative / absolute** — `./`, `../`, `/…`, and `.` / `..` as package directories. Extension probes try, in order: as written, then `.js`, `.json`, `.ts`, `.mjs`, `.cjs`, `.tsx`. A directory uses `"exports"` / `"main"` / `index.*` (not `"module"`).
4. **Bare packages** — walk parent directories for `node_modules/<name>`, including scoped names. Self-reference by package `"name"` uses that package's `"exports"`.
5. **`package.json` `"exports"`** — preferred over `"main"`. CommonJS conditions, in order: `require`, `wintercg`, `wintertc`, `node`, `amber`, `amberjs`, `default`. Arrays try targets until one resolves; a `null` entry blocks later targets. Subpath keys, `*` patterns, and blocked unexported paths use the Node-shaped reason strings below. `"module"` is ignored for `require`.
6. **Invalid / blocked targets** — export or import targets must stay inside the package. Targets without `./`, with a `node_modules` segment, or with a `.` / `..` segment after the package prefix are rejected. Mixed subpath keys and condition keys in one `"exports"` object is `ERR_INVALID_PACKAGE_CONFIG`.

### Failure messages

Unresolved relative or package paths throw `Error` whose message is `Cannot find module '<specifier>' from '<parentDir>'`.

Package configuration failures put the Node-shaped token in the message string (still no `error.code`):

| Token in `error.message` | Meaning |
| :--- | :--- |
| `ERR_PACKAGE_PATH_NOT_EXPORTED` | Subpath not listed in `"exports"` (or blocked by `null`) |
| `ERR_PACKAGE_IMPORT_NOT_DEFINED` | `#` import missing from `"imports"` |
| `ERR_INVALID_PACKAGE_TARGET` | Illegal export/import target |
| `ERR_INVALID_MODULE_SPECIFIER` | Illegal specifier (pattern capture with `..` / `node_modules`, bare `#`, …) |
| `ERR_INVALID_PACKAGE_CONFIG` | Malformed or mixed `"exports"` / unreadable `package.json` |

Under `amber run --sandbox` (and any other permission-broker deny of filesystem read), requiring a file that is not an allowed path throws `TypeError` whose message contains `permission denied` before the module body runs. The entry file itself remains readable because the CLI allows that path; a sibling `require('./…')` is denied unless `--allow-read` covers it.

## Non-goals

- Full Node `Module` class, `Module._load`, `Module._resolveFilename`, `Module.createRequire` from a separate class instance, or `require.extensions`.
- `require.cache` as a user-visible map (the runtime cache is internal).
- Populated `module.children` / `module.parent` graphs for nested loads (`module.parent` on the entry is `null`).
- `error.code === 'MODULE_NOT_FOUND'` (or any `error.code` on resolve failures).
- `createRequire(…).resolve` and `createRequire(…).main`.
- npm lifecycle, Yarn/pnpm resolution, or reading lockfiles during `require`.
- Promoting builtin module method tables to Stable by naming them here.
- Claiming webpack/Node ESM interop parity beyond “`require` of an ESM file returns that namespace object”.

## Tests

```bash
cargo test --test require_contract_tests -- --test-threads=1
```

CI runs that command as `node require Stable contract`, next to the other Stable contract steps.
