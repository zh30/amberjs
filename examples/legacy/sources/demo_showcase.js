// Amber High-Performance JavaScript Runtime Showcase
// This demonstrates the core features of Amber

console.log("🐝 Amber Runtime Showcase");
console.log("=".repeat(50));

// 1. Basic JavaScript Execution
console.log("\n1. Basic JavaScript Execution:");
const numbers = [1, 2, 3, 4, 5];
const doubled = numbers.map(n => n * 2);
console.log("   Doubled:", doubled);

// 2. Performance API (AI workload optimized)
console.log("\n2. Performance API:");
const start = performance.now();
let sum = 0;
for (let i = 0; i < 1000000; i++) {
    sum += i;
}
const end = performance.now();
console.log(`   Sum of 1M numbers: ${sum}`);
console.log(`   Time: ${(end - start).toFixed(2)}ms`);

// 3. Node.js APIs
console.log("\n3. Node.js APIs:");
console.log(`   process.version: ${process.version}`);
console.log(`   process.platform: ${process.platform}`);
console.log(`   process.cwd: ${process.cwd()}`);
console.log(`   process.arch: ${process.arch}`);

// 4. Event Loop (nextTick, microtasks, timers)
console.log("\n4. Event Loop:");
process.nextTick(() => {
    console.log("   [nextTick] Executed first");
});

Promise.resolve().then(() => {
    console.log("   [microtask] Executed second");
});

setTimeout(() => {
    console.log("   [timer] Executed fourth (after microtasks)");
}, 0);

setImmediate(() => {
    console.log("   [setImmediate] Executed after I/O");
});

// 5. Console APIs
console.log("\n5. Console APIs:");
console.log("   Standard console.log");
console.count("counter");
console.count("counter");
console.countReset("counter");
console.count("counter");

// 6. Web Streams API (v0.3.275)
console.log("\n6. Web Streams API:");
const readable = new ReadableStream({
    start(controller) {
        controller.enqueue("Hello ");
        controller.enqueue("Amber!");
        controller.close();
    }
});

// 7. Structured Clone (v0.3.299+)
console.log("\n7. Structured Clone:");
const original = {
    name: "test",
    data: new Uint8Array([1, 2, 3, 4, 5]),
    nested: { value: 42 }
};
const cloned = structuredClone(original);
console.log("   Original:", JSON.stringify(original));
console.log("   Cloned:", JSON.stringify(cloned));
console.log("   Deep equal:", JSON.stringify(original) === JSON.stringify(cloned));

// 8. SharedArrayBuffer (v0.3.322)
console.log("\n8. SharedArrayBuffer:");
const sab = new SharedArrayBuffer(1024);
console.log("   SharedArrayBuffer created:", sab.byteLength, "bytes");

// 9. ArrayBuffer Transfer (v0.3.311 - requires V8 >= 12.0)
console.log("\n9. ArrayBuffer Transfer:");
if (typeof ArrayBuffer.prototype.transfer !== 'undefined') {
    const buffer = new ArrayBuffer(100);
    const view = new Uint8Array(buffer);
    view[0] = 42;
    const transferred = buffer.transfer();
    console.log("   Original byteLength:", buffer.byteLength);
    console.log("   Transferred byteLength:", transferred.byteLength);
} else {
    console.log("   ArrayBuffer.transfer requires V8 >= 12.0");
    console.log("   Available:", typeof ArrayBuffer.prototype.transfer);
}

// 10. Timer APIs with unref/ref
console.log("\n10. Timer APIs:");
const timer = setTimeout(() => {
    console.log("   Timer fired!");
}, 100);
console.log("   Timer created with ID:", timer);
timer.unref();
console.log("   Timer unreferenced (won't keep process alive)");

console.log("\n" + "=".repeat(50));
console.log("✅ Amber Runtime Showcase Complete!");
console.log("\nAmber features implemented:");
console.log("  - V8 JavaScript Engine");
console.log("  - Event Loop (nextTick -> microtasks -> timers -> setImmediate)");
console.log("  - Node.js APIs (buffer, child_process, crypto, dns, events, fs, http, net, os, path, stream, timers, url, util)");
console.log("  - Web APIs (crypto, events, abort, blob, encoding, performance, url, fetch, websocket, streams)");
console.log("  - Advanced: BroadcastChannel, MessageChannel, Worker, SharedArrayBuffer, ServiceWorker, Cache");
console.log("  - Package Manager (npm compatible)");
console.log("  - CLI: run, eval, repl, test, bundle, debug, serve, init, add, remove, install, prune, create, bunx");
