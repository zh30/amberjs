/**
 * Beejs-native AI tensor operator benchmarks (bee:ai.Tensor).
 * Skipped on runtimes without bee:ai.
 */

const WARMUP = 1;
const SAMPLES = 5;

async function runBench(name, fn) {
    try {
        for (let i = 0; i < WARMUP; i++) fn();
        const times = [];
        for (let i = 0; i < SAMPLES; i++) {
            const start = performance.now();
            fn();
            times.push(performance.now() - start);
        }
        const avg = times.reduce((a, b) => a + b, 0) / times.length;
        return { name, avgMs: avg, minMs: Math.min(...times), maxMs: Math.max(...times), opsSec: 1000 / avg };
    } catch (err) {
        return {
            name,
            skipped: true,
            error: String((err && err.message) || err),
            avgMs: 0,
            minMs: 0,
            maxMs: 0,
            opsSec: 0,
        };
    }
}

function loadTensor() {
    const ai = require('bee:ai');
    if (!ai || typeof ai.Tensor !== 'function') throw new Error('bee:ai.Tensor unavailable');
    return ai.Tensor;
}

function fill(n, seed) {
    const data = new Float32Array(n);
    for (let i = 0; i < n; i++) data[i] = ((i * 17 + seed) % 1000) / 1000;
    return data;
}

function jsMatmul(a, b, n) {
    const c = new Float32Array(n * n);
    for (let i = 0; i < n; i++) {
        for (let k = 0; k < n; k++) {
            const aik = a[i * n + k];
            for (let j = 0; j < n; j++) c[i * n + j] += aik * b[k * n + j];
        }
    }
    return c[0];
}

function jsDot(a, b) {
    let acc = 0;
    for (let i = 0; i < a.length; i++) acc += a[i] * b[i];
    return acc;
}

function jsSoftmax(a) {
    let max = -Infinity;
    for (let i = 0; i < a.length; i++) if (a[i] > max) max = a[i];
    const out = new Float32Array(a.length);
    let sum = 0;
    for (let i = 0; i < a.length; i++) {
        out[i] = Math.exp(a[i] - max);
        sum += out[i];
    }
    for (let i = 0; i < a.length; i++) out[i] /= sum;
    return out[0];
}

(async () => {
    let Tensor;
    try {
        Tensor = loadTensor();
    } catch (err) {
        const skipped = [
            '28. Tensor / matmul 64x64 (native)',
            '29. Tensor / matmul 64x64 (pure JS)',
            '30. Tensor / dot 16k (native)',
            '31. Tensor / softmax 4k (native)',
            '32. Tensor / cosineSimilarity 1k (native)',
        ].map((name) => ({
            name,
            skipped: true,
            error: String((err && err.message) || err),
            avgMs: 0,
            minMs: 0,
            maxMs: 0,
            opsSec: 0,
        }));
        console.log('bee:ai unavailable, skipping tensor suite');
        console.log('\nSummary JSON:');
        console.log(JSON.stringify(skipped, null, 2));
        return;
    }

    const n = 64;
    const a64 = fill(n * n, 1);
    const b64 = fill(n * n, 2);
    const ta = new Tensor(a64, [n, n], 'float32');
    const tb = new Tensor(b64, [n, n], 'float32');

    const v16kA = fill(16384, 3);
    const v16kB = fill(16384, 4);
    const tvA = new Tensor(v16kA, [16384], 'float32');
    const tvB = new Tensor(v16kB, [16384], 'float32');

    const s4k = fill(4096, 5);
    const ts = new Tensor(s4k, [4096], 'float32');

    const c1kA = fill(1024, 6);
    const c1kB = fill(1024, 7);
    const tcA = new Tensor(c1kA, [1024], 'float32');
    const tcB = new Tensor(c1kB, [1024], 'float32');

    const benches = [
        { name: '28. Tensor / matmul 64x64 (native)', fn: () => ta.matmul(tb) },
        { name: '29. Tensor / matmul 64x64 (pure JS)', fn: () => jsMatmul(a64, b64, n) },
        { name: '30. Tensor / dot 16k (native)', fn: () => tvA.dot(tvB) },
        { name: '31. Tensor / softmax 4k (native)', fn: () => ts.softmax() },
        { name: '32. Tensor / cosineSimilarity 1k (native)', fn: () => tcA.cosineSimilarity(tcB) },
        { name: '33. Tensor / dot 16k (pure JS)', fn: () => jsDot(v16kA, v16kB) },
        { name: '34. Tensor / softmax 4k (pure JS)', fn: () => jsSoftmax(s4k) },
    ];

    console.log('Starting AI Tensor Suite (' + benches.length + ' workloads)...\n');
    const results = [];
    for (const b of benches) {
        const res = await runBench(b.name, b.fn);
        results.push(res);
        if (res.skipped) {
            console.log(`${res.name.padEnd(52)}: SKIPPED  (${res.error})`);
        } else {
            console.log(
                `${res.name.padEnd(52)}: ${res.avgMs.toFixed(2).padStart(8)} ms  (${res.opsSec.toFixed(1).padStart(7)} ops/s)`
            );
        }
    }
    console.log('\nSummary JSON:');
    console.log(JSON.stringify(results, null, 2));
})();
