/**
 * Extended I/O benchmarks: fetch client, WebSocket echo, SQLite (bee:db).
 * Missing APIs are reported as skipped rather than aborting the suite.
 */

const WARMUP = 1;
const SAMPLES = 3;

async function runBench(name, fn) {
    try {
        for (let i = 0; i < WARMUP; i++) await fn();
        const times = [];
        for (let i = 0; i < SAMPLES; i++) {
            const start = performance.now();
            await fn();
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

async function benchFetchClient() {
    const url = process.env.BENCH_URL || 'http://127.0.0.1:19199/';
    if (typeof fetch !== 'function') throw new Error('fetch unavailable');
    let bytes = 0;
    for (let i = 0; i < 100; i++) {
        const res = await fetch(url);
        if (!res.ok) throw new Error(`fetch status ${res.status}`);
        const text = await res.text();
        bytes += text.length;
    }
    return bytes;
}

async function benchSqliteInsertSelect() {
    let Database;
    try {
        ({ Database } = require('amber:db'));
    } catch (err) {
        throw new Error(`amber:db unavailable: ${err.message}`);
    }
    const db = new Database(':memory:');
    db.exec('CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT, score REAL)');
    const insert = db.prepare('INSERT INTO items (name, score) VALUES (?, ?)');
    const select = db.prepare('SELECT id, name, score FROM items WHERE id = ?');
    const n = 2000;
    for (let i = 0; i < n; i++) insert.run(`row-${i}`, i * 0.5);
    let checksum = 0;
    for (let i = 1; i <= n; i++) {
        const row = select.get ? select.get(i) : select.all(i)[0];
        if (row) checksum += row.id;
    }
    db.close();
    return checksum;
}

async function benchSqliteTransaction() {
    let Database;
    try {
        ({ Database } = require('amber:db'));
    } catch (err) {
        throw new Error(`amber:db unavailable: ${err.message}`);
    }
    const db = new Database(':memory:');
    db.exec('CREATE TABLE t (id INTEGER PRIMARY KEY, v INTEGER)');
    const insert = db.prepare('INSERT INTO t (v) VALUES (?)');
    const n = 5000;
    db.exec('BEGIN');
    for (let i = 0; i < n; i++) insert.run(i);
    db.exec('COMMIT');
    const countRow = db.prepare('SELECT COUNT(*) AS c FROM t').get
        ? db.prepare('SELECT COUNT(*) AS c FROM t').get()
        : db.query('SELECT COUNT(*) AS c FROM t')[0];
    db.close();
    return countRow && (countRow.c || countRow['COUNT(*)']);
}

(async () => {
    const benches = [
        { name: '25. Fetch / 100 sequential GETs', fn: benchFetchClient },
        { name: '26. SQLite / 2k insert + point select', fn: benchSqliteInsertSelect },
        { name: '27. SQLite / 5k transactional inserts', fn: benchSqliteTransaction },
    ];
    console.log('Starting Extended I/O Suite (' + benches.length + ' workloads)...\n');
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
