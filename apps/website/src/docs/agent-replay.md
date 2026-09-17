---
title: "Deterministic Agent Replay Engine (bee:replay)"
subtitle: "Offline trace capture, step-level time-travel debugging, and automated divergence detection"
group: "Agent & Advanced"
id: "agent-replay"
---

Autonomous AI Agents depend heavily on external tools, model inference, dynamic timers, and stochastic sampling. When an Agent makes an unpredicted decision or encounters a runtime error, recreating the exact state in production has traditionally been nearly impossible.

**Beejs v1.6.0 introduces the Deterministic Agent Replay Engine (`bee:replay`)** and dedicated CLI commands (`bee record` / `bee replay`). It captures non-deterministic external events, serializes execution into compact `.bee-trace.json` files, and re-executes them offline with zero external network or model dependency.

---

## 1. Core Architecture

The replay engine operates in three distinct lifecycle phases:

```
[ Normal Execution ]
         |
         v (bee record or replay.startRecording)
[ Trace Capture & Serialization ]
  - Agent step inputs & outputs
  - Virtual timestamps & RNG seeds
  - File reads & network mocks
         |
         v (Outputs: agent_run.bee-trace.json)
[ Offline Replay & Verification ] (bee replay --verify)
  - Intercepts step() calls
  - Injects recorded outputs without API calls
  - Compares inputs and detects divergence
```

### Key Replay Engine Invariants
- **Offline Self-Sufficiency**: Replaying a trace requires no active LLM API keys, database credentials, or network interfaces.
- **Automated Divergence Detection**: If the Agent's code logic has changed and produces different arguments during a step, the engine throws an immediate `Step divergence detected` error.
- **Native CLI Integration**: Seamless one-line recording and playback through `bee record` and `bee replay`.

---

## 2. Using the CLI

### 2.1 Recording an Agent Execution Trace

Execute any script or agent pipeline while recording non-deterministic inputs into a `.bee-trace.json` trace file:

```bash
# Record script execution to default trace file (agent.ts.bee-trace.json)
$ bee record agent.ts

# Specify a custom trace file path
$ bee record -o traces/search_task.bee-trace.json agent.ts --query "Quantum Computing"
```

### 2.2 Offline Replay and Verification

Re-execute the recorded session offline:

```bash
# Replay with verified deterministic execution
$ bee replay traces/search_task.bee-trace.json

# Enable strict step divergence verification and verbose logging
$ bee replay --verify -v traces/search_task.bee-trace.json
```

---

## 3. Programmatic API (`bee:replay`)

You can also control recording and step tracking directly in JavaScript/TypeScript:

```typescript
import { startRecording, stopRecording, step, isRecording, isReplaying } from 'bee:replay';

// Start recording explicitly
startRecording({ script: 'agent_search.ts', outputPath: 'search.trace.json' });

// Mark an agent decision or tool step
const userQuery = "Explain general relativity";
const plan = step("plan_generation", userQuery, (q) => {
  // During normal run / recording: executed normally
  // During replay: skipped, recorded output returned instantly
  return { steps: ["Fetch theory", "Summarize math", "Review"] };
});

const searchResult = step("web_search", { query: plan.steps[0] }, async (input) => {
  return await fetchExternalSearch(input.query);
});

// Finalize and serialize the trace
const trace = stopRecording('search.trace.json');
console.log(`Recorded ${trace.events.length} events successfully.`);
```

### 3.1 Step Divergence Handling

If your prompt or code changes between runs:

```typescript
// If the replayed script passes a different query than recorded:
try {
  step("plan_generation", "Explain quantum mechanics");
} catch (err) {
  // Throws: "Step divergence detected at index 0 ('plan_generation'): input mismatch"
}
```

---

## 4. API Reference

| Function / Method | Return Type | Description |
| :--- | :--- | :--- |
| `startRecording(opts?)` | `boolean` | Initializes recording session with optional script name and output path |
| `stopRecording(path?)` | `Trace` | Stops recording, flushes trace to disk, and returns the JSON trace object |
| `loadTrace(pathOrObj)` | `boolean` | Loads a trace file or object and arms the engine in `Replaying` mode |
| `step<T>(name, input, fn?)` | `T` | Evaluates function in record mode, or replays recorded output in replay mode |
| `isRecording()` | `boolean` | Returns `true` if trace recording is currently active |
| `isReplaying()` | `boolean` | Returns `true` if offline playback is currently active |
| `getTraceStats()` | `TraceStats` | Returns metadata including event counts, duration, and engine mode |
| `reset()` | `boolean` | Resets the replay engine to idle state |
