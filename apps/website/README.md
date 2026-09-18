# Amber Website

Official Amber website built with React, Vite, and Tailwind CSS. The site is deployed to Cloudflare Workers static assets through Wrangler.

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

`get.amberjs.com` is a custom domain on the same Worker as `amberjs.com` (`apps/website/wrangler.toml`). There is no GitHub Actions deploy job; publishing is a local Wrangler deploy (needs a Cloudflare account with access to this Worker):

```bash
pnpm run deploy:dry-run
pnpm run deploy
```

`wrangler.toml` serves `dist/` as static assets and uses `single-page-application` fallback so direct links such as `/docs/installation` work on Cloudflare.

`src/worker.ts` serves `/install.sh` and `/install.ps1` from the R2 bucket `amber-dist` **when that object exists**, otherwise from the Worker static assets. Live `get.amberjs.com/install.sh` currently matches the asset-hosted copy (`content-type: application/x-sh`, `max-age=0`), so a Wrangler deploy updates the installer. If R2 later has `install.sh` / `install.ps1`, it shadows the deploy — also upload:

```bash
npx wrangler r2 object put install.sh --file=../../install.sh --bucket=amber-dist --remote --content-type text/plain
npx wrangler r2 object put install.ps1 --file=../../install.ps1 --bucket=amber-dist --remote --content-type text/plain
```
