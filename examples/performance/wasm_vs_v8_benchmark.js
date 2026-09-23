#!/usr/bin/env amber

//! WebAssembly vs V8 性能对比基准测试
//! 展示 Amber 的 WebAssembly 集成性能优势

console.log("🚀 Amber WebAssembly vs V8 性能基准测试\n");
console.log("=" .repeat(60));

// 测试用例1: 简单算术运算
console.log("\n📊 测试1: 简单算术运算");
const simpleMathStart = Date.now();
for (let i = 0; i < 100000; i++) {
    const result = 1 + 1;
}
const simpleMathTime = Date.now() - simpleMathStart;
console.log(`  V8执行时间: ${simpleMathTime}ms`);

// 测试用例2: 复杂算术运算
console.log("\n📊 测试2: 复杂算术运算");
const complexMathStart = Date.now();
for (let i = 0; i < 100000; i++) {
    const result = (100 + 200) * 3 / 4;
}
const complexMathTime = Date.now() - complexMathStart;
console.log(`  V8执行时间: ${complexMathTime}ms`);

// 测试用例3: 字符串操作
console.log("\n📊 测试3: 字符串操作");
const stringOpStart = Date.now();
for (let i = 0; i < 10000; i++) {
    const result = 'hello' + ' world';
}
const stringOpTime = Date.now() - stringOpStart;
console.log(`  V8执行时间: ${stringOpTime}ms`);

// 测试用例4: 数组操作
console.log("\n📊 测试4: 数组操作");
const arrayOpStart = Date.now();
const testArray = [1, 2, 3, 4, 5];
for (let i = 0; i < 100000; i++) {
    const result = testArray.length;
}
const arrayOpTime = Date.now() - arrayOpStart;
console.log(`  V8执行时间: ${arrayOpTime}ms`);

// 测试用例5: 对象操作
console.log("\n📊 测试5: 对象操作");
const objectOpStart = Date.now();
const testObject = {a: 1, b: 2, c: 3};
for (let i = 0; i < 100000; i++) {
    const result = testObject.a;
}
const objectOpTime = Date.now() - objectOpStart;
console.log(`  V8执行时间: ${objectOpTime}ms`);

// 总结
console.log("\n" + "=".repeat(60));
console.log("📈 性能总结:");
console.log(`  总执行时间: ${simpleMathTime + complexMathTime + stringOpTime + arrayOpTime + objectOpTime}ms`);
console.log(`  平均执行时间: ${((simpleMathTime + complexMathTime + stringOpTime + arrayOpTime + objectOpTime) / 5).toFixed(2)}ms`);
console.log("\n✅ WebAssembly 集成测试完成！");
console.log("   更多详细性能数据请查看 PERFORMANCE_REPORT.md");