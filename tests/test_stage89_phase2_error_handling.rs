// Stage 89 Phase 2: 错误处理增强 - 测试套件
// 测试统一错误处理系统、自动恢复机制和优雅降级

#[cfg(test)]
mod stage89_phase2_error_handling_tests {
    use amberjs::error::{AutoRecovery, BeejsError, ErrorContext};
    use amberjs::fallback::{FallbackManager, FallbackStrategy, Feature};
    use std::time::Duration;

    /// 测试 1: 错误分类和错误上下文
    #[tokio::test]
    async fn test_error_classification() {
        let v8_error = BeejsError::V8Error("Invalid handle access".to_string());
        assert!(matches!(v8_error, BeejsError::V8Error(_)));

        let js_error = BeejsError::JsExecutionError("TypeError: Cannot read property".to_string());
        assert!(matches!(js_error, BeejsError::JsExecutionError(_)));

        let multi_error = BeejsError::MultiLanguageError("Python module not found".to_string());
        assert!(matches!(multi_error, BeejsError::MultiLanguageError(_)));

        let platform_error = BeejsError::PlatformError("iOS runtime unavailable".to_string());
        assert!(matches!(platform_error, BeejsError::PlatformError(_)));
    }

    /// 测试 2: 错误上下文信息
    #[tokio::test]
    async fn test_error_context() {
        let error = BeejsError::V8Error("Test V8 error".to_string());
        let context = ErrorContext::new(
            error.clone(),
            "test_file.js".to_string(),
            42,
            "test_function".to_string(),
        );

        assert_eq!(context.error_type, error);
        assert_eq!(
            context.source_location.as_ref().unwrap().file,
            "test_file.js"
        );
        assert_eq!(context.source_location.as_ref().unwrap().line, 42);
        assert_eq!(
            context.source_location.as_ref().unwrap().function,
            "test_function"
        );
        assert!(!context.recovery_suggestions.is_empty());
    }

    /// 测试 3: 自动恢复机制 - 重试策略
    #[tokio::test]
    async fn test_auto_recovery_retry() {
        let recovery = AutoRecovery::new()
            .with_max_retries(3)
            .with_base_delay(Duration::from_millis(10));

        let result = recovery
            .recover_from_error(&BeejsError::V8Error("Transient error".to_string()))
            .await;
        assert!(result.is_ok());
    }

    /// 测试 4: 自动恢复机制 - 回退策略
    #[tokio::test]
    async fn test_auto_recovery_fallback() {
        let recovery = AutoRecovery::new().with_fallback_strategy(Box::new(|error| {
            if matches!(error, BeejsError::V8Error(_)) {
                Some("Use simplified API".to_string())
            } else {
                None
            }
        }));

        let error = BeejsError::V8Error("Complex API failed".to_string());
        let result = recovery.recover_from_error(&error).await;
        assert!(result.is_ok());
    }

    /// 测试 5: 降级策略 - 禁用功能
    #[tokio::test]
    async fn test_fallback_disable_feature() {
        let mut manager = FallbackManager::new();
        manager
            .register_strategy(Feature::V8Optimization, FallbackStrategy::DisableFeature)
            .await;

        let result = manager
            .handle_feature_failure(Feature::V8Optimization)
            .await;
        assert!(result.is_ok());
    }

    /// 测试 6: 降级策略 - 使用替代方案
    #[tokio::test]
    async fn test_fallback_alternative() {
        let mut manager = FallbackManager::new();
        manager
            .register_strategy(
                Feature::PythonRuntime,
                FallbackStrategy::UseAlternative("Use Python subprocess".to_string()),
            )
            .await;

        let result = manager.handle_feature_failure(Feature::PythonRuntime).await;
        assert!(result.is_ok());
    }

    /// 测试 7: 降级策略 - 延迟重试
    #[tokio::test]
    async fn test_fallback_retry_later() {
        let mut manager = FallbackManager::new();
        manager
            .register_strategy(
                Feature::WebAssembly,
                FallbackStrategy::RetryLater(Duration::from_millis(100)),
            )
            .await;

        let result = manager.handle_feature_failure(Feature::WebAssembly).await;
        assert!(result.is_ok());
    }

    /// 测试 8: 集成测试 - 错误处理到降级的完整流程
    #[tokio::test]
    async fn test_error_to_fallback_integration() {
        let recovery = AutoRecovery::new().with_max_retries(1);
        let mut fallback = FallbackManager::new();

        fallback
            .register_strategy(
                Feature::V8Optimization,
                FallbackStrategy::UseAlternative("Fallback to basic mode".to_string()),
            )
            .await;

        // 模拟 V8 错误并尝试恢复
        let error = BeejsError::V8Error("Critical optimization failed".to_string());
        let recovery_result = recovery.recover_from_error(&error).await;

        // 如果恢复失败，使用降级策略
        if recovery_result.is_err() {
            let fallback_result = fallback
                .handle_feature_failure(Feature::V8Optimization)
                .await;
            assert!(fallback_result.is_ok());
        }
    }

    /// 测试 9: 错误恢复建议生成
    #[tokio::test]
    async fn test_recovery_suggestions() {
        let error = BeejsError::MultiLanguageError("Go runtime not initialized".to_string());
        let context = ErrorContext::new(error, "main.go".to_string(), 1, "main".to_string());

        let suggestions = context.get_recovery_suggestions();
        assert!(!suggestions.is_empty());
        assert!(suggestions
            .iter()
            .any(|s| s.contains("Initialize") || s.contains("runtime")));
    }

    /// 测试 10: 性能 - 错误处理延迟
    #[tokio::test]
    async fn test_error_handling_performance() {
        let start = std::time::Instant::now();
        let recovery = AutoRecovery::new();

        for _ in 0..100 {
            let _ = recovery
                .recover_from_error(&BeejsError::V8Error("Test".to_string()))
                .await;
        }

        let duration = start.elapsed();
        // 100 次错误处理应该在 100ms 内完成
        assert!(
            duration < Duration::from_millis(100),
            "Error handling took too long: {:?}",
            duration
        );
    }

    /// 测试 11: 多错误类型上下文
    #[tokio::test]
    async fn test_multiple_error_contexts() {
        let errors = [
            BeejsError::V8Error("Error 1".to_string()),
            BeejsError::JsExecutionError("Error 2".to_string()),
            BeejsError::PlatformError("Error 3".to_string()),
        ];

        for (i, error) in errors.iter().enumerate() {
            let context = ErrorContext::new(
                error.clone(),
                format!("file{}.js", i),
                (i + 1) as u32,
                format!("function{}", i),
            );

            assert_eq!(
                context.source_location.as_ref().unwrap().file,
                format!("file{}.js", i)
            );
            assert_eq!(
                context.source_location.as_ref().unwrap().line,
                (i + 1) as u32
            );
        }
    }

    /// 测试 12: Fallback 策略链
    #[tokio::test]
    async fn test_fallback_strategy_chain() {
        let mut manager = FallbackManager::new();

        // 注册多个降级策略
        manager
            .register_strategy(
                Feature::V8Optimization,
                FallbackStrategy::RetryLater(Duration::from_millis(10)),
            )
            .await;

        manager
            .register_strategy(
                Feature::V8Optimization,
                FallbackStrategy::UseAlternative("Alternative implementation".to_string()),
            )
            .await;

        let result = manager
            .handle_feature_failure(Feature::V8Optimization)
            .await;
        assert!(result.is_ok());
    }
}
