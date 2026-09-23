// 测试 Amber v0.2.1 Promise 功能
console.log('=== 测试 Promise API v0.2.1 ===\n');

// 1. 测试 Promise.resolve
console.log('--- Promise.resolve ---');
Promise.resolve(42).then(result => {
    console.log('Promise.resolve(42):', result);
});

// 2. 测试 Promise.reject
console.log('--- Promise.reject ---');
Promise.reject(new Error('Test error')).catch(err => {
    console.log('Promise.reject error:', err.message);
});

// 3. 测试 Promise.all
console.log('--- Promise.all ---');
Promise.all([
    Promise.resolve(1),
    Promise.resolve(2),
    Promise.resolve(3)
]).then(results => {
    console.log('Promise.all results:', results);
});

// 4. 测试 Promise.allSettled
console.log('--- Promise.allSettled ---');
Promise.allSettled([
    Promise.resolve(1),
    Promise.reject(new Error('Failed')),
    Promise.resolve(3)
]).then(results => {
    console.log('Promise.allSettled results:', results);
});

// 5. 测试 Promise.race
console.log('--- Promise.race ---');
Promise.race([
    new Promise(resolve => setTimeout(() => resolve('slow'), 100)),
    Promise.resolve('fast')
]).then(result => {
    console.log('Promise.race winner:', result);
});

// 6. 测试 Promise.any
console.log('--- Promise.any ---');
Promise.any([
    Promise.resolve(1),
    Promise.resolve(2),
    Promise.resolve(3)
]).then(result => {
    console.log('Promise.any result:', result);
});

// 7. 测试链式调用
console.log('--- Promise Chain ---');
Promise.resolve(1)
    .then(x => x + 1)
    .then(x => x * 2)
    .then(x => x + 3)
    .then(result => {
        console.log('Promise chain result:', result);
    });

console.log('\n✅ Promise API 测试完成!');
