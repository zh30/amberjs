# Amber 产品事实

采集日期：2026-10-28。只写核对过的事实。设计稿不得把阶段报告或旧博客里的性能口号写成现状。

## 产品

- 品牌名 Amber，仓库与站点 Amberjs。命令是 `amber`。一个二进制。
- 实现：Rust + V8。跑 JavaScript 和 TypeScript。MIT。
- 当前站点版本号：`v1.17.0`（`apps/website/src/lib/version.ts`）。
- 事实来源：`docs/CURRENT_SCOPE.md`（2026-10-08 审阅）。站点 `scope.ts` 仍写着 G1–G69，和这份文档已经不完全同步。稿子不把「69 项」或「fs.cp 不支持」写成现状。`CURRENT_SCOPE` 把 `fs.cp` / `fs.opendir` / `fs.statfs` 记为 G76。
- 记分卡文件 `tests/conformance/scorecard.md` 生成于 2026-09-24，56 行结果全部是 PASS。仓库约定是回归门禁：PASS 低于 55，或出现 FAIL / SKIP，CI 失败。通过记分卡不是 Node 替代证明。

## 站点现在有的页面

首页、文档、兼容性、博客、游乐场。语言：中文、英文、西班牙文、法文、印地文。

## 安装（站点源码里的真实命令）

- macOS / Linux：`curl -fsSL https://get.amberjs.com/install.sh | sh`
- Windows：`irm https://get.amberjs.com/install.ps1 | iex`
- 源码：`git clone https://github.com/zh30/amberjs && cd amberjs && cargo build --release`

## 可以写进页面的能力

- `.ts` / `.tsx` 直接运行，擦除类型，堆栈指回 `.ts` 行号。不做类型检查。
- `amber test`：Jest 风格 `describe` / `test` / `expect`，自动发现。`--parallel` 以退出码 2 拒绝。
- `amber run --watch` / `amber test --watch`：上一轮退出后整进程重跑，不是热替换。
- `amber serve`：HTTP/1.1，一个 fetch 处理函数。`--https` 用 rustls。不是通用 Web 服务器。
- 沙箱默认拒绝文件、网络、环境变量和子进程。放行要写到具体路径或主机，并写 JSONL 审计。
- `amber run --export-tools`：打印 JSON schema 后退出。不执行工具文件，不调用模型。
- `amber install`：按契约安装 `package.json` 依赖。不是 npm、yarn 或 pnpm。

## 明确不写成卖点

- 不发布性能结论。旧博客标题里的「性能王座」「AI-Native 新纪元」不是当前承诺。
- 没有客户名单，不做 logo 墙，不写评价。
- 不把 Amber 写成 Node 的替代品。

## 受众与动作（来自 `apps/website/PRODUCT.md`，并标成假设）

- 受众：正在评估运行时的工程师，用来跑脚本、测试和 Agent 工具。
- 主动作：复制安装命令。
- 转化：安装，或接着打开契约。不设账号。卸掉就是删掉二进制。
