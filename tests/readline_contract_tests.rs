//! Pins docs/READLINE_CONTRACT.md — createInterface + question host stdin/stdout.
use serial_test::serial;
use std::io::Write;
use std::process::{Command, Stdio};

fn amber() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_amber"));
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}

fn run_question(script: &str, stdin_bytes: &[u8]) -> (String, String, bool) {
    let mut child = amber()
        .args(["eval", script])
        .spawn()
        .expect("spawn amber eval");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        stdin
            .write_all(stdin_bytes)
            .expect("write stdin for question");
        // Drop stdin to send EOF after the line payload.
    }
    let output = child.wait_with_output().expect("wait amber eval");
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.success(),
    )
}

#[test]
#[serial]
fn require_readline_create_interface_and_question_are_functions() {
    let (stdout, stderr, ok) = run_question(
        r#"
        const readline = require('readline');
        const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
        console.log([
          typeof readline.createInterface,
          typeof rl.question,
          typeof rl.close,
          require('node:readline') === readline
        ].join('|'));
        rl.close();
        "#,
        b"\n",
    );
    assert!(ok, "eval failed: stdout={stdout} stderr={stderr}");
    assert_eq!(
        stdout.lines().last().unwrap_or("").trim(),
        "function|function|function|true"
    );
}

#[test]
#[serial]
fn question_reads_piped_stdin_line_and_writes_query() {
    let (stdout, stderr, ok) = run_question(
        r#"
        const readline = require('readline');
        const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
        rl.question('Q?', (answer) => {
          console.log('ANS=' + JSON.stringify(answer));
          rl.close();
        });
        "#,
        b"hello world\n",
    );
    assert!(ok, "eval failed: stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("Q?"),
        "query must be written to stdout: {stdout}"
    );
    assert!(
        stdout.contains("ANS=\"hello world\""),
        "callback must receive the stdin line: {stdout}"
    );
}

#[test]
#[serial]
fn question_strips_crlf_and_empty_line_is_empty_string() {
    let (stdout, stderr, ok) = run_question(
        r#"
        const readline = require('readline');
        const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
        rl.question('', (answer) => {
          console.log('ANS=' + JSON.stringify(answer) + '|T=' + typeof answer);
          rl.close();
        });
        "#,
        b"\r\n",
    );
    assert!(ok, "eval failed: stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("ANS=\"\"|T=string"),
        "bare CRLF must yield empty string: {stdout}"
    );
}

#[test]
#[serial]
fn question_eof_without_bytes_calls_callback_with_null() {
    let (stdout, stderr, ok) = run_question(
        r#"
        const readline = require('readline');
        const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
        rl.question('>', (answer) => {
          console.log('ANS=' + String(answer) + '|NULL=' + (answer === null));
          rl.close();
        });
        "#,
        b"",
    );
    assert!(ok, "eval failed: stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("ANS=null|NULL=true"),
        "EOF with no bytes must callback null: {stdout}"
    );
}

#[test]
#[serial]
fn question_requires_two_arguments() {
    let (stdout, stderr, ok) = run_question(
        r#"
        const readline = require('readline');
        const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
        try {
          rl.question('only-query');
          console.log('NO_THROW');
        } catch (e) {
          console.log('ERR=' + (e && e.message ? e.message : String(e)));
        }
        rl.close();
        "#,
        b"\n",
    );
    assert!(ok, "eval failed: stdout={stdout} stderr={stderr}");
    assert!(
        stdout.contains("ERR=") && stdout.contains("question requires 2 arguments"),
        "arity error expected: {stdout}"
    );
}

#[test]
#[serial]
fn question_does_not_invent_empty_answer_without_stdin() {
    // Guard against regressing to the old immediate cb("") stub when stdin has data.
    let (stdout, stderr, ok) = run_question(
        r#"
        const readline = require('readline');
        const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
        rl.question('P:', (answer) => {
          if (answer === '') {
            console.log('EMPTY_LIE');
          } else {
            console.log('GOT=' + answer);
          }
          rl.close();
        });
        "#,
        b"not-empty\n",
    );
    assert!(ok, "eval failed: stdout={stdout} stderr={stderr}");
    assert!(
        !stdout.contains("EMPTY_LIE"),
        "must not ignore stdin: {stdout}"
    );
    assert!(
        stdout.contains("GOT=not-empty"),
        "must echo real answer: {stdout}"
    );
}
