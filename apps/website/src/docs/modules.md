---
title: "Modules, Package Management & Testing"
subtitle: "Dual-cache module resolution, seamless npm interoperability, and Jest-compatible built-in test runner"
group: "Developer Guide"
id: "modules"
---

## 1. Dual-Cache Module Resolution (4.6M ops/s)

In large-scale microservice applications, hundreds of modules are evaluated during startup.

### Traditional Resolution Inefficiencies
When executing `require()` or `import`, traditional Node.js engines perform repeated, expensive syscalls:
1. Crawling parent directory hierarchies to locate `node_modules`.
2. Parsing `package.json` for `exports` and `main` entries.
3. Probing multiple extensions (`.js`, `.json`, `.node`).
4. Making dozens of `stat` or `access` calls per module, degrading cold start times.

### Beejs Stat-Bypass Architecture
Beejs utilizes a **two-tier in-memory normalized cache**:

```text
         require('lodash') or import ... from './utils'
                            │
                            ▼
      +─────────────────────────────────────────────+
      |  L1: Normalized Path Cache                  |
      |  - Instant hash lookup for resolved paths   |
      +─────────────────────────────────────────────+
                            │ Cache Miss
                            ▼
      +─────────────────────────────────────────────+
      |  L2: Resolved Specifier Cache               |
      |  - Avoids re-parsing package.json exports   |
      |  - Bypasses filesystem stat syscalls        |
      +─────────────────────────────────────────────+
```

In official benchmarks, Beejs achieves **4,601,226 ops/s** in module resolution—**4.1x faster than Node.js** (1.12M ops/s) and ahead of Bun (3.88M ops/s).

---

## 2. ESM & CommonJS Interoperability

Beejs natively supports seamless interop between ECMAScript Modules (ESM) and CommonJS (CJS):

```typescript
// 1. Standard ESM imports
import { readFileSync } from 'node:fs';
import { Tensor } from 'bee:ai';

// 2. CommonJS require alongside ESM
const path = require('node:path');

// 3. Dynamic import expressions
if (process.env.LOAD_OPTIONAL) {
  const mod = await import('./optional-module.js');
  mod.init();
}

// 4. Module metadata
console.log('Module URL:', import.meta.url);
console.log('Directory name:', __dirname);
console.log('File name:', __filename);
```

### Module Scheme Prefixes
- **`node:*`**: Explicitly imports Node.js compatible core modules (recommended).
- **`bee:*`**: Imports Beejs native built-ins (e.g. `bee:ai` for tensors and inference).
- **Relative / Absolute paths**: `./`, `../`, `/` for local disk modules with automatic `.ts` and `.tsx` extension resolution.

---

## 3. Built-In Package Management

Beejs includes lightweight package management compatible with the npm registry, requiring no separate `npm` or `pnpm` installation:

```bash
# 1. Initialize a new project with package.json
bee init my-app

# 2. Add production dependency
bee add lodash@4.17.21

# 3. Add development dependency
bee add --dev @types/node

# 4. Install dependencies in CI with strict integrity
bee install --frozen-lockfile

# 5. Remove unused dependencies
bee prune
```

---

## 4. Built-in Test Framework (`bee test`)

Beejs provides a zero-dependency test runner compatible with **Jest and Vitest** conventions:

### Writing Tests
Create a test file such as `math.test.ts`:

```typescript
// math.test.ts
import { describe, it, test, expect } from 'bee:test';

describe('Arithmetic & Logic', () => {
  it('adds numbers correctly', () => {
    expect(1 + 1).toBe(2);
    expect([1, 2, 3]).toHaveLength(3);
    expect({ name: 'beejs' }).toEqual({ name: 'beejs' });
  });

  test('handles async resolutions', async () => {
    const data = await Promise.resolve('ready');
    expect(data).toBe('ready');
  });

  test('asserts thrown errors', () => {
    expect(() => {
      throw new Error('Invalid input');
    }).toThrow('Invalid input');
  });
});
```

### Running Tests
```bash
# Run all test files (*.test.js, *.test.ts, *.spec.ts)
$ bee test

# Filter tests by matching pattern
$ bee test -t "async"

# Run tests in parallel across workers
$ bee test --parallel

# Terminate immediately on first failure
$ bee test --bail

# Watch mode
$ bee test -w
```

Execution report:
```text
 PASS  tests/math.test.ts (12 ms)
  Arithmetic & Logic
    ✓ adds numbers correctly (1 ms)
    ✓ handles async resolutions (2 ms)
    ✓ asserts thrown errors (0 ms)

Test Suites: 1 passed, 1 total
Tests:       3 passed, 3 total
Snapshots:   0 total
Time:        0.018s
```
