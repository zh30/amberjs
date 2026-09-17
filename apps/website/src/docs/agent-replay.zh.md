---
title: "确定性 Agent 回放引擎 (bee:replay)"
subtitle: "离线轨迹捕获、单步时光旅行调试与自动化逻辑分歧检测"
group: "Agent & Advanced"
id: "agent-replay"
---

自治 AI Agent 深度依赖外部工具、大模型推理、动态定时器以及随机采样。当生产环境中的 Agent 做出不可预期的决策或报错时，以往几乎无法完全重现其执行上下文。

**Beejs v1.6.0 正式引入确定性 Agent 回放引擎（`bee:replay`）**与专属 CLI 工具（`bee record` / `bee replay`）。它能够捕获所有非确定性外部输入，将其序列化为紧凑的 `.bee-trace.json` 轨迹文件，并在无外网、无模型依赖的环境下百分之百精准复现历史执行。

---

## 1. 核心架构与生命周期

回放引擎分为三个核心阶段：

```
[ 正常执行 ]
     |
     v (通过 bee record 或 replay.startRecording)
[ 轨迹捕获与序列化 ]
  - 记录 Agent 步骤入参与返回值
  - 冻结虚拟时间戳与 RNG 随机种子
  - 拦截并持久化文件与网络响应
     |
     v (生成: agent_run.bee-trace.json)
[ 离线重放与确定性校验 ] (bee replay --verify)
  - 拦截 step() 调用并注入历史结果（无需调用 LLM）
  - 校验当前执行入参与历史轨迹的一致性
  - 触发分歧警告（Divergence Detection）
```

### 核心设计优势
- **脱机自给自足**：离线重放不需要配置 LLM API 密钥、数据库凭证或公网连接。
- **自动化分歧检测**：若 Agent 代码逻辑发生改动导致某一步入参发生变化，引擎立即抛出分歧错误，精确定位逻辑漂移点。
- **无缝 CLI 支持**：通过 `bee record` 与 `bee replay` 实现单命令行轨迹录制与离线验证。

---

## 2. CLI 命令行使用

### 2.1 录制 Agent 执行轨迹

执行脚本并在后台自动记录所有非确定性输入：

```bash
# 录制脚本执行并保存为默认轨迹文件 (agent.ts.bee-trace.json)
$ bee record agent.ts

# 指定自定义输出轨迹文件路径与参数
$ bee record -o traces/search_task.bee-trace.json agent.ts --query "量子计算"
```

### 2.2 离线重放与严格校验

在不消耗 Token 与无网络连接的机器上复现执行：

```bash
# 离线重放已录制的轨迹
$ bee replay traces/search_task.bee-trace.json

# 开启严格步骤入参校验与详细事件日志
$ bee replay --verify -v traces/search_task.bee-trace.json
```

---

## 3. 编程式 API (`bee:replay`)

您可以在 JavaScript / TypeScript 逻辑中灵活使用 `bee:replay` 进行细粒度控制：

```typescript
import { startRecording, stopRecording, step, isRecording, isReplaying } from 'bee:replay';

// 显式启动录制会话
startRecording({ script: 'agent_search.ts', outputPath: 'search.trace.json' });

// 标记 Agent 的核心决策与工具调用步骤
const userQuery = "解释广义相对论";
const plan = step("plan_generation", userQuery, (q) => {
  // 录制阶段：正常执行生成逻辑
  // 重放阶段：直接跳过执行，毫秒级注入历史返回值
  return { steps: ["检索物理理论", "整理数学公式", "生成摘要"] };
});

const searchResult = step("web_search", { query: plan.steps[0] }, async (input) => {
  return await fetchExternalSearch(input.query);
});

// 结束录制并将轨迹落盘
const trace = stopRecording('search.trace.json');
console.log(`成功捕获 ${trace.events.length} 个执行事件。`);
```

### 3.1 步骤分歧检测

如果本地代码改动导致提示词或参数改变：

```typescript
try {
  // 若传入参数与录制时记录的入参不一致
  step("plan_generation", "解释量子力学");
} catch (err) {
  // 抛出错误: "Step divergence detected at index 0 ('plan_generation'): input mismatch"
}
```

---

## 4. API 完整参考

| 函数 / 方法 | 返回类型 | 详细说明 |
| :--- | :--- | :--- |
| `startRecording(opts?)` | `boolean` | 初始化录制会话，支持指定脚本标识与输出路径 |
| `stopRecording(path?)` | `Trace` | 停止录制，落盘 `.bee-trace.json` 文件并返回轨迹对象 |
| `loadTrace(pathOrObj)` | `boolean` | 加载轨迹文件或 JSON 对象，并将引擎置于 `Replaying` 重放模式 |
| `step<T>(name, input, fn?)` | `T` | 录制时执行计算，重放时直接返回历史对应的快照结果 |
| `isRecording()` | `boolean` | 检查当前是否正处于轨迹录制状态 |
| `isReplaying()` | `boolean` | 检查当前是否正处于离线重放状态 |
| `getTraceStats()` | `TraceStats` | 获取当前录制/重放的统计元数据（事件数、耗时、模式等） |
| `reset()` | `boolean` | 重置引擎状态为空闲（idle） |
