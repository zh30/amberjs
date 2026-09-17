---
title: "CLI 参考"
subtitle: "v1.16.0 实际提供的命令 — Stable / Preview / Experimental"
group: "参考"
id: "cli-usage"
---

以 `bee --help` 为准。本页按成熟度分组。仓库里的 [Current Scope](https://github.com/zh30/beejs/blob/main/docs/CURRENT_SCOPE.md) 是能力边界。

`--verbose` 是全局参数，必须放在子命令前面：`bee --verbose run app.js`。

---

## Stable

| 命令 | 作用 |
| :--- | :--- |
| `bee run <file> [args...]` | 跑 JS。`.ts` / `.tsx` 先走 oxc（TS 契约仍是 Preview）。 |
| `bee eval <code>` | 求值表达式 |
| `bee repl` | 交互 REPL |
| `bee test [files...] [--watch]` | Jest 风格测试 |
| `bee snapshot [build\|status\|clean]` | V8 启动快照 |
| `bee session <tool>` | Agent 宿主的 stdin JSON-RPC |
| `bee mcp [tool]` | MCP stdio 服务 |
| `bee --version` / `bee version` | 版本 |

### `bee run`

```bash
bee run app.ts
bee run app.js -- arg1 arg2
bee run --watch --debounce 200 app.ts
bee run --preload ./setup.js app.js
bee run --sandbox --permission-policy policy.json app.ts
bee run --inspect-brk app.ts
```

常用参数：

| 参数 | 作用 |
| :--- | :--- |
| `-w, --watch` | 文件变化后重启 |
| `--debounce <ms>` | watch 去抖（默认 100） |
| `-r, --preload <module>` | 主入口前加载 |
| `--timeout <ms>` | CPU watchdog |
| `--max-memory <mb>` | V8 堆上限 |
| `--seed <u64>` | 确定性 `Math.random` |
| `--freeze-time <spec>` | 冻结 `Date.now` / `performance.now` |
| `--sandbox` | 拒绝 fs / net / env / run，再用 `--allow-*` 放行 |
| `--permission-policy <file>` | JSON 策略（别名 `--policy`） |
| `--inspect` / `--inspect-brk` | CDP `127.0.0.1:9229`（Preview） |

`bee test --parallel` 会被拒绝（退出码 2）。

---

## Preview

默认二进制里有，契约还在收紧。

| 命令 | 作用 |
| :--- | :--- |
| `bee serve [file]` | WinterCG `fetch` 处理器。`--https --cert --key` 是 rustls HTTP/1.1。 |
| `bee bundle <entry>` | oxc 模块图 → 单个 JS |
| `bee compile <file>` | 复制 `bee` 并追加 payload + `BEE_STANDALONE` trailer |
| TypeScript / TSX | oxc 类型擦除，不是 `tsc` |
| `--inspect` / `--inspect-brk` | CDP `Runtime.evaluate` |

```bash
bee serve app.js --host 127.0.0.1 --port 3000
bee bundle src/index.ts -o dist/bundle.js --minify
bee compile app.ts -o myapp
```

细节：[打包与编译](/docs/bundling-compilation)。

---

## Experimental

不要当成产品承诺。CLI 上有这些子命令，行为可能不完整。

`debug`、`record`、`replay`、`init`、`create`、`add`、`remove`、`install`、`prune`、`x`、`upgrade`、`fmt`、`lint`、`bench`、`types`、`task`、`profile`、`lsp`、`deploy`。

Chrome DevTools 附加请用 `bee run --inspect`，不要用 `bee debug`。

---

## 权限参数

未加 `--sandbox` 或 `--deny-*` 时，当前默认仍是全部允许。

```bash
bee eval --deny-fs "require('fs').readFileSync('secret.txt', 'utf8')"
bee run --deny-fs --allow-read config.json app.js
bee eval --deny-net --allow-net example.com "fetch('https://example.com')"
```

策略文件（相对路径相对策略文件所在目录解析）：

```json
{
  "permissions": {
    "deny_fs": true,
    "allow_read": ["./config.json"],
    "allow_write": ["./out"],
    "deny_net": true,
    "allow_net": ["api.example.com"],
    "deny_env": true,
    "allow_env": ["PUBLIC_TOKEN"],
    "deny_run": true,
    "allow_run": ["git"]
  }
}
```

更多：[Agent 沙箱](/docs/agent-sandbox)。
