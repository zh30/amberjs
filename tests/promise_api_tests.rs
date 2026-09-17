#![allow(unexpected_cfgs)]
#![cfg(feature = "unstable_runtime")]
// Promise API 完整测试套件 - v0.2.1
//
// 目标：验证 Beejs 对现代 JavaScript Promise 语法的完整支持

#[cfg(test)]
mod tests {
    use amberjs::*;

    /// 测试 Promise.resolve()
    #[test]
    fn test_promise_resolve() {
        let code = r#"
            Promise.resolve(42)
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(
            result.is_ok(),
            "Promise.resolve should execute without errors"
        );
    }

    /// 测试 Promise.reject()
    #[test]
    fn test_promise_reject() {
        let code = r#"
            Promise.reject(new Error("Test error"))
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(
            result.is_ok(),
            "Promise.reject should execute without errors"
        );
    }

    /// 测试 Promise.all() - 所有 Promise 都成功
    #[test]
    fn test_promise_all_success() {
        let code = r#"
            Promise.all([
                Promise.resolve(1),
                Promise.resolve(2),
                Promise.resolve(3)
            ])
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(result.is_ok(), "Promise.all should execute without errors");
    }

    /// 测试 Promise.all() - 包含 rejection
    #[test]
    fn test_promise_all_with_rejection() {
        let code = r#"
            Promise.all([
                Promise.resolve(1),
                Promise.reject(new Error("Failed")),
                Promise.resolve(3)
            ])
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        // Promise.all 在有 rejection 时应该立即 rejection
        assert!(result.is_ok(), "Promise.all with rejection should execute");
    }

    /// 测试 Promise.all() - 空数组
    #[test]
    fn test_promise_all_empty() {
        let code = r#"
            Promise.all([])
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(
            result.is_ok(),
            "Promise.all([]) should execute without errors"
        );
    }

    /// 测试 Promise.all() - 非 Promise 值
    #[test]
    fn test_promise_all_non_promise() {
        let code = r#"
            Promise.all([1, 2, 3])
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(
            result.is_ok(),
            "Promise.all with non-Promise values should execute"
        );
    }

    /// 测试 Promise.allSettled() - 所有成功
    #[test]
    fn test_promise_all_settled_success() {
        let code = r#"
            Promise.allSettled([
                Promise.resolve(1),
                Promise.resolve(2),
                Promise.resolve(3)
            ])
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(
            result.is_ok(),
            "Promise.allSettled should execute without errors"
        );
    }

    /// 测试 Promise.allSettled() - 混合成功和失败
    #[test]
    fn test_promise_all_settled_mixed() {
        let code = r#"
            Promise.allSettled([
                Promise.resolve(1),
                Promise.reject(new Error("Failed")),
                Promise.resolve(3)
            ])
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(
            result.is_ok(),
            "Promise.allSettled with mixed results should execute"
        );
    }

    /// 测试 Promise.allSettled() - 空数组
    #[test]
    fn test_promise_all_settled_empty() {
        let code = r#"
            Promise.allSettled([])
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(
            result.is_ok(),
            "Promise.allSettled([]) should execute without errors"
        );
    }

    /// 测试 Promise.race() - 第一个成功
    #[test]
    fn test_promise_race_first_success() {
        let code = r#"
            Promise.race([
                new Promise(resolve => setTimeout(() => resolve(1), 100)),
                Promise.resolve(2),
                Promise.resolve(3)
            ])
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(result.is_ok(), "Promise.race should execute without errors");
    }

    /// 测试 Promise.race() - 空数组
    #[test]
    fn test_promise_race_empty() {
        let code = r#"
            Promise.race([])
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(
            result.is_ok(),
            "Promise.race([]) should execute without errors"
        );
    }

    /// 测试 Promise.any() - 所有成功
    #[test]
    fn test_promise_any_success() {
        let code = r#"
            Promise.any([
                Promise.resolve(1),
                Promise.resolve(2),
                Promise.resolve(3)
            ])
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(result.is_ok(), "Promise.any should execute without errors");
    }

    /// 测试 Promise.any() - 所有失败
    #[test]
    fn test_promise_any_all_rejected() {
        let code = r#"
            Promise.any([
                Promise.reject(new Error("Error 1")),
                Promise.reject(new Error("Error 2")),
                Promise.reject(new Error("Error 3"))
            ])
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        // Promise.any 在所有 Promise 都失败时应该抛出 AggregateError
        assert!(
            result.is_ok(),
            "Promise.any with all rejections should execute"
        );
    }

    /// 测试 Promise.any() - 空数组
    #[test]
    fn test_promise_any_empty() {
        let code = r#"
            Promise.any([])
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(
            result.is_ok(),
            "Promise.any([]) should execute without errors"
        );
    }

    /// 测试链式调用
    #[test]
    fn test_promise_chain() {
        let code = r#"
            Promise.resolve(1)
                .then(x => x + 1)
                .then(x => x * 2)
                .then(x => x + 3)
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(
            result.is_ok(),
            "Promise chaining should execute without errors"
        );
    }

    /// 测试错误处理
    #[test]
    fn test_promise_error_handling() {
        let code = r#"
            Promise.reject(new Error("Test error"))
                .catch(err => err.message)
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(
            result.is_ok(),
            "Promise error handling should execute without errors"
        );
    }

    /// 测试 finally 方法
    #[test]
    fn test_promise_finally() {
        let code = r#"
            Promise.resolve(42)
                .finally(() => console.log("Finally called"))
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(
            result.is_ok(),
            "Promise.finally should execute without errors"
        );
    }

    /// 测试基本 async 函数
    #[test]
    fn test_basic_async_function() {
        let code = r#"
            async function greet(name) {
                return "Hello, " + name + "!";
            }
            greet("Beejs")
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(
            result.is_ok(),
            "Async function should execute without errors"
        );
    }

    /// 测试基本 await 语法
    #[test]
    fn test_basic_await() {
        let code = r#"
            async function test() {
                const result = await Promise.resolve(42);
                return result;
            }
            test()
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(result.is_ok(), "Await should work with Promise.resolve");
    }

    /// 测试 async 箭头函数
    #[test]
    fn test_async_arrow_function() {
        let code = r#"
            const add = async (a, b) => {
                return a + b;
            };
            add(5, 3)
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(result.is_ok(), "Async arrow functions should work");
    }

    /// 测试 async/await 错误处理
    #[test]
    fn test_async_error_handling() {
        let code = r#"
            async function test() {
                try {
                    await Promise.reject(new Error("Test error"));
                } catch (e) {
                    return "Error caught: " + e.message;
                }
            }
            test()
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(result.is_ok(), "Async error handling should work");
    }

    /// 测试多个 await 操作
    #[test]
    fn test_multiple_awaits() {
        let code = r#"
            async function test() {
                const a = await Promise.resolve(1);
                const b = await Promise.resolve(2);
                const c = await Promise.resolve(3);
                return a + b + c;
            }
            test()
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(result.is_ok(), "Multiple awaits should work");
    }

    /// 测试并发 await
    #[test]
    fn test_concurrent_awaits() {
        let code = r#"
            async function test() {
                const [a, b, c] = await Promise.all([
                    Promise.resolve(1),
                    Promise.resolve(2),
                    Promise.resolve(3)
                ]);
                return a + b + c;
            }
            test()
        "#;

        let runtime = RuntimeLite::new(false).expect("Failed to create runtime");
        let result = runtime.execute_standard(code);
        assert!(result.is_ok(), "Concurrent awaits should work");
    }
}
