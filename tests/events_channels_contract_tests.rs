// Pins the Stable events, abort, and channel contract in docs/EVENTS_CHANNELS_CONTRACT.md.

use amberjs::runtime_minimal::MinimalRuntime;

fn run_js(code: &str) -> Result<String, String> {
    let mut runtime = MinimalRuntime::new().map_err(|error| error.to_string())?;
    runtime
        .execute_code(code)
        .map(|value| value.trim().to_string())
        .map_err(|error| error.to_string())
}

#[test]
#[serial_test::serial]
fn event_target_dispatches_capture_then_bubble_and_honors_once() {
    let output = run_js(
        r#"
        const target = new EventTarget();
        const seen = [];
        function capture() { seen.push('capture'); }
        function bubble() { seen.push('bubble'); }
        target.addEventListener('ping', capture, true);
        target.addEventListener('ping', bubble);
        target.addEventListener('ping', bubble);
        const once = () => seen.push('once');
        target.addEventListener('ping', once, { once: true });
        const event = new Event('ping', { bubbles: true, cancelable: true, composed: true });
        const phase = [];
        target.addEventListener('ping', (ev) => {
            phase.push(ev.eventPhase + ':' + (ev.currentTarget === target) + ':' + (ev.composedPath()[0] === target));
        });
        const first = target.dispatchEvent(event);
        target.dispatchEvent(new Event('ping'));
        const after = event.eventPhase + ':' + (event.currentTarget === null) + ':' + event.composedPath().length;
        [
            event instanceof Event,
            event.bubbles, event.cancelable, event.composed, event.isTrusted,
            typeof event.timeStamp,
            Event.AT_TARGET,
            first,
            seen.join(','),
            phase[0],
            after
        ].join('|');
        "#,
    )
    .expect("event dispatch");
    assert_eq!(
        output,
            "true|true|true|true|false|number|2|true|capture,bubble,once,capture,bubble|2:true:true|0:true:0",
        "got {output}"
    );
}

#[test]
#[serial_test::serial]
fn event_stop_immediate_prevent_default_and_signal_removal() {
    let output = run_js(
        r#"
        const target = new EventTarget();
        const seen = [];
        target.addEventListener('submit', () => seen.push('first'));
        target.addEventListener('submit', (event) => {
            event.stopImmediatePropagation();
            event.preventDefault();
            seen.push('stop');
        });
        target.addEventListener('submit', () => seen.push('skipped'));
        const cancelable = new Event('submit', { cancelable: true });
        const result = target.dispatchEvent(cancelable);

        const passiveTarget = new EventTarget();
        const passive = new Event('submit', { cancelable: true });
        passiveTarget.addEventListener('submit', (event) => event.preventDefault(), { passive: true });
        passiveTarget.dispatchEvent(passive);

        let calls = 0;
        const controller = new AbortController();
        const listener = () => { calls += 1; };
        target.addEventListener('tick', listener, { signal: controller.signal });
        target.dispatchEvent(new Event('tick'));
        controller.abort();
        target.dispatchEvent(new Event('tick'));

        const handleTarget = new EventTarget();
        const handle = { handleEvent() { seen.push('handle'); } };
        handleTarget.addEventListener('handle', handle);
        handleTarget.dispatchEvent(new Event('handle'));

        const nested = new EventTarget();
        let reentered = false;
        const ev = new Event('loop');
        nested.addEventListener('loop', () => {
            try {
                nested.dispatchEvent(ev);
            } catch (error) {
                reentered = error.name === 'InvalidStateError';
            }
        });
        nested.dispatchEvent(ev);
        let missing = false;
        try { new Event(); } catch (error) { missing = error instanceof TypeError; }

        [
            seen.join(','),
            result,
            cancelable.defaultPrevented,
            passive.defaultPrevented,
            calls,
            reentered,
            missing
        ].join('|');
        "#,
    )
    .expect("event controls");
    assert_eq!(
        output, "first,stop,handle|false|true|false|1|true|true",
        "got {output}"
    );
}

#[test]
#[serial_test::serial]
fn custom_event_detail_inherits_event() {
    let output = run_js(
        r#"
        const payload = { n: 1 };
        const event = new CustomEvent('ready', { detail: payload, composed: true, cancelable: true });
        event.preventDefault();
        let missing = false;
        try { new CustomEvent(); } catch (error) { missing = error instanceof TypeError; }
        const plain = new CustomEvent('plain', { foo: 1 });
        [
            event instanceof CustomEvent,
            event instanceof Event,
            event.detail === payload,
            event.composed,
            event.defaultPrevented,
            plain.detail === null,
            missing
        ].join('|');
        "#,
    )
    .expect("custom event");
    assert_eq!(output, "true|true|true|true|true|true|true", "got {output}");
}

