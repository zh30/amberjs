---
title: "Enterprise Capability-Based Security (amber:security)"
subtitle: "Dynamic permission introspection, runtime privilege attenuation, and granular ResourceBroker enforcement"
group: "Agent & Advanced"
id: "capability-security"
---

When executing autonomous multi-agent workloads or running plugins from third-party ecosystems, binary allow/deny flags specified at CLI startup are often insufficient. Applications need to **query their own permission state, drop privileges dynamically after initialization, and attenuate policies before delegating work to untrusted sub-agents**.

**Amber v1.6.0 introduces Enterprise Capability-Based Security (`amber:security` & `amber:permissions`)**, providing standard W3C-aligned permission APIs, policy attenuation math, and dynamic capability revocation directly within JavaScript.

---

## 1. Core Principles of Capability Security

1. **Principle of Least Privilege**: Agents only retain the minimum permissions necessary to execute the current task.
2. **Monotonic Attenuation**: An Agent can restrict or drop its own capabilities, but can never elevate privileges without host supervisor consent.
3. **Dynamic Introspection**: Code can query whether an operation will succeed before invoking sensitive I/O, preventing unhandled runtime exceptions.

---

## 2. Dynamic Permission Queries (`amber:permissions`)

Compatible with the web `navigator.permissions` specification:

```typescript
import { query, has, list, revoke } from 'amber:permissions';

// Query if reading a specific path is permitted
const status = query({ name: 'read', path: '/etc/hosts' });
if (status.state === 'granted') {
  console.log("Safe to read file");
}

// Quick boolean capability check
if (has({ name: 'net', host: 'api.github.com' })) {
  await fetch("https://api.github.com/zen");
}

// Inspect all currently active allow and deny rules
const activeRules = list();
console.log(`Active allow rules: ${activeRules.allow.length}`);
console.log(`Active deny rules: ${activeRules.deny.length}`);
```

---

## 3. Dynamic Privilege Revocation

Drop capabilities permanently during application lifecycle:

```typescript
import { revoke } from 'amber:permissions';

// After reading database credentials, revoke read access to secret folders
revoke({ name: 'read', path: '/run/secrets' });

// Drop outbound network access after fetching tasks
revoke({ name: 'net', host: '*' });
```

---

## 4. Policy Creation & Policy Attenuation (`amber:security`)

Create sandboxed sub-policies and attenuate capabilities before executing sub-agents or untrusted code:

```typescript
import { createSandboxPolicy, attenuate } from 'amber:security';

// 1. Define base supervisor policy
const supervisorPolicy = createSandboxPolicy({
  allowRead: ['/app/data', '/tmp'],
  allowNet: ['api.openai.com', 'internal.db'],
  denyEnv: true
});

// 2. Define restricted sub-agent policy constraints
const subAgentConstraint = createSandboxPolicy({
  allowRead: ['/tmp'],
  denyNet: true
});

// 3. Attenuate: The resulting policy is strictly the intersection of permissions
const workerPolicy = attenuate(supervisorPolicy, subAgentConstraint);

console.log(workerPolicy.allowRead); // ['/tmp']
console.log(workerPolicy.denyNet);   // true
```

---

## 5. Security API Reference

| Module | API Signature | Description |
| :--- | :--- | :--- |
| `amber:permissions` | `query(descriptor): PermissionStatus` | Queries granted/denied status for resource |
| `amber:permissions` | `has(descriptor): boolean` | Returns boolean check for permission grant |
| `amber:permissions` | `list(): { allow, deny }` | Dumps current active ResourceBroker rule list |
| `amber:permissions` | `revoke(descriptor): boolean` | Permanently revokes capability at runtime |
| `amber:security` | `createSandboxPolicy(rules): Policy` | Creates a validated sandbox policy object |
| `amber:security` | `attenuate(base, restriction): Policy` | Combines policies using least-privilege attenuation |
