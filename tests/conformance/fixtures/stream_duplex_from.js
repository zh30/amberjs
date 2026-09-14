const assert = require('node:assert');
const { Duplex, Writable } = require('node:stream');
const EventEmitter = require('node:events');

async function test() {
  // 1. Prototype inheritance
  const w = new Writable();
  assert.ok(w instanceof EventEmitter, 'Writable must inherit from EventEmitter');
  assert.strictEqual(typeof w.once, 'function', 'Writable must have once method');
  let onceFired = false;
  w.once('custom', () => { onceFired = true; });
  w.emit('custom');
  assert.strictEqual(onceFired, true);

  // 2. Duplex.from
  assert.strictEqual(typeof Duplex.from, 'function', 'Duplex.from must be a function');
  async function* gen() {
    yield 'hello ';
    yield 'world';
  }
  const stream = Duplex.from(gen());
  const chunks = [];
  for await (const chunk of stream) {
    chunks.push(String(chunk));
  }
  assert.strictEqual(chunks.join(''), 'hello world');

  console.log('CONFORMANCE_PASS');
}

test().catch((err) => {
  console.error(err);
  process.exit(1);
});
