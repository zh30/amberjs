# `amber compile` SEA contract

This is the user-facing contract for Stable `amber compile`. It is derived from `src/tooling/compiler.rs`, `src/main.rs`, and the executable tests under `tests/compile_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`amber compile` builds a **host Single Executable Application (SEA)**. It copies the `amber` binary that runs the command and appends one bundled script. It is not [`pkg`](https://github.com/vercel/pkg), not [`nexe`](https://github.com/nexe/nexe), and not Bun's `bun build --compile`.

## Command

```bash
amber compile <entry> [-o|--output <file>]
```

- `<entry>` is a single file, not a directory and not a multi-entry list.
- Default output is the entry's file stem in the **current working directory** (`app.ts` → `./app`). On Windows the default name is `<stem>.exe`.
- `-o` and `--output` set the output path. Parent directories are created. An existing directory is an error. The command refuses to overwrite the Amber runtime binary it is cloning.
- There is no `--target`, `--minify`, `--sourcemap`, or cross-compile flag. The bundle inside the payload is always unminified, without a sourcemap, target comment `es2022`.
- Exit status is non-zero on every contracted compile failure. Those failures do not leave the output binary in place.
- Missing CLI arguments are clap usage errors. The `error: amber compile:` prefix applies once the subcommand is actually running.

## How the binary is built

1. Resolve the host runtime: the running `amber` / `amber.exe`, or a sibling with that name when the caller is a test harness. A renamed executable falls back to itself.
2. Reject hosts other than **linux**, **macos**, and **windows**. Cross-compilation is not implemented; the output OS and CPU are whatever that `amber` was built for.
3. Walk the entry's static module graph and bundle it with the same local resolver `amber bundle` uses on this checkout (relative paths, extension and `index.*` substitution, `node_modules/<name>/package.json` `"main"` only).
4. Copy that runtime to the output path.
5. On Linux and Windows, append the payload and trailer at the end of the file.
6. On macOS, `codesign --remove-signature` (a host that is not signed is fine), insert the payload and trailer as Mach-O segment `__AMBER` / section `__payload` immediately before `__LINKEDIT`, then ad-hoc sign with `codesign --sign - --force`. The signature is the last bytes of the file. `codesign` ships with macOS. If the header has no room for the new load command, the binary is fat, or `codesign` fails, the compile fails and the output file is removed. This signature only lets the local machine run the modified Mach-O. It is not a Developer ID or notarized distribution signature.
7. On Unix, mark the output executable (`0755`) before the macOS signature.

The result is about the size of `amber` plus the bundled script plus 32 bytes. It has the **same dynamic linker and shared libraries** as the `amber` that produced it. It is not a freestanding image, not a static musl-only blob, and not a kernel.

## Trailer layout

Bytes are appended in this order. Integers are little-endian. Nothing in the trailer is an environment variable.

```text
+--------------------------------------------------+
| host amber executable (unmodified copy)          |
+--------------------------------------------------+
| UTF-8 bundled JavaScript payload (payload_len)   |
+--------------------------------------------------+
| payload_len (u64 LE) | flags (u64 LE, must be 0) |
+--------------------------------------------------+
| magic "AMBER_STANDALONE" (16 bytes, no NUL)      |
+--------------------------------------------------+
```

| Field | Size | Rule |
| :--- | :--- | :--- |
| payload | `payload_len` bytes | UTF-8 JavaScript produced by the bundler. Empty payloads are not written. |
| `payload_len` | 8 | Byte length of the payload. `0` or a length that does not fit in the file is corrupt. |
| `flags` | 8 | Reserved. This runtime writes `0` and accepts only `0`. Any other value is an unsupported future flag. |
| magic | 16 | The ASCII bytes `AMBER_STANDALONE`. |

`AMBER_STANDALONE` is **only** this magic. Exporting an environment variable named `AMBER_STANDALONE` does not enter standalone mode and does not change `amber eval` / `amber run`.

On Linux and Windows this block is appended to an unmodified copy of `amber`, so the magic is the last 16 bytes of the file. On macOS the same block is the end of segment `__AMBER` (the copy is not byte-for-byte unmodified: the Mach-O header gains that segment and `__LINKEDIT` moves down). The ad-hoc signature is what ends the macOS file.

## Runtime recognition

On startup, before clap runs, `amber` opens its own executable (`std::env::current_exe()`):

1. File missing, unreadable, or no trailer → normal CLI.
2. Trailer location:
   - Mach-O segment `__AMBER`: the trailer is the last 32 bytes of that segment's file size. On macOS the ad-hoc code signature is after `__LINKEDIT`, so the magic is **not** the last 16 bytes of the file.
   - Otherwise, the last 16 bytes are the magic → the trailer is at EOF (Linux, Windows, and raw fixtures).
3. Magic matches and `flags != 0` → exit 1 with `error: amber standalone: unsupported trailer flags: <n>`. The CLI does not run.
4. Magic matches and `payload_len` is 0 or does not fit in front of the trailer (and, for `__AMBER`, inside that segment) → exit 1 with `error: amber standalone: invalid payload length`.
5. Magic matches and the payload is not UTF-8 → exit 1 with `error: amber standalone: payload is not UTF-8`.
6. Otherwise the payload is executed and the process returns. Subcommands, `--help`, and `--version` are **not** parsed. They are script arguments.

An `__AMBER` segment whose magic does not match is also `error: amber standalone: invalid trailer` and does not fall through to the CLI. Changing trailer bytes on macOS invalidates the ad-hoc signature; the kernel rejects that file until it is signed again. The diagnostics above apply to a signature that still covers the trailer.

`process.argv` inside the payload:

| Index | Value |
| :--- | :--- |
| 0 | Absolute or resolved path of the SEA executable |
| 1 | The same path again. There is no separate script filename. |
| 2… | Arguments the user passed to the SEA binary |

A thrown exception or a rejected top-level execution prints `error: amber standalone: …` and exits 1. The status is not a JavaScript exit code.

There is no virtual filesystem. `fs`, `child_process`, and `worker_threads` paths are real host paths. A worker or child script is not pulled out of the trailer unless it was statically bundled into the one payload.

## What is embedded

Inlined when the static specifier resolves on disk:

| Kind | Extensions |
| :--- | :--- |
| JavaScript | `.js`, `.mjs`, `.cjs` |
| TypeScript | `.ts`, `.tsx`, `.mts` (transpile-only via oxc; no `tsc` check). `.tsx` may contain JSX only if the emitted `React.createElement` calls exist at runtime. |
| JSON | `.json` (emitted as `module.exports = …`) |

Unresolved **bare** specifiers (`path`, `fs`, a package name that is not installed) stay as runtime `require('…')` against the embedded Amber runtime. That is how Node builtins are used. It is not a `node_modules` tree copied into the binary.

Direct `require("literal")` and static `import` / `export from` are the static graph. Specifier discovery uses the same scan as `amber bundle`, so a `require("…")` or `import … from "…"` written as text on its own line (including some comments and strings) is treated as a specifier.

## Stable failure diagnostics

Compile failures print a line that **starts with** `error: amber compile:` on stderr. The output path is not left behind.

| Case | Body contains |
| :--- | :--- |
| Missing entry | `entry file not found: <path>` |
| Directory entry | `entry must be a file: <path>` |
| Unresolved `./` or `../` (any specifier the bundler treats as relative) | `cannot resolve '<spec>' from <file>` |
| Absolute specifier (`/…`, or a drive path on Windows) | `absolute module specifiers are not embedded` |
| Extension outside the embed list, including `.jsx` and `.css` | `unsupported module '<path>' (embedded files must be js, mjs, cjs, ts, tsx, mts, or json)` |
| `require("./addon.node")` or any specifier ending in `.node`, even when the file exists | `native addon '<spec>' is not embedded` |
| `import()` | `dynamic import() is not embedded` |
| `require(expr)` whose argument is not a string literal | `computed require() is not embedded` |
| Output path is an existing directory | `output path is a directory: <path>` |
| Output path is the runtime being cloned | `refusing to overwrite the Amber runtime binary` |
| Host OS is not linux, macos, or windows | `unsupported host OS '<os>'` |
| macOS `codesign` failure | `macOS codesign failed` |
| Bundler/TypeScript failure (missing ESM export, syntax) | `bundle failed: <underlying message>` |
| Unreadable source or output I/O | `failed to read` / `failed to create` / `failed to write trailer` |

SEA boot failures use `error: amber standalone:` (see Runtime recognition). A script that throws uses that same prefix and includes the runtime's error text.

## Platforms

| Host | Smoke | Notes |
| :--- | :--- | :--- |
| Linux | `cargo test --test compile_contract_tests` and `scripts/sea_compile_smoke.sh` on the release binary | glibc (or whatever the producing `amber` linked). Not a static pie-by-default promise beyond that binary. |
| macOS | Same contract tests when CI runs them, plus `scripts/sea_compile_smoke.sh` on the release binary | Trailer is segment `__AMBER` before `__LINKEDIT`, then ad-hoc `codesign --sign - --force`. Appending past the signature fails strict validation and is a compile error, not a silent unsigned binary. |
| Windows | `scripts/sea_compile_smoke.ps1` on `amber.exe` in the Windows smoke job | Output is a PE image with the trailer after the file. Default name ends in `.exe`. No Authenticode signing. |
| Anything else (FreeBSD, Android, WASI, …) | none | `amber compile` exits non-zero with `unsupported host OS`. It does not write a binary. |

No platform is a silent success. A host that is not in the table fails the compile.

## Explicitly out of scope

- pkg / nexe / Bun compile feature parity, virtual filesystems, asset packs, and bytecode snapshots of user code
- Cross-compilation, target triples, and producing a Linux binary from macOS or Windows (or the reverse)
- Freestanding, statically linked, or musl-only redistribution beyond the link mode of the host `amber`
- Embedding arbitrary Node native addons (`.node`, `process.dlopen`, N-API). A resolved `.node` import fails the compile
- Dynamic `import()`, computed `require(expr)`, and indirect calls such as `const r = require; r("./x.js")` (the indirect form is not detected; it is not embedded and uses the host `require` plus the real filesystem)
- `import.meta.resolve`, `createRequire`, code splitting, and watch mode
- Distribution code signing, notarization, Windows resources, icons, and version info
- A `node_modules` directory inside the binary. Only modules the static graph inlines are present, as JavaScript text
- Worker and child-process scripts that are not part of that one bundle
- `amber install` (still Preview)
- Year-2 sandbox / N-API / polyglot / io_uring work

## How this is enforced

```bash
cargo test --lib tooling::compiler::tests
cargo test --test compile_contract_tests
bash scripts/sea_compile_smoke.sh ./target/release/amber
```

Windows:

```powershell
./scripts/sea_compile_smoke.ps1 -Amber .\target\release\amber.exe
```

CI runs `compile_contract_tests` as an explicit step, and runs the smoke script on the Linux, macOS, and Windows release binaries.
