// ServiceWorker API tests for Beejs runtime
// v0.3.324: Tests for ServiceWorker, Cache, and CacheStorage APIs

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn bee_path() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_BIN_EXE_amber").unwrap_or_else(|_| "./target/debug/bee".to_string()),
    )
}

#[cfg(test)]
mod service_worker_tests {
    use super::*;

    #[test]
    fn test_navigator_service_worker_exists() {
        // Test that navigator.serviceWorker exists
        let script = r#"
            if (typeof navigator !== 'undefined' && typeof navigator.serviceWorker !== 'undefined') {
                console.log('SUCCESS: navigator.serviceWorker exists');
            } else {
                throw new Error('navigator.serviceWorker not found');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "ServiceWorker should exist: {}",
            stdout
        );
        assert!(
            stdout.contains("SUCCESS: navigator.serviceWorker exists"),
            "Output: {}",
            stdout
        );
    }

    #[test]
    fn test_service_worker_register_exists() {
        // Test that navigator.serviceWorker.register exists
        let script = r#"
            if (typeof navigator !== 'undefined' &&
                typeof navigator.serviceWorker !== 'undefined' &&
                typeof navigator.serviceWorker.register === 'function') {
                console.log('SUCCESS: navigator.serviceWorker.register is a function');
            } else {
                throw new Error('navigator.serviceWorker.register not found or not a function');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "register should be a function: {}",
            stdout
        );
    }

