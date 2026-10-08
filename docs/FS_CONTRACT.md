# Node `fs` contract

This is the user-facing contract for the Stable subset of Node `fs` in Amber. It is derived from `src/nodejs_core/fs.rs`, `src/runtime_minimal.rs`, and `tests/fs_contract_tests.rs`. Historical `docs/STAGE_*` reports are not part of this contract.

`require('fs')`, `require('fs/promises')`, and `import` from `'fs'` or `'node:fs'` reach the same object installed by `setup_fs_api`. The default ESM export is that object. Named ESM exports include `appendFileSync` and the other sync methods listed below, plus `constants`.

This is not full Node `fs`. Methods that are not listed here are outside the contract. `fs.watch` is not implemented (`typeof fs.watch === "undefined"`).

The rest of `src/nodejs_core/` (`http`, `net`, streams, `dns`, `child_process`, and the other modules) stays Preview, except the Stable Node `path` contract in [`docs/PATH_CONTRACT.md`](PATH_CONTRACT.md), the Stable Node `events` contract in [`docs/NODE_EVENTS_CONTRACT.md`](NODE_EVENTS_CONTRACT.md), the Stable Node `buffer` / `Buffer` contract in [`docs/BUFFER_CONTRACT.md`](BUFFER_CONTRACT.md), the Stable Node `os` contract in [`docs/OS_CONTRACT.md`](OS_CONTRACT.md), the Stable Node `zlib` sync contract in [`docs/ZLIB_CONTRACT.md`](ZLIB_CONTRACT.md), the Stable `util` / `process` basics contracts, the Stable Node `crypto` subset in [`docs/NODE_CRYPTO_CONTRACT.md`](NODE_CRYPTO_CONTRACT.md) (G23), and the Stable Node `stream` subset in [`docs/NODE_STREAM_CONTRACT.md`](NODE_STREAM_CONTRACT.md) (G24).

## Stable surface

| Call | Behavior |
| :--- | :--- |
| `readFile` / `readFileSync` | Bytes. No encoding returns a `Buffer`. `utf8`, `hex`, `base64`, and `latin1` return strings. Missing path is `ENOENT` with syscall `open`. |
| `writeFile` / `writeFileSync` | Writes bytes. Default flag is truncate-and-create (`w`). |
| `appendFile` / `appendFileSync` | Appends unless `options.flag` says otherwise. |
| `mkdir` / `mkdirSync` | One directory. An existing path is `EEXIST`. A missing parent is `ENOENT`. `{ recursive: true }` creates parents and succeeds when the path is already a directory. An existing file is `EEXIST`. Syscall `mkdir`. |
| `stat` / `statSync` | Follows symlinks. `isSymbolicLink()` is false. Syscall `stat`. |
| `lstat` / `lstatSync` | Does not follow. A symlink reports `isSymbolicLink() === true`. Syscall `lstat`. |
| `Stats` | `mode` includes the Unix type bits. `mtime`, `atime`, `ctime`, and `birthtime` are `Date`s. `mtimeMs`, `atimeMs`, `ctimeMs`, and `birthtimeMs` are numbers from the same timestamps. `uid` and `gid` are numbers. |
| `readdir` / `readdirSync` | Names as strings. `{ withFileTypes: true }` returns dirents with `name`, `isFile()`, `isDirectory()`, and `isSymbolicLink()`. Dirents do not follow links. Order is the OS order. Syscall `scandir`. |
| `unlink` / `unlinkSync` | Removes a file. Syscall `unlink`. |
| `rename` / `renameSync` | `rename(2)`. Syscall `rename`. |
| `rmdir` / `rmdirSync` | Removes an empty directory. Syscall `rmdir`. |
| `copyFile` / `copyFileSync` | Copies bytes. `constants.COPYFILE_EXCL` (value `1`) fails with `EEXIST` when the destination exists. Syscall `copyfile`. |
| `rm` / `rmSync` | Removes a file or symlink. A directory without `{ recursive: true }` is `EISDIR`. `{ recursive: true }` removes a tree. A missing path with `{ force: true }` succeeds; without `force` it is `ENOENT`. Syscall `rm`. |
| `realpath` / `realpathSync` | `canonicalize`. Syscall `realpath`. |
| `open` / `openSync` | Returns a number fd starting at 3. Default flag is `r`. `mode` is the Unix permission used when the file is created. |
| `read` / `readSync` | `read(fd, buffer, offset, length, position)` writes into the `Uint8Array` / `Buffer` and returns the byte count. Callback shape is `(err, bytesRead, buffer)`. A bad fd is `EBADF`. |
| `write` / `writeSync` | `write(fd, buffer or string, ...)`. Returns the byte count. A bad fd is `EBADF`. |
| `close` / `closeSync` | Releases that fd. A bad fd is `EBADF`. |
| `chmod` / `chmodSync` | Sets the permission bits. Syscall `chmod`. |
| `access` / `accessSync` | Unix `access(2)`. `constants.F_OK` is `0`. A missing path is `ENOENT`. Syscall `access`. |
| `existsSync` | Synchronous boolean. An existing file or directory is `true`. A missing path is `false` (it does not throw). Follows symlinks: a link to an existing target is `true`; a broken link is `false`. Named ESM export. Under the permission broker, a denied read throws `TypeError` whose message contains `permission denied` before the path is checked. |
| `fs.constants` | `F_OK`, `R_OK`, `W_OK`, `X_OK`, `O_RDONLY`, `O_WRONLY`, `O_RDWR`, `O_APPEND`, `O_CREAT`, `O_EXCL`, `O_TRUNC`, `COPYFILE_EXCL`. |

