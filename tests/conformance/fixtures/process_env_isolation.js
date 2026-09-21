// process.env conformance fixture
const assert = require('assert');

assert.ok(process.env, 'process.env must exist');
assert.strictEqual(typeof process.env, 'object', 'process.env must be an object');

process.env.__AMBER_TEST_VAR__ = 'conformance_val_123';
assert.strictEqual(process.env.__AMBER_TEST_VAR__, 'conformance_val_123');
delete process.env.__AMBER_TEST_VAR__;
assert.strictEqual(process.env.__AMBER_TEST_VAR__, undefined);

console.log('CONFORMANCE_PASS');
