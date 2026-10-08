# Events, abort, and channel contract

This is the user-facing contract for Stable `Event` / `EventTarget` / `CustomEvent`, the `AbortController` / `AbortSignal` surface beyond fetch, `MessageChannel` / `MessagePort`, and `BroadcastChannel`. It is derived from `src/web_api/events.rs`, `src/web_api/custom_event.rs`, `src/web_api/abort.rs`, `src/web_api/message_channel.rs`, `src/web_api/broadcast_channel.rs`, and `tests/events_channels_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

These constructors are on `globalThis` from `amber run` / `amber eval`. Fetch's use of an aborted signal is still the contract in [`FETCH_CONTRACT.md`](FETCH_CONTRACT.md). `ExtendableEvent`, service workers, and the rest of `src/web_api/` stay Preview. This is not the DOM or HTML standard, and it is not a browser document.

## `Event` and `EventTarget`

`new Event(type, init)` and `new EventTarget()` are required. A missing `type`, or a call without `new`, throws `TypeError`.

`init` reads `bubbles`, `cancelable`, and `composed`. Each defaults to false. The instance is `instanceof Event`.

| Field or method | Behavior |
| :--- | :--- |
| `type`, `bubbles`, `cancelable`, `composed` | Taken from the constructor arguments. |
| `isTrusted` | `false`. |
| `timeStamp` | A number from `performance.now()` when that function exists. |
| `eventPhase` | `0` outside dispatch. `2` (`Event.AT_TARGET`) while listeners run. |
| `target` | The `EventTarget` passed to `dispatchEvent`, kept after dispatch. |
| `currentTarget` | That same target while listeners run, then `null`. |
| `defaultPrevented` | Becomes `true` only when `preventDefault()` runs on a cancelable event that is not in a `{ passive: true }` listener. |
| `preventDefault()` | Sets `defaultPrevented` when `cancelable` is true and the current listener is not passive. |
| `stopImmediatePropagation()` | Skips listeners that have not run yet on this target. |
| `stopPropagation()` | Records the stop. There is no parent node, so later listeners on this same target still run. |
| `composedPath()` | `[currentTarget]` while the event is dispatching, otherwise `[]`. |
| `Event.NONE` / `CAPTURING_PHASE` / `AT_TARGET` / `BUBBLING_PHASE` | `0`, `1`, `2`, `3`, on both the constructor and `Event.prototype`. |

`dispatchEvent(event)` returns `false` when `defaultPrevented` is true, otherwise `true`. Dispatching an event that is already dispatching throws `InvalidStateError`. A non-object argument throws `TypeError`. A listener that throws does not skip the remaining listeners, and that exception does not propagate out of `dispatchEvent`.

`addEventListener(type, listener, options)`:

- `type` is converted with `ToString`.
- `listener` is a function or an object with `handleEvent`. Anything else is ignored.
- The same listener and capture flag are stored once.
- `{ once: true }` removes the listener before it is called, so it runs once.
- `{ capture: true }` runs before non-capture listeners. Both run at the target. There is no tree to bubble or capture through.
- A boolean third argument is the capture flag.
- `{ passive: true }` makes `preventDefault()` a no-op for that call.
- `{ signal }` skips registration when the signal is already aborted, and removes the listener when that signal later aborts.
- `removeEventListener` matches the listener and the capture flag.
- Listeners added during dispatch do not run for that dispatch. A listener removed before its turn does not run.

`class extends EventTarget` sees `addEventListener`, `removeEventListener`, and `dispatchEvent` on the prototype.

## `CustomEvent`

`new CustomEvent(type, init)` is `instanceof CustomEvent` and `instanceof Event`. `detail` is `init.detail`, or `null` when `detail` is missing or `undefined`. `bubbles`, `cancelable`, and `composed` use the same defaults as `Event`. A missing `type` throws `TypeError`. `detail` is the value that was passed, not a clone.

## `AbortController` and `AbortSignal`

`new AbortController()` has a `signal`. `signal instanceof AbortSignal` is true. `new AbortSignal()` throws `TypeError` (`Illegal constructor`).

| Member | Behavior |
| :--- | :--- |
| `signal.aborted` | `false` until the first successful `abort`. |
| `controller.abort(reason)` | Idempotent. The first call sets `aborted`, stores `reason`, and runs listeners. A later call does not change `reason` or run listeners again. A missing or `undefined` reason is a `DOMException` named `AbortError` whose message is `The operation was aborted`. |
| `signal.reason` | `undefined` before abort, then the reason above. |
| `signal.onabort` | Called with the abort event when it is a function. |
| `addEventListener('abort', listener)` | Stores a function or `{ handleEvent }` listener. The event is an `Event` with `type === "abort"` and `target` set to the signal. A listener added when `aborted` is already true runs before `addEventListener` returns. `{ once: true }` is honored. Other event types are ignored. |
| `removeEventListener('abort', listener)` | Removes that listener. |
| `throwIfAborted()` | Throws `signal.reason` when aborted. |
| `AbortSignal.abort(reason)` | Returns an already-aborted signal. A missing reason uses the `AbortError` above. |
| `AbortSignal.any(signals)` | `signals` must be iterable. A non-signal element throws `TypeError`. If one signal is already aborted, the result is aborted with that signal's `reason`. Otherwise the result aborts when the first source aborts, with that source's `reason`. |
| `AbortSignal.timeout(milliseconds)` | `milliseconds` must be a finite number `>= 0`, otherwise `TypeError`. The timer stays referenced, so the runtime keeps running until it fires. The reason is a `DOMException` named `TimeoutError` with message `The operation was aborted due to timeout`. |

`controller.abort()` still flips the fetch flag stored on the signal. An aborted signal passed to `fetch` follows [`FETCH_CONTRACT.md`](FETCH_CONTRACT.md).

## `MessageChannel` and `MessagePort`

`new MessageChannel()` has `port1` and `port2`. Each port is `instanceof MessagePort`. `new MessagePort()` throws `TypeError`.

`postMessage(value, transfer)` structured-clones `value`. The clone is delivered on the same turn when the peer has been started, otherwise it is queued until `start()`. Assigning `onmessage` calls `start()`. `addEventListener('message', listener)` does not.

The message event has `type === "message"`, `data`, `origin === ""`, `lastEventId === ""`, and `ports`. `onmessage` runs, then `addEventListener` listeners. `removeEventListener` removes the matching function.

`transfer` is an array:

- An `ArrayBuffer`, or a view's `.buffer`, is detached after the clone is made. The clone keeps the bytes. A detached buffer throws `DataCloneError`.
- A `MessagePort` in the list is disentangled from its current object and re-entangled on a new port delivered in `event.ports` (and as `event.data` when the message itself is that port). `addEventListener` listeners move with it. `onmessage` does not. Transferring the sending port or its peer throws `DataCloneError`.

`close()` disentangles both ends. `postMessage` on a closed or transferred port throws `InvalidStateError`. An uncloneable value throws `DataCloneError` and is not delivered. `start()` is idempotent.

Messages are delivered on the same turn. They are not queued as HTML tasks.

## `BroadcastChannel`

`new BroadcastChannel(name)` requires `name`. `name` is `ToString(name)`. `instanceof BroadcastChannel` is true.

`postMessage(value)` structured-clones `value` and delivers it, on the same turn, to every other open channel in this isolate with the same `name`. The sender does not receive its own message. Each peer gets its own clone. `onmessage` and `addEventListener('message')` both run. `removeEventListener` removes the matching function. The event uses the same `type`, `data`, `origin`, and `lastEventId` fields as a port message.

`close()` stops delivery. A later `postMessage` throws `InvalidStateError`. An uncloneable value throws `DataCloneError` and is not delivered. Different names do not see each other. Channels do not cross processes or isolates.

## Non-goals

- A document tree, bubbling through parents, or `EventTarget` on DOM nodes.
- `ExtendableEvent` and service-worker lifetime events.
- Cancelling a blocked `fetch` connect. That limit stays in the fetch contract.
- Cross-isolate or cross-process `BroadcastChannel`.
- `MessagePort` transfer into a `Worker`. Workers stay Preview.
- HTML task scheduling for channel messages. Delivery is same-turn.

## Tests

```bash
cargo test --test events_channels_contract_tests -- --test-threads=1
```

CI runs that command as `web events channels Stable contract`, next to the other Stable contract steps.
