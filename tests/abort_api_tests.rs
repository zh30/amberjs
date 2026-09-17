// AbortController/AbortSignal API 测试套件 - v0.3.340
//
// 目标：验证 Beejs 对 AbortController 和 AbortSignal 接口的完整支持
// 用于异步操作取消、fetch 请求取消等场景

#[cfg(test)]
mod tests {
    use amberjs::MinimalRuntime;
    use serial_test::serial;

    /// 测试 AbortController 构造函数可用性
    #[test]
    #[serial]
    fn test_abort_controller_constructor() {
        let code = r#"
            typeof AbortController
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "AbortController constructor should be available"
        );
        assert_eq!(result.unwrap().trim(), "function");
    }

    /// 测试 AbortSignal 全局对象可用性 (v0.3.340 修复)
    #[test]
    #[serial]
    fn test_abort_signal_global() {
        let code = r#"
            typeof AbortSignal
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "AbortSignal should be available globally");
        assert_eq!(result.unwrap().trim(), "object");
    }

    /// 测试 AbortController 基本创建
    #[test]
    #[serial]
    fn test_abort_controller_basic() {
        let code = r#"
            const controller = new AbortController();
            controller.signal.aborted === false
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "AbortController should be creatable");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 abort() 方法设置 aborted 状态
    #[test]
    #[serial]
    fn test_abort_sets_aborted() {
        let code = r#"
            const controller = new AbortController();
            controller.abort();
            controller.signal.aborted === true
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "abort() should set aborted to true");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 AbortSignal.aborted 静态属性
    #[test]
    #[serial]
    fn test_abort_signal_static_aborted() {
        let code = r#"
            AbortSignal.aborted === false
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "AbortSignal.aborted should be false initially"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 abort 事件监听器
    #[test]
    #[serial]
    fn test_abort_event_listener() {
        let code = r#"
            let called = false;
            const controller = new AbortController();
            controller.signal.addEventListener('abort', () => { called = true; });
            controller.abort();
            called === true
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "abort event listener should be called");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试多个 abort 事件监听器
    #[test]
    #[serial]
    fn test_multiple_abort_listeners() {
        let code = r#"
            let count = 0;
            const controller = new AbortController();
            controller.signal.addEventListener('abort', () => { count++; });
            controller.signal.addEventListener('abort', () => { count++; });
            controller.signal.addEventListener('abort', () => { count++; });
            controller.abort();
            count === 3
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "all abort listeners should be called");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 AbortController.signal 属性
    #[test]
    #[serial]
    fn test_abort_controller_signal() {
        let code = r#"
            const controller = new AbortController();
            const signal = controller.signal;
            typeof signal === 'object' && signal !== null
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "controller.signal should be an object");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 abort() 在已中止信号上重复调用
    #[test]
    #[serial]
    fn test_double_abort() {
        let code = r#"
            let count = 0;
            const controller = new AbortController();
            controller.signal.addEventListener('abort', () => { count++; });
            controller.abort();
            controller.abort();
            count >= 1
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "abort should trigger at least once");
        assert_eq!(result.unwrap().trim(), "true");
    }
}
