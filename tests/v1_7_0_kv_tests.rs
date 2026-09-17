use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;
use tempfile::tempdir;

#[test]
#[serial]
fn test_kv_in_memory_and_basic_operations() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime creation");

    let code = r#"
    const { KVStore } = require('bee:kv');
    const store = KVStore.openMemory();

    // 1. Basic Set and Get
    store.set('username', 'alice');
    store.set('config', { theme: 'dark', retries: 3 });
    store.set('tags', ['ai', 'agent', 'v1.7']);
    store.set('counter', 42);

    if (store.get('username') !== 'alice') throw new Error('username mismatch');
    if (store.get('config').theme !== 'dark') throw new Error('config mismatch');
    if (store.get('tags').length !== 3) throw new Error('tags mismatch');
    if (store.get('counter') !== 42) throw new Error('counter mismatch');

    // 2. Has and Delete
    if (!store.has('username')) throw new Error('has username failed');
    const deleted = store.delete('username');
    if (!deleted) throw new Error('delete failed');
    if (store.has('username')) throw new Error('has after delete failed');
    if (store.get('username') !== undefined) throw new Error('get after delete should be undefined');

    // 3. Clear
    store.clear();
    if (store.has('config')) throw new Error('clear failed, config still present');

    JSON.stringify({ success: true });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_kv_disk_persistence_and_wal_reload() {
    let dir = tempdir().expect("tempdir");
    let kv_path = dir.path().join("agent_state.bee-kv");
    let kv_path_str = kv_path.to_string_lossy().replace('\\', "\\\\");

    // Phase 1: Write state to disk-backed KVStore
    {
        let mut runtime = MinimalRuntime::new().expect("MinimalRuntime 1");
        let code = format!(
            r#"
            const kv = require('bee:kv');
            const store = kv.open("{path}");

            store.set('session:1', {{ agent: 'researcher', step: 5 }});
            store.set('session:2', {{ agent: 'coder', step: 12 }});
            store.set('metadata:version', '1.7.0');

            store.flush();
            store.close();
            JSON.stringify({{ success: true }});
            "#,
            path = kv_path_str
        );
        let res = runtime
            .execute_code(&code)
            .expect("Phase 1 execution failed");
        assert!(res.contains("\"success\":true"));
    }

    // Phase 2: Open in a completely fresh isolate and verify replayed WAL
    {
        let mut runtime = MinimalRuntime::new().expect("MinimalRuntime 2");
        let code = format!(
            r#"
            const {{ KVStore }} = require('bee:kv');
            const store = KVStore.open("{path}");

            const s1 = store.get('session:1');
            if (!s1 || s1.agent !== 'researcher' || s1.step !== 5) {{
                throw new Error('session:1 mismatch: ' + JSON.stringify(s1));
            }}

            const s2 = store.get('session:2');
            if (!s2 || s2.step !== 12) {{
                throw new Error('session:2 mismatch');
            }}

            const ver = store.get('metadata:version');
            if (ver !== '1.7.0') throw new Error('version mismatch: ' + ver);

            // Compact and verify
            store.compact();
            if (store.get('session:1').agent !== 'researcher') {{
                throw new Error('Post-compact read failed');
            }}

            store.close();
            JSON.stringify({{ success: true }});
            "#,
            path = kv_path_str
        );
        let res = runtime
            .execute_code(&code)
            .expect("Phase 2 execution failed");
        assert!(res.contains("\"success\":true"));
    }
}

#[test]
#[serial]
fn test_kv_prefix_scanning_and_ttl() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const { KVStore } = require('bee:kv');
    const store = KVStore.openMemory();

    store.set('agent:memory:1', 'Fact A');
    store.set('agent:memory:2', 'Fact B');
    store.set('agent:memory:3', 'Fact C');
    store.set('agent:config:model', 'qwen');
    store.set('user:name', 'Bob');

    // Scan prefix
    const memEntries = store.scan({ prefix: 'agent:memory:' });
    if (memEntries.length !== 3) throw new Error('Scan length mismatch: ' + memEntries.length);
    if (memEntries[0][0] !== 'agent:memory:1') throw new Error('Scan entry 0 mismatch');

    // Scan with limit
    const limited = store.scan({ prefix: 'agent:memory:', limit: 2 });
    if (limited.length !== 2) throw new Error('Limited scan mismatch');

    // Keys with prefix
    const keys = store.keys('agent:memory:');
    if (keys.length !== 3) throw new Error('Keys mismatch: ' + keys.length);

    // TTL expiration test (using 20ms TTL)
    store.set('temp_token', 'xyz123', { ttlMs: 20 });
    if (store.get('temp_token') !== 'xyz123') throw new Error('temp_token should be active initially');

    // Busy wait 50ms to exceed TTL
    const start = Date.now();
    while (Date.now() - start < 60) {}

    if (store.has('temp_token')) throw new Error('temp_token should have expired');
    if (store.get('temp_token') !== undefined) throw new Error('get(temp_token) should return undefined');

    JSON.stringify({ success: true, count: memEntries.length });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_kv_atomic_increments_and_batch() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const { KVStore } = require('bee:kv');
    const store = KVStore.openMemory();

    // 1. Atomic increments
    const val1 = store.incr('hits', 1);
    if (val1 !== 1) throw new Error('incr 1 mismatch: ' + val1);

    const val2 = store.incr('hits', 5);
    if (val2 !== 6) throw new Error('incr 2 mismatch: ' + val2);

    const val3 = store.incr('hits', -2);
    if (val3 !== 4) throw new Error('incr 3 mismatch: ' + val3);

    // 2. Batch operations
    store.batch([
        { type: 'put', key: 'batch:k1', value: 'v1' },
        { type: 'put', key: 'batch:k2', value: { x: 100 } },
        { type: 'del', key: 'hits' }
    ]);

    if (store.get('batch:k1') !== 'v1') throw new Error('batch put 1 failed');
    if (store.get('batch:k2').x !== 100) throw new Error('batch put 2 failed');
    if (store.has('hits')) throw new Error('batch del failed');

    JSON.stringify({ success: true });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}
