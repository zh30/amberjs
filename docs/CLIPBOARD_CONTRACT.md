# Web clipboard contract (`navigator.clipboard.writeText` / `readText`)

This is the user-facing contract for Stable `navigator.clipboard.writeText` and `navigator.clipboard.readText` in the default Amber runtime. It is derived from `src/web_api/clipboard.rs` (`setup_clipboard_api`) and `tests/clipboard_api_tests.rs`. Historical `docs/STAGE_*` reports and [`docs/CLIPBOARD_PREVIEW.md`](CLIPBOARD_PREVIEW.md) are not part of this contract; the Preview page only points here.

These methods are reachable from `amber run` / `amber eval` through `src/runtime_minimal.rs`. This is not the browser Clipboard API, not the OS clipboard, and not a secure-context / Permissions Policy surface.

**Numbering:** This contract is **G31** after `ErrorEvent` / `onerror` (**provisional G30**, [`ERROR_EVENT_CONTRACT.md`](ERROR_EVENT_CONTRACT.md)). Do not renumber G9–G16, G8 / G17–G28, G29, or G30.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `navigator.clipboard.writeText(text)` | Coerces `text` with `ToString` (missing / `null` / `undefined` → `""`). Stores the string in a **process-local** `Mutex<String>` buffer. Returns a Promise that resolves to `undefined`. |
| `navigator.clipboard.readText()` | Returns a Promise that resolves to the current buffer string (`""` if never written in this process). |

Round-trip within one Amber process: after `await writeText(s)`, `await readText()` returns `s` for ordinary Unicode text (including newlines and emoji).

`typeof navigator.clipboard === "object"`, and `typeof navigator.clipboard.writeText` / `readText` are `"function"`.

## Limits

These limits are part of the Stable contract:

- **Process-local Mutex text only.** There is no OS / system clipboard integration. Other apps and other Amber processes do not see this buffer. Isolates / runtimes in the same process share one store. A new `MinimalRuntime` does not reset prior text until something writes again.
- **No secure-context theater.** CLI always allows `writeText` / `readText`; there is no Permissions Policy, user activation, or `isSecureContext` gate.
- **No `ClipboardItem`.** `navigator.clipboard.read()` and `navigator.clipboard.write(...)` reject with an ordinary `Error` whose message names `ClipboardItem` and points callers at `readText` / `writeText`. Rich / MIME / OS clipboard is outside this contract (DEFER), not a Stable stub product.
- Errors from the text path and from ClipboardItem rejection are ordinary `Error` objects, not `DOMException` / `NotAllowedError`.

## Non-goals

- OS clipboard (Wayland / X11 / macOS pasteboard / Windows clipboard).
- `ClipboardItem`, image/HTML MIME types, or `clipboardchange`.
- Permissions API / secure-context / user-gesture requirements.
- Graduating Cache, service-worker fetch intercept, DOMParser, Background Sync, Push, Notification, or Payment in this contract.

## Tests

```bash
cargo test --test clipboard_api_tests -- --test-threads=1
```

CI runs that command as `web clipboard Stable contract`, next to the other Stable contract steps.
