---
name: deploy-cloudflare-website
description: Cloudflare Workers Assets / Vite React 网站自动化打包构建与一键发布技能。当用户要求部署、发布或更新 Vite/React/Static 前端项目到 Cloudflare 平台时使用。
---

# Deploy Cloudflare Website Skill

本 Skill 提供了将 **Vite / React / Static SPA 静态网站** 一键部署发布到 **Cloudflare Workers & Assets** 全球边缘节点的标准自动化流程，解决 SPA 单页路由 404 降级与 OAuth 权限依赖问题。

---

## 1. 适用场景

- 将 Vite + React / Vue / Vanilla JS 等前端单页应用（SPA）部署至 Cloudflare。
- 需要使用命令行一键编译发布（`wrangler deploy`），且无需依赖 Cloudflare Pages 特定 OAuth 作用域。
- 处理 Cloudflare 上的单页应用 404 路由降级（`/index.html` fallback）。

---

## 2. 核心配置文件说明

### 2.1 创建 Worker Assets 路由文件 (`src/worker.ts`)

为了支持 Vite 单页应用（SPA）路由降级，并在不需要额外 `pages:write` 权限的情况下直接发布到 Cloudflare Workers & Assets，需在前端项目根目录下创建 `src/worker.ts`：

```typescript
interface Env {
  ASSETS: {
    fetch: (request: Request) => Promise<Response>;
  };
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    // 1. 尝试直接获取静态 Asset 资源 (CSS/JS/图片/index.html)
    const response = await env.ASSETS.fetch(request);
    if (response.status !== 404) {
      return response;
    }

    // 2. 如果返回 404，针对 SPA 路由自动降级转发至 /index.html
    const indexRequest = new Request(new URL('/index.html', url.origin), request);
    return env.ASSETS.fetch(indexRequest);
  },
};
```

---

### 2.2 配置文件 (`wrangler.toml`)

在项目根目录下配置 `wrangler.toml`：

```toml
name = "<your-website-name>"
main = "src/worker.ts"
compatibility_date = "2024-11-01"
assets = { directory = "./dist" }

[build]
command = "pnpm run build"
watch_dir = "src"
```

---

## 3. 标准构建与部署流程

### 步骤 1：前端编译构建
```bash
# 执行前端生产环境打包
pnpm run build
# 或者 npm run build
```

### 步骤 2：部署预演验证 (Dry Run)
在正式上推之前，运行 dry-run 验证配置与语法：
```bash
npx wrangler deploy --dry-run
```

### 步骤 3：正式一键发布
```bash
npx wrangler deploy
```

发布成功后，命令行会返回生产环境的全球 CDN 访问域名：
`https://amber.zhanghe.dev`

---

## 4. 故障排查 (Troubleshooting)

| 异常现象 / 错误码 | 原因分析 | 解决方案 |
| :--- | :--- | :--- |
| **Error Code 1042 / 404** | 静态 Assets 配置缺乏 Worker 入口脚本。 | 按照 `2.1` 添加 `src/worker.ts` 并设置 `main = "src/worker.ts"`。 |
| **OAuth Token missing `pages:write`** | 使用了 `pages deploy` 命令但 Token 仅有 Workers 权限。 | 改用本 Skill 推荐的 `wrangler deploy` (Worker Assets 模式)，仅需 `workers:write` 权限。 |
| **刷新页面返回 404** | 单页应用未配置 `/index.html` 降级路由。 | 确保 `src/worker.ts` 中包含了 `env.ASSETS.fetch(indexRequest)` 降级逻辑。 |
