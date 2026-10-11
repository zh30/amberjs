//! Pins docs/FS_WATCH_CONTRACT.md — narrow Node `fs.watch` / `fs.watchFile` (G64).
//! Real notify (inotify/FSEvents) + real stat polling. No fabricated change events.

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use std::fs;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn setup() {
    amberjs::nodejs_core::fs_watch::reset_fs_watch_state();
    thread::sleep(Duration::from_millis(50));
}

fn js_path(path: &std::path::Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
}

fn close_watchers(runtime: &mut MinimalRuntime) {
    let _ = runtime.execute_code(
        r#"
        if (globalThis._testWatcher && typeof globalThis._testWatcher.close === 'function') {
            globalThis._testWatcher.close();
            globalThis._testWatcher = null;
        }
        if (globalThis._testWatchFilePath) {
            require('fs').unwatchFile(globalThis._testWatchFilePath);
            globalThis._testWatchFilePath = null;
        }
        "#,
    );
    amberjs::nodejs_core::fs_watch::reset_fs_watch_state();
    thread::sleep(Duration::from_millis(30));
}

fn pump_until<F>(runtime: &mut MinimalRuntime, mut done: F, timeout: Duration) -> bool
where
    F: FnMut(&mut MinimalRuntime) -> bool,
{
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        let _ = runtime.pump_fs_watch_events();
        if done(runtime) {
            return true;
        }
        thread::sleep(Duration::from_millis(20));
    }
    let _ = runtime.pump_fs_watch_events();
    done(runtime)
}

#[test]
#[serial]
fn watch_and_watch_file_are_functions() {
    setup();
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let out = runtime
        .execute_code(
            r#"
            const fs = require('fs');
            [
              typeof fs.watch,
              typeof fs.watchFile,
              typeof fs.unwatchFile,
              require('node:fs') === fs
            ].join('|');
            "#,
        )
        .expect("exec")
        .trim()
        .to_string();
    assert_eq!(out, "function|function|function|true");
}

#[test]
#[serial]
fn watch_emits_change_on_real_write() {
    setup();
    let dir = TempDir::new().expect("temp");
    let file = dir.path().join("watched.txt");
    fs::write(&file, "v1").expect("seed");
    let path = js_path(&file);

    let mut runtime = MinimalRuntime::new().expect("runtime");
    let code = format!(
        r#"
        const fs = require('fs');
        globalThis._events = [];
        const w = fs.watch('{path}', (type, name) => {{
          globalThis._events.push(String(type) + ':' + String(name));
        }});
        globalThis._testWatcher = w;
        [
          typeof w.close === 'function',
          typeof w.on === 'function',
          typeof w.emit === 'function'
        ].join('|');
        "#,
        path = path
    );
    let shape = runtime
        .execute_code(&code)
        .expect("watch")
        .trim()
        .to_string();
    assert_eq!(shape, "true|true|true", "FSWatcher shape: {shape}");

    // Allow inotify watch to settle, then mutate on the host.
    thread::sleep(Duration::from_millis(100));
    fs::write(&file, "v2-changed").expect("mutate");

    let ok = pump_until(
        &mut runtime,
        |rt| {
            let n = rt
                .execute_code("String(globalThis._events.length)")
                .ok()
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "0".into());
            n.parse::<usize>().unwrap_or(0) >= 1
        },
        Duration::from_secs(5),
    );
    assert!(
        ok,
        "expected at least one 'change' delivery after real write"
    );

    let detail = runtime
        .execute_code("globalThis._events.join(',')")
        .expect("detail")
        .trim()
        .to_string();
    assert!(
        detail.contains("change:") || detail.contains("rename:"),
        "eventType must be change|rename, got {detail}"
    );
    assert!(
        detail.contains("watched.txt"),
        "filename should include basename, got {detail}"
    );

    close_watchers(&mut runtime);
}

#[test]
#[serial]
fn watch_close_stops_delivery() {
    setup();
    let dir = TempDir::new().expect("temp");
    let file = dir.path().join("close-me.txt");
    fs::write(&file, "a").expect("seed");
    let path = js_path(&file);

    let mut runtime = MinimalRuntime::new().expect("runtime");
    let _ = runtime
        .execute_code(&format!(
            r#"
            const fs = require('fs');
            globalThis._n = 0;
            const w = fs.watch('{path}', () => {{ globalThis._n += 1; }});
            globalThis._testWatcher = w;
            w.close();
            'closed';
            "#,
            path = path
        ))
        .expect("close");

    thread::sleep(Duration::from_millis(80));
    fs::write(&file, "b").expect("mutate");
    thread::sleep(Duration::from_millis(200));
    for _ in 0..10 {
        let _ = runtime.pump_fs_watch_events();
        thread::sleep(Duration::from_millis(20));
    }
    let n = runtime
        .execute_code("String(globalThis._n)")
        .expect("n")
        .trim()
        .to_string();
    assert_eq!(n, "0", "closed watcher must not deliver, got {n}");
    close_watchers(&mut runtime);
}

