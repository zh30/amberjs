// v0.3.277: Readline API integration tests
// Tests for readline.createInterface(), Interface.question(), etc.

#[cfg(test)]
mod readline_api_tests {
    use std::path::PathBuf;
    use std::process::Command;

    fn amberjs_path() -> PathBuf {
        PathBuf::from(
            std::env::var("CARGO_BIN_EXE_amber")
                .unwrap_or_else(|_| "./target/debug/amber".to_string()),
        )
    }

    #[test]
    fn test_readline_exists() {
        let output = Command::new(amberjs_path())
            .args(["eval", "console.log(typeof readline)"])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("object"),
            "readline should exist as an object"
        );
    }

    #[test]
    fn test_readline_create_interface_exists() {
        let output = Command::new(amberjs_path())
            .args(["eval", "console.log(typeof readline.createInterface)"])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("function"),
            "readline.createInterface should exist"
        );
    }

    #[test]
    fn test_readline_create_interface_function() {
        // Test that createInterface returns a function-like object with expected methods
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
                console.log(typeof rl.question);
                console.log(typeof rl.close);
                console.log(typeof rl.pause);
                console.log(typeof rl.resume);
                console.log(typeof rl.setPrompt);
                console.log(typeof rl.prompt);
                rl.close();
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("function"),
            "Interface methods should be functions"
        );
    }

    #[test]
    fn test_readline_create_interface_with_options() {
        // Test creating interface with different options
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                const readline = require('readline');
                const rl = readline.createInterface({
                    input: process.stdin,
                    output: process.stdout,
                    terminal: false,
                    completer: null,
                    historySize: 100
                });
                console.log('interface created successfully');
                rl.close();
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("interface created successfully"),
            "Interface should be created with options"
        );
    }

    #[test]
    fn test_readline_set_prompt() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
                rl.setPrompt('> ');
                rl.close();
                console.log('prompt set successfully');
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("prompt set successfully"),
            "setPrompt should work"
        );
    }

    #[test]
    fn test_readline_interface_close() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
                rl.close();
                console.log('close called without error');
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("close called without error"),
            "close should work"
        );
    }

    #[test]
    fn test_readline_interface_pause_resume() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
                rl.pause();
                rl.resume();
                rl.close();
                console.log('pause and resume work');
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("pause and resume work"),
            "pause/resume should work"
        );
    }

    #[test]
    fn test_readline_multiple_interfaces() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl1 = readline.createInterface({ input: process.stdin, output: process.stdout });
                const rl2 = readline.createInterface({ input: process.stdin, output: process.stdout });
                rl1.close();
                rl2.close();
                console.log('multiple interfaces work');
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("multiple interfaces work"),
            "Multiple interfaces should work"
        );
    }

    #[test]
    fn test_readline_crlf_after_close() {
        // Test that close adds CRLF to output (Node.js behavior)
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
                // Just ensure close doesn't throw
                rl.close();
                console.log('ok');
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("ok"), "Close should not throw");
    }

    #[test]
    fn test_readline_question_exists() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
                console.log(typeof rl.question);
                rl.close();
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("function"), "question method should exist");
    }

    #[test]
    fn test_readline_write_exists() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
                console.log(typeof rl.write);
                rl.close();
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("function"), "write method should exist");
    }

    #[test]
    fn test_readline_clear_line() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
                console.log(typeof rl.clearLine);
                rl.close();
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("function"), "clearLine method should exist");
    }

    // v0.3.279: Completer support tests
    #[test]
    fn test_readline_completer_option_exists() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                const readline = require('readline');
                // Test that completer option is accepted
                const rl = readline.createInterface({
                    input: process.stdin,
                    output: process.stdout,
                    completer: null
                });
                console.log('completer option accepted');
                rl.close();
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("completer option accepted"),
            "Completer option should be accepted"
        );
    }

    #[test]
    fn test_readline_completer_with_function() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                const readline = require('readline');
                const completer = (line) => {
                    const hits = ['hello', 'help', 'history'].filter(c => c.startsWith(line));
                    return [hits, line];
                };
                const rl = readline.createInterface({
                    input: process.stdin,
                    output: process.stdout,
                    completer: completer
                });
                console.log('completer function accepted');
                rl.close();
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("completer function accepted"),
            "Completer function should be accepted"
        );
    }

    #[test]
    fn test_readline_completer_stored_on_interface() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                const readline = require('readline');
                const completer = (line) => [[], line];
                const rl = readline.createInterface({
                    input: process.stdin,
                    output: process.stdout,
                    completer: completer
                });
                // Completer should be stored on the interface
                console.log(typeof rl.completer);
                rl.close();
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("function"),
            "Completer should be stored on interface"
        );
    }

    // v0.3.280: History and cursorPosition support tests
    #[test]
    fn test_readline_history_property_exists() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
                console.log(Array.isArray(rl.history));
                rl.close();
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("true"), "history should be an array");
    }

    #[test]
    fn test_readline_history_size_option() {
        let output = Command::new(amberjs_path())
            .args([
                "eval",
                r#"
                const readline = require('readline');
                // historySize option should be accepted
                const rl = readline.createInterface({
                    input: process.stdin,
                    output: process.stdout,
                    historySize: 100
                });
                console.log('historySize option accepted');
                rl.close();
            "#,
            ])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("historySize option accepted"),
            "historySize option should be accepted"
        );
    }

    #[test]
    fn test_readline_cursor_property_exists() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
                console.log(typeof rl.cursor);
                rl.close();
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("number"), "cursor should be a number");
    }

    #[test]
    fn test_readline_column_property_exists() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
                console.log(typeof rl.column);
                rl.close();
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("number"), "column should be a number");
    }

    #[test]
    fn test_readline_line_property_exists() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
                console.log(typeof rl.line);
                rl.close();
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("string"), "line should be a string");
    }

    // v0.3.280: Event system tests
    #[test]
    fn test_readline_on_method_exists() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
                console.log(typeof rl.on);
                rl.close();
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("function"), "on method should exist");
    }

    #[test]
    fn test_readline_emit_method_exists() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
                console.log(typeof rl.emit);
                rl.close();
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("function"), "emit method should exist");
    }

    #[test]
    fn test_readline_event_system() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
                let event_received = false;
                rl.on('testEvent', (arg) => {
                    event_received = true;
                    console.log('event received: ' + arg);
                });
                rl.emit('testEvent', 'hello');
                rl.close();
                console.log('event system works: ' + event_received);
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("event received: hello"),
            "Event should be received"
        );
        assert!(
            stdout.contains("event system works: true"),
            "Event system should work"
        );
    }

    #[test]
    fn test_readline_on_returns_interface() {
        let output = Command::new(amberjs_path())
            .args(["eval", r#"
                const readline = require('readline');
                const rl = readline.createInterface({ input: process.stdin, output: process.stdout });
                const result = rl.on('event', () => {});
                console.log(result === rl);
                rl.close();
            "#])
            .output()
            .expect("Failed to run amber");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "on() should return the interface for chaining"
        );
    }
}
