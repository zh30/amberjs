# Node `readline` contract

This is the user-facing contract for a **narrow** Stable subset of Node `readline` in Amber: **`createInterface` + `Interface.question` line I/O on host stdin/stdout**. It is derived from `src/nodejs_core/readline.rs` (`setup_readline_api`, `interface_question_callback`), `src/runtime_minimal.rs` (CLI install + CJS `require('readline')`), and `tests/readline_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

This is **not** full Node `readline`. Completer, history navigation, keypress events, async iterators, and `readline.promises` stay outside this page.

**Provisional numbering:** **tentative G46** after inventory READY paperwork (G38–G40), plain `amber serve` (G41), `require('timers')` (G42), and other NEEDS_IMPL slices ahead of readline rank 6. Tip Stable max at authorship was **G37**. Do not renumber G1–G37.

## Stable surface

| Call / field | Behavior |
| :--- | :--- |
| `require('readline')` / `require('node:readline')` | Same object installed by `setup_readline_api` on `globalThis.readline`. |
| `readline.createInterface(options)` | Returns an Interface object. `options` may include Node-shaped `input` / `output` / `terminal` / `historySize` / `completer` fields for API shape; **line I/O for `question` uses the host process stdin and stdout**, not custom stream byte sources. |
| `rl.question(query, callback)` | Coerces `query` with `ToString`, writes those bytes to **host stdout** (no automatic trailing newline beyond what `query` already contains), then **blocks the isolate** while reading one UTF-8 line from **host stdin**. When a complete line is available (or EOF after some bytes without a terminator), strips one trailing `\n` or `\r\n` (and leftover `\r`) and invokes `callback(answer)` on the **same turn** with that string. A bare newline yields `""`. **EOF with no bytes** invokes `callback(null)`. Fewer than two arguments throws `TypeError` (`question requires 2 arguments: query and callback`). A non-function second argument is ignored (no throw). |
| `rl.close()` | Present as a function; calling it does not throw. Closing semantics (emit `'close'`, cancel a pending read) are **not** pinned here. |

## Limits

These are real behaviors of the Stable surface, not cover for missing APIs:

- `question` **blocks** the V8 isolate until a line (or EOF) arrives. There is no libuv-style async read or event-loop deferral for the Stable path.
- Custom `input` / `output` stream objects are **not** used as byte sources/sinks. Host stdin/stdout are always used for Stable `question` I/O.
- `process.stdin.read()` remaining a null-stub elsewhere does not affect this path; `question` reads through Rust `std::io::stdin`, not `process.stdin.read`.
- Interactive TTY editing (cursor motion, history recall, tab completion) is not provided.

## Non-goals

Outside this contract (Preview / unimplemented). Do **not** treat these as Stable Limits of dishonest stubs:

- Immediate `callback("")` without reading stdin (removed; never reintroduce as Stable fiction).
- `pause` / `resume` / `prompt` / `write` / `clearLine` product behavior (may exist as no-ops).
- `'line'` / `'close'` events driven by input, completer invocation, history fill/`historySize`, `emitKeypressEvents`, `Symbol.asyncIterator`, `readline.promises`, cursor helpers (`cursorTo` / `moveCursor` / `clearScreenDown`).
- Reading from arbitrary Node `Readable` streams or writing to arbitrary `Writable` streams.
- Full Node `Interface` EventEmitter parity.

## Reachability

The CLI `amber` binary installs `globalThis.readline` from `src/runtime_minimal.rs` via `nodejs_core::readline::setup_readline_api`. `require('readline')` / `require('node:readline')` return that object through the CommonJS builtin arm. Library users reach the installer through `amberjs::nodejs_core::readline::setup_readline_api`. The contract is not feature-gated.

## Tests

```bash
cargo test --test readline_contract_tests -- --test-threads=1
```
