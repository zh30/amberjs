// 测试 Amber v0.1.7 fs Web API 完整功能
console.log("🧪 测试 Amber v0.1.7 fs Web API 完整功能");

// 测试 fs API 是否可用
console.log("\n1. fs API 可用性测试:");
console.log("typeof fs =", typeof fs);

// 测试 fs.exists
console.log("\n2. fs.exists() 测试:");
console.log("fs.exists('./Cargo.toml') =", fs.exists('./Cargo.toml'));
console.log("fs.exists('./nonexistent.txt') =", fs.exists('./nonexistent.txt'));

// 测试 fs.writeFile
console.log("\n3. fs.writeFile() 测试:");
const testContent = 'Hello from Amber v0.1.7! Time: ' + new Date().toISOString();
console.log("fs.writeFile('./test_fs.txt', '" + testContent + "')");
const writeResult = fs.writeFile('./test_fs.txt', testContent);
console.log("Result:", writeResult);

// 测试 fs.readFile
console.log("\n4. fs.readFile() 测试:");
console.log("fs.readFile('./test_fs.txt', 'utf8') =", fs.readFile('./test_fs.txt', 'utf8'));

// 测试 fs.stat
console.log("\n5. fs.stat() 测试:");
const stats = fs.stat('./test_fs.txt');
console.log("fs.stat('./test_fs.txt') =", stats);
console.log("stats.size =", stats.size);
console.log("stats.isFile =", stats.isFile);
console.log("stats.isDirectory =", stats.isDirectory);

// 测试 fs.readdir
console.log("\n6. fs.readdir() 测试:");
console.log("fs.readdir('.') =", fs.readdir('.'));

// 测试 fs.mkdir
console.log("\n7. fs.mkdir() 测试:");
console.log("fs.mkdir('./test_dir')");
const mkdirResult = fs.mkdir('./test_dir');
console.log("Result:", mkdirResult);
console.log("fs.exists('./test_dir') =", fs.exists('./test_dir'));

// 测试 fs.unlink
console.log("\n8. fs.unlink() 测试:");
console.log("fs.unlink('./test_fs.txt')");
const unlinkResult = fs.unlink('./test_fs.txt');
console.log("Result:", unlinkResult);
console.log("fs.exists('./test_fs.txt') =", fs.exists('./test_fs.txt'));

// 清理测试目录
console.log("\n9. 清理测试:");
console.log("fs.unlink('./test_dir')");
const cleanupResult = fs.unlink('./test_dir');
console.log("Result:", cleanupResult);

console.log("\n✅ fs Web API 所有功能测试完成!");
