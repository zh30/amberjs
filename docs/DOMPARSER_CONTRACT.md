# Web `DOMParser` contract

This is the user-facing contract for Stable `DOMParser` in the default Amber runtime. It is derived from `src/web_api/dom_parser.rs` (`setup_dom_parser_api`) and `tests/dom_parser_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`DOMParser` is reachable from `amber run` / `amber eval` through `src/runtime_minimal.rs` as `globalThis.DOMParser`. This is a read-only HTML/XML parse tree for CLI workloads (scraper / html5ever for HTML, roxmltree for XML). It is not a live browser DOM, not a document viewer, and not HTML Living Standard `DOMParser` parity.

**Numbering:** This contract is **G32** after clipboard `writeText` / `readText` (**G31**, [`CLIPBOARD_CONTRACT.md`](CLIPBOARD_CONTRACT.md)). Do not renumber G9–G16, G8 / G17–G28, G29–G31.

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `new DOMParser()` | Constructs a parser. Calling without `new` throws `TypeError` (`DOMParser constructor must be called with new`). |
| `parser.parseFromString(string, contentType)` | Parses `string` (ToString) as a document for the given `contentType`. Returns a plain object with document-shaped fields and query methods (not a browser `Document`). |

### Supported `contentType` values

| `contentType` | Path |
| :--- | :--- |
| `text/html` | HTML via scraper; exposes `documentElement`, `head`, `body`, `title`, `children`, `contentType`, `URL` (`"about:blank"`). |
| `text/xml`, `application/xml`, `application/xhtml+xml`, `image/svg+xml` | XML via roxmltree; exposes `documentElement`, `children`, `contentType`, `URL` (`"about:blank"`). No HTML `head` / `body`. |

Missing or non-string `contentType` throws `TypeError` (`contentType is required`). An unsupported type throws `TypeError` (`unsupported contentType`).

### Document query (returned document)

| Method | Behavior |
| :--- | :--- |
| `getElementById(id)` | First element with that `id`, or `null`. Empty `id` → `null`. |
| `querySelector(selector)` | First match, or `null`. Invalid HTML selector throws `SyntaxError`. |
| `querySelectorAll(selector)` | Array of matches (not a live `NodeList`). |
| `getElementsByTagName(tag)` | Array of elements with that tag (`*` = all). |

### Element surface (returned elements)

Own string fields: `tagName`, `id`, `className`, `textContent`, `innerHTML`, `outerHTML`, plus `children` (array of element children). Methods: `getAttribute(name)` (`null` if absent), `querySelector` / `querySelectorAll` / `getElementsByTagName` scoped to that element’s outer markup.

HTML `tagName` is uppercase. XML `tagName` keeps the parsed local name (case as in the source for the root / elements).

### XML parse failure

Invalid XML does **not** throw from `parseFromString`. The returned document’s `documentElement` is a `parsererror` element whose `textContent` names the parse error (Mozilla-shaped `parsererror` wrapper string in `outerHTML`).

## Limits

These limits are part of the Stable contract:

- **Read-only parse tree.** There is no live DOM, mutation (`appendChild`, `innerHTML` setters), layout, CSSOM, events, or scripting inside the tree. Query re-parses stored source strings; objects are plain V8 objects, not `instanceof Node` / `Element` / `Document`.
- **HTML vs XML engines differ.** HTML uses scraper selectors (CSS-like). XML `querySelector` / `querySelectorAll` support only **tag**, `*`, `#id`, `tag#id`, and whitespace descendant combinators (`a b`). Attribute selectors, `:pseudo`, `>`, and comma lists throw `SyntaxError` naming the unsupported selector.
- **Not browser DOM identity.** `URL` is always `"about:blank"`. Empty HTML still exposes a `body` object. `querySelectorAll` / `getElementsByTagName` return arrays, not live collections.
- Errors from the constructor / `contentType` checks are `TypeError`. Invalid selectors are `SyntaxError`. They are not `DOMException`.

## Non-goals

- Live DOM, Shadow DOM, custom elements, or document writing (`document.write`).
- Full CSS Selectors Level 4 on XML.
- `DOMParser` as a browser embedding or JSDOM replacement.
- Graduating Background Sync, Cache, service-worker fetch intercept, Push, Notification, or Payment in this contract.

## Tests

```bash
cargo test --test dom_parser_tests -- --test-threads=1
```

CI runs that command as `web DOMParser Stable contract`, next to the other Stable contract steps.
