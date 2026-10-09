# Clipboard Preview → Stable

`navigator.clipboard.writeText` / `readText` graduated to **Stable (G31)**.

The user-facing contract, Limits, and Non-goals live in [`docs/CLIPBOARD_CONTRACT.md`](CLIPBOARD_CONTRACT.md), pinned by `tests/clipboard_api_tests.rs` and the CI step `web clipboard Stable contract`.

`ClipboardItem` (`navigator.clipboard.read` / `write`) stays outside Stable: those calls reject and are Non-goals / DEFER, not a separate Preview product to graduate here.
