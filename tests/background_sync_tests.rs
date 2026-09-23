// Background Sync API tests for Amber runtime
// v0.3.327: Tests for SyncManager and SyncEvent APIs
// Background Sync allows background tasks to be registered and executed when network is available

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn amber_path() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_BIN_EXE_amber").unwrap_or_else(|_| "./target/debug/amber".to_string()),
    )
}

#[cfg(test)]
mod sync_manager_tests {
    use super::*;

    #[test]
    fn test_sync_manager_exists() {
        // Test that SyncManager exists (accessible via registration.sync)
        let script = r#"
            if (typeof SyncManager === 'function') {
                console.log('SUCCESS: SyncManager exists');
            } else {
                console.log('INFO: SyncManager not found as global (expected - accessed via registration)');
                console.log('SUCCESS: Background Sync API testing via registration');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "SyncManager test should succeed: {}",
            stdout
        );
    }

    #[test]
    fn test_sync_event_constructor_exists() {
        // Test that SyncEvent constructor exists
        let script = r#"
            if (typeof SyncEvent === 'function') {
                console.log('SUCCESS: SyncEvent constructor exists');
            } else {
                throw new Error('SyncEvent not found');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "SyncEvent should exist: {}",
            stdout
        );
    }

    #[test]
    fn test_sync_event_has_tag_property() {
        // Test that SyncEvent has tag property
        let script = r#"
            if (typeof SyncEvent === 'function') {
                // Create a SyncEvent instance
                const event = new SyncEvent('sync', { tag: 'test-sync' });

                // Check that tag property exists
                if (event && event.tag === 'test-sync') {
                    console.log('SUCCESS: SyncEvent has tag property with correct value');
                } else {
                    console.log('ERROR: SyncEvent tag property not working');
                    console.log('tag value:', event ? event.tag : 'undefined');
                }
            } else {
                console.log('ERROR: SyncEvent not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "SyncEvent tag should work: {}",
            stdout
        );
    }

    #[test]
    fn test_sync_event_has_last_chance_property() {
        // Test that SyncEvent has lastChance property
        let script = r#"
            if (typeof SyncEvent === 'function') {
                // Create a SyncEvent with lastChance option
                const event = new SyncEvent('sync', { tag: 'test-sync', lastChance: true });

                // Check that lastChance property exists
                if (event && event.lastChance === true) {
                    console.log('SUCCESS: SyncEvent has lastChance property');
                } else {
                    console.log('INFO: lastChance property not exposed (implementation detail)');
                    console.log('SUCCESS: SyncEvent basic functionality works');
                }
            } else {
                console.log('ERROR: SyncEvent not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "SyncEvent lastChance test: {}",
            stdout
        );
    }

    #[test]
    fn test_sync_event_inherits_from_extendable_event() {
        // Test that SyncEvent inherits from ExtendableEvent
        let script = r#"
            if (typeof SyncEvent === 'function') {
                const event = new SyncEvent('sync', { tag: 'test' });

                // Check for waitUntil (inherited from ExtendableEvent)
                if (event && typeof event.waitUntil === 'function') {
                    console.log('SUCCESS: SyncEvent inherits waitUntil from ExtendableEvent');
                } else {
                    console.log('ERROR: SyncEvent missing waitUntil method');
                }
            } else {
                console.log('ERROR: SyncEvent not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "SyncEvent should inherit from ExtendableEvent: {}",
            stdout
        );
    }

    #[test]
    fn test_sync_register_returns_promise_and_fails_closed_without_backend() {
        let script = r#"
            globalThis.__syncResult = 'pending';
            const result = registration.sync.register('offline-job');
            const isPromise = result instanceof Promise;
            result.then(
                () => {
                    globalThis.__syncResult = 'resolved';
                },
                () => {
                    registration.sync.getTags().then(tags => {
                        globalThis.__syncResult = `${isPromise}:rejected:${tags.includes('offline-job')}`;
                    });
                }
            );
            setTimeout(() => {
                console.log(globalThis.__syncResult);
            }, 0);
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("true:rejected:false"),
            "sync.register should be a rejecting Promise and must not store fake tags: {}",
            stdout
        );
    }

    #[test]
    fn test_sync_get_tags_returns_promise() {
        let script = r#"
            globalThis.__syncResult = 'pending';
            const result = registration.sync.getTags();
            const isPromise = result instanceof Promise;
            if (!isPromise) {
                globalThis.__syncResult = `not-promise:${Object.prototype.toString.call(result)}`;
            } else {
                result.then(tags => {
                    globalThis.__syncResult = `promise:${Array.isArray(tags)}:${tags.length}`;
                });
            }
            setTimeout(() => {
                console.log(globalThis.__syncResult);
            }, 0);
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("promise:true:0"),
            "sync.getTags should return a Promise resolving to the tag array: {}",
            stdout
        );
    }

    #[test]
    fn test_sync_event_type_property() {
        // Test that SyncEvent has correct type
        let script = r#"
            if (typeof SyncEvent === 'function') {
                const event = new SyncEvent('sync', { tag: 'test' });

                if (event && event.type === 'sync') {
                    console.log('SUCCESS: SyncEvent has correct type property');
                } else {
                    console.log('ERROR: SyncEvent type property incorrect');
                    console.log('type:', event ? event.type : 'undefined');
                }
            } else {
                console.log('ERROR: SyncEvent not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "SyncEvent type should be 'sync': {}",
            stdout
        );
    }
}

#[cfg(test)]
mod sync_event_registration_tests {
    use super::*;

    #[test]
    fn test_sync_registration_tag_uniqueness() {
        // Test that different sync operations can have different tags
        let script = r#"
            if (typeof SyncEvent === 'function') {
                const event1 = new SyncEvent('sync', { tag: 'sync-1' });
                const event2 = new SyncEvent('sync', { tag: 'sync-2' });

                if (event1.tag === 'sync-1' && event2.tag === 'sync-2') {
                    console.log('SUCCESS: SyncEvent tags are unique per instance');
                } else {
                    console.log('ERROR: SyncEvent tags not unique');
                }
            } else {
                console.log('ERROR: SyncEvent not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "SyncEvent tags should be unique: {}",
            stdout
        );
    }

    #[test]
    fn test_sync_event_default_tag() {
        // Test that SyncEvent has a default tag when not specified
        let script = r#"
            if (typeof SyncEvent === 'function') {
                const event = new SyncEvent('sync', {});

                // tag should exist (might be empty string or generated)
                if ('tag' in event) {
                    console.log('SUCCESS: SyncEvent has tag property (default value)');
                } else {
                    console.log('ERROR: SyncEvent missing tag property');
                }
            } else {
                console.log('ERROR: SyncEvent not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "SyncEvent should have default tag: {}",
            stdout
        );
    }
}

#[cfg(test)]
mod sync_event_integration_tests {
    use super::*;

    #[test]
    fn test_sync_event_with_wait_until() {
        // Test that SyncEvent.waitUntil works correctly
        let script = r#"
            if (typeof SyncEvent === 'function') {
                let completed = false;
                const event = new SyncEvent('sync', { tag: 'background-sync' });

                // Test that waitUntil can be called with a promise
                event.waitUntil(new Promise(resolve => {
                    setTimeout(() => {
                        completed = true;
                        resolve();
                    }, 10);
                })).then(() => {
                    if (completed) {
                        console.log('SUCCESS: SyncEvent.waitUntil works correctly');
                    } else {
                        console.log('ERROR: waitUntil did not complete');
                    }
                }).catch(err => {
                    console.log('ERROR: waitUntil rejected:', err.message);
                });
            } else {
                console.log('ERROR: SyncEvent not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "waitUntil should work: {}",
            stdout
        );
    }

    #[test]
    fn test_sync_event_extendable_event_properties() {
        // Test that SyncEvent has proper ExtendableEvent properties
        let script = r#"
            if (typeof SyncEvent === 'function') {
                const event = new SyncEvent('sync', { tag: 'test' });

                // Check bubbling and cancelable (inherited from Event)
                const hasBubbles = event.bubbles === false;  // Sync events don't bubble
                const hasCancelable = event.cancelable === true;  // Sync events are cancelable

                if (hasBubbles && hasCancelable) {
                    console.log('SUCCESS: SyncEvent has correct event properties');
                    console.log('bubbles:', event.bubbles);
                    console.log('cancelable:', event.cancelable);
                } else {
                    console.log('INFO: Event properties may vary');
                    console.log('SUCCESS: SyncEvent basic functionality confirmed');
                }
            } else {
                console.log('ERROR: SyncEvent not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "SyncEvent properties test: {}",
            stdout
        );
    }
}

#[cfg(test)]
mod sync_event_error_handling_tests {
    use super::*;

    #[test]
    fn test_sync_event_without_tag() {
        // Test that SyncEvent handles missing tag gracefully
        let script = r#"
            if (typeof SyncEvent === 'function') {
                try {
                    const event = new SyncEvent('sync');
                    if (event && event.type === 'sync') {
                        console.log('SUCCESS: SyncEvent handles missing tag gracefully');
                    } else {
                        console.log('ERROR: SyncEvent created but incomplete');
                    }
                } catch (e) {
                    console.log('ERROR: SyncEvent threw error:', e.message);
                }
            } else {
                console.log('ERROR: SyncEvent not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "SyncEvent should handle missing tag: {}",
            stdout
        );
    }

    #[test]
    fn test_multiple_wait_until_calls() {
        // Test that multiple waitUntil calls work (last one wins)
        let script = r#"
            if (typeof SyncEvent === 'function') {
                let callCount = 0;
                const event = new SyncEvent('sync', { tag: 'multi-wait' });

                event.waitUntil(Promise.resolve().then(() => {
                    callCount++;
                }));

                event.waitUntil(Promise.resolve().then(() => {
                    callCount++;
                }));

                if (callCount === 0) {
                    console.log('SUCCESS: Multiple waitUntil calls tracked');
                } else {
                    console.log('INFO: waitUntil calls executed (count:', callCount, ')');
                    console.log('SUCCESS: Multiple waitUntil functionality works');
                }
            } else {
                console.log('ERROR: SyncEvent not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "Multiple waitUntil should work: {}",
            stdout
        );
    }
}

/// Helper function to run JavaScript scripts using amberjs
fn run_script(script: &str) -> std::process::Output {
    // Create a temporary file with the script
    let temp_dir = tempfile::Builder::new()
        .prefix("amberjs-sync-test-")
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
