# Stage 56.4 实施计划 - 测试运行器实现

## 📋 阶段概述

Stage 56.4 专注于实现 Amber 的测试运行器功能，提供类似 Jest 的测试体验，支持 test() / describe() API、断言库、测试发现和并行执行。

**目标**: 构建完整的测试运行系统，使 Amber 具备 Bun 和 Jest 兼容的测试能力。

---

## 🎯 成功标准

### 核心功能
- [ ] **测试 API**: 实现 test(), describe(), it(), expect() 等全局函数
- [ ] **测试发现**: 自动扫描和收集测试文件（*.test.js, *.spec.js）
- [ ] **测试执行**: 支持串行和并行测试执行
- [ ] **断言库**: 完整的断言函数（assert, expect, toBe, toEqual, toThrow 等）
- [ ] **CLI 集成**: `amberjs test` 命令和选项支持
- [ ] **测试报告**: 清晰的测试结果输出和统计信息

### 测试类型支持
- [ ] **同步测试**: 常规函数测试
- [ ] **异步测试**: Promise, async/await 支持
- [ ] **生命周期**: beforeEach, afterEach, beforeAll, afterAll
- [ ] **跳过测试**: test.skip(), describe.skip() 支持
- [ ] **仅测试**: test.only(), describe.only() 支持

### 断言 API
- [ ] **相等性**: toBe, toEqual, toStrictEqual
- [ ] **真值**: toBeTruthy, toBeFalsy, toBeNull, toBeUndefined
- [ ] **数字**: toBeGreaterThan, toBeLessThan, toBeCloseTo
- [ ] **字符串**: toMatch, toContain
- [ ] **数组**: toHaveLength, toContainEqual
- [ ] **对象**: toHaveProperty, toMatchObject
- [ ] **异常**: toThrow, toThrowError

### CLI 选项
- [ ] `amberjs test` - 运行所有测试
- [ ] `amberjs test <pattern>` - 运行匹配的文件
- [ ] `amberjs test --reporter` - 指定报告格式（basic, json）
- [ ] `amberjs test --watch` - 监听模式（可选）

---

## 📝 任务分解

### 阶段 1: 测试 API 实现
**优先级**: 🔴 高
**预计时间**: 2-3 小时

#### 1.1 全局测试函数
- [ ] **创建测试上下文模块**
  - [ ] `src/testing/test_context.rs` - 测试上下文管理
  - [ ] `TestSuite`, `TestCase` 结构体定义
  - [ ] `TestRunner` 执行引擎

- [ ] **实现 test() 函数**
  - [ ] 注册测试用例
  - [ ] 支持名称、函数、超时时间
  - [ ] 支持 only/skip 修饰符

- [ ] **实现 describe() 函数**
  - [ ] 测试套件分组
  - [ ] 嵌套描述支持
  - [ ] 生命周期钩子管理

#### 1.2 生命周期钩子
- [ ] **beforeEach/afterEach**
  - [ ] 每个测试前后执行
  - [ ] 上下文隔离

- [ ] **beforeAll/afterAll**
  - [ ] 整个描述块一次执行
  - [ ] 资源清理

### 阶段 2: 断言库实现
**优先级**: 🔴 高
**预计时间**: 2-3 小时

#### 2.1 断言核心
- [ ] **创建断言模块**
  - [ ] `src/testing/assertions.rs` - 断言实现
  - [ ] `AssertionResult` 结果类型
  - [ ] `Matcher` 匹配器 trait

- [ ] **实现 expect() 函数**
  - [ ] 链式调用支持
  - [ ] 类型转换和验证

#### 2.2 匹配器实现
- [ ] **基础匹配器**
  - [ ] `toBe` - 严格相等 (===)
  - [ ] `toEqual` - 深度相等
  - [ ] `toBeTruthy/toBeFalsy` - 真值检查
  - [ ] `toBeNull/toBeUndefined` - 空值检查

- [ ] **高级匹配器**
  - [ ] `toThrow` - 异常检查
  - [ ] `toMatch` - 正则表达式匹配
  - [ ] `toContain` - 包含检查
  - [ ] `toHaveLength` - 长度检查

### 阶段 3: 测试发现与执行
**优先级**: 🟡 中
**预计时间**: 2-3 小时

