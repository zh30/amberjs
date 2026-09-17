// Async/Await 支持测试套件
//
// 目标：验证 Beejs 对现代 JavaScript async/await 语法的支持

#[cfg(test)]
mod tests {
    use amberjs::runtime_minimal::MinimalRuntime;
    use serial_test::serial;

    /// 测试1: 基本 async 函数
    #[test]
    #[serial]
    fn test_basic_async_function() {
        let code = r#"
            async function greet(name) {
                return "Hello, " + name + "!";
            }
            greet("Beejs");
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "Async function should execute without errors"
        );
    }

    /// 测试2: 基本 await 语法
    #[test]
    #[serial]
    fn test_basic_await() {
        let code = r#"
            async function test() {
                const result = await Promise.resolve(42);
                return result;
            }
            test();
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Await should work with Promise.resolve");
    }

    /// 测试3: Async/await 错误处理
    #[test]
    #[serial]
    fn test_async_error_handling() {
        let code = r#"
            async function test() {
                try {
                    await Promise.reject(new Error("Test error"));
                } catch (e) {
                    return "Error caught: " + e.message;
                }
            }
            test();
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Async error handling should work");
    }

    /// 测试4: Promise.all 支持
    #[test]
    #[serial]
    fn test_promise_all() {
        let code = r#"
            async function test() {
                const promises = [
                    Promise.resolve(1),
                    Promise.resolve(2),
                    Promise.resolve(3)
                ];
                const results = await Promise.all(promises);
                return results;
            }
            test();
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Promise.all should work");
    }

    /// 测试5: 多个 await 操作
    #[test]
    #[serial]
    fn test_multiple_awaits() {
        let code = r#"
            async function test() {
                const a = await Promise.resolve(1);
                const b = await Promise.resolve(2);
                const c = await Promise.resolve(3);
                return a + b + c;
            }
            test();
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Multiple awaits should work");
    }

    /// 测试6: Async 箭头函数
    #[test]
    #[serial]
    fn test_async_arrow_function() {
        let code = r#"
            const add = async (a, b) => {
                return a + b;
            };
            add(5, 3);
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Async arrow functions should work");
    }
}
