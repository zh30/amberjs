// Pins the Stable URL, encoding, and structuredClone contract in docs/URL_ENCODING_CONTRACT.md.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn run_js(code: &str) -> String {
    let mut runtime = MinimalRuntime::new().expect("runtime");
    runtime
        .execute_code(code)
        .unwrap_or_else(|error| panic!("execution failed: {error}"))
        .trim()
        .to_string()
}

#[test]
#[serial]
fn url_parses_absolute_userinfo_ipv6_and_default_port() {
    let output = run_js(
        r#"
        const absolute = new URL(" HTTPS://Example.COM:8080/a/./b/../c?q=1#h ");
        const user = new URL("https://user:secret@example.com/a");
        const ipv6 = new URL("https://[::1]:8080/a");
        const stripped = new URL("https://example.com:443/a");
        const file = new URL("file:///tmp/a");
        const opaque = new URL("mailto:user@example.com?subject=Hi");
        const space = new URL("https://example.com/a b");
        const dots = new URL("https://example.com/a/%2e%2e/b");
        const slash = new URL("https://example.com\\a\\b");
        [
            absolute.protocol,
            absolute.hostname,
            absolute.port,
            absolute.host,
            absolute.pathname,
            absolute.search,
            absolute.hash,
            absolute.origin,
            absolute.href,
            user.username + ":" + user.password + ":" + user.origin + ":" + user.href,
            ipv6.hostname + ":" + ipv6.port + ":" + ipv6.host,
            stripped.port + ":" + stripped.host + ":" + stripped.origin + ":" + stripped.href,
            file.pathname + ":" + file.origin + ":" + file.href,
            opaque.protocol + opaque.pathname + ":" + opaque.origin + ":" + opaque.href,
            space.pathname,
            dots.pathname,
            slash.pathname,
            absolute instanceof URL,
            String(absolute) === absolute.href,
            JSON.stringify(absolute)
        ].join("|");
        "#,
    );
    assert_eq!(
        output,
        "https:|example.com|8080|example.com:8080|/a/c|?q=1|#h|https://example.com:8080|https://example.com:8080/a/c?q=1#h|user:secret:https://example.com:https://user:secret@example.com/a|[::1]:8080:[::1]:8080|:example.com:https://example.com:https://example.com/a|/tmp/a:null:file:///tmp/a|mailto:user@example.com:null:mailto:user@example.com?subject=Hi|/a%20b|/b|/a/b|true|true|\"https://example.com:8080/a/c?q=1#h\"",
        "got {output}"
    );
}

#[test]
#[serial]
fn url_resolves_relative_references_and_setters() {
    let output = run_js(
        r##"
        const base = "https://example.com/a/b?x=1#h";
        const parent = new URL("../c", base);
        const here = new URL("./c", "https://example.com/a/b/");
        const abs = new URL("/c", base);
        const query = new URL("?y=2", base);
        const hash = new URL("#z", base);
        const protocolRelative = new URL("//other.example/x", base);
        const empty = new URL("", base);
        const invalid = (() => {
            try {
                new URL("/only-relative");
                return "accepted";
            } catch (error) {
                return error instanceof TypeError ? "type-error" : String(error);
            }
        })();
        const can = [
            URL.canParse("https://example.com/a"),
            URL.canParse("/a"),
            URL.canParse("/a", "https://example.com/b"),
            URL.canParse("http:foo")
        ].join(",");
        const edited = new URL("https://example.com/a/b?x=1#h");
        edited.pathname = "b/../c";
        edited.hash = "next";
        edited.port = "443";
        edited.username = "user";
        const after = edited.href;
        edited.href = "http://example.org:80/z";
        [
            parent.href,
            here.href,
            abs.href,
            query.href,
            hash.href,
            protocolRelative.href,
            empty.href,
            invalid,
            can,
            after,
            edited.href,
            edited.origin
        ].join("|");
        "##,
    );
    assert_eq!(
        output,
        "https://example.com/c|https://example.com/a/b/c|https://example.com/c|https://example.com/a/b?y=2|https://example.com/a/b?x=1#z|https://other.example/x|https://example.com/a/b?x=1|type-error|true,false,true,false|https://user@example.com/c?x=1#next|http://example.org/z|http://example.org",
        "got {output}"
    );
}

#[test]
#[serial]
fn url_search_params_round_trip_and_stay_live() {
    let output = run_js(
        r#"
        const params = new URLSearchParams("q=amber+runtime&plus=a%2Bb&q=second");
        const built = new URLSearchParams({ q: "amber runtime", plus: "a+b" });
        const sequence = new URLSearchParams([["a", "1"], ["a", "2"], ["b", "3"]]);
        const copied = new URLSearchParams(sequence);
        sequence.delete("a", "1");
        const fromMap = new URLSearchParams(new Map([["topic", "runtime"], ["space", "amber js"]]));
        let bad = "accepted";
        try {
            new URLSearchParams([["name"]]);
        } catch (error) {
            bad = error instanceof TypeError ? "type-error" : String(error);
        }
        const url = new URL("https://example.com/path?a=1#hash");
        url.searchParams.set("a", "2");
        url.searchParams.append("b", "3");
        const live = url.href;
        url.search = "?c=4";
        const seen = [];
        params.forEach(function (value, key, owner) {
            seen.push(this.label + ":" + key + "=" + value + ":" + (owner === params));
        }, { label: "ctx" });
        [
            params.get("q"),
            params.get("plus"),
            params.getAll("q").join(","),
            built.toString(),
            copied.getAll("a").join(","),
            sequence.toString(),
            sequence.has("a", "2"),
            sequence.has("a", "1"),
            sequence.size,
            fromMap.toString(),
            bad,
            live,
            url.search + ":" + url.searchParams.get("c") + ":" + url.searchParams.get("a"),
            Array.from(params.keys()).join(","),
            seen[0]
        ].join("|");
        "#,
    );
    assert_eq!(
        output,
        "amber runtime|a+b|amber runtime,second|q=amber+runtime&plus=a%2Bb|1,2|a=2&b=3|true|false|2|topic=runtime&space=amber+js|type-error|https://example.com/path?a=2&b=3#hash|?c=4:4:null|q,plus,q|ctx:q=amber runtime:true",
        "got {output}"
    );
}

