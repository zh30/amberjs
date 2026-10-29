# Amber · Brand Spec

采集日期：2026-10-28
资产来源：`apps/website/public/logo.svg`、`logo.png`、`logo-b.png`；色值来自 `apps/website/DESIGN.md`
资产完整度：部分。Logo 是官方文件。没有独立的浅色反白版（`logo-b.png` 与 `logo.png` 是同一枚宝石）。没有客户截图。数字产品的界面就是终端，用站点源码里的真实命令，不画假仪表盘。

## 核心资产

### Logo

- 主文件：`assets/logo.svg`（也有 512 PNG）
- 内容：琥珀色刻面八边形宝石，深褐描边。文件本身带宝石高光，这是标志，不是页面背景。
- 用法：`<img>` 或内联这份 SVG。不重画，不改成单线，不换成别的符号。
- 禁用：拉变形、改色、加外发光当页面背景。

### 产品界面

- 主角是终端里的真实命令，不是生成的 App 截图。
- 命令来自 `apps/website/src/routes/index.tsx` 与 `PRODUCT` / `CURRENT_SCOPE`。

## 辅助资产

### 色板（现有规范，三个方向可以换骨架，不换这枚标志的颜色）

- Honey：`#F0C000`。DESIGN.md 规定它是一整块底，不是细线点缀。
- Honey 上的字：`#14120B`
- Ink：`#18181B`
- Paper：`#F5F5F4`
- 终端底：`#18181B`
- 页面背景不用渐变。宝石 SVG 内部的高光保留。

### 字

- 中文正文用 Noto Sans SC。Geist 不含中文，不能当唯一字体。
- 命令用 Geist Mono。只用于代码和命令。
- 不用 Inter、Roboto、Arial、Open Sans、Helvetica。不用斜体。字重到 700 为止。

### 禁区（DESIGN.md 与产品事实）

- 玻璃、模糊光球、赛博网格、星星、性能大数字、假 logo 墙、假评价。
- 不把旧博客的性能标题当成现在的结论。

### 气质

短、具体、把边界说清楚。工程师在决定要不要装。
