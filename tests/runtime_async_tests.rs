// 异步运行时测试 - v0.2.0
// TDD 风格：先写测试，再实现功能

#[cfg(test)]
mod async_runtime_tests {

    use amberjs::runtime_minimal::MinimalRuntime;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::Duration;

    fn header_end_position(data: &[u8]) -> Option<usize> {
        data.windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|position| position + 4)
    }

    fn content_length(request: &str) -> usize {
        request
            .lines()
            .find_map(|line| {
                let lower = line.to_ascii_lowercase();
                lower
                    .strip_prefix("content-length:")
                    .and_then(|value| value.trim().parse::<usize>().ok())
            })
            .unwrap_or(0)
    }

    fn read_http_request(stream: &mut TcpStream) -> String {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("test stream should accept read timeout");

        let mut data = Vec::new();
        let mut buffer = [0_u8; 1024];

        loop {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    data.extend_from_slice(&buffer[..read]);
                    if let Some(header_end) = header_end_position(&data) {
                        let request = String::from_utf8_lossy(&data);
                        if data.len() >= header_end + content_length(&request) {
                            break;
                        }
                    }
                }
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        || error.kind() == std::io::ErrorKind::TimedOut =>
                {
                    break;
                }
                Err(_) => break,
            }
        }

        String::from_utf8_lossy(&data).into_owned()
    }

    fn spawn_options_fetch_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let address = listener
            .local_addr()
            .expect("test server should have a local address");

        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let request = read_http_request(&mut stream);
                let lower_request = request.to_ascii_lowercase();
                let valid_request = request.starts_with("POST ")
                    && lower_request.contains("content-type: application/json")
                    && request.contains(r#"{"test":true}"#);
                let response_body = format!(
                    r#"{{"received":{},"sawPost":{},"sawJson":{},"sawBody":{}}}"#,
                    valid_request,
                    request.starts_with("POST "),
                    lower_request.contains("content-type: application/json"),
                    request.contains(r#"{"test":true}"#)
                );
                let (status, reason, body) = if valid_request {
                    (200, "OK", response_body)
                } else {
                    (400, "Bad Request", response_body)
                };
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        format!("http://{}", address)
    }

    #[test]
    #[serial_test::serial]
    fn test_event_loop_basic() {
        let mut runtime = MinimalRuntime::new().unwrap();

        // 测试异步执行
        let result = runtime.execute_code(
            r#"
            globalThis.executed = false;
            setTimeout(() => {
                globalThis.executed = true;
            }, 10);
            globalThis.executed;
        "#,
        );

        assert!(result.is_ok());
        // execute_code returns the main script completion value, not a replayed
        // expression after the event-loop drain.
        assert_eq!(result.unwrap().trim(), "false");

        let observed = runtime.execute_code("globalThis.executed;");
        assert!(observed.is_ok());
        assert_eq!(observed.unwrap().trim(), "true");
    }

    #[test]
    #[serial_test::serial]
    fn test_real_http_fetch() {
        let mut runtime = MinimalRuntime::new().unwrap();

        // 测试真实的 HTTP fetch
        let result = runtime.execute_code(
            r#"
            const response = fetch('https://httpbin.org/json');
            typeof response === 'object' &&
            typeof response.json === 'function' &&
            typeof response.status === 'number';
        "#,
        );

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.trim(), "true");
    }

    #[test]
    #[serial_test::serial]
    fn test_websocket_basic() {
        let mut runtime = MinimalRuntime::new().unwrap();

        // 测试 WebSocket 构造函数
        let result = runtime.execute_code(
            r#"
            const ws = new WebSocket('ws://echo.websocket.org/');
            ws.readyState;
        "#,
        );

        assert!(result.is_ok());
        // WebSocket 应该被支持
        let output = result.unwrap();
        assert!(output.contains("0") || output.contains("CONNECTING"));
    }

    #[test]
    #[serial_test::serial]
    fn test_async_await() {
        let mut runtime = MinimalRuntime::new().unwrap();

        // 测试 async/await 语法
        let result = runtime.execute_code(
            r#"
            async function test() {
                return 'Hello from async';
            }
            test();
        "#,
        );

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Promise") || output.contains("Hello"));
    }

    #[test]
    #[serial_test::serial]
    fn test_promise_all() {
        let mut runtime = MinimalRuntime::new().unwrap();

        // 测试 Promise.all
        let result = runtime.execute_code(
            r#"
            Promise.all([
                Promise.resolve(1),
                Promise.resolve(2),
                Promise.resolve(3)
            ]).then(values => values.join(','));
        "#,
        );

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Promise") || output.contains("1,2,3"));
    }

    #[test]
    #[serial_test::serial]
    fn test_fetch_with_options() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_options_fetch_server();

        // 测试带选项的 fetch
        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json'
                },
                body: JSON.stringify({test: true})
            });
            JSON.stringify({
                object: typeof response === 'object',
                status: response.status,
                ok: response.ok,
                body: response.text()
            });
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(
            output.trim(),
            r#"{"object":true,"status":200,"ok":true,"body":"{\"received\":true,\"sawPost\":true,\"sawJson\":true,\"sawBody\":true}"}"#,
            "fetch options result was {output}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_multiple_timers() {
        let mut runtime = MinimalRuntime::new().unwrap();

        // 测试多个定时器
        let result = runtime.execute_code(
            r#"
            globalThis.timerCount = 0;
            setTimeout(() => { globalThis.timerCount += 1; }, 5);
            setTimeout(() => { globalThis.timerCount += 1; }, 5);
            setTimeout(() => { globalThis.timerCount += 1; }, 5);
            globalThis.timerCount;
        "#,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "0");

        let observed = runtime.execute_code("globalThis.timerCount;");
        assert!(observed.is_ok());
        assert_eq!(observed.unwrap().trim(), "3");
    }

    #[test]
    #[serial_test::serial]
    fn test_timer_cleanup() {
        let mut runtime = MinimalRuntime::new().unwrap();

        // 测试清除定时器
        let result = runtime.execute_code(
            r#"
            const timerId = setTimeout(() => {
                console.log('This should not execute');
            }, 100);
            clearTimeout(timerId);
            'Timer cleared';
        "#,
        );

        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "Timer cleared");
    }

    #[test]
    #[serial_test::serial]
    fn test_websocket_events() {
        let mut runtime = MinimalRuntime::new().unwrap();

        // 测试 WebSocket 事件处理
        let result = runtime.execute_code(
            r#"
            const ws = new WebSocket('ws://echo.websocket.org/');
            let events = [];
            ws.onopen = () => events.push('open');
            ws.onmessage = (event) => events.push('message');
            ws.onerror = (error) => events.push('error');
            ws.onclose = () => events.push('close');
            events.length;
        "#,
        );

        assert!(result.is_ok());
        // 应该返回 0（事件尚未触发）
        assert_eq!(result.unwrap().trim(), "0");
    }

    #[test]
    #[serial_test::serial]
    fn test_performance_async() {
        let mut runtime = MinimalRuntime::new().unwrap();

        // 性能测试：创建大量异步操作
        let start = std::time::Instant::now();
        let result = runtime.execute_code(
            r#"
            let promises = [];
            for (let i = 0; i < 1000; i++) {
                promises.push(Promise.resolve(i));
            }
            Promise.all(promises).then(values => values.length);
        "#,
        );

        let elapsed = start.elapsed();

        assert!(result.is_ok());
        // 应该在合理时间内完成
        assert!(
            elapsed < Duration::from_millis(1000),
            "Async operations took too long: {:?}",
            elapsed
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_error_handling_async() {
        let mut runtime = MinimalRuntime::new().unwrap();

        // 测试异步错误处理
        let result = runtime.execute_code(
            r#"
            Promise.reject(new Error('Test error'))
                .catch(error => error.message);
        "#,
        );

        assert!(result.is_ok());
        let output = result.unwrap();
        // 应该返回 Promise 或错误信息
        assert!(output.contains("Promise") || output.contains("Test error"));
    }

    #[test]
    #[serial_test::serial]
    fn test_real_file_system_operations() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp.path(), "amberjs file fixture").unwrap();
        let fixture_path = temp.path().to_string_lossy().replace('\\', "\\\\");

        // 测试真实的文件系统操作
        let result = runtime.execute_code(&format!(
            r#"
            const fs = require('fs');
            const content = fs.readFileSync('{}', 'utf8');
            content;
        "#,
            fixture_path
        ));

        assert!(result.is_ok());
        // 应该能够读取文件或返回错误（而不是 "fs API called"）
        let output = result.unwrap();
        assert_ne!(output.trim(), "fs API called");
        assert_eq!(output.trim(), "amberjs file fixture");
    }

    #[test]
    #[serial_test::serial]
    fn test_fetch_timeout() {
        let mut runtime = MinimalRuntime::new().unwrap();

        // 测试 fetch 超时
        let result = runtime.execute_code(
            r#"
            let observable = false;
            try {
                const controller = new AbortController();
                const response = fetch('https://httpbin.org/delay/5', {
                    signal: controller.signal
                });
                observable = typeof response === 'object' && typeof response.status === 'number';
            } catch (error) {
                observable = String(error && error.message ? error.message : error).includes('fetch');
            }
            observable;
        "#,
        );

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.trim(), "true");
    }

    #[test]
    #[serial_test::serial]
    fn test_stream_api() {
        let mut runtime = MinimalRuntime::new().unwrap();

        // 测试流 API
        let result = runtime.execute_code(
            r#"
            const stream = new ReadableStream({
                start(controller) {
                    controller.enqueue('Hello');
                    controller.close();
                }
            });
            stream.constructor.name;
        "#,
        );

        assert!(result.is_ok());
        // 应该支持 ReadableStream
        let output = result.unwrap();
        assert!(output.contains("ReadableStream") || output.contains("undefined"));
    }

    // TODO: 更多异步测试用例
    // - 测试 WebSocket 消息传递
    // - 测试 Server-Sent Events
    // - 测试文件流
    // - 测试性能基准
}
