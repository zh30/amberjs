// fs conformance fixture: sync read/write/stat/mkdir/readdir contracts
const fs = require('fs');
const os = require('os');
const path = require('path');

function assert(cond, msg) {
  if (!cond) throw new Error(msg || 'assertion failed');
}

const root = path.join(os.tmpdir(), 'amber_conformance_fs_' + process.pid);
const file = path.join(root, 'hello.txt');
const nested = path.join(root, 'a', 'b');

// write + read roundtrip
fs.mkdirSync(nested, { recursive: true });
assert(fs.existsSync(nested), 'mkdirSync recursive creates nested dirs');
fs.writeFileSync(file, 'hello amber');
assert(fs.readFileSync(file, 'utf8') === 'hello amber', 'readFileSync utf8 roundtrip');

// overwrite semantics
fs.writeFileSync(file, 'second');
assert(fs.readFileSync(file, 'utf8') === 'second', 'writeFileSync overwrites');

// stat contract
const st = fs.statSync(file);
assert(st.isFile() === true, 'statSync isFile for file');
assert(st.isDirectory() === false, 'statSync isDirectory false for file');
assert(st.size === 6, 'statSync size matches content length');
const sd = fs.statSync(root);
assert(sd.isDirectory() === true, 'statSync isDirectory for dir');

// readdir contract
fs.writeFileSync(path.join(nested, 'x.txt'), 'x');
const entries = fs.readdirSync(root);
assert(Array.isArray(entries), 'readdirSync returns array');
assert(entries.indexOf('hello.txt') !== -1 && entries.indexOf('a') !== -1, 'readdirSync lists entries');
assert(fs.readdirSync(nested).indexOf('x.txt') !== -1, 'readdirSync nested');

// existsSync contract
assert(fs.existsSync(file) === true, 'existsSync true');
assert(fs.existsSync(path.join(root, 'nope')) === false, 'existsSync false');

// rename + unlink
const moved = path.join(root, 'moved.txt');
fs.renameSync(file, moved);
assert(!fs.existsSync(file) && fs.existsSync(moved), 'renameSync moves file');
fs.unlinkSync(moved);
assert(!fs.existsSync(moved), 'unlinkSync removes file');

// error contract: reading a missing file must throw
let threw = false;
try {
  fs.readFileSync(path.join(root, 'missing.txt'), 'utf8');
} catch (e) {
  threw = true;
}
assert(threw, 'readFileSync missing file throws');

// Clean up the fixture tree so repeated runs stay hermetic.
fs.unlinkSync(path.join(nested, 'x.txt'));
fs.rmdirSync(nested);
fs.rmdirSync(path.join(root, 'a'));
fs.rmdirSync(root);
assert(!fs.existsSync(root), 'fixture tree removed');

console.log('CONFORMANCE_PASS');
