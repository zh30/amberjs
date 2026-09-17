---
title: "Standard Library (bee:std)"
subtitle: "Zero-dependency, production-grade official stdlib: dotenv, terminal CLI, filesystem, cryptography & assertions"
group: "Ecosystem"
id: "standard-library"
---

## 1. Why an Official Standard Library?

In the traditional Node.js ecosystem, even the most basic tasks—loading a `.env` file, printing colored tables to the terminal, generating UUIDs, recursively copying folders, or making deep equality assertions—require third-party npm packages like `dotenv`, `chalk`, `cli-table`, `uuid`, or `fs-extra`. This leads to bloated `node_modules`, slow cold starts, and supply chain security vulnerabilities.

Beejs introduces the **`bee:std`** official standard library: all utilities are natively implemented in Rust for maximum execution speed, safety, and zero external npm dependencies.

---

## 2. Modules & Import Conventions

You can import through the unified namespace or granular submodules:

```typescript
// Unified import
import { dotenv, cli, fs, crypto, assert } from 'bee:std';

// Granular submodule imports
import { config, parse } from 'bee:std/dotenv';
import { colors, table, ProgressBar } from 'bee:std/cli';
import { walk, copyDir } from 'bee:std/fs';
import { uuidv4, uuidv7, jwt } from 'bee:std/crypto';
import { assertEquals, assertThrows } from 'bee:std/assert';
```

---

## 3. Submodule Details

### 1. `bee:std/dotenv` Environment Config

Loads and parses `.env` files with comment filtering, quote stripping, and variable interpolation (`${VAR}`).

```typescript
import { config, parse } from 'bee:std/dotenv';

// Automatically loads .env in the current directory and injects into process.env
config();

// Or specify a custom path
config({ path: './config/.env.production' });

// Parse raw text
const envVars = parse(`
  PORT=8080
  DATABASE_URL=sqlite://${process.env.HOME}/app.db
`);
console.log('Port:', process.env.PORT);
```

---

### 2. `bee:std/cli` Terminal Styling & Interaction

Out-of-the-box ANSI coloring, Unicode box tables, and interactive terminal widgets:

```typescript
import { colors, table, ProgressBar } from 'bee:std/cli';

// 1. Text styling
console.log(colors.green(colors.bold('✔ Deployment successful!')));
console.log(colors.yellow('⚠ Warning: High memory usage detected'));

// 2. Unicode table formatter
const headers = ['Package', 'Version', 'Status'];
const rows = [
  ['beejs', 'v1.16.0', colors.green('Active')],
  ['sqlite', 'v3.45', colors.green('Active')],
];
console.log(table(headers, rows));

// 3. Progress bar indicator
const bar = new ProgressBar(100);
for (let i = 0; i <= 100; i += 20) {
  bar.update(i);
}
```

---

### 3. `bee:std/fs` High-Level Filesystem

Extends built-in filesystem operations with recursive directory walking and manipulation:

```typescript
import { walk, copyDir, emptyDir } from 'bee:std/fs';

// 1. Recursively walk directory and filter by extensions
const tsFiles = walk('./src', { extensions: ['ts', 'tsx'] });
console.log('All TypeScript files:', tsFiles);

// 2. Recursively copy directory tree
copyDir('./templates', './dist/project');

// 3. Empty directory contents while keeping the directory itself
emptyDir('./temp_cache');
```

---

### 4. `bee:std/crypto` Cryptography & Tokens

Provides UUID v4 (random), UUID v7 (time-ordered monotonic, ideal for database keys), and JWT authentication:

```typescript
import { uuidv4, uuidv7, jwt, hash } from 'bee:std/crypto';

// 1. Generate UUIDs
const id = uuidv4();
const timeOrderedId = uuidv7(); // Perfect for B-Tree index keys

// 2. JWT (HMAC-SHA256) sign and verify
const secret = 'beejs-super-secret-key';
const token = jwt.sign({ userId: 1001, role: 'admin' }, secret, { expiresIn: 3600 });
console.log('Generated JWT:', token);

// Verify token
const payload = jwt.verify(token, secret);
console.log('Decoded payload:', payload.userId, payload.role);

// 3. Fast cryptographic hashing
console.log('SHA-256:', hash('hello world', 'sha256'));
```

---

### 5. `bee:std/assert` Lightweight Assertions

Clear error reporting and deep equality checks:

```typescript
import { assert, assertEquals, assertThrows } from 'bee:std/assert';

assert(1 + 1 === 2, 'Math must hold');
assertEquals({ a: 1, b: [2, 3] }, { a: 1, b: [2, 3] });

assertThrows(() => {
  throw new Error('Expected failure');
}, 'Expected failure');
```
