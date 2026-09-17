---
title: "Hardened Sandbox & Audit Logging Engine (bee:sandbox)"
subtitle: "Isolated micro-enclaves, real-time ResourceBroker JSONL audit trails, and strict fail-closed defense"
group: "Agent & Advanced"
id: "hardened-sandbox"
---

Running untrusted agent code or dynamically synthesized tools in enterprise environments requires robust boundary defense and tamper-evident compliance logs. If an agent attempts an unauthorized file access or network connection, security teams need immediate visibility.

**Beejs v1.7.0 brings a Hardened Sandbox & Audit Logging Engine (`bee:sandbox` & CLI `--audit-log`)**. It introduces secure execution micro-enclaves that insulate host globals, alongside streaming real-time JSONL audit logging for every runtime resource decision.

---

## 1. Secure Micro-Enclaves (`createEnclave`)

When evaluating dynamic code strings or executing sub-agent logic, `createEnclave` restricts the global namespace to safe ECMAScript built-ins:

```typescript
import { createEnclave } from 'bee:sandbox';

// Execute code inside an isolated enclave
const result = createEnclave('25 * 4 + 10');
console.log(result); // 110

// Inject controlled context variables
const computed = createEnclave('x * y + base', {
  context: { x: 7, y: 8, base: 100 }
});
console.log(computed); // 156

// Execute functions safely
const sum = createEnclave(() => {
  return [10, 20, 30].reduce((a, b) => a + b, 0);
});
console.log(sum); // 60
```

Inside an enclave, dangerous host accessors like `process`, `require`, and unconstrained global access are shielded, keeping execution strictly confined.

---

## 2. Real-Time Security Audit Logging

Track every capability request and security check performed by the runtime:

### 2.1 Programmatic Audit Control

```typescript
import { startAuditLog, stopAuditLog, getAuditLogPath } from 'bee:sandbox';
import { query, revoke } from 'bee:permissions';

// Start streaming audit log to file
startAuditLog('./audit/agent_decisions.jsonl');
console.log('Logging to:', getAuditLogPath());

// Every subsequent permission check is logged automatically
query({ name: 'read', path: '/etc/passwd' });
revoke({ name: 'net', host: '*' });

// Stop logging when session concludes
stopAuditLog();
```

### 2.2 CLI Audit Flag (`--audit-log`)

You can also enable audit logging at the CLI level for any script execution:

```bash
# Append every ResourceBroker decision to audit.jsonl
$ bee run --audit-log ./audit.jsonl agent.ts
```

---

## 3. Audit Log JSONL Format

Each line is a structured JSON record:

```json
{"ts":"2026-09-09T06:00:00.123Z","kind":"FileSystem","action":"Read","resource":"/etc/passwd","decision":"Deny"}
{"ts":"2026-09-09T06:00:01.456Z","kind":"Network","action":"Connect","resource":"api.github.com","decision":"Allow"}
{"ts":"2026-09-09T06:00:02.789Z","kind":"Environment","action":"Read","resource":"OPENAI_API_KEY","decision":"Allow"}
```

Audit entries capture:
- `ts`: ISO8601 millisecond timestamp
- `kind`: Resource domain (`FileSystem`, `Network`, `Environment`, `Process`)
- `action`: Operation attempted (`Read`, `Write`, `Connect`, `Listen`, `Execute`)
- `resource`: Target resource string or normalized path
- `decision`: Verdict (`Allow`, `Deny`)

---

## 4. API Reference

| Function | Parameters | Return Type | Description |
| :--- | :--- | :--- | :--- |
| `createEnclave(codeOrFn, opts?)` | `string \| Function, { context? }` | `T` | Evaluates code within an isolated micro-enclave |
| `startAuditLog(path)` | `path: string` | `boolean` | Starts streaming decisions to a JSONL file |
| `stopAuditLog()` | None | `boolean` | Flushes and terminates audit log streaming |
| `getAuditLogPath()` | None | `string \| null` | Returns active audit log file path or null |
| `isEnabled()` | None | `boolean` | Checks if Virtual Filesystem sandbox is active |
| `enable(cow?)` | `cow = true` | `void` | Enables in-memory virtual filesystem sandbox |
| `disable()` | None | `void` | Disables virtual filesystem sandbox |
| `reset()` | None | `void` | Clears all virtual filesystem memory files |
| `listFiles()` | None | `string[]` | Returns all virtual files in memory |
| `snapshot()` | None | `any` | Exports full virtual filesystem state JSON |
