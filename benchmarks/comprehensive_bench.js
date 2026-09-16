/**
 * Comprehensive JavaScript Runtime Benchmark Suite 2.0
 * 24 in-process workloads, identical across Beejs, Node.js, and Bun.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const EventEmitter = require('events');

const WARMUP_ITERS = 2;
const BENCH_ITERS = 5;

async function runBench(name, fn) {
    try {
        for (let i = 0; i < WARMUP_ITERS; i++) {
            await fn();
        }
        const times = [];
        for (let i = 0; i < BENCH_ITERS; i++) {
            const start = performance.now();
            await fn();
            times.push(performance.now() - start);
        }
        const avg = times.reduce((a, b) => a + b, 0) / times.length;
        return {
            name,
            avgMs: avg,
            minMs: Math.min(...times),
            maxMs: Math.max(...times),
            opsSec: 1000 / avg,
        };
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

function benchFibonacci() {
    function fib(n) {
        if (n <= 1) return n;
        let a = 0, b = 1;
        for (let i = 2; i <= n; i++) {
            const temp = a + b;
            a = b;
            b = temp;
        }
        return b;
    }
    let sum = 0;
    for (let i = 0; i < 500000; i++) sum += fib(40);
    return sum;
}

function benchPrimes() {
    const max = 100000;
    const flags = new Uint8Array(max + 1);
    let count = 0;
    for (let i = 2; i <= max; i++) {
        if (!flags[i]) {
            count++;
            for (let j = i * 2; j <= max; j += i) flags[j] = 1;
        }
    }
    return count;
}

function benchMatrixMultiply() {
    const N = 80;
    const A = new Float64Array(N * N);
    const B = new Float64Array(N * N);
    const C = new Float64Array(N * N);
    for (let i = 0; i < N * N; i++) {
        A[i] = i * 0.1;
        B[i] = i * 0.2;
    }
    for (let i = 0; i < N; i++) {
        for (let k = 0; k < N; k++) {
            const aik = A[i * N + k];
            for (let j = 0; j < N; j++) C[i * N + j] += aik * B[k * N + j];
        }
    }
    return C[0];
}

function benchObjectCreation() {
    const arr = [];
    for (let i = 0; i < 50000; i++) {
        arr.push({
            id: i,
            name: `user_${i}`,
            active: i % 2 === 0,
            meta: { role: 'admin', score: i * 1.5 },
            tags: ['tag1', 'tag2'],
        });
    }
    let totalScore = 0;
    for (let i = 0; i < arr.length; i++) totalScore += arr[i].meta.score;
    return totalScore;
}

function benchArrayTransforms() {
    const data = [];
    for (let i = 0; i < 20000; i++) data.push(i);
    return data.filter((x) => x % 2 === 0).map((x) => x * 3).reduce((acc, x) => acc ^ x, 0);
}

function benchJsonSerialization() {
    const obj = { title: 'Benchmark Dataset', count: 1000, items: [] };
    for (let i = 0; i < 200; i++) {
        obj.items.push({
            id: i,
            title: `Item number ${i}`,
            price: 19.99 + i * 0.5,
            inStock: i % 3 !== 0,
            attributes: { color: 'blue', size: 'M', weight: 1.2 },
        });
    }
    let totalLen = 0;
    for (let i = 0; i < 50; i++) {
        const json = JSON.stringify(obj);
        totalLen += json.length;
        try {
            totalLen += JSON.parse(json).items.length;
        } catch (err) {
            throw new Error(`JSON roundtrip failed: ${err.message}`);
        }
    }
    return totalLen;
}

function benchStringRegex() {
    const text =
        'The quick brown fox jumps over the lazy dog. HTTP/1.1 200 OK. Content-Type: application/json; charset=utf-8\r\n\r\n';
    let count = 0;
    const regex = /[a-zA-Z0-9_-]+:\s*[^\r\n]+/g;
    for (let i = 0; i < 10000; i++) {
        const matches = text.match(regex);
        if (matches) count += matches.length;
        count += text.replace(/quick/, 'slow').replace(/brown/, 'red').length;
    }
    return count;
}

function benchBufferOperations() {
    let total = 0;
    for (let i = 0; i < 1000; i++) {
        const buf = Buffer.alloc(16 * 1024);
        buf.fill(0xaa);
        total += buf.slice(100, 500)[0];
        total += Buffer.from(`hello world from buffer benchmark ${i}`).length;
    }
    return total;
}

function benchCryptoSha256() {
    const data = Buffer.alloc(16 * 1024, 'a');
    let hashLen = 0;
    for (let i = 0; i < 500; i++) {
        hashLen += crypto.createHash('sha256').update(data).digest('hex').length;
    }
    return hashLen;
}

function benchCryptoRandomBytes() {
    let totalLen = 0;
    for (let i = 0; i < 500; i++) totalLen += crypto.randomBytes(1024).length;
    return totalLen;
}

function benchEventEmitter() {
    const ee = new EventEmitter();
    let counter = 0;
    const handler = (val) => {
        counter += val;
    };
    ee.on('event', handler);
    for (let i = 0; i < 50000; i++) ee.emit('event', 1);
    ee.removeListener('event', handler);
    return counter;
}

function benchFsSync() {
    const tmpFile = path.join('/tmp', `bench_io_${process.pid}.tmp`);
    const content = 'X'.repeat(32 * 1024);
    for (let i = 0; i < 100; i++) {
        fs.writeFileSync(tmpFile, content);
        const readBack = fs.readFileSync(tmpFile);
        if (readBack.length !== content.length) throw new Error('I/O mismatch');
    }
    try {
        fs.unlinkSync(tmpFile);
    } catch {
        /* best-effort cleanup */
    }
}

