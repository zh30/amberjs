---
title: "Testing, Coverage & Benchmarking (bee test & bee bench)"
subtitle: "Built-in Jest-compatible test runner, LCOV coverage reporting, microbenchmark suite & CPU flamecharts"
group: "Developer Tooling"
id: "testing-benchmarking"
---

In production and performance-sensitive systems, quality tests, code coverage metrics, and microbenchmarks are essential. Beejs provides native tools without needing Jest, Vitest, or C8.

---

## 1. Automated Testing & Code Coverage (`bee test --coverage`)

Beejs features a native test runner supporting Jest and Vitest syntax.

### 1.1 Running Tests

```bash
# Run all *.test.ts and *.spec.js files
$ bee test

# Run a specific test file
$ bee test tests/auth.test.ts
```

### 1.2 Collecting Code Coverage (`--coverage`)
Add the `--coverage` flag to track line-level coverage. Beejs outputs an aligned summary table to the console and generates a standard `coverage/lcov.info` file (ready for Codecov, Coveralls, or CI):

```bash
$ bee test --coverage
```

Output:
```text
=============================== Coverage summary ===============================
Statements   : 96.42% ( 538/558 )
Branches     : 91.17% ( 155/170 )
Functions    : 98.24% ( 112/114 )
Lines        : 96.42% ( 538/558 )
================================================================================
📄 LCOV report saved to: coverage/lcov.info
```

---

## 2. Language-Level Microbenchmarks (`bee bench`)

To measure the impact of algorithm tweaks, parsing, or math operations, use `bee bench`.

### 2.1 Writing a Benchmark File

Create `*.bench.ts` or `*.bench.js` files:

```typescript
// math.bench.ts
bench("JSON.parse small payload", () => {
  JSON.parse('{"id": 1, "name": "beejs", "ok": true}');
});

bench("Array sort 1000 items", () => {
  const arr = Array.from({ length: 1000 }, (_, i) => 1000 - i);
  arr.sort((a, b) => a - b);
});
```

### 2.2 Running Benchmarks

```bash
$ bee bench
# Or specify a directory pattern
$ bee bench benchmarks/
```

Console output:
```text
🚀 Running benchmarks...

  Benchmark                          Iterations   Avg Time (ns)   Throughput (ops/s)
  ──────────────────────────────────────────────────────────────────────────────────
  JSON.parse small payload               10,000          412 ns        2,427,184/s
  Array sort 1000 items                   2,000       12,850 ns           77,821/s

✨ All benchmarks completed in 42.8ms.
```

---

## 3. V8 CPU Profiling (`bee profile`)

Diagnose CPU hotspots and bottlenecks with microsecond precision using `bee profile`, which exports Chrome DevTools compliant `.cpuprofile` files.

### 3.1 Capturing a Profile

```bash
# Execute and profile app.ts
$ bee profile app.ts -o app.cpuprofile
```

### 3.2 Visualizing Flamecharts in Chrome
1. Open Google Chrome or Chromium;
2. Press `F12` to open DevTools and navigate to the **Performance** or **Memory** panel;
3. Click **Load profile...** and select `app.cpuprofile`;
4. Interactively zoom into the flamechart, inspect function self-times, and trace call trees.
