// Amber Streaming Structured JSON & LLM Token Grammar Engine (amber:grammar)
// Real-time incremental partial JSON repair, SSE parser, and constrained token grammars.

use rusty_v8 as v8;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SSEMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<u64>,
}

/// Repairs an incomplete/partial JSON string so it can be parsed into a valid JSON object.
/// Handles unclosed strings, trailing commas, open arrays `[` and open objects `{`.
pub fn repair_partial_json(input: &str) -> String {
    let text = input.trim();
    if text.is_empty() {
        return "null".to_string();
    }

    // If it's already valid JSON, return directly
    if serde_json::from_str::<serde_json::Value>(text).is_ok() {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len() + 16);
    let mut in_string = false;
    let mut escape = false;
    let mut stack = Vec::new(); // tracks open '{' and '['

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];

        if in_string {
            result.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        match ch {
            '"' => {
                in_string = true;
                result.push(ch);
            }
            '{' => {
                stack.push('}');
                result.push(ch);
            }
            '[' => {
                stack.push(']');
                result.push(ch);
            }
            '}' => {
                if let Some(&expected) = stack.last() {
                    if expected == '}' {
                        stack.pop();
                    }
                }
                result.push(ch);
            }
            ']' => {
                if let Some(&expected) = stack.last() {
                    if expected == ']' {
                        stack.pop();
                    }
                }
                result.push(ch);
            }
            _ => {
                result.push(ch);
            }
        }
        i += 1;
    }

    // 1. If inside an unclosed string, close the quote
    if in_string {
        if escape {
            result.pop(); // drop trailing solitary backslash
        }
        result.push('"');
    }

    // 2. Clean up trailing separators like trailing commas or uncompleted key-value colons
    let mut trimmed = result.trim_end().to_string();

    loop {
        if trimmed.ends_with(',') {
            trimmed.pop();
            trimmed = trimmed.trim_end().to_string();
        } else if trimmed.ends_with(':') {
            // E.g. {"key": -> supply null
            trimmed.push_str(" null");
            break;
        } else {
            break;
        }
    }

    // 3. Balance unclosed braces and brackets in reverse order
    while let Some(closing) = stack.pop() {
        trimmed.push(closing);
    }

    // 4. Verify if repaired JSON is valid; if not, fallback to object or array if started with them
    if serde_json::from_str::<serde_json::Value>(&trimmed).is_ok() {
        trimmed
    } else if text.starts_with('{') {
        "{}".to_string()
    } else if text.starts_with('[') {
        "[]".to_string()
    } else {
        serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
    }
}

/// Parses a raw Server-Sent Events (SSE) chunk into structured SSE messages.
pub fn parse_sse_chunk(chunk: &str) -> Vec<SSEMessage> {
    let mut messages = Vec::new();
    let mut current_event: Option<String> = None;
    let mut current_data_lines = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_retry: Option<u64> = None;

    for line in chunk.lines() {
        let line = line.trim_end_matches('\r');

        if line.is_empty() {
            // Empty line indicates dispatch of event
            if !current_data_lines.is_empty() || current_event.is_some() {
                messages.push(SSEMessage {
                    event: current_event.take(),
                    data: current_data_lines.join("\n"),
                    id: current_id.take(),
                    retry: current_retry.take(),
                });
                current_data_lines.clear();
            }
            continue;
        }

        if line.starts_with(':') {
            // SSE comment/ping line, ignore
            continue;
        }

        if let Some((field, value)) = line.split_once(':') {
            let val = value.strip_prefix(' ').unwrap_or(value);
            match field {
                "event" => current_event = Some(val.to_string()),
                "data" => current_data_lines.push(val.to_string()),
                "id" => current_id = Some(val.to_string()),
                "retry" => {
                    if let Ok(ms) = val.parse::<u64>() {
                        current_retry = Some(ms);
                    }
                }
                _ => {}
            }
        } else {
            // Field without colon
            if line == "data" {
                current_data_lines.push(String::new());
            }
        }
    }

    // If chunk ended without final empty line, dispatch remaining
    if !current_data_lines.is_empty() || current_event.is_some() {
        messages.push(SSEMessage {
            event: current_event,
            data: current_data_lines.join("\n"),
            id: current_id,
            retry: current_retry,
        });
    }

    messages
}

fn grammar_native_dispatch(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    if args.length() < 1 {
        return;
    }

    let action = args.get(0).to_rust_string_lossy(scope);

    match action.as_str() {
        "repair_partial_json" => {
            let input = args.get(1).to_rust_string_lossy(scope);
            let repaired = repair_partial_json(&input);
            let s = v8::String::new(scope, &repaired).unwrap();
            rv.set(s.into());
        }
        "parse_sse" => {
            let input = args.get(1).to_rust_string_lossy(scope);
            let events = parse_sse_chunk(&input);
            let json = serde_json::to_string(&events).unwrap_or_else(|_| "[]".to_string());
            let s = v8::String::new(scope, &json).unwrap();
            rv.set(s.into());
        }
        _ => {
            let msg =
                v8::String::new(scope, &format!("Unknown grammar action '{action}'")).unwrap();
            let exc = v8::Exception::type_error(scope, msg);
            scope.throw_exception(exc);
        }
    }
}