#[test]
#[serial]
fn text_encoder_and_decoder_utf8_stream_and_fatal() {
    let output = run_js(
        r#"
        const encoder = new TextEncoder();
        const encoded = encoder.encode("é");
        const nul = encoder.encode(null);
        const omitted = encoder.encode();
        const explicit = encoder.encode(undefined);
        const short = new Uint8Array(1);
        const into = encoder.encodeInto("é", short);
        const decoder = new TextDecoder();
        const held = decoder.decode(new Uint8Array([0xc3]), { stream: true });
        const flushed = decoder.decode(new Uint8Array([0xa9]));
        const bom = new Uint8Array([0xef, 0xbb, 0xbf, 65]);
        const stripped = new TextDecoder().decode(bom);
        const preserved = new TextDecoder("UTF-8", { ignoreBOM: true }).decode(bom);
        let fatal = "";
        try {
            new TextDecoder("utf-8", { fatal: true }).decode(new Uint8Array([0xff]));
        } catch (error) {
            fatal = error instanceof TypeError ? "type-error" : String(error);
        }
        let label = "";
        try {
            new TextDecoder("utf-16");
        } catch (error) {
            label = error instanceof RangeError ? "range-error" : String(error);
        }
        let notNew = "";
        try {
            TextEncoder();
        } catch (error) {
            notNew = error instanceof TypeError ? "type-error" : String(error);
        }
        const view = new Uint8Array([0, 65, 66]).subarray(1);
        [
            encoder instanceof TextEncoder,
            encoder.encoding,
            encoded.length + ":" + encoded[0] + ":" + encoded[1],
            nul.length,
            omitted.length,
            explicit.length,
            into.read + ":" + into.written + ":" + short[0],
            decoder instanceof TextDecoder,
            held + ":" + flushed,
            stripped + ":" + stripped.length + ":" + preserved.charCodeAt(0) + ":" + preserved.slice(1),
            fatal,
            label,
            notNew,
            new TextDecoder().decode(view)
        ].join("|");
        "#,
    );
    assert_eq!(
        output,
        "true|utf-8|2:195:169|4|0|0|0:0:0|true|:é|A:1:65279:A|type-error|range-error|type-error|AB",
        "got {output}"
    );
}

#[test]
#[serial]
fn structured_clone_copies_claimed_types_and_transfers_array_buffer() {
    let output = run_js(
        r#"
        const date = new Date(Date.UTC(2020, 0, 2));
        const regexp = /a+/gi;
        const map = new Map([["k", 1], [undefined, "u"]]);
        const set = new Set([1, 1]);
        const bytes = new Uint8Array([4, 5]);
        const view = new DataView(new ArrayBuffer(2));
        view.setUint8(0, 9);
        const cycle = { n: 1 };
        cycle.self = cycle;
        const plain = { name: "E", message: "m", extra: 1 };
        const nested = { p: Promise.resolve(3) };
        const cloned = structuredClone({ date, regexp, map, set, bytes, view, cycle, plain, nested });
        const transferred = new Uint8Array([7, 8, 9]);
        const moved = structuredClone(transferred.buffer, { transfer: [transferred.buffer] });
        let symbolName = "";
        try {
            structuredClone(Symbol.for("g10"));
        } catch (error) {
            symbolName = error.name;
        }
        let pendingName = "";
        try {
            structuredClone(new Promise(() => {}));
        } catch (error) {
            pendingName = error.name;
        }
        const fulfilled = structuredClone(Promise.resolve({ n: 2 }));
        Promise.all([
            cloned.nested.p,
            fulfilled
        ]).then(([nestedValue, fulfilledValue]) => [
            cloned.date instanceof Date,
            cloned.date.toISOString(),
            cloned.regexp instanceof RegExp,
            cloned.regexp.source + "/" + cloned.regexp.flags,
            cloned.regexp !== regexp,
            cloned.map instanceof Map,
            cloned.map.get("k") + ":" + cloned.map.get(undefined),
            cloned.set instanceof Set,
            cloned.set.size,
            cloned.bytes instanceof Uint8Array,
            cloned.bytes[0] + ":" + (cloned.bytes.buffer !== bytes.buffer),
            cloned.view instanceof DataView,
            cloned.view.getUint8(0) + ":" + (cloned.view.buffer !== view.buffer),
            cloned.cycle.self === cloned.cycle,
            cloned.cycle !== cycle,
            cloned.plain instanceof Error,
            cloned.plain.name + ":" + cloned.plain.message + ":" + cloned.plain.extra,
            moved instanceof ArrayBuffer,
            moved.byteLength,
            new Uint8Array(moved)[0],
            transferred.buffer.byteLength,
            moved !== transferred.buffer,
            symbolName,
            pendingName,
            nestedValue,
            fulfilledValue.n
        ].join("|"));
        "#,
    );
    assert_eq!(
        output,
        "true|2020-01-02T00:00:00.000Z|true|a+/gi|true|true|1:u|true|1|true|4:true|true|9:true|true|true|false|E:m:1|true|3|7|0|true|DataCloneError|DataCloneError|3|2",
        "got {output}"
    );
}
