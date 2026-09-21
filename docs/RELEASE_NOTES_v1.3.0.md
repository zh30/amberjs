# Amber v1.3.0 Release Notes: Native AI Embeddings, Model Context Protocol & Virtual FS Sandbox

> **Release Tag**: `v1.3.0`  
> **Release Type**: Minor Release (次版本 / 中版本)  
> **Target Commits**: Native zero-dependency AI text embeddings, full MCP (Model Context Protocol 2.0) server/client & CLI inspect, Copy-on-Write Virtual Filesystem (VFS) sandbox, TypeScript types, and comprehensive bilingual documentation.

---

## 概述 (Overview)

Amber **v1.3.0** 是专为 **AI Agent 时代**打造的重大次版本升级。继 v1.2.0 推出内置 SQLite/VectorDB 与标准库生态后，v1.3.0 进一步在运行时底层原生整合了 AI Agent 基础设施三件套：

1. **AI 2.0 原生零依赖文本嵌入与语义推断 (`amber:ai`)**：
   - 纯 Rust 高效实现的零外部依赖确定性文本向量嵌入引擎（`embed`, `embedBatch`, `cosineSimilarity`）。
   - 默认输出 64 维超轻量归一化向量（亦支持 128 / 384 维），单次推断耗时 `< 50 µs`。
   - 采用 subword/n-gram 哈希投影、GELU 激活、位置权重衰减与 L2 范数归一化，具强区分度的语义余弦相似度。
   - 与 `amber:vector` 的 `VectorDB` 及 `Tensor` 库无缝联动，实现极速本地嵌入式 RAG。

2. **Model Context Protocol 2.0 原生支持与 CLI 调试 (`amber:mcp`)**：
   - 全面支持 Anthropic MCP (2024-11-05) 标准。
   - 原生提供 `McpServer` 与 `McpClient` API，涵盖工具 (`tools`)、资源 (`resources`) 与提示词模板 (`prompts`)。
   - 传输通道原生支持标准输入输出流 `server.startStdio()` 与超轻量同进程内存通道 `server.connectLocal()`。
   - 全新 CLI 命令 `amber mcp --inspect [file]`，可视化输出 MCP 服务端导出的工具、资源与提示词清单。

3. **Copy-on-Write 纯内存虚拟文件系统安全沙箱 (`--virtual-fs` & `amber:vfs`)**：
   - 专为 AI Agent 自主生成代码执行与不受信任代码评估设计的写时复制 (COW) 内存文件系统。
   - 命令行标志 `--virtual-fs`（别名 `--vfs`）与 `--virtual-fs-strict`：
     - 所有写操作、目录创建、文件修改拦截于 RAM 中，物理宿主磁盘 100% 保持零突变、零污染。
     - 读操作优先走内存视图，不存在时透明 fallback 至物理宿主磁盘（strict 模式下禁用 fallback）。
   - 全面透明拦截 Node.js `fs` 同步与 promise 异步全套 API，并提供 `amber:vfs` / `amber:sandbox` 控制接口与快照导出能力。

4. **完善的类型系统与文档生态**：
   - `types/amberjs.d.ts` 与 `src/types_export.rs` 全量同步更新，提供完整的 TypeScript 类型提示。
   - 官网新增 3 篇双语技术深度文档，覆盖 5 种国际化语言（中、英、西、法、印）导航体系，并通过 Vite 全量构建。

---

## 模块新特性深度解析

### 1. `amber:ai` 2.0 本地文本嵌入与向量计算
```javascript
import { embed, embedBatch, cosineSimilarity } from 'amber:ai';
import { VectorDB } from 'amber:vector';

// 零依赖、纯原生极速推断 (默认 64 维 Float32Array)
const vecA = embed("Rust 语言与高性能系统编程");
const vecB = embed("Amber 采用 Rust 构建极速运行时");
const similarity = cosineSimilarity(vecA, vecB);
console.log(`相似度: ${similarity.toFixed(4)}`); // > 0.85

// 与 VectorDB 无缝协同
const db = new VectorDB({ dimensions: 64, metric: 'cosine' });
db.insert('doc1', embed("Amber v1.3.0 发布"));
const results = db.search(embed("Amber 最新版本"), 1);
```

### 2. `amber:mcp` Model Context Protocol 协议支持
```javascript
import { McpServer, McpClient } from 'amber:mcp';

const server = new McpServer({ name: 'calc-agent', version: '1.0.0' });

server.tool('add', 'Add two numbers', {
  a: { type: 'number', description: 'First number' },
  b: { type: 'number', description: 'Second number' }
}, async ({ a, b }) => {
  return { result: a + b };
});

// 支持直接本地内存连通，无需网络或子进程通信
const client = server.connectLocal();
await client.ping();
const result = await client.callTool('add', { a: 40, b: 2 });
console.log(result); // { result: 42 }
```

### 3. `--virtual-fs` 纯内存 COW 沙箱
```bash
# 运行不受信任脚本，所有落盘操作均在 RAM 中，对宿主磁盘零污染
amber run --virtual-fs untrusted_agent_script.js
amber eval --virtual-fs "require('fs').writeFileSync('important.txt', 'mutated');"
# 宿主磁盘上的 important.txt 毫发无损！
```

---

## 验证与质量保证

- `cargo test --test v1_3_0_features_tests`：6/6 测试全部通过
- `cargo test --lib`：单元测试通过
- `cargo fmt --all -- --check`：通过 (0 格式差异)
- `cargo clippy -- -D warnings`：通过 (0 warnings)
- `website npm run build`：全量文档与国际化页面 1.81s 构建通过
