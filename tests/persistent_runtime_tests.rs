// 持久化运行时实例测试
// 测试 Runtime 结构体复用 MinimalRuntime 实例以保持模块缓存

#[cfg(test)]
mod persistent_runtime_tests {
    use amberjs::Runtime;

    /// 测试 Runtime 能够复用内部 MinimalRuntime 实例
    #[test]
    fn test_runtime_persists_minimal_runtime() {
        let runtime = Runtime::new_default();

        // 第一次执行应该创建 MinimalRuntime 实例
        let result1 = runtime.execute_code("1 + 1");
        assert!(result1.is_ok());
        assert!(result1.unwrap().contains("2"));

        // 后续执行应该复用同一个 MinimalRuntime 实例
        let result2 = runtime.execute_code("2 + 2");
        assert!(result2.is_ok());
        assert!(result2.unwrap().contains("4"));

        // 第三次执行验证仍然正常工作
        let result3 = runtime.execute_code("3 + 3");
        assert!(result3.is_ok());
        assert!(result3.unwrap().contains("6"));
    }

    /// 测试多次 execute_code 调用之间模块缓存有效
    #[test]
    fn test_module_cache_persists_across_executions() {
        let runtime = Runtime::new_default();

        // 定义一个函数
        let result1 = runtime.execute_code("function add(a, b) { return a + b; }");
        assert!(result1.is_ok());

        // 在后续调用中使用同一个函数
        let result2 = runtime.execute_code("add(10, 20)");
        assert!(result2.is_ok());

        // 验证函数定义仍然存在
        let result3 = runtime.execute_code("add(5, 5)");
        assert!(result3.is_ok());
        assert!(result3.unwrap().contains("10"));
    }

    /// 测试 Runtime 在同一实例中支持多次执行
    #[test]
    fn test_multiple_executions_same_runtime() {
        let runtime = Runtime::new_default();

        for i in 0..10 {
            let code = format!("{} * {}", i, i);
            let result = runtime.execute_code(&code);
            assert!(result.is_ok(), "Execution {} should succeed", i);
            let output = result.unwrap();
            let expected = i * i;
            assert!(
                output.contains(&expected.to_string()),
                "Expected {} * {} = {}, got: {}",
                i,
                i,
                expected,
                output
            );
        }
    }

    /// 测试不同 Runtime 实例保持独立
    #[test]
    fn test_different_runtime_instances_are_independent() {
        let runtime1 = Runtime::new_default();
        let runtime2 = Runtime::new_default();

        // 在 runtime1 中定义变量
        let result1 = runtime1.execute_code("globalThis.testVar = 100");
        assert!(result1.is_ok());

        // runtime2 应该没有这个变量（状态隔离）
        let result2 = runtime2.execute_code("typeof testVar");
        assert!(result2.is_ok());
        let output2 = result2.unwrap();
        // "undefined" 表示变量不存在
        assert!(
            output2.contains("undefined"),
            "Different runtime instances should be isolated, got: {}",
            output2
        );
    }

    /// 测试持久化运行时支持 TypeScript 语法
    #[test]
    fn test_persistent_runtime_typescript_support() {
        let runtime = Runtime::new_default();

        // 第一次执行 TypeScript
        let result1 = runtime.execute_code("let num: number = 42; num * 2;");
        assert!(result1.is_ok());

        // 后续执行也支持 TypeScript
        let result2 = runtime.execute_code("let str: string = 'hello'; str.length;");
        assert!(result2.is_ok());
        assert!(result2.unwrap().contains("5"));
    }

    /// 测试持久化运行时支持 Node.js API
    #[test]
    fn test_persistent_runtime_nodejs_api() {
        let runtime = Runtime::new_default();

        // 使用 process 对象
        let result1 = runtime.execute_code("process.arch");
        assert!(result1.is_ok());

        // 使用 path 对象
        let result2 = runtime.execute_code("path.join('/tmp', 'test.js')");
        assert!(result2.is_ok());
        assert!(result2.unwrap().contains("test.js"));
    }

