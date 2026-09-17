// Push API tests for Amber runtime
// v0.3.326: Tests for PushManager, PushSubscription, and PushEvent APIs

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn amber_path() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_BIN_EXE_amber").unwrap_or_else(|_| "./target/debug/amber".to_string()),
    )
}

#[cfg(test)]
mod push_manager_tests {
    use super::*;

    #[test]
    fn test_push_manager_exists() {
        // Test that PushManager exists
        let script = r#"
            if (typeof PushManager === 'function') {
                console.log('SUCCESS: PushManager exists');
            } else {
                throw new Error('PushManager not found');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "PushManager should exist: {}",
            stdout
        );
        assert!(stdout.contains("SUCCESS"), "Output: {}", stdout);
    }

    #[test]
    fn test_push_manager_subscribe_exists() {
        // Test that PushManager.subscribe exists (method on prototype or static)
        let script = r#"
            if (typeof PushManager !== 'undefined') {
                // Check if subscribe is accessible via prototype (standard API)
                const proto = PushManager.prototype;
                const hasSubscribe = proto && typeof proto.subscribe === 'function';
                if (hasSubscribe) {
                    console.log('SUCCESS: PushManager.subscribe is a function');
                } else {
                    console.log('ERROR: PushManager.subscribe not found');
                }
            } else {
                console.log('ERROR: PushManager not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "subscribe should be a function: {}",
            stdout
        );
    }

    #[test]
    fn test_push_manager_subscribe_fails_closed_until_push_service_exists() {
        let script = r#"
            if (typeof PushManager === 'undefined') {
                console.log('ERROR: PushManager not defined');
            } else {
                const manager = new PushManager();
                const result = manager.subscribe({ userVisibleOnly: true });
                if (!result || typeof result.then !== 'function') {
                    console.log('ERROR: subscribe did not return a Promise');
                } else {
                    result.then(
                        subscription => {
                            console.log('ERROR: subscribe resolved fake endpoint: ' + subscription.endpoint);
                        },
                        error => {
                            const message = String(error && error.message || error);
                            console.log(message === 'Push subscription is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                        }
                    );
                }
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushManager.subscribe should fail closed until push service support exists: {}",
            stdout
        );
    }

    #[test]
    fn test_push_manager_get_subscription_exists() {
        // Test that PushManager.getSubscription exists
        let script = r#"
            if (typeof PushManager !== 'undefined') {
                const proto = PushManager.prototype;
                const hasGetSub = proto && typeof proto.getSubscription === 'function';
                if (hasGetSub) {
                    console.log('SUCCESS: PushManager.getSubscription is a function');
                } else {
                    console.log('ERROR: PushManager.getSubscription not found');
                }
            } else {
                console.log('ERROR: PushManager not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "getSubscription should be a function: {}",
            stdout
        );
    }

    #[test]
    fn test_push_manager_permission_state_exists() {
        // Test that PushManager.permissionState exists
        let script = r#"
            if (typeof PushManager !== 'undefined') {
                const proto = PushManager.prototype;
                const hasPermState = proto && typeof proto.permissionState === 'function';
                if (hasPermState) {
                    console.log('SUCCESS: PushManager.permissionState is a function');
                } else {
                    console.log('ERROR: PushManager.permissionState not found');
                }
            } else {
                console.log('ERROR: PushManager not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "permissionState should be a function: {}",
            stdout
        );
    }
}

#[cfg(test)]
mod push_subscription_tests {
    use super::*;

    #[test]
    fn test_push_subscription_exists() {
        // Test that PushSubscription exists
        let script = r#"
            if (typeof PushSubscription === 'function') {
                console.log('SUCCESS: PushSubscription exists');
            } else {
                throw new Error('PushSubscription not found');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushSubscription should exist: {}",
            stdout
        );
    }

    #[test]
    fn test_push_subscription_constructor_fails_closed_instead_of_mock_endpoint() {
        let script = r#"
            if (typeof PushSubscription !== 'function') {
                console.log('ERROR: PushSubscription not defined');
            } else {
                try {
                    const subscription = new PushSubscription();
                    console.log('ERROR: constructed fake endpoint: ' + subscription.endpoint);
                } catch (error) {
                    const message = String(error && error.message || error);
                    console.log(message === 'PushSubscription construction is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                }
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushSubscription constructor should fail closed instead of returning a mock endpoint: {}",
            stdout
        );
    }

    #[test]
    fn test_push_subscription_constructor_rejects_fake_get_key_instance() {
        let script = r#"
            if (typeof PushSubscription === 'function') {
                try {
                    const sub = new PushSubscription();
                    console.log('ERROR: constructed fake getKey instance: ' + typeof sub.getKey);
                } catch (error) {
                    const message = String(error && error.message || error);
                    console.log(message === 'PushSubscription construction is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                }
            } else {
                console.log('ERROR: PushSubscription not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushSubscription constructor should not expose a fake getKey instance: {}",
            stdout
        );
    }

    #[test]
    fn test_push_subscription_constructor_rejects_fake_to_json_instance() {
        let script = r#"
            if (typeof PushSubscription === 'function') {
                try {
                    const sub = new PushSubscription();
                    console.log('ERROR: constructed fake toJSON instance: ' + typeof sub.toJSON);
                } catch (error) {
                    const message = String(error && error.message || error);
                    console.log(message === 'PushSubscription construction is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                }
            } else {
                console.log('ERROR: PushSubscription not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushSubscription constructor should not expose a fake toJSON instance: {}",
            stdout
        );
    }

    #[test]
    fn test_push_subscription_constructor_rejects_fake_unsubscribe_instance() {
        let script = r#"
            if (typeof PushSubscription === 'function') {
                try {
                    const sub = new PushSubscription();
                    console.log('ERROR: constructed fake unsubscribe instance: ' + typeof sub.unsubscribe);
                } catch (error) {
                    const message = String(error && error.message || error);
                    console.log(message === 'PushSubscription construction is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                }
            } else {
                console.log('ERROR: PushSubscription not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushSubscription constructor should not expose a fake unsubscribe instance: {}",
            stdout
        );
    }

    #[test]
    fn test_push_subscription_constructor_rejects_fake_endpoint() {
        let script = r#"
            if (typeof PushSubscription === 'function') {
                try {
                    const sub = new PushSubscription();
                    console.log('ERROR: constructed fake endpoint: ' + sub.endpoint);
                } catch (error) {
                    const message = String(error && error.message || error);
                    console.log(message === 'PushSubscription construction is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                }
            } else {
                console.log('ERROR: PushSubscription not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushSubscription constructor should not expose a fake endpoint: {}",
            stdout
        );
    }

    #[test]
    fn test_push_subscription_constructor_rejects_fake_options() {
        let script = r#"
            if (typeof PushSubscription === 'function') {
                try {
                    const sub = new PushSubscription();
                    console.log('ERROR: constructed fake options: ' + typeof sub.options);
                } catch (error) {
                    const message = String(error && error.message || error);
                    console.log(message === 'PushSubscription construction is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                }
            } else {
                console.log('ERROR: PushSubscription not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushSubscription constructor should not expose fake options: {}",
            stdout
        );
    }

    #[test]
    fn test_push_subscription_get_key_prototype_call_fails_closed() {
        let script = r#"
            if (typeof PushSubscription !== 'function') {
                console.log('ERROR: PushSubscription not defined');
            } else {
                try {
                    const key = PushSubscription.prototype.getKey.call({});
                    const byteLength = key && key.byteLength;
                    console.log('ERROR: getKey returned fake key length: ' + byteLength);
                } catch (error) {
                    const message = String(error && error.message || error);
                    console.log(message === 'PushSubscription is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                }
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushSubscription.prototype.getKey should fail closed without a real subscription: {}",
            stdout
        );
    }

    #[test]
    fn test_push_subscription_to_json_prototype_call_fails_closed() {
        let script = r#"
            if (typeof PushSubscription !== 'function') {
                console.log('ERROR: PushSubscription not defined');
            } else {
                try {
                    const json = PushSubscription.prototype.toJSON.call({});
                    console.log('ERROR: toJSON returned fake endpoint: ' + json.endpoint);
                } catch (error) {
                    const message = String(error && error.message || error);
                    console.log(message === 'PushSubscription is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                }
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushSubscription.prototype.toJSON should fail closed without a real subscription: {}",
            stdout
        );
    }

    #[test]
    fn test_push_subscription_unsubscribe_prototype_call_rejects() {
        let script = r#"
            if (typeof PushSubscription !== 'function') {
                console.log('ERROR: PushSubscription not defined');
            } else {
                const result = PushSubscription.prototype.unsubscribe.call({});
                if (!result || typeof result.then !== 'function') {
                    console.log('ERROR: unsubscribe did not return a Promise');
                } else {
                    result.then(
                        value => {
                            console.log('ERROR: unsubscribe resolved fake success: ' + value);
                        },
                        error => {
                            const message = String(error && error.message || error);
                            console.log(message === 'PushSubscription is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                        }
                    );
                }
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushSubscription.prototype.unsubscribe should reject without a real subscription: {}",
            stdout
        );
    }
}

#[cfg(test)]
mod push_event_tests {
    use super::*;

    #[test]
    fn test_push_event_exists() {
        // Test that PushEvent exists
        let script = r#"
            if (typeof PushEvent === 'function') {
                console.log('SUCCESS: PushEvent exists');
            } else {
                throw new Error('PushEvent not found');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushEvent should exist: {}",
            stdout
        );
    }

    #[test]
    fn test_push_event_is_extendable_event() {
        // Test that PushEvent extends ExtendableEvent
        let script = r#"
            if (typeof PushEvent === 'function' && typeof ExtendableEvent !== 'undefined') {
                const event = new PushEvent('push');
                if (event && typeof event.waitUntil === 'function') {
                    console.log('SUCCESS: PushEvent has waitUntil method (extends ExtendableEvent)');
                } else {
                    console.log('ERROR: PushEvent missing waitUntil method');
                }
            } else {
                console.log('ERROR: PushEvent or ExtendableEvent not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushEvent should extend ExtendableEvent: {}",
            stdout
        );
    }

    #[test]
    fn test_push_event_has_data() {
        // Test that PushEvent has data property
        let script = r#"
            if (typeof PushEvent === 'function') {
                const event = new PushEvent('push', { data: 'test message' });
                if (event && event.data && event.data === 'test message') {
                    console.log('SUCCESS: PushEvent.data property works');
                } else {
                    console.log('ERROR: PushEvent.data property not working');
                    console.log('event.data: ' + event.data);
                }
            } else {
                console.log('ERROR: PushEvent not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushEvent.data should work: {}",
            stdout
        );
    }

    #[test]
    fn test_push_event_data_null_by_default() {
        // Test that PushEvent.data is null by default
        let script = r#"
            if (typeof PushEvent === 'function') {
                const event = new PushEvent('push');
                if (event && event.data === null) {
                    console.log('SUCCESS: PushEvent.data is null by default');
                } else {
                    console.log('ERROR: PushEvent.data should be null by default');
                    console.log('event.data: ' + event.data);
                }
            } else {
                console.log('ERROR: PushEvent not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushEvent.data should be null by default: {}",
            stdout
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_push_api_global_availability() {
        // Test that all Push API components are globally available
        let script = r#"
            const components = ['PushManager', 'PushSubscription', 'PushEvent'];
            const missing = [];

            for (const name of components) {
                // Check globalThis first (works in both browser and Node.js-like environments)
                const globalObj = typeof globalThis !== 'undefined' ? globalThis :
                                  typeof window !== 'undefined' ? window :
                                  typeof global !== 'undefined' ? global : {};
                if (typeof globalObj[name] === 'undefined') {
                    missing.push(name);
                }
            }

            if (missing.length === 0) {
                console.log('SUCCESS: All Push API components are globally available');
            } else {
                console.log('ERROR: Missing components: ' + missing.join(', '));
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "All Push API should be available: {}",
            stdout
        );
    }

    #[test]
    fn test_push_subscription_instance_methods_fail_closed() {
        let script = r#"
            if (typeof PushSubscription === 'function') {
                try {
                    const sub = new PushSubscription();
                    console.log('ERROR: constructed fake methods: ' + [typeof sub.getKey, typeof sub.toJSON, typeof sub.unsubscribe].join(','));
                } catch (error) {
                    const message = String(error && error.message || error);
                    console.log(message === 'PushSubscription construction is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                }
            } else {
                console.log('ERROR: PushSubscription not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushSubscription instance method shell should fail closed: {}",
            stdout
        );
    }

    #[test]
    fn test_push_subscription_instance_properties_fail_closed() {
        let script = r#"
            if (typeof PushSubscription === 'function') {
                try {
                    const sub = new PushSubscription();
                    console.log('ERROR: constructed fake properties: ' + sub.endpoint + '|' + typeof sub.options);
                } catch (error) {
                    const message = String(error && error.message || error);
                    console.log(message === 'PushSubscription construction is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                }
            } else {
                console.log('ERROR: PushSubscription not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "PushSubscription instance property shell should fail closed: {}",
            stdout
        );
    }
}

/// Helper function to run JavaScript scripts using amberjs
fn run_script(script: &str) -> std::process::Output {
    // Create a temporary file with the script
    let temp_dir = tempfile::Builder::new()
        .prefix("amberjs-push-test-")
        .tempdir()
        .unwrap();
    let temp_file = temp_dir.path().join("test.js");
    fs::write(&temp_file, script).unwrap();

    // Run amberjs with the script
    let output = Command::new(amber_path())
        .arg("run")
        .arg(&temp_file)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to run amber");

    // Clean up
    drop(temp_dir);

    output
}
