//! Pins docs/CLIPBOARD_CONTRACT.md (G31).
//!
//! Stable surface: navigator.clipboard.writeText / readText against a
//! process-local Mutex<String>. ClipboardItem read/write stay rejected.
//! Not OS clipboard / secure-context / permissions parity.

#[cfg(test)]
mod tests {
    use amberjs::MinimalRuntime;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_clipboard_available() {
        let code = r#"
            typeof navigator !== 'undefined' && typeof navigator.clipboard === 'object'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "navigator.clipboard should be available");
        assert_eq!(result.unwrap().trim(), "true");
    }

    #[test]
    #[serial]
    fn test_read_text_method() {
        let code = r#"
            typeof navigator.clipboard.readText === 'function'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "readText method should be available");
        assert_eq!(result.unwrap().trim(), "true");
    }

    #[test]
    #[serial]
    fn test_write_text_method() {
        let code = r#"
            typeof navigator.clipboard.writeText === 'function'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "writeText method should be available");
        assert_eq!(result.unwrap().trim(), "true");
    }

    #[test]
    #[serial]
    fn test_write_text_read_text_round_trip() {
        let code = r#"
            (async () => {
                await navigator.clipboard.writeText('Hello, Amber!');
                const value = await navigator.clipboard.readText();
                return value;
            })();
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code).unwrap();
        assert_eq!(
            result.trim(),
            "Hello, Amber!",
            "writeText/readText must round-trip in-process: {}",
            result
        );
    }

    #[test]
    #[serial]
    fn test_write_text_empty_string() {
        let code = r#"
            (async () => {
                await navigator.clipboard.writeText('before');
                await navigator.clipboard.writeText('');
                return await navigator.clipboard.readText();
            })();
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code).unwrap();
        assert_eq!(result.trim(), "", "empty writeText must clear the store");
    }

    #[test]
    #[serial]
    fn test_write_text_special_chars_round_trip() {
        let code = r#"
            (async () => {
                const text = 'Hello 世界! 🐝';
                await navigator.clipboard.writeText(text);
                return await navigator.clipboard.readText();
            })();
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code).unwrap();
        assert_eq!(result.trim(), "Hello 世界! 🐝");
    }

    #[test]
    #[serial]
    fn test_write_text_newlines_round_trip() {
        let code = r#"
            (async () => {
                const text = 'Line 1\nLine 2\tTabbed';
                await navigator.clipboard.writeText(text);
                return await navigator.clipboard.readText();
            })();
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code).unwrap();
        assert_eq!(result.trim(), "Line 1\nLine 2\tTabbed");
    }

    #[test]
    #[serial]
    fn test_write_text_tostring_coercion() {
        let code = r#"
            (async () => {
                await navigator.clipboard.writeText(null);
                const fromNull = await navigator.clipboard.readText();
                await navigator.clipboard.writeText(undefined);
                const fromUndef = await navigator.clipboard.readText();
                await navigator.clipboard.writeText();
                const fromMissing = await navigator.clipboard.readText();
                await navigator.clipboard.writeText(42);
                const fromNumber = await navigator.clipboard.readText();
                return [fromNull, fromUndef, fromMissing, fromNumber].join('|');
            })();
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code).unwrap();
        assert_eq!(
            result.trim(),
            "|||42",
            "writeText must coerce null/undefined/missing to empty and ToString others: {}",
            result
        );
    }

    #[test]
    #[serial]
    fn test_write_text_shared_across_runtimes() {
        {
            let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
            let result = runtime
                .execute_code(
                    r#"
                    (async () => {
                        await navigator.clipboard.writeText('shared-across-runtimes');
                        return 'ok';
                    })();
                    "#,
                )
                .unwrap();
            assert_eq!(result.trim(), "ok");
        }
        {
            let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
            let result = runtime
                .execute_code(
                    r#"
                    (async () => {
                        return await navigator.clipboard.readText();
                    })();
                    "#,
                )
                .unwrap();
            assert_eq!(
                result.trim(),
                "shared-across-runtimes",
                "process-local store must survive a new MinimalRuntime: {}",
                result
            );
        }
    }

    #[test]
    #[serial]
    fn test_write_text_returns_promise() {
        let code = r#"
            const result = navigator.clipboard.writeText('Hello, Amber!');
            result instanceof Promise
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "writeText should return a Promise");
        assert_eq!(result.unwrap().trim(), "true");
    }

    #[test]
    #[serial]
    fn test_read_text_returns_promise() {
        let code = r#"
            const result = navigator.clipboard.readText();
            result instanceof Promise
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "readText should return a Promise");
        assert_eq!(result.unwrap().trim(), "true");
    }

    #[test]
    #[serial]
    fn test_read_method() {
        let code = r#"
            typeof navigator.clipboard.read === 'function'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "read method should be available");
        assert_eq!(result.unwrap().trim(), "true");
    }

    #[test]
    #[serial]
    fn test_write_method() {
        let code = r#"
            typeof navigator.clipboard.write === 'function'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "write method should be available");
        assert_eq!(result.unwrap().trim(), "true");
    }

    #[test]
    #[serial]
    fn test_clipboard_item_methods_reject_honestly() {
        let code = r#"
            (async () => {
                const readResult = navigator.clipboard.read();
                const writeResult = navigator.clipboard.write([]);
                const readIsPromise = readResult instanceof Promise;
                const writeIsPromise = writeResult instanceof Promise;
                const readOutcome = await readResult.then(
                    value => `read-resolved:${Array.isArray(value)}:${value.length}`,
                    error => `read-rejected:${String(error && error.message ? error.message : error)}`
                );
                const writeOutcome = await writeResult.then(
                    () => 'write-resolved',
                    error => `write-rejected:${String(error && error.message ? error.message : error)}`
                );
                return `${readIsPromise}:${writeIsPromise}:${readOutcome}:${writeOutcome}`;
            })();
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code).unwrap();
        assert!(
            result.trim().starts_with("true:true:read-rejected:")
                && result.trim().contains(":write-rejected:")
                && result.trim().contains("ClipboardItem")
                && result.trim().contains("writeText"),
            "ClipboardItem read/write must reject with an honest Limit pointing at writeText/readText: {}",
            result
        );
    }

    #[test]
    #[serial]
    fn test_read_returns_promise() {
        let code = r#"
            const result = navigator.clipboard.read();
            result instanceof Promise
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "read should return a Promise");
        assert_eq!(result.unwrap().trim(), "true");
    }

    #[test]
    #[serial]
    fn test_write_returns_promise() {
        let code = r#"
            const result = navigator.clipboard.write([]);
            result instanceof Promise
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "write should return a Promise");
        assert_eq!(result.unwrap().trim(), "true");
    }

    #[test]
    #[serial]
    fn test_ai_workload_copy_paste_round_trip() {
        let code = r#"
            (async () => {
                const aiResult = JSON.stringify({ prediction: 'cat', confidence: 0.95 });
                await navigator.clipboard.writeText(aiResult);
                const pasted = await navigator.clipboard.readText();
                return pasted === aiResult ? 'ok' : `mismatch:${pasted}`;
            })();
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code).unwrap();
        assert_eq!(result.trim(), "ok");
    }
}
