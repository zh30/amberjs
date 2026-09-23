---
title: "模块系统、包管理与内置测试"
subtitle: "模块解析、有文档边界的 amber install 子集，以及内置测试运行器"
group: "开发者指南"
id: "modules"
---

## 1. 双层全内存模块解析机制 (4.6M ops/s)

在庞大的微服务工程中，启动时往往需要加载成百上千个模块。

### 传统运行时的解析痛点
传统 Node.js 在执行 `require()` 或 `import` 时，必须反复执行昂贵的系统调用：
1. 向上逐层目录寻找 `node_modules`；
2. 读取并解析 `package.json` 中的 `exports` 或 `main` 字段；
3. 检查 `.js`、`.json`、`.node` 等多种扩展名；
4. 每次判断都会发起数十次底层的 `stat` 或 `access` 系统调用，成为启动速度的“头号杀手”。

### Amber 的 Stat Bypass 双缓存架构
Amber 设计了**双层全内存规范化模块缓存系统**：

```text
       require('lodash') 或 import ... from './utils'
                            │
                            ▼
      +─────────────────────────────────────────────+
      |  L1: 规范化路径缓存 (Normalized Path Cache)  |
      |  - 哈希表瞬间命中已定位的绝对物理路径         |
      +─────────────────────────────────────────────+
                            │ 未命中
                            ▼
      +─────────────────────────────────────────────+
      |  L2: 解析规格缓存 (Resolved Specifier Cache)|
      |  - 避免重复读取 package.json exports 映射    |
      |  - 完全绕过文件系统 stat 系统调用            |
      +─────────────────────────────────────────────+
```

在官方模块解析基准测试中，Amber 的解析吞吐达到了 **4,601,226 ops/s**，比 Node.js (1.12M ops/s) **快 4.1 倍**，比 Bun (3.88M ops/s) 更进一步！

---

## 2. ESM 与 CommonJS 深度混用

Amber 原生支持 ECMAScript 模块（ESM）与 CommonJS（CJS）规范的自由互操作：

```typescript
// 1. 标准 ESM 导入
import { readFileSync } from 'node:fs';
import { Tensor } from 'amber:ai';

// 2. 传统 CommonJS require 混用
const path = require('node:path');

// 3. 动态导入 (Dynamic import())
if (process.env.LOAD_OPTIONAL) {
  const mod = await import('./optional-module.js');
  mod.init();
}

// 4. 获取当前模块路径与目录
console.log('当前模块 URL:', import.meta.url);
console.log('当前模块绝对目录:', __dirname);
console.log('当前模块绝对文件名:', __filename);
```

### 模块协议前缀支持
- **`node:*`**：显式引用 Node.js 兼容核心模块（推荐做法）；
- **`amber:*`**：引用 Amber 原生独有模块（如 `amber:ai` 原生张量与模型引擎）；
- **相对/绝对路径**：`./`、`../`、`/` 加载本地磁盘模块，支持省略 `.ts`、`.tsx`、`.js` 扩展名。

---

## 3. `amber install`（Stable 子集）

`amber install` 安装 `package.json` 里的直接 `dependencies` 和 `devDependencies`，并核对 `package-lock.json` 顶层 `dependencies` 的版本、`resolved` 和 `integrity`。它**不是** npm、yarn 或 pnpm 的替代品。workspaces、生命周期脚本、`peerDependencies`、`yarn.lock`、`pnpm-lock.yaml` 以及 lockfile 的 `packages` 字段都不在契约内。`amber add` 和 `amber prune` 仍是 Experimental。

契约：[INSTALL_CONTRACT.md](https://github.com/zh30/amberjs/blob/main/docs/INSTALL_CONTRACT.md)。

```bash
amber install
amber install --frozen-lockfile
```

---

## 4. 免配置内置测试框架 (`amber test`)

Amber 提供了对齐 **Jest / Vitest** 现代测试生态的内置测试套件，零依赖开箱即用：

### 编写测试用例
创建一个名为 `math.test.ts` 的文件：

```typescript
// math.test.ts
import { describe, it, test, expect } from 'amber:test'; // 或直接使用全局注入的 describe/test

describe('核心算术与张量逻辑', () => {
  it('基础数值相加应当正确', () => {
    expect(1 + 1).toBe(2);
    expect([1, 2, 3]).toHaveLength(3);
    expect({ name: 'amberjs' }).toEqual({ name: 'amberjs' });
  });

  test('异步操作应正确完成', async () => {
    const data = await Promise.resolve('ready');
    expect(data).toBe('ready');
  });

  test('错误抛出断言', () => {
    expect(() => {
      throw new Error('非法输入');
    }).toThrow('非法输入');
  });
});
```

### 运行测试
```bash
# 运行当前目录下所有测试文件 (*.test.js, *.test.ts, *.spec.ts)
$ amber test

# 仅运行匹配名称的测试用例
$ amber test -t "异步操作"

# 并行执行测试套件以提速
$ amber test --parallel

# 遇到首个失败立即终止 (Bail)
$ amber test --bail

# 监听模式 (保存文件时自动重跑测试)
$ amber test -w
```

测试执行结果输出精炼清晰：
```text
 PASS  tests/math.test.ts (12 ms)
  核心算术与张量逻辑
    ✓ 基础数值相加应当正确 (1 ms)
    ✓ 异步操作应正确完成 (2 ms)
    ✓ 错误抛出断言 (0 ms)

Test Suites: 1 passed, 1 total
Tests:       3 passed, 3 total
Snapshots:   0 total
Time:        0.018s
```
