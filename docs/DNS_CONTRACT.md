# Node `dns` contract

This is the user-facing contract for the Stable getaddrinfo subset of Node `dns` in Amber. It is derived from `src/nodejs_core/dns.rs`, `src/runtime_minimal.rs`, and `tests/dns_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('dns')` and `require('node:dns')` reach the same object installed by `setup_dns_api` on `globalThis.dns`.

This is not full Node `dns`. Only `lookup`, `resolve`, `resolve4`, and `resolve6` are Stable. They use the host `getaddrinfo` path (`ToSocketAddrs`), not a DNS wire client. Methods that are not listed here are outside the contract, including the Preview stubs `reverse` and `getServers`.

**Provisional numbering:** This contract is intended as **G27** after url **G26** (stream **G24**, child_process **G25**). CommonJS `require` stays last as **G28**. Do not assign G24 to require or G28 to dns.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `lookup(hostname[, options|callback][, callback])` | Resolves `hostname` with the OS address lookup. Blocks the isolate. The return value is the first address string after sort+dedup, or an error string. When a function callback is present (as the second or third argument), it is invoked on the **same turn** as `(err, address, family)`. Success: `err` is `null`, `address` is that first string, `family` is the number `4` (always, including when the address is IPv6). Failure: `err` is a string (not an `Error` object) and there is no `address`. An empty hostname yields `"Error: hostname is required"`. A lookup failure yields a string starting with `"Error: dns.lookup "`. |
| `resolve(hostname[, rrtype])` | Same OS lookup. Returns a string array of address literals (IPv4 and/or IPv6). The `rrtype` argument is accepted and ignored; there is no record-type-specific resolution. On failure returns an error string starting with `"Error: dns.resolve "` (does not throw). Empty hostname returns `"Error: hostname is required"`. No callback form. |
| `resolve4(hostname)` | Same OS lookup filtered to IPv4 addresses only. Returns a string array (possibly empty when the host has only IPv6). Failure and empty-hostname strings match `resolve` with the `dns.resolve4` prefix. No callback form. |
| `resolve6(hostname)` | Same OS lookup filtered to IPv6 addresses only. Returns a string array (possibly empty when the host has only IPv4). Failure and empty-hostname strings match `resolve` with the `dns.resolve6` prefix. No callback form. |

## Permission broker

Each Stable call checks the global Network/Connect permission for the hostname (or IP string) before lookup I/O. When denied, the call does **not** throw: it returns a permission-denial string, and `lookup` also passes that string as the callback `err` when a callback is present.

## Non-goals

- `dns.reverse` and `dns.getServers` (Preview stubs; not this contract).
- `dns.promises`, `dns.Resolver`, `setServers`, `lookupService`, `resolveMx` / `resolveTxt` / `resolveCname` / `resolveNs` / `resolveSrv` / `resolveAny` / `resolvePtr`, and TTL/`all` options.
- Node-shaped `Error` objects with `code` such as `ENOTFOUND` / `EAI_AGAIN`.
- Async deferral to a later turn or a threadpool. Every Stable call completes on the calling turn.
- A real `family` of `6` for IPv6 results from `lookup` (the third callback argument stays `4`).
- Record-type fidelity for `resolve`'s `rrtype`.

## Reachability

The CLI `amber` binary installs this object from `src/runtime_minimal.rs`. Library users reach it through `amberjs::nodejs_core::dns::setup_dns_api` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.
