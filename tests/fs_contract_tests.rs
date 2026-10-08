//! Pins docs/FS_CONTRACT.md. Async fs settles on process.nextTick.
use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::fs;
use std::os::unix::fs::symlink;
use std::process::Command;
use tempfile::TempDir;

fn runtime() -> MinimalRuntime {
    MinimalRuntime::new().expect("Failed to create runtime")
}

fn run(code: &str) -> String {
    runtime()
        .execute_code(code)
        .unwrap_or_else(|err| panic!("Execution failed: {err}"))
        .trim()
        .to_string()
}

fn js_path(path: &std::path::Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
}

#[test]
#[serial]
fn mkdir_recursive_and_eexist() {
    let dir = TempDir::new().expect("temp dir");
    let root = js_path(dir.path());
    let code = format!(
        r#"
        const fs = require('fs');
        const root = '{root}';
        const missing = root + '/no-parent/child';
        const once = root + '/once';
        const nested = root + '/a/b';
        const file = root + '/file';
        fs.writeFileSync(file, 'x');
        const codes = [];
        try {{ fs.mkdirSync(missing); codes.push('ok'); }} catch (e) {{ codes.push(e.code + ':' + e.syscall); }}
        fs.mkdirSync(once);
        try {{ fs.mkdirSync(once); codes.push('ok'); }} catch (e) {{ codes.push(e.code); }}
        fs.mkdirSync(nested, {{ recursive: true }});
        fs.mkdirSync(nested, {{ recursive: true }});
        try {{ fs.mkdirSync(file, {{ recursive: true }}); codes.push('ok'); }} catch (e) {{ codes.push(e.code); }}
        codes.concat([fs.existsSync(nested), fs.statSync(nested).isDirectory()]).join('|');
        "#
    );
    assert_eq!(run(&code), "ENOENT:mkdir|EEXIST|EEXIST|true|true");
}

#[test]
#[serial]
fn stat_fields_follow_links_and_lstat_reports_symlink() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("note.txt");
    fs::write(&file, "abcdef").expect("seed");
    let link = dir.path().join("note.link");
    symlink(&file, &link).expect("symlink");
    let code = format!(
        r#"
        const fs = require('fs');
        const file = '{file}';
        const link = '{link}';
        const st = fs.statSync(file);
        const followed = fs.statSync(link);
        const lst = fs.lstatSync(link);
        const modeOk = (st.mode & 0o170000) === 0o100000 && (st.mode & 0o777) === (st.mode & 0o777);
        [
          modeOk,
          st.mtime instanceof Date,
          typeof st.mtimeMs === 'number',
          Math.abs(st.mtime.getTime() - st.mtimeMs) < 2,
          typeof st.uid === 'number',
          typeof st.gid === 'number',
          typeof st.isSymbolicLink === 'function',
          st.isSymbolicLink(),
          followed.isSymbolicLink(),
          followed.isFile(),
          lst.isSymbolicLink(),
          lst.isFile()
        ].join('|');
        "#,
        file = js_path(&file),
        link = js_path(&link)
    );
    assert_eq!(
        run(&code),
        "true|true|true|true|true|true|true|false|false|true|true|false"
    );
}

#[test]
#[serial]
fn copy_file_rm_realpath_chmod_access_and_constants() {
    let dir = TempDir::new().expect("temp dir");
    let src = dir.path().join("src.txt");
    let dest = dir.path().join("dest.txt");
    let tree = dir.path().join("tree");
    fs::create_dir(&tree).expect("tree");
    fs::write(tree.join("child.txt"), "c").expect("child");
    fs::write(&src, "copied").expect("src");
    let real = fs::canonicalize(&src).expect("canonicalize");
    let missing = dir.path().join("missing.txt");
    let code = format!(
        r#"
        const fs = require('fs');
        const src = '{src}';
        const dest = '{dest}';
        const tree = '{tree}';
        const missing = '{missing}';
        fs.copyFileSync(src, dest);
        let excl = '';
        try {{
          fs.copyFileSync(src, dest, fs.constants.COPYFILE_EXCL);
          excl = 'wrote';
        }} catch (e) {{
          excl = e.code + ':' + e.syscall;
        }}
        let rmDir = '';
        try {{ fs.rmSync(tree); rmDir = 'ok'; }} catch (e) {{ rmDir = e.code + ':' + e.syscall; }}
        fs.rmSync(tree, {{ recursive: true }});
        fs.rmSync(missing, {{ force: true }});
        let rmMissing = '';
        try {{ fs.rmSync(missing); rmMissing = 'ok'; }} catch (e) {{ rmMissing = e.code; }}
        fs.chmodSync(dest, 0o600);
        fs.accessSync(dest, fs.constants.F_OK);
        let accessMissing = '';
        try {{ fs.accessSync(missing); accessMissing = 'ok'; }} catch (e) {{ accessMissing = e.code + ':' + e.syscall; }}
        [
          fs.readFileSync(dest, 'utf8'),
          excl,
          rmDir,
          fs.existsSync(tree),
          rmMissing,
          fs.realpathSync(src),
          (fs.statSync(dest).mode & 0o777) === 0o600,
          accessMissing,
          fs.constants.F_OK === 0,
          fs.constants.COPYFILE_EXCL !== 0,
          typeof fs.constants.R_OK === 'number'
        ].join('|');
        "#,
        src = js_path(&src),
        dest = js_path(&dest),
        tree = js_path(&tree),
        missing = js_path(&missing)
    );
    assert_eq!(
        run(&code),
        format!(
            "copied|EEXIST:copyfile|EISDIR:rm|false|ENOENT|{}|true|ENOENT:access|true|true|true",
            real.display()
        )
    );
}

