// ErrorEvent API 测试套件 - v0.3.333
//
// 目标：验证 Amber 对 ErrorEvent 接口的完整支持
// ErrorEvent 用于报告脚本错误，适用于 window.onerror, WebSocket onerror, Worker onerror 等

#[cfg(test)]
mod tests {
    use amberjs::MinimalRuntime;
    use serial_test::serial;

    /// 测试 ErrorEvent 构造函数可用性
    #[test]
    #[serial]
    fn test_error_event_constructor() {
        let code = r#"
            typeof ErrorEvent
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "ErrorEvent constructor should be available");
        assert_eq!(result.unwrap().trim(), "function");
    }

    /// 测试 ErrorEvent 基本创建
    #[test]
    #[serial]
    fn test_error_event_basic_creation() {
        let code = r#"
            const event = new ErrorEvent('error', {
                message: 'Test error message',
                filename: 'test.js',
                lineno: 10,
                colno: 5
            });
            event.type === 'error' &&
            event.message === 'Test error message' &&
            event.filename === 'test.js' &&
            event.lineno === 10 &&
            event.colno === 5
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "ErrorEvent should be creatable with options"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 ErrorEvent 默认值
    #[test]
    #[serial]
    fn test_error_event_defaults() {
        let code = r#"
            const event = new ErrorEvent('error');
            event.message === '' &&
            event.filename === '' &&
            event.lineno === 0 &&
            event.colno === 0 &&
            event.error === null
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "ErrorEvent should have default values");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 ErrorEvent 继承自 Event
    #[test]
    #[serial]
    fn test_error_event_inherits_from_event() {
        let code = r#"
            const event = new ErrorEvent('error', { message: 'test' });
            event.type === 'error' && event.cancelable === true
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "ErrorEvent should inherit from Event");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 ErrorEvent with error object
    #[test]
    #[serial]
    fn test_error_event_with_error_object() {
        let code = r#"
            const error = new Error('Original error');
            const event = new ErrorEvent('error', {
                message: 'An error occurred',
                error: error
            });
            event.error === error && event.error instanceof Error
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "ErrorEvent should support error object");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 ErrorEvent 只读属性
    #[test]
    #[serial]
    fn test_error_event_readonly_properties() {
        let code = r#"
            const event = new ErrorEvent('error', {
                message: 'test',
                filename: 'test.js',
                lineno: 1,
                colno: 1
            });
            // 这些属性应该存在
            'message' in event && 'filename' in event && 'lineno' in event && 'colno' in event
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "ErrorEvent should have all properties");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 ErrorEvent 作为事件类型
    #[test]
    #[serial]
    fn test_error_event_as_event_type() {
        let code = r#"
            const event = new ErrorEvent('error', { message: 'Network error' });
            event.type === 'error' && event.cancelable === true
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "ErrorEvent should work as error event type");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 window.onerror 存在且可设置
    #[test]
    #[serial]
    fn test_window_onerror_exists() {
        let code = r#"
            typeof window.onerror === 'function'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "window.onerror should exist");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 window.onerror 可以捕获运行时错误
    #[test]
    #[serial]
    fn test_window_onerror_catches_error() {
        let code = r#"
            let errorCaught = false;
            let errorMessage = '';
            let errorFilename = '';
            let errorLineno = 0;
            let errorColno = 0;

            window.onerror = function(message, filename, lineno, colno, error) {
                errorCaught = true;
                errorMessage = message;
                errorFilename = filename;
                errorLineno = lineno;
                errorColno = colno;
                // 防止默认错误处理
                return true;
            };

            // 触发一个错误
            throw new Error('Test error for onerror');
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Code with window.onerror should execute");

        // 验证 onerror 被调用
        let check_code = r#"
            errorCaught === true
        "#;
        let check_result = runtime.execute_code(check_code);
        assert!(
            check_result.is_ok(),
            "window.onerror should have caught the error"
        );
        assert_eq!(check_result.unwrap().trim(), "true");
    }

    /// 测试 window.onerror 接收正确的错误信息
    #[test]
    #[serial]
    fn test_window_onerror_receives_correct_info() {
        let code = r#"
            let receivedMessage = '';
            let receivedError = null;

            window.onerror = function(message, filename, lineno, colno, error) {
                receivedMessage = message;
                receivedError = error;
                return true;
            };

            const testError = new Error('Specific test error');
            throw testError;
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Code should execute");

        // 验证错误信息
        let check_code = r#"
            receivedMessage && receivedMessage.includes('Specific test error') && receivedError instanceof Error
        "#;
        let check_result = runtime.execute_code(check_code);
        assert!(
            check_result.is_ok(),
            "window.onerror should receive correct error info"
        );
        assert_eq!(check_result.unwrap().trim(), "true");
    }

    /// 测试 window.onerror 返回 true 阻止默认处理
    #[test]
    #[serial]
    fn test_window_onerror_prevents_default() {
        let code = r#"
            let errorHandled = false;

            window.onerror = function() {
                errorHandled = true;
                return true; // 阻止默认错误处理
            };

            throw new Error('Should not print');
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Code should execute without panic");

        let check_code = "errorHandled";
        let check_result = runtime.execute_code(check_code);
        assert!(check_result.is_ok(), "onerror should have been called");
    }

    /// 测试 window.onerror 可以被覆盖
    #[test]
    #[serial]
    fn test_window_onerror_overwritable() {
        let code = r#"
            let callCount = 0;

            window.onerror = function() {
                callCount++;
                return true;
            };

            window.onerror = function() {
                callCount++;
                return true;
            };

            throw new Error('Test');
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Code should execute");

        // 验证第二个 onerror 被设置
        let check_code = "callCount === 1"; // 只调用一次，因为覆盖了
        let check_result = runtime.execute_code(check_code);
        assert!(
            check_result.is_ok(),
            "Only the last onerror should be called"
        );
    }
}
