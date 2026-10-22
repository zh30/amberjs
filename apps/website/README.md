# Amber Website

Official Amber website built with React, Vite, and Tailwind CSS. The site is deployed to Cloudflare Workers static assets with the Cloudflare CLI (`cf`).

## Development

```bash
pnpm install --frozen-lockfile
pnpm run dev
```

## Production Build

```bash
pnpm run build
```

The production bundle is written to `dist/`. The `prebuild` step copies repo-root `install.sh` / `install.ps1` into `public/` so Cloudflare assets include the installer. The canonical install host is `https://get.amberjs.com`.

## Cloudflare Deploy

`get.amberjs.com` is a custom domain on the same Worker as `amberjs.com` (`apps/website/cloudflare.config.ts`). `wrangler.toml` is kept for rollback and is not read by `cf`.

Production deploys run through `.github/workflows/deploy-website-cf.yml`. Pull requests run `cf build` and `cf deploy --prebuilt --mode production --dry-run` with no credentials. A push to `main` deploys that same build when `apps/website/**`, `install.sh`, `install.ps1`, or the workflow file change. The only step that receives repository secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` is the real deploy. `cf` is in beta.

Local dry-run (no Cloudflare login):

```bash
pnpm run deploy:dry-run
```

That creates `dist/` if needed, runs `cf build --mode production`, then `cf deploy --prebuilt --mode production --dry-run`. `pnpm run deploy` needs an API token. The Wrangler bundler records no mode unless `--mode` is passed, so build and deploy both use `production`.

### Rollback to Wrangler

1. In GitHub, open **Actions → Deploy website (Wrangler rollback) → Run workflow** on `main`.
2. That workflow is `workflow_dispatch` only, so it will not deploy on push while the `cf` workflow is active.
3. It publishes with `pnpm run deploy:wrangler` (`wrangler deploy --config wrangler.toml`) and uploads the installers with Wrangler. Do not turn its `push` trigger back on while `deploy-website-cf.yml` also deploys.

`cloudflare.config.ts` serves the Vite `dist/` directory as static assets (`assetsDirectory` in `wrangler.config.ts`) and uses `single-page-application` fallback so direct links such as `/docs/installation` work on Cloudflare.

`src/worker.ts` serves `/install.sh` and `/install.ps1` from the R2 bucket `amber-dist` **when that object exists**, otherwise from the Worker static assets. If R2 has `install.sh` / `install.ps1`, it shadows the Worker assets. The production workflow uploads both scripts to `amber-dist` after `cf deploy` so `get.amberjs.com` stays in sync:

```bash
pnpm exec cf r2 objects put install.sh --bucket-name amber-dist --file ../../install.sh --content-type "text/plain; charset=utf-8"
pnpm exec cf r2 objects put install.ps1 --bucket-name amber-dist --file ../../install.ps1 --content-type "text/plain; charset=utf-8"
```
