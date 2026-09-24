const assert = require('node:assert');
const path = require('node:path');

// The require callback used to keep the builtin-module arm on the stack for
// every file load. That frame overflowed while compiling a chain only a few
// modules deep (express and fastify). This chain is longer than that limit.
const loaded = require(path.join(__dirname, 'cjs_nested/m0.js'));
assert.strictEqual(loaded.ok, true);
assert.strictEqual(loaded.depth, 12);
console.log('CONFORMANCE_PASS');
