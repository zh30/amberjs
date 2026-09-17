# Stage 56 实施计划 - CLI 功能完善与 Bun 兼容性

## 📋 阶段概述

Stage 56 专注于完善 Amber 的 CLI 功能，使其具备 Bun 运行时的大部分 CLI 能力，为用户提供熟悉的命令行体验。

**目标**: 构建完整的 CLI 系统，支持脚本执行、包管理、测试运行等核心功能。

---

## 🎯 成功标准

### 核心功能
- [ ] **脚本执行**: 支持 `.js`、`.ts`、`.mjs` 文件直接执行
- [ ] **包管理器**: 集成 npm/yarn 兼容的包管理功能
- [ ] **测试运行器**: 内置测试框架，支持 Jest 风格测试
- [ ] **REPL 交互**: 交互式 JavaScript/TypeScript 执行环境
- [ ] **热重载**: `--watch` 模式支持文件监听和自动重载

### CLI 命令
- [ ] `amberjs run <script>` - 执行脚本
- [ ] `amberjs test` - 运行测试
- [ ] `amberjs repl` - 启动 REPL
- [ ] `amberjs <file>` - 直接执行文件
- [ ] `amberjs --watch <file>` - 热重载模式

### 兼容性
- [ ] **Bun 兼容**: 90%+ Bun CLI 命令兼容
- [ ] **Node.js 兼容**: 支持大部分 Node.js 脚本
- [ ] **TypeScript**: 原生 TypeScript 支持，无需预编译

---

## 📝 任务分解

### 阶段 56.1: CLI 核心架构
**优先级**: 🔴 高
**预计时间**: 3-4 小时

#### 1.1 CLI 框架搭建
- [ ] **选择 CLI 框架**
  - [ ] 评估 clap、structopt 等选项
  - [ ] 设计命令结构
  - [ ] 实现基础命令解析

#### 1.2 子命令系统
- [ ] **实现子命令架构**
  - [ ] `run` 子命令 - 脚本执行
  - [ ] `test` 子命令 - 测试运行
  - [ ] `repl` 子命令 - 交互式环境
  - [ ] `bundle` 子命令 - 代码打包（可选）

#### 1.3 全局选项
- [ ] **标准选项支持**
  - [ ] `--version` / `-v` - 显示版本
  - [ ] `--help` / `-h` - 帮助信息
  - [ ] `--config` - 配置文件路径
  - [ ] `--env` - 环境变量设置

### 阶段 56.2: 脚本执行引擎
**优先级**: 🔴 高
**预计时间**: 4-5 小时

#### 2.1 文件类型检测
- [ ] **自动文件类型识别**
  - [ ] `.js` / `.mjs` - JavaScript
  - [ ] `.ts` - TypeScript（需编译）
  - [ ] `.json` - JSON 脚本
  - [ ] Shebang 检测（`#!/usr/bin/env amberjs`）

#### 2.2 执行上下文
- [ ] **脚本执行环境**
  - [ ] 全局对象设置
  - [ ] `__dirname` / `__filename` 支持
  - [ ] `module.exports` / `exports` 支持
  - [ ] `require()` 模块加载
  - [ ] `import` / `export` ES 模块支持

#### 2.3 参数传递
- [ ] **命令行参数处理**
  - [ ] `process.argv` 填充
  - [ ] `--` 参数分隔符支持
  - [ ] 环境变量传递

### 阶段 56.3: 包管理器集成
**优先级**: 🟡 中
**预计时间**: 3-4 小时

#### 3.1 依赖解析
- [ ] **package.json 支持**
  - [ ] 读取依赖声明
  - [ ] 解析版本范围
  - [ ] 锁文件支持（可选）

#### 3.2 模块解析算法
- [ ] **Node.js 模块算法**
  - [ ] `node_modules` 查找
  - [ ] 文件扩展名优先级（.js → .json → .node）
  - [ ] 目录模块（package.json main 字段）
  - [ ] 相对/绝对路径解析

#### 3.3 核心模块polyfill
- [ ] **内置模块模拟**
  - [ ] `fs` - 文件系统
  - [ ] `path` - 路径处理
  - [ ] `os` - 操作系统信息
  - [ ] `crypto` - 加密功能
  - [ ] `http` / `https` - HTTP 客户端

### 阶段 56.4: 测试运行器
**优先级**: 🟡 中
**预计时间**: 3-4 小时

#### 4.1 测试框架
- [ ] **内置测试支持**
  - [ ] `test()` / `describe()` API
  - [ ] 断言库（expect / assert）
  - [ ] 钩子函数（beforeEach / afterEach）

#### 4.2 测试发现
- [ ] **自动测试收集**
  - [ ] `**/*.test.js` 模式匹配
  - [ ] `**/*.spec.js` 模式匹配
  - [ ] `test/` 目录扫描

#### 4.3 测试执行
- [ ] **并行执行**
  - [ ] 多进程测试运行
  - [ ] 超时控制
  - [ ] 错误捕获和报告

### 阶段 56.5: REPL 实现
**优先级**: 🟢 低
**预计时间**: 2-3 小时

#### 5.1 交互式环境
- [ ] **行编辑功能**
  - [ ] 命令历史（上下箭头）
  - [ ] 自动补全（Tab）
  - [ ] 多行输入支持

#### 5.2 特殊命令
- [ ] **REPL 专属命令**
  - [ ] `.help` - 帮助信息
  - [ ] `.exit` / `.quit` - 退出
  - [ ] `.load` - 加载文件
  - [ ] `.save` - 保存会话

