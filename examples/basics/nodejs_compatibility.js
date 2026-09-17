// 测试 Amber 的 Node.js 兼容性
console.log("=== Amber Node.js 兼容性测试 ===\n");

// 测试 process 对象
console.log("1. Process 对象测试:");
console.log("   process.version:", process.version);
console.log("   process.argv 长度:", process.argv.length);
console.log("   process.argv:", process.argv);

// 测试 path 模块
console.log("\n2. Path 模块测试:");
console.log("   path.join('/a', 'b', 'c'):", path.join('/a', 'b', 'c'));
console.log("   path.resolve('/a', 'b'):", path.resolve('/a', 'b'));

// 测试环境变量
console.log("\n3. 环境变量测试:");
console.log("   NODE_ENV 存在:", 'NODE_ENV' in process.env);

// 测试完成
console.log("\n=== Node.js 兼容性测试完成 ===");
