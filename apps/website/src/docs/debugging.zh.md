---
title: "调试"
nav: "调试"
subtitle: "一个小型 Chrome DevTools Protocol 端点，用于在运行中的脚本里求值。"
status: "stable"
id: "debugging"
---

`amber run --inspect` 和 `--inspect-brk` 是 **Stable**（G5），范围是 [`docs/INSPECT_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/INSPECT_CONTRACT.md) 中的 CDP 子集。

## 用法

```bash
amber run --inspect app.js
amber run --inspect-brk app.js            # 等待客户端连接后再运行
amber run --inspect --inspect-port 9300 app.js
```

监听地址为 `127.0.0.1`（默认端口 9229，不接受端口 0）。支持：

- `GET /json/version` 和 `GET /json/list`。
- 在用户 isolate 上执行 `Runtime.evaluate`。
- `Runtime.runIfWaitingForDebugger` 和 `Debugger.resume`，用于放行 `--inspect-brk`。

## 限制

这**不是** Chrome DevTools，也不是 V8 inspector。

- 没有断点、单步执行或作用域查看。
- 其他 CDP 方法返回 JSON-RPC 错误 `-32601`。
- `Runtime.evaluate` 只在 isolate 线程空闲时执行：暂停期间或事件循环任务之间。它不会打断正在进行的同步代码。
- 结果只包含 `description`。
- `--inspect` 与 `--watch` 或 `--workers` 大于 1 同时使用时会被拒绝。

## 其他工具

`amber debug`、`amber lsp` 和 `amber profile` 存在于二进制中，但不在 Stable 范围内。详见 [Experimental](/docs/experimental)。
