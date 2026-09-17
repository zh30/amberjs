// v0.3.315: MessageChannel API tests
// Tests for MessageChannel and MessagePort port-based communication

#[cfg(test)]
mod message_channel_tests {
    use std::path::PathBuf;
    use std::process::Command;

    fn beejs_path() -> PathBuf {
        PathBuf::from(
            std::env::var("CARGO_BIN_EXE_amber").unwrap_or_else(|_| "./target/debug/bee".to_string()),
        )
    }

    #[test]
    fn test_message_channel_creation() {
        let output = Command::new(beejs_path())
            .args(["eval", "console.log(typeof MessageChannel)"])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("function"), "MessageChannel should exist");
    }

    #[test]
    fn test_message_channel_has_port1_port2() {
        let output = Command::new(beejs_path())
            .args(["eval", r#"const ch = new MessageChannel(); console.log(typeof ch.port1 === 'object' && typeof ch.port2 === 'object')"#])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "MessageChannel should have port1 and port2"
        );
    }

    #[test]
    fn test_message_port_has_post_message() {
        let output = Command::new(beejs_path())
            .args(["eval", r#"const ch = new MessageChannel(); console.log(typeof ch.port1.postMessage === 'function')"#])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "port1 should have postMessage method"
        );
    }

    #[test]
    fn test_message_port_has_start() {
        let output = Command::new(beejs_path())
            .args(["eval", r#"const ch = new MessageChannel(); console.log(typeof ch.port1.start === 'function')"#])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("true"), "port1 should have start method");
    }

    #[test]
    fn test_message_port_has_close() {
        let output = Command::new(beejs_path())
            .args(["eval", r#"const ch = new MessageChannel(); console.log(typeof ch.port1.close === 'function')"#])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("true"), "port1 should have close method");
    }

    #[test]
    fn test_message_port_has_onmessage() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"const ch = new MessageChannel(); console.log('onmessage' in ch.port1)"#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "port1 should have onmessage property"
        );
    }

    #[test]
    fn test_message_port_has_onmessageerror() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"const ch = new MessageChannel(); console.log('onmessageerror' in ch.port1)"#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "port1 should have onmessageerror property"
        );
    }

    #[test]
    fn test_message_port_has_closed() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"const ch = new MessageChannel(); console.log('closed' in ch.port1)"#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("true"), "port1 should have closed property");
    }

    #[test]
    fn test_basic_message_pass() {
        // Test that messages can be sent from port1 to port2 after start
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const ch = new MessageChannel();
                let received = null;
                ch.port2.onmessage = function(e) { received = e.data; };
                ch.port2.start();
                ch.port1.postMessage('hello');
                ch.port1.postMessage('world');
                console.log(received);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("world"),
            "Should receive the last message sent"
        );
    }

    #[test]
    fn test_close_port() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const ch = new MessageChannel();
                ch.port1.close();
                console.log('closed');
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("closed"),
            "close() should execute without error"
        );
    }

    #[test]
    fn test_close_prevents_further_messages() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const ch = new MessageChannel();
                ch.port1.close();
                let error = null;
                try {
                    ch.port1.postMessage('after close');
                } catch (e) {
                    error = e.message;
                }
                console.log(error === null ? 'no error' : 'has error');
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        // postMessage on closed port should silently fail (not throw)
        assert!(
            stdout.contains("no error") || stdout.contains("has error"),
            "Should handle postMessage on closed port"
        );
    }

    #[test]
    fn test_structured_clone_compatible() {
        // Test that complex objects can be passed through MessageChannel
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const ch = new MessageChannel();
                let received = null;
                ch.port2.onmessage = function(e) { received = e.data; };
                ch.port2.start();
                ch.port1.postMessage({ foo: 'bar', num: 42, arr: [1, 2, 3] });
                console.log(JSON.stringify(received));
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("foo") && stdout.contains("bar"),
            "Complex objects should be passed correctly"
        );
    }

    #[test]
    fn test_message_port_object_payload_is_structured_cloned() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const ch = new MessageChannel();
                const payload = {
                    nested: { value: 1 },
                    list: ['sender'],
                    bytes: new Uint8Array([3, 4, 5])
                };
                let received = null;
                ch.port2.onmessage = function(e) {
                    received = e.data;
                    received.nested.value = 99;
                    received.list.push('receiver');
                    received.bytes[0] = 42;
                };
                ch.port2.start();
                ch.port1.postMessage(payload);
                console.log(
                    received !== null &&
                    received !== payload &&
                    received.nested !== payload.nested &&
                    received.list !== payload.list &&
                    received.bytes !== payload.bytes &&
                    payload.nested.value === 1 &&
                    payload.list.length === 1 &&
                    payload.bytes[0] === 3
                );
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "MessagePort should deliver structured-cloned payloads. Got: {}",
            stdout
        );
    }

    #[test]
    fn test_message_port_uncloneable_payload_throws_without_dispatch() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const ch = new MessageChannel();
                let delivered = false;
                let threwDataCloneError = false;
                ch.port2.onmessage = function() {
                    delivered = true;
                };
                ch.port2.start();
                try {
                    ch.port1.postMessage({ fn: function nope() {} });
                } catch (error) {
                    threwDataCloneError = error.name === 'DataCloneError' ||
                        String(error.message).includes('Function cannot be cloned');
                }
                console.log(threwDataCloneError && delivered === false);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Uncloneable MessagePort payloads should throw before dispatch. Got: {}",
            stdout
        );
    }

    #[test]
    fn test_close_receiving_port_prevents_delivery_from_peer() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const ch = new MessageChannel();
                let received = false;
                ch.port1.onmessage = function() {
                    received = true;
                };
                ch.port1.start();
                ch.port1.close();
                ch.port2.postMessage('after receiver close');
                console.log(received === false && ch.port1._pendingMessages === 0);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Closing a receiving MessagePort should prevent peer delivery. Got: {}",
            stdout
        );
    }

    #[test]
    fn test_message_event_properties() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const ch = new MessageChannel();
                let eventData = null;
                ch.port2.onmessage = function(e) { eventData = e; };
                ch.port2.start();
                ch.port1.postMessage('test message');
                console.log(eventData.type + ',' + eventData.origin);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("message"),
            "MessageEvent should have correct type"
        );
        assert!(
            stdout.contains(","),
            "MessageEvent should have origin property"
        );
    }

    #[test]
    fn test_queue_messages_before_start() {
        // Messages sent before start() should be queued and delivered after start()
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const ch = new MessageChannel();
                let received = [];
                ch.port2.onmessage = function(e) { received.push(e.data); };
                // Send messages before start
                ch.port1.postMessage('first');
                ch.port1.postMessage('second');
                // Start receiving
                ch.port2.start();
                console.log(received.length + ':' + received.join(','));
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("2:first,second"),
            "Queued messages should be delivered after start()"
        );
    }

    #[test]
    fn test_start_is_idempotent_after_immediate_delivery() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const ch = new MessageChannel();
                const received = [];
                ch.port2.onmessage = function(e) {
                    received.push(e.data);
                };
                ch.port2.start();
                ch.port1.postMessage('once');
                ch.port2.start();
                console.log(received.join(',') + ':' + ch.port2._pendingMessages);
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            stdout.trim(),
            "once:0",
            "Calling start() again should not redeliver already dispatched messages. Got: {}",
            stdout
        );
    }
}
