---
title: "企业级能力安全控制 (bee:security)"
subtitle: "动态权限探查、运行时特权衰减与细粒度 ResourceBroker 规则治理"
group: "Agent & Advanced"
id: "capability-security"
---

在运行复杂的多 Agent 协作流水线或执行来自外部的插件时，仅依靠 CLI 启动时的静态参数往往捉襟见肘。现代企业级应用需要在 JavaScript 运行时内**自我查询权限状态、在初始化后主动丢弃敏感特权、并在调度不可信子 Agent 前实施权限衰减（Attenuation）**。

**Beejs v1.6.0 正式推出企业级能力安全引擎（`bee:security` 与 `bee:permissions`）**。完全对齐 W3C 权限查询标准，提供基于最小特权原则的沙箱策略收敛与动态权限撤销。

---

## 1. 能力安全三大核心准则

1. **最小特权原则 (Least Privilege)**：Agent 仅持有完成当前特定子任务所必须的最小能力。
2. **单调特权衰减 (Monotonic Attenuation)**：代码只能自我约束或放弃权限，未经宿主明确授权，绝不可在运行时提升特权。
3. **动态可自省 (Introspection)**：在发起可能受限的 I/O 操作前，允许通过标准接口探查能力可用性，避免未捕获的运行时异常。

---

## 2. 动态权限探查 (`bee:permissions`)

与 Web 标准 `navigator.permissions` 理念对齐：

```typescript
import { query, has, list, revoke } from 'bee:permissions';

// 探查特定文件路径的读权限状态
const status = query({ name: 'read', path: '/etc/hosts' });
if (status.state === 'granted') {
  console.log("允许读取该文件");
}

// 快速布尔值能力检查
if (has({ name: 'net', host: 'api.github.com' })) {
  await fetch("https://api.github.com/zen");
}

// 导出当前底层 ResourceBroker 正在生效的所有 allow 与 deny 规则
const rules = list();
console.log(`当前允许规则数: ${rules.allow.length}`);
console.log(`当前拒绝规则数: ${rules.deny.length}`);
```

---

## 3. 运行时动态特权撤销

在完成敏感操作后，主动丢弃对应权限以防后续逻辑发生提权越界：

```typescript
import { revoke } from 'bee:permissions';

// 读取完数据库密码与证书后，立即永久撤销对保密目录的读取特权
revoke({ name: 'read', path: '/run/secrets' });

// 任务分发完毕后，封锁后续出站公网连接
revoke({ name: 'net', host: '*' });
```

---

## 4. 策略生成与权限衰减 (`bee:security`)

在调用子 Agent 或运行沙箱逻辑前，构建严格的约束策略：

```typescript
import { createSandboxPolicy, attenuate } from 'bee:security';

// 1. 定义主进程/监管者策略
const supervisorPolicy = createSandboxPolicy({
  allowRead: ['/app/data', '/tmp'],
  allowNet: ['api.openai.com', 'internal.db'],
  denyEnv: true
});

// 2. 定义受限 Worker 子任务策略
const workerConstraint = createSandboxPolicy({
  allowRead: ['/tmp'],
  denyNet: true
});

// 3. 衰减（交集计算）：生成只拥有两者交集能力的子沙箱策略
const subPolicy = attenuate(supervisorPolicy, workerConstraint);

console.log(subPolicy.allowRead); // ['/tmp']
console.log(subPolicy.denyNet);   // true
```

---

## 5. 安全 API 完整参考

| 模块名称 | 接口签名 | 功能说明 |
| :--- | :--- | :--- |
| `bee:permissions` | `query(descriptor): PermissionStatus` | 探查指定资源描述符的权限状态（granted / denied） |
| `bee:permissions` | `has(descriptor): boolean` | 便捷返回权限是否被授予的布尔值 |
| `bee:permissions` | `list(): { allow, deny }` | 获取当前生效的全部底层白名单与黑名单规则 |
| `bee:permissions` | `revoke(descriptor): boolean` | 在运行时永久撤销某项能力 |
| `bee:security` | `createSandboxPolicy(rules): Policy` | 构建经过验证的标准沙箱权限策略对象 |
| `bee:security` | `attenuate(base, restriction): Policy` | 基于最小特权原则对两个策略执行交集衰减 |