function benchMapSet() {
    const map = new Map();
    const set = new Set();
    for (let i = 0; i < 50000; i++) {
        map.set(`k${i}`, i);
        set.add(i);
    }
    let hits = 0;
    for (let i = 0; i < 50000; i++) {
        if (map.has(`k${i}`)) hits += map.get(`k${i}`);
        if (set.has(i)) hits += 1;
    }
    return hits;
}

function benchTypedArray() {
    const n = 1000000;
    const a = new Float64Array(n);
    for (let i = 0; i < n; i++) a[i] = i * 0.001;
    const b = a.slice();
    let acc = 0;
    for (let i = 0; i < n; i++) acc += a[i] * b[i];
    return acc;
}

function benchTextCodec() {
    const encoder = new TextEncoder();
    const decoder = new TextDecoder();
    const sample = `测🐝试 UTF-8 payload ${'x'.repeat(1024)}`;
    let total = 0;
    for (let i = 0; i < 200; i++) {
        const bytes = encoder.encode(sample.repeat(64));
        total += bytes.byteLength;
        total += decoder.decode(bytes).length;
    }
    return total;
}

function benchUrlParse() {
    let total = 0;
    for (let i = 0; i < 20000; i++) {
        let url;
        try {
            url = new URL(`https://example.com/api/v1/items/${i}?q=bench&page=${i % 50}&sort=desc#frag`);
        } catch (err) {
            throw new Error(`URL parse failed: ${err.message}`);
        }
        const params = new URLSearchParams(url.search);
        params.set('extra', String(i));
        total += url.pathname.length + params.toString().length + url.hash.length;
    }
    return total;
}

function benchStructuredClone() {
    const tree = {
        id: 1,
        nested: { list: [1, 2, 3, { deep: true }], map: { a: 1, b: 2 } },
        tags: ['alpha', 'beta', 'gamma'],
        when: new Date(),
    };
    let copies = 0;
    for (let i = 0; i < 2000; i++) {
        const cloned = structuredClone(tree);
        copies += cloned.nested.list.length + cloned.tags.length;
    }
    return copies;
}

async function benchPromiseMicrotasks() {
    const n = 20000;
    const tasks = new Array(n);
    for (let i = 0; i < n; i++) tasks[i] = Promise.resolve(i);
    const values = await Promise.all(tasks);
    return values[n - 1];
}

async function benchWebCryptoSha256() {
    if (!globalThis.crypto || !crypto.subtle || typeof crypto.subtle.digest !== 'function') {
        throw new Error('crypto.subtle.digest unavailable');
    }
    const data = new Uint8Array(16 * 1024);
    data.fill(97);
    let total = 0;
    for (let i = 0; i < 100; i++) {
        const digest = await crypto.subtle.digest('SHA-256', data);
        total += digest.byteLength;
    }
    return total;
}

async function benchReadableStream() {
    if (typeof ReadableStream !== 'function') throw new Error('ReadableStream unavailable');
    const chunks = 5000;
    const stream = new ReadableStream({
        start(controller) {
            for (let i = 0; i < chunks; i++) controller.enqueue(Uint8Array.of(i & 0xff));
            controller.close();
        },
    });
    const reader = stream.getReader();
    let total = 0;
    for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        total += value.length;
    }
    return total;
}

function benchWasmAddCalls() {
    const bytes = new Uint8Array([
        0, 97, 115, 109, 1, 0, 0, 0, 1, 7, 1, 96, 2, 127, 127, 1, 127, 3, 2, 1, 0, 7, 7, 1, 3, 97, 100, 100, 0, 0, 10,
        9, 1, 7, 0, 32, 0, 32, 1, 106, 11,
    ]);
    const inst = new WebAssembly.Instance(new WebAssembly.Module(bytes));
    const add = inst.exports.add;
    let acc = 0;
    for (let i = 0; i < 100000; i++) acc += add(i, 1);
    return acc;
}

