// AI Workload Demo for Amber Runtime
// Demonstrates streaming LLM responses using Web Streams API
// Run with: ./target/release/amber run examples/ai_workload_demo.js

console.log("Amber AI Workload Demo");
console.log("======================\n");

// Performance measurement for AI workloads
const startTime = performance.now();

// Demo 1: Basic ReadableStream
console.log("Demo 1: ReadableStream Basic");
console.log("----------------------------");

const stream1 = new ReadableStream({
    start(controller) {
        const data = ["Hello", ", ", "AI", " ", "World", "!"];
        data.forEach(chunk => controller.enqueue(chunk));
        controller.close();
    }
});

const reader1 = stream1.getReader();
let count = 0;

function readStream1() {
    reader1.read().then(function(result) {
        if (result.done) {
            console.log("\n  Read " + count + " chunks");
            console.log();
            runDemo2();
        } else {
            count++;
            process.stdout.write(result.value);
            readStream1();
        }
    });
}

readStream1();

function runDemo2() {
    // Demo 2: Performance measurement
    console.log("Demo 2: AI Workload Performance");
    console.log("--------------------------------");

    performance.mark('workload_start');

    // Simulate AI computation
    const iterations = 5000;
    let result = 0;
    for (let i = 0; i < iterations; i++) {
        result += Math.sin(i * 0.01) * Math.cos(i * 0.02);
    }

    performance.mark('workload_end');
    const measure = performance.measure('ai_work', 'workload_start', 'workload_end');

    console.log("  Computations: " + iterations + " iterations");
    console.log("  Time: " + measure.duration.toFixed(2) + "ms");
    console.log("  Throughput: " + (iterations / measure.duration * 1000).toFixed(0) + " ops/sec");
    console.log();

    performance.clearMarks();
    performance.clearMeasures();

    runDemo3();
}

function runDemo3() {
    // Demo 3: TextEncoder/Decoder
    console.log("Demo 3: TextEncoder/Decoder");
    console.log("----------------------------");

    const text = "AI streaming with Amber!";
    const encoded = new TextEncoder().encode(text);
    const decoded = new TextDecoder().decode(encoded);

    console.log('  Original: "' + text + '"');
    console.log("  Encoded: " + encoded.length + " bytes");
    console.log('  Decoded: "' + decoded + '"');
    console.log();

    finishDemo();
}

function finishDemo() {
    const totalTime = performance.now() - startTime;
    console.log("======================");
    console.log("Demo completed in " + totalTime.toFixed(2) + "ms");
    console.log("======================");
    console.log("\nAmber - High-performance runtime for AI workloads!");
}
