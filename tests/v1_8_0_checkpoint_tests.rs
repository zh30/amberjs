use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_checkpoint_save_and_restore() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const { createCheckpointManager } = require('amber:checkpoint');
    const mgr = createCheckpointManager();

    // 1. Initial checkpoint
    const cp1 = mgr.save('step_1', {
        agentState: 'init',
        variables: { counter: 10, items: ['apple'] }
    });
    if (cp1.id !== 'step_1') throw new Error('cp1 id mismatch: ' + cp1.id);

    // 2. Second checkpoint
    const cp2 = mgr.save('step_2', {
        agentState: 'processing',
        variables: { counter: 20, items: ['apple', 'banana'] }
    });
    if (cp2.parent_id !== 'step_1') throw new Error('cp2 parent mismatch: ' + cp2.parent_id);

    // 3. Third checkpoint with failure
    mgr.save('step_3_failed', {
        agentState: 'error',
        variables: { counter: 999, items: [] }
    });

    // 4. Restore state to step_2
    const restored = mgr.restore('step_2');
    if (restored.agentState !== 'processing' || restored.variables.counter !== 20) {
        throw new Error('State restore failed: ' + JSON.stringify(restored));
    }

    JSON.stringify({ success: true, restored });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_checkpoint_state_diffing() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const { createCheckpointManager } = require('amber:checkpoint');
    const mgr = createCheckpointManager();

    mgr.save('v1', {
        title: 'Original',
        unchanged: 100,
        toDelete: true
    });

    mgr.save('v2', {
        title: 'Modified',
        unchanged: 100,
        addedField: 'new value'
    });

    const diff = mgr.diff('v1', 'v2');

    if (!diff.added || diff.added.addedField !== 'new value') {
        throw new Error('Diff added field mismatch: ' + JSON.stringify(diff.added));
    }

    if (!diff.modified || diff.modified.title.from !== 'Original' || diff.modified.title.to !== 'Modified') {
        throw new Error('Diff modified field mismatch: ' + JSON.stringify(diff.modified));
    }

    if (!diff.deleted || !diff.deleted.includes('toDelete')) {
        throw new Error('Diff deleted field mismatch: ' + JSON.stringify(diff.deleted));
    }

    JSON.stringify({ success: true, diff });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_checkpoint_tree_branching_and_fork() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const { createCheckpointManager } = require('amber:checkpoint');
    const mainMgr = createCheckpointManager();

    mainMgr.save('root', { path: 'root' });
    mainMgr.save('branch_point', { path: 'branch_point', value: 1 });

    // Fork a speculative exploration branch
    const branchMgr = mainMgr.fork('branch_point', 'speculative_reasoning');
    branchMgr.save('spec_1', { path: 'spec_1', value: 99 });

    // Main line continues independently
    mainMgr.save('main_3', { path: 'main_3', value: 2 });

    const mainList = mainMgr.list();
    const branchList = branchMgr.list();

    if (mainList.length !== 3) throw new Error('Main lineage length mismatch: ' + mainList.length);
    if (branchList.length !== 3) throw new Error('Branch lineage length mismatch: ' + branchList.length);

    if (branchList[2].id !== 'spec_1' || branchList[2].branch !== 'speculative_reasoning') {
        throw new Error('Forked branch checkpoint mismatch: ' + JSON.stringify(branchList[2]));
    }

    JSON.stringify({ success: true, mainLen: mainList.length, branchLen: branchList.length });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_checkpoint_kv_store_persistence() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const { createCheckpointManager } = require('amber:checkpoint');
    const { openInMemory } = require('amber:kv');

    const kv = openInMemory();
    const mgr1 = createCheckpointManager();

    mgr1.save('persist_1', { step: 1, text: 'Hello KV' });
    mgr1.save('persist_2', { step: 2, text: 'World KV' });

    // Persist checkpoints into KV store
    const persistedCount = mgr1.persist(kv, 'agent_cp:');
    if (persistedCount !== 2) throw new Error('Persist count mismatch: ' + persistedCount);

    // Create a fresh manager and restore from KV store
    const mgr2 = createCheckpointManager();
    const restoredCount = mgr2.restoreFromKV(kv, 'agent_cp:');
    if (restoredCount !== 2) throw new Error('Restored count mismatch: ' + restoredCount);

    const cp2 = mgr2.get('persist_2');
    if (!cp2 || cp2.state.text !== 'World KV') {
        throw new Error('Restored checkpoint content mismatch: ' + JSON.stringify(cp2));
    }

    JSON.stringify({ success: true, persistedCount, restoredCount });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}
