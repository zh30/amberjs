# URL, encoding, and structuredClone contract

This is the user-facing contract for Stable `URL`, `URLSearchParams`, `TextEncoder`, `TextDecoder`, and `structuredClone` in the default Amber runtime. It is derived from `src/web_api/url_fast.js`, `src/web_api/url.rs`, `src/web_api/url_search_params.rs`, `src/web_api/encoding.rs`, `src/web_api/structured_clone.rs`, and `tests/url_encoding_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

These globals are reachable from `amber run` / `amber eval`. `atob` and `btoa` are Stable under G29 ([`docs/ATOB_BTOA_CONTRACT.md`](ATOB_BTOA_CONTRACT.md)), not this page. `fetch`, `Blob` / `File` / `FormData`, streams, compression, timers, and `performance` stay on their own Stable contracts. Every other `src/web_api/` module stays Preview. This is not the whole URL standard, Encoding standard, or HTML structured clone algorithm.

## `URL`

`URL` parses a hierarchical URL (`scheme://...`) and resolves a relative reference against an absolute base. `new URL(input, base)` and `URL(input, base)` both construct. A failure throws `TypeError` (`Invalid URL`). `URL.canParse(input, base)` returns false instead of throwing.

An absolute URL has a scheme of at least two characters. `http`, `https`, `ws`, `wss`, `ftp`, and `file` require `://`. Any other scheme without `://` is an opaque URL: `pathname` is the scheme-specific part, `origin` is `"null"`, and host fields are empty (`mailto:user@example.com`).

Parsed fields:

| Field | Value |
| :--- | :--- |
| `href` | Serialized URL. `toString()` and `toJSON()` return it. |
| `protocol` | Lowercase scheme, including `:`. |
| `username`, `password` | Userinfo, percent-decoded. They are omitted from `origin`. |
| `hostname` | Host, ASCII-lowercased. An IPv6 address keeps its brackets (`[::1]`). |
| `port` | Decimal port with leading zeros removed. The default port is omitted (`http`/`ws` 80, `https`/`wss` 443, `ftp` 21). |
| `host` | `hostname`, plus `:` and `port` when the port is present. |
| `pathname` | Path with `.` and `..` segments removed, including `%2E` / `%2e`. `\` is a `/` in a special scheme. Space and the ASCII path set (`"`, `<`, `>`, `\`, `^`, `` ` ``, `{`, `|`, `}`, controls) are percent-encoded. |
| `search`, `hash` | Including the leading `?` or `#`, or empty. A raw space becomes `%20`. |
| `origin` | `scheme://host` for special schemes. `ws` reports an `http` origin and `wss` reports `https`. `file:` and opaque URLs report `"null"`. Userinfo is not included. |

Relative resolution against a base that itself has `://`:

- `//host/path` keeps the base scheme and replaces the host.
- A path starting with `/` replaces the base path.
- `?query` and `#hash` keep the base path. `#hash` also keeps the base query.
- Any other relative path replaces the last base segment, then dot segments are removed.
- An empty input keeps the base path and query and drops the hash.

Setting `href`, `protocol`, `username`, `password`, `host`, `hostname`, `port`, `pathname`, `search`, or `hash` updates `href`. An invalid `protocol`, `host`, `hostname`, or `port` assignment is ignored. A default port assigned through `port` or `host` is stored as `""`.

ASCII tab, newline, and carriage return are removed before parsing. Leading and trailing C0 controls and spaces are trimmed.

## `URLSearchParams`

`searchParams` is a live `URLSearchParams` for `search`. `append`, `set`, `delete`, and `sort` write back to `search` and `href`. Assigning `search` or `href` replaces that list.

Constructors accept a query string (a leading `?` is ignored), a sequence of pairs, another `URLSearchParams` (a copy), a record object, or an iterable of pairs such as a `Map`. A sequence entry that is not a pair of length 2 throws `TypeError`.

`+` in a query string is a space. `toString()` uses `application/x-www-form-urlencoded`: space is `+`, and `encodeURIComponent` encodes the rest. Pair order is insertion order. `sort()` orders names by UTF-16 code units and keeps the relative order of equal names.

`get` returns the first value or `null`. `getAll` returns every value. `set` replaces every pair with that name. `delete(name)` removes every pair with that name. `delete(name, value)` removes only matching pairs. `has(name)` and `has(name, value)` follow the same rule. An omitted or `undefined` value argument does not filter by value. `size` is the number of pairs. `forEach(callback, thisArg)` passes `(value, name, params)`. `keys`, `values`, `entries`, and iterating the object yield pairs in list order. The iterators are themselves iterable.

