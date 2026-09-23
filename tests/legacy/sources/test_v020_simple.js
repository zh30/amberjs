// Amber v0.2.0 简化功能测试
// 测试当前已支持的功能

console.log('=== Amber v0.2.0 简化功能测试 ===');

// 1. 基础 JavaScript 执行
console.log('\n--- 基础功能测试 ---');
let sum = 0;
for (let i = 0; i < 1000000; i++) {
    sum += i;
}
console.log(`基础算术测试: ${sum}`);

// 2. 测试 fetch 功能（检查状态码）
console.log('\n--- HTTP Fetch 测试 ---');
const response = fetch('https://httpbin.org/json');
console.log(`HTTP 状态码: ${response.status}`);
console.log(`响应 OK: ${response.ok}`);

// 3. 测试 Web API
console.log('\n--- Web API 测试 ---');
console.log('console.log 正常');
console.log('Math.PI:', Math.PI);
console.log('JSON.stringify:', JSON.stringify({test: true}));

// 4. 性能测试
console.log('\n--- 性能测试 ---');
let start = Date.now();
let ops = 0;
for (let i = 0; i < 10000000; i++) {
    ops++;
}
let end = Date.now();
let duration = end - start;
let opsPerSec = Math.round((ops / duration) * 1000);
console.log(`执行 ${ops} 操作耗时: ${duration}ms`);
console.log(`性能: ${opsPerSec} ops/sec`);

// 5. 测试 URL API
console.log('\n--- URL API 测试 ---');
const url = new URL('https://example.com:8080/path/to/resource?query=value#fragment');
console.log('URL href:', url.href);
console.log('URL host:', url.host);
console.log('URL pathname:', url.pathname);

// 6. 测试 Crypto API
console.log('\n--- Crypto API 测试 ---');
console.log('Random UUID:', crypto.randomUUID());

console.log('\n=== 测试完成 ===');
