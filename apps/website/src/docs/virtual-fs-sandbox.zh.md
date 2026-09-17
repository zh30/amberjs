---
title: "纯内存隔离 Virtual Filesystem (VFS)"
subtitle: "为不可信 Agent 生成代码提供写时复制 (COW) 与纯内存沙箱，保证宿主磁盘绝对零污染"
group: "Agent 与高级特性"
id: "virtual-fs-sandbox"
---

## 1. 为什么需要 Virtual Filesystem (VFS) 沙箱？

当运行自治 Agent、LLM 编写的代码片段或自动化测试时，脚本通常会执行：
- 生成临时构建文件与配置文件；
- 创建、读取或删除磁盘目录；
- 在意外崩溃或逻辑错误时可能破坏宿主机重要目录。

传统解决方案如 Docker 容器、轻量级 VM（Firecracker）或 chroot 隔离机制：
- **启动缓慢**：冷启动动辄数百毫秒至数秒，无法支持毫秒级 Agent 极速交互；
- **资源浪费**：需要挂载完整的根文件系统与特权环境；
- **清理繁琐**：必须编写复杂的后置脚本清理临时卷。

Beejs v1.3.0 独创了 **线程安全、轻量级的确定性 Virtual Filesystem (VFS)** 沙箱机制：
- **纯内存存储**：所有文件写入、目录创建均在 RAM 中完成，**宿主机磁盘物理上 100% 零修改**；
- **写时复制 (Copy-on-Write)**：默认允许读取宿主机的只读基础文件，写操作仅影响内存影子层；
- **纯净严格模式**：通过 `--virtual-fs-strict` 切断宿主基底只读映射，打造绝对密闭的空白虚拟盘；
- **零成本重置与快照**：支持一键导出全局 JSON 状态快照，或通过 `vfs.reset()` 在 1 微秒内复原文件系统。

---

## 2. CLI 启用方式

无需修改业务代码，直接在执行时添加标志：

```bash
# 1. 启用标准写时复制 (COW) 虚拟文件系统
bee run --virtual-fs ./untrusted_agent.js
bee eval --virtual-fs "require('fs').writeFileSync('/etc/passwd', 'hack'); console.log('Wrote to memory!')"

# 2. 启用严格纯内存模式 (完全隔离宿主磁盘读取)
bee run --virtual-fs --virtual-fs-strict ./agent_task.ts

# 3. 组合使用沙箱与虚拟文件系统
bee run --sandbox --virtual-fs ./sandboxed_script.js
```

---

## 3. JavaScript / TypeScript API (`bee:vfs`)

在程序运行时可通过 `bee:vfs` 模块动态控制沙箱生命周期：

```typescript
import fs from 'fs';
import * as vfs from 'bee:vfs';

// 1. 检查 VFS 状态
console.log("VFS 是否激活:", vfs.isEnabled());
console.log("是否处于 COW 模式:", vfs.isCow());

// 2. 在代码中动态开启
vfs.enable(true); // true = 启用 COW 基础读取, false = 严格纯内存

// 3. 执行常规 Node.js fs 同步与异步操作
fs.writeFileSync('/my_workspace/report.txt', 'Agent 生成的分析总结');
console.log("文件是否存在:", fs.existsSync('/my_workspace/report.txt')); // true
console.log("读取内容:", fs.readFileSync('/my_workspace/report.txt', 'utf8'));

// 4. 获取当前内存中驻留的全部虚拟路径
console.log("虚拟文件列表:", vfs.listFiles());

// 5. 导出文件系统快照为 JSON
const snapshot = vfs.snapshot();
console.log("已虚拟化文件字典:", snapshot.files);

// 6. 重置或退出沙箱
vfs.reset();    // 清理所有内存文件
vfs.disable();  // 退出沙箱模式
```

---

## 4. 关键安全保障

| 维度 | 传统执行 | Beejs --virtual-fs |
| :--- | :--- | :--- |
| **宿主磁盘写入** | 真实写入磁盘，存在污染与越权风险 | **100% 内存拦截，宿主物理文件零修改** |
| **初始化开销** | Docker/VM 需 500ms ~ 3s | **0.001ms 瞬时生效** |
| **测试隔离度** | 依赖手工 `rm -rf /tmp/...` 清理 | **进程退出时内存自动释放，零残留** |
| **快照与回滚** | 耗时磁盘快照或 git reset | **`vfs.snapshot()` 零拷贝毫秒导出** |