/// Sets up the `amber:grammar` API in V8 context
pub fn setup_grammar_api(
    scope: &mut v8::PinScope,
    context: &v8::Local<v8::Context>,
) -> anyhow::Result<()> {
    let global = context.global(scope);

    // Register native dispatcher callback
    let native_fn = v8::Function::new(scope, grammar_native_dispatch).unwrap();
    let k_native = v8::String::new(scope, "__amber_grammar_native").unwrap();
    global.set(scope, k_native.into(), native_fn.into());

    let grammar_js_bootstrap = r#"
    (function() {
        const native = globalThis.__amber_grammar_native;

        // --- 1. Partial JSON Parsing ---
        function parsePartialJSON(input) {
            if (typeof input !== 'string') {
                throw new TypeError('parsePartialJSON expects a string input');
            }
            const repaired = native('repair_partial_json', input);
            try {
                return JSON.parse(repaired);
            } catch (err) {
                return null;
            }
        }

        // --- 2. Stream Decoder ---
        function createStreamDecoder(options = {}) {
            let buffer = '';
            let lastParsed = null;
            const onChunk = options.onChunk || null;

            return {
                push(chunk) {
                    if (typeof chunk === 'string') {
                        buffer += chunk;
                    }
                    const parsed = parsePartialJSON(buffer);
                    lastParsed = parsed;
                    if (typeof onChunk === 'function') {
                        onChunk(parsed, false);
                    }
                    return parsed;
                },
                finish() {
                    const parsed = parsePartialJSON(buffer);
                    lastParsed = parsed;
                    if (typeof onChunk === 'function') {
                        onChunk(parsed, true);
                    }
                    return parsed;
                },
                get current() {
                    return lastParsed;
                },
                get raw() {
                    return buffer;
                },
                reset() {
                    buffer = '';
                    lastParsed = null;
                }
            };
        }

        // --- 3. Server-Sent Events (SSE) Parser ---
        function parseSSEChunk(chunk) {
            if (typeof chunk !== 'string') {
                throw new TypeError('parseSSEChunk expects a string chunk');
            }
            const raw = native('parse_sse', chunk);
            const list = JSON.parse(raw);
            return list.map(item => ({
                event: item.event || 'message',
                data: item.data,
                id: item.id,
                retry: item.retry,
                json() {
                    try {
                        return JSON.parse(item.data);
                    } catch {
                        return parsePartialJSON(item.data);
                    }
                }
            }));
        }

        // --- 4. Constrained Grammars ---
        class Grammar {
            #type;
            #validator;
            #acceptor;

            constructor(type, { validate, accept }) {
                this.#type = type;
                this.#validator = validate;
                this.#acceptor = accept;
            }

            get type() {
                return this.#type;
            }

            validate(text) {
                return this.#validator(String(text));
            }

            accept(prefix, nextToken) {
                if (typeof this.#acceptor === 'function') {
                    return this.#acceptor(String(prefix), String(nextToken));
                }
                const candidate = String(prefix) + String(nextToken);
                const res = this.validate(candidate);
                return res.valid;
            }
        }

        function createChoiceGrammar(choices) {
            if (!Array.isArray(choices) || choices.length === 0) {
                throw new TypeError('createChoiceGrammar requires a non-empty array of strings');
            }
            const choiceList = choices.map(String);

            return new Grammar('choices', {
                validate(text) {
                    const completed = choiceList.includes(text);
                    const valid = completed || choiceList.some(c => c.startsWith(text));
                    return { valid, completed, choices: choiceList };
                },
                accept(prefix, nextToken) {
                    const candidate = prefix + nextToken;
                    return choiceList.some(c => c.startsWith(candidate) || candidate.startsWith(c));
                }
            });
        }

        function createRegexGrammar(pattern) {
            const regex = typeof pattern === 'string' ? new RegExp(pattern) : pattern;

            return new Grammar('regex', {
                validate(text) {
                    const valid = regex.test(text);
                    return { valid, completed: valid, pattern: regex.source };
                },
                accept(prefix, nextToken) {
                    const candidate = prefix + nextToken;
                    return regex.test(candidate);
                }
            });
        }

        function createJSONGrammar(schema = null) {
            return new Grammar('json', {
                validate(text) {
                    const parsed = parsePartialJSON(text);
                    const isComplete = Boolean(parsed && (text.trim().endsWith('}') || text.trim().endsWith(']')));
                    return {
                        valid: parsed !== null,
                        completed: isComplete,
                        parsed
                    };
                },
                accept(prefix, nextToken) {
                    const candidate = prefix + nextToken;
                    return parsePartialJSON(candidate) !== null;
                }
            });
        }

        function createGrammar(spec) {
            if (!spec || typeof spec !== 'object') {
                throw new TypeError('createGrammar requires a grammar specification object');
            }
            if (spec.choices) return createChoiceGrammar(spec.choices);
            if (spec.regex || spec.pattern) return createRegexGrammar(spec.regex || spec.pattern);
            if (spec.schema || spec.type === 'json') return createJSONGrammar(spec.schema);
            throw new TypeError("Grammar specification must have 'choices', 'regex', or 'schema'");
        }

        const grammarModule = {
            parsePartialJSON,
            createStreamDecoder,
            parseSSEChunk,
            Grammar,
            createGrammar,
            createChoiceGrammar,
            createRegexGrammar,
            createJSONGrammar,
            default: {
                parsePartialJSON,
                createStreamDecoder,
                parseSSEChunk,
                createGrammar,
                createChoiceGrammar,
                createRegexGrammar,
                createJSONGrammar
            }
        };

        globalThis.__amber_grammar = grammarModule;
        globalThis.grammar = grammarModule;
    })();
    "#;

    if let Some(code) = v8::String::new(scope, grammar_js_bootstrap) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }

    Ok(())
}
