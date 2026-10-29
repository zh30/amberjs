# 内容规格（三个方向共用，不规定长相）

输出：每个方向一个可点击的单文件 HTML，1440 宽，并在 375 上能用。页面：首页、文档、兼容性、博客、游乐场。同一套导航，当前页要标出来。

## 这一页要完成的事

一个提议，一类人，一个动作。

- 提议：一个 `amber` 二进制，跑 JavaScript 和 TypeScript，稳定能力写成契约。
- 人：正在评估运行时的工程师。
- 动作：复制安装命令。首屏不再并列第二个按钮。兼容性放在导航里。

版式类型：A。经典首屏加分段。产品能从一条命令和一个终端看懂。文档方向把「首屏」收成手册的第一栏，这是那个方向的胆量，不是另一套文案。

## 文案

标题（在意思断开的地方换行，最宽约 680px）：

一个二进制，
跑 JavaScript 和 TypeScript。

副题：给正在评估运行时的工程师。脚本、测试、本地服务和 Agent 工具，从下面这条安装命令开始。每一项稳定能力都有书面契约，契约同时写明缺什么。

证明，放在命令旁边，不放成大数字：v1.17.0。MIT 许可。

中段大字（至少两行，和首屏分开）：

跑这个文件。
先读契约，再依赖它。

好处：

- 直接跑 TypeScript。类型被擦掉，堆栈指回你的 .ts 行号。不做类型检查。
- 测试写成 describe、test、expect。amber test 会发现它们。--watch 是整进程重跑，不是热替换。
- amber serve 用 HTTP/1.1 跑一个 fetch 处理函数。加上 --https 就用 rustls。它不是通用 Web 服务器。
- 沙箱默认拒绝文件、网络、环境变量和子进程。放行要写到具体路径或主机，并留下 JSONL 审计。
- amber run --export-tools 打印 JSON schema 后退出。它不执行工具文件，也不调用模型。

三步：

1. 装上这个二进制。
2. amber run 一个文件，或进入 repl。
3. 依赖某个模块之前，打开它的契约。

风险：不要账号。MIT 许可。卸掉就是删掉二进制。预览和实验会标明，避免误当成稳定承诺。

不支持（短名单，来自 CURRENT_SCOPE，不抄已经过期的站点非目标表）：

- 不是 Node 的替代品。
- 不做 TypeScript 类型检查。
- amber install 不是 npm、yarn 或 pnpm。
- 不发布没有提交、命令、硬件和正确性校验的性能数字。
- amber test --parallel 以退出码 2 拒绝。

常见问题：

- Amber 能换掉 Node 吗？不能。每个模块只承诺契约里写明的子集。
- 稳定是什么意思？有书面契约，并且有测试钉住。事实来源是 docs/CURRENT_SCOPE.md。
- 会做类型检查吗？不会。只擦除类型。
- 有性能数字吗？目前不发布。仓库里旧文章的性能标题不是当前承诺。
- Windows 能装吗？能。用 install.ps1，发布页也有 zip。
- amber install 是 npm 吗？不是。它安装 package.json 里的依赖。add、remove、prune、upgrade、init、x 仍是实验。
- 要注册吗？不要。
- 不安装能试吗？站点有游乐场。这个原型不执行代码。安装仍是主路径。

记分卡：tests/conformance/scorecard.md（2026-09-24）56 项均为 PASS。这是回归门禁，不是无缝替换 Node 的证明。链接到 GitHub 上的该文件。

博客只列仓库里的中文标题，并写明性能口号不是当前承诺：

- Amber v0.1 公开发布范围说明
- Amber v0.1.2 Agent 工具沙箱
- Amber v0.4.3 发布：底层架构全面突破，登顶 Node 与 Bun 关键性能王座
- Amber v1.0.0 正式发布：首个生产级版本，开启 AI-Native 边缘运行时新纪元
- 全站多语言上线与未来3年技术拓扑路线图发布
- 发布前的运行时代码梳理与重构
- Amber v1.16.0：Wasm 2.0、打包，以及 Web 热路径

文档页给出真实目录分组：开始、指南、兼容、Agent、实验。安装页只重复上面的三条安装命令。

外链只用这些，不要用空锚点：

- https://github.com/zh30/amberjs
- https://github.com/zh30/amberjs/blob/main/docs/CURRENT_SCOPE.md
- https://github.com/zh30/amberjs/blob/main/tests/conformance/scorecard.md

索引建议：首页、文档、兼容性、安装说明要被索引。标题示例：Amber，一个二进制跑 JavaScript 和 TypeScript。说明：给评估运行时的工程师。安装 amber，并阅读每项稳定能力的契约。

## 假设

用户这次要先看方向，不先改生产站点。中文为主。没有客户证言，就不做证言。落地页视觉系统里的字号阶梯、间距阶梯、平涂背景、命令用等宽字，三个方向都遵守。骨架必须不同，所以不把三个方向都收成同一套蜜色首屏。
