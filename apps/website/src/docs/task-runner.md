---
title: "Task Runner & Script Execution"
subtitle: "Lightweight zero-dependency scripts runner deeply integrated with package.json workflows"
group: "Developer Tooling"
id: "task-runner"
---

Executing lifecycle scripts in `package.json` traditionally requires a heavyweight Node.js runtime and npm installation. Beejs provides a native, high-performance **Task Runner** built directly in Rust.

---

## 1. Quick Usage

### 1.1 Listing Available Tasks
Run `bee task` without arguments in any project directory containing a `package.json`:

```bash
$ bee task
```

Console output:
```text
📋 Available tasks in package.json:

  Task                 Command
  ────────────────────────────────────────────────────────
  build                bee bundle src/index.ts -o dist/bundle.js
  test                 bee test --coverage
  lint                 bee lint src/
  format               bee fmt src/
  serve                bee serve app.ts --port 3000
```

### 1.2 Running a Task
Execute a task with `bee task <name>` or `bee run <script>`:

```bash
$ bee task build
# Or identically to npm / bun:
$ bee run build
```

Pass additional arguments to the underlying command after `--`:
```bash
$ bee task test -- --bail
```

---

## 2. Architecture & Environment Isolation

### 2.1 Zero Node.js / npm Dependency
The task runner parses `package.json` directly using fast Rust deserialization and spawns lightweight OS processes without needing Node.js or npm installed on the machine.

### 2.2 Intelligent PATH Prioritization
When invoking commands, Beejs automatically configures environment variables:
1. **Local `.bin` Precedence**: Prepends `<project_root>/node_modules/.bin` to `PATH`;
2. **Current `bee` Binary Forwarding**: Prepends the directory of the running `bee` binary to `PATH`, ensuring that `bee fmt` or `bee test` scripts invoke the current runtime version;
3. **Cross-Platform Shell Spawning**: Dispatches through `sh -c` on Unix (macOS / Linux) and `cmd.exe /C` on Windows to cleanly support compound shell operators and pipes.

---

## 3. Recommended Workflow

Sample `package.json` for a modern Beejs project:

```json
{
  "name": "my-beejs-service",
  "version": "1.0.0",
  "scripts": {
    "dev": "bee serve app.ts --watch",
    "build": "bee bundle app.ts -o dist/bundle.js --minify",
    "compile": "bee compile app.ts -o my-service",
    "check": "bee lint src/ && bee fmt src/ --check",
    "test": "bee test --coverage",
    "bench": "bee bench benches/"
  }
}
```

Now you can drive your entire development lifecycle with clean commands:
```bash
$ bee run check
$ bee run test
$ bee run build
```