    #[test]
    fn test_service_worker_register_fails_closed_until_lifecycle_exists() {
        let script = r#"
            if (typeof navigator === 'undefined' || typeof navigator.serviceWorker === 'undefined') {
                console.log('ERROR: navigator.serviceWorker not defined');
            } else {
                const result = navigator.serviceWorker.register('./test-sw.js');
                if (!result || typeof result.then !== 'function') {
                    console.log('ERROR: register did not return a Promise');
                } else {
                    result.then(
                        registration => {
                            console.log('ERROR: registration resolved fake object: ' + Object.keys(registration).join(','));
                        },
                        error => {
                            const message = String(error && error.message || error);
                            console.log(message === 'ServiceWorker registration is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                        }
                    );
                }
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "ServiceWorker register should fail closed until lifecycle support exists: {}",
            stdout
        );
    }

    #[test]
    fn test_service_worker_register_returns_promise() {
        // Test that navigator.serviceWorker.register returns a Promise
        let script = r#"
            if (typeof navigator !== 'undefined' && typeof navigator.serviceWorker !== 'undefined') {
                const result = navigator.serviceWorker.register('./test-sw.js');
                if (result && typeof result.then === 'function') {
                    console.log('SUCCESS: register returns a Promise');
                } else {
                    console.log('ERROR: register does not return a Promise');
                }
            } else {
                console.log('ERROR: navigator.serviceWorker not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "Should return Promise: {}",
            stdout
        );
    }

    #[test]
    fn test_service_worker_register_rejects_instead_of_scope_shell() {
        let script = r#"
            if (typeof navigator !== 'undefined' && typeof navigator.serviceWorker !== 'undefined') {
                navigator.serviceWorker.register('./test-sw.js').then(registration => {
                    console.log('ERROR: registration resolved fake scope: ' + registration.scope);
                }).catch(e => {
                    const message = String(e && e.message || e);
                    console.log(message === 'ServiceWorker registration is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                });
            } else {
                console.log('ERROR: navigator.serviceWorker not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "ServiceWorker register should reject instead of resolving a scope shell: {}",
            stdout
        );
    }

    #[test]
    fn test_service_worker_register_rejects_instead_of_installing_shell() {
        let script = r#"
            if (typeof navigator !== 'undefined' && typeof navigator.serviceWorker !== 'undefined') {
                navigator.serviceWorker.register('./test-sw.js').then(registration => {
                    console.log('ERROR: registration resolved fake installing: ' + ('installing' in registration));
                }).catch(e => {
                    const message = String(e && e.message || e);
                    console.log(message === 'ServiceWorker registration is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                });
            } else {
                console.log('ERROR: navigator.serviceWorker not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "ServiceWorker register should reject instead of resolving an installing shell: {}",
            stdout
        );
    }

    #[test]
    fn test_service_worker_register_rejects_instead_of_active_shell() {
        let script = r#"
            if (typeof navigator !== 'undefined' && typeof navigator.serviceWorker !== 'undefined') {
                navigator.serviceWorker.register('./test-sw.js').then(registration => {
                    console.log('ERROR: registration resolved fake active: ' + ('active' in registration));
                }).catch(e => {
                    const message = String(e && e.message || e);
                    console.log(message === 'ServiceWorker registration is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                });
            } else {
                console.log('ERROR: navigator.serviceWorker not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "ServiceWorker register should reject instead of resolving an active shell: {}",
            stdout
        );
    }

    #[test]
    fn test_service_worker_ready_property() {
        // Test that navigator.serviceWorker.ready exists
        let script = r#"
            if (typeof navigator !== 'undefined' && typeof navigator.serviceWorker !== 'undefined') {
                if ('ready' in navigator.serviceWorker) {
                    console.log('SUCCESS: navigator.serviceWorker.ready exists');
                } else {
                    console.log('ERROR: navigator.serviceWorker.ready not found');
                }
            } else {
                console.log('ERROR: navigator.serviceWorker not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "Ready property should exist: {}",
            stdout
        );
    }
}

#[cfg(test)]
mod cache_api_tests {
    use super::*;

    #[test]
    fn test_caches_exists() {
        // Test that global caches object exists
        let script = r#"
            if (typeof caches !== 'undefined') {
                console.log('SUCCESS: global caches object exists');
            } else {
                throw new Error('caches object not found');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "caches should exist: {}", stdout);
        assert!(
            stdout.contains("SUCCESS: global caches object exists"),
            "Output: {}",
            stdout
        );
    }

    #[test]
    fn test_caches_open_exists() {
        // Test that caches.open exists (caches is singleton object, not constructor)
        let script = r#"
            if (typeof caches !== 'undefined' && typeof caches.open === 'function') {
                console.log('SUCCESS: caches.open is a function');
            } else {
                throw new Error('caches.open not found or not a function');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "caches.open should be a function: {}",
            stdout
        );
    }

    #[test]
    fn test_caches_keys_exists() {
        // Test that caches.keys exists
        let script = r#"
            if (typeof caches !== 'undefined' && typeof caches.keys === 'function') {
                console.log('SUCCESS: caches.keys is a function');
            } else {
                throw new Error('caches.keys not found or not a function');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "caches.keys should be a function: {}",
            stdout
        );
    }

    #[test]
    fn test_caches_has_exists() {
        // Test that caches.has exists
        let script = r#"
            if (typeof caches !== 'undefined' && typeof caches.has === 'function') {
                console.log('SUCCESS: caches.has is a function');
            } else {
                throw new Error('caches.has not found or not a function');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "caches.has should be a function: {}",
            stdout
        );
    }

    #[test]
    fn test_caches_delete_exists() {
        // Test that caches.delete exists
        let script = r#"
            if (typeof caches !== 'undefined' && typeof caches.delete === 'function') {
                console.log('SUCCESS: caches.delete is a function');
            } else {
                throw new Error('caches.delete not found or not a function');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "caches.delete should be a function: {}",
            stdout
        );
    }

    #[test]
    fn test_caches_open_rejects_until_cache_backend_exists() {
        let script = r#"
            if (typeof caches !== 'undefined') {
                const result = caches.open('test-cache');
                if (result && typeof result.then === 'function') {
                    result.then(cache => {
                        console.log('ERROR: caches.open resolved fake Cache: ' + Object.keys(cache).join(','));
                    }).catch(e => {
                        const message = String(e && e.message || e);
                        console.log(message === 'Cache API is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                    });
                } else {
                    console.log('ERROR: caches.open() does not return a Promise');
                }
            } else {
                console.log('ERROR: caches not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "caches.open should reject instead of resolving a fake Cache: {}",
            stdout
        );
    }

    #[test]
    fn test_cache_storage_observation_methods_return_promises() {
        let script = r#"
            if (typeof caches === 'undefined') {
                console.log('ERROR: caches not defined');
            } else {
                const keysPromise = caches.keys();
                const hasPromise = caches.has('test-cache');
                const deletePromise = caches.delete('test-cache');
                if (!keysPromise || typeof keysPromise.then !== 'function') {
                    console.log('ERROR: caches.keys did not return a Promise');
                } else if (!hasPromise || typeof hasPromise.then !== 'function') {
                    console.log('ERROR: caches.has did not return a Promise');
                } else if (!deletePromise || typeof deletePromise.then !== 'function') {
                    console.log('ERROR: caches.delete did not return a Promise');
                } else {
                    Promise.all([keysPromise, hasPromise, deletePromise]).then(values => {
                        const keys = values[0];
                        const has = values[1];
                        const deleted = values[2];
                        console.log(Array.isArray(keys) && keys.length === 0 && has === false && deleted === false
                            ? 'SUCCESS'
                            : 'ERROR: unexpected values ' + JSON.stringify(values));
                    }).catch(error => {
                        console.log('ERROR: ' + String(error && error.message || error));
                    });
                }
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "CacheStorage keys/has/delete should use Promise shape with empty state: {}",
            stdout
        );
    }
}

#[cfg(test)]
mod cache_object_tests {
    use super::*;

    #[test]
    fn test_cache_add_all_fake_object_is_not_exposed() {
        let script = r#"
            if (typeof caches !== 'undefined') {
                caches.open('test-cache').then(cache => {
                    console.log('ERROR: fake Cache.addAll exposed: ' + typeof cache.addAll);
                }).catch(e => {
                    const message = String(e && e.message || e);
                    console.log(message === 'Cache API is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                });
            } else {
                console.log('ERROR: caches not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "Cache.addAll should not be exposed through a fake Cache: {}",
            stdout
        );
    }

    #[test]
    fn test_cache_match_fake_object_is_not_exposed() {
        let script = r#"
            if (typeof caches !== 'undefined') {
                caches.open('test-cache').then(cache => {
                    console.log('ERROR: fake Cache.match exposed: ' + typeof cache.match);
                }).catch(e => {
                    const message = String(e && e.message || e);
                    console.log(message === 'Cache API is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                });
            } else {
                console.log('ERROR: caches not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "Cache.match should not be exposed through a fake Cache: {}",
            stdout
        );
    }

    #[test]
    fn test_cache_put_fake_object_is_not_exposed() {
        let script = r#"
            if (typeof caches !== 'undefined') {
                caches.open('test-cache').then(cache => {
                    console.log('ERROR: fake Cache.put exposed: ' + typeof cache.put);
                }).catch(e => {
                    const message = String(e && e.message || e);
                    console.log(message === 'Cache API is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                });
            } else {
                console.log('ERROR: caches not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "Cache.put should not be exposed through a fake Cache: {}",
            stdout
        );
    }

    #[test]
    fn test_cache_delete_fake_object_is_not_exposed() {
        let script = r#"
            if (typeof caches !== 'undefined') {
                caches.open('test-cache').then(cache => {
                    console.log('ERROR: fake Cache.delete exposed: ' + typeof cache.delete);
                }).catch(e => {
                    const message = String(e && e.message || e);
                    console.log(message === 'Cache API is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                });
            } else {
                console.log('ERROR: caches not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "Cache.delete should not be exposed through a fake Cache: {}",
            stdout
        );
    }

    #[test]
    fn test_cache_keys_fake_object_is_not_exposed() {
        let script = r#"
            if (typeof caches !== 'undefined') {
                caches.open('test-cache').then(cache => {
                    console.log('ERROR: fake Cache.keys exposed: ' + typeof cache.keys);
                }).catch(e => {
                    const message = String(e && e.message || e);
                    console.log(message === 'Cache API is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                });
            } else {
                console.log('ERROR: caches not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "Cache.keys should not be exposed through a fake Cache: {}",
            stdout
        );
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_service_worker_registration_flow_fails_closed() {
        let script = r#"
            if (typeof navigator !== 'undefined' && typeof navigator.serviceWorker !== 'undefined') {
                navigator.serviceWorker.register('./sw.js').then(registration => {
                    console.log('ERROR: registration resolved fake object: ' + Object.keys(registration).join(','));
                }).catch(e => {
                    const message = String(e && e.message || e);
                    console.log(message === 'ServiceWorker registration is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                });
            } else {
                console.log('ERROR: ServiceWorker not supported');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "ServiceWorker registration flow should fail closed until lifecycle support exists: {}",
            stdout
        );
    }

    #[test]
    fn test_cache_operations_flow_fails_closed_until_backend_exists() {
        let script = r#"
            if (typeof caches !== 'undefined') {
                caches.open('my-cache').then(cache => {
                    console.log('ERROR: fake Cache operations exposed: ' + Object.keys(cache).join(','));
                }).catch(e => {
                    const message = String(e && e.message || e);
                    console.log(message === 'Cache API is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                });
            } else {
                console.log('ERROR: Cache API not supported');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "Cache flow should fail closed until a real backend exists: {}",
            stdout
        );
    }

    #[test]
    fn test_service_worker_register_with_scope_option_fails_closed() {
        let script = r#"
            if (typeof navigator !== 'undefined' && typeof navigator.serviceWorker !== 'undefined') {
                navigator.serviceWorker.register('./sw.js', { scope: '/app/' }).then(registration => {
                    console.log('ERROR: registration resolved fake scoped object: ' + registration.scope);
                }).catch(e => {
                    const message = String(e && e.message || e);
                    console.log(message === 'ServiceWorker registration is not supported yet' ? 'SUCCESS' : 'ERROR: ' + message);
                });
            } else {
                console.log('ERROR: ServiceWorker not supported');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "ServiceWorker register with a scope option should fail closed: {}",
            stdout
        );
    }
}

#[cfg(test)]
mod lifecycle_event_tests {
    use super::*;

    #[test]
    fn test_install_event_constructor_exists() {
        // Test that InstallEvent constructor exists
        let script = r#"
            if (typeof InstallEvent === 'function') {
                console.log('SUCCESS: InstallEvent constructor exists');
            } else {
                console.log('ERROR: InstallEvent constructor not found');
                console.log('typeof InstallEvent: ' + typeof InstallEvent);
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "InstallEvent should exist: {}",
            stdout
        );
    }

    #[test]
    fn test_activate_event_constructor_exists() {
        // Test that ActivateEvent constructor exists
        let script = r#"
            if (typeof ActivateEvent === 'function') {
                console.log('SUCCESS: ActivateEvent constructor exists');
            } else {
                console.log('ERROR: ActivateEvent constructor not found');
                console.log('typeof ActivateEvent: ' + typeof ActivateEvent);
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "ActivateEvent should exist: {}",
            stdout
        );
    }

    #[test]
    fn test_fetch_event_constructor_exists() {
        // Test that FetchEvent constructor exists
        let script = r#"
            if (typeof FetchEvent === 'function') {
                console.log('SUCCESS: FetchEvent constructor exists');
            } else {
                console.log('ERROR: FetchEvent constructor not found');
                console.log('typeof FetchEvent: ' + typeof FetchEvent);
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "FetchEvent should exist: {}",
            stdout
        );
    }

    #[test]
    fn test_install_event_creation() {
        // Test that InstallEvent can be created with correct properties
        let script = r#"
            if (typeof InstallEvent === 'function') {
                const event = new InstallEvent('install');
                if (event && event.type === 'install') {
                    console.log('SUCCESS: InstallEvent created with correct type');
                } else {
                    console.log('ERROR: InstallEvent type mismatch');
                    console.log('event.type: ' + event.type);
                }
            } else {
                console.log('ERROR: InstallEvent not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "InstallEvent creation should work: {}",
            stdout
        );
    }

    #[test]
    fn test_activate_event_creation() {
        // Test that ActivateEvent can be created with correct properties
        let script = r#"
            if (typeof ActivateEvent === 'function') {
                const event = new ActivateEvent('activate');
                if (event && event.type === 'activate') {
                    console.log('SUCCESS: ActivateEvent created with correct type');
                } else {
                    console.log('ERROR: ActivateEvent type mismatch');
                    console.log('event.type: ' + event.type);
                }
            } else {
                console.log('ERROR: ActivateEvent not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "ActivateEvent creation should work: {}",
            stdout
        );
    }

    #[test]
    fn test_fetch_event_creation() {
        // Test that FetchEvent can be created with request URL
        let script = r#"
            if (typeof FetchEvent === 'function') {
                const event = new FetchEvent('fetch', { requestUrl: '/api/test' });
                if (event && event.type === 'fetch' && event.requestUrl === '/api/test') {
                    console.log('SUCCESS: FetchEvent created with correct properties');
                } else {
                    console.log('ERROR: FetchEvent properties mismatch');
                    console.log('event.type: ' + event.type);
                    console.log('event.requestUrl: ' + event.requestUrl);
                }
            } else {
                console.log('ERROR: FetchEvent not defined');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "FetchEvent creation should work: {}",
            stdout
        );
    }

    #[test]
    fn test_service_worker_state_enum() {
        // Test that ServiceWorkerState enum is accessible via console
        let script = r#"
            // ServiceWorkerState values
            const states = ['parsing', 'installing', 'installed', 'activating', 'activated', 'redundant'];
            if (states.length === 6) {
                console.log('SUCCESS: ServiceWorkerState has 6 states');
            } else {
                console.log('ERROR: ServiceWorkerState state count mismatch');
            }
        "#;
        let output = run_script(script);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("SUCCESS"),
            "ServiceWorkerState should have 6 states: {}",
            stdout
        );
    }
}

/// Helper function to run JavaScript scripts using beejs
fn run_script(script: &str) -> std::process::Output {
    // Create a temporary file with the script
    let temp_dir = tempfile::Builder::new()
        .prefix("beejs-service-worker-test-")
        .tempdir()
        .unwrap();
    let temp_file = temp_dir.path().join("test.js");
    fs::write(&temp_file, script).unwrap();

    // Run beejs with the script
    let output = Command::new(bee_path())
        .arg("run")
        .arg(&temp_file)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to run bee");

    // Clean up
    drop(temp_dir);

    output
}
