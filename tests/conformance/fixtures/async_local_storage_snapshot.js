const assert = require('node:assert');
const { AsyncLocalStorage } = require('node:async_hooks');

async function test() {
  const als1 = new AsyncLocalStorage();
  const als2 = new AsyncLocalStorage();

  let runSnapshot;
  als1.run('store-1', () => {
    als2.run('store-2', () => {
      runSnapshot = AsyncLocalStorage.snapshot();
    });
  });

  assert.strictEqual(typeof runSnapshot, 'function');
  assert.strictEqual(als1.getStore(), undefined);
  assert.strictEqual(als2.getStore(), undefined);

  // Run in snapshot outside the context
  runSnapshot(() => {
    assert.strictEqual(als1.getStore(), 'store-1');
    assert.strictEqual(als2.getStore(), 'store-2');
  });

  // Ensure outside context remains unaffected
  assert.strictEqual(als1.getStore(), undefined);
  assert.strictEqual(als2.getStore(), undefined);

  console.log('CONFORMANCE_PASS');
}

test().catch((err) => {
  console.error(err);
  process.exit(1);
});
