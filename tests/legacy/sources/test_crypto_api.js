// Test Amber v0.1.7 Crypto API
console.log("🧪 测试 Amber v0.1.7 Crypto API");

// Test crypto object
console.log("\n1. Crypto Object 测试:");
console.log("typeof crypto =", typeof crypto);

// Test crypto.randomUUID
console.log("\n2. crypto.randomUUID() 测试:");
const uuid = crypto.randomUUID();
console.log("crypto.randomUUID() =", uuid);
console.log("UUID length:", uuid.length);

// Test crypto.getRandomValues
console.log("\n3. crypto.getRandomValues() 测试:");
const arr = new Uint8Array(5);
const result = crypto.getRandomValues(arr);
console.log("getRandomValues result:", result);

// Test crypto.subtle
console.log("\n4. crypto.subtle 对象测试:");
console.log("typeof crypto.subtle =", typeof crypto.subtle);

console.log("\n✅ Crypto API 测试完成!");
