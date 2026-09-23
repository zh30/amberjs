const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');

const fastifyPath = path.resolve(__dirname, '../../../benchmarks/idle_memory/node_modules/fastify');
if (!fs.existsSync(fastifyPath)) {
    console.log('CONFORMANCE_SKIP');
    process.exit(0);
}
const Fastify = require(fastifyPath);

const app = Fastify({ logger: false });
assert.ok(app);
assert.strictEqual(typeof app.get, 'function');
assert.strictEqual(typeof app.post, 'function');
assert.strictEqual(typeof app.listen, 'function');

app.get('/health', async () => ({ status: 'ok', framework: 'fastify' }));
app.post('/echo', async (req) => ({ echo: req.body }));

async function main() {
    // 1. GET /health via app.inject
    const res1 = await app.inject({ method: 'GET', url: '/health' });
    assert.strictEqual(res1.statusCode, 200);
    const body1 = JSON.parse(res1.body);
    assert.strictEqual(body1.status, 'ok');
    assert.strictEqual(body1.framework, 'fastify');

    // 2. POST /echo with JSON payload
    const res2 = await app.inject({
        method: 'POST',
        url: '/echo',
        headers: { 'content-type': 'application/json' },
        payload: { greeting: 'fastify-amberjs' },
    });
    assert.strictEqual(res2.statusCode, 200);
    const body2 = JSON.parse(res2.body);
    assert.deepStrictEqual(body2.echo, { greeting: 'fastify-amberjs' });

    console.log('CONFORMANCE_PASS');
}

main().catch((err) => {
    console.error(err);
    process.exit(1);
});
