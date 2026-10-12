---
title: "Package installer"
nav: "amber install"
subtitle: "Install direct dependencies from package.json, with lockfile pins and integrity checks. Not an npm replacement."
status: "stable"
id: "package-manager"
---

`amber install` is **Stable** (G3) for the subset in [`docs/INSTALL_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/INSTALL_CONTRACT.md).

## Usage

```bash
amber install
amber install --frozen-lockfile
```

## What it does

- Reads `dependencies` and `devDependencies` from `./package.json`.
- Downloads tarballs from `https://registry.npmjs.org` into `node_modules`.
- When `./package-lock.json` has a top-level `dependencies` entry for a package, checks the version request, the `resolved` URL, and `integrity`.
- Hashes every tarball before unpacking it (SRI `sha512`, `sha384`, `sha256`, or `sha1`, else the SHA-1 `shasum`).
- `--frozen-lockfile` fails if the lock is missing or a direct request does not match it, and never rewrites the lock.

Contracted failures print `error: amber install:` and exit non-zero.

## What it does not do

`amber install` is not npm, yarn, or pnpm. These are outside the contract:

- Workspaces, lifecycle scripts (`postinstall` and friends), and `.bin` links.
- Custom registries and `.npmrc`.
- Installing `peerDependencies`.
- The lockfile `packages` map, nested lock entries, `yarn.lock`, and `pnpm-lock.yaml`.
- Lock pins for transitive dependencies. Unlocked ranges follow registry order, not the highest matching semver.

`optionalDependencies` download errors do not fail the command.

## Experimental package commands

`amber add`, `amber remove`, `amber prune`, `amber upgrade`, `amber init`, `amber create`, and `amber x` (also `dlx` and `bunx`) exist but are **Experimental**. They are outside the install contract.

```bash
amber add lodash@4.17.21   # Experimental
amber x cowsay hello       # Experimental
```