#[test]
#[serial]
fn fd_roundtrip_and_bad_fd() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("fd.txt");
    let code = format!(
        r#"
        const fs = require('fs');
        const path = '{path}';
        const fd = fs.openSync(path, 'w', 0o644);
        const wrote = fs.writeSync(fd, 'hello');
        fs.closeSync(fd);
        const fd2 = fs.openSync(path, 'r');
        const buf = Buffer.alloc(8);
        const got = fs.readSync(fd2, buf, 0, 8, 0);
        fs.closeSync(fd2);
        let bad = '';
        try {{ fs.readSync(99, buf, 0, 1, 0); bad = 'ok'; }} catch (e) {{ bad = e.code + ':' + e.syscall; }}
        [typeof fd === 'number', fd >= 3, wrote, got, buf.toString('utf8', 0, got), bad].join('|');
        "#,
        path = js_path(&file)
    );
    assert_eq!(run(&code), "true|true|5|5|hello|EBADF:read");
}

#[test]
#[serial]
fn readdir_with_file_types_and_write_flags() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("plain.txt");
    let link = dir.path().join("plain.link");
    fs::write(&file, "abcdef").expect("seed");
    symlink(&file, &link).expect("symlink");
    let code = format!(
        r#"
        const fs = require('fs');
        const root = '{root}';
        const file = '{file}';
        const entries = fs.readdirSync(root, {{ withFileTypes: true }});
        const plain = entries.find(e => e.name === 'plain.txt');
        const linked = entries.find(e => e.name === 'plain.link');
        fs.writeFileSync(file, 'XY', {{ flag: 'r+' }});
        fs.writeFileSync(file, 'Z', {{ flag: 'a' }});
        let unknown = '';
        try {{ fs.writeFileSync(file, 'no', {{ flag: 'nope' }}); unknown = 'wrote'; }}
        catch (e) {{ unknown = e.code; }}
        let readonly = '';
        try {{ fs.writeFileSync(file, 'no', {{ flag: 'r' }}); readonly = 'wrote'; }}
        catch (e) {{ readonly = e.code; }}
        [
          plain.isFile(),
          plain.isDirectory(),
          plain.isSymbolicLink(),
          linked.isSymbolicLink(),
          fs.readFileSync(file, 'utf8'),
          unknown,
          readonly,
          typeof fs.watch
        ].join('|');
        "#,
        root = js_path(dir.path()),
        file = js_path(&file)
    );
    assert_eq!(
        run(&code),
        "true|false|false|true|XYcdefZ|ERR_INVALID_ARG_VALUE|EBADF|undefined"
    );
}

#[test]
#[serial]
fn callbacks_and_promises_settle_on_a_later_turn() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("later.txt");
    fs::write(&file, "hello").expect("seed");
    let code = format!(
        r#"
        const fs = require('fs');
        const path = '{path}';
        let phase = 'sync';
        const callback = new Promise((resolve) => {{
          fs.readFile(path, 'utf8', (err, data) => resolve(err ? err.code : phase + ':' + data));
        }});
        const promise = fs.promises.readFile(path, 'utf8');
        const isPromise = promise instanceof Promise;
        phase = 'after-call';
        Promise.all([callback, promise.then((text) => isPromise + ':' + phase + ':' + text)])
          .then((parts) => parts.join('|'));
        "#,
        path = js_path(&file)
    );
    assert_eq!(run(&code), "after-call:hello|true:after-call:hello");
}

