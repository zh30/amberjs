# Node `os.hostname` contract

This is the user-facing contract for the Stable `os.hostname` slice of Node `os` in Amber (**G62**). It is derived from `src/nodejs_core/os.rs` (`host_hostname`, `os_hostname_callback`, `setup_os_api`), `src/runtime_minimal.rs` (CLI install and ESM named export), and `tests/os_hostname_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

The G20 host-info surface stays [`docs/OS_CONTRACT.md`](OS_CONTRACT.md). This page does not replace G20. This is **not** full Node `os`.

## Stable surface

| Call / export | Behavior |
| :--- | :--- |
| `require('os')` / `require('node:os')` | Same object installed by `setup_os_api` on `globalThis.os`. |
| `os.hostname()` | Returns a string: the host OS hostname from a real OS call. On Unix (Linux / macOS) that call is `gethostname(2)`. On Windows it is `GetComputerNameW`. It does **not** read `HOSTNAME` or `COMPUTERNAME` environment variables and does not invent `"localhost"` when those vars are unset. |
| ESM `import { hostname } from 'os'` / `'node:os'` | Named export is the same function as `require('os').hostname`. |

`hostname()` takes no arguments. Extra arguments are ignored.

## Limits

These are real behaviors of the Stable surface, not cover for missing APIs:

- On syscall / Win32 failure, `hostname()` returns the empty string `""` (it does not throw and does not fall back to env vars or `"localhost"`).
- The returned string is the kernel/computer name as reported by the OS call above. It is not a DNS FQDN lookup and is not required to match `dns` results.

## Non-goals

Outside this contract (Preview / unimplemented). Do **not** treat these as Stable Limits fiction:

- `os.networkInterfaces` — still a hardcoded fake loopback (`lo` / `lo0` with `127.0.0.1` only). Must **not** graduate as Stable until real OS interface enumeration lands. Documenting that fake as a Stable Limit is forbidden.
- `cpus()[].speed` / `cpus()[].times` and every other Node `os` API not named in G20 or this page
- Reading `HOSTNAME` / `COMPUTERNAME` / other env vars as a hostname source
- DNS reverse / FQDN resolution of the host name

## Reachability

Same installer as G20: `nodejs_core::os::setup_os_api` on the CLI path installs `globalThis.os`. `require('os')` / `require('node:os')` resolve that object. ESM named `hostname` is wired in `runtime_minimal` with the G20 named exports. The contract is not feature-gated.

## Tests

```bash
cargo test --test os_hostname_contract_tests -- --test-threads=1
```

CI runs that command as `node os hostname Stable contract`.
