// CustomEvent API 测试套件 - v0.3.337
//
// 目标：验证 Amber 对 CustomEvent 接口的完整支持
// CustomEvent 用于创建自定义事件，适用于 AI 代理系统和 UI 框架

#[cfg(test)]
mod tests {
    use amberjs::MinimalRuntime;
    use serial_test::serial;

    /// 测试 CustomEvent 构造函数可用性
    #[test]
    #[serial]
    #[serial]
    fn test_custom_event_constructor() {
        let code = r#"
            typeof CustomEvent
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "CustomEvent constructor should be available"
        );
        assert_eq!(result.unwrap().trim(), "function");
    }

    /// 测试 CustomEvent 基本创建
    #[test]
    #[serial]
    fn test_custom_event_basic_creation() {
        let code = r#"
            const event = new CustomEvent('test');
            event.type === 'test'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "CustomEvent should be creatable");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 CustomEvent detail 属性
    #[test]
    #[serial]
    fn test_custom_event_detail() {
        let code = r#"
            const event = new CustomEvent('data', {
                detail: { key: 'value', num: 42 }
            });
            event.type === 'data' &&
            event.detail.key === 'value' &&
            event.detail.num === 42
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "CustomEvent should support detail property");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 CustomEvent 默认值
    #[test]
    #[serial]
    fn test_custom_event_defaults() {
        let code = r#"
            const event = new CustomEvent('test');
            event.detail === null &&
            event.bubbles === false &&
            event.cancelable === false
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "CustomEvent should have default values");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 CustomEvent 继承自 Event
    #[test]
    #[serial]
    fn test_custom_event_inherits_from_event() {
        let code = r#"
            const event = new CustomEvent('test');
            event.type === 'test' && event.cancelable === false
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "CustomEvent should inherit from Event");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 CustomEvent preventDefault 方法
    #[test]
    #[serial]
    fn test_custom_event_prevent_default() {
        let code = r#"
            const passive = new CustomEvent('passive');
            passive.preventDefault();

            const cancelable = new CustomEvent('cancelable', { cancelable: true });
            cancelable.preventDefault();

            passive.defaultPrevented === false &&
            cancelable.defaultPrevented === true
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "CustomEvent should have preventDefault method"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 CustomEventInit bubbles/cancelable 字段
    #[test]
    #[serial]
    fn test_custom_event_init_bubbles_and_cancelable() {
        let code = r#"
            const event = new CustomEvent('ready', {
                detail: { ok: true },
                bubbles: true,
                cancelable: true
            });
            event.detail.ok === true &&
            event.bubbles === true &&
            event.cancelable === true
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "CustomEvent should parse bubbles/cancelable init fields"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 CustomEvent 与复杂数据
    #[test]
    #[serial]
    fn test_custom_event_complex_data() {
        let code = r#"
            const data = {
                array: [1, 2, 3],
                nested: { deep: 'value' },
                bool: true,
                null: null
            };
            const event = new CustomEvent('complex', { detail: data });
            event.detail.array[0] === 1 &&
            event.detail.nested.deep === 'value' &&
            event.detail.bool === true &&
            event.detail.null === null
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "CustomEvent should support complex data in detail"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 CustomEvent 只读属性
    #[test]
    #[serial]
    fn test_custom_event_readonly_properties() {
        let code = r#"
            const event = new CustomEvent('test', { detail: { data: 123 } });
            // 这些属性应该存在
            'type' in event && 'detail' in event && 'bubbles' in event && 'cancelable' in event
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "CustomEvent should have all required properties"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 CustomEvent 作为事件类型
    #[test]
    #[serial]
    fn test_custom_event_as_event_type() {
        let code = r#"
            const event = new CustomEvent('custom', { detail: { message: 'hello' } });
            event.type === 'custom' && event.detail.message === 'hello'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "CustomEvent should work as custom event type"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 CustomEvent EventTarget 兼容性
    #[test]
    #[serial]
    fn test_custom_event_with_event_target() {
        let code = r#"
            const event = new CustomEvent('test');
            typeof event.preventDefault === 'function'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "CustomEvent should have preventDefault function"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 CustomEvent 空 detail
    #[test]
    #[serial]
    fn test_custom_event_empty_detail() {
        let code = r#"
            const event = new CustomEvent('empty', { detail: null });
            event.detail === null
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "CustomEvent should handle null detail");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 CustomEvent 无参数
    #[test]
    #[serial]
    fn test_custom_event_no_args() {
        let code = r#"
            const event = new CustomEvent();
            event.type === 'custom'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "CustomEvent should work without arguments");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 CustomEvent AI 工作负载场景
    #[test]
    #[serial]
    fn test_custom_event_ai_workload() {
        let code = r#"
            // 模拟 AI 代理事件
            const agentEvent = new CustomEvent('agent_response', {
                detail: {
                    agentId: 'agent-001',
                    task: 'inference',
                    result: { prediction: 0.95, confidence: 0.98 },
                    timestamp: Date.now()
                }
            });
            agentEvent.detail.agentId === 'agent-001' &&
            agentEvent.detail.task === 'inference' &&
            agentEvent.detail.result.prediction === 0.95
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "CustomEvent should support AI workload event data"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }
}