## `TextEncoder` and `TextDecoder`

Both constructors must be called with `new`. `instanceof` is true for those instances.

`TextEncoder` has `encoding === "utf-8"`. `encode()` and `encode(undefined)` return an empty `Uint8Array`. `encode(null)` encodes the string `"null"`. Other values are converted with `ToString` and encoded as UTF-8. A lone surrogate becomes U+FFFD (`EF BF BD`).

`encodeInto(source, destination)` writes into a `Uint8Array` and returns `{ read, written }`. `read` counts UTF-16 code units. A character that does not fit is not split. The destination must be a `Uint8Array`; anything else throws `TypeError`.

`TextDecoder` accepts the labels `utf-8`, `utf8`, and `unicode-1-1-utf-8` (case-insensitive). Any other label throws `RangeError`. The `encoding` property is `"utf-8"`. Options are `fatal` and `ignoreBOM`, both defaulting to false.

`decode(input, options)` accepts `Uint8Array`, another `ArrayBuffer` view (including a view with a byte offset), or an `ArrayBuffer`. A string throws `TypeError`. Omitted input is an empty buffer. Without `fatal`, invalid UTF-8 is replaced with U+FFFD. With `fatal: true`, invalid UTF-8 throws `TypeError` (`The encoded data was not valid UTF-8`). A leading UTF-8 BOM (`EF BB BF`) is removed unless `ignoreBOM` is true. A BOM that appears later in the stream is decoded as U+FEFF.

`{ stream: true }` keeps an incomplete trailing UTF-8 sequence for the next `decode` on that object. The following `decode` without `stream`, or `decode()` with no bytes, flushes those bytes. A `fatal` failure does not consume the new input.

## `structuredClone`

`structuredClone(value)` returns a deep copy. `structuredClone(value, { transfer })` moves each listed `ArrayBuffer` (or the buffer of a listed view) with `ArrayBuffer.prototype.transfer`. The clone receives the moved bytes. The original buffer is detached (`byteLength === 0`).

Cloned types:

- Primitives, including `bigint`, `null`, and `undefined`.
- Plain objects and arrays, including cycles. Own enumerable string keys are copied. A hole in an array stays a hole.
- `Date`, `RegExp` (source and flags), `Map`, and `Set`, including object keys and `undefined` keys. Cycles through a `Map` or `Set` are preserved.
- `Error`, `TypeError`, `RangeError`, `ReferenceError`, `SyntaxError`, `EvalError`, and `URIError`, including `message`, `name`, `stack`, and own enumerable extra fields.
- `ArrayBuffer`, `DataView`, and the typed arrays `Uint8Array`, `Int8Array`, `Uint8ClampedArray`, `Uint16Array`, `Int16Array`, `Uint32Array`, `Int32Array`, `Float32Array`, `Float64Array`, `BigInt64Array`, and `BigUint64Array`. The copy has its own buffer. A `DataView` copies the viewed bytes.
- A settled `Promise`. A fulfilled promise resolves to a clone of its value. A rejected promise rejects with an `Error` that carries the reason's message and own fields. A `Promise` nested in an object or array is cloned the same way.

These values throw `DataCloneError`:

- A `Symbol`, a function, a `WeakMap`, a `WeakSet`, or a `SharedArrayBuffer`, including when one appears inside an object, array, `Map`, or `Set`.
- An object with an own symbol key, or an own enumerable function.
- A pending `Promise`.

A plain object that happens to have `name` and `message` stays a plain object. It is not turned into an `Error`. An object with `forEach` and `get` stays a plain object. It is not turned into a `Map`.

## Limits

- Hostnames are not IDNA-encoded. A non-ASCII host is kept as Unicode and lowercased when it is not an IPv6 literal.
- `TextDecoder` decodes UTF-8 only. Other Encoding-standard labels throw `RangeError`.
- `structuredClone` does not clone platform objects that are not in the list above (`Blob`, `File`, DOM nodes, `MessagePort`). A settled `Promise` is cloned; the HTML structured clone algorithm rejects every `Promise`.

## Tests

```bash
cargo test --test url_encoding_contract_tests -- --test-threads=1
```

CI runs that command as `web url encoding Stable contract`, next to the other Stable contract steps.
