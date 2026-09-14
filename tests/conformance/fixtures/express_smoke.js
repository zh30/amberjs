const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');

const expressPath = path.resolve(__dirname, '../../../benchmarks/idle_memory/node_modules/express');
if (!fs.existsSync(expressPath)) {
  console.log('CONFORMANCE_SKIP');
  process.exit(0);
}

const express = require(expressPath);
const app = express();
assert.strictEqual(typeof app, 'function');
assert.strictEqual(typeof app.get, 'function');
assert.strictEqual(typeof app.post, 'function');
assert.strictEqual(typeof app.use, 'function');

app.use(express.json());

app.get('/health', (req, res) => {
  res.status(200).json({ status: 'ok', framework: 'express' });
});

app.get('/user/:id', (req, res) => {
  res.status(200).json({ id: req.params.id, query: req.query });
});

async function main() {
  // 1. Test GET /health
  const getRes = await new Promise((resolve) => {
    const req = new http.IncomingMessage();
    req.method = 'GET';
    req.url = '/health';
    req.headers = { host: 'localhost' };
    const res = new http.ServerResponse(req);
    let body = '';
    res.end = function(chunk) {
      if (chunk) body += chunk;
      resolve({ status: res.statusCode, body });
    };
    app(req, res);
  });
  assert.strictEqual(getRes.status, 200);
  const getParsed = JSON.parse(getRes.body);
  assert.strictEqual(getParsed.status, 'ok');
  assert.strictEqual(getParsed.framework, 'express');

  // 2. Test GET /user/42?tab=profile (Route Params & Query Parsing)
  const userRes = await new Promise((resolve) => {
    const req = new http.IncomingMessage();
    req.method = 'GET';
    req.url = '/user/42?tab=profile';
    req.headers = { host: 'localhost' };
    const res = new http.ServerResponse(req);
    let body = '';
    res.end = function(chunk) {
      if (chunk) body += chunk;
      resolve({ status: res.statusCode, body });
    };
    app(req, res);
  });
  assert.strictEqual(userRes.status, 200);
  const userParsed = JSON.parse(userRes.body);
  assert.strictEqual(userParsed.id, '42');
  assert.deepStrictEqual(userParsed.query, { tab: 'profile' });

  console.log('CONFORMANCE_PASS');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
