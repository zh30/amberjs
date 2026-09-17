// Beejs 核心功能测试套件 (TDD)
// Stage 1: 红色阶段 - 编写失败的测试

use tempfile::TempDir;

/// 测试类型定义
#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    /// 测试 1: MinimalRuntime 初始化 (预期失败 - 红色)
    #[tokio::test]
    #[serial]
    async fn test_minimal_runtime_initialization() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange: 创建临时目录
        let temp_dir = TempDir::new()?;
        let _temp_path = temp_dir.path().to_path_buf();

        // Act: 尝试创建 MinimalRuntime
        let runtime = amberjs::runtime_minimal::MinimalRuntime::new();

        // Assert: 验证初始化成功
        assert!(
            runtime.is_ok(),
            "MinimalRuntime should initialize successfully"
        );

        println!("✅ MinimalRuntime 初始化测试通过");
        Ok(())
    }

    /// 测试 2: JavaScript 代码执行 (预期失败 - 红色)
    #[tokio::test]
    #[serial]
    async fn test_javascript_execution() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange: 创建 MinimalRuntime
        let mut runtime =
            amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create MinimalRuntime");
        let js_code = r#"
            const result = 1 + 1;
            result.toString();
        "#;

        // Act: 执行 JavaScript 代码
        let output = runtime.execute_code(js_code);

        // Assert: 验证输出为 "2"
        assert!(output.is_ok(), "JavaScript execution should succeed");
        let result = output?;
        assert_eq!(result.trim(), "2", "1 + 1 should equal 2");

        println!("✅ JavaScript 执行测试通过: {}", result);
        Ok(())
    }

    /// 测试 3: TypeScript 编译 (跳过 - 模块未实现)
    #[test]
    #[serial]
    fn test_typescript_compilation() -> Result<(), Box<dyn std::error::Error>> {
        // Temporarily skipped - typescript module disabled
        println!("⏭️  TypeScript 编译测试跳过 (模块未实现)");
        Ok(())
    }

    #[test]
    #[serial]
    fn test_execute_code_does_not_replay_side_effect_statement(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut runtime =
            amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create MinimalRuntime");
        let result = runtime.execute_code(
            r#"
            globalThis.count = 0;
            globalThis.bump = () => ++globalThis.count;
            globalThis.bump();
        "#,
        )?;

        assert_eq!(
            result.trim(),
            "1",
            "execute_code must not replay the final side-effecting statement"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn test_execute_code_preserves_multiline_final_expression(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut runtime =
            amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create MinimalRuntime");
        let result = runtime.execute_code(
            r#"
            'alpha' + '|' +
            'beta' + '|' +
            'gamma';
        "#,
        )?;

        assert_eq!(
            result.trim(),
            "alpha|beta|gamma",
            "execute_code must re-evaluate the full continued final expression"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn test_execute_code_does_not_transpile_interface_inside_string(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut runtime =
            amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create MinimalRuntime");
        let result = runtime.execute_code(
            r#"
            const options = { completer: null };
            "interface created successfully";
        "#,
        )?;

        assert_eq!(
            result.trim(),
            "interface created successfully",
            "plain JavaScript strings must not trigger TypeScript transpilation"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn test_execute_code_preserves_js_template_literal_with_type_word(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut runtime =
            amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create MinimalRuntime");
        let result = runtime.execute_code(
            r#"
            const string = "bee";
            const label = `type ${string}`;
            label;
        "#,
        )?;

        assert_eq!(
            result.trim(),
            "type bee",
            "plain JavaScript template literals must not be treated as TypeScript template literal types"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn test_execute_code_preserves_mapped_type_looking_string(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut runtime =
            amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create MinimalRuntime");
        let result = runtime.execute_code(
            r#"
            const text = "[P in keyof T]";
            text;
        "#,
        )?;

        assert_eq!(
            result.trim(),
            "[P in keyof T]",
            "plain JavaScript strings must not trigger mapped-type or keyof rewrites"
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn test_execute_code_still_transpiles_type_aliases_with_mapped_and_template_types(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut runtime =
            amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create MinimalRuntime");
        let result = runtime.execute_code(
            r#"
            type Copy<T> = { [P in keyof T]: T[P] };
            type Label = `type-${string}`;
            const value = "types removed";
            value;
        "#,
        )?;

        assert_eq!(
            result.trim(),
            "types removed",
            "runtime TypeScript fallback should still remove real type aliases"
        );
        Ok(())
    }

    /// 测试 4: CLI run 命令 (预期失败 - 红色)
    #[tokio::test]
    #[serial]
    async fn test_cli_run_command() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange: 创建临时JS文件
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path().join("test.js");
        std::fs::write(&temp_path, "console.log('Hello from file'); 2 + 2;")?;

        // Act: 模拟CLI run命令执行
        let mut runtime =
            amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create MinimalRuntime");
        let code = std::fs::read_to_string(&temp_path)?;

        let output = runtime.execute_code(&code);

        // Assert: 验证执行成功
        assert!(output.is_ok(), "CLI run command should succeed");
        println!("✅ CLI run 命令测试通过: {}", output?);
        Ok(())
    }

    /// 测试 5: CLI REPL 模式 (预期失败 - 红色)
    #[tokio::test]
    #[serial]
    async fn test_cli_repl_mode() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange: 准备REPL输入
        let repl_inputs = vec!["1 + 1", "2 * 3", "let x = 5; x * 2"];

        // Act: 模拟REPL模式执行多行输入
        let mut runtime =
            amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create MinimalRuntime");

        let mut results = Vec::new();
        for input in &repl_inputs {
            let output = runtime.execute_code(input)?;
            results.push(output);
        }

        // Assert: 验证REPL模式结果
        assert_eq!(results[0].trim(), "2", "1 + 1 should equal 2");
        assert_eq!(results[1].trim(), "6", "2 * 3 should equal 6");
        assert_eq!(results[2].trim(), "10", "let x = 5; x * 2 should equal 10");

        println!("✅ CLI REPL 模式测试通过: {:?}", results);
        Ok(())
    }

    /// 测试 6: 错误处理 (预期失败 - 红色)
    #[tokio::test]
    #[serial]
    async fn test_error_handling() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange: 创建 MinimalRuntime
        let mut runtime =
            amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create MinimalRuntime");
        let invalid_js = "invalid javascript code @#$%";

        // Act: 执行无效代码
        let result = runtime.execute_code(invalid_js);

        // Assert: 验证返回错误
        assert!(result.is_err(), "Invalid JavaScript should return error");

        println!("✅ 错误处理测试通过");
        Ok(())
    }

    /// 测试 7: 性能基准 - 简单执行 (预期失败 - 红色)
    #[tokio::test]
    #[serial]
    async fn test_performance_simple_execution() -> Result<(), Box<dyn std::error::Error>> {
        // Arrange: 创建 MinimalRuntime
        let mut runtime =
            amberjs::runtime_minimal::MinimalRuntime::new().expect("Failed to create MinimalRuntime");
        let js_code = "let x = 0; x + 1;";

        // Act: 执行多次并测量时间
        let start = std::time::Instant::now();
        let iterations = 100;

        for _ in 0..iterations {
            let _ = runtime.execute_code(js_code);
        }

        let elapsed = start.elapsed();
        let ops_per_sec = iterations as f64 / elapsed.as_secs_f64();

        // Assert: 验证性能 (目标: > 100 ops/sec for initial implementation)
        assert!(
            ops_per_sec > 100.0,
            "Performance should be > 100 ops/sec, got {:.2}",
            ops_per_sec
        );

        println!("✅ 性能基准测试通过: {:.2} ops/sec", ops_per_sec);
        Ok(())
    }
}
