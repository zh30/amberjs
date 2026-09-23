// Stage 89 Phase 2: 错误处理模块
// 提供统一的错误处理、自动恢复和错误上下文管理
pub mod recovery;
pub mod types;
pub use recovery::{
    AutoRecovery, AutoRecoveryConfig, FallbackStrategyFn, RecoveryStats, RetryPolicy,
};
pub use types::{AmberError, ErrorContext, ErrorSeverity, SourceLocation, StackFrame};
/// 创建错误上下文的便捷函数
pub fn create_error_context(
    error: AmberError,
    file: String,
    line: u32,
    function: String,
) -> ErrorContext {
    ErrorContext::new(error, file, line, function)
}
/// 创建简单错误上下文的便捷函数
pub fn create_simple_error_context(error: AmberError) -> ErrorContext {
    ErrorContext::new_without_location(error)
}
/// 错误处理结果类型
pub type Result<T> = std::result::Result<T, AmberError>;
/// 全局错误处理配置
#[derive(Debug, Clone)]
pub struct GlobalErrorConfig {
    pub enable_auto_recovery: bool,
    pub enable_fallback: bool,
    pub max_error_history: usize,
    pub enable_error_logging: bool,
}
impl Default for GlobalErrorConfig {
    fn default() -> Self {
        Self {
            enable_auto_recovery: true,
            enable_fallback: true,
            max_error_history: 1000,
            enable_error_logging: true,
        }
    }
}
/// 错误处理工具函数
pub struct ErrorHandler;
impl ErrorHandler {
    /// 包装可能出错的操作，提供错误上下文
    pub async fn wrap_with_context<F, T, Fut>(
        error_type: AmberError,
        file: &'static str,
        line: u32,
        function: &'static str,
        f: F,
    ) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        match f().await {
            Ok(result) => Ok(result),
            Err(original_error) => {
                // 创建错误上下文
                let context: _ =
                    ErrorContext::new(original_error, file.to_string(), line, function.to_string());
                // 记录错误
                if GlobalErrorConfig::default().enable_error_logging {
                    eprintln!("Error context: {}", context);
                }
                // 返回包装后的错误
                Err(error_type)
            }
        }
    }
    /// 安全执行可能出错的操作
    pub async fn safe_execute<F, T, Fut>(
        _file: &'static str,
        _line: u32,
        _function: &'static str,
        f: F,
    ) -> Result<T>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        match f().await {
            Ok(result) => Ok(result),
            Err(error) => {
                let context: _ = ErrorContext::new_without_location(error);
                eprintln!("Execution failed: {}", context);
                Err(context.error_type)
            }
        }
    }
    /// 转换任意错误为 AmberError
    pub fn convert_error<E: std::fmt::Display + 'static>(error: E) -> AmberError {
        let any = &error as &dyn std::any::Any;
        if let Some(amberjs_err) = any.downcast_ref::<AmberError>() {
            return amberjs_err.clone();
        }
        AmberError::RuntimeError(error.to_string())
    }
}
/// 错误宏
#[macro_export]
macro_rules! amberjs_try {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => {
                return Err($crate::error::ErrorHandler::convert_error(error));
            }
        }
    };
    ($result:expr, $context:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => {
                let context: _ = $crate::error::ErrorContext::new_without_location(
                    $crate::error::ErrorHandler::convert_error(error),
                );
                eprintln!("Error context: {}", context);
                return Err(context.error_type);
            }
        }
    };
}
/// 异步错误处理宏
#[macro_export]
macro_rules! amberjs_try_async {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => {
                return Err($crate::error::ErrorHandler::convert_error(error));
            }
        }
    };
    ($result:expr, $file:expr, $line:expr, $function:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => {
                let context: _ = $crate::error::ErrorContext::new(
                    $crate::error::ErrorHandler::convert_error(error),
                    $file.to_string(),
                    $line,
                    $function.to_string(),
                );
                eprintln!("Error context: {}", context);
                return Err(context.error_type);
            }
        }
    };
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_error_context_creation() {
        let error: _ = AmberError::V8Error("Test error".to_string());
        let context: _ = create_error_context(
            error.clone(),
            "test.rs".to_string(),
            42,
            "test_function".to_string(),
        );
        assert_eq!(context.error_type, error);
        assert_eq!(context.source_location.as_ref().unwrap().file, "test.rs");
        assert_eq!(context.source_location.as_ref().unwrap().line, 42);
    }
    #[tokio::test]
    async fn test_simple_error_context() {
        let error: _ = AmberError::RuntimeError("Simple error".to_string());
        let context: _ = create_simple_error_context(error.clone());
        assert_eq!(context.error_type, error);
        assert!(context.source_location.is_none());
    }
    #[tokio::test]
    async fn test_error_conversion() {
        let std_error: _ = "Standard error";
        let amberjs_error: _ = ErrorHandler::convert_error(std_error);
        assert!(matches!(amberjs_error, AmberError::RuntimeError(_)));
    }
    #[test]
    fn test_amberjs_try_macro() -> Result<()> {
        let result: Result<i32> = Ok(42);
        let value: _ = amberjs_try!(result);
        assert_eq!(value, 42);
        Ok(())
    }
    #[test]
    fn test_amberjs_try_macro_error() {
        fn helper() -> Result<()> {
            let result: Result<i32> = Err(AmberError::V8Error("Test".to_string()));
            let _error: _ = amberjs_try!(result);
            // This test verifies macro propagates error
            assert!(false); // Should not reach here
            Ok(())
        }
        let res = helper();
        assert!(res.is_err());
        assert!(matches!(res.err().unwrap(), AmberError::V8Error(_)));
    }
}
