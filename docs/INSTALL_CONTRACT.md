# `amber install` contract

This is the user-facing contract for Stable `amber install`. It is derived from `src/main.rs`, `src/package_manager.rs`, and `tests/install_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`amber install` installs **direct** dependencies declared in the current directory's `package.json` from the public npm registry. It is not npm, not Yarn, and not pnpm.

## Command

```bash
amber install [--frozen-lockfile]
```

- The project root is the process working directory. There is no `--prefix`, `--registry`, or `--omit`.
- `--frozen-lockfile` checks the lock before creating `node_modules` or `.amberjs_cache`, then installs without rewriting `package-lock.json`.
- Without `--frozen-lockfile`, a successful install rewrites `package-lock.json` from the packages that landed in `node_modules`.
- Exit status is non-zero on every contracted failure below. Contracted failures print a stderr line that starts with `error: amber install:`.
- Missing CLI flags are clap usage errors. The prefix applies once the subcommand is running.
- Registry access uses `curl` against `https://registry.npmjs.org/`. `.npmrc` is not read. `curl` must be on `PATH` when a download is required.

## What is read

| Input | Used for |
| :--- | :--- |
| `./package.json` | Required. `name` and `version` must be strings. `dependencies`, `devDependencies`, and `optionalDependencies` are string maps. |
| `./package-lock.json` | Optional, unless `--frozen-lockfile`. Only the top-level `dependencies` object is a pin (npm lockfileVersion 1 / 2 shape). Each entry's `version`, `resolved`, and `integrity` are read. |
| `https://registry.npmjs.org/<name>` | Package metadata and tarball URL, when a direct dependency is not rejected by a lock pin first. |

`devDependencies` are always installed. `NODE_ENV` and `--omit=dev` are not consulted.

A lock pin applies only to a **direct** name listed in those three maps. Nested `dependencies` inside a lock entry are not applied to transitive installs. The `packages` map (the npm lockfileVersion 3 shape) is ignored.

## What is verified

### Frozen lockfile (`--frozen-lockfile`)

Before any registry access, cache directory, or `node_modules` directory is created:

1. `package-lock.json` must exist, be readable, and parse as JSON with a `dependencies` object (missing means an empty pin map).
2. Every string dependency in `dependencies`, `devDependencies`, and `optionalDependencies` must have an entry in that map.
3. The requested range must match the locked `version` (rules below).

`lockfileVersion` is not a pass/fail gate. A version 1 file with a `dependencies` entry is still checked. A version 3 file that only has `packages` does not satisfy the check.

This check does **not** download tarballs and does **not** hash them. A later optional-dependency download failure still does not fail the command (see Ignored).

### Lock pin during install

When a direct `dependencies` or `devDependencies` entry has a lock pin:

- The request must match the locked version, or install fails with `package-lock.json version mismatch` before registry range resolution.
- The locked version string is what gets downloaded (not a newly resolved range).
- If the lock `resolved` URL is non-empty, it must equal the registry metadata tarball URL.
- If both the lock `integrity` and the registry metadata `integrity` are non-empty, they must be the same string.
- The tarball bytes are hashed before unpack. A lock `integrity` is the hash input when it is non-empty (registry `shasum` is not used in that case). Otherwise registry `integrity` is used, then registry `shasum`.

`optionalDependencies` use the same pin when the frozen pre-check has passed, but a failure while installing an optional package does not fail the command.

### Tarball integrity

Checked by the installer before the archive is unpacked. A mismatch deletes the cached `.tgz` and does not extract it.

| Material | Rule |
| :--- | :--- |
| SRI `integrity` | Whitespace-separated `alg-base64` tokens. The first token whose digest matches wins. Accepted algorithms: `sha512`, `sha384`, `sha256`, `sha1`. npm packages normally use `sha512`. |
| `shasum` | Lower-case or mixed-case hex SHA-1. Used only when no integrity string was selected. |
| Neither | `refusing untrusted tarball`. The package is not unpacked. |

An empty integrity string is treated as absent.

### Request vs locked version

| Request | Matches locked version when |
| :--- | :--- |
| `*` or `latest` (any ASCII case) | Always. |
| identical string | Always. |
| `=X` | Locked version equals `X` after trim. |
| `^X` | Numeric triplet of the locked version is `>= X`, and the npm caret major rule holds (`^1.2.3` stays on major 1; `^0.2.3` stays on `0.2`; `^0.0.3` stays on `0.0.3`). |
| `~X` | Same major and minor, and locked `>= X`. |

The triplet is `major.minor.patch`. A leading `v` is stripped. Text after `-` or `+` is stripped before compare (`1.2.3-beta.1` compares as `1.2.3`). Omitted minor or patch becomes `0`.

These are **not** full semver: `||`, hyphen ranges, `1.x`, `1.*`, and whitespace ranges do not match.

### Without a lock entry

`--frozen-lockfile` fails. A normal install resolves against the registry:

| Request | Resolution |
| :--- | :--- |
| `latest` | `dist-tags.latest` (exact string; `LATEST` is not this tag). |
| `^`, `~`, `>=`, `>`, `<=`, `<` | The last `versions` key in registry JSON order that passes a numeric filter. This is **not** semver-max. |
| anything else | Used as an exact version key. |

