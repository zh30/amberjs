# Blob, File, and FormData contract

This is the user-facing contract for Stable `Blob`, `File`, and `FormData` in the default Amber runtime. It is derived from `src/web_api/blob.rs`, `src/web_api/form_data.rs`, and `tests/blob_formdata_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`Blob`, `File`, and `FormData` are reachable from `amber run` and `amber eval`. `src/runtime_minimal.rs` installs them on the default isolate. Library users reach the same installers through `amberjs::web_api::blob::setup_blob_api` and `amberjs::web_api::form_data::setup_form_data_api` (`src/lib.rs` exports `web_api`). The contract is not feature-gated.

This is not the File API and it is not the HTML `FormData` standard. The rest of `src/web_api/` stays Preview. Web `fetch` remains the contract in [`docs/FETCH_CONTRACT.md`](FETCH_CONTRACT.md).

## `Blob`

`new Blob(parts, options)` builds a byte container.

| `parts` | Bytes |
| :--- | :--- |
| omitted, `null`, or `undefined` | Empty. |
| a string, not inside an array | UTF-8 of that string. The string is not iterated as separate characters. |
| an array | Concatenated in order. A string is UTF-8. An `ArrayBuffer` is its bytes. An `ArrayBufferView` (`Uint8Array`, including `subarray`) is the view's bytes. A `Blob` or `File` is that object's stored bytes. Any other part is skipped and does not throw. |

`options.type`, when it is a string, is stored as given. It is not lowercased and it is not checked as a MIME type. Any other `type` value, or a missing options object, stores `""`.

`size` is the byte length. `type` is the string above. Both are own data properties of the instance. Assigning `size` or `type` does not rewrite the stored bytes. `blob instanceof Blob` is true. `text`, `arrayBuffer`, and `slice` are own properties of the instance.

`text()` returns a string now. It is not a `Promise`. The string is the stored bytes decoded as UTF-8, with each invalid sequence replaced by U+FFFD.

`arrayBuffer()` returns an `ArrayBuffer` now. It is not a `Promise`. The buffer is the stored bytes, including bytes that are not valid UTF-8.

`slice(start, end, contentType)` returns a new object that is `instanceof Blob`.

- `start` defaults to `0`. `end` defaults to the byte length.
- A negative index counts from the end. Both indexes clamp to the byte length.
- When the clamped end is not past the clamped start, the result is empty.
- A string `contentType`, including `""`, is the new `type`. A non-string third argument sets `type` to `""`. An omitted third argument copies the source's current `type` property.
- `text`, `arrayBuffer`, and `slice` on the result read the sliced bytes.

## `File`

`new File(parts, name, options)` is the `Blob` constructor plus a name.

- Only an array `parts` value is read, with the same part rules as `Blob`. A bare string is not the body (`size` stays `0`).
- `name` is the second argument when it is a string, otherwise `""`.
- `options.type` follows the `Blob` rule.
- `options.lastModified`, when it is a number, is stored, including `0`. Otherwise `lastModified` is the current time in milliseconds since the Unix epoch.
- `name`, `lastModified`, `size`, and `type` are own data properties.
- `file instanceof File` and `file instanceof Blob` are true.
- `text`, `arrayBuffer`, and `slice` are the `Blob` methods.

`file.slice(...)` returns an object that is `instanceof File` and `instanceof Blob`. The slice does not copy `name` or `lastModified`.

## `FormData`

`new FormData()` ignores its arguments. There is no document, so an element is not read. `formData instanceof FormData` is true.

Own methods are `append`, `delete`, `get`, `getAll`, `has`, `set`, `entries`, `keys`, `values`, and `forEach`. `Symbol.iterator` is `entries`.

`append(name, value, filename)` stores one entry at the end. `name` is converted with `ToString`.

| `value` | What is stored |
| :--- | :--- |
| `null` or `undefined` | Nothing. `has(name)` stays false for that call. |
| string, number, boolean, or any other value that is not blob-like | `ToString` of the value. The multipart content type is `text/plain`. A third argument that is not `null` or `undefined` is the filename; otherwise there is no filename. |
| blob-like object | An object that has an `arrayBuffer` or `blobData` property. The body is `blobBytes` when that property is an `ArrayBuffer`, otherwise the UTF-8 of `blobData`, otherwise empty. `get` and the iterators see the UTF-8 text (U+FFFD replacement), not a `File`. The content type is the object's `type` when that string is non-empty, otherwise `application/octet-stream`. |

A `Uint8Array` has neither `arrayBuffer` nor `blobData`, so `append` stringifies it (`"1,2"`), the same as any other non-blob object.

Filename for a blob-like value: the third argument when it is not `null` or `undefined`; otherwise a `File` (`_isFile`) uses its non-empty `name`; otherwise a `Blob` uses `"blob"`. A `File` whose `name` is `""` and that was appended without a third argument has no filename.

`get(name)` returns the first stored string, or `null`. `getAll(name)` returns an array of those strings in insertion order, or an empty array. `has(name)` is a boolean. `delete(name)` removes every entry with that name. `set(name, value, filename)` removes every entry with that name and then appends the new entry at the end (the new entry is not left in the first old position).

`entries()`, `keys()`, and `values()` return array iterators in insertion order and keep duplicate names. `forEach(callback, thisArg)` calls `callback` as `callback(value, name, formData)` with `this` set to `thisArg`.

## Multipart `fetch` body

`fetch(input, init)` serializes `init.body` when that value is one of these `FormData` objects. The body bytes are, for each entry in order:

```text
--{boundary}\r\n
Content-Disposition: form-data; name="{name}"[; filename="{filename}"]\r\n
Content-Type: {content_type}\r\n
\r\n
{raw body bytes}\r\n
```

and then `--{boundary}--\r\n`. `{boundary}` is `----AmberFormBoundary` plus a random `u128`. `name` and `filename` are inserted as stored. Quotes and newlines in those strings are not escaped.

String entries use `Content-Type: text/plain`. Blob and File entries use the content type from the table above. A binary `Blob` is written as its raw bytes. `get` still returns the lossy text for that entry.

The request `Content-Type` header is set to `multipart/form-data; boundary={boundary}` only when the caller did not already supply a content-type header. A caller-supplied content-type is left unchanged. The body is still the multipart bytes.

## Limits

- `text()` and `arrayBuffer()` return their values on the calling turn. They are not `Promise`s.
- `size` and `type` are writable own data properties. Changing them does not change `arrayBuffer()` or `text()`. `slice` without a content type reads the current `type` property.
- `type` is not ASCII-lowercased and is not cleared when it is not a MIME type.
- Parts that are not a string, `ArrayBuffer`, `ArrayBufferView`, `Blob`, or `File` are skipped.
- `get`, `getAll`, `entries`, `values`, and `forEach` return strings for Blob and File entries.
- Multipart `name` and `filename` are not escaped.
- Entries stay in a process-wide table. Dropping the `FormData` object does not remove that slot. The slot index is a 32-bit integer.

## Non-goals

- `Blob.stream()`. It is not part of this contract.
- The `endings` option. Newlines are kept as given. There is no `native` conversion.
- `HTMLFormElement`, form submit, and reading fields from a document.
- A `File` object as the return value of `get` for a blob part.
- A `Blob` or `File` passed directly as a `fetch` or `Request` body. Those bytes are not read by this contract.
- `FormData` passed to the `Request` constructor. That path does not run the multipart serializer. Only `fetch(input, init)` with `init.body` set to `FormData` does.
- `response.blob()` from the fetch contract. That object is not this `Blob`.

## Tests

```bash
cargo test --test blob_formdata_contract_tests -- --test-threads=1
```

CI runs that command as `web blob formdata Stable contract`, next to the other Stable contract steps.
