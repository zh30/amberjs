# Beejs Website

Official Beejs website built with React, Vite, and Tailwind CSS. The site is deployed to Cloudflare Workers static assets through Wrangler.

## Development

```bash
pnpm install --frozen-lockfile
pnpm run dev
```

## Production Build

```bash
pnpm run build
```

The production bundle is written to `dist/`. The `prebuild` step syncs `../install.sh` to `public/install.sh` so the official install URL is included in the Cloudflare assets.

## Cloudflare Deploy

```bash
pnpm run deploy:dry-run
pnpm run deploy
```

`wrangler.toml` serves `dist/` as static assets and uses `single-page-application` fallback so direct links such as `/docs/installation` work on Cloudflare.
