// BroadcastChannel API Tests for Beejs
// v0.3.312: Tests for BroadcastChannel cross-tab communication API
// Enables real-time communication between browsing contexts (tabs, windows, frames)

#[cfg(test)]
mod broadcast_channel_tests {
    use std::path::PathBuf;
    use std::process::Command;

    fn beejs_path() -> PathBuf {
        PathBuf::from(
            std::env::var("CARGO_BIN_EXE_amber").unwrap_or_else(|_| "./target/debug/bee".to_string()),
        )
    }

    /// Test 1: Basic BroadcastChannel creation
    #[test]
    fn test_broadcast_channel_creation() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const channel = new BroadcastChannel('test-channel');
                console.log('channel name:', channel.name);
                channel.name === 'test-channel'
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("channel name: test-channel"),
            "Expected channel name to be 'test-channel'. Got: {}",
            stdout
        );
        assert!(
            stdout.contains("true"),
            "Expected test to pass. Got: {}",
            stdout
        );
    }

    /// Test 2: BroadcastChannel with different name
    #[test]
    fn test_broadcast_channel_different_names() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const channel1 = new BroadcastChannel('channel-a');
                const channel2 = new BroadcastChannel('channel-b');
                channel1.name === 'channel-a' && channel2.name === 'channel-b'
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected channels with different names. Got: {}",
            stdout
        );
    }

    /// Test 3: postMessage delivers to same-name peers, not the sender
    #[test]
    fn test_post_message() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const sender = new BroadcastChannel('test');
                const peer = new BroadcastChannel('test');
                let senderReceived = null;
                let peerReceived = null;
                sender.onmessage = (event) => {
                    senderReceived = event.data;
                };
                peer.onmessage = (event) => {
                    peerReceived = event.data;
                };
                sender.postMessage('hello');
                senderReceived === null && peerReceived === 'hello'
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected same-name peer delivery without sender self-delivery. Got: {}",
            stdout
        );
    }

    /// Test 4: postMessage with object data
    #[test]
    fn test_post_message_object() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const sender = new BroadcastChannel('test');
                const peer = new BroadcastChannel('test');
                let received = null;
                peer.onmessage = (event) => {
                    received = event.data;
                };
                sender.postMessage({ text: 'hello', count: 42 });
                received !== null && received.text === 'hello' && received.count === 42
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected object message to be received. Got: {}",
            stdout
        );
    }

    /// Test 5: postMessage with array data
    #[test]
    fn test_post_message_array() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const sender = new BroadcastChannel('test');
                const peer = new BroadcastChannel('test');
                let received = null;
                peer.onmessage = (event) => {
                    received = event.data;
                };
                sender.postMessage([1, 2, 3, 4, 5]);
                received !== null && Array.isArray(received) && received.length === 5
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected array message to be received. Got: {}",
            stdout
        );
    }

    /// Test 6: addEventListener for message
    #[test]
    fn test_add_event_listener() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const sender = new BroadcastChannel('test');
                const peer = new BroadcastChannel('test');
                let received = null;
                peer.addEventListener('message', (event) => {
                    received = event.data;
                });
                sender.postMessage('via listener');
                received === 'via listener'
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected message via addEventListener. Got: {}",
            stdout
        );
    }

    /// Test 7: removeEventListener removes the matching listener
    #[test]
    fn test_remove_event_listener() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const sender = new BroadcastChannel('test');
                const peer = new BroadcastChannel('test');
                let callCount = 0;
                const listener = () => callCount++;
                peer.addEventListener('message', listener);
                sender.postMessage('first');
                peer.removeEventListener('message', listener);
                sender.postMessage('second');
                callCount === 1
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected removeEventListener to remove the matching listener. Got: {}",
            stdout
        );
    }

    /// Test 8: close method removes the channel from future delivery
    #[test]
    fn test_close() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const sender = new BroadcastChannel('test');
                const peer = new BroadcastChannel('test');
                let received = false;
                peer.onmessage = () => {
                    received = true;
                };
                peer.close();
                sender.postMessage('after close');
                received === false
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected close to prevent future delivery. Got: {}",
            stdout
        );
    }

    /// Test 9: message event has correct origin
    #[test]
    fn test_message_event_origin() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const sender = new BroadcastChannel('test');
                const peer = new BroadcastChannel('test');
                let origin = null;
                peer.onmessage = (event) => {
                    origin = event.origin;
                };
                sender.postMessage('test');
                // Origin should be empty string or 'null' for same-origin
                origin === '' || origin === 'null'
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected message event to have origin. Got: {}",
            stdout
        );
    }

    /// Test 10: message event data property is correct
    #[test]
    fn test_message_event_data() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const sender = new BroadcastChannel('test');
                const peer = new BroadcastChannel('test');
                let dataValue = null;
                peer.onmessage = (event) => {
                    dataValue = event.data;
                };
                sender.postMessage({ key: 'value', num: 123 });
                dataValue !== null && dataValue.key === 'value' && dataValue.num === 123
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected message event data to be correct. Got: {}",
            stdout
        );
    }

    /// Test 11: messageerror event
    #[test]
    fn test_message_error_event() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const channel = new BroadcastChannel('test');
                let errorReceived = false;
                channel.onmessageerror = (event) => {
                    errorReceived = true;
                };
                channel.addEventListener('messageerror', () => {
                    errorReceived = true;
                });
                channel.postMessage('normal message');
                errorReceived === false
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected normal postMessage not to dispatch messageerror. Got: {}",
            stdout
        );
    }

    /// Test 12: Empty name channel
    #[test]
    fn test_empty_name_channel() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const channel = new BroadcastChannel('');
                channel.name === ''
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected empty name to work. Got: {}",
            stdout
        );
    }

    /// Test 13: Unicode name channel
    #[test]
    fn test_unicode_name_channel() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const channel = new BroadcastChannel('测试频道-日本語');
                channel.name === '测试频道-日本語'
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected unicode name to work. Got: {}",
            stdout
        );
    }

    /// Test 14: Multiple channels with same name receive peer messages only
    #[test]
    fn test_multiple_channels_same_name() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const channel1 = new BroadcastChannel('shared');
                const channel2 = new BroadcastChannel('shared');

                let count1 = 0;
                let count2 = 0;
                channel1.onmessage = () => count1++;
                channel2.onmessage = () => count2++;

                channel1.postMessage('from1');
                channel2.postMessage('from2');

                // Each channel receives the other channel's message, not its own.
                count1 === 1 && count2 === 1
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected same-name channels to receive peer messages only. Got: {}",
            stdout
        );
    }

    /// Test 15: channels with different names are isolated for delivery
    #[test]
    fn test_different_name_channels_are_isolated() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const sender = new BroadcastChannel('alpha');
                const other = new BroadcastChannel('beta');
                let received = false;
                other.onmessage = () => {
                    received = true;
                };
                sender.postMessage('isolated');
                received === false
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected different-name channels to be isolated. Got: {}",
            stdout
        );
    }

    /// Test 16: object payloads are delivered as structured clones, not shared references
    #[test]
    fn test_post_message_object_payload_is_cloned() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const sender = new BroadcastChannel('clone-object');
                const peer = new BroadcastChannel('clone-object');
                const payload = {
                    nested: { value: 1 },
                    items: ['a', 'b'],
                    buffer: new Uint8Array([7, 8, 9])
                };
                let received = null;
                peer.onmessage = (event) => {
                    received = event.data;
                    received.nested.value = 99;
                    received.items.push('peer');
                    received.buffer[0] = 42;
                };
                sender.postMessage(payload);
                received !== null &&
                    received !== payload &&
                    received.nested !== payload.nested &&
                    received.items !== payload.items &&
                    received.buffer !== payload.buffer &&
                    received.nested.value === 99 &&
                    payload.nested.value === 1 &&
                    payload.items.length === 2 &&
                    payload.buffer[0] === 7
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected BroadcastChannel object payload to be structured-cloned. Got: {}",
            stdout
        );
    }

    /// Test 17: uncloneable payloads fail closed and do not dispatch message events
    #[test]
    fn test_post_message_uncloneable_payload_throws_without_dispatch() {
        let output = Command::new(beejs_path())
            .args([
                "eval",
                r#"
                const sender = new BroadcastChannel('clone-error');
                const peer = new BroadcastChannel('clone-error');
                let delivered = false;
                let threwDataCloneError = false;
                peer.onmessage = () => {
                    delivered = true;
                };
                try {
                    sender.postMessage({ fn: function nope() {} });
                } catch (error) {
                    threwDataCloneError = error.name === 'DataCloneError' ||
                        String(error.message).includes('Function cannot be cloned');
                }
                threwDataCloneError && delivered === false
            "#,
            ])
            .output()
            .expect("Failed to run bee");

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true"),
            "Expected uncloneable BroadcastChannel payload to throw before dispatch. Got: {}",
            stdout
        );
    }
}
