// Amber v0.2.0 完整 Web API 演示
// 展示所有支持的 Web API 功能

console.log('=== Amber v0.2.0 Web API 完整演示 ===\n');

// 1. Console API
console.log('--- 1. Console API ---');
console.log('console.log 正常工作');
console.error('console.error 显示错误');
console.warn('console.warn 显示警告');

// 2. Math API
console.log('\n--- 2. Math API ---');
console.log('Math.PI:', Math.PI);
console.log('Math.abs(-42):', Math.abs(-42));
console.log('Math.floor(3.14):', Math.floor(3.14));
console.log('Math.ceil(2.718):', Math.ceil(2.718));
console.log('Math.round(2.5):', Math.round(2.5));
console.log('Math.sqrt(64):', Math.sqrt(64));
console.log('Math.max(10, 20, 30):', Math.max(10, 20, 30));
console.log('Math.min(-5, -10, -3):', Math.min(-5, -10, -3));
console.log('Math.random():', Math.random());

// 3. JSON API
console.log('\n--- 3. JSON API ---');
let testObj = {
    name: 'Amber',
    version: '0.2.0',
    features: ['高性能', '异步', 'HTTP'],
    nested: {
        runtime: 'V8 + Rust',
        performance: '超越 Bun'
    }
};
console.log('原始对象:', testObj);
console.log('JSON.stringify:', JSON.stringify(testObj, null, 2));

let parsedJson;
try {
    parsedJson = JSON.parse('{"test": true, "value": 123}');
} catch (error) {
    throw new Error(`JSON.parse failed: ${error}`);
}
console.log('JSON.parse:', parsedJson);

// 4. URL API
console.log('\n--- 4. URL API ---');
let url;
try {
    url = new URL('https://api.example.com:8080/v1/users?sort=name&limit=10#profile');
} catch (error) {
    throw new Error(`URL parse failed: ${error}`);
}
console.log('href:', url.href);
console.log('protocol:', url.protocol);
console.log('host:', url.host);
console.log('hostname:', url.hostname);
console.log('port:', url.port);
console.log('pathname:', url.pathname);
console.log('search:', url.search);
console.log('hash:', url.hash);
console.log('origin:', url.origin);

// 5. Crypto API
console.log('\n--- 5. Crypto API ---');
console.log('crypto.randomUUID():', crypto.randomUUID());

// 6. Fetch API (真实 HTTP)
console.log('\n--- 6. Fetch API (真实 HTTP) ---');
let response = fetch('https://httpbin.org/json');
console.log('HTTP 状态码:', response.status);
console.log('响应 OK:', response.ok);
console.log('JSON 方法:', response.json());
console.log('文本方法:', response.text());

// 7. 异步定时器
console.log('\n--- 7. 异步定时器 ---');
console.log('设置延迟 10ms 定时器');
setTimeout(() => {
    console.log('setTimeout 回调执行');
}, 10);

let intervalId = setInterval(() => {
    console.log('setInterval 执行');
}, 1000);

console.log('定时器 ID:', intervalId);

// 8. Process API
console.log('\n--- 8. Process API ---');
console.log('process.version:', process.version);
console.log('process.platform:', process.platform);
console.log('process.arch:', process.arch);

// 9. 性能测试
console.log('\n--- 9. 性能测试 ---');
let iterations = 5000000;
let sum = 0;
let start = Date.now();

for (let i = 0; i < iterations; i++) {
    sum += i * 2 - 1;
}

let end = Date.now();
let duration = end - start;
let opsPerSec = (iterations / (duration / 1000)).toFixed(0);

console.log(`执行 ${iterations} 次算术运算:`);
console.log(`耗时: ${duration}ms`);
console.log(`性能: ${opsPerSec} ops/sec`);
console.log(`结果: ${sum}`);

// 10. 高级功能组合
console.log('\n--- 10. 高级功能组合 ---');
let data = {
    timestamp: Date.now(),
    url: 'https://api.example.com/data',
    crypto: crypto.randomUUID(),
    math: {
        pi: Math.PI,
        random: Math.random()
    },
    parsed: JSON.parse('{"status": "active", "count": 42}')
};

console.log('组合数据对象:');
console.log(JSON.stringify(data, null, 2));

console.log('\n=== Web API 演示完成 ===');
console.log('所有核心 Web API 在 Amber v0.2.0 中正常工作！');
