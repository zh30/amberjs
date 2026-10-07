# `amber run --export-tools` contract

This is the user-facing contract for Stable `amber run --export-tools`. It is derived from `src/main.rs`, `src/agent.rs`, and `tests/export_tools_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

The command prints tool schemas as JSON and exits. It does not execute the module, does not call a model, and does not use Cargo feature `ai`. `amber session` and `amber mcp` are separate Stable commands. They still load schemas with `export_tools_from_entry`. This command is the one that prints them and applies the print-time checks below.

## Command

```bash
amber run [--sandbox] [--permission-policy <file>] [--deny-fs] [--audit-log <file>] --export-tools <file> [args...]
```

- `<file>` is one path. It must exist and be a regular file. A directory is rejected.
- Exit status is 0 when schemas are printed. Every contracted failure below is non-zero and prints a stderr line that starts with `error: amber run --export-tools:`.
- Missing CLI arguments are clap usage errors. The prefix applies once the subcommand is running.
- Stdout on success is pretty-printed JSON (two-space indent) and a trailing newline. Stderr does not contain the error prefix.
- `[args...]`, `--watch`, `--inspect`, `--inspect-brk`, `--preload`, and `--require` are ignored. The process does not stay up, does not bind an inspector port, and does not load preloads.
- A missing `<file>` is not a `package.json` script. `amber run --export-tools build` does not run `scripts.build`.

## What is read

| Input | Used for |
| :--- | :--- |
| `<file>` | Required regular file. Read permission is checked. Bytes are scanned only when no sibling manifest is a regular file. |
| `<file's directory>/tools.json` | Optional. When it is a regular file, it is the only schema source. A directory named `tools.json` is ignored. |
| `--permission-policy` file | Optional broker rules, read by the CLI before the schema files are checked. |
| `--audit-log` file | Optional JSONL of the read checks below. |

The manifest path is the parent of `<file>` joined with the literal name `tools.json`. A `tools.json` in the process working directory is not used for an entry in a subdirectory. One manifest covers every entry in that directory.

## What is printed

```json
{
  "tools": [
    {
      "name": "echo",
      "description": "Echo the input text back to the host",
      "inputSchema": { "type": "object" }
    }
  ]
}
```

`name`, `description`, and `inputSchema` are always present. An omitted manifest description is `""`. An omitted manifest `inputSchema` is `{"type":"object"}`. Extra JSON fields on the manifest or on a tool are ignored.

### Sibling `tools.json`

When that file exists, the `tools` array is printed in file order. The entry's source is not scanned, and names are not checked against functions in the file.

Before printing, the command rejects:

- an empty `tools` array
- an empty `name`
- a repeated `name`
- an `inputSchema` that is present and is not a JSON object

`amber session` and `amber mcp` do not apply those three print-time checks. A manifest they already accept still loads. A manifest this command rejects is not a schema this command will print.

### Source scan

Used only when sibling `tools.json` is not a regular file. The entry is read as text. It is not parsed by V8 and not transpiled. TypeScript parameter types are not JSON Schema.

A tool is a trimmed line that starts with `export function ` or `export async function `. The name is the identifier immediately after that prefix. Order is source order. A repeated name keeps the first description and drops the later line.

`inputSchema` is always `{"type":"object"}`.

The description is the pending comment text joined with single spaces, or ``Exported function `<name>` `` when no comment is pending. Comment lines are trimmed lines that start with `/**`, `*`, or `//`. Text on those lines that starts with `@` is omitted (`@param` does not become a schema). Blank lines do not clear the pending comment. Any other non-empty line clears it.

A `/* */` block comment hides every line inside it, including `export function`. The line that closes the block is not scanned for an export. `/** */` is a description, not a hiding comment. An unclosed `/*` hides the rest of the file. This scan is line-oriented, not a JavaScript parser.

## What is ignored

- Executing the module. Top-level `console.log`, `require`, and filesystem writes do not run.
- `export const`, arrow functions, `export default`, `export function*`, generators, decorators, `module.exports`, and an `export` keyword split onto the next line.
- JSDoc `@param` / `@returns` and TypeScript types as JSON Schema.
- Cargo feature `ai`, `amber:ai`, and any model.
- `--watch`, `--inspect`, `--inspect-brk`, `--preload`, `--require`, and script arguments.
- `package.json` scripts when the entry path is missing.

## Permissions

`--sandbox` denies filesystem, network, environment, and process access, then allows reading `<file>` and, when it is a regular file, the sibling `tools.json`. Those two reads are checked and succeed without `--allow-read`. A policy or `--deny-fs` cannot revoke them while `--sandbox` is set, because the allow is applied after the policy. The module is not executed, so tool code does not observe the broker.

Without `--sandbox`, `--deny-fs` or a policy `deny_fs` denies the read unless `allow_read` includes the entry and, when present, the manifest. There is no automatic allow.

`--audit-log` records one JSONL object per read check (`kind` `FileSystem`, `action` `Read`, `decision` `Allow` on success). It does not record a `tool:` execution. A missing entry or a directory does not write a read check.

## Stable failure diagnostics

Every contracted failure below is non-zero and includes `error: amber run --export-tools:` on stderr. Stdout has no `tools` array.

| Case | Body contains |
| :--- | :--- |
| Missing entry | `entry file not found:` |
| Directory entry | `entry must be a file:` |
| Unreadable entry or manifest | `Failed to read` |
| Manifest is not JSON | `Failed to parse tools manifest` |
| Manifest `tools` array is empty | `empty tools array` |
| Empty tool name | `tool name must be non-empty` |
| Duplicate tool name in the manifest | `duplicate tool name` |
| `inputSchema` is not a JSON object | `must be a JSON object` |
| No manifest and no scanned export | `no exported functions found` |
| Broker denial (`--deny-fs` or a policy without `--sandbox`) | `permission denied` |
| Policy file missing | `Failed to read permission policy` |
| Policy file is not JSON | `Failed to parse permission policy` |

## Explicitly out of scope

- Calling a tool. Use `amber session` or `amber mcp`.
- Loading, prompting, or embedding a model. Models stay outside Amber.
- Turning TypeScript types or JSDoc `@param` tags into JSON Schema.
- Recognizing `export const`, arrows, default exports, or CommonJS exports.
- Checking a `tools.json` name against a function in the entry.
- A per-file manifest name other than sibling `tools.json`.
- Executing under `--sandbox`. Export does not run the file.
- `amber session` and `amber mcp` wire formats (already Stable; this command does not change them).
- Cargo feature `ai`.

## How this is enforced

```bash
cargo test --test export_tools_contract_tests -- --test-threads=1
```

CI runs that target as an explicit step. The same checkout's `tests/agent_sandbox_mcp_tests.rs` and the `session` / `export-tools` cases in `tests/cli_regression_tests.rs` keep the existing session, MCP, and example echo paths.
