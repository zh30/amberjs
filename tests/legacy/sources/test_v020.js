// Amber v0.2.0 功能测试
// 测试异步事件循环和真实 HTTP 功能

console.log('=== Amber v0.2.0 功能测试 ===');

// 1. 基础 JavaScript 执行
let sum = 0;
for (let i = 0; i < 1000000; i++) {
    sum += i;
}
console.log(`基础算术测试: ${sum}`);

// 2. 测试定时器功能
console.log('\n--- 定时器测试 ---');
setTimeout(() => {
    console.log('setTimeout 延迟 100ms 执行');
}, 100);

setInterval(() => {
    console.log('setInterval 执行');
}, 500);

// 3. 测试 fetch 功能（真实 HTTP）
console.log('\n--- HTTP Fetch 测试 ---');
fetch('https://httpbin.org/json')
    .then(response => {
        console.log(`HTTP 状态码: ${response.status}`);
        console.log(`响应 OK: ${response.ok}`);
        return response.json();
    })
    .then(data => {
        console.log('JSON 数据:', data);
    })
    .catch(error => {
        console.log('Fetch 错误:', error.message);
    });

// 4. 测试 Web API
console.log('\n--- Web API 测试 ---');
console.log('console.log 正常');
console.log('Math.PI:', Math.PI);

// 5. 性能测试
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

console.log('\n=== 测试完成 ===');
