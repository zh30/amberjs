# Node `dns` contract

This is the user-facing contract for the Stable Node `dns` surface in Amber. It is derived from `src/nodejs_core/dns.rs`, `src/runtime_minimal.rs`, and `tests/dns_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('dns')` and `require('node:dns')` reach the same object installed by `setup_dns_api` on `globalThis.dns`.

This is not full Node `dns`. Stable methods use host OS lookups (`ToSocketAddrs` / `getnameinfo`) and the system resolver list — not a DNS wire client and not invented defaults.

**Numbering:** getaddrinfo (`lookup` / `resolve` / `resolve4` / `resolve6`) is **G27**. Real `reverse` / `getServers` are additive **tentative G65**. Do not renumber G1–G64.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `lookup(hostname[, options|callback][, callback])` | Resolves `hostname` with the OS address lookup. Blocks the isolate. The return value is the first address string after sort+dedup, or an error string. When a function callback is present (as the second or third argument), it is invoked on the **same turn** as `(err, address, family)`. Success: `err` is `null`, `address` is that first string, `family` is the number `4` (always, including when the address is IPv6). Failure: `err` is a string (not an `Error` object) and there is no `address`. An empty hostname yields `"Error: hostname is required"`. A lookup failure yields a string starting with `"Error: dns.lookup "`. |
| `resolve(hostname[, rrtype])` | Same OS lookup. Returns a string array of address literals (IPv4 and/or IPv6). The `rrtype` argument is accepted and ignored; there is no record-type-specific resolution. On failure returns an error string starting with `"Error: dns.resolve "` (does not throw). Empty hostname returns `"Error: hostname is required"`. No callback form. |
| `resolve4(hostname)` | Same OS lookup filtered to IPv4 addresses only. Returns a string array (possibly empty when the host has only IPv6). Failure and empty-hostname strings match `resolve` with the `dns.resolve4` prefix. No callback form. |
| `resolve6(hostname)` | Same OS lookup filtered to IPv6 addresses only. Returns a string array (possibly empty when the host has only IPv4). Failure and empty-hostname strings match `resolve` with the `dns.resolve6` prefix. No callback form. |
| `reverse(ip)` (**G65**) | Real reverse DNS via OS `getnameinfo` with `NI_NAMEREQD`. On success returns a **string array** of hostnames (typically one). The hostname is never the IP string itself (the old Echo-IP Preview stub is gone). Empty input returns `"Error: IP address is required"`. Invalid IP or lookup failure returns a string starting with `"Error: dns.reverse "` (does not throw). No callback form. Sync on the calling turn. |
| `getServers()` (**G65**) | Returns a string array of configured DNS server addresses from the host: Unix reads `nameserver` lines from `/etc/resolv.conf`; Windows uses `GetNetworkParams`. Never invents `8.8.8.8` or any other default. The array may be empty when the OS has no configured servers. Sync on the calling turn. |

## Permission broker

Each Stable call that performs network-facing lookup I/O (`lookup`, `resolve`, `resolve4`, `resolve6`, `reverse`) checks the global Network/Connect permission for the hostname (or IP string) before I/O. When denied, the call does **not** throw: it returns a permission-denial string, and `lookup` also passes that string as the callback `err` when a callback is present. `getServers` only reads local resolver configuration and does not perform a Network/Connect check.

## Limits

These are real behaviors of the Stable surface (G27 getaddrinfo + G65 `reverse` / `getServers`), not cover for missing APIs:

- **Sync / same-turn.** Every Stable call completes on the calling turn and may block the isolate. There is no libuv-style threadpool and no later-turn deferral. `lookup`'s callback (when present) runs on that same turn.
- **String errors, not Node `Error`.** Failures and permission denials return (or pass to `lookup`'s callback) plain strings such as `"Error: dns.reverse …"` / `"Error: dns.lookup …"`. There are no `Error` objects with `code` fields such as `ENOTFOUND` / `EAI_AGAIN`.
- **`lookup` family is always `4`.** The third callback argument stays the number `4` even when `address` is IPv6.
- **`resolve` `rrtype` is ignored.** Address lists come from the OS getaddrinfo-style lookup only; there is no record-type-specific path.
- **`reverse` is one OS `getnameinfo(NI_NAMEREQD)`.** Success is a string array (typically length 1). Results are OS- and resolver-dependent. If the OS returns no name, or only the IP string itself, the call fails with `"Error: dns.reverse …"` (the old Echo-IP Preview stub is not Stable behavior). Empty / invalid IP and lookup failures return error strings (do not throw). No callback form. Sandbox Network/Connect deny returns a permission-denial string without performing I/O.
- **`getServers` reads the host resolver list only.** Unix: `nameserver` lines from `/etc/resolv.conf`. Windows: `GetNetworkParams`. The array may be empty when the file is missing, unreadable, or the OS reports no servers. Never invents `8.8.8.8` or any other default. Sync on the calling turn; no Network/Connect check.

## Non-goals

Outside this contract (Preview / unimplemented). Do **not** document these as Stable Limits fiction:

- `dns.promises`, `dns.Resolver`, `setServers`, `lookupService`, `resolveMx` / `resolveTxt` / `resolveCname` / `resolveNs` / `resolveSrv` / `resolveAny` / `resolvePtr`, and TTL/`all` options.
- Multi-RR PTR enumeration / `/etc/hosts` alias lists beyond what a single `getnameinfo(NI_NAMEREQD)` returns.
- Graduating Echo-IP `reverse` or fake `8.8.8.8` `getServers` as Stable (forbidden honesty rule — those stubs are gone and must not reappear under Limits).

## Reachability

The CLI `amber` binary installs this object from `src/runtime_minimal.rs`. Library users reach it through `amberjs::nodejs_core::dns::setup_dns_api` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.