#[test]
#[serial]
fn watch_missing_path_enoent() {
    setup();
    let dir = TempDir::new().expect("temp");
    let missing = js_path(&dir.path().join("nope.txt"));
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let out = runtime
        .execute_code(&format!(
            r#"
            const fs = require('fs');
            try {{
              fs.watch('{missing}');
              'ok';
            }} catch (e) {{
              [
                e instanceof Error,
                e instanceof TypeError,
                e.code,
                e.syscall,
                e.path === '{missing}',
                typeof e.errno === 'number' && e.errno < 0
              ].join(',');
            }}
            "#,
            missing = missing
        ))
        .expect("exec")
        .trim()
        .to_string();
    assert_eq!(
        out, "true,false,ENOENT,watch,true,true",
        "ENOENT shape: {out}"
    );
}

#[test]
#[serial]
fn watch_file_listener_on_real_write() {
    setup();
    let dir = TempDir::new().expect("temp");
    let file = dir.path().join("polled.txt");
    fs::write(&file, "tiny").expect("seed");
    let path = js_path(&file);

    let mut runtime = MinimalRuntime::new().expect("runtime");
    let _ = runtime
        .execute_code(&format!(
            r#"
            const fs = require('fs');
            globalThis._wf = [];
            globalThis._testWatchFilePath = '{path}';
            fs.watchFile('{path}', {{ interval: 40 }}, (curr, prev) => {{
              globalThis._wf.push([
                typeof curr.size,
                typeof curr.mtimeMs,
                typeof prev.size,
                curr.size
              ].join(':'));
            }});
            'armed';
            "#,
            path = path
        ))
        .expect("watchFile");

    thread::sleep(Duration::from_millis(80));
    fs::write(&file, "much-longer-content").expect("mutate");

    let ok = pump_until(
        &mut runtime,
        |rt| {
            let n = rt
                .execute_code("String(globalThis._wf.length)")
                .ok()
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "0".into());
            n.parse::<usize>().unwrap_or(0) >= 1
        },
        Duration::from_secs(5),
    );
    assert!(ok, "watchFile listener must fire after real write");

    let detail = runtime
        .execute_code("globalThis._wf[0]")
        .expect("detail")
        .trim()
        .to_string();
    assert!(
        detail.starts_with("number:number:number:"),
        "curr/prev Stats fields, got {detail}"
    );
    let size: u64 = detail.rsplit(':').next().unwrap().parse().expect("size");
    assert_eq!(size, b"much-longer-content".len() as u64);

    close_watchers(&mut runtime);
}

#[test]
#[serial]
fn watch_file_requires_listener() {
    setup();
    let dir = TempDir::new().expect("temp");
    let file = dir.path().join("x.txt");
    fs::write(&file, "x").expect("seed");
    let path = js_path(&file);
    let mut runtime = MinimalRuntime::new().expect("runtime");
    let out = runtime
        .execute_code(&format!(
            r#"
            const fs = require('fs');
            try {{
              fs.watchFile('{path}');
              'ok';
            }} catch (e) {{
              (e instanceof TypeError) + ':' + String(e.message).includes('listener');
            }}
            "#,
            path = path
        ))
        .expect("exec")
        .trim()
        .to_string();
    assert_eq!(out, "true:true");
}

#[test]
#[serial]
fn unwatch_file_stops_polling() {
    setup();
    let dir = TempDir::new().expect("temp");
    let file = dir.path().join("stop.txt");
    fs::write(&file, "a").expect("seed");
    let path = js_path(&file);

    let mut runtime = MinimalRuntime::new().expect("runtime");
    let _ = runtime
        .execute_code(&format!(
            r#"
            const fs = require('fs');
            globalThis._c = 0;
            fs.watchFile('{path}', {{ interval: 40 }}, () => {{ globalThis._c += 1; }});
            fs.unwatchFile('{path}');
            'stopped';
            "#,
            path = path
        ))
        .expect("unwatch");

    thread::sleep(Duration::from_millis(80));
    fs::write(&file, "bbbbbbbb").expect("mutate");
    thread::sleep(Duration::from_millis(250));
    for _ in 0..10 {
        let _ = runtime.pump_fs_watch_events();
        thread::sleep(Duration::from_millis(20));
    }
    let c = runtime
        .execute_code("String(globalThis._c)")
        .expect("c")
        .trim()
        .to_string();
    assert_eq!(c, "0", "unwatchFile must stop delivery, got {c}");
    close_watchers(&mut runtime);
}
