# Amber 对标 Bun 优化计划

**当前主叙事（2026-09）：** Amber 是 Agent 工具沙箱，Node 够用即可。不要把「对标 Bun 的 all-in-one」当成产品目标。性能数字只来自 `benches/honest/`。`feature=ai` 不是可服务 LLM。

- 日期：2026-08-17
- 依据：[@bunjavascript](https://x.com/bunjavascript) 2026-05 至 2026-08 公开推文、[Bun 博客](https://bun.com/blog)、[Rewriting Bun in Rust](https://bun.com/blog/bun-in-rust)、Amber `docs/CURRENT_SCOPE.md` / `Cargo.toml` / `src/lib.rs` / `src/main.rs`
- 原则：只按当前可编译、可测试的事实排优先级。历史 `STAGE_*` 报告和旧性能倍数不作为目标。

## 1. 结论先说

Bun 最近不是在比谁功能清单更长，而是在做三件事：

1. **把运行时从 Zig 迁到 Rust（v1.4）**，系统性地消灭 UAF / 泄漏 / GC 生命周期 bug。
2. **把 Node.js 兼容做成 1.4 的主叙事**（官方原话：自 v1.0 以来最大的兼容性跃迁）。
3. **用一条推文一个可感知数字** 持续占领心智：启动更快、空闲 CPU 降 5x、`bun:ffi` 快 3x、修 Next.js SSR 泄漏。

Amber 已经站在 Bun 刚花大成本才到达的位置：**Rust + 引擎绑定**。引擎还是 V8，和 Node / Chrome 同族，这是中长期兼容性的真正护城河。

但当前产品事实是：Amber 是 `0.1.2`，默认路径只有 `eval` / `run` / `repl` 算稳定；`amber serve` 仍是 `tiny_http` 固定 JSON health stub（应用服务器走 `http.createServer` + `amber run`）；`bundle` 是按行拼接本地 import；Node 符合性以 `tests/conformance/` 分数为准。**现在去抄 `Bun.Image` / S3 / HTTP/3，只会再堆一层空的阶段报告。**

要比 Bun 强，不是功能数量超过它，而是在一条更窄的产品线上做到三件事同时成立：

- 真实 npm 包能跑
- HTTP 服务能被框架用
- 权限、泄漏、符合性有公开分数，而不是口号

## 2. Bun 官方账号最近在推什么

账号定位仍是 all-in-one toolkit。2026 年 5 月到 8 月的推文节奏高度固定：`In the next version of Bun` + 一条可截图的结果。

| 时间 | 推文主题 | 真实产品信号 |
|------|----------|--------------|
| 2026-08-02 | Next.js App Router SSR 在 `fetch()` 时的 GC 保留环泄漏 | 框架兼容 + 内存正确性，不再只报微基准 |
| 2026-08-01 | 多处 backpressure 传播 bug | 流 / HTTP 正确性，生产服务器才在乎 |
| 2026-07-26 | `bun:ffi` 最高快 3x | 引擎原生 FFI，对标 V8 Fast API |
| 2026-07-23 | v1.4 启动更快 | 冷启动仍是核心 KPI |
| 2026-07-22 | **v1.4 将是自 1.0 以来最大的 Node 兼容跃迁** | 1.4 的主叙事，不是新电池 |
| 2026-07-20 | Windows 定时器精度，`Bun.sleep` 更快 | 事件循环 / 定时器正确性 |
| 2026-07-14 | 空闲 CPU 降 5x | 长驻进程成本，Agent / 服务器场景 |
| 2026-06-24 | 月下载量 | 增长叙事 |
| 2026-05-18 | Zig → Rust 重写 | 稳定性比继续加功能更优先 |
| 2026-05-02 | `Bun.Image` | 1.3 周期的“删掉工具链”电池，近期推文已退居次位 |

博客侧（1.3.x，2026-01 至 05）补全了他们已经做成的“电池包”：

- 零配置前端 / 替代 Vite 的 dev server
- 全栈打成单文件可执行，含交叉编译和签名
- 内置 SQL（Postgres / MySQL / SQLite）、Redis、S3
- `fetch` 上的 HTTP/2、HTTP/3；`Bun.serve` 上的 QUIC
- 包管理：isolated install、catalog、Security Scanner、热安装最高 7x
- 测试：`--parallel --isolate --shard`
- 2025-12 被 Anthropic 收购；Claude Code / OpenCode 已押 Bun

解读：

- **1.3 在卖电池**（SQL / Redis / Image / 前端 / compile）。
- **1.4 在卖正确性和 Node 兼容**。官方账号最近两个月几乎不再推新电池，而是推泄漏、反压、FFI、启动、空闲 CPU。
- 他们自己承认：JSC + 手工内存是崩溃和泄漏的主因。Amber 用 Rust 管 V8 handle，本来就该把这条故事讲清楚，但前提是默认路径真的不漏、不崩。

## 3. Amber 当前事实（不要用阶段报告）

来源：`docs/CURRENT_SCOPE.md`（2026-08-12）、`Cargo.toml`、`src/main.rs`。

| 层级 | 现状 |
|------|------|
| 版本 | `0.1.2`，默认 feature 为空 |
| 引擎 | `rusty_v8 = "0.22"`，升级到 0.32 / `v8` crate 必须独立分支（`docs/V8_UPGRADE.md`） |
| 稳定 | `amber eval` / `amber run` / `amber repl`，V8 执行小脚本 |
| 预览 | TS/TSX 转译 + 内容哈希缓存；`nodejs_core` / `web_api` 已安装但合同未锁死；`--watch` |
| 实验 | `test` / `bundle` / `debug` / `serve` / `init` / `create` / `add` / `install` / `bunx` / `upgrade` |
| 符合性 | `tests/conformance/` scorecard（CI 硬门禁，目标 ≥16 fixture） |
| 权限 | CLI 已有 Deno 风格 `--deny-*` / `--allow-*` / `--permission-policy`，这是 Bun 没有做成产品的面 |
| `amber serve` | `tiny_http` 对任意请求返回 `{"runtime":"amberjs","ok":true}`，HTTPS 只打印提示 |
| `amber bundle` | 按行解析静态 `import`/`export from` 后拼接，不是 bundler |
| 快照 | `src/v8_snapshot/` 存在，默认 CLI 路径未作为启动主路径 |

仓库里还有大量 feature-gated 或历史模块（`ai` / `cloud_native` / `enterprise`）。它们不是当前对标 Bun 的战场。

旧文档 `docs/REALISTIC_OPTIMIZATION_PLAN.md` 里的“启动慢 6 万倍”一类数字**禁止再引用**。任何公开性能数字必须带 commit、release/debug、硬件、命令和正确性检查。

## 4. 战略：不要在 Bun 的主场全面开战

| 轴 | Bun 现在 | Amber 现在 | 12 个月内该不该硬刚 |
|----|----------|------------|---------------------|
| Node 兼容深度 | 跑 Node 官方套件，1.4 主叙事 | 6 个基础 fixture | **必须刚**，这是采用门槛 |
| 内存 / GC 正确性 | 每天在推泄漏和反压 | 有权限和事件循环修补，缺泄漏套件 | **必须刚** |
| 启动 / 空闲 CPU | 有数字、有用户体感 | 无当前可复现数字 | **必须刚**，但先测再优 |
| 包管理速度 | 热安装、isolated、catalog | 实验性 npm lockfile v2/v3 | 先正确，再谈快 |
| HTTP 服务器 | `Bun.serve` + 路由 + WS + H3 | stub JSON / 自研 http 模块未产品化 | **必须刚到能跑框架** |
| 打包 / 前端 HMR | 有人从 Vite 迁过去 | 行拼接 | 不要自研 esbuild；接入 oxc/swc 或先不宣传 |
| 单文件 compile | 交叉编译 + 签名 | 无 | P2 以后 |
| SQL / Redis / S3 / Image | 1.3 已上 | 无 | **先不做** |
| FFI | JSC 原生 FFI，快 3x | 无 | P2，走 V8 Fast API |
| 权限模型 | 基本开放 | CLI 已有 | **做成产品差异** |
| 引擎族 | JSC（和 Safari 同族） | V8（和 Node/Chrome 同族） | **长期护城河，不要换引擎** |

产品一句话：

> Amber 是 Rust + V8 的 Node 兼容运行时：默认能跑真实 npm 包，HTTP 能被框架用，权限默认可关，每周公布符合性分数。

这比“功能比 Bun 多”更可能赢。Bun 有 Anthropic、2200 万月下载、以及已经能跑 Next.js 的兼容面。全面抄电池是必输。

## 5. 四阶段计划

### P0（4–6 周）：让默认路径值得信任

目标：一个新用户按 README 操作，不会碰到“命令在、行为是 stub”的落差。

1. **冻结对外承诺**
   - 对外只承诺 `run` / `eval` / `repl` + 已测过的 Node/Web 子集。
   - `amber serve` / `amber bundle` 在帮助文本和网站上标 Experimental，或暂时从主帮助里降级，直到行为不是 stub。
   - 任何“100% Node 兼容”注释（例如 `src/nodejs_core/mod.rs`）改成符合性分数链接。

2. **符合性分数成为唯一进度条**
   - 把 `tests/conformance/` 从 6 个 fixture 扩到第一批生产路径：
     - `fs`：read/write/stat/mkdir/readdir/watch 基础合同
     - `path` / `buffer` / `url` / `querystring` / `assert` / `events`（加深，不只 basics）
     - `timers` + `process.nextTick` + 事件循环保活
     - `crypto` 已有 AES-GCM 测试，补 HMAC / random / hash
     - `http` / `https`：client GET/POST、server listen、keep-alive 至少一条
     - `stream` 反压：这正是 Bun 8 月在修的东西
   - CI 每次跑 `./tests/conformance/run_conformance.sh`，更新 `scorecard.md`。
   - 毕业规则沿用 `CURRENT_SCOPE.md`：进 Stable 必须有可执行测试。

3. **修默认路径的正确性，而不是加 API**
   - 事件循环：定时器、`nextTick`、Promise microtask、HTTP 回调在同一套 keep-alive 里。
   - `fetch` / `http` 的 handle 生命周期：对照 Bun 8 月那条 Next.js SSR 泄漏，加“请求结束后对象可被 GC”的回归。
   - 权限：`fs` / `net` / `child_process` / `env` 全部走 `ResourceBroker`，加拒绝路径测试。

4. **`amber install` 必须能装并 `require` 一个真包**
   - 验收：`amber init demo && cd demo && amber add lodash && amber eval "console.log(require('lodash').VERSION)"` 退出码 0。
   - lockfile v3、integrity、`--frozen-lockfile` 已有骨架，补端到端测试即可，不要重写解析器。

5. **建立诚实基准，不报倍数**
   - 新增 `benches/honest/`：启动、`eval "1+1"`、读 1MB 文件、HTTP hello、空闲 10s CPU。
   - 同一台机器、同一组脚本，对比 `amber` / `node` / `bun` / `deno`。
   - 输出写 commit、二进制类型、硬件。在有这组数字之前，禁止对外说“比 Bun 快”。

P0 完成标准：

- 符合性 ≥ 30 个 fixture，且 `fs` / `http` / `timers` / `crypto` 各有真实失败过的合同
- `amber add lodash` 端到端绿
- `amber serve` 要么能执行用户脚本，要么帮助文本不再暗示它是应用服务器
- 一份带 commit 的四运行时对比表

### P1（8–12 周）：对标 Bun 的采用门槛

目标：一个中等 npm 服务（Hono 或 Express 风格）用 Amber 能 `install` + `run` + 响应 HTTP。

1. **模块系统**
   - ESM / CJS 互操作：`require` 已有，补 `import`、`import.meta`、`createRequire`、extensionless、`exports` 字段。
   - 这是跑真实包的第一阻塞，优先级高于任何新内置 API。

2. **HTTP 产品化**
   - 不要继续把 `amber serve` 当独立玩具。两条路径只留一条主路径：
     - A. `http.createServer` / `https.createServer` 达到能跑 Hono/Express 适配层
     - B. 增加 `Amber.serve({ fetch })` 作为薄封装，内部仍走同一套 listener
   - 必须有：keep-alive、streaming body、正确 status/header、WebSocket 升级、Ctrl+C 退出。
   - HTTPS：不要再打印“需要外部 TLS”。用 rustls 或现有 TLS 模块真正听端口。

3. **测试运行器毕业**
   - `amber test` 能跑本仓库 `examples/testing/` 和符合性 fixture。
   - 对齐用户已有 CLI：`--test-name-pattern` / `--bail` / `--timeout` / `--update-snapshots`。
   - 并行先正确再快；Bun 的 `--isolate --shard` 是 P2。

4. **Watch**
   - `amber run --watch` 已有 `notify` + WebSocket。验收改成：改文件后进程重启，HTTP 连接不脏读旧模块缓存。

5. **打包：接入，不自研**
   - `amber bundle` 当前实现不能宣传。
   - P1 决策：接入 oxc 或 swc 做 TS/JS/JSX 打包，或把子命令改名为 `amber concat` 直到替换完成。
   - 不要在 P1 做 CSS/HTML 全栈 bundler。那是 Bun 1.3 的主场。

6. **V8 升级独立分支**
   - 按 `docs/V8_UPGRADE.md` 开 `upgrade/rusty-v8-0.32`。
   - 不与 Node 兼容功能混在同一 PR。
   - 升级完成后才能认真做 startup snapshot。

P1 完成标准：

- 公开示例：`amber install && amber run server.ts` 跑通一个 Hono 或自研 50 行路由服务
- 符合性覆盖 `fs` / `http` / `https` / `net` / `stream` / `crypto` / `timers` / `process` / `buffer` / `path` / `url` / `events` / `zlib`
- `amber test` 在 CI 作为门禁，而不是“能跑几个 smoke”
- V8 升级分支至少能 `cargo test --lib`

### P2（3–6 个月）：在 V8 + Rust 上超过 Bun

目标：在 Bun 正在用推文防守的轴上，拿出可复现的优势。

1. **V8 Fast API 绑定**
   - Bun 7 月推 `bun:ffi` 快 3x，并点名 V8 Fast API 是 prior art。
   - Amber 应把 `fs.readFileSync`、`Buffer` 热路径、`crypto` hash、`path` 做成 Fast API / 类型化绑定。
   - 这是“比 JSC 手工 trampoline 更强”的工程故事，但只对已经正确的 API 做。

2. **启动快照进入默认 CLI**
   - Isolate 从 snapshot 创建，内置 Node/Web 绑定预热。
   - 验收：`amber eval "1"` 的 p50 启动时间写入诚实基准，连续 4 周不回退。

3. **权限成为默认产品**
   - `amber run --deny-net --allow-net=api.example.com app.js` 作为首页第二个示例。
   - 政策文件、审计日志、测试。Bun 在这条轴上几乎空白。

4. **泄漏与空闲 CPU 套件**
   - 对照 Bun 最近两条推文，建回归：
     - 1 万次 `fetch` 后 heap 回到基线
     - 空闲 30s 的 CPU < 约定阈值
     - HTTP 反压：慢客户端不爆内存
   - 有这套测试，才有资格说“比正在重写的 Bun 更稳”。

5. **Worker / isolate 池**
   - `src/web_api/worker_host.rs` 已有脚手架。做成 Cloudflare Workers 风格的预热 isolate，服务短任务和 Agent 工具调用。
   - 这是 V8 相对 JSC 的传统强项，也是 Amber `ai` feature 以后能站住的唯一理由。

6. **包管理第二跳**
   - isolated install（workspace 默认）、`amber why`、从 npm/pnpm/yarn lock 迁移。
   - 速度优化放在正确性之后：内容寻址缓存、并行解压、hardlink。

P2 完成标准：

- 诚实基准上，启动和空闲 CPU 相对 Node 有优势，相对 Bun 差距可解释（snapshot / Fast API）
- 权限示例出现在 README 和 website
- 至少 1 个真实框架（Hono 或 Fastify 子集）有每周自动兼容任务
- 公开符合性分数 ≥ 核心 Node 模块的“常用子集”，并写清未实现项

### P3（6–12 个月）：只加能缩短工具链的电池

只有 P1 的 HTTP + 模块系统已经能跑真实应用，才做下面这些。顺序按“用户少装几个包”而不是“和 Bun 功能清单对齐”。

1. `amber compile`：把脚本和运行时打成单文件。先本机，再交叉编译。不要一上来做 code signing。
2. `amber:sqlite`：用 `rusqlite`，API 对齐 `node:sqlite` / `bun:sqlite` 的小子集。
3. 统一 SQL：Postgres 用 rust 客户端，**一套 tagged template**。MySQL 更后。
4. 前端 dev server：只有在 oxc/swc bundler 已经替换行拼接之后才做 HMR。
5. FFI：`amber:ffi` 走 V8 Fast API + libloading，而不是 TinyCC。
6. Redis / S3 / Image：**默认不做**。除非有明确用户（例如 website 或内部 Agent）在用，并且核心符合性已经稳定。

P3 的否决条件：如果符合性分数连续两周下降，电池工作全部停，先修回归。

## 6. 90 天执行清单（按周）

| 周 | 产出 | 验收命令 / 证据 |
|----|------|-----------------|
| 1 | 符合性扩到 `fs` + `timers` + `process.nextTick`；删掉对外 stub 暗示 | `./tests/conformance/run_conformance.sh` |
| 2 | `amber add lodash` 端到端；lockfile 测试 | 见 P0.4 |
| 3 | `http.createServer` 最小可测合同 + keep-alive | 新 `tests/http_server_keepalive_tests.rs` |
| 4 | `fetch` / server 泄漏回归；权限拒绝路径 | 循环 1 万次后 RSS 不单调涨 |
| 5 | ESM `import` + `import.meta` + `exports` | 用真实小包（`ms` / `zod`）跑 |
| 6 | `amber serve` 改为执行入口文件，或从主帮助降级 | `amber serve examples/...` 不再返回固定 JSON |
| 7 | 诚实基准 v0，对比 node/bun/deno | `docs` 里一张带 commit 的表 |
| 8 | `amber test` CI 门禁；snapshot 更新路径 | `cargo test --test ...` + `amber test examples/testing` |
| 9–10 | Hono 或 50 行路由示例作为 weekly smoke | 独立 job，失败即红 |
| 11–12 | V8 升级分支可编译；决定 bundler 接入 oxc 还是隐藏子命令 | `cargo test --lib` on upgrade branch |

## 7. 明确不做（2026 年内）

- 再开新的 `STAGE_9x` 大而全计划，替代本文件。
- 把 `ai` / `cloudnative` / `enterprise` / `multilang` 当主产品。
- 自研 CSS/HTML bundler、Image codec、S3 SDK、HTTP/3。
- 用历史基准数字写 README。
- 在符合性分数还是 6/6 basics 时宣称“对标 Bun 100% Node API”。
- 为了推文去优化 `array.flat()` 这类引擎微基准。那是 JSC/V8 上游的事；Amber 该优化绑定和 I/O。

## 8. 组织与发布节奏（学 Bun 的传播，不学它的范围）

Bun 账号有效的不是功能多，而是：

- 一条推文只讲一个可复现结果
- 先修用户能叫出名字的框架（Next.js）
- 版本叙事清晰（1.3 电池，1.4 正确性）

Amber 对应做法：

- 每周只公开一件事：新符合性分数、或一次泄漏修复、或一次启动时间。
- 网站和 README 顶部放分数，不放阶段完成百分比。
- 版本叙事建议：
  - **0.2**：默认路径诚实 + lodash/zod 能跑 + HTTP 能听
  - **0.3**：Hono 级服务 + 测试门禁 + 权限作为产品
  - **0.4**：V8 升级 + snapshot + Fast API 热路径
  - **1.0**：才讨论 compile / sqlite / 前端 dev server

## 9. 成功定义（12 个月）

“比 Bun 强”在本计划里只允许用下面这些句子，且每句都要有命令：

1. 在公布的 Node 常用子集上，Amber 的符合性分数和失败列表比 Bun 的公开兼容页更诚实、更新更勤。
2. 带权限策略跑同一个 HTTP 服务时，Amber 能拒绝未授权的 `fs`/`net`，Bun 做不到同等粒度。
3. 诚实基准上，冷启动和空闲 CPU 相对 Node 有优势；相对 Bun 的差距能用 snapshot / Fast API 解释，而不是“调试构建”。
4. 至少一个真实框架的 weekly smoke 是绿的。
5. 用户装一个普通 npm 库再 `require`/`import`，不需要看阶段报告。

做不到这五句，就还不是比 Bun 强，只是仓库更大。
