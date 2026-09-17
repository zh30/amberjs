// Clipboard API 测试套件 - v0.3.342
//
// 目标：验证 Beejs 对 Clipboard 接口的完整支持
// Clipboard API 用于 AI 工作负载中的复制/粘贴功能

#[cfg(test)]
mod tests {
    use amberjs::MinimalRuntime;
    use serial_test::serial;

    /// 测试 navigator.clipboard 可用性
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

    /// 测试 clipboard.readText 方法可用性
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

    /// 测试 clipboard.writeText 方法可用性
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
    fn test_clipboard_text_methods_are_promises_and_fail_closed() {
        let code = r#"
            (async () => {
                const writeResult = navigator.clipboard.writeText('Hello, Beejs!');
                const readResult = navigator.clipboard.readText();
                const writeIsPromise = writeResult instanceof Promise;
                const readIsPromise = readResult instanceof Promise;
                const writeOutcome = await writeResult.then(
                    () => 'write-resolved',
                    error => `write-rejected:${String(error && error.message ? error.message : error)}`
                );
                const readOutcome = await readResult.then(
                    value => `read-resolved:${value}`,
                    error => `read-rejected:${String(error && error.message ? error.message : error)}`
                );
                return `${writeIsPromise}:${readIsPromise}:${writeOutcome}:${readOutcome}`;
            })();
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code).unwrap();
        assert!(
            result.trim().contains("true:true:write-rejected:")
                && result.trim().contains(":read-rejected:")
                && result.trim().contains("not supported"),
            "clipboard text methods must be Promise-based and fail closed without permission/backend: {}",
            result
        );
    }

    /// 测试 writeText 基本功能
    #[test]
    #[serial]
    fn test_write_text_basic() {
        let code = r#"
            const result = navigator.clipboard.writeText('Hello, Beejs!');
            result instanceof Promise
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "writeText should return a Promise");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 readText 返回 Promise
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

    /// 测试 writeText 特殊字符
    #[test]
    #[serial]
    fn test_write_text_special_chars() {
        let code = r#"
            const text = 'Hello 世界! 🐝';
            const result = navigator.clipboard.writeText(text);
            result instanceof Promise
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "writeText should return a Promise for special characters"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 writeText 空字符串
    #[test]
    #[serial]
    fn test_write_text_empty() {
        let code = r#"
            const result = navigator.clipboard.writeText('');
            result instanceof Promise
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "writeText should return a Promise for empty string"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 writeText 换行符
    #[test]
    #[serial]
    fn test_write_text_newlines() {
        let code = r#"
            const text = 'Line 1\nLine 2\tTabbed';
            const result = navigator.clipboard.writeText(text);
            result instanceof Promise
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "writeText should return a Promise for newlines and tabs"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 clipboard.read 方法可用性（现代 API）
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

    /// 测试 clipboard.write 方法可用性（现代 API）
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
    fn test_clipboard_item_methods_are_promises_and_fail_closed() {
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
            result.trim().contains("true:true:read-rejected:")
                && result.trim().contains(":write-rejected:")
                && result.trim().contains("not supported"),
            "clipboard item methods must be Promise-based and fail closed without backend: {}",
            result
        );
    }

    /// 测试 read 返回 Promise
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

    /// 测试 write 返回 Promise
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

    /// 测试 AI 工作负载场景 - 复制处理结果
    #[test]
    #[serial]
    fn test_ai_workload_copy_result() {
        let code = r#"
            // Simulate AI processing result
            const aiResult = JSON.stringify({ prediction: 'cat', confidence: 0.95 });
            const result = navigator.clipboard.writeText(aiResult);
            result instanceof Promise
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "AI workload copy should return a Promise");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 AI 工作负载场景 - 读取输入数据
    #[test]
    #[serial]
    fn test_ai_workload_paste_input() {
        let code = r#"
            // Simulate reading input data from clipboard
            const hasReadText = typeof navigator.clipboard.readText === 'function';
            hasReadText
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "AI workload paste should work");
        assert_eq!(result.unwrap().trim(), "true");
    }
}
