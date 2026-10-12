---
title: "测试运行器"
nav: "测试"
subtitle: "amber test 无需额外依赖即可发现并运行 Jest 风格测试。"
status: "stable"
id: "testing"
---

测试运行器是 **Stable**：包括发现、执行和 `--watch`。watch 行为由 [`docs/WATCH_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/WATCH_CONTRACT.md) 约束。

## 编写测试

```js
// math.test.js
describe("math", () => {
  test("adds numbers", () => {
    expect(2 + 3).toBe(5);
  });

  test("maps arrays", () => {
    const doubled = [1, 2, 3].map((n) => n * 2);
    expect(doubled).toEqual([2, 4, 6]);
  });
});
```

全局的 `describe`、`test`、`it` 和 `expect` 会自动注入。TypeScript 测试文件用法相同。

## 运行测试

```bash
amber test                 # 发现并运行所有测试文件
amber test math.test.js    # 运行单个文件
amber test --watch         # 文件变化时重新运行已发现的测试
amber test -t "adds"       # 按测试名过滤（正则）
amber test --bail          # 第一次失败后停止
```

## 发现规则

不传文件参数时，`amber test` 会在当前目录下查找以下模式：

```text
*.test.js  *.test.ts  *.test.mjs
*.spec.js  *.spec.ts  *.spec.mjs
```

会跳过 `node_modules`、`manual` 和 `__snapshots__` 目录。`--watch` 启动后新建的测试文件不会被自动发现。

## 内置能力

内置测试框架包含常用的 Jest 接口：

- 钩子：`beforeAll`、`afterAll`、`beforeEach`、`afterEach`。
- 修饰符：`test.skip`、`test.only`、`test.todo`、`test.each`，`describe` 和 `it` 同样可用。
- 断言：`toBe`、`toEqual`、`toStrictEqual`、`toMatchObject`、`toContain`、`toHaveLength`、`toHaveProperty`、`toThrow`、`toBeCloseTo`、`.not` 等。
- Mock：`jest.fn()`、`jest.spyOn()`，以及 `toHaveBeenCalled*` 系列断言。
- 模块工厂：`jest.mock(specifier, factory)`。它是 `jest.doMock` 的别名，不会被提升，所以要在受影响的 `require` 之前调用。
- 文件快照：`toMatchSnapshot`（`--update-snapshots` 会重写快照）。

Stable 承诺覆盖发现、执行、退出状态和 watch 模式。单个断言、快照和 `--coverage` 没有单独的契约，请在你的构建中自行确认。

## 限制

- `amber test --parallel` 以退出码 2 拒绝。V8 isolate 不跨线程共享。
- 这不是 Jest。不支持配置文件、自定义 reporter、mock 自动提升和假定时器。
