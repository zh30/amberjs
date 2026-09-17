---
title: "Streaming Structured Output & Token Grammar Engine (bee:grammar)"
subtitle: "Incremental partial JSON auto-repair, SSE stream transformer, and constrained token generation"
group: "Agent & Advanced"
id: "streaming-grammar"
---

When interacting with Large Language Models (LLMs) and local SLMs, responses arrive token-by-token over streams (such as Server-Sent Events / SSE). Standard JSON parsers like `JSON.parse` immediately throw a `SyntaxError` when fed incomplete JSON chunks, forcing applications to wait until the entire generation completes before parsing results, updating user interfaces, or triggering downstream tools.

**Beejs v1.8.0 introduces the Native Streaming Structured Output & Token Grammar Engine (`bee:grammar`)**. It provides real-time partial JSON repair, incremental stream decoding, SSE chunk parsing, and grammar validation for constrained generation.

---

## 1. Incremental Partial JSON Auto-Repair

The `parsePartialJSON` function automatically detects unclosed strings, trailing commas, open arrays `[` and open objects `{`, balancing and closing them on the fly to return the valid partial JavaScript object:

```typescript
import { parsePartialJSON } from 'bee:grammar';

// Incomplete JSON chunk from LLM streaming
const incompleteChunk = '{"status": "running", "tags": ["agent", "work';

// Automatically balances quote and array: {"status": "running", "tags": ["agent", "work"]}
const parsed = parsePartialJSON(incompleteChunk);

console.log(parsed.status); // "running"
console.log(parsed.tags);   // ["agent", "work"]
```

---

## 2. Incremental Stream Decoder

The `createStreamDecoder` utility accumulates text chunks and emits updated parsed snapshots in real time:

```typescript
import { createStreamDecoder } from 'bee:grammar';

const decoder = createStreamDecoder({
  onChunk: (snapshot, isComplete) => {
    console.log(`Updated snapshot (complete: ${isComplete}):`, snapshot);
  }
});

decoder.push('{"plan": ["gather requirements", ');
decoder.push('"write code", ');
decoder.push('"run tests"]}');
decoder.finish();
```

---

## 3. Server-Sent Events (SSE) Parser

Parse streaming chunks from OpenAI, Anthropic, or Ollama endpoints directly:

```typescript
import { parseSSEChunk } from 'bee:grammar';

const rawSSE = `
event: delta
id: 1
data: {"token": "Hello"}

event: delta
id: 2
data: {"token": " world!"}
`;

const events = parseSSEChunk(rawSSE);
for (const ev of events) {
  console.log(ev.event, ev.json().token);
}
```

---

## 4. Constrained Token Grammars

Ensure local models and agents adhere to choice lists, regular expressions, or JSON schemas:

```typescript
import { createChoiceGrammar, createRegexGrammar } from 'bee:grammar';

// Choice grammar
const decisionGrammar = createChoiceGrammar(['ACCEPT', 'REJECT', 'NEED_MORE_INFO']);
console.log(decisionGrammar.accept('', 'ACC')); // true
console.log(decisionGrammar.accept('ACC', 'EPT')); // true
console.log(decisionGrammar.accept('ACC', 'XYZ')); // false

// Regex grammar
const ticketGrammar = createRegexGrammar(/^BUG-\d{4}$/);
console.log(ticketGrammar.validate('BUG-1024').valid); // true
```
