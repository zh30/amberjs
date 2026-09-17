// Tests for Beejs v1.6.0 Deterministic Agent Replay Engine (`bee:replay`)

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_replay_module_resolution_and_exports() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create MinimalRuntime");
    let code = r#"
        const replay = require('bee:replay');
        const replayAlias = require('replay');
        if (typeof replay.startRecording !== 'function') throw new Error('Missing startRecording');
        if (typeof replay.stopRecording !== 'function') throw new Error('Missing stopRecording');
        if (typeof replay.loadTrace !== 'function') throw new Error('Missing loadTrace');
        if (typeof replay.step !== 'function') throw new Error('Missing step');
        if (typeof replay.isRecording !== 'function') throw new Error('Missing isRecording');
        if (typeof replay.isReplaying !== 'function') throw new Error('Missing isReplaying');
        if (typeof replay.getTraceStats !== 'function') throw new Error('Missing getTraceStats');
        if (typeof replay.reset !== 'function') throw new Error('Missing reset');
        if (replay !== replayAlias) throw new Error('Alias mismatch');
        'OK';
    "#;
    let res = runtime.execute_code(code).expect("Script execution failed");
    assert!(res.contains("OK"));
}

#[test]
#[serial]
fn test_agent_step_recording_and_trace_export() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create MinimalRuntime");
    let code = r#"
        const replay = require('bee:replay');
        replay.reset();

        replay.startRecording({ script: 'agent_task.js' });
        if (!replay.isRecording()) throw new Error('Should be in recording mode');

        // Execute 3 agent steps
        const step1 = replay.step('search_kb', { query: 'Beejs v1.6' }, () => {
            return { documents: ['doc1.md', 'doc2.md'] };
        });
        if (step1.documents.length !== 2) throw new Error('Step 1 output mismatch');

        const step2 = replay.step('synthesize_plan', { docCount: step1.documents.length }, {
            plan: 'Execute step A and B',
            confidence: 0.98
        });
        if (step2.confidence !== 0.98) throw new Error('Step 2 output mismatch');

        const step3 = replay.step('execute_action', { action: 'deploy' }, 'Success');
        if (step3 !== 'Success') throw new Error('Step 3 output mismatch');

        const stats = replay.getTraceStats();
        if (stats.step_count !== 3) throw new Error('Expected 3 steps, got ' + stats.step_count);

        const trace = replay.stopRecording();
        if (replay.isRecording()) throw new Error('Should not be recording after stop');
        if (trace.version !== '1.6.0') throw new Error('Trace version mismatch: ' + trace.version);
        if (trace.events.length !== 3) throw new Error('Trace events length mismatch: ' + trace.events.length);
        if (trace.events[0].payload.name !== 'search_kb') throw new Error('Event 0 name mismatch');
        if (trace.events[1].payload.name !== 'synthesize_plan') throw new Error('Event 1 name mismatch');

        JSON.stringify({ ok: true, eventCount: trace.events.length });
    "#;
    let res = runtime.execute_code(code).expect("Script execution failed");
    assert!(res.contains("\"ok\":true"));
    assert!(res.contains("\"eventCount\":3"));
}

#[test]
#[serial]
fn test_deterministic_offline_replay_and_step_playback() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create MinimalRuntime");
    let code = r#"
        const replay = require('bee:replay');
        replay.reset();

        const mockTrace = {
            version: '1.6.0',
            created_at: '2026-09-09T00:00:00Z',
            platform: 'darwin',
            arch: 'arm64',
            script: 'calc_agent.js',
            random_seed: 42,
            events: [
                {
                    type: 'agent_step',
                    payload: {
                        name: 'fetch_user',
                        input: { userId: 'u_100' },
                        output: { name: 'Alice', balance: 500 }
                    }
                },
                {
                    type: 'agent_step',
                    payload: {
                        name: 'deduct_balance',
                        input: { userId: 'u_100', amount: 50 },
                        output: { newBalance: 450, status: 'approved' }
                    }
                }
            ]
        };

        // Load trace for replay
        replay.loadTrace(mockTrace);
        if (!replay.isReplaying()) throw new Error('Should be in replaying mode');

        // Playback step 1: Provide dummy fallback, must return recorded output
        const user = replay.step('fetch_user', { userId: 'u_100' }, () => {
            throw new Error('This function must NOT be called during replay!');
        });
        if (user.name !== 'Alice' || user.balance !== 500) {
            throw new Error('Playback step 1 data mismatch: ' + JSON.stringify(user));
        }

        // Playback step 2
        const tx = replay.step('deduct_balance', { userId: 'u_100', amount: 50 }, null);
        if (tx.newBalance !== 450 || tx.status !== 'approved') {
            throw new Error('Playback step 2 data mismatch: ' + JSON.stringify(tx));
        }

        const stats = replay.getTraceStats();
        replay.reset();

        JSON.stringify({ replayed: true, current_cursor: stats.current_cursor });
    "#;
    let res = runtime.execute_code(code).expect("Script execution failed");
    assert!(res.contains("\"replayed\":true"));
}

#[test]
#[serial]
fn test_replay_divergence_detection_on_step_mismatch() {
    let mut runtime = MinimalRuntime::new().expect("Failed to create MinimalRuntime");
    let code = r#"
        const replay = require('bee:replay');
        replay.reset();

        const mockTrace = {
            version: '1.6.0',
            created_at: '2026-09-09T00:00:00Z',
            platform: 'darwin',
            arch: 'arm64',
            events: [
                {
                    type: 'agent_step',
                    payload: {
                        name: 'step_alpha',
                        input: { flag: true },
                        output: 'result_alpha'
                    }
                }
            ]
        };

        replay.loadTrace(mockTrace);

        let divergenceCaught = false;
        try {
            // Attempt to call a different step 'step_beta' instead of 'step_alpha'
            replay.step('step_beta', { flag: true }, null);
        } catch (e) {
            if (e.message.includes('ReplayDivergenceError') && e.message.includes('step_alpha')) {
                divergenceCaught = true;
            }
        }

        replay.reset();
        if (!divergenceCaught) throw new Error('Should have detected step divergence');
        'DIVERGENCE_CAUGHT';
    "#;
    let res = runtime.execute_code(code).expect("Script execution failed");
    assert!(res.contains("DIVERGENCE_CAUGHT"));
}
