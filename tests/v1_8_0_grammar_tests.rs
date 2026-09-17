use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_grammar_partial_json_repair() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const { parsePartialJSON } = require('amber:grammar');

    // 1. Unclosed object & string
    const r1 = parsePartialJSON('{"name": "Alice", "status": "work');
    if (r1.name !== 'Alice' || r1.status !== 'work') {
        throw new Error('Unclosed string repair failed: ' + JSON.stringify(r1));
    }

    // 2. Unclosed nested arrays and trailing comma
    const r2 = parsePartialJSON('{"items": [1, 2, {"nested": [true,');
    if (!r2.items || r2.items[0] !== 1 || !Array.isArray(r2.items[2].nested)) {
        throw new Error('Nested array repair failed: ' + JSON.stringify(r2));
    }

    // 3. Trailing key colon
    const r3 = parsePartialJSON('{"ready": true, "pending":');
    if (r3.ready !== true || r3.pending !== null) {
        throw new Error('Trailing colon repair failed: ' + JSON.stringify(r3));
    }

    JSON.stringify({ success: true, r1, r2, r3 });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_grammar_incremental_stream_decoder() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const { createStreamDecoder } = require('amber:grammar');

    const snapshots = [];
    const decoder = createStreamDecoder({
        onChunk: (parsed, isComplete) => {
            snapshots.push({ parsed, isComplete });
        }
    });

    decoder.push('{"agent": "coder", ');
    if (!decoder.current || decoder.current.agent !== 'coder') {
        throw new Error('Snapshot 1 failed: ' + JSON.stringify(decoder.current));
    }

    decoder.push('"plan": ["read code", ');
    if (!decoder.current.plan || decoder.current.plan[0] !== 'read code') {
        throw new Error('Snapshot 2 failed: ' + JSON.stringify(decoder.current));
    }

    decoder.push('"run tests"], "status": "ok"}');
    decoder.finish();

    if (decoder.current.plan.length !== 2 || decoder.current.status !== 'ok') {
        throw new Error('Finished decode mismatch: ' + JSON.stringify(decoder.current));
    }

    if (snapshots.length !== 4) { // 3 pushes + 1 finish
        throw new Error('Snapshot count mismatch: ' + snapshots.length);
    }

    JSON.stringify({ success: true, count: snapshots.length, final: decoder.current });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_grammar_sse_chunk_parser() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const { parseSSEChunk } = require('amber:grammar');

    const sseRaw = `
event: delta
id: chunk-1
data: {"token": "The capital"}

event: delta
id: chunk-2
data: {"token": " of France"}

event: complete
id: chunk-3
data: {"token": " is Paris."}
`;

    const events = parseSSEChunk(sseRaw);
    if (events.length !== 3) {
        throw new Error('SSE events count mismatch: ' + events.length);
    }

    if (events[0].event !== 'delta' || events[0].id !== 'chunk-1') {
        throw new Error('SSE event header mismatch: ' + JSON.stringify(events[0]));
    }

    const payload0 = events[0].json();
    if (payload0.token !== 'The capital') {
        throw new Error('SSE JSON extraction mismatch: ' + JSON.stringify(payload0));
    }

    const fullText = events.map(e => e.json().token).join('');
    if (fullText !== 'The capital of France is Paris.') {
        throw new Error('Full streamed text mismatch: ' + fullText);
    }

    JSON.stringify({ success: true, fullText });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_grammar_constrained_token_acceptance() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const { createChoiceGrammar, createRegexGrammar, createJSONGrammar } = require('amber:grammar');

    // 1. Choice grammar
    const choice = createChoiceGrammar(['approve', 'reject', 'defer']);
    if (!choice.accept('', 'app')) throw new Error('Choice accept prefix failed');
    if (!choice.accept('app', 'rove')) throw new Error('Choice accept continuation failed');
    if (choice.accept('app', 'xyz')) throw new Error('Choice should reject illegal token');

    const val = choice.validate('approve');
    if (!val.valid || !val.completed) throw new Error('Choice validation failed: ' + JSON.stringify(val));

    // 2. Regex grammar
    const regex = createRegexGrammar(/^[A-Z]{2}\d{3}$/);
    if (!regex.validate('AB123').valid) throw new Error('Regex validation failed');
    if (regex.validate('ab123').valid) throw new Error('Regex should reject lowercase');

    // 3. JSON grammar
    const jsonG = createJSONGrammar();
    if (!jsonG.accept('{"a":', '1}')) throw new Error('JSON grammar accept failed');

    JSON.stringify({ success: true });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}
