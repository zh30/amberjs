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

`get.amberjs.com` is a custom domain on the same Worker as `amberjs.com` (`apps/website/wrangler.toml`).

Production deploys run through GitHub Actions on the **`production`** Environment (secret `CLOUDFLARE_API_TOKEN`, variable `CLOUDFLARE_ACCOUNT_ID`). The workflow publishes on push to `main` when `apps/website/**`, `install.sh`, `install.ps1`, or the workflow file change. Tokens are not stored in the YAML.

To publish manually: **Actions → Deploy website → Run workflow**.

Local Wrangler is still available for dry-run and development (needs a Cloudflare account with access to this Worker):

```bash
pnpm run deploy:dry-run
pnpm run deploy
```

`wrangler.toml` serves `dist/` as static assets and uses `single-page-application` fallback so direct links such as `/docs/installation` work on Cloudflare.

`src/worker.ts` serves `/install.sh` and `/install.ps1` from the R2 bucket `amber-dist` **when that object exists**, otherwise from the Worker static assets. If R2 has `install.sh` / `install.ps1`, it shadows the Worker assets. The production workflow also uploads both scripts to `amber-dist` after deploy so `get.amberjs.com` stays in sync. Local equivalent (wrangler 4.x uses `{bucket}/{key}`):

```bash
npx wrangler r2 object put amber-dist/install.sh --file=../../install.sh --remote --content-type "text/plain; charset=utf-8"
npx wrangler r2 object put amber-dist/install.ps1 --file=../../install.ps1 --remote --content-type "text/plain; charset=utf-8"
```
