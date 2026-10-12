---
title: "Agent 沙箱"
nav: "沙箱与权限"
subtitle: "运行工具代码时默认拒绝文件、网络、环境变量和进程访问，除非你显式放行。"
status: "stable"
id: "agent-sandbox"
---

权限沙箱、JSONL 审计日志，以及确定性时间与随机数都是 **Stable**。它们是运行 Agent 或模型生成代码时的安全层。

## 默认拒绝

```bash
amber run --sandbox tool.js
```

在 `--sandbox` 下，JavaScript 无法读写文件、建立网络连接或监听端口、读取环境变量，也无法启动子进程。被拒绝的调用会在操作执行前抛出包含 `permission denied` 的错误。入口文件本身仍可读取。

## 只放行需要的内容

```bash
amber run --sandbox \
  --allow-read ./data/input.txt \
  --allow-write ./out/result.json \
  --allow-net api.example.com \
  --allow-env API_TOKEN \
  --allow-run git \
  tool.js
```

每个 `--allow-*` 参数都可重复，分别精确匹配路径、主机或 URL、变量名或命令名。`--allow-listen` 对服务端监听起同样作用。

如果不想开启完整沙箱、只拒绝某一类能力，可以用 `--deny-fs`、`--deny-net`、`--deny-env` 或 `--deny-run`。

## 策略文件

把同样的规则写成 JSON，并通过 `--permission-policy`（别名 `--policy`）传入：

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

## 审计日志

```bash
amber run --sandbox --allow-read ./data --audit-log audit.jsonl tool.js
```

每次权限判定会以一行 JSON 追加写入，包含类别、动作、资源和判定结果。程序的其他内容不会写入日志。

## 确定性运行

```bash
amber run --seed 42 --freeze-time 2026-01-01T00:00:00Z agent.js
```

- `--seed` 让 `Math.random()`、`crypto.getRandomValues()` 和 `crypto.randomBytes()` 可复现。
- `--freeze-time` 把 `Date.now()`、`new Date()` 和 `performance.now()` 固定在一个时间戳或 ISO 8601 字符串。

## 资源限制

```bash
amber run --timeout 5000 --max-memory 256 tool.js
```

`--timeout` 在指定毫秒数后终止脚本，`--max-memory` 以 MB 为单位限制 isolate 堆大小。

## 它不是什么

这个沙箱是进程内的权限代理，不是操作系统级隔离、容器或 seccomp 配置。如果威胁模型需要，请同时在容器或虚拟机中运行不受信任的代码。

`--virtual-fs` 参数和 `amber:security` 模块存在于二进制中，但不在 Stable 范围内。详见 [Experimental](/docs/experimental)。