#[test]
#[serial_test::serial]
fn abort_reason_event_any_and_timeout() {
    let output = run_js(
        r#"
        const controller = new AbortController();
        const reasons = [];
        let eventType = '';
        let eventTargetIsSignal = false;
        controller.signal.addEventListener('abort', (event) => {
            eventType = event.type;
            eventTargetIsSignal = event.target === controller.signal;
            reasons.push(controller.signal.reason && controller.signal.reason.name);
        });
        controller.signal.onabort = () => reasons.push('onabort');
        controller.abort();
        controller.abort('second');
        let threw = false;
        try { controller.signal.throwIfAborted(); } catch (error) {
            threw = error.name === 'AbortError' && error.message === 'The operation was aborted';
        }
        const already = AbortSignal.abort('because');
        let late = 0;
        already.addEventListener('abort', () => { late += 1; });
        const followed = AbortSignal.any([controller.signal, new AbortController().signal]);
        const pending = new AbortController();
        const composite = AbortSignal.any([pending.signal]);
        pending.abort('later');
        let badTimeout = false;
        try { AbortSignal.timeout(-1); } catch (error) { badTimeout = error instanceof TypeError; }
        let illegal = false;
        try { new AbortSignal(); } catch (error) { illegal = error instanceof TypeError; }
        new Promise((resolve) => {
            const signal = AbortSignal.timeout(20);
            signal.addEventListener('abort', () => {
                resolve([
                    controller.signal instanceof AbortSignal,
                    controller.signal.aborted,
                    reasons.join(','),
                    eventType,
                    eventTargetIsSignal,
                    threw,
                    controller.signal.reason.name,
                    already.aborted,
                    already.reason,
                    late,
                    followed.aborted,
                    followed.reason && followed.reason.name,
                    composite.reason,
                    badTimeout,
                    illegal,
                    signal.reason && signal.reason.name,
                    signal.reason && signal.reason.message
                ].join('|'));
            });
        });
        "#,
    )
    .expect("abort surface");
    assert_eq!(
        output,
        "true|true|AbortError,onabort|abort|true|true|AbortError|true|because|1|true|AbortError|later|true|true|TimeoutError|The operation was aborted due to timeout",
        "got {output}"
    );
}

#[test]
#[serial_test::serial]
fn message_channel_queues_clones_transfers_and_closes() {
    let output = run_js(
        r#"
        const channel = new MessageChannel();
        const queued = [];
        channel.port2.addEventListener('message', (event) => queued.push(event.data));
        channel.port1.postMessage({ n: 1, bytes: new Uint8Array([4, 5]) });
        const beforeStart = queued.length;
        channel.port2.start();
        const payload = { nested: { n: 1 } };
        let received = null;
        channel.port2.onmessage = (event) => { received = event.data; };
        channel.port1.postMessage(payload);
        received.nested.n = 9;

        const buffer = new ArrayBuffer(4);
        new Uint8Array(buffer).set([1, 2, 3, 4]);
        let moved = null;
        channel.port2.onmessage = (event) => { moved = event.data; };
        channel.port1.postMessage(buffer, [buffer]);
        const movedBytes = new Uint8Array(moved);

        const extra = new MessageChannel();
        let portEvent = null;
        extra.port2.start();
        extra.port2.addEventListener('message', (event) => { portEvent = event; });
        extra.port1.postMessage('go', [channel.port1]);
        let transferredThrows = false;
        try { channel.port1.postMessage('nope'); } catch (error) {
            transferredThrows = error.name === 'InvalidStateError';
        }
        let reply = null;
        channel.port2.addEventListener('message', (event) => { reply = event.data; });
        portEvent.ports[0].postMessage('back');

        const closed = new MessageChannel();
        closed.port1.close();
        let closeThrows = false;
        try { closed.port2.postMessage('x'); } catch (error) {
            closeThrows = error.name === 'InvalidStateError';
        }
        let cloneThrows = false;
        const unclone = new MessageChannel();
        unclone.port2.start();
        let delivered = false;
        unclone.port2.onmessage = () => { delivered = true; };
        try { unclone.port1.postMessage(() => {}); } catch (error) {
            cloneThrows = error.name === 'DataCloneError';
        }

        [
            channel instanceof MessageChannel,
            channel.port1 instanceof MessagePort,
            beforeStart,
            queued[0] && queued[0].n,
            received !== payload && payload.nested.n === 1,
            buffer.byteLength,
            movedBytes[0] + ',' + movedBytes[3],
            portEvent && portEvent.data,
            portEvent && portEvent.ports.length,
            transferredThrows,
            reply,
            closeThrows,
            cloneThrows,
            delivered
        ].join('|');
        "#,
    )
    .expect("message channel");
    assert_eq!(
        output, "true|true|0|1|true|0|1,4|go|1|true|back|true|true|false",
        "got {output}"
    );
}

#[test]
#[serial_test::serial]
fn broadcast_channel_clones_to_peers_and_closes() {
    let output = run_js(
        r#"
        const sender = new BroadcastChannel('room');
        const peer = new BroadcastChannel('room');
        const other = new BroadcastChannel('other');
        const payload = { items: ['a'], bytes: new Uint8Array([7]) };
        let senderData = 'none';
        let peerData = null;
        let otherHit = false;
        sender.onmessage = (event) => { senderData = event.data; };
        peer.addEventListener('message', (event) => { peerData = event.data; });
        other.onmessage = () => { otherHit = true; };
        sender.postMessage(payload);
        peerData.items.push('peer');
        peerData.bytes[0] = 1;
        const listener = () => {};
        peer.addEventListener('message', listener);
        peer.removeEventListener('message', listener);
        peer.close();
        let closed = false;
        try { peer.postMessage('after'); } catch (error) {
            closed = error.name === 'InvalidStateError';
        }
        let missing = false;
        try { new BroadcastChannel(); } catch (error) { missing = error instanceof TypeError; }
        let cloneThrows = false;
        const again = new BroadcastChannel('room');
        try { sender.postMessage({ fn() {} }); } catch (error) {
            cloneThrows = error.name === 'DataCloneError';
        }
        [
            sender instanceof BroadcastChannel,
            sender.name,
            senderData,
            peerData !== payload && payload.items.length === 1 && payload.bytes[0] === 7,
            peerData.items[0],
            otherHit,
            closed,
            missing,
            cloneThrows,
            again.name === 'room'
        ].join('|');
        "#,
    )
    .expect("broadcast channel");
    assert_eq!(
        output, "true|room|none|true|a|false|true|true|true|true",
        "got {output}"
    );
}
