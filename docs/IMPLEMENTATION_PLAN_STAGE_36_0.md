# Amber Stage 36.0 实施计划 - CLI 增强与性能基准

## 📋 任务概览

**目标**: 增强 CLI 功能并建立性能基准体系，为比 Bun 更快奠定基础
**阶段**: Stage 36.0
**开始时间**: 2025-12-19
**预计完成**: 2025-12-19

## 🎯 Stage 36.0 核心目标

### 1. 增强 CLI 功能 (优先级: 极高)

#### 目标
- 实现 `--watch` 模式的文件监控
- 添加 REPL 功能
- 支持 package.json 集成

#### 成功标准
- [ ] 文件监控：自动检测文件变化并重新执行
- [ ] REPL：交互式 JavaScript/TypeScript 解释器
- [ ] package.json：自动读取并支持 scripts、dependencies
- [ ] 错误处理：友好的错误提示和堆栈跟踪

#### 关键实现
```rust
// CLI 增强组件
1. file_watcher.rs - 文件监控
2. repl.rs - 交互式 REPL
3. package_json.rs - package.json 集成
4. cli.rs - 主 CLI 逻辑增强
```

### 2. 性能基准测试系统 (优先级: 高)

#### 目标
- 对比 Bun/Node.js 的性能基准
- 实现自动化性能回归检测
- 添加可视化性能报告

#### 成功标准
- [ ] 基准测试套件：启动时间、执行速度、内存使用
- [ ] 自动化对比：Bun vs Node.js vs Amber
- [ ] 回归检测：自动检测性能退化
- [ ] 可视化报告：HTML/Markdown 格式性能报告

#### 关键实现
```rust
// 性能基准组件
1. benchmark_suite.rs - 基准测试套件
2. performance_comparison.rs - 与 Bun/Node.js 对比
3. regression_detection.rs - 自动化回归检测
4. report_generator.rs - 性能报告生成
```

## 🔧 技术实现方案

### 1. 文件监控实现

#### 方案 A: 使用 `notify` crate
```rust
pub struct FileWatcher {
    paths: Vec<PathBuf>,
    sender: mpsc::Sender<()>,
    receiver: mpsc::Receiver<()>,
}

impl FileWatcher {
    pub async fn watch(&mut self) -> Result<(), Box<dyn Error>> {
        // 使用 notify 监听文件变化
        // 支持 .js/.ts/.mjs/.cjs/.jsx/.tsx 文件
        // 忽略 node_modules 和 .git 目录
    }
}
```

#### 方案 B: 使用 `tokio::fs::metadata` 轮询
```rust
pub struct SimpleFileWatcher {
    last_modified: HashMap<PathBuf, SystemTime>,
    interval: Duration,
}
```

**选择**: 方案 A（notify crate）- 更高效，实时响应

### 2. REPL 实现

#### 核心功能
```rust
pub struct Repl {
    runtime: Arc<Runtime>,
    history: Vec<String>,
    prompt: String,
}

impl Repl {
    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        // 1. 读取用户输入
        // 2. 执行 JavaScript/TypeScript
        // 3. 输出结果
        // 4. 保存到历史记录
        // 5. 重复
    }
}
```

#### 支持功能
- 多行输入（以 `\` 结尾）
- 自动补全（基础）
- 历史记录（上下箭头）
- 语法高亮（可选）

### 3. package.json 集成

#### 支持的字段
```json
{
  "name": "amberjs-app",
  "version": "1.0.0",
  "scripts": {
    "start": "amberjs src/index.js",
    "dev": "amberjs watch src/index.js",
    "test": "amberjs test"
  },
  "amberjs": {
    "entry": "src/index.js",
    "optimize": "aggressive"
  }
}
```

#### 实现逻辑
```rust
pub struct PackageJson {
    scripts: HashMap<String, String>,
    entry: Option<PathBuf>,
    config: AmberConfig,
}

impl PackageJson {
    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        // 读取并解析 package.json
        // 提取 scripts 字段
        // 提取 amberjs 专用配置
    }
}
```

### 4. 性能基准测试

#### 基准测试套件
```rust
pub enum BenchmarkType {
    StartupTime,    // 启动时间
    ExecutionSpeed, // 执行速度
    MemoryUsage,    // 内存使用
    Concurrency,    // 并发性能
}

pub struct Benchmark {
    name: String,
    test_type: BenchmarkType,
    iterations: usize,
    code: String,
}

impl Benchmark {
    pub async fn run(&self) -> Result<BenchmarkResult, Box<dyn Error>> {
        // 执行基准测试
        // 记录时间、内存等指标
        // 返回结果
    }
}
```

#### 性能对比
```rust
pub struct PerformanceComparison {
    amberjs_result: BenchmarkResult,
    bun_result: Option<BenchmarkResult>,
    nodejs_result: Option<BenchmarkResult>,
}

