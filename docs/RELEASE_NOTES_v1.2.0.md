# Amber v1.2.0 Release Notes: Embedded Data Engine, Standard Library & Ecosystem Tooling

> **Release Tag**: `v1.2.0`  
> **Release Type**: Minor Release (中版本)  
> **Target Commits**: Full ecosystem tools, embedded SQLite & VectorDB, official stdlib, dynamic package runner, automated deployer, and modernized VS Code extension.

---

## 概述 (Overview)

Amber **v1.2.0** 是继 v1.1.0 工具链升级后的重大次版本升级。基于**高性能、零外部依赖、开发者生态优先**的设计哲学，v1.2.0 带来了全套深度集成的生态与周边系统：

1. **嵌入式数据与向量底座 (`amber:db` & `amber:vector`)**：
   - 静态内置编译的 **SQLite 3** 引擎（基于 `rusqlite bundled`），支持 `:memory:` 临时库与磁盘持久化、参数化语句与事务自动回滚。
   - 极速内存**高维向量检索数据库 (VectorDB)**，支持 `cosine` 余弦相似度、`euclidean` 欧氏距离与 `dot` 点积，提供 Top-K 快速近邻排序与 JSON 序列化。
2. **官方现代零依赖标准库 (`amber:std`)**：
   - `amber:std/dotenv`：环境变量零依赖加载与展开。
   - `amber:std/cli`：终端 ANSI 颜色、格式化边框表格与动态进度条。
   - `amber:std/fs`：高级文件系统递归遍历 (`walk`)、递归拷贝 (`copyDir`) 与清空 (`emptyDir`)。
   - `amber:std/crypto`：高熵 UUID v4、时间单调递增 UUID v7、JWT (HMAC-SHA256) 签名与验签。
   - `amber:std/assert`：现代轻量深度相等断言库。
3. **动态包即时运行器 (`amber x` / `amber dlx`)**：
   - 类似 `npx` / `bunx`，流式拉取远程 npm 包并在 `~/.amberjs/x_cache` 进行内容寻址缓存。
   - 零安装、零污染项目 `node_modules`，直接在 V8 中运行，支持 `--timeout` 与 `--max-memory` 硬配额沙箱。
4. **全自动部署与容器编排 (`amber deploy`)**：
   - 自动探测应用入口，一键生成生产级非 root（`amberjs:10001`）多阶段 `Dockerfile`、`.dockerignore` 与 `docker-compose.yml`。
   - 支持独立单文件可执行应用编译（`--target compile`）。
   - 支持云原生 Kubernetes 生产配置清单（`--target k8s`：Deployment、Service、HPA）。
5. **VS Code 官方编辑器深度扩展 (`tools/vscode-extension`)**：
   - 升级至 v1.2.0，提供原生 stdio LSP 语言服务（诊断、悬停、智能补全）。
   - 支持 Chrome DevTools CDP 远程单步断点调试、oxc 极速保存格式化与一键部署。
6. **官网文档中心全量升级**：
   - 新增 10 篇双语技术手册，并在 5 种国际化语言（中、英、西、法、印）中同步上线「生态与扩展」大纲与图标。

---

## 验证与质量保证

- `cargo test --test ecosystem_tools_tests`：9/9 测试通过 (0.05s)
- `cargo test --test bundler_integration_tests`：23/23 测试通过 (9.32s)
- `cargo test --test tooling_cli_tests`：17/17 测试通过 (1.29s)
- `cargo test --lib`：385/385 测试通过 (1.00s)
- `cargo fmt --all -- --check`：通过 (0 格式差异)
- `cargo clippy --bin amber --test ecosystem_tools_tests -- -D warnings`：通过 (0 warnings)
- `website npm run build`：2426 模块 1.83s 编译通过
