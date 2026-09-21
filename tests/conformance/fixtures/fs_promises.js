const fs = require('fs');
const os = require('os');
const path = require('path');

function assert(cond, msg) {
  if (!cond) throw new Error(msg || 'assertion failed');
}

const root = path.join(os.tmpdir(), 'amber_conformance_fs_promises_' + process.pid);
const file = path.join(root, 'hello.txt');
fs.mkdirSync(root, { recursive: true });

fs.promises
  .writeFile(file, 'hello promises')
  .then(() => fs.promises.readFile(file, 'utf8'))
  .then((data) => {
    assert(data === 'hello promises', 'promises.readFile/writeFile roundtrip, got: ' + data);
    fs.unlinkSync(file);
    fs.rmdirSync(root);
    console.log('CONFORMANCE_PASS');
  });
