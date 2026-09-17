# 贡献指南

感谢您对 Amber 项目的兴趣！我们欢迎所有形式的贡献，包括但不限于代码、文档、测试、问题报告和功能建议。

## 📋 目录

- [行为准则](#行为准则)
- [如何贡献](#如何贡献)
- [开发环境设置](#开发环境设置)
- [代码规范](#代码规范)
- [提交规范](#提交规范)
- [Pull Request 流程](#pull-request-流程)
- [测试指南](#测试指南)
- [文档贡献](#文档贡献)
- [问题报告](#问题报告)

## 🤝 行为准则

### 我们的承诺

我们致力于为每个人创造一个开放、友好的环境。无论您是新手还是专家，我们都欢迎您加入我们的社区。

### 我们的标准

积极贡献的例子包括：
- ✅ 使用友好和包容的语言
- ✅ 尊重不同的观点和经历
- ✅ 优雅地接受建设性批评
- ✅ 关注对社区最有利的事情
- ✅ 对其他社区成员表示同理心

不可接受的行为包括：
- ❌ 使用性化的语言或图像
- ❌ 恶意评论、人身攻击或政治攻击
- ❌ 公开或私下的骚扰
- ❌ 未经明确许可发布他人的私人信息
- ❌ 在专业环境中可能被合理认为不适当的其他行为

## 🚀 如何贡献

您可以通过以下方式为项目做出贡献：

1. **🐛 报告 Bug**
   - 使用 GitHub Issues
   - 提供详细的复现步骤
   - 包含环境信息

2. **💡 提出新功能**
   - 使用 GitHub Issues
   - 详细描述功能需求
   - 讨论实现的可行性

3. **📝 改进文档**
   - 修复错别字
   - 添加示例
   - 完善 API 文档

4. **💻 提交代码**
   - 修复 Bug
   - 实现新功能
   - 优化性能

5. **🧪 添加测试**
   - 单元测试
   - 集成测试
   - 性能测试

## 🛠️ 开发环境设置

### 前置要求

- **Rust**: 1.70 或更高版本
- **Git**: 最新版本
- **Python**: 3.7-3.13 (用于 PyO3 构建)
- **CMake**: 3.0+ (用于构建 V8)

### 设置步骤

1. **Fork 仓库**

```bash
# 点击 GitHub 上的 Fork 按钮
# 然后克隆您的 fork
git clone https://github.com/YOUR_USERNAME/beejs.git
cd beejs

# 添加上游仓库
git remote add upstream https://github.com/ORIGINAL_OWNER/beejs.git
```

2. **安装 Rust**

```bash
# 安装 rustup (如果尚未安装)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 验证安装
rustc --version
cargo --version
```

3. **设置 Python 环境** (可选，如果使用 Python 集成)

```bash
# Python 3.14+ 用户需要设置环境变量
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1

# 或者使用 pyenv 管理 Python 版本
pyenv install 3.11.0
pyenv local 3.11.0
```

4. **构建项目**

```bash
# 开发构建 (更快)
cargo build

# 发布构建 (优化性能)
cargo build --release

# 运行测试
cargo test
```

5. **验证安装**

```bash
./target/debug/amber --version
./target/debug/amber run examples/basics/hello_world.js
```

### IDE 配置

#### VS Code

推荐扩展：
- `rust-analyzer` - Rust 语言服务器
- `CodeLLDB` - 调试器
- `error lens` - 错误显示
- `better-comments` - 注释高亮

创建 `.vscode/settings.json`:

```json
{
    "rust-analyzer.cargo.features": "all",
    "rust-analyzer.checkOnSave.command": "clippy",
    "editor.formatOnSave": true,
    "editor.codeActionsOnSave": {
        "source.fixAll": true
    },
    "files.associations": {
        "*.rs": "rust"
    }
}
```

#### CLion

1. 安装 Rust 插件
2. 导入项目为 Cargo 项目
3. 配置运行/调试配置

## 📏 代码规范

### Rust 代码规范

我们遵循标准的 Rust 编码规范：

1. **格式化**: 使用 `cargo fmt`

```bash
cargo fmt
```

2. **Linting**: 使用 `clippy`

```bash
cargo clippy --all-targets --all-features -- -D warnings
```

3. **文档**: 所有公共 API 必须有文档

```rust
/// 执行 JavaScript 代码
///
/// # 示例
///
/// ```
/// let runtime = RuntimeLite::new()?;
/// let result = runtime.execute_string("1 + 1")?;
/// assert_eq!(result.as_i32(), Some(2));
/// ```
pub fn execute_string(&self, code: &str) -> Result<Value> {
    // 实现
}
```

4. **错误处理**: 使用 `anyhow::Result`

```rust
use anyhow::{Context, Result};

pub fn execute_file<P: AsRef<Path>>(&self, path: P) -> Result<Value> {
    let path = path.as_ref();
    let content = std::fs::read_to_string(path)
        .context(format!("Failed to read file: {:?}", path))?;
    // ...
}
```

5. **命名约定**

```rust
// 结构体和枚举：PascalCase
struct RuntimeLite;

// 函数和方法：snake_case
fn execute_file() {}

// 常量：SCREAMING_SNAKE_CASE
const MAX_HEAP_SIZE: usize = 1024 * 1024 * 1024;

// 变量：snake_case
let execution_result = ...;

// 文件名：snake_case
src/runtime_lite.rs
```

### JavaScript 代码规范

对于 JavaScript 示例和测试：

1. 使用 4 空格缩进
2. 使用单引号而不是双引号
3. 使用分号
4. 使用有意义的变量名

```javascript
// 好的写法
const sum = numbers.reduce((acc, num) => acc + num, 0);

// 避免
const s = n.reduce((a, n) => a + n, 0);
```

## 📝 提交规范

我们使用 [Conventional Commits](https://www.conventionalcommits.org/) 规范。

### 提交信息格式

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### 类型 (Type)

- `feat`: 新功能
- `fix`: Bug 修复
- `docs`: 文档更改
- `style`: 代码格式化
- `refactor`: 重构
- `test`: 添加或修改测试
- `chore`: 构建过程或辅助工具的变动
- `perf`: 性能优化
- `ci`: CI/CD 配置

### 示例

```bash
# 新功能
git commit -m "feat(runtime): add support for async/await"

# Bug 修复
git commit -m "fix(cache): resolve memory leak in SmartCache"

# 文档
git commit -m "docs(api): update RuntimeLite documentation"

# 重构
git commit -m "refactor(debugger): simplify breakpoint management"

# 测试
git commit -m "test(monitoring): add performance metrics tests"
```

### 提交消息要求

1. **标题**: 简洁明了，不超过 72 字符
2. **正文**: 详细说明"为什么"而不是"做了什么"
3. **引用**: 如果修复了 issue，在 footer 中引用

```bash
git commit -m "feat(monitoring): add real-time performance tracking

- Implement PerformanceMonitor with microsecond precision
- Add memory usage tracking and GC pause monitoring
- Support custom metrics and alerting thresholds

Closes #123"
```

## 🔄 Pull Request 流程

### 1. 创建分支

```bash
# 从 main 分支创建功能分支
git checkout main
git pull upstream main
git checkout -b feature/your-feature-name
```

### 2. 开发

```bash
# 进行更改
# ...

# 运行测试
cargo test

# 运行格式化
cargo fmt

# 运行 clippy
cargo clippy --all-targets --all-features -- -D warnings
```

### 3. 提交更改

```bash
# 添加更改
git add .

# 提交 (遵循提交规范)
git commit -m "feat(module): add dynamic module loading"
```

### 4. 推送并创建 PR

```bash
# 推送到您的 fork
git push origin feature/your-feature-name

# 在 GitHub 上创建 Pull Request
```

### 5. PR 检查清单

创建 PR 时，请确保：

- [ ] 标题清晰，遵循提交规范
- [ ] PR 描述详细说明更改内容
- [ ] 所有测试通过
- [ ] 代码经过格式化 (cargo fmt)
- [ ] 没有 clippy 警告
- [ ] 添加了必要的测试
- [ ] 更新了相关文档
- [ ] 运行了基准测试 (如果适用)

### 6. 代码审查

- 至少需要 1 个审查者的批准
- 响应审查意见并及时更新
- 所有检查必须通过

### 7. 合并

只有审查者和维护者可以合并 PR。

## 🧪 测试指南

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_runtime_lite

# 运行测试并显示输出
cargo test -- --nocapture

# 并行运行测试
cargo test --parallel
```

### 编写测试

#### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_simple_code() {
        let runtime = RuntimeLite::new().unwrap();
        let result = runtime.execute_string("1 + 1").unwrap();
        assert_eq!(result.as_i32(), Some(2));
    }

    #[test]
    fn test_execute_async_code() -> Result<()> {
        let runtime = RuntimeLite::new()?;
        let result = runtime.execute_string("Promise.resolve(42)")?;
        // 异步测试需要特殊处理
        Ok(())
    }
}
```

#### 集成测试

创建 `tests/` 目录下的文件：

```rust
// tests/integration_test.rs
use beejs::RuntimeLite;

#[test]
fn test_execute_file() {
    let runtime = RuntimeLite::new().unwrap();
    let result = runtime.execute_file("examples/basics/hello_world.js");
    assert!(result.is_ok());
}
```

#### 性能测试

```rust
#[cfg(test)]
mod perf_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_performance() {
        let start = Instant::now();
        let runtime = RuntimeLite::new().unwrap();

        for _ in 0..1000 {
            runtime.execute_string("Math.sqrt(16)").unwrap();
        }

        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 1000, "Performance test failed");
    }
}
```

### 测试覆盖率

```bash
# 安装 tarpaulin
cargo install cargo-tarpaulin

# 生成覆盖率报告
cargo tarpaulin --out html

# 查看报告
open tarpaulin-report.html
```

### 测试标准

- 测试覆盖率应 > 90%
- 所有新功能必须有测试
- 修复 Bug 时必须添加回归测试
- 测试应该是确定性的

## 📚 文档贡献

### 文档类型

1. **API 文档**: 使用 `///` 注释
2. **用户指南**: Markdown 格式
3. **示例代码**: `examples/` 目录
4. **README**: 项目概述

### 编写 API 文档

```rust
/// 执行 JavaScript 代码字符串
///
/// # 参数
///
/// * `code` - 要执行的 JavaScript 代码
///
/// # 返回值
///
/// 返回执行结果，如果出错返回错误
///
/// # 示例
///
/// ```
/// let runtime = RuntimeLite::new()?;
/// let result = runtime.execute_string("console.log('hello')")?;
/// ```
pub fn execute_string(&self, code: &str) -> Result<Value> {
    // 实现
}
```

### 文档格式

- 使用中文撰写用户面向的文档
- 使用英文撰写 API 文档
- 代码块使用 triple backticks
- 添加适当的标题层级

## 🐛 问题报告

### Bug 报告模板

在 GitHub Issues 中使用以下模板：

```markdown
**Bug 描述**
清晰简洁地描述 bug 是什么。

**复现步骤**
1. 运行 '...'
2. 点击 '...'
3. 滚动到 '...'
4. 看到错误

**期望行为**
清晰简洁地描述您期望发生什么。

**实际行为**
清晰简洁地描述实际发生了什么。

**环境信息**
 - OS: [e.g. macOS 13.0]
 - Rust 版本: [e.g. 1.70.0]
 - Amber 版本: [e.g. v0.1.0]

**额外信息**
添加任何其他关于问题的信息。
```

### 功能请求模板

```markdown
**功能描述**
清晰简洁地描述您希望的功能。

**问题背景**
这个功能解决了什么问题？

**期望解决方案**
您希望这个功能如何工作？

**替代方案**
您考虑过其他解决方案吗？

**额外信息**
添加任何其他关于功能请求的信息或截图。
```

## 🎯 开发流程

### Stage 开发流程

项目采用 Stage 驱动的开发模式：

1. **Stage 规划**: 在 `PROGRESS.md` 中记录目标
2. **阶段分解**: 每个 Stage 分为多个 Phase
3. **实现**: 按 Phase 逐步实现
4. **测试**: 全面测试和性能验证
5. **文档**: 更新文档和报告
6. **合并**: 合并到 main 分支

### 发布流程

1. **版本号**: 遵循 [Semantic Versioning](https://semver.org/)
   - 主版本号：不兼容的 API 修改
   - 次版本号：向后兼容的功能性新增
   - 修订号：向后兼容的问题修正

2. **发布检查清单**:
   - [ ] 所有测试通过
   - [ ] 文档更新
   - [ ] 性能基准测试通过
   - [ ] 更新 CHANGELOG.md
   - [ ] 创建 GitHub Release

## 📞 获取帮助

如果您需要帮助，可以通过以下方式联系我们：

1. **GitHub Discussions**: 社区讨论
2. **GitHub Issues**: 问题报告和功能请求
3. **Email**: [项目维护者邮箱]

## 🙏 致谢

感谢所有为 Amber 项目做出贡献的开发者！

## 📄 许可证

通过贡献代码，您同意您的贡献将在 MIT 许可证下授权。

---

**感谢您的贡献！** 🚀
