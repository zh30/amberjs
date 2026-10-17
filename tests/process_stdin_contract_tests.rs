//! Pins docs/PROCESS_STDIN_CONTRACT.md — process.stdin real reads + events (G70).
use serial_test::serial;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(20);

fn amber() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_amber"));
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}

/// Run `amber eval <script>` feeding `stdin_bytes` to fd 0.
/// `close_stdin=false` keeps the pipe open — used to prove pause/unref
/// releases the event-loop keep-alive. Kills the child on TIMEOUT so a
/// keep-alive regression fails instead of hanging CI.
fn run_eval(script: &str, stdin_bytes: &[u8], close_stdin: bool) -> (String, String, bool) {
    let mut child = amber()
        .args(["eval", script])
        .spawn()
        .expect("spawn amber eval");
    let mut stdin = child.stdin.take().expect("stdin pipe");
    if !stdin_bytes.is_empty() {
        stdin.write_all(stdin_bytes).expect("write stdin");
    }
    if close_stdin {
        drop(stdin);
    } else {
        // Keep the write end alive in a detached thread so the child's fd 0
        // stays open; the child must exit via pause/unref, not EOF.
        std::thread::spawn(move || {
            std::thread::sleep(TIMEOUT + Duration::from_secs(5));
            drop(stdin);
        });
    }
    let deadline = Instant::now() + TIMEOUT;
    loop {
        match child.try_wait().expect("try_wait amber eval") {
            Some(_) => break,
            None => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("amber eval did not exit within {TIMEOUT:?} — keep-alive regression?");
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
    let mut stdout = String::new();
    let mut stderr = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut stdout)
        .expect("read stdout");
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .expect("read stderr");
    (stdout, stderr, true)
}

#[test]
#[serial]
fn stdin_shape_and_emitter_methods() {
    let (stdout, stderr, _ok) = run_eval(
        r#"
        const { EventEmitter } = require('events');
        const s = process.stdin;
        console.log([
          s.fd === 0,
          s.isTTY === false,
          typeof s.read,
          typeof s.resume,
          typeof s.pause,
          typeof s.ref,
          typeof s.unref,
          typeof s.setEncoding,
          typeof s.on,
          typeof s.once,
          typeof s.emit,
          s instanceof EventEmitter
        ].join('|'));
        "#,
        b"",
        true,
    );
    assert_eq!(
        stdout.lines().last().unwrap_or("").trim(),
        "true|true|function|function|function|function|function|function|function|function|function|true",
        "stdin surface mismatch: stdout={stdout} stderr={stderr}"
    );
}

#[test]
#[serial]
fn data_event_delivers_real_chunks_and_end_fires_once() {
    let (stdout, stderr, _ok) = run_eval(
        r#"
        let buf = '';
        let ends = 0;
        process.stdin.on('data', (c) => { buf += c; });
        process.stdin.on('end', () => {
          ends += 1;
          console.log('JOIN=' + JSON.stringify(buf) + '|ENDS=' + ends);
        });
        "#,
        b"hello stdin\nsecond line\n",
        true,
    );
    assert!(
        stdout.contains("JOIN=\"hello stdin\\nsecond line\\n\""),
        "must deliver real piped bytes: {stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("|ENDS=1"),
        "'end' must fire exactly once: {stdout}"
    );
}

#[test]
#[serial]
fn data_chunks_are_strings_not_buffers() {
    let (stdout, stderr, _ok) = run_eval(
        r#"
        process.stdin.on('data', (c) => {
          console.log('T=' + typeof c + '|BUF=' + (typeof Buffer !== 'undefined' && Buffer.isBuffer(c)));
        });
        process.stdin.on('end', () => console.log('END'));
        "#,
        b"abc",
        true,
    );
    assert!(
        stdout.contains("T=string"),
        "chunk must be a string: {stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("|BUF=false"),
        "chunk must not be a Buffer: {stdout}"
    );
    assert!(stdout.contains("END"), "'end' must fire: {stdout}");
}

#[test]
#[serial]
fn readable_event_then_read_returns_real_bytes() {
    let (stdout, stderr, _ok) = run_eval(
        r#"
        let got = [];
        process.stdin.on('readable', () => {
          let c;
          while ((c = process.stdin.read()) !== null) got.push(c);
          console.log('READ=' + JSON.stringify(got.join('')));
        });
        process.stdin.on('end', () => console.log('END'));
        "#,
        b"piped-read",
        true,
    );
    assert!(
        stdout.contains("READ=\"piped-read\""),
        "read() must return real bytes: {stdout} stderr={stderr}"
    );
    assert!(stdout.contains("END"), "'end' must fire: {stdout}");
}

#[test]
#[serial]
fn read_returns_null_when_queue_empty_at_eof() {
    let (stdout, stderr, _ok) = run_eval(
        r#"
        process.stdin.on('end', () => {
          console.log('AFTER=' + String(process.stdin.read()));
        });
        process.stdin.resume();
        "#,
        b"",
        true,
    );
    assert!(
        stdout.contains("AFTER=null"),
        "read() at drained EOF must return null: {stdout} stderr={stderr}"
    );
}

#[test]
#[serial]
fn large_payload_arrives_complete_across_chunks() {
    // > 8 KiB forces multiple host reads; concatenation must equal the payload.
    let (stdout, stderr, _ok) = run_eval(
        r#"
        let n = 0, chunks = 0;
        process.stdin.on('data', (c) => { n += c.length; chunks += 1; });
        process.stdin.on('end', () => console.log('N=' + n + '|CHUNKS=' + chunks));
        "#,
        &vec![b'x'; 25_000],
        true,
    );
    assert!(
        stdout.contains("N=25000"),
        "all 25000 bytes must arrive: {stdout} stderr={stderr}"
    );
}

#[test]
#[serial]
fn invalid_utf8_replaced_lossy() {
    let (stdout, stderr, _ok) = run_eval(
        r#"
        process.stdin.on('data', (c) => {
          console.log('FFFD=' + (c.indexOf('�') !== -1) + '|LEN=' + c.length);
        });
        "#,
        b"a\xff\xffb",
        true,
    );
    assert!(
        stdout.contains("FFFD=true"),
        "invalid bytes must become U+FFFD: {stdout} stderr={stderr}"
    );
}

#[test]
#[serial]
fn set_encoding_validates_argument() {
    let (stdout, stderr, _ok) = run_eval(
        r#"
        const s = process.stdin;
        const results = [];
        for (const enc of ['utf8', 'utf-8', 'UTF8', null, undefined]) {
          try { s.setEncoding(enc); results.push('ok'); }
          catch (e) { results.push('throw'); }
        }
        try { s.setEncoding('base64'); results.push('no-throw'); }
        catch (e) { results.push(e.constructor.name + ':' + /Unknown encoding/.test(e.message)); }
        console.log(results.join('|'));
        "#,
        b"",
        true,
    );
    assert_eq!(
        stdout.lines().last().unwrap_or("").trim(),
        "ok|ok|ok|ok|ok|TypeError:true",
        "setEncoding contract mismatch: stdout={stdout} stderr={stderr}"
    );
}

#[test]
#[serial]
fn resume_pause_ref_unref_are_chainable() {
    let (stdout, stderr, _ok) = run_eval(
        r#"
        const s = process.stdin;
        console.log([
          s.resume() === s,
          s.pause() === s,
          s.unref() === s,
          s.ref() === s,
          s.setEncoding('utf8') === s
        ].join('|'));
        "#,
        b"",
        true,
    );
    assert_eq!(
        stdout.lines().last().unwrap_or("").trim(),
        "true|true|true|true|true",
        "fluent returns expected: stdout={stdout} stderr={stderr}"
    );
}

#[test]
#[serial]
fn pause_releases_keepalive_with_open_stdin() {
    // stdin stays OPEN (no EOF): if pause() failed to unref the loop,
    // the process would block on read forever and hit TIMEOUT.
    let (stdout, stderr, _ok) = run_eval(
        r#"
        process.stdin.on('data', () => {});
        process.stdin.pause();
        console.log('PAUSED_EXIT');
        "#,
        b"",
        false,
    );
    assert!(
        stdout.contains("PAUSED_EXIT"),
        "paused stdin must let the process exit: {stdout} stderr={stderr}"
    );
}

#[test]
#[serial]
fn bare_read_does_not_keep_process_alive() {
    // A one-off read() must not ref the event loop (Node semantics): with an
    // OPEN stdin the process still exits. Regression guard for the case where
    // starting the reader also claimed liveness and hung the process.
    let (stdout, stderr, _ok) = run_eval(
        r#"
        const r = process.stdin.read();
        console.log('READ_RET=' + String(r === null || typeof r === 'string'));
        "#,
        b"",
        false,
    );
    assert!(
        stdout.contains("READ_RET=true"),
        "bare read() must return null-or-string and let the process exit: {stdout} stderr={stderr}"
    );
}

#[test]
#[serial]
fn unref_releases_keepalive_with_open_stdin() {
    let (stdout, stderr, _ok) = run_eval(
        r#"
        process.stdin.resume();
        process.stdin.unref();
        console.log('UNREF_EXIT');
        "#,
        b"",
        false,
    );
    assert!(
        stdout.contains("UNREF_EXIT"),
        "unref'd stdin must let the process exit: {stdout} stderr={stderr}"
    );
}

#[test]
#[serial]
fn resume_holds_loop_until_eof_then_end() {
    // Proves the keep-alive works in the other direction too: with an open
    // stdin that later gets EOF, 'end' is delivered before exit.
    let (stdout, stderr, _ok) = run_eval(
        r#"
        let ended = false;
        process.stdin.on('end', () => { ended = true; console.log('ENDED=' + ended); });
        process.stdin.resume();
        console.log('SYNC_DONE');
        "#,
        b"tail",
        true,
    );
    let sync_pos = stdout.find("SYNC_DONE");
    let end_pos = stdout.find("ENDED=true");
    assert!(
        sync_pos.is_some() && end_pos.is_some() && end_pos > sync_pos,
        "'end' must be delivered on a later pump turn: {stdout} stderr={stderr}"
    );
}
