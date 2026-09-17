// Amber v0.2.0 基础功能测试

console.log('=== Amber v0.2.0 基础功能测试 ===');

// 1. 基础算术
console.log('\n--- 基础算术 ---');
let sum = 0;
for (let i = 0; i < 100000; i++) {
    sum += i;
}
console.log('Sum:', sum);

// 2. HTTP Fetch
console.log('\n--- HTTP Fetch ---');
const response = fetch('https://httpbin.org/json');
console.log('Status:', response.status);
console.log('OK:', response.ok);

// 3. Web API
console.log('\n--- Web API ---');
console.log('Math.PI:', Math.PI);
console.log('JSON:', JSON.stringify({test: true}));

// 4. URL API
console.log('\n--- URL API ---');
const url = new URL('https://example.com/path');
console.log('Host:', url.host);
console.log('Path:', url.pathname);

// 5. Crypto API
console.log('\n--- Crypto API ---');
console.log('UUID:', crypto.randomUUID());

console.log('\n=== 完成 ===');
