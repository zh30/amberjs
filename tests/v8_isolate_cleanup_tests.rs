// 测试 V8 Isolate 清理问题
// 验证 Isolate 在异常情况下的正确清理

use amberjs::Runtime;

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// 测试正常情况下的 Isolate 清理
    #[test]
    fn test_isolate_cleanup_normal_case() {
        let runtime = Runtime::new(67108864, 1073741824, false, false);

        // 执行正常代码
        let result = runtime.execute_code("const x = 1; x + 1;");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "2");

        // 验证运行时仍然可用
        let result = runtime.execute_code("console.log('still working'); 'ok'");
        assert!(result.is_ok());
    }

    /// 测试异常情况下的 Isolate 清理
    #[test]
    fn test_isolate_cleanup_with_exception() {
        let runtime = Runtime::new(67108864, 1073741824, false, false);

        // 执行会导致异常的代码
        let result = runtime.execute_code("throw new Error('test error');");
        assert!(result.is_err());

        // 验证运行时在异常后仍然可用
        let result = runtime.execute_code("const y = 2; y * 2;");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "4");
    }

    /// 测试文件执行异常情况下的清理
    #[test]
    fn test_isolate_cleanup_with_file_exception() {
        let runtime = Runtime::new(67108864, 1073741824, false, false);

        // 创建一个包含异常的文件
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "throw new Error('file error');").unwrap();
        let file_path = file.path().to_path_buf();

        // 执行会导致异常的文件
        let result = runtime.execute_file(&file_path);
        assert!(result.is_err());

        // 验证运行时在异常后仍然可用
        let result = runtime.execute_code("const z = 3; z + 3;");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "6");
    }

    /// 测试连续异常情况下的清理
    #[test]
    fn test_isolate_cleanup_multiple_exceptions() {
        let runtime = Runtime::new(67108864, 1073741824, false, false);

        // 执行多个会导致异常的代码
        for i in 1..=5 {
            let code = format!("throw new Error('error {}');", i);
            let result = runtime.execute_code(&code);
            assert!(result.is_err(), "Iteration {} should fail", i);
        }

        // 验证运行时在多个异常后仍然可用
        let result = runtime.execute_code("const sum = 10; sum * 2;");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "20");
    }

    /// 测试正常和异常代码混合执行
    #[test]
    fn test_isolate_cleanup_mixed_execution() {
        let runtime = Runtime::new(67108864, 1073741824, false, false);

        // 正常执行
        let result = runtime.execute_code("const a = 1;");
        assert!(result.is_ok());

        // 异常执行
        let result = runtime.execute_code("throw new Error('error');");
        assert!(result.is_err());

        // 再次正常执行
        let result = runtime.execute_code("const b = 2; a + b;");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "3");

        // 又一个异常
        let result = runtime.execute_code("undefined.nonExistentMethod();");
        assert!(result.is_err());

        // 最后一次正常执行
        let result = runtime.execute_code("const c = 3; c * 3;");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "9");
    }
}
