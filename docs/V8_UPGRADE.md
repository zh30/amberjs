# rusty_v8 upgrade path (0.22 → 0.32 / `v8` crate)

## Current state

- Amber pins `rusty_v8 = "0.22"` in `Cargo.toml`.
- crates.io latest of the same package line is `rusty_v8 0.32.1`.
- The modern Deno-aligned binding is also published as `v8` (versioned with Chromium).

## Why this is a dedicated branch

Upgrading touches nearly every V8 callback signature, isolate create params, module
host callbacks (`HostInitializeImportMetaObjectCallback`), and snapshot APIs.
It must not interleave with Node compatibility feature work.

## Recommended steps

1. Branch `upgrade/rusty-v8-0.32`.
2. Change dependency to `rusty_v8 = "0.32"` (or migrate to `v8 = "…"` and rewrite imports).
3. Fix compile errors in waves:
   - `src/lib.rs` initialize flags / platform
   - `src/runtime_minimal.rs` isolate + module host callbacks
   - `src/nodejs_core/*` and `src/web_api/*` FunctionCallback signatures
4. Re-enable native startup snapshot via SnapshotCreator once API is available.
5. Run `cargo test` + `./tests/conformance/run_conformance.sh` before merge.

## Interim mitigation in this sprint

- Branch `upgrade/rusty-v8-0.32-prep` (worktree) bumps `rusty_v8` to 0.32 and
  uses `new_default_platform(0, false).make_shared()`. `cargo check --lib`
  succeeds there. Do **not** mix that merge with Agent sandbox / session / MCP.
- Keep 0.22 as the default compile pin on this tree for stability.
- Land warmup-artifact snapshot (`SnapshotManager::generate_snapshot`) and
  progressive `import.meta` (`global.import.meta.url` plus ESM namespace
  publish). Full `HostInitializeImportMetaObjectCallback` lands with 0.32.
- Track upgrade completion with CI once the dedicated branch is merged.