function benchDateFormat() {
    let total = 0;
    for (let i = 0; i < 50000; i++) {
        const d = new Date(1700000000000 + i * 1000);
        total += d.getTime() + d.toISOString().length;
    }
    return total;
}

async function benchFsPromises() {
    const tmpFile = path.join('/tmp', `bench_fs_p_${process.pid}.tmp`);
    const content = 'Y'.repeat(32 * 1024);
    const promises = fs.promises;
    if (!promises || typeof promises.writeFile !== 'function') throw new Error('fs.promises unavailable');
    for (let i = 0; i < 50; i++) {
        await promises.writeFile(tmpFile, content);
        const readBack = await promises.readFile(tmpFile);
        if (readBack.length !== content.length) throw new Error('async I/O mismatch');
    }
    try {
        await promises.unlink(tmpFile);
    } catch {
        /* best-effort cleanup */
    }
}

async function benchCompressionStream() {
    if (typeof CompressionStream !== 'function') throw new Error('CompressionStream unavailable');
    const payload = new Uint8Array(32 * 1024);
    payload.fill(65);
    let total = 0;
    for (let i = 0; i < 20; i++) {
        const input = new ReadableStream({
            start(controller) {
                controller.enqueue(payload);
                controller.close();
            },
        });
        const compressed = input.pipeThrough(new CompressionStream('gzip'));
        const buf = await new Response(compressed).arrayBuffer();
        total += buf.byteLength;
    }
    return total;
}

const benchmarks = [
    { name: '1. JIT / Fibonacci (500k ops)', fn: benchFibonacci },
    { name: '2. JIT / Primes Sieve (100k)', fn: benchPrimes },
    { name: '3. JIT / Matrix Multiply (80x80)', fn: benchMatrixMultiply },
    { name: '4. Objects / Alloc & Property Access (50k)', fn: benchObjectCreation },
    { name: '5. Arrays / Filter-Map-Reduce (20k)', fn: benchArrayTransforms },
    { name: '6. JSON / Stringify & Parse (50 iters)', fn: benchJsonSerialization },
    { name: '7. String & RegExp (10k iters)', fn: benchStringRegex },
    { name: '8. Buffer / Alloc, Fill, Slice (1k x 16KB)', fn: benchBufferOperations },
    { name: '9. Crypto / SHA-256 (500 x 16KB)', fn: benchCryptoSha256 },
    { name: '10. Crypto / randomBytes (500 x 1KB)', fn: benchCryptoRandomBytes },
    { name: '11. EventEmitter / emit & listen (50k)', fn: benchEventEmitter },
    { name: '12. File System / Sync Read & Write (100 x 32KB)', fn: benchFsSync },
    { name: '13. Map & Set / Insert & Lookup (50k)', fn: benchMapSet },
    { name: '14. TypedArray / Float64 Reduce (1M)', fn: benchTypedArray },
    { name: '15. TextEncoder / TextDecoder (200 x 64KB)', fn: benchTextCodec },
    { name: '16. URL & URLSearchParams (20k)', fn: benchUrlParse },
    { name: '17. structuredClone nested objects (2k)', fn: benchStructuredClone },
    { name: '18. Promise / microtask storm (20k)', fn: benchPromiseMicrotasks },
    { name: '19. Web Crypto / subtle.digest SHA-256 (100 x 16KB)', fn: benchWebCryptoSha256 },
    { name: '20. ReadableStream produce & consume (5k chunks)', fn: benchReadableStream },
    { name: '21. WebAssembly / instantiate + 100k add calls', fn: benchWasmAddCalls },
    { name: '22. Date / toISOString (50k)', fn: benchDateFormat },
    { name: '23. File System / Async promises R&W (50 x 32KB)', fn: benchFsPromises },
    { name: '24. CompressionStream / gzip (20 x 32KB)', fn: benchCompressionStream },
];

(async () => {
    console.log('Runtime Platform:', process.platform, process.arch);
    console.log(
        'Runtime Version:',
        typeof process.versions === 'object' ? JSON.stringify(process.versions) : 'unknown'
    );
    console.log(`Starting Benchmark Suite (${benchmarks.length} workloads, ${BENCH_ITERS} samples each)...\n`);

    const results = [];
    for (const b of benchmarks) {
        const res = await runBench(b.name, b.fn);
        results.push(res);
        if (res.skipped) {
            console.log(`${res.name.padEnd(58)}: SKIPPED  (${res.error})`);
        } else {
            console.log(
                res.name.padEnd(58) +
                    ': ' +
                    res.avgMs.toFixed(2).padStart(8) +
                    ' ms  (' +
                    res.opsSec.toFixed(1).padStart(7) +
                    ' ops/s)'
            );
        }
    }

    console.log('\nSummary JSON:');
    console.log(JSON.stringify(results, null, 2));
})();