    /// 测试 execute_file 也复用运行时实例
    #[test]
    fn test_execute_file_reuses_runtime() {
        use std::fs;
        use std::path::PathBuf;

        let runtime = Runtime::new_default();

        // 创建临时测试文件
        let mut test_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_file.push("tests/fixtures/test_file_reuse.js");
        test_file.set_extension("js");

        let test_content = r#"
function multiply(a, b) {
    return a * b;
}
multiply(7, 8);
"#;

        // 确保 tests/fixtures 目录存在
        if let Some(parent) = test_file.parent() {
            fs::create_dir_all(parent).ok();
        }

        fs::write(&test_file, test_content).expect("Failed to write test file");

        // 第一次执行文件
        let result1 = runtime.execute_file(&test_file);
        assert!(result1.is_ok(), "First file execution should succeed");

        // 后续执行应该复用运行时实例
        let result2 = runtime.execute_file(&test_file);
        assert!(result2.is_ok(), "Second file execution should succeed");

        // 清理
        fs::remove_file(&test_file).ok();
    }

    /// 测试 execute_file 会把当前文件路径传给底层 runtime，用于相对 dynamic import
    #[test]
    fn test_execute_file_dynamic_import_resolves_relative_to_file() {
        use std::fs;

        let temp = tempfile::tempdir().expect("Failed to create temp dir");
        let app_dir = temp.path().join("app");
        fs::create_dir_all(&app_dir).expect("Failed to create app dir");
        let main_path = app_dir.join("main.js");
        fs::write(
            app_dir.join("dep.mjs"),
            r#"
globalThis.__runtimeExecuteFileDynamicImportLoads =
    (globalThis.__runtimeExecuteFileDynamicImportLoads || 0) + 1;
export const answer = 42;
export const loadCount = globalThis.__runtimeExecuteFileDynamicImportLoads;
"#,
        )
        .expect("Failed to write dependency");
        fs::write(
            &main_path,
            r#"
globalThis.__runtimeExecuteFileDynamicImportDone = false;
import('./dep.mjs').then((mod) => {
    globalThis.__runtimeExecuteFileDynamicImportResult =
        `${mod.answer}:${mod.loadCount}`;
    globalThis.__runtimeExecuteFileDynamicImportDone = true;
}).catch((error) => {
    globalThis.__runtimeExecuteFileDynamicImportResult =
        error && error.message ? error.message : String(error);
    globalThis.__runtimeExecuteFileDynamicImportDone = true;
});
"#,
        )
        .expect("Failed to write main file");

        let runtime = Runtime::new_default();
        runtime
            .execute_file(&main_path)
            .expect("File execution should succeed");
        let done = runtime
            .execute_code("globalThis.__runtimeExecuteFileDynamicImportDone")
            .expect("Failed to read dynamic import completion marker");
        assert_eq!(done.trim(), "true");
        let result = runtime
            .execute_code("globalThis.__runtimeExecuteFileDynamicImportResult")
            .expect("Failed to read dynamic import result");

        assert_eq!(result.trim(), "42:1");
    }

    /// 测试快速模式下 Runtime 持久化
    #[test]
    fn test_fast_mode_runtime_persistence() {
        let runtime = Runtime::new(1, 512 * 1024 * 1024, true, false);

        // 多次执行验证
        for i in 0..5 {
            let code = format!("'test-{}'", i);
            let result = runtime.execute_code(&code);
            assert!(result.is_ok(), "Execution {} should succeed", i);
        }
    }

    /// 测试持久化运行时的错误隔离
    #[test]
    fn test_error_isolation_in_persistent_runtime() {
        let runtime = Runtime::new_default();

        // 第一次执行产生错误
        let result1 = runtime.execute_code("throw new Error('test error')");
        assert!(result1.is_err() || result1.unwrap().contains("Error"));

        // 后续执行应该仍然正常工作
        let result2 = runtime.execute_code("1 + 1");
        assert!(result2.is_ok());
        assert!(result2.unwrap().contains("2"));
    }

    /// 测试持久化运行时的全局状态管理
    #[test]
    fn test_global_state_management() {
        let runtime = Runtime::new_default();

        // 设置全局变量
        let result1 = runtime.execute_code("globalThis.appName = 'amberjs'");
        assert!(result1.is_ok());

        // 读取全局变量
        let result2 = runtime.execute_code("globalThis.appName");
        assert!(result2.is_ok());
        assert!(result2.unwrap().contains("amberjs"));

        // 修改全局变量
        let result3 = runtime.execute_code("globalThis.appName = 'amberjs-v2'");
        assert!(result3.is_ok());

        // 验证修改
        let result4 = runtime.execute_code("globalThis.appName");
        assert!(result4.is_ok());
        assert!(result4.unwrap().contains("amberjs-v2"));
    }
}