`fs.promises` has the same methods without the `Sync` suffix, except there is no `promises.exists`. `promises.open` / `read` / `write` / `close` use the same numeric fds. They do not return a Node `FileHandle`.

## Write flags

`options.flag` accepts `r`, `rs`, `r+`, `rs+`, `w`, `wx`, `xw`, `w+`, `wx+`, `xw+`, `a`, `as`, `ax`, `xa`, `a+`, `as+`, `ax+`, and `xa+`.

- `w` truncates or creates.
- `a` appends or creates.
- `x` is exclusive create (`EEXIST` when the path exists).
- `r` is read-only. A write with flag `r` fails with `EBADF`.
- `r+` writes at the start and keeps the unread tail.
- Any other flag throws `TypeError` with `code === "ERR_INVALID_ARG_VALUE"` before I/O.

## Async

Callback APIs and `fs.promises` validate arguments and the permission broker on the calling turn. Permission denial throws `TypeError` whose message contains `permission denied` before any job is queued. The path is the string captured at the call. Mutating a property on the returned promise does not retarget the operation.

The I/O itself runs on a later `process.nextTick` turn, on the isolate thread. It is not a libuv threadpool. The callback or the promise settlement therefore observes state that was assigned after the `fs` call returned. A promise from `fs.promises` is a real `Promise` (`instanceof Promise`).

## System errors

Failures from `mkdir`, `stat`, `lstat`, `unlink`, `rename`, and `rmdir` throw or reject an `Error` (not a `TypeError`) with:

| Field | Value |
| :--- | :--- |
| `code` | `ENOENT`, `EEXIST`, `EACCES`, `EISDIR`, `ENOTDIR`, `ENOTEMPTY`, `EBADF`, `EINVAL`, `ENAMETOOLONG`, or `EIO` |
| `syscall` | The name in the table above |
| `path` | The path passed to the call |
| `errno` | A number less than 0 |

`readFile`, `writeFile`, `copyFile`, `rm`, `realpath`, `chmod`, `access`, `open`, `read`, `write`, and `close` use the same error shape with their own syscall names.

## Non-goals

- `fs.watch`, `fs.watchFile`, `fs.glob`, `fs.cp`, `fs.opendir`, and `fs.statfs` (not implemented; `typeof fs.watch === "undefined"`).
- `fs.exists` (callback) and `fs.promises.exists` (not installed).
- Node `FileHandle` (`fd.close()` as a method, `readableWebStream`, and the rest of that class).
- A worker threadpool. Async `fs` does not run concurrently with JavaScript on another thread.
- Sorted `readdir` results.
- Windows-specific flags and ACLs. This contract is the Unix behavior of the current host.
- When the sandbox virtual filesystem is enabled, `rmdir` removes the directory tree. On the real filesystem, `rmdir` only removes an empty directory.

## Reachability

The CLI `amber` binary installs this object from `src/runtime_minimal.rs`. Library users reach it through `amberjs::nodejs_core::fs::setup_fs_api` (`src/lib.rs` exports `nodejs_core`). The contract is not feature-gated.
