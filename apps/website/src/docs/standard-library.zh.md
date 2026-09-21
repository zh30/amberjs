---
title: "官方标准库 (amber:std)"
subtitle: "零依赖、工程实用的现代标准库：环境变量、终端交互、文件系统、密码学与断言"
group: "生态与扩展"
id: "standard-library"
---

## 1. 为什么推出官方标准库？

在传统 Node.js 生态中，即使是极其常见的基础功能（如加载 `.env`、在终端打印带颜色的表格、生成 UUID、递归复制目录、或执行断言验证），开发者也不得不引入 `dotenv`、`chalk`、`cli-table`、`uuid`、`fs-extra` 等大量第三方 npm 依赖。这导致了 `node_modules` 极度膨胀、启动变慢，并面临深层供应链安全风险。

Amber 推出 **`amber:std`** 官方现代标准库：所有模块均使用 Rust 原生实现，具备极高的执行效率与零依赖保证。

---

## 2. 模块一览与引入方式

你既可以通过统一的命名空间引入，也可以按需导入特定子模块：

```typescript
// 统一导入
import { dotenv, cli, fs, crypto, assert } from 'amber:std';

// 或者按需导入子模块
import { config, parse } from 'amber:std/dotenv';
import { colors, table, ProgressBar } from 'amber:std/cli';
import { walk, copyDir } from 'amber:std/fs';
import { uuidv4, uuidv7, jwt } from 'amber:std/crypto';
import { assertEquals, assertThrows } from 'amber:std/assert';
```

---

## 3. 子模块功能详解

### 一、`amber:std/dotenv` 环境变量管理

零依赖加载并解析 `.env` 配置文件，完美支持注释过滤、单双引号去除以及环境变量插值展开（`${VAR}`）。

```typescript
import { config, parse } from 'amber:std/dotenv';

// 自动读取当前目录的 .env 文件并注入到 process.env 中
config();

// 或者指定路径
config({ path: './config/.env.production' });

// 解析字符串配置
const envVars = parse(`
  PORT=8080
  DATABASE_URL=sqlite://${process.env.HOME}/app.db
`);
console.log('端口:', process.env.PORT);
```

---

### 二、`amber:std/cli` 终端样式与交互工具

提供开箱即用的终端 ANSI 颜色着色、Unicode 边框格式化表格与控制台交互。

```typescript
import { colors, table, ProgressBar } from 'amber:std/cli';

// 1. 文本颜色与修饰
console.log(colors.green(colors.bold('✔ 部署成功！')));
console.log(colors.yellow('⚠ 警告：检测到内存占用超过 80%'));

// 2. 格式化表格
const headers = ['名称', '版本', '状态'];
const rows = [
  ['amberjs', 'v1.16.0', colors.green('Active')],
  ['sqlite', 'v3.45', colors.green('Active')],
];
console.log(table(headers, rows));

// 3. 进度条指示器
const bar = new ProgressBar(100);
for (let i = 0; i <= 100; i += 20) {
  bar.update(i);
}
```

---

### 三、`amber:std/fs` 高级文件系统操作

补充原生 `node:fs` 所欠缺的高级操作，提供同步与递归文件遍历：

```typescript
import { walk, copyDir, emptyDir } from 'amber:std/fs';

// 1. 递归遍历目录，并按文件后缀筛选
const tsFiles = walk('./src', { extensions: ['ts', 'tsx'] });
console.log('所有 TypeScript 文件:', tsFiles);

// 2. 递归拷贝整目录
copyDir('./templates', './dist/project');

// 3. 清空目录内部所有内容但保留根目录
emptyDir('./temp_cache');
```

---

### 四、`amber:std/crypto` 现代密码学与令牌

提供 UUID v4（随机）、UUID v7（按时间单调递增排序，非常适合作为数据库主键）、JWT 令牌签名与验签。

```typescript
import { uuidv4, uuidv7, jwt, hash } from 'amber:std/crypto';

// 1. 生成 UUID
const id = uuidv4();
const timeOrderedId = uuidv7(); // 适合用作 B-Tree 索引主键

// 2. JWT (HMAC-SHA256) 签名与验签
const secret = 'amberjs-super-secret-key';
const token = jwt.sign({ userId: 1001, role: 'admin' }, secret, { expiresIn: 3600 });
console.log('生成的 JWT Token:', token);

// 验证 Token
const payload = jwt.verify(token, secret);
console.log('解码有效载荷:', payload.userId, payload.role);

// 3. 快速哈希计算
console.log('SHA-256:', hash('hello world', 'sha256'));
```

---

### 五、`amber:std/assert` 轻量深度断言

具备清晰的错误提示信息与类型安全断言：

```typescript
import { assert, assertEquals, assertThrows } from 'amber:std/assert';

assert(1 + 1 === 2, '基础数学逻辑必须成立');
assertEquals({ a: 1, b: [2, 3] }, { a: 1, b: [2, 3] });

assertThrows(() => {
  throw new Error('预期中的异常');
}, '预期中的异常');
```
