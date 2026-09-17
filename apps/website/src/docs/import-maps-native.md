---
title: "WICG Import Maps & Native Addons"
subtitle: "Standard bare specifier module remapping and Node-API C/C++ dynamic library loading"
group: "Agent & Advanced"
id: "import-maps-native"
---

Modern JavaScript applications require standard module resolution and native extension support. Beejs provides comprehensive integration for both **WICG Import Maps** and **Node-API Native Addons (`.node`)**.

---

## 1. WICG Import Maps (`--import-map`)

### 1.1 What are Import Maps?
Import Maps is a standard WICG specification allowing runtimes and browsers to remap bare specifiers (like `"lodash"`) to specific relative paths or URLs without requiring extra bundlers.

### 1.2 Defining `import_map.json`

```json
{
  "imports": {
    "lodash": "./vendor/lodash.js",
    "components/": "./src/components/",
    "chalk": "https://esm.sh/chalk@5"
  }
}
```

### 1.3 Execution & Bundling Integration

Pass `--import-map` during run or bundle:

```bash
# Direct execution with mapped bare specifiers
$ bee run --import-map import_map.json app.ts

# Bundle using the import map
$ bee bundle app.ts -o dist/bundle.js --import-map import_map.json
```

In your application:
```typescript
// Mapped to ./vendor/lodash.js
import _ from 'lodash';
import Button from 'components/Button.tsx';
```

---

## 2. Node-API & C/C++ Native Addons (`process.dlopen`)

Many performance-critical libraries utilize compiled C/C++ binary extensions with the `.node` suffix.

### 2.1 Dynamic Loading Support
Beejs natively exposes `process.dlopen` mapped to the underlying dynamic linker:

```typescript
const addonModule = { exports: {} };
process.dlopen(addonModule, './build/Release/my_native_addon.node');
console.log(addonModule.exports.calculate());
```

### 2.2 CommonJS Automatic Dispatch
When calling `require()` on a `.node` file, Beejs intercepts the extension and automatically invokes dynamic linking (`dlopen` / `dlsym`):

```javascript
// Automatically loaded via process.dlopen
const nativeBinding = require('./addon.node');
```

---

## 3. Best Practices
- **Zero `node_modules` Architectures**: Use Import Maps to map dependencies directly to vendor directories, eliminating disk stat overhead;
- **Native Performance**: Leverage compiled C/C++ `.node` modules for CPU-bound SIMD routines.