#[test]
#[serial]
fn system_errors_for_mkdir_stat_unlink_rename_rmdir() {
    let dir = TempDir::new().expect("temp dir");
    let missing = js_path(&dir.path().join("nope"));
    let code = format!(
        r#"
        const fs = require('fs');
        const path = '{missing}';
        function sys(fn, expectedPath) {{
          try {{ fn(); return 'ok'; }}
          catch (e) {{
            return [
              e instanceof Error,
              e instanceof TypeError,
              e.code,
              e.syscall,
              e.path === expectedPath,
              typeof e.errno === 'number' && e.errno < 0
            ].join(',');
          }}
        }}
        [
          sys(() => fs.statSync(path), path),
          sys(() => fs.mkdirSync(path + '/child'), path + '/child'),
          sys(() => fs.unlinkSync(path), path),
          sys(() => fs.renameSync(path, path + '.next'), path),
          sys(() => fs.rmdirSync(path), path)
        ].join('|');
        "#
    );
    assert_eq!(
        run(&code),
        "true,false,ENOENT,stat,true,true|true,false,ENOENT,mkdir,true,true|true,false,ENOENT,unlink,true,true|true,false,ENOENT,rename,true,true|true,false,ENOENT,rmdir,true,true"
    );
}

#[test]
#[serial]
fn exists_sync_boolean_follows_links_and_is_named_esm_export() {
    let dir = TempDir::new().expect("temp dir");
    let file = dir.path().join("present.txt");
    fs::write(&file, "x").expect("seed");
    let link = dir.path().join("present.link");
    let broken = dir.path().join("broken.link");
    symlink(&file, &link).expect("symlink");
    symlink(dir.path().join("missing-target"), &broken).expect("broken symlink");
    let missing = dir.path().join("absent.txt");
    let code = format!(
        r#"
        const fs = require('fs');
        [
          fs.existsSync('{file}'),
          fs.existsSync('{missing}'),
          fs.existsSync('{link}'),
          fs.existsSync('{broken}'),
          fs.existsSync('{dir}'),
          typeof fs.exists,
          typeof fs.promises.exists,
          typeof fs.watch
        ].join('|');
        "#,
        file = js_path(&file),
        missing = js_path(&missing),
        link = js_path(&link),
        broken = js_path(&broken),
        dir = js_path(dir.path())
    );
    assert_eq!(
        run(&code),
        "true|false|true|false|true|undefined|undefined|undefined"
    );

    let main_path = dir.path().join("exists.mjs");
    let mut runtime = runtime();
    runtime.set_main_module_path(&main_path);
    let esm = format!(
        r#"
        import {{ existsSync }} from 'fs';
        export const forceNativeModule = true;
        globalThis.__fsExistsEsm = [
          existsSync('{file}'),
          existsSync('{missing}')
        ].join('|');
        "#,
        file = js_path(&file),
        missing = js_path(&missing)
    );
    runtime.execute_code(&esm).expect("esm execute");
    let result = runtime
        .execute_code("globalThis.__fsExistsEsm")
        .expect("read esm result");
    assert_eq!(result.trim(), "true|false");
}

#[test]
#[serial]
fn esm_named_export_includes_append_file_sync() {
    let dir = TempDir::new().expect("temp dir");
    let main_path = dir.path().join("main.mjs");
    let file = dir.path().join("appended.txt");
    fs::write(&file, "A").expect("seed");
    let mut runtime = runtime();
    runtime.set_main_module_path(&main_path);
    let code = format!(
        r#"
        import {{ appendFileSync, constants }} from 'fs';
        export const forceNativeModule = true;
        appendFileSync('{path}', 'B');
        globalThis.__fsContractEsm = constants.F_OK + ':' + constants.COPYFILE_EXCL;
        "#,
        path = js_path(&file)
    );
    runtime.execute_code(&code).expect("esm execute");
    let result = runtime
        .execute_code("globalThis.__fsContractEsm")
        .expect("read esm result");
    assert_eq!(result.trim(), "0:1");
    assert_eq!(fs::read_to_string(&file).expect("read"), "AB");
}

#[test]
#[serial]
fn cli_require_fs_reaches_the_contract() {
    let binary = env!("CARGO_BIN_EXE_amber");
    let dir = TempDir::new().expect("temp dir");
    let script = dir.path().join("probe.js");
    let out = dir.path().join("out");
    fs::write(
        &script,
        format!(
            r#"
        const fs = require('fs');
        const path = '{}';
        fs.mkdirSync(path, {{ recursive: true }});
        fs.writeFileSync(path + '/a.txt', 'cli');
        console.log([
          fs.readFileSync(path + '/a.txt', 'utf8'),
          typeof fs.appendFileSync,
          typeof fs.copyFileSync,
          typeof fs.existsSync,
          fs.existsSync(path + '/a.txt'),
          typeof fs.constants.F_OK
        ].join('|'));
        "#,
            js_path(&out)
        ),
    )
    .expect("script");
    let output = Command::new(binary)
        .arg("run")
        .arg(&script)
        .output()
        .expect("amber run");
    assert!(
        output.status.success(),
        "amber run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "cli|function|function|function|true|number"
    );
}
