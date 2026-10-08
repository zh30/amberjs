# Node `path` contract

This is the user-facing contract for the Stable subset of Node `path` in Amber. It is derived from `src/nodejs_core/path.rs`, `src/runtime_minimal.rs` (`require('path')` returns the global installed by `setup_path_api`), and `tests/path_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('path')`, `require('node:path')`, and `import` from `'path'` / `'node:path'` reach that same object. The default ESM export is that object. Named ESM exports are `join`, `resolve`, `basename`, `dirname`, `extname`, and `normalize`.

This is a **POSIX** path contract for the host build used in CI (Linux). It is not full Node `path` and not a Windows path implementation.

## Stable surface

| Call / field | Behavior |
| :--- | :--- |
| `join(...parts)` | Joins non-empty string parts with `/`, then normalizes. Zero args or all-empty args yield `"."`. |
| `resolve(...parts)` | Resolves from right to left. The rightmost absolute segment (`/`…) restarts. Relative segments are anchored to `process.cwd()`. Collapses `.` and `..`. Never reports a trailing separator except for `"/"`. Empty args are ignored. Zero args yield the cwd string. |
| `normalize(path)` | Collapses `.`, `..`, and repeated `/`. Keeps a leading `/` for absolute paths. Preserves a trailing `/` when the input body had one. `""` yields `"."`. Absolute `..` cannot climb above `"/"`. |
| `dirname(path)` | Directory of the last `/`. `"/"` stays `"/"`. A path with no `/` yields `"."`. |
| `basename(path[, ext])` | Final `/`-separated component. Optional `ext` is stripped when it is a suffix of that component. `"/"` yields `""`. |
| `extname(path)` | From the last `.` in the basename when that `.` is not the first character and the basename is not `".."`. Dotfiles such as `.gitignore` yield `""`. |
| `relative(from, to)` | POSIX segment relative path using `/` and `../`. Equal paths yield `"."`. |
| `isAbsolute(path)` | `true` when `path` starts with `/`. |
| `parse(path)` | Object with string fields `root`, `dir`, `base`, `ext`, `name` for hierarchical POSIX paths (see limits for leading-dot basenames). |
| `format(obj)` | Builds a path from `dir` + `base`, or `root` + `base`, or `name` + `ext` when `base` is empty. Uses `/` between `dir` and the file part. |
| `sep` | `"/"`. |
| `delimiter` | `":"`. |
| `posix` | Object with the same method implementations as `path`, plus `sep === "/"` and `delimiter === ":"`. |
| `win32` | Object that shares the same method implementations as `path` (POSIX semantics). Only `sep === "\\"` and `delimiter === ";"` differ. |

## Limits

- This contract is the POSIX host behavior. It does not implement Windows drive letters, UNC roots, or backslash-separated semantics in the methods.
- `path.win32.join`, `resolve`, `normalize`, `dirname`, `basename`, `extname`, `relative`, `isAbsolute`, `parse`, and `format` are the POSIX implementations. Only `path.win32.sep` and `path.win32.delimiter` differ from `path` / `path.posix`.
- `parse` of a basename that starts with `.` (for example `.gitignore`) puts the leading-dot segment in `ext` and leaves `name` empty. `extname` for that basename is still `""`.
- Named ESM exports do not include `relative`, `parse`, `format`, `isAbsolute`, `sep`, `delimiter`, `posix`, or `win32`. Those remain properties on the default export / `require('path')` object.

## Non-goals

Methods and fields that are not in the Stable surface table are outside this contract. This is not Node path parity.

## Reachability

The CLI `amber` binary installs this object from `src/runtime_minimal.rs` via `nodejs_core::path::setup_path_api`. Library users reach it through `amberjs::nodejs_core::path::setup_path_api` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.
