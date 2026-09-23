// 测试 Amber v0.1.7 核心功能
console.log("🧪 测试 Amber v0.1.7 核心功能");

// 1. 测试基础 JavaScript
console.log("\n1. 基础 JavaScript 测试:");
console.log("1 + 1 =", 1 + 1);
console.log("'Hello ' + 'World' =", 'Hello ' + 'World');
console.log("[1, 2, 3].length =", [1, 2, 3].length);

// 2. 测试 JSON API
console.log("\n2. JSON API 测试:");
console.log("JSON.stringify({name: 'test', value: 42}) =", JSON.stringify({name: 'test', value: 42}));
console.log("JSON.stringify([1, 'hello', true, null]) =", JSON.stringify([1, 'hello', true, null]));
console.log("JSON.stringify({nested: {x: 10, y: 20}}) =", JSON.stringify({nested: {x: 10, y: 20}}));

// 3. 测试 fs API (如果可用)
console.log("\n3. fs API 测试:");
try {
    console.log("typeof fs =", typeof fs);
    console.log("fs.exists('./Cargo.toml') =", fs.exists('./Cargo.toml'));
} catch (e) {
    console.log("fs API 不可用:", e.message);
}

// 4. 测试 Date API
console.log("\n4. Date API 测试:");
console.log("Date.now() =", Date.now());
console.log("new Date().toISOString() =", new Date().toISOString());

// 5. 测试 Base64 API
console.log("\n5. Base64 API 测试:");
console.log("btoa('Hello') =", btoa('Hello'));

console.log("\n✅ 所有核心功能测试完成!");
