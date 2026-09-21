// EventTarget/Event API 测试套件
//
// 目标：验证 Amber 对 EventTarget、Event 和 CustomEvent 的完整支持

#[cfg(test)]
mod tests {
    use amberjs::MinimalRuntime;
    use serial_test::serial;

    /// 测试 EventTarget 构造函数可用性
    #[test]
    #[serial]
    fn test_event_target_constructor() {
        let code = r#"
            typeof EventTarget
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "EventTarget constructor should be available"
        );
        assert_eq!(result.unwrap().trim(), "function");
    }

    /// 测试 EventTarget 实例创建
    #[test]
    #[serial]
    fn test_event_target_instance() {
        let code = r#"
            const target = new EventTarget();
            typeof target
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "EventTarget instance should be created");
        assert_eq!(result.unwrap().trim(), "object");
    }

    /// 测试 addEventListener 方法可用性
    #[test]
    #[serial]
    fn test_add_event_listener() {
        let code = r#"
            const target = new EventTarget();
            typeof target.addEventListener
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "addEventListener should be available");
        assert_eq!(result.unwrap().trim(), "function");
    }

    /// 测试 addEventListener 错误处理 - 非函数类型
    #[test]
    #[serial]
    fn test_add_event_listener_invalid_type() {
        let code = r#"
            const target = new EventTarget();
            try {
                target.addEventListener('test', 'not a function');
                false;
            } catch (e) {
                true;
            }
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "addEventListener should throw error for non-function"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 removeEventListener 方法可用性
    #[test]
    #[serial]
    fn test_remove_event_listener() {
        let code = r#"
            const target = new EventTarget();
            typeof target.removeEventListener
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "removeEventListener should be available");
        assert_eq!(result.unwrap().trim(), "function");
    }

    /// 测试 dispatchEvent 方法可用性
    #[test]
    #[serial]
    fn test_dispatch_event() {
        let code = r#"
            const target = new EventTarget();
            typeof target.dispatchEvent
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "dispatchEvent should be available");
        assert_eq!(result.unwrap().trim(), "function");
    }

    /// 测试基本事件监听和派发
    #[test]
    #[serial]
    fn test_event_listen_and_dispatch() {
        let code = r#"
            let count = 0;
            const target = new EventTarget();
            target.addEventListener('test', () => { count++; });
            target.dispatchEvent(new Event('test'));
            count === 1
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Event listener should be called");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试多个事件监听器
    #[test]
    #[serial]
    fn test_multiple_event_listeners() {
        let code = r#"
            let count = 0;
            const target = new EventTarget();
            target.addEventListener('test', () => { count++; });
            target.addEventListener('test', () => { count++; });
            target.dispatchEvent(new Event('test'));
            count === 2
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Multiple listeners should all be called");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试不同事件类型
    #[test]
    #[serial]
    fn test_different_event_types() {
        let code = r#"
            let test1Called = false;
            let test2Called = false;
            const target = new EventTarget();
            target.addEventListener('event1', () => { test1Called = true; });
            target.addEventListener('event2', () => { test2Called = true; });
            target.dispatchEvent(new Event('event1'));
            target.dispatchEvent(new Event('event2'));
            test1Called && test2Called
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "Different event types should work independently"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 Event 构造函数可用性
    #[test]
    #[serial]
    fn test_event_constructor() {
        let code = r#"
            typeof Event
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Event constructor should be available");
        assert_eq!(result.unwrap().trim(), "function");
    }

    /// 测试 Event 实例属性
    #[test]
    #[serial]
    fn test_event_properties() {
        let code = r#"
            const event = new Event('test');
            event.type === 'test' && event.bubbles === false && event.cancelable === false
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Event properties should be set correctly");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 CustomEvent 构造函数可用性
    #[test]
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

    /// 测试 CustomEvent detail 属性
    #[test]
    #[serial]
    fn test_custom_event_detail() {
        let code = r#"
            const detail = { foo: 'bar' };
            const event = new CustomEvent('test', detail);
            event.detail && event.detail.foo === 'bar'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "CustomEvent detail should be set correctly");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试事件中传递数据
    #[test]
    #[serial]
    fn test_event_with_data() {
        let code = r#"
            let receivedData = null;
            const target = new EventTarget();
            target.addEventListener('data', (e) => { receivedData = e.detail; });
            target.dispatchEvent(new CustomEvent('data', { data: { value: 42 } }));
            receivedData && receivedData.value === 42
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Event data should be passed to listener");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 preventDefault 方法
    #[test]
    #[serial]
    fn test_prevent_default() {
        let code = r#"
            const passive = new Event('passive');
            passive.preventDefault();

            const cancelable = new Event('cancelable', { cancelable: true });
            cancelable.preventDefault();

            passive.defaultPrevented === false &&
            cancelable.defaultPrevented === true
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "preventDefault should set defaultPrevented flag"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 EventInit bubbles/cancelable 语义和 dispatchEvent 返回值
    #[test]
    #[serial]
    fn test_event_init_and_dispatch_default_prevented() {
        let code = r#"
            const target = new EventTarget();
            target.addEventListener('submit', (event) => event.preventDefault());

            const passive = new Event('submit');
            const cancelable = new Event('submit', { bubbles: true, cancelable: true });

            const passiveResult = target.dispatchEvent(passive);
            const cancelableResult = target.dispatchEvent(cancelable);

            `${passive.bubbles}:${passive.defaultPrevented}:${passiveResult}:${cancelable.bubbles}:${cancelable.defaultPrevented}:${cancelableResult}`;
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "EventInit should control cancelability");
        assert_eq!(result.unwrap().trim(), "false:false:true:true:true:false");
    }

    /// 测试扩展 EventTarget
    #[test]
    #[serial]
    fn test_extend_event_target() {
        let code = r#"
            class MyEmitter extends EventTarget {
                emit(eventType) {
                    this.dispatchEvent(new Event(eventType));
                }
            }
            let called = false;
            const emitter = new MyEmitter();
            emitter.addEventListener('test', () => { called = true; });
            emitter.emit('test');
            called
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "EventTarget should be extendable");
        assert_eq!(result.unwrap().trim(), "true");
    }
}