#### 3.1 测试发现
- [ ] **文件扫描**
  - [ ] 目录递归扫描
  - [ ] 文件名模式匹配（*.test.js, *.spec.js）
  - [ ] 排除 node_modules

- [ ] **测试加载**
  - [ ] 动态导入测试文件
  - [ ] 全局函数注入
  - [ ] 测试注册

#### 3.2 测试执行引擎
- [ ] **串行执行**
  - [ ] 顺序执行所有测试
  - [ ] 错误处理和报告
  - [ ] 超时控制

- [ ] **并行执行**
  - [ ] 多线程测试执行
  - [ ] 资源隔离
  - [ ] 结果聚合

### 阶段 4: CLI 集成
**优先级**: 🟡 中
**预计时间**: 1-2 小时

#### 4.1 test 子命令
- [ ] **命令解析**
  - [ ] 添加 test 子命令到 CLI
  - [ ] 参数选项支持
  - [ ] 帮助信息

#### 4.2 测试报告
- [ ] **输出格式**
  - [ ] 基础报告（dots, spec）
  - [ ] JSON 报告（机器可读）
  - [ ] 颜色支持

- [ ] **统计信息**
  - [ ] 测试数量统计
  - [ ] 执行时间统计
  - [ ] 失败详情

---

## 🛠️ 技术实现方案

### 架构设计

```rust
// 测试上下文
pub struct TestContext {
    pub suite_name: String,
    pub tests: Vec<TestCase>,
    pub before_each: Vec<Closure>,
    pub after_each: Vec<Closure>,
    pub before_all: Option<Closure>,
    pub after_all: Option<Closure>,
}

// 测试用例
pub struct TestCase {
    pub name: String,
    pub fn: Box<dyn Fn() + Send + Sync>,
    pub timeout: Duration,
    pub skip: bool,
    pub only: bool,
}

// 断言结果
pub struct AssertionResult {
    pub passed: bool,
    pub message: String,
    pub expected: Option<Value>,
    pub actual: Option<Value>,
}
```

### V8 集成
- 使用 `v8::FunctionTemplate` 创建全局测试函数
- 在测试文件执行前注入 test/describe/expect
- 通过回调收集测试结果

### 并行执行
- 使用 `rayon` 或 Tokio 进行并行测试
- 每个测试用例独立 V8 isolate
- 结果通过 channel 聚合

---

## 📊 测试策略

### 单元测试
- [ ] **测试 API 测试**
  - [ ] test() 函数注册
  - [ ] describe() 分组
  - [ ] 生命周期钩子

- [ ] **断言库测试**
  - [ ] 所有匹配器验证
  - [ ] 错误消息生成
  - [ ] 链式调用

### 集成测试
- [ ] **完整测试流程**
  - [ ] 测试发现 → 加载 → 执行 → 报告
  - [ ] 串行和并行执行
  - [ ] 异步测试

### 兼容性测试
- [ ] **Jest 兼容**
  - [ ] Jest 测试用例运行
  - [ ] 断言行为一致
  - [ ] API 兼容性

---

## 📈 性能目标

- **测试执行速度**: 比 Jest 快 20-30%
- **内存使用**: 每个测试 < 5MB
- **启动时间**: < 100ms（100个测试）
- **并行效率**: 4核CPU 提升 3x

---

## 🔮 后续工作（Stage 56.5）

### REPL 实现
- 交互式环境
- 命令历史
- 自动补全
- 特殊命令 (.help, .exit, .load, .save)

### 性能优化
- 测试缓存
- 增量执行
- 智能重跑

---

## 📝 总结

Stage 56.4 将为 Amber 添加完整的测试运行器功能，使其具备与 Bun 和 Jest 相似的测试能力。这对于 AI 时代的高性能 JS/TS 脚本开发至关重要。

**预计完成时间**: 6-8 小时
**主要文件数量**: 8-10 个新文件
**测试覆盖**: 95%+

---

**状态**: 📝 计划制定完成
**下一步**: 开始实现测试 API
**维护者**: Henry Zhang & Claude Code Assistant
**版本**: v0.1.0 (Stage 56.4 Planning Complete - Test Runner)