---

## 🛠️ 技术实现细节

### CLI 架构设计

```rust
/// 主 CLI 应用
#[derive(Command, Debug)]
#[command(name = "amberjs")]
#[command(about = "High-performance JavaScript/TypeScript runtime")]
pub struct CliApp {
    /// 启用详细输出
    #[arg(short, long)]
    pub verbose: bool,

    /// 配置文件路径
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// 子命令
    #[command(subcommand)]
    pub command: Option<SubCommand>,
}

/// 子命令枚举
#[derive(Subcommand, Debug)]
pub enum SubCommand {
    /// 运行脚本
    Run(RunCommand),
    /// 运行测试
    Test(TestCommand),
    /// 启动 REPL
    Repl(ReplCommand),
    /// 版本信息
    Version,
}

/// 运行命令
#[derive(Command, Debug)]
pub struct RunCommand {
    /// 要执行的脚本文件
    pub script: PathBuf,

    /// 脚本参数
    pub args: Vec<String>,

    /// 启用热重载
    #[arg(short, long)]
    pub watch: bool,
}
```

### 脚本执行器

```rust
/// 脚本执行器
pub struct ScriptExecutor {
    /// V8 isolate
    isolate: v8::OwnedIsolate,
    /// 模块解析器
    module_resolver: ModuleResolver,
    /// 全局对象
    global_context: v8::Global<v8::Context>,
}

impl ScriptExecutor {
    /// 执行脚本文件
    pub async fn execute_file(&mut self, path: &Path) -> Result<Value> {
        // 1. 检测文件类型
        let file_type = self.detect_file_type(path)?;

        // 2. 读取源代码
        let source = tokio::fs::read_to_string(path).await?;

        // 3. 编译/转译（如需要）
        let compiled = match file_type {
            FileType::JavaScript => source,
            FileType::TypeScript => self.compile_typescript(&source)?,
            _ => return Err(Error::UnsupportedFileType),
        };

        // 4. 执行代码
        self.execute_script(&compiled, path)
    }
}
```

### 模块解析器

```rust
/// Node.js 兼容的模块解析器
pub struct ModuleResolver {
    /// 当前工作目录
    current_dir: PathBuf,
    /// 模块缓存
    cache: HashMap<String, Module>,
    /// node_modules 搜索路径
    search_paths: Vec<PathBuf>,
}

impl ModuleResolver {
    /// 解析模块路径
    pub fn resolve(&self, request: &str, parent: &Path) -> Result<PathBuf> {
        // 1. 相对/绝对路径检查
        if request.starts_with('./') || request.starts_with('../') {
            return self.resolve_relative(request, parent);
        }

        // 2. 内置模块检查
        if self.is_builtin_module(request) {
            return self.resolve_builtin(request);
        }

        // 3. node_modules 搜索
        self.resolve_from_node_modules(request, parent)
    }
}
```

---

## 📊 预期成果

### CLI 功能对比
| 功能 | Bun | Amber Stage 56 | Node.js |
|------|-----|----------------|---------|
| `amberjs script.js` | ✅ | ✅ | ✅ |
| `amberjs test` | ✅ | ✅ | ❌ |
| `amberjs repl` | ✅ | ✅ | ✅ |
| `--watch` 模式 | ✅ | ✅ | ❌ |
| TypeScript 支持 | ✅ | ✅ | ❌ |
| 包管理器 | ✅ | 🟡 | ❌ |

### 性能目标
- [ ] **启动时间**: < 30ms（空脚本）
- [ ] **执行性能**: 与 Bun 相当（±10%）
- [ ] **内存占用**: < 15MB（空运行时）
- [ ] **热重载**: < 100ms（文件变更到重新执行）

---

## 📅 时间计划

| 子阶段 | 预计时间 | 关键里程碑 |
|-------|---------|-----------|
| 56.1: CLI 核心架构 | 3-4 小时 | CLI 框架搭建完成 |
| 56.2: 脚本执行引擎 | 4-5 小时 | 支持 JS/TS 文件执行 |
| 56.3: 包管理器集成 | 3-4 小时 | 模块解析完成 |
| 56.4: 测试运行器 | 3-4 小时 | 测试框架完成 |
| 56.5: REPL 实现 | 2-3 小时 | 交互式环境完成 |
| **总计** | **15-20 小时** | **Stage 56 完成** |

---

## 🎓 学习要点

### CLI 设计最佳实践
1. **用户友好** - 提供清晰的错误信息和帮助文档
2. **性能优先** - 快速启动，最小开销
3. **兼容性** - 遵循现有工具（Bun/Node.js）的使用习惯

### 模块系统设计
1. **一致性** - 遵循 Node.js 模块解析算法
2. **性能** - 缓存已解析的模块
3. **安全性** - 限制文件系统访问范围

---

## 📚 参考文献

- [Bun CLI 文档](https://bun.sh/docs/cli)
- [Node.js 模块系统](https://nodejs.org/api/modules.html)
- [Rust CLI 设计指南](https://rust-cli.github.io/book/)
- [V8 脚本执行](https://v8.dev/blog/embedded-v8)

---

**状态**: 📋 Stage 56 计划制定完成
**下一步**: 开始阶段 56.1 - CLI 核心架构
**维护者**: Henry Zhang & Claude Code Assistant
**版本**: v0.1.0 (Stage 56 Planning Complete)
