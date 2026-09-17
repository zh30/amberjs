---
title: "Agent Deterministic Sandbox & Hard Resource Quotas"
subtitle: "CPU Watchdog interrupts, V8 heap quotas, deterministic seed PRNG & frozen timestamps"
group: "Agent & Advanced"
id: "agent-sandbox"
---

When running untrusted scripts or executing autonomous AI Agent code, basic I/O sandboxing is insufficient: runaway loops like `while(true){}` can pin the host CPU at 100%, and unconstrained memory allocations can trigger system-wide OOM panics.

Beejs provides an **Agent-native deterministic sandbox and hardware-grade resource quota architecture**.

---

## 1. The Four Quota Dimensions

| Option Flag | Dimension | Underlying Mechanism |
| :--- | :--- | :--- |
| **`--timeout <MS>`** | **CPU Timeout Interrupt** | Independent Watchdog thread + `IsolateHandle::terminate_execution()` |
| **`--max-memory <MB>`** | **Physical Heap Quota** | V8 `ResourceConstraints` old-generation heap upper bound |
| **`--seed <U64>`** | **Deterministic Randomness** | Mulberry32 PRNG replaces standard `Math.random()` |
| **`--freeze-time <TS>`** | **Timestamp Freezing** | Freezes `Date.now()` to a fixed temporal anchor |

---

## 2. Usage & Technical Deep-Dive

### 2.1 CPU Watchdog Hard Interrupt (`--timeout`)
In typical JavaScript runtimes, a tight CPU-bound infinite loop completely stalls the single event loop. Beejs introduces an **external watchdog thread architecture**:

```bash
$ bee run --timeout 2000 infinite_loop.ts
```

If execution exceeds 2000 milliseconds, the watchdog thread forcibly terminates V8 execution:
```text
Error: Execution timed out after 2000ms
```

### 2.2 Physical Heap Hard Limits (`--max-memory`)
Prevents Agent scripts from leaking or allocating unbounded buffers:

```bash
# Strictly cap V8 heap at 128 MB
$ bee run --max-memory 128 mem_heavy_task.ts
```

### 2.3 Deterministic Seed PRNG (`--seed`)
Agent evaluation and benchmark replay require reproducible randomness. Supplying `--seed` guarantees deterministic outputs:

```bash
$ bee run --seed 123456789 simulation.ts
```

Across any machine or platform, `Math.random()` will yield the exact identical sequence of values.

### 2.4 Frozen Timestamps (`--freeze-time`)
Pin the global clock to avoid wall-clock drift:

```bash
# Pin to 2026-01-01 00:00:00 UTC (1767225600000 ms)
$ bee run --freeze-time 1767225600000 test_date.ts
```

`Date.now()` will permanently return the specified value.

---

## 3. Production Agent Sandbox Paradigm

Recommended invocation flags for running untrusted Agent tasks:

```bash
$ bee run \
    --sandbox \
    --timeout 3000 \
    --max-memory 256 \
    --seed 42 \
    --allow-read ./workspace \
    --allow-write ./workspace/output \
    agent_task.ts
```

**Guarantees Achieved**:
1. **Loop Immune**: Hard-terminates within 3 seconds;
2. **OOM Immune**: Bounded to 256MB heap;
3. **Deterministic Replay**: Fixed seed allows 100% exact trajectory replay;
4. **Isolated I/O**: Access restricted exclusively to `./workspace`.
