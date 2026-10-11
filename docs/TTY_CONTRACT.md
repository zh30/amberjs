# Node `tty.isatty` contract

This is the user-facing contract for the Stable `tty.isatty` slice of Node `tty` in Amber (**G49**). It is derived from `src/nodejs_core/tty.rs` (`is_terminal_fd`, `tty_isatty_callback`, `setup_tty_api`), `src/runtime_minimal.rs` (CLI install), and `tests/tty_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

This is **not** full Node `tty`. Only `isatty` is Stable on this page. `ReadStream` / `WriteStream` and the rest of the module stay outside Stable.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `require('tty')` / `require('node:tty')` | Same object installed by `setup_tty_api` on `globalThis.tty`. |
| `tty.isatty(fd)` | Returns a boolean. On Unix (Linux / macOS), when `fd` is a number ≥ `0`, the result is `libc::isatty(fd) == 1` (real OS TTY check). When `fd` is omitted, not a number, or a negative number, the result is `false`. On Windows (and other non-Unix targets), the result is always `false` today. |

`isatty` does not throw for bad `fd` values; it returns `false`.

## Limits

These are real behaviors of the Stable surface, not cover for missing APIs:

- On non-Unix hosts (including Windows), `isatty` always returns `false`. There is no Win32 console TTY probe in this contract.
- Whether a given fd is a TTY depends on the host process (pipes, files, and redirected stdio are typically not TTYs). Contract tests pin negative / non-number / regular-file fds as `false`; they do not require that stdin/stdout be TTYs.

## Non-goals

Outside this contract (Preview / unimplemented). Do **not** treat these as Stable Limits fiction:

- `tty.ReadStream` / `tty.WriteStream` as a product surface (`setRawMode` is flag-only; `columns` / `rows` are hardcoded `80`×`24`; color / window helpers are not real terminal I/O)
- `process.stdin` / `stdout` / `stderr` as full TTY streams (`isTTY` / `columns` / `rows` on stdio stay outside G22 / G44)
- Win32 console detection that would make `isatty` return `true` on Windows
- Node `tty.isatty` throwing `ERR_INVALID_ARG_TYPE` for non-number `fd` (Amber returns `false`)

## Reachability

The CLI `amber` binary installs `globalThis.tty` from `src/runtime_minimal.rs` via `nodejs_core::tty::setup_tty_api`. `require('tty')` / `require('node:tty')` return that object through the CommonJS builtin arm. Library users reach the installer through `amberjs::nodejs_core::tty::setup_tty_api`. The contract is not feature-gated.

## Tests

```bash
cargo test --test tty_contract_tests -- --test-threads=1
```

CI runs that command as `node tty Stable contract`.
