---
title: "Edge SLM & Constrained JSON Schema Decoding (bee:ai)"
subtitle: "Native zero-dependency local text generation, streaming token synthesis, and guaranteed JSON schema conformance"
group: "Agent & Advanced"
id: "slm-inference"
---

## 1. Overview & Architecture

Autonomous agents require fast, deterministic local token generation and guaranteed structural conformance for function calling, tool use, and JSON payloads.

While traditional cloud API calls suffer from network latency, rate limits, and JSON hallucination, **Beejs v1.4.0 expands `bee:ai`** with native Edge Small Language Model (SLM) generation and constrained JSON decoding:

### Core Capabilities
- **Zero-Dependency Autoregressive Generation**: Run token synthesis directly in-process without Python or heavy external frameworks.
- **Constrained JSON Schema Decoding**: Guarantees 100% syntactically valid JSON output complying with provided JSON schemas.
- **Async Streaming Protocol**: `generateStream` implements standard `AsyncIterableIterator<string>` for real-time token streaming.
- **Seamless Integration with `LLM` & `AgentPipeline`**: Plug directly into existing agent orchestration layers.

---

## 2. Text Generation & Token Streaming

Import `generate` and `generateStream` from `bee:ai`:

```typescript
import { generate, generateStream } from 'bee:ai';

// 1. Synchronous or Asynchronous Full Text Generation
const result = await generate("Explain how Beejs provides native execution speed", {
  maxTokens: 64,
  temperature: 0.7,
});

console.log('Generated text:', result.text);
console.log('Tokens used:', result.tokens);
console.log('Finish reason:', result.finishReason);

// 2. Real-time Token Streaming
console.log('Streaming tokens:');
for await (const chunk of generateStream("Stream tokens for real-time chat UI")) {
  process.stdout.write(chunk);
}
console.log('\nStream completed.');
```

---

## 3. Constrained JSON Schema Decoding

Guarantee that the generated output strictly obeys your data model:

```typescript
import { generate } from 'bee:ai';

// Define expected JSON Schema
const schema = {
  type: "object",
  properties: {
    tool: { type: "string" },
    action: { type: "string" },
    confidence: { type: "number" },
    dryRun: { type: "boolean" },
  },
  required: ["tool", "action", "confidence"]
};

// Generate response with schema enforcement
const res = await generate("Call file search tool for package.json", {
  schema: schema,
  responseFormat: "json_object"
});

// Guaranteed to be valid JSON matching your schema
const payload = JSON.parse(res.text);
console.log('Valid structured payload:', payload);
// { tool: 'execute_command', action: '...', confidence: 42, dryRun: true }
```

---

## 4. Using with the `LLM` Class

The `LLM` class in `bee:ai` natively adopts the generator backend:

```typescript
import { LLM } from 'bee:ai';

const model = new LLM("bee-slm-0.5b");

// Single-call generation
const response = await model.generate("Summarize system telemetry");

// Streaming generation
for await (const token of model.generateStream("Step-by-step reasoning")) {
  process.stdout.write(token);
}
```
