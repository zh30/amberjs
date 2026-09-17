use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_bus_topic_wildcards_and_pubsub() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const { createBus, topicMatches } = require('amber:bus');

    // 1. Test topic pattern matching helper
    if (!topicMatches('agent.*.task', 'agent.planner.task')) throw new Error('Wildcard * failed');
    if (topicMatches('agent.*.task', 'agent.planner.sub.task')) throw new Error('Wildcard * matched multiple segments');
    if (!topicMatches('agent.#', 'agent.planner.sub.task')) throw new Error('Wildcard # failed');

    // 2. PubSub with single & multi-wildcard subscriptions
    const bus = createBus();
    const received = [];

    // Exact subscriber
    bus.subscribe('agent.planner.task', (msg) => {
        received.push({ sub: 'exact', topic: msg.topic, payload: msg.payload });
    });

    // Single-wildcard subscriber
    bus.subscribe('agent.*.task', (msg) => {
        received.push({ sub: 'star', topic: msg.topic, payload: msg.payload });
    });

    // Multi-wildcard subscriber
    bus.subscribe('agent.#', (msg) => {
        received.push({ sub: 'hash', topic: msg.topic, payload: msg.payload });
    });

    // Publish to planner task
    bus.publish('agent.planner.task', { action: 'plan' });

    // Publish to critic task
    bus.publish('agent.critic.task', { action: 'review' });

    // Publish to deep nested topic
    bus.publish('agent.critic.deep.result', { score: 95 });

    const exactCount = received.filter(r => r.sub === 'exact').length;
    const starCount = received.filter(r => r.sub === 'star').length;
    const hashCount = received.filter(r => r.sub === 'hash').length;

    if (exactCount !== 1) throw new Error('Exact count mismatch: ' + exactCount);
    if (starCount !== 2) throw new Error('Star count mismatch: ' + starCount);
    if (hashCount !== 3) throw new Error('Hash count mismatch: ' + hashCount);

    JSON.stringify({ success: true, exactCount, starCount, hashCount });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_bus_request_reply_pattern() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    (async () => {
        const bus = require('amber:bus');

        // Subscriber listening for calculation requests
        bus.subscribe('service.calculator.add', (msg) => {
            const { a, b } = msg.payload;
            bus.reply(msg, { result: a + b });
        });

        // Request with response expectation
        const response = await bus.request('service.calculator.add', { a: 25, b: 17 });
        if (!response || response.result !== 42) {
            throw new Error('Request reply mismatch: ' + JSON.stringify(response));
        }

        return JSON.stringify({ success: true, result: response.result });
    })()
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_bus_priority_and_dead_letter_queue() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const { createBus } = require('amber:bus');
    const bus = createBus();

    const order = [];

    // Low priority listener
    bus.subscribe('task.run', () => {
        order.push('low');
    }, { priority: 1 });

    // High priority listener
    bus.subscribe('task.run', () => {
        order.push('high');
    }, { priority: 10 });

    // Normal priority listener
    bus.subscribe('task.run', () => {
        order.push('normal');
    }, { priority: 5 });

    bus.publish('task.run', { id: 1 });

    if (order.join(',') !== 'high,normal,low') {
        throw new Error('Priority dispatch order failed: ' + order.join(','));
    }

    // Dead letter test: publish to topic with no subscribers
    bus.publish('unhandled.topic', { test: true });

    const deadLetters = bus.getDeadLetters();
    if (deadLetters.length === 0) throw new Error('Expected dead letter message');
    if (deadLetters[deadLetters.length - 1].topic !== 'unhandled.topic') {
        throw new Error('Dead letter topic mismatch: ' + deadLetters[0].topic);
    }

    const metrics = bus.getMetrics();
    if (metrics.dead_letter_count < 1) throw new Error('Dead letter count metric missing');

    JSON.stringify({ success: true, order: order.join(','), deadLettersCount: deadLetters.length });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_bus_middleware_pipeline() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const { createBus } = require('amber:bus');
    const bus = createBus();

    // Middleware: inject tracing header
    bus.use((msg) => {
        msg.headers = msg.headers || {};
        msg.headers['x-trace-id'] = 'trace_12345';
    });

    let receivedTrace = null;
    bus.subscribe('audit.event', (msg) => {
        receivedTrace = msg.headers['x-trace-id'];
    });

    bus.publish('audit.event', { action: 'login' });

    if (receivedTrace !== 'trace_12345') {
        throw new Error('Middleware header propagation failed: ' + receivedTrace);
    }

    JSON.stringify({ success: true, receivedTrace });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}
