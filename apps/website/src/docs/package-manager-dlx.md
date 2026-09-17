---
title: "Dynamic Package Runner (amber x / amber dlx)"
subtitle: "Execute remote npm packages and CLI tools instantly with content-addressable cache and sandbox limits"
group: "Ecosystem"
id: "package-manager-dlx"
---

## 1. What is `amber x` / `amber dlx`?

Similar to `npx` or `bunx`, `amber x` (aliased as `amber dlx`) lets developers run CLI executables from remote npm packages **without running `npm install`**, without polluting project `node_modules`, and without modifying `package.json`.

Unlike traditional tools, `amber x` is implemented natively in Rust with three key optimizations:
1. **Concurrent Streaming Download & Unpack**: Directly streams tarballs from the registry and extracts to cache via fast async pipelines.
2. **Content-Addressable Global Cache**: Cached under `~/.amber/x_cache`. Consecutive runs of the same version execute in sub-millisecond time.
3. **Native V8 Execution & Sandboxing**: Inspects the `bin` field of `package.json` and runs directly on `amber run`, with support for timeout and memory caps.

---

## 2. Command Syntax & Examples

```bash
# Basic syntax
amber x <package-specifier> [arguments...]

# The dlx alias is also supported
amber dlx cowsay "Hello Amber!"
```

### Specifying Versions & Scopes

```bash
# 1. Run the latest version
amber x cowsay "Hello Amber!"

# 2. Specify exact version
amber x prettier@3.2.5 --write "src/**/*.ts"

# 3. Scoped packages
amber x @biomejs/biome check ./src

# 4. Multi-binary packages
amber x typescript tsc --init
```

---

## 3. Sandboxing & CLI Options

`amber x` inherits the Amber security philosophy, allowing safe execution of untrusted scripts:

```bash
# Cap execution time to 10 seconds
amber x --timeout 10000 untrusted-script

# Cap V8 heap memory to 128MB
amber x --max-memory 134217728 heavy-cli-tool

# Bypass local cache and fetch latest
amber x --no-cache cowsay "Always Fresh"
```

### CLI Flag Reference

| Flag | Short | Description | Default |
| :--- | :--- | :--- | :--- |
| `--timeout <ms>` | `-t` | Maximum execution timeout in milliseconds | Unlimited |
| `--max-memory <bytes>` | `-m` | Maximum V8 heap size in bytes | Unlimited |
| `--no-cache` | | Skip `~/.amber/x_cache` and force download | `false` |
| `--registry <url>` | | Custom npm registry mirror URL | npmjs.org |

---

## 4. CI/CD Pipeline Benefits

In GitHub Actions or containerized build pipelines, `amber x` eliminates heavy `npm install` steps, reducing one-off tool runs and scaffolding setup times from tens of seconds to hundreds of milliseconds:

```bash
# Direct tool invocation in CI
amber x eslint src/
amber x rimraf dist/
```
