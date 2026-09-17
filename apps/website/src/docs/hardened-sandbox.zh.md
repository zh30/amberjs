---
title: "加固沙箱与安全合规审计引擎 (bee:sandbox)"
subtitle: "隔离微环境 Enclave、全量 ResourceBroker 决策 JSONL 流式合规审计与零信任防护"
group: "Agent & Advanced"
id: "hardened-sandbox"
---

在企业生产级环境中执行不可信的 Agent 脚本或动态合成的外部工具时，强效的边界隔离与不可篡改的审计追踪是不可或缺的防线。当 Agent 试图发起越权文件访问或非法网络探测时，安全团队需要毫秒级的实时可见性。

**Beejs v1.7.0 带来了加固沙箱与安全审计日志引擎（`bee:sandbox` 与 CLI `--audit-log`）**。支持创建屏蔽宿主全局污染的安全隔离微环境（Enclave），并为所有底层系统资源决策生成结构化流式 JSONL 审计追踪。

---

## 1. 安全隔离微环境 (`createEnclave`)

在动态执行来自外部的复杂计算逻辑或子 Agent 产物时，`createEnclave` 将执行环境严格限制于无害的安全 ECMAScript 全局对象集合中：

```typescript
import { createEnclave } from 'bee:sandbox';

// 在完全隔离的环境中执行表达式
const result = createEnclave('25 * 4 + 10');
console.log(result); // 110

// 安全注入受控的上下文变量
const computed = createEnclave('x * y + base', {
  context: { x: 7, y: 8, base: 100 }
});
console.log(computed); // 156

// 执行闭包函数并返回结果
const sum = createEnclave(() => {
  return [10, 20, 30].reduce((a, b) => a + b, 0);
});
console.log(sum); // 60
```

在 Enclave 内部，诸如 `process`、`require` 以及未经允许的宿主全局属性均被彻底隔离屏蔽，从根本上防止原型链越权逃逸。

---

## 2. 运行时全量安全审计流

全生命周期追踪运行时的每一次能力权限申请与拦截：

### 2.1 编程式审计日志管理

```typescript
import { startAuditLog, stopAuditLog, getAuditLogPath } from 'bee:sandbox';
import { query, revoke } from 'bee:permissions';

// 开启 JSONL 审计日志流式记录
startAuditLog('./audit/agent_decisions.jsonl');
console.log('审计写入目标:', getAuditLogPath());

// 此后所有的权限校验与资源访问决策均自动落盘记录
query({ name: 'read', path: '/etc/passwd' });
revoke({ name: 'net', host: '*' });

// 会话结束时停止记录并刷盘
stopAuditLog();
```

### 2.2 命令行一键审计参数 (`--audit-log`)

亦可在启动脚本时直接通过 CLI 参数开启审计：

```bash
# 将本次运行产生的所有 ResourceBroker 裁决记录到 audit.jsonl 中
$ bee run --audit-log ./audit.jsonl agent.ts
```

---

## 3. 审计日志 JSONL 结构规范

每一行均为严格的单行 JSON 记录，方便与 ELK、Splunk 等日志分析系统无缝对接：

```json
{"ts":"2026-09-09T06:00:00.123Z","kind":"FileSystem","action":"Read","resource":"/etc/passwd","decision":"Deny"}
{"ts":"2026-09-09T06:00:01.456Z","kind":"Network","action":"Connect","resource":"api.github.com","decision":"Allow"}
{"ts":"2026-09-09T06:00:02.789Z","kind":"Environment","action":"Read","resource":"OPENAI_API_KEY","decision":"Allow"}
```

字段说明：
- `ts`: ISO8601 毫秒级时间戳
- `kind`: 资源类别（`FileSystem` / `Network` / `Environment` / `Process`）
- `action`: 操作行为（`Read` / `Write` / `Connect` / `Listen` / `Execute`）
- `resource`: 目标资源描述符（路径、域名或变量名）
- `decision`: 仲裁结论（`Allow` 允许 / `Deny` 拒绝）

---

## 4. API 完整参考

| 函数名称 | 参数说明 | 返回类型 | 功能说明 |
| :--- | :--- | :--- | :--- |
| `createEnclave(codeOrFn, opts?)` | `string \| Function, { context? }` | `T` | 在安全隔离微环境中执行代码或函数并返回结果 |
| `startAuditLog(path)` | `path: string` | `boolean` | 启动 JSONL 格式安全合规审计日志记录 |
| `stopAuditLog()` | 无 | `boolean` | 停止审计记录并立即刷盘 |
| `getAuditLogPath()` | 无 | `string \| null` | 获取当前正在写入的审计文件路径 |
| `isEnabled()` | 无 | `boolean` | 检查内存虚拟文件系统是否处于启用状态 |
| `enable(cow?)` | `cow = true` | `void` | 开启内存虚拟文件系统沙箱 |
| `disable()` | 无 | `void` | 关闭内存虚拟文件系统沙箱 |
| `reset()` | 无 | `void` | 清空内存虚拟文件系统中的所有虚拟文件 |
| `listFiles()` | 无 | `string[]` | 列出内存中现存的所有虚拟文件路径 |
| `snapshot()` | 无 | `any` | 导出当前内存虚拟文件系统的全量状态 JSON 快照 |
