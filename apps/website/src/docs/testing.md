---
title: "Test runner"
nav: "Testing"
subtitle: "amber test discovers and runs Jest-style tests without extra dependencies."
status: "stable"
id: "testing"
---

The test runner is **Stable**: discovery, execution, and `--watch`. The watch behavior is pinned by [`docs/WATCH_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/WATCH_CONTRACT.md).

## Write a test

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

The globals `describe`, `test`, `it`, and `expect` are installed for you. TypeScript test files work the same way.

## Run tests

```bash
amber test                 # discover and run every test file
amber test math.test.js    # run one file
amber test --watch         # re-run the discovered set on change
amber test -t "adds"       # filter by test name (regex)
amber test --bail          # stop after the first failure
```

## Discovery

Without a file argument, `amber test` walks the current directory for these patterns:

```text
*.test.js  *.test.ts  *.test.mjs
*.spec.js  *.spec.ts  *.spec.mjs
```

It skips `node_modules`, `manual`, and `__snapshots__` directories. `--watch` does not pick up new test files created after it starts.

## What the harness provides

The built-in harness includes the common Jest surface:

- Hooks: `beforeAll`, `afterAll`, `beforeEach`, `afterEach`.
- Modifiers: `test.skip`, `test.only`, `test.todo`, `test.each`, and the same on `describe` and `it`.
- Matchers such as `toBe`, `toEqual`, `toStrictEqual`, `toMatchObject`, `toContain`, `toHaveLength`, `toHaveProperty`, `toThrow`, `toBeCloseTo`, and `.not`.
- Mocks with `jest.fn()` and `jest.spyOn()`, plus the `toHaveBeenCalled*` matchers.
- Module factories with `jest.mock(specifier, factory)`. This is an alias of `jest.doMock`: it is not hoisted, so call it before the `require` it should affect.
- File snapshots with `toMatchSnapshot` (`--update-snapshots` rewrites them).

The Stable promise covers discovery, execution, exit status, and watch mode. Individual matchers, snapshots, and `--coverage` are not separately contracted, so check them on your build.

## Limits

- `amber test --parallel` is rejected with exit code 2. V8 isolates are not shared across threads.
- This is not Jest. Config files, custom reporters, automatic mock hoisting, and fake timers are not part of the runner.
