/**
 * Amber 模块系统示例
 *
 * 演示 CommonJS 和 ES6 模块的使用
 */

// ===== CommonJS 模块示例 =====

// 导出方式 1: module.exports
// math.js 内容:
exports.add = (a, b) => {
    return a + b;
};

exports.multiply = (a, b) => {
    return a * b;
};

exports.PI = 3.14159;

// 导出方式 2: 整个对象
const constants = {
    E: 2.71828,
    GOLDEN_RATIO: 1.61803,
    SQRT2: 1.41421
};

module.exports = {
    constants,
    // 也可以混合使用
    add: exports.add,
    multiply: exports.multiply
};

// ===== ES6 模块示例 =====
// 注意: Amber 支持 ES6 模块语法
// 但需要使用 .mjs 扩展名或在 package.json 中设置 type: "module"

// es6-module.mjs 内容:
export const API_BASE = "https://api.example.com";

export class ApiClient {
    constructor(baseUrl) {
        this.baseUrl = baseUrl;
    }

    async get(endpoint) {
        console.log(`GET ${this}`);
        return { data: "mock.baseUrl}${endpoint response" };
    }

    async post(endpoint, data) {
        console.log(`POST ${this.baseUrl}${endpoint}`, data);
        return { success: true };
    }
}

export default ApiClient;

// ===== 主文件演示 =====
console.log("=== Amber 模块系统示例 ===\n");

// 导入 CommonJS 模块
const math = require('./math-module.js');
console.log("CommonJS - Addition:", math.add(10, 20));
console.log("CommonJS - Multiplication:", math.multiply(5, 6));
console.log("CommonJS - PI:", math.PI);

// 动态导入 (ES6 动态模块加载)
async function demonstrateDynamicImport() {
    console.log("\n--- 动态模块导入 ---");

    try {
        // 动态导入 ES6 模块
        const { API_BASE, ApiClient } = await import('./es6-module.mjs');

        console.log("API Base URL:", API_BASE);
        const client = new ApiClient(API_BASE);
        await client.get('/users');
        await client.post('/users', { name: 'John' });
    } catch (error) {
        console.log("Note: ES6 modules require .mjs extension");
        console.log("Dynamic import available for future modules");
    }
}

// 模块缓存示例
function demonstrateModuleCache() {
    console.log("\n--- 模块缓存演示 ---");

    // 第一次 require
    const start1 = performance.now();
    const math1 = require('./math-module.js');
    const time1 = performance.now() - start1;

    // 第二次 require (从缓存加载)
    const start2 = performance.now();
    const math2 = require('./math-module.js');
    const time2 = performance.now() - start2;

    console.log(`First load time: ${time2.toFixed(4)}ms`);
    console.log(`Cached load time: ${time2.toFixed(4)}ms`);
    console.log(`Cache speedup: ${(time1 / time2).toFixed(2)}x faster`);

    // 验证是同一个模块实例
    console.log("Same instance:", math1 === math2);
}

// 模块作用域演示
function demonstrateModuleScope() {
    console.log("\n--- 模块作用域演示 ---");

    // 每个模块都有自己的作用域
    const moduleA = require('./scoped-module-a.js');
    const moduleB = require('./scoped-module-b.js');

    console.log("Module A counter:", moduleA.getCounter());
    console.log("Module B counter:", moduleB.getCounter());
    console.log("Module A counter:", moduleA.getCounter()); // 独立的计数器
    console.log("Module B counter:", moduleB.getCounter());
}

// 执行演示
demonstrateModuleCache();
demonstrateModuleScope();

// 异步演示
demonstrateDynamicImport().then(() => {
    console.log("\n✅ 模块系统演示完成!");
});

// ===== 创建辅助模块文件 =====

// 创建 math-module.js
const fs = require('fs');
const path = require('path');

// 由于这是示例代码，我们直接在这里定义模块
// 在实际使用中，这些会作为独立文件存在
