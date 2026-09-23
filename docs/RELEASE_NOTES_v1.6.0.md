# Amber v1.6.0 Release Notes: Deterministic Agent Replay Engine, GGUF/SafeTensors Model Weights Loader & Enterprise Capability Security

> **Release Tag**: `v1.6.0`  
> **Release Type**: Minor Release (次版本 / 中版本)  
> **Target Commits**: Deterministic Agent Replay Engine (`amber:replay` & CLI `amber record` / `amber replay`), native binary GGUF v2/v3 & SafeTensors model weights loader (`amber:weights` / `amber:ai.weights`) with zero-copy mmap tensor slicing into `amber:ai.Tensor`, enterprise capability-based security & dynamic permission control (`amber:security` / `amber:permissions`), unified TypeScript type definitions (`types/amberjs.d.ts`), and multilingual documentation across 5 languages.

---

## 概述 (Overview)

Amber **v1.6.0** 是围绕**自治 AI Agent 生产级可靠性、边缘侧模型极速载入与企业级零信任权限管控**打造的里程碑版本。本版本带来了三大核心模块以及全方位的配套生态：

1. **确定性 Agent 回放引擎 (`amber:replay` & CLI `amber record` / `amber replay`)**：
   - **全链路非确定性输入捕获**：在 Agent 执行过程中自动拦截并记录用户输入、单步工具调用返回值、随机种子、虚拟时间戳与外部数据。
   - **紧凑可移植轨迹落盘**：生成标准 `.amber-trace.json` 文件，包含完整元数据与结构化事件流。
   - **离线零网络零模型重放**：在不连接外部模型 API、不消耗 Token、不连外网的环境下百分之百精准复现历史 Agent 行为。
   - **自动化单步分歧检测 (Step Divergence Detection)**：当重放过程中的代码逻辑与录制时的入参或状态发生漂移时，引擎自动阻断并准确定位逻辑分歧点。

2. **原生 GGUF / SafeTensors 模型权重加载器 (`amber:weights` / `amber:ai.weights`)**：
   - **原生二进制格式快速解析**：无需外部 C++ Addon 或 Python 绑定，直接解析 GGUF (v2/v3) 与 HuggingFace SafeTensors 文件头部元数据，耗时小于 1ms。
   - **零拷贝内存映射张量切片**：基于 Rust `mmap` 系统调用，将任意大尺寸权重文件切片直接映射到 `ArrayBuffer`，零 V8 堆冗余拷贝。
   - **直接互通 `amber:ai.Tensor`**：通过 `Tensor.fromBuffer` 或 `weights.loadTensor` 将权重直通用于矩阵乘法（`matmul`）、向量投影与注意力计算。

3. **企业级能力安全控制 (`amber:security` / `amber:permissions`)**：
   - **对齐 W3C 规范的动态权限自省**：支持 `permissions.query()`、`permissions.has()` 与 `permissions.list()`，允许 Agent 在发起敏感 I/O 前预判权限状态。
   - **运行时特权主动撤销 (`permissions.revoke`)**：在完成敏感密钥读取或配置拉取后，支持主动永久废除文件读取、公网访问或环境变量读取等能力。
   - **沙箱策略生成与最小特权衰减 (`createSandboxPolicy` & `attenuate`)**：提供数学级策略收敛能力，在分发工作给不可信子 Agent 或插件时，严格计算两者的交集约束，杜绝特权越界。

4. **类型系统与全语种文档生态**：
   - `types/amberjs.d.ts` 与 `src/types_export.rs` 完整纳入 `amber:replay`、`amber:weights`、`amber:security` 与 `amber:permissions` 声明。
   - 官方文档新增三大专题中英双语技术指南。
   - 官方网站导航同步支持英语、中文、西班牙语、法语、印地语 5 种语言。

---

## 模块新特性深度解析

### 1. 确定性 Agent 回放 (`amber:replay`) 与 CLI 录制

```bash
# 1. 录制 Agent 执行轨迹至 search.amber-trace.json
$ amber record -o search.amber-trace.json agent.ts

# 2. 离线重放已录制的轨迹（无模型调用与网络依赖）
$ amber replay --verify search.amber-trace.json
```

编程式 JavaScript / TypeScript 控制：

```typescript
import { startRecording, stopRecording, step } from 'amber:replay';

startRecording({ script: 'agent.ts', outputPath: 'trace.json' });

// 录制阶段执行生成逻辑，重放阶段直接注入历史返回值
const plan = step("plan_generation", "分析财报", (query) => {
  return { steps: ["下载 PDF", "计算同比增长", "生成图表"] };
});

const trace = stopRecording('trace.json');
```

---

### 2. 原生 GGUF / SafeTensors 模型加载 (`amber:weights`)

```typescript
import { readGGUFMetadata, readSafeTensorsMetadata, loadTensor } from 'amber:weights';
import { Tensor } from 'amber:ai';

// 1. 亚毫秒级探测模型元数据
const meta = readSafeTensorsMetadata("./models/model.safetensors");
console.log(`Tensors: ${meta.tensors.length}`);

// 2. 零拷贝提取权重张量并与 amber:ai 互通
const loaded = loadTensor("./models/model.safetensors", "model.embed_tokens.weight");
const weightTensor = Tensor.fromBuffer(loaded.buffer, loaded.shape, 'float32');

// 开展高性能矩阵计算
const input = Tensor.ones([1, loaded.shape[0]]);
const output = input.matmul(weightTensor);
console.log('Projected shape:', output.shape);
```

---

### 3. 企业级能力安全与特权衰减 (`amber:security` & `amber:permissions`)

```typescript
import { permissions, createSandboxPolicy, attenuate } from 'amber:security';

// 1. 检查能力可用性
if (permissions.has({ name: 'read', path: '/etc/config.json' })) {
  // 读取核心配置
}

// 2. 敏感操作完成后动态撤销特权
permissions.revoke({ name: 'read', path: '/run/secrets' });

// 3. 构建子 Agent 受限策略衰减
const supervisor = createSandboxPolicy({
  allowRead: ['/app/data', '/tmp'],
  allowNet: ['api.openai.com']
});
const subAgentConstraint = createSandboxPolicy({
  allowRead: ['/tmp'],
  denyNet: true
});
const workerPolicy = attenuate(supervisor, subAgentConstraint);
// workerPolicy 只拥有两者交集: allowRead: ['/tmp'], denyNet: true
```

---

## 验证与测试套件

- **集成测试套件**：
  - `tests/v1_6_0_replay_tests.rs`：测试轨迹录制、离线回放、模块导出与单步分歧拦截（4/4 通过）。
  - `tests/v1_6_0_weights_tests.rs`：测试 GGUF 与 SafeTensors 头部解析、张量切片与 `Tensor.fromBuffer` 互操作（2/2 通过）。
  - `tests/v1_6_0_security_tests.rs`：测试动态权限查询、规则导出、特权撤销与策略衰减（4/4 通过）。
- **静态类型与代码质量**：
  - `cargo test --lib types_export` 通过。
  - `website` 生产级构建测试成功（TypeScript 与 Markdown 语法全部校验通过）。
