# Node `url` contract

This is the user-facing contract for the Stable subset of Node `require('url')` in Amber. It is derived from `src/nodejs_core/url.rs` (`create_require_url_module`, `file_url_to_path`, `path_to_file_url_href`), the CLI `require('url')` arm in `src/runtime_minimal.rs`, and `tests/node_url_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('url')`, `require('node:url')`, and `import` from `'url'` / `'node:url'` reach that same module object. The default ESM export is that object. Named ESM exports are `URL`, `URLSearchParams`, `fileURLToPath`, and `pathToFileURL`.

This is a **POSIX host** file-URL contract (Linux CI). It is not full Node `url` and not the web G10 URL contract (G10 stays separate for the globals).

**Provisional numbering:** This contract is **G26** after stream **G24** and child_process **G25**. Wave-2 continues as dns **G27** → CommonJS `require` **G28** (last). Do not assign G24 to require or G28 to dns. Independent of web G10 URL globals.

## Stable surface

| Call / field | Behavior |
| :--- | :--- |
| `URL` | Same constructor as `globalThis.URL` (Stable web **G10**). Re-exported on the Node module for Node-shaped `require('url').URL` access. Behavior of the constructor itself is the G10 contract in [`docs/URL_ENCODING_CONTRACT.md`](URL_ENCODING_CONTRACT.md). |
| `URLSearchParams` | Same constructor as `globalThis.URLSearchParams` (G10). Re-exported only. |
| `pathToFileURL(path)` | `path` must be a string. Resolves a single path with POSIX `path.resolve` semantics against `process.cwd()` (`.` / `..` collapse; absolute `/…` restarts). Returns a real `URL` instance (`instanceof URL`) built with `new URL(fileHref)`. The helper percent-encodes ASCII controls, space, `"`, `#`, `%`, `<`, `>`, `?`, `[`, `\`, `]`, `^`, `` ` ``, `{`, `|`, `}`, `~`, DEL, and non-ASCII UTF-8 bytes as `%HH`, and keeps `/` plus `A–Z a–z 0–9 -._!$&'()*+,;=:@`. The returned `href` is the G10 `URL` serialization of that `file:` URL (pinned cases include `file:///tmp/foo%20bar`, `file:///tmp/foo%23bar`, and `file:///tmp/foo%3Fx=1`). |
| `fileURLToPath(url)` | Accepts a string or a `URL` instance. Requires scheme `file` (case-insensitive). On POSIX, the host must be empty or `localhost` (case-insensitive); otherwise throws `TypeError` with `code` `ERR_INVALID_FILE_URL_HOST`. Percent-decodes the pathname and returns a POSIX path string (leading `/`). `file:///` yields `"/"`. Wrong scheme throws `TypeError` with `code` `ERR_INVALID_URL_SCHEME`. Unparseable input throws `TypeError` with `code` `ERR_INVALID_URL`. Non-string / non-`URL` throws `TypeError` with `code` `ERR_INVALID_ARG_TYPE`. |

## Limits

- POSIX host only. Windows drive-letter / UNC `file:` URLs are outside this contract.
- `pathToFileURL` does not accept non-string values (no `URL` input round-trip helper beyond string paths).
- `fileURLToPath` does not accept plain `{ href }` objects; only strings and `URL` instances.
- Because `pathToFileURL` returns a G10 `URL` instance, any future G10 pathname serialization change also changes the returned `href`. The contract tests pin the current serialization for the cases above.
- Re-exporting `URL` / `URLSearchParams` does **not** graduate additional WHATWG behavior beyond G10.

## Non-goals

These are outside this contract and must not be claimed Stable by listing them as “limits” of a fake surface:

- Legacy Node `url.parse` / `url.format` / `url.resolve` (including any global `url` object from `setup_url_api` that is not what CLI `require('url')` returns).
- `domainToASCII` / `domainToUnicode` / `urlToHttpOptions` / `URLCanParse` module helpers beyond what G10 already provides on the constructor.
- Full Node `url` parity, WHATWG URL standard completeness, or IDNA.

## Reachability

The CLI `amber` binary builds this module in `src/runtime_minimal.rs` via `nodejs_core::url::create_require_url_module` for both CommonJS `require('url')` / `require('node:url')` and the synthetic ESM builtin. Library users reach the helpers through `amberjs::nodejs_core::url`. The contract is not feature-gated.

## Relationship to G10

Web `URL` / `URLSearchParams` remain Stable under **G10** and [`docs/URL_ENCODING_CONTRACT.md`](URL_ENCODING_CONTRACT.md). This Node module contract only pins the **module shape** and the file-URL helpers. Do not treat G10 as a Node `url` graduation, and do not treat this contract as expanding G10.
