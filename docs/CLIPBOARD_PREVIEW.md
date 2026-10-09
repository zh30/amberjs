# Clipboard Preview Limits (`navigator.clipboard`)

Preview surface on the CLI path (`src/main.rs` → `src/runtime_minimal.rs` → `setup_clipboard_api` in `src/web_api/clipboard.rs`). This is **not** a Stable contract and does **not** graduate under the CURRENT_SCOPE Graduation Rule. G9–G16 and G29–G30 stay intact.

## Behavior

| Call | Behavior |
| :--- | :--- |
| `navigator.clipboard.writeText(text)` | Coerces `text` with `ToString` (missing / null / undefined → `""`). Stores the string in a **process-local** in-memory buffer. Returns a Promise that resolves to `undefined`. |
| `navigator.clipboard.readText()` | Returns a Promise that resolves to the current buffer string (`""` if never written in this process). |

Round-trip within one Amber process: after `await writeText(s)`, `await readText()` returns `s` for ordinary Unicode text.

## Limits (honest)

- **In-process only.** There is no OS / system clipboard integration. Other apps and other Amber processes do not see this buffer.
- **No secure-context theater.** CLI always allows `writeText` / `readText`; there is no Permissions Policy, user activation, or `isSecureContext` gate.
- **No `ClipboardItem`.** `navigator.clipboard.read()` and `navigator.clipboard.write(...)` reject with an Error whose message names `ClipboardItem` and points callers at `readText` / `writeText`.
- **Shared process buffer.** Isolates / runtimes in the same process share one store. A new `MinimalRuntime` does not reset prior text until something writes again.
- Errors are ordinary `Error` objects, not `DOMException` / `NotAllowedError`.

## Non-goals

- OS clipboard (Wayland / X11 / macOS pasteboard / Windows clipboard).
- `ClipboardItem`, image/HTML MIME types, or `clipboardchange`.
- Permissions API / secure-context / user-gesture requirements.
- Stable graduation (no G-number, no CURRENT_SCOPE Stable bullet, no CI Stable contract step) until a deliberate graduation PR meets the Graduation Rule.

## Tests

```bash
cargo test --test clipboard_api_tests -- --test-threads=1
```
