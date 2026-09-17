#!/usr/bin/env amberjs
/**
 * 极简 ICU 错误定位测试
 * 逐步增加复杂度，确定 ICU 错误的确切触发条件
 */

console.log("开始 ICU 错误定位测试...\n");

// 测试 1: 最简单的数值计算
console.log("测试 1: 纯数值计算");
let sum = 0;
for (let i = 0; i < 1000; i++) {
    sum += i * 2 + 1;
}
console.log(`结果: ${sum}\n`);

// 测试 2: 增加迭代次数
console.log("测试 2: 增加迭代到 10000");
sum = 0;
for (let i = 0; i < 10000; i++) {
    sum += i * 2 + 1;
}
console.log(`结果: ${sum}\n`);

// 测试 3: 增加迭代到 50000
console.log("测试 3: 增加迭代到 50000");
sum = 0;
for (let i = 0; i < 50000; i++) {
    sum += i * 2 + 1;
}
console.log(`结果: ${sum}\n`);

// 测试 4: 增加到 100000
console.log("测试 4: 增加迭代到 100000");
sum = 0;
for (let i = 0; i < 100000; i++) {
    sum += i * 2 + 1;
}
console.log(`结果: ${sum}\n`);

console.log("✅ 所有测试通过，未触发 ICU 错误");
