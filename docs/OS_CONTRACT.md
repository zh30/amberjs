# Node `os` contract

This is the user-facing contract for the Stable subset of Node `os` in Amber. It is derived from `src/nodejs_core/os.rs`, `src/runtime_minimal.rs`, and `tests/os_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('os')`, `require('node:os')`, and `import` from `'os'` or `'node:os'` reach the same object installed by `setup_os_api` on `globalThis.os`. The default ESM export is that object. Named ESM exports used by this contract are `platform`, `arch`, `cpus`, `freemem`, `totalmem`, `uptime`, `type`, `release`, `homedir`, and `tmpdir`.

This is not full Node `os`. Only the calls in the table below are Stable. Everything else on the `os` object stays outside this contract.

The Node `fs` contract stays [`docs/FS_CONTRACT.md`](FS_CONTRACT.md). The Node `path` contract stays [`docs/PATH_CONTRACT.md`](PATH_CONTRACT.md). The Node `events` contract stays [`docs/NODE_EVENTS_CONTRACT.md`](NODE_EVENTS_CONTRACT.md). The Node `buffer` / `Buffer` contract stays [`docs/BUFFER_CONTRACT.md`](BUFFER_CONTRACT.md). The Node `zlib` sync contract stays [`docs/ZLIB_CONTRACT.md`](ZLIB_CONTRACT.md). The rest of `src/nodejs_core/` stays Preview.

## Stable surface

| Call / property | Behavior |
| :--- | :--- |
| `arch()` | Host CPU architecture string. `x86_64` → `"x64"`, `aarch64` → `"arm64"`, `x86` → `"ia32"`, otherwise the raw `std::env::consts::ARCH` value. |
| `platform()` | Host OS string. `macos` → `"darwin"`, `windows` → `"win32"`, otherwise the raw `std::env::consts::OS` value (for example `"linux"`). |
| `type()` | `"Windows_NT"` on Windows, `"Darwin"` on macOS, `"Linux"` on every other target. |
| `release()` | Non-empty string from `sys_info::os_release`. On failure the fallback is `"Darwin"`, `"Windows_NT"`, or `"Linux"` by target. |
| `uptime()` | Seconds since boot as a number ≥ 0. Unix uses `sys_info::boottime` against wall clock; Windows uses `GetTickCount64`. On failure it is `0`. |
| `cpus()` | An array whose length is `num_cpus::get()`. Each entry has a non-empty string `model`. `model` is the first `/proc/cpuinfo` `model name` on Linux, else `sysctl -n machdep.cpu.brand_string` on macOS, else `"<arch> CPU"`. Every entry shares that same model string. |
| `freemem()` / `totalmem()` | Free / total memory in bytes from `sys_info::mem_info` (`avail` / `total` × 1024). On failure either call returns `0`. When both succeed, `freemem() <= totalmem()`. |
| `homedir()` | Non-empty path from `dirs::home_dir`, or `"/home/user"` when that lookup fails. |
| `tmpdir()` | Windows: `TEMP`, else `"C:\\Windows\\Temp"`. Non-Windows: `"/tmp"`. |
| `EOL` | `"\r\n"` on Windows, `"\n"` elsewhere. |
| `constants.signals` | Fixed integer map for the POSIX names installed in `os.rs` (`SIGHUP` … `SIGUNUSED`). Values are the compile-time constants in that file. |

## Outside this contract

`hostname`, `loadavg`, `networkInterfaces`, `tmpDir`, `cpus()[].speed`, `cpus()[].times`, `constants.UV_UDP_REUSEADDR`, `constants.arch`, `constants.platform`, and every other Node `os` API are not part of this Stable surface. They may exist on the object; this contract does not pin them.

## Reachability

The CLI `amber` binary installs this object from `src/runtime_minimal.rs` via `nodejs_core::os::setup_os_api`. Library users reach it through `amberjs::nodejs_core::os::setup_os_api` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.
