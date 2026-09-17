const assert = require('assert');
const zlib = require('zlib');

const input = 'Amber deterministic engine & zlib compression test string!';
const compressed = zlib.gzipSync(input);
assert(compressed.length > 0, 'compressed output should not be empty');
assert.strictEqual(typeof compressed.toString, 'function', 'gzipSync should return a Buffer');

const decompressed = zlib.gunzipSync(compressed);
assert.strictEqual(decompressed.toString('utf8'), input, 'gunzipSync output should match original input');

// deflateSync / inflateSync
const deflated = zlib.deflateSync(input);
const inflated = zlib.inflateSync(deflated);
assert.strictEqual(inflated.toString('utf8'), input, 'inflateSync output should match original input');

console.log('CONFORMANCE_PASS');
