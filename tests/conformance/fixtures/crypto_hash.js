const crypto = require('crypto');

function assert(cond, msg) {
  if (!cond) throw new Error(msg || 'assertion failed');
}

assert(typeof crypto.createHash === 'function', 'createHash exists');
assert(typeof crypto.randomBytes === 'function', 'randomBytes exists');

const digest = crypto.createHash('sha256').update('amberjs').digest('hex');
assert(typeof digest === 'string', 'digest hex is string');
assert(digest.length === 64, 'sha256 hex length, got: ' + digest.length);
assert(/^[0-9a-f]{64}$/.test(digest), 'sha256 hex charset');

const again = crypto.createHash('sha256').update('amberjs').digest('hex');
assert(digest === again, 'createHash is deterministic');

const bytes = crypto.randomBytes(16);
assert(bytes && (bytes.byteLength === 16 || bytes.length === 16), 'randomBytes(16) length');

console.log('CONFORMANCE_PASS');
