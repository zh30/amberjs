# Node `os.networkInterfaces` contract

This is the user-facing contract for the Stable **`os.networkInterfaces`** carve in Amber (**G63**). It is derived from `src/nodejs_core/os.rs`, `src/runtime_minimal.rs`, and `tests/os_network_interfaces_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

The G20 host-info surface stays [`docs/OS_CONTRACT.md`](OS_CONTRACT.md). `os.hostname` is Stable under [`docs/OS_HOSTNAME_CONTRACT.md`](OS_HOSTNAME_CONTRACT.md) (**G62**), not this page. This page does **not** renumber G20 or G62. This page does **not** touch [`docs/PROCESS_CONTRACT.md`](PROCESS_CONTRACT.md).

This is not full Node `os`. Only `networkInterfaces()` as pinned below is Stable under G63.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `networkInterfaces()` | Returns a plain object whose own keys are **real host network interface names** from the OS enumerator (`getifaddrs` on POSIX via the `if-addrs` crate; `GetAdaptersAddresses` on Windows). Each value is an array of address entries for that interface. The pre-G63 hardcoded fiction that always exposed only `lo` / `lo0` with a single synthetic `127.0.0.1` entry is gone. |

Each address entry is a plain object with:

| Field | Behavior |
| :--- | :--- |
| `address` | IP address string (`Ipv4Addr` / `Ipv6Addr` display). |
| `netmask` | Netmask string from the OS. |
| `family` | `"IPv4"` or `"IPv6"` (string form). |
| `mac` | MAC address string. On Linux, read from `/sys/class/net/<name>/address` when present. When unavailable, `"00:00:00:00:00:00"`. |
| `internal` | `true` when the address is a loopback address (`127.0.0.0/8` or `::1`); otherwise `false`. |
| `cidr` | `"<address>/<prefixlen>"` where `prefixlen` is the bit count of the netmask (from `if-addrs`). |

On enumerator failure the call returns an empty object `{}` (does not throw and does not invent interfaces).

`require('os')` / `require('node:os')` / `globalThis.os` share the same function installed by `setup_os_api`.

## Limits

- MAC is best-effort: Linux sysfs when readable; otherwise the all-zero MAC string. Do not claim Node/libuv MAC parity on every host.
- Interface and address order follow the OS enumerator (not a pinned sort).
- IPv6 `scopeid` is not pinned on the entry object.

## Non-goals

These stay outside Stable. Do **not** document them as Stable Limits fiction:

- `os.hostname` — Stable under [`docs/OS_HOSTNAME_CONTRACT.md`](OS_HOSTNAME_CONTRACT.md) (**G62**); not this page
- `cpus()[].speed` / `cpus()[].times`
- `os.loadavg` as part of this carve
- Numeric `family` (`4` / `6`) or an options bag (`{ family: 4 }`)
- Inventing interfaces that the OS does not report
- Graduating the pre-G63 hardcoded `lo`/`lo0` loopback-only stub

## Reachability

Same installer as G20: `nodejs_core::os::setup_os_api` on the CLI path installs `globalThis.os`. `require('os')` / `require('node:os')` resolve that object. The contract is not feature-gated.

## Tests

```bash
cargo test --test os_network_interfaces_contract_tests -- --test-threads=1
```

CI runs that command as `node os networkInterfaces Stable contract`.
