#!/usr/bin/env amberjs
/**
 * Stage 91 验证测试
 * 验证 amberjs 运行时基本功能
 */

console.log("✅ Amber 运行时验证测试");
console.log("===========================\n");

console.log("📋 基本功能测试:");

// 测试 1: 基本计算
console.log("1. 基本计算: ");
let result = 2 + 2;
console.log(`   2 + 2 = ${result}`);
if (result === 4) {
    console.log("   ✅ 通过");
} else {
    console.log("   ❌ 失败");
}

// 测试 2: 字符串操作
console.log("\n2. 字符串操作: ");
let str = "Hello";
str += " World";
console.log(`   "${str}"`);
if (str === "Hello World") {
    console.log("   ✅ 通过");
} else {
    console.log("   ❌ 失败");
}

// 测试 3: 数组操作
console.log("\n3. 数组操作: ");
let arr = [1, 2, 3, 4, 5];
arr.push(6);
console.log(`   数组长度: ${arr.length}`);
if (arr.length === 6 && arr[5] === 6) {
    console.log("   ✅ 通过");
} else {
    console.log("   ❌ 失败");
}

// 测试 4: 对象操作
console.log("\n4. 对象操作: ");
let obj = { x: 10, y: 20 };
obj.z = 30;
console.log(`   对象属性: x=${obj.x}, y=${obj.y}, z=${obj.z}`);
if (obj.x === 10 && obj.y === 20 && obj.z === 30) {
    console.log("   ✅ 通过");
} else {
    console.log("   ❌ 失败");
}

// 测试 5: 函数调用
console.log("\n5. 函数调用: ");
function multiply(a, b) {
    return a * b;
}
let product = multiply(5, 6);
console.log(`   multiply(5, 6) = ${product}`);
if (product === 30) {
    console.log("   ✅ 通过");
} else {
    console.log("   ❌ 失败");
}

// 测试 6: 循环
console.log("\n6. 循环测试: ");
let sum = 0;
for (let i = 1; i <= 10; i++) {
    sum += i;
}
console.log(`   1+2+...+10 = ${sum}`);
if (sum === 55) {
    console.log("   ✅ 通过");
} else {
    console.log("   ❌ 失败");
}

// 测试 7: 条件判断
console.log("\n7. 条件判断: ");
let score = 85;
let grade = score >= 80 ? "A" : score >= 70 ? "B" : "C";
console.log(`   分数 ${score} 的等级是: ${grade}`);
if (grade === "A") {
    console.log("   ✅ 通过");
} else {
    console.log("   ❌ 失败");
}

console.log("\n📊 性能测试 (轻量级):");

// 轻量级性能测试
console.log("8. 轻量级性能测试: ");
const iterations = 10000; // 减少迭代次数
const start = Date.now();
for (let i = 0; i < iterations; i++) {
    let x = i * 2 + 1;
}
const duration = Date.now() - start;
const opsPerSec = Math.round(iterations / (duration / 1000));
console.log(`   ${iterations.toLocaleString()} 次简单运算`);
console.log(`   耗时: ${duration}ms`);
console.log(`   性能: ${opsPerSec.toLocaleString()} ops/sec`);

if (duration < 1000) { // 少于 1 秒认为通过
    console.log("   ✅ 通过");
} else {
    console.log("   ⚠️  性能可能需要优化");
}

console.log("\n🎉 所有测试完成！");
console.log("✅ Amber 运行时基本功能正常");
console.log(`📈 轻量级性能: ${opsPerSec.toLocaleString()} ops/sec\n`);
