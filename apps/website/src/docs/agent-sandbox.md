---
title: "Agent sandbox"
nav: "Sandbox & permissions"
subtitle: "Run tool code with file, network, environment, and process access denied unless you allow it."
status: "stable"
id: "agent-sandbox"
---

The permission sandbox, the JSONL audit log, and deterministic time and randomness are **Stable**. They are the safety layer for running code that an agent or model wrote.

## Deny by default

```bash
amber run --sandbox tool.js
```

Under `--sandbox`, JavaScript cannot read or write files, open network connections or listeners, read environment variables, or start child processes. A denied call throws an error that contains `permission denied` before the operation runs. The entry file itself can still be read.

## Allow exactly what you need

```bash
amber run --sandbox \
  --allow-read ./data/input.txt \
  --allow-write ./out/result.json \
  --allow-net api.example.com \
  --allow-env API_TOKEN \
  --allow-run git \
  tool.js
```

Each `--allow-*` flag is repeatable and matches an exact path, host or URL, variable name, or command name. `--allow-listen` does the same for servers.

To deny one capability without the full sandbox, use `--deny-fs`, `--deny-net`, `--deny-env`, or `--deny-run`.

## Policy files

Put the same rules in JSON and pass `--permission-policy` (alias `--policy`):

```json
{
  "permissions": {
    "deny_fs": true,
    "deny_net": true,
    "allow_read": ["./data/input.txt"],
    "allow_net": ["api.example.com"]
  }
}
```

```bash
amber run --permission-policy policy.json tool.js
```

## Audit log

```bash
amber run --sandbox --allow-read ./data --audit-log audit.jsonl tool.js
```

Each permission decision is appended as one JSON line with its kind, action, resource, and decision. Nothing else from your program is written to the log.

## Deterministic runs

```bash
amber run --seed 42 --freeze-time 2026-01-01T00:00:00Z agent.js
```

- `--seed` makes `Math.random()`, `crypto.getRandomValues()`, and `crypto.randomBytes()` reproducible.
- `--freeze-time` pins `Date.now()`, `new Date()`, and `performance.now()` to a fixed timestamp or ISO 8601 string.

## Resource limits

```bash
amber run --timeout 5000 --max-memory 256 tool.js
```

`--timeout` stops a script after that many milliseconds. `--max-memory` caps the isolate heap in megabytes.

## What this is not

The sandbox is a permission broker inside one process. It is not an OS-level jail, a container, or a seccomp profile. Run untrusted code in a container or VM as well if the threat model needs it.

The `--virtual-fs` flag and the `amber:security` module exist in the binary but are not in the Stable scope. See [Experimental](/docs/experimental).
