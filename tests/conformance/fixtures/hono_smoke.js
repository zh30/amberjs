const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');

const honoPath = path.resolve(__dirname, '../../../benchmarks/idle_memory/node_modules/hono');
const nodeServerPath = path.resolve(__dirname, '../../../benchmarks/idle_memory/node_modules/@hono/node-server');
if (!fs.existsSync(honoPath) || !fs.existsSync(nodeServerPath)) {
  console.log('CONFORMANCE_SKIP');
  process.exit(0);
}

const { Hono } = require(honoPath);
const { getRequestListener } = require(nodeServerPath);

const app = new Hono();
app.get('/health', (c) => c.json({ status: 'ok', framework: 'hono' }));
app.get('/greet/:name', (c) => c.text('Hello, ' + c.req.param('name') + '!'));

const listener = getRequestListener(app.fetch);
assert.strictEqual(typeof listener, 'function');

async function main() {
  // 1. Test GET /health
  const res1 = await new Promise((resolve) => {
    const req = new http.IncomingMessage();
    req.method = 'GET';
    req.url = '/health';
    req.headers = { host: 'localhost:3000' };
    req.socket = { encrypted: false, remoteAddress: '127.0.0.1', remotePort: 12345 };

    const res = new http.ServerResponse(req);
    let body = '';
    let status = 200;
    res.writeHead = function(s) { status = s; return res; };
    res.write = function(chunk) { if (chunk) body += chunk; return true; };
    res.end = function(chunk) {
      if (chunk) body += chunk;
      resolve({ status, body });
    };
    listener(req, res);
  });
  assert.strictEqual(res1.status, 200);
  const json1 = JSON.parse(res1.body);
  assert.strictEqual(json1.status, 'ok');
  assert.strictEqual(json1.framework, 'hono');

  // 2. Test param route /greet/Beejs
  const res2 = await new Promise((resolve) => {
    const req = new http.IncomingMessage();
    req.method = 'GET';
    req.url = '/greet/Beejs';
    req.headers = { host: 'localhost:3000' };
    req.socket = { encrypted: false, remoteAddress: '127.0.0.1', remotePort: 12345 };

    const res = new http.ServerResponse(req);
    let body = '';
    let status = 200;
    res.writeHead = function(s) { status = s; return res; };
    res.write = function(chunk) { if (chunk) body += chunk; return true; };
    res.end = function(chunk) {
      if (chunk) body += chunk;
      resolve({ status, body });
    };
    listener(req, res);
  });
  assert.strictEqual(res2.status, 200);
  assert.strictEqual(res2.body, 'Hello, Beejs!');

  console.log('CONFORMANCE_PASS');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
