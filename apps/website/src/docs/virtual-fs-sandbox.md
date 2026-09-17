---
title: "In-Memory Virtual Filesystem Sandbox"
subtitle: "Deterministic Copy-On-Write (COW) in-memory filesystem for safely executing untrusted AI Agent code with zero host disk mutation"
group: "Agent & Advanced"
id: "virtual-fs-sandbox"
---

## 1. Why an In-Memory Virtual Filesystem?

Executing untrusted AI-generated scripts or autonomous Agent workflows frequently entails:
- Writing dynamic configuration and temporary artifacts;
- Creating, modifying, or unlinking project files;
- Risking accidental data corruption or pollution of host directories.

Traditional sandboxes like Docker or Firecracker microVMs suffer from:
- **High Startup Overhead**: 500ms to several seconds cold boot, impeding sub-millisecond agent execution;
- **Resource Heaviness**: Demands container daemons, root privileges, and storage layers;
- **Cluttered Teardown**: Requires explicit cleanup scripts to avoid orphan files on disk.

Beejs v1.3.0 provides a **thread-safe, high-performance Virtual Filesystem (VFS)** sandbox:
- **Pure In-Memory Storage**: Writes and mutations occur in RAM with **zero host disk side-effects**;
- **Copy-On-Write (COW)**: Allows reading base host files transparently while isolating all writes;
- **Strict Mode**: `--virtual-fs-strict` cuts off host read fallbacks for a completely blank in-memory workspace;
- **Instant Reset & Snapshot**: Export JSON filesystem snapshots or reset state in less than 1 microsecond.

---

## 2. CLI Usage

Enable with CLI flags on any command without code changes:

```bash
# 1. Enable standard Copy-On-Write (COW) Virtual Filesystem
bee run --virtual-fs ./untrusted_script.js
bee eval --virtual-fs "require('fs').writeFileSync('/etc/secret.txt', 'test'); console.log('Isolated!')"

# 2. Enable strict pure-RAM mode (disables host read fallback)
bee run --virtual-fs --virtual-fs-strict ./agent_task.ts

# 3. Combine with full permission sandbox
bee run --sandbox --virtual-fs ./agent.js
```

---

## 3. JavaScript / TypeScript API (`bee:vfs`)

Control the virtual filesystem lifecycle programmatically:

```typescript
import fs from 'fs';
import * as vfs from 'bee:vfs';

// 1. Inspect status
console.log("VFS Enabled:", vfs.isEnabled());
console.log("COW Mode:", vfs.isCow());

// 2. Enable dynamically in code
vfs.enable(true);

// 3. Standard Node.js fs APIs transparently route to memory
fs.writeFileSync('/workspace/output.txt', 'Agent generated summary');
console.log("File exists:", fs.existsSync('/workspace/output.txt')); // true
console.log("Content:", fs.readFileSync('/workspace/output.txt', 'utf8'));

// 4. List virtualized files
console.log("Files in RAM:", vfs.listFiles());

// 5. Export JSON snapshot
const snapshot = vfs.snapshot();
console.log("Snapshot files:", snapshot.files);

// 6. Reset or disable
vfs.reset();
vfs.disable();
```

---

## 4. Security & Performance Comparison

| Metric | Traditional Docker / VM | Beejs `--virtual-fs` |
| :--- | :--- | :--- |
| **Host Disk Safety** | Relies on mount isolation | **100% in-memory; zero physical mutation** |
| **Startup Latency** | 500ms – 3000ms | **< 0.001ms instantaneous** |
| **Teardown Cleanup** | `rm -rf` / volume cleanup | **Auto-freed on isolate teardown** |
| **Snapshot Speed** | Disk image / git commit | **`vfs.snapshot()` instant JSON export** |