Exact versions and lock pins are the reproducible Stable path. Unlocked ranges are best-effort.

## What is ignored

- `peerDependencies` (not installed, not part of the frozen check)
- `scripts`, including `preinstall`, `install`, `postinstall`, and `prepare` (not executed)
- `workspaces`, `overrides`, `resolutions`, `bundledDependencies`, `engines`, `os`, `cpu`, `packageManager`
- `yarn.lock`, `pnpm-lock.yaml`, `npm-shrinkwrap.json`
- The lockfile `packages` object and any nested lock `dependencies` tree
- `.npmrc`, custom registries, and auth tokens
- Install errors for `optionalDependencies`, including integrity failures after the frozen name/version check
- `NODE_ENV=production` (devDependencies are still installed)

A non-frozen install that skips a failed optional dependency still rewrites `package-lock.json` from `node_modules`, so that optional pin can disappear from the generated lock.

## On-disk result

| Path | Contents |
| :--- | :--- |
| `node_modules/<name>/` | Extracted package. Scoped names keep the slash (`@scope/pkg`). Archive entries must stay under the tarball's `package/` prefix. |
| `node_modules/<name>/.amberjs-integrity.json` | `integrity` and `resolved` recorded for the direct install. |
| `.amberjs_cache/<name>/<version>.tgz` | Tarball cache in the project directory. Not a global store and not a symlink farm. |
| `package-lock.json` | Rewritten on non-frozen success as `lockfileVersion` 3 with a top-level `dependencies` map. `packages` is not written. `dev` is always `false`. Not rewritten in frozen mode. |

Transitive dependencies are installed from each package's own `dependencies` with **no lock pin**, into the same top-level `node_modules`. The first-seen name wins. There is no npm hoisting algorithm and no peer-dependency solver. `.bin` shims are not created.

## Stable failure diagnostics

Every contracted failure below is non-zero and includes `error: amber install:` on stderr.

Frozen checks fail before `node_modules` and `.amberjs_cache` exist. Invalid JSON fails before those directories too. A JSON file that is missing string `name` or `version` fails in the typed parser after empty `node_modules` and `.amberjs_cache` directories have been created; it still does not write `package-lock.json`. A lock pin that fails inside install may leave those empty directories, because the installer creates them before walking dependencies. It does not unpack a package whose pin or hash failed, and it does not rewrite the lock on failure.

| Case | Body contains |
| :--- | :--- |
| Missing `package.json` | `package.json not found` |
| Unreadable `package.json` | `Failed to read package.json` |
| Invalid JSON `package.json` | `Failed to parse package.json` |
| JSON `package.json` missing string `name` or `version` | `Failed to parse package.json` |
| Frozen mode, no lockfile | `frozen lockfile requires package-lock.json to exist` |
| Frozen mode, non-string dependency | `frozen lockfile cannot validate non-string dependency` |
| Frozen mode, direct name absent from lock `dependencies` (including a `packages`-only lock) | `frozen lockfile mismatch for package` and `missing from package-lock.json` |
| Frozen mode, request does not match locked version | `frozen lockfile mismatch for package` and `package.json requests` |
| Direct or dev dependency lock version mismatch during install | `package-lock.json version mismatch for package` |
| Lock `resolved` differs from registry metadata | `package-lock.json resolved mismatch for package` |
| Lock `integrity` differs from registry metadata | `package-lock.json integrity mismatch for package` |
| Tarball SRI digest mismatch | `Package integrity mismatch` |
| Tarball SHA-1 shasum mismatch | `Package shasum mismatch` |
| No integrity and no shasum | `refusing untrusted tarball` |
| SRI algorithm outside the accepted list | `Unsupported package integrity algorithm` |
| Permission broker denial | `permission denied` (plus the prefix) |

Install-time package failures are wrapped as `Failed to install dependencies:` and still include the body above. Frozen failures are reported directly, before that wrapper.

Permission denials and network failures do not write `package-lock.json`.

## Explicitly out of scope

- Replacing npm, Yarn, pnpm, or `bun install`
- Workspaces, `workspace:` specifiers, and multi-package repositories
- Lifecycle scripts and package-manager plugins
- `peerDependencies`, overrides, bundled dependencies, and `.bin` links
- git, `file:`, `link:`, `github:`, and URL specifiers as local installs
- Reading `packages` from an npm lockfileVersion 3 file, or reproducing npm's transitive tree from that file
- Yarn and pnpm lockfiles
- A global content-addressed cache, hard links, or a symlink `node_modules`
- Optional-dependency hard failures (native addons such as `fsevents` may be skipped)
- Semver-max range resolution for an unlocked range
- `amber add`, `amber remove`, `amber prune`, `amber upgrade`, `amber init`, and `amber x` (those commands stay Experimental)
- `amber bundle` and `amber compile` (separate Stable contracts)
- Year-2 sandbox / N-API / polyglot / io_uring work

## How this is enforced

```bash
cargo test --test install_contract_tests -- --test-threads=1
```

CI runs that target as an explicit step. The happy path covered there is an install with no registry dependencies (including ignored peers, scripts, and other lockfiles) plus frozen success that does not rewrite the lock. Integrity, lock-metadata mismatch, and lock version mismatch do not contact the registry.