impl PerformanceComparison {
    pub async fn run_all() -> Result<Self, Box<dyn Error>> {
        // 运行 Amber 基准测试
        // 尝试运行 Bun/Node.js 基准测试（如果可用）
        // 对比结果
    }
}
```

## 📁 文件结构

```
src/
├── cli/
│   ├── mod.rs
│   ├── file_watcher.rs        # 新增：文件监控
│   ├── repl.rs                # 新增：REPL 功能
│   ├── package_json.rs        # 新增：package.json 集成
│   └── enhanced_cli.rs        # 增强的 CLI
├── benchmarks/
│   ├── mod.rs
│   ├── benchmark_suite.rs     # 新增：基准测试套件
│   ├── performance_comparison.rs # 新增：性能对比
│   ├── regression_detection.rs   # 新增：回归检测
│   └── report_generator.rs       # 新增：报告生成
└── main.rs                    # 更新：集成新功能

tests/
├── cli_enhancement_tests.rs   # 新增：CLI 增强测试
└── performance_benchmark_tests.rs # 新增：性能基准测试
```

## 🧪 测试策略

### 1. CLI 增强测试
| 功能 | 测试用例 | 预期结果 |
|------|----------|----------|
| 文件监控 | 文件修改检测 | < 100ms 响应时间 |
| 文件监控 | 多文件监控 | 同时监控多个文件 |
| REPL | 单行输入 | 正确执行并输出结果 |
| REPL | 多行输入 | 支持连续输入 |
| package.json | 读取 scripts | 正确解析并执行 |
| package.json | 读取 amberjs 配置 | 正确读取专用配置 |

### 2. 性能基准测试
| 测试类型 | 测试场景 | 预期指标 |
|----------|----------|----------|
| 启动时间 | 空脚本 | < 5ms |
| 启动时间 | 复杂脚本 | < 20ms |
| 执行速度 | 循环测试 | 比 Node.js 快 2x+ |
| 内存使用 | 大对象创建 | 比 Node.js 少 30%+ |
| 并发性能 | 100 并发任务 | 线性扩展 |

## 🚀 性能目标

### 启动时间目标
- **当前**: ~11ms
- **目标**: < 5ms
- **提升**: 2.2x 更快

### 执行速度目标
- **对比基准**: Node.js 基准为 1.0x
- **目标**: 2.0x - 5.0x 更快
- **重点**: 循环、函数调用、对象操作

### 内存使用目标
- **对比基准**: Node.js 基准为 100MB
- **目标**: < 70MB
- **优化**: 内存池、智能垃圾回收

## 📊 实施步骤

### Step 1: 文件监控功能 (30 分钟)
1. 创建 `file_watcher.rs` 模块
2. 实现基于 `notify` 的文件监控
3. 集成到主 CLI
4. 编写测试用例

### Step 2: REPL 功能 (45 分钟)
1. 创建 `repl.rs` 模块
2. 实现基础 REPL 功能
3. 支持多行输入和历史记录
4. 编写测试用例

### Step 3: package.json 集成 (30 分钟)
1. 创建 `package_json.rs` 模块
2. 实现 package.json 解析
3. 支持 scripts 执行
4. 编写测试用例

### Step 4: 性能基准系统 (60 分钟)
1. 创建 `benchmark_suite.rs` 模块
2. 实现基础基准测试
3. 添加性能对比功能
4. 编写测试用例

### Step 5: 文档和报告 (30 分钟)
1. 更新 README.md
2. 生成性能报告
3. 更新 PROGRESS.md
4. 创建 Stage 36.0 完成报告

**总计**: ~3.5 小时

## ✅ 成功标准

### 必达目标
- [ ] 文件监控功能正常工作
- [ ] REPL 可以执行基础 JavaScript
- [ ] package.json 脚本可以正确执行
- [ ] 性能基准测试套件运行正常
- [ ] 所有测试用例通过

### 期望目标
- [ ] 启动时间 < 8ms（vs 当前 11ms）
- [ ] 执行速度比 Node.js 快 2x+
- [ ] 内存使用减少 20%+
- [ ] 生成详细的性能对比报告

## 🔍 风险评估

### 高风险
- **文件监控性能**: 大量文件时可能性能下降
  - **缓解**: 添加文件过滤和延迟机制

### 中风险
- **REPL 实现复杂度**: 需要正确处理异步和错误
  - **缓解**: 从简单实现开始，逐步增强

### 低风险
- **package.json 兼容性**: 标准化格式，风险低
  - **缓解**: 仅支持核心字段，避免复杂配置

## 📝 总结

Stage 36.0 将显著增强 Amber 的 CLI 功能和性能基准能力，为成为"比 Bun 更快的运行时"奠定基础。通过文件监控、REPL、package.json 集成，Amber 将拥有现代化的开发体验；通过性能基准系统，我们将量化性能改进并持续优化。

---

**实施时间**: 2025-12-19
**负责人**: Amber 开发团队
**状态**: 待开始
