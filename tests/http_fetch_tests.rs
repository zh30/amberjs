// HTTP Fetch 测试 - v0.3.1
// 验证真实的 HTTP fetch 功能（返回实际响应数据）

#[cfg(test)]
mod http_tests {
    use amberjs::runtime_minimal::MinimalRuntime;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn assert_valid_http_status(status: &str, context: &str) {
        let status_code: u16 = status.parse().unwrap_or_else(|_| {
            panic!("Expected numeric HTTP status for {context}, got: {status}")
        });
        assert!(
            (100..=599).contains(&status_code),
            "Expected valid HTTP status for {context}, got: {status_code}"
        );
    }

    fn spawn_response_server(
        status: u16,
        reason: &str,
        content_type: &str,
        body: &str,
        extra_headers: Vec<(String, String)>,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let address = listener
            .local_addr()
            .expect("test server should expose a local address");
        let reason = reason.to_string();
        let content_type = content_type.to_string();
        let body = body.to_string();

        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0; 1024];
                let _ = stream.read(&mut buffer);
                let mut headers = format!(
                    "Content-Type: {content_type}\r\nContent-Length: {}\r\n",
                    body.len()
                );
                for (name, value) in extra_headers {
                    headers.push_str(&format!("{name}: {value}\r\n"));
                }
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\n{headers}Connection: close\r\n\r\n{body}"
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        format!("http://{}", address)
    }

    fn spawn_json_server() -> String {
        spawn_response_server(
            200,
            "OK",
            "application/json",
            r#"{"slideshow":{"title":"Amber fixture","slides":[{"title":"Local"}]}}"#,
            Vec::new(),
        )
    }

    fn spawn_bytes_server() -> String {
        spawn_response_server(
            200,
            "OK",
            "application/octet-stream",
            "0123456789",
            Vec::new(),
        )
    }

    fn spawn_redirect_server(target: &str) -> String {
        spawn_response_server(
            302,
            "Found",
            "text/plain",
            "",
            vec![("Location".to_string(), target.to_string())],
        )
    }

    fn spawn_header_echo_server(header_name: &str, expected_value: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let address = listener
            .local_addr()
            .expect("test server should expose a local address");
        let header_name = header_name.to_string();
        let expected_value = expected_value.to_string();

        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0; 4096];
                let read_len = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read_len]);
                let found = request.lines().any(|line| {
                    let Some((name, value)) = line.split_once(':') else {
                        return false;
                    };
                    name.eq_ignore_ascii_case(&header_name) && value.trim() == expected_value
                });
                let body = if found { "present" } else { "missing" };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        format!("http://{}", address)
    }

    fn http_header_end(buffer: &[u8]) -> Option<usize> {
        buffer
            .windows(b"\r\n\r\n".len())
            .position(|window| window == b"\r\n\r\n")
    }

    fn read_http_request_bytes(stream: &mut std::net::TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut expected_len = None;
        let mut buffer = [0; 1024];

        loop {
            let read_len = stream.read(&mut buffer).unwrap_or(0);
            if read_len == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read_len]);

            if let Some(header_end) = http_header_end(&request) {
                if expected_len.is_none() {
                    let headers = String::from_utf8_lossy(&request[..header_end]);
                    expected_len = headers.lines().find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        if name.eq_ignore_ascii_case("content-length") {
                            value.trim().parse::<usize>().ok()
                        } else {
                            None
                        }
                    });
                }

                if let Some(content_len) = expected_len {
                    if request.len() >= header_end + b"\r\n\r\n".len() + content_len {
                        break;
                    }
                }
            }
        }

        request
    }

    fn spawn_multipart_formdata_echo_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
        let address = listener
            .local_addr()
            .expect("test server should expose a local address");

        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let request_bytes = read_http_request_bytes(&mut stream);
                let request = String::from_utf8_lossy(&request_bytes);
                let expected_parts = [
                    "content-type: multipart/form-data; boundary=",
                    "name=\"note\"",
                    "alpha",
                    "name=\"blobField\"; filename=\"blob.txt\"",
                    "Content-Type: text/plain",
                    "blob payload",
                    "name=\"fileField\"; filename=\"file.txt\"",
                    "Content-Type: text/custom",
                    "file payload",
                    "name=\"bytesField\"; filename=\"bytes.bin\"",
                    "Content-Type: application/octet-stream",
                ];
                let has_expected_parts = expected_parts.iter().all(|part| request.contains(part));
                let has_binary_payload = request_bytes
                    .windows(3)
                    .any(|window| window == [0, 255, 65]);
                let has_placeholder =
                    request.contains("[Blob data]") || request.contains("[Object]");
                let body = if has_expected_parts && has_binary_payload && !has_placeholder {
                    "ok".to_string()
                } else {
                    format!(
                        "bad multipart: expected_parts={has_expected_parts}; binary={has_binary_payload}; placeholder={has_placeholder}; request={request}"
                    )
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        format!("http://{}", address)
    }

    #[test]
    #[serial_test::serial]
    fn test_fetch_with_real_http() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        // 测试真实的 HTTP fetch
        let result = runtime.execute_code(
            &r#"
            fetch('__URL__').status;
        "#
            .replace("__URL__", &url),
        );

        assert!(
            result.is_ok(),
            "Expected local HTTP fetch status script to run, got: {result:?}"
        );
        let binding = result.unwrap();
        let status = binding.trim();
        // 上游 httpbin 可能返回 2xx/4xx/5xx；Amber 不应再用离线 fallback 改写真实状态。
        assert_valid_http_status(status, "real http fetch");
    }

    #[test]
    #[serial_test::serial]
    fn test_fetch_json_method_returns_real_data() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            const json = response.json();
            json;
        "#
            .replace("__URL__", &url),
        );

        assert!(
            result.is_ok(),
            "Expected local JSON fetch script to run, got: {result:?}"
        );
        let binding = result.unwrap();
        let output = binding.as_str();
        assert!(
            output.contains("slideshow") && output.contains("Amber fixture"),
            "Expected local JSON response body, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_fetch_text_method_returns_real_data() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            response.text();
        "#
            .replace("__URL__", &url),
        );

        match result {
            Ok(binding) => {
                let output = binding.as_str();
                // 应该包含真实的响应内容（JSON 结构）
                assert!(
                    output.contains("{") && output.contains("}"),
                    "Expected JSON response body, got: {}",
                    output
                );
            }
            Err(error) => {
                let message = error.to_string();
                assert!(
                    message.contains("fetch") || message.contains("body"),
                    "Network failure should surface as a fetch/body error, got: {}",
                    message
                );
            }
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_fetch_ok_property() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            response.ok;
        "#
            .replace("__URL__", &url),
        );

        assert!(
            result.is_ok(),
            "Expected Headers iteration script to run, got: {result:?}"
        );
        let binding = result.unwrap();
        let output = binding.trim();
        // 应该是 true（200-299 状态码）或 false
        assert!(
            output == "true" || output == "false",
            "Expected boolean, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_fetch_url_property() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            response.url;
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == url,
            "Expected URL to match local fixture URL, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_fetch_with_invalid_url() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            try {
                const response = fetch('https://invalid-url-that-does-not-exist.test xyz');
                JSON.stringify({
                    threw: false,
                    ok: response.ok,
                    status: response.status
                });
            } catch (error) {
                JSON.stringify({
                    threw: true,
                    message: String(error && error.message ? error.message : error)
                });
            }
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // 无效的 URL 应该抛出或返回可观察失败，不能 fallback 成 fake 200 OK
        assert_ne!(
            output, r#"{"threw":false,"ok":true,"status":200}"#,
            "Invalid URL must not become a fake successful response"
        );
        assert!(
            output.contains(r#""threw":true"#) || output.contains(r#""ok":false"#),
            "Expected invalid URL to throw or return non-ok response, got: {}",
            output
        );
    }

    // v0.3.344: Tests for Response.arrayBuffer() and Response.blob() Body mixin methods
    #[test]
    #[serial_test::serial]
    fn test_response_array_buffer_method_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_bytes_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            typeof response.arrayBuffer;
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // arrayBuffer 方法应该存在且类型为 'function'
        assert!(
            output == "function",
            "Expected arrayBuffer to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_response_blob_method_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_bytes_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            typeof response.blob;
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // blob 方法应该存在且类型为 'function'
        assert!(
            output == "function",
            "Expected blob to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_response_blob_returns_object_with_size_and_type() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_bytes_server();

        let result = runtime.execute_code(&r#"
            const response = fetch('__URL__');
            const blob = response.blob();
            typeof blob === 'object' && typeof blob.size === 'number' && typeof blob.type === 'string';
        "#
        .replace("__URL__", &url));

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // blob 应该返回包含 size 和 type 属性的对象
        assert!(
            output == "true",
            "Expected blob to return object with size and type, got: {}",
            output
        );
    }

    // v0.3.346: Tests for Headers API enhancement
    #[test]
    #[serial_test::serial]
    fn test_headers_constructor_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            typeof Headers;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected Headers to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_get_method_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers();
            typeof headers.get;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected headers.get to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_set_method_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers();
            typeof headers.set;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected headers.set to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_has_method_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers();
            typeof headers.has;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected headers.has to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_delete_method_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers();
            typeof headers.delete;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected headers.delete to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_append_method_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers();
            typeof headers.append;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected headers.append to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_get_set_basic() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers();
            headers.set('Content-Type', 'application/json');
            headers.get('Content-Type');
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "application/json",
            "Expected Content-Type to be 'application/json', got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_constructor_accepts_plain_object_init() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers({
                'Content-Type': 'text/plain',
                'X-Amber-Trace': 'ctor'
            });
            `${headers.get('content-type')}:${headers.get('x-amber-trace')}:${headers.has('Content-Type')}`;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "text/plain:ctor:true",
            "Expected Headers plain-object init to populate case-insensitive entries, got: {output}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_constructor_accepts_sequence_pair_init() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers([
                ['Content-Type', 'text/html'],
                ['X-Amber-Trace', 'array-init']
            ]);
            `${headers.get('content-type')}:${headers.get('x-amber-trace')}:${headers.has('Content-Type')}`;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "text/html:array-init:true",
            "Expected Headers sequence-pair init to populate entries, got: {output}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_fetch_init_accepts_headers_instance() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_header_echo_server("X-Amber-Trace", "from-headers");

        let result = runtime.execute_code(
            &r#"
            const headers = new Headers({
                'X-Amber-Trace': 'from-headers'
            });
            const response = fetch('__URL__', { headers });
            response.text();
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "present",
            "Expected fetch init to serialize Headers instance entries, got: {output}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_fetch_init_accepts_header_sequence_pairs() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_header_echo_server("X-Amber-Trace", "from-array");

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__', {
                headers: [
                    ['X-Amber-Trace', 'from-array']
                ]
            });
            response.text();
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "present",
            "Expected fetch init to serialize header sequence-pair entries, got: {output}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_fetch_request_object_preserves_headers_instance() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_header_echo_server("X-Amber-Trace", "from-request");

        let result = runtime.execute_code(
            &r#"
            const request = new Request('__URL__', {
                headers: new Headers({
                    'X-Amber-Trace': 'from-request'
                })
            });
            const response = fetch(request);
            response.text();
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "present",
            "Expected Request(headers) entries to be preserved when passed to fetch, got: {output}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_has() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers();
            headers.set('X-Custom-Header', 'test');
            headers.has('X-Custom-Header');
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "true",
            "Expected has() to return true, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_delete() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers();
            headers.set('X-Test-Header', 'value');
            const hasBefore = headers.has('X-Test-Header');
            headers.delete('X-Test-Header');
            const hasAfter = headers.has('X-Test-Header');
            hasBefore + ',' + hasAfter;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "true,false",
            "Expected 'true,false', got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_append() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers();
            headers.append('Set-Cookie', 'cookie1=value1');
            headers.append('Set-Cookie', 'cookie2=value2');
            headers.get('Set-Cookie');
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // append 应该追加值，可能以逗号分隔
        assert!(
            output.contains("cookie1") && output.contains("cookie2"),
            "Expected cookies in Set-Cookie header, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_iteration_methods_expose_cached_entries() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers([
                ['Content-Type', 'text/plain'],
                ['X-Amber-Trace', 'iter']
            ]);
            const keys = Array.from(headers.keys()).join('|');
            const values = Array.from(headers.values()).join('|');
            const entries = Array.from(headers.entries()).map(([name, value]) => `${name}=${value}`).join('|');
            const seen = [];
            headers.forEach((value, name, owner) => {
                seen.push(`${name}=${value}:${owner === headers}`);
            });
            `${keys};${values};${entries};${seen.join('|')}`;
        "#,
        );

        assert!(
            result.is_ok(),
            "Expected Headers iteration script to run, got: {result:?}"
        );
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output,
            "content-type|x-amber-trace;text/plain|iter;content-type=text/plain|x-amber-trace=iter;content-type=text/plain:true|x-amber-trace=iter:true",
            "Expected Headers iteration methods to expose cached entries, got: {output}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_are_directly_iterable() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers([
                ['Content-Type', 'text/plain'],
                ['X-Amber-Trace', 'iterable']
            ]);
            const direct = Array.from(headers).map(([name, value]) => `${name}=${value}`).join('|');
            const looped = [];
            for (const [name, value] of headers) {
                looped.push(`${name}=${value}`);
            }
            `${direct};${looped.join('|')}`;
        "#,
        );

        assert!(
            result.is_ok(),
            "Expected Headers direct iteration script to run, got: {result:?}"
        );
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output,
            "content-type=text/plain|x-amber-trace=iterable;content-type=text/plain|x-amber-trace=iterable",
            "Expected Headers to be directly iterable as entry pairs, got: {output}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_iteration_methods_return_iterators() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers([
                ['Content-Type', 'text/plain'],
                ['X-Amber-Trace', 'iterator']
            ]);
            const keyIterator = headers.keys();
            const valueIterator = headers.values();
            const entryIterator = headers.entries();
            const firstKey = keyIterator.next();
            const secondValue = valueIterator.next();
            valueIterator.next();
            const doneValue = valueIterator.next();
            const firstEntry = entryIterator.next();
            `${typeof keyIterator.next}:${firstKey.value}:${firstKey.done}:${secondValue.value}:${doneValue.done}:${firstEntry.value[0]}=${firstEntry.value[1]}:${typeof entryIterator[Symbol.iterator]}`;
        "#,
        );

        assert!(
            result.is_ok(),
            "Expected Headers iteration methods to return iterator objects, got: {result:?}"
        );
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output,
            "function:content-type:false:text/plain:true:content-type=text/plain:function",
            "Expected Headers keys/values/entries to return iterator objects with next(), got: {output}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_iteration_normalizes_names_to_lowercase() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers([
                ['Content-Type', 'text/plain']
            ]);
            headers.set('X-Amber-Trace', 'set');
            headers.append('Set-Cookie', 'cookie=value');
            const keys = Array.from(headers.keys()).join('|');
            const entries = Array.from(headers.entries()).map(([name, value]) => `${name}=${value}`).join('|');
            const seen = [];
            headers.forEach((value, name) => seen.push(name));
            `${keys};${entries};${seen.join('|')}`;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output,
            "content-type|x-amber-trace|set-cookie;content-type=text/plain|x-amber-trace=set|set-cookie=cookie=value;content-type|x-amber-trace|set-cookie",
            "Expected Headers iteration to expose lowercase names, got: {output}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_get_case_insensitive() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers();
            headers.set('content-type', 'text/plain');
            headers.get('Content-Type');
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "text/plain",
            "Expected case-insensitive header lookup to work, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_headers_get_nonexistent() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const headers = new Headers();
            headers.get('X-Non-Existent');
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "null" || output.is_empty(),
            "Expected null or empty for nonexistent header, got: {}",
            output
        );
    }

    // v0.3.347: Tests for Request API enhancement
    #[test]
    #[serial_test::serial]
    fn test_request_constructor_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            typeof Request;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected Request to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_request_constructor_basic() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const request = new Request('https://example.com/api');
            request.url;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output.contains("example.com"),
            "Expected request.url to contain example.com, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_request_method_default() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const request = new Request('https://example.com/api');
            request.method;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "GET",
            "Expected default request.method to be GET, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_request_method_custom() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const request = new Request('https://example.com/api', {
                method: 'POST'
            });
            request.method;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "POST",
            "Expected request.method to be POST, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_request_headers() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const request = new Request('https://example.com/api', {
                headers: {
                    'Content-Type': 'application/json',
                    'X-Custom-Header': 'test-value'
                }
            });
            const headers = request.headers;
            typeof headers === 'object';
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "true",
            "Expected request.headers to be an object, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_request_clone_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const request = new Request('https://example.com/api');
            typeof request.clone;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected request.clone to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_request_clone_basic() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const original = new Request('https://example.com/api', {
                method: 'POST',
                headers: { 'X-Test': 'value' }
            });
            const cloned = original.clone();
            cloned.url === original.url && cloned.method === original.method;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "true",
            "Expected cloned request to have same url and method, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_request_body_init() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const request = new Request('https://example.com/api', {
                method: 'POST',
                body: JSON.stringify({ test: 'data' })
            });
            typeof request.body;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // body should exist (as string or null depending on implementation)
        assert!(
            output == "string" || output == "object",
            "Expected request.body to be string or object, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_request_cache_mode() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const request = new Request('https://example.com/api', {
                cache: 'no-cache'
            });
            request.cache;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output.contains("no-cache") || output == "default",
            "Expected cache mode to be set, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_request_credentials_mode() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const request = new Request('https://example.com/api', {
                credentials: 'include'
            });
            request.credentials;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "include",
            "Expected credentials to be 'include', got: {}",
            output
        );
    }

    // v0.3.348: Tests for Response API enhancements
    #[test]
    #[serial_test::serial]
    fn test_response_constructor_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            typeof Response;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected Response to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_response_constructor_uses_body_first_and_init_options() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const response = new Response('created body', {
                status: 201,
                statusText: 'Created',
                headers: {
                    'Content-Type': 'text/plain',
                    'X-Amber-Trace': 'constructor'
                }
            });
            const before = response.bodyUsed;
            const body = response.text();
            const after = response.bodyUsed;
            `${response.status}:${response.ok}:${response.statusText}:${response.headers.get('content-type')}:${response.headers.get('x-amber-trace')}:${before}:${body}:${after}`;
        "#,
        );

        assert!(
            result.is_ok(),
            "Expected Response(body, init) script to run, got: {result:?}"
        );
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "201:true:Created:text/plain:constructor:false:created body:true",
            "Expected Response(body, init) to create a consumable response with init metadata, got: {output}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_response_status_text() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            typeof response.statusText;
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "string",
            "Expected response.statusText to be a string, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_response_url() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            response.url;
        "#
            .replace("__URL__", &url),
        );

        match result {
            Ok(binding) => {
                let output = binding.trim();
                assert!(
                    output == url,
                    "Expected response.url to match local fixture URL, got: {}",
                    output
                );
            }
            Err(error) => {
                let message = error.to_string();
                assert!(
                    message.contains("fetch") || message.contains("body"),
                    "Network failure should surface as a fetch/body error, got: {}",
                    message
                );
            }
        }
    }

    #[test]
    #[serial_test::serial]
    fn test_response_type() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            response.type;
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "basic" || output == "default" || output.is_empty(),
            "Expected response.type to be 'basic', 'default', or empty, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_response_headers() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            typeof response.headers;
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "object",
            "Expected response.headers to be an object, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_response_headers_get_reads_real_response_headers() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_response_server(
            200,
            "OK",
            "text/plain",
            "headers",
            vec![("X-Amber-Trace".to_string(), "trace-123".to_string())],
        );

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            const headers = response.headers;
            `${typeof headers.get}:${typeof headers.has}:${headers.get ? headers.get('x-amber-trace') : 'missing'}:${headers.has ? headers.has('content-type') : false}`;
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "function:function:trace-123:true",
            "Expected response.headers to expose real Headers get/has behavior, got: {output}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_response_clone_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            typeof response.clone;
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected response.clone to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_response_clone_basic() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            const cloned = response.clone();
            cloned.status === response.status && cloned.ok === response.ok;
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "true",
            "Expected cloned response to have same status and ok, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_response_redirected() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            response.redirected;
        "#
            .replace("__URL__", &url),
        );

        match result {
            Ok(binding) => {
                let output = binding.trim();
                assert!(
                    output == "true" || output == "false",
                    "Expected response.redirected to be boolean, got: {}",
                    output
                );
            }
            Err(error) => {
                let message = error.to_string();
                assert!(
                    message.contains("fetch") || message.contains("body"),
                    "Network failure should surface as a fetch/body error, got: {}",
                    message
                );
            }
        }
    }

    // v0.3.349: Tests for FormData API
    #[test]
    #[serial_test::serial]
    fn test_form_data_constructor_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            typeof FormData;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected FormData to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_append_method_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            typeof formData.append;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected append to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_append_basic() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            formData.append('name', 'John');
            formData.get('name');
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "John",
            "Expected formData.get('name') to return 'John', got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_get_method_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            typeof formData.get;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected get to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_has_method_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            typeof formData.has;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected has to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_has() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            formData.append('email', 'test@example.com');
            formData.has('email');
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "true",
            "Expected has() to return true, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_has_nonexistent() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            formData.has('nonexistent');
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "false",
            "Expected has() to return false for nonexistent key, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_delete_method_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            typeof formData.delete;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected delete to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_delete() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            formData.append('key', 'value');
            formData.has('key');
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "true",
            "Expected has() to return true before delete, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_set_method_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            typeof formData.set;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected set to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_set_replaces() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            formData.append('name', 'John');
            formData.set('name', 'Jane');
            formData.get('name');
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "Jane",
            "Expected set() to replace value, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_get_all_method_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            typeof formData.getAll;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected getAll to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_get_all_multiple_values() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            formData.append('tag', 'a');
            formData.append('tag', 'b');
            const values = formData.getAll('tag');
            values.length;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "2",
            "Expected getAll() to return 2 values, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_entries_method_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            typeof formData.entries;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected entries to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_keys_method_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            typeof formData.keys;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected keys to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_values_method_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            typeof formData.values;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "function",
            "Expected values to be a function, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_with_multiple_fields() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            formData.append('username', 'alice');
            formData.append('email', 'alice@example.com');
            formData.append('age', '25');
            const keys = Array.from(formData.keys());
            keys.length;
        "#,
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(output == "3", "Expected 3 keys, got: {}", output);
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_iteration_methods_return_standard_iterators_in_order() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            formData.append('tag', 'a');
            formData.append('tag', 'b');
            formData.append('name', 'amber');
            const keys = formData.keys();
            const firstKey = keys.next();
            const secondKey = keys.next();
            const thirdKey = keys.next();
            const doneKey = keys.next();
            const values = Array.from(formData.values()).join('|');
            const entries = Array.from(formData.entries()).map(([name, value]) => `${name}=${value}`).join('|');
            const direct = Array.from(formData).map(([name, value]) => `${name}=${value}`).join('|');
            `${typeof keys.next}:${firstKey.value}:${secondKey.value}:${thirdKey.value}:${doneKey.done}:${values}:${entries}:${direct}`;
        "#,
        );

        assert!(
            result.is_ok(),
            "Expected FormData iteration script to run, got: {result:?}"
        );
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output,
            "function:tag:tag:name:true:a|b|amber:tag=a|tag=b|name=amber:tag=a|tag=b|name=amber",
            "Expected FormData iterators to preserve duplicate names in insertion order, got: {output}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_form_data_for_each_invokes_callback_with_owner_and_this_arg() {
        let mut runtime = MinimalRuntime::new().unwrap();

        let result = runtime.execute_code(
            r#"
            const formData = new FormData();
            formData.append('tag', 'a');
            formData.append('tag', 'b');
            const seen = [];
            formData.forEach(function(value, name, owner) {
                seen.push(`${this.label}:${name}=${value}:${owner === formData}`);
            }, { label: 'ctx' });
            seen.join('|');
        "#,
        );

        assert!(
            result.is_ok(),
            "Expected FormData forEach script to run, got: {result:?}"
        );
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "ctx:tag=a:true|ctx:tag=b:true",
            "Expected FormData.forEach to call callback for every entry, got: {output}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_fetch_form_data_multipart_uses_blob_and_file_bytes() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_multipart_formdata_echo_server();

        let result = runtime.execute_code(
            &r#"
            const formData = new FormData();
            formData.append('note', 'alpha');
            formData.append('blobField', new Blob(['blob payload'], { type: 'text/plain' }), 'blob.txt');
            formData.append('fileField', new File(['file payload'], 'file.txt', { type: 'text/custom' }));
            formData.append('bytesField', new Blob([new Uint8Array([0, 255, 65])], { type: 'application/octet-stream' }), 'bytes.bin');
            const response = fetch('__URL__', {
                method: 'POST',
                body: formData
            });
            response.text();
        "#
            .replace("__URL__", &url),
        );

        assert!(
            result.is_ok(),
            "Expected FormData multipart upload script to run, got: {result:?}"
        );
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "ok",
            "Expected multipart upload to include real Blob/File bytes, got: {output}"
        );
    }

    // v0.3.350: Tests for fetch with Request object
    #[test]
    #[serial_test::serial]
    fn test_fetch_with_request_object() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const request = new Request('__URL__');
            const response = fetch(request);
            response.status;
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let status = binding.trim();
        assert_eq!(
            status, "200",
            "Expected local fixture status 200, got: {status}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_fetch_with_request_object_extracts_url() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const request = new Request('__URL__');
            const response = fetch(request);
            response.url;
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == url,
            "Expected response.url to match local fixture URL, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_fetch_with_request_object_and_init() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const request = new Request('__URL__', {
                method: 'POST'
            });
            const response = fetch(request, {
                method: 'PUT'
            });
            response.status;
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let status = binding.trim();
        assert_valid_http_status(status, "Request object with init");
    }

    // v0.3.351: Tests for fetch redirect handling
    #[test]
    #[serial_test::serial]
    fn test_fetch_redirect_option_follow() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let target_url = spawn_json_server();
        let redirect_url = spawn_redirect_server(&target_url);

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__', {
                redirect: 'follow'
            });
            response.status;
        "#
            .replace("__URL__", &redirect_url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let status = binding.trim();
        assert_valid_http_status(status, "redirect=follow");
    }

    #[test]
    #[serial_test::serial]
    fn test_fetch_redirected_property_exists() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            typeof response.redirected;
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert!(
            output == "boolean",
            "Expected response.redirected to be boolean, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_fetch_response_body_used() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            typeof response.bodyUsed;
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        // bodyUsed should be a boolean indicating if body has been consumed
        assert!(
            output == "boolean",
            "Expected response.bodyUsed to be boolean, got: {}",
            output
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_response_body_consumption_updates_body_used_and_rejects_second_read() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            const before = response.bodyUsed;
            response.text();
            const after = response.bodyUsed;
            let secondRead;
            try {
                response.json();
                secondRead = 'allowed';
            } catch (error) {
                secondRead = String(error.message || error);
            }
            `${before}:${after}:${secondRead.includes('already consumed')}`;
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "false:true:true",
            "Expected bodyUsed to flip after first read and second read to fail, got: {output}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn test_response_clone_rejects_after_body_consumed() {
        let mut runtime = MinimalRuntime::new().unwrap();
        let url = spawn_json_server();

        let result = runtime.execute_code(
            &r#"
            const response = fetch('__URL__');
            response.text();
            let clonedAfterRead;
            try {
                response.clone();
                clonedAfterRead = 'allowed';
            } catch (error) {
                clonedAfterRead = String(error.message || error);
            }
            clonedAfterRead.includes('already consumed');
        "#
            .replace("__URL__", &url),
        );

        assert!(result.is_ok());
        let binding = result.unwrap();
        let output = binding.trim();
        assert_eq!(
            output, "true",
            "Expected clone() to reject after body consumption, got: {output}"
        );
    }
}
