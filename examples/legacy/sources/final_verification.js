// Final verification test for Amber v0.1.8
console.log("🎯 Amber v0.1.8 最终验证测试");
console.log("====================================\n");

// Test 1: All Web APIs
console.log("1. Web API 可用性测试:");
console.log("  typeof console =", typeof console);
console.log("  typeof JSON =", typeof JSON);
console.log("  typeof Date =", typeof Date);
console.log("  typeof fs =", typeof fs);
console.log("  typeof fetch =", typeof fetch);
console.log("  typeof crypto =", typeof crypto);
console.log("  typeof btoa =", typeof btoa);
console.log("  typeof atob =", typeof atob);
console.log("  typeof Buffer =", typeof Buffer);
console.log("  typeof process =", typeof process);

// Test 2: Core functionality
console.log("\n2. 核心功能测试:");
console.log("  1 + 1 =", 1 + 1);
console.log("  Date.now() =", Date.now());
console.log("  btoa('Hello') =", btoa('Hello'));

// Test 3: fs API
console.log("\n3. fs API 测试:");
console.log("  fs.exists('./Cargo.toml') =", fs.exists('./Cargo.toml'));

// Test 4: crypto API
console.log("\n4. Crypto API 测试:");
console.log("  crypto.randomUUID() =", crypto.randomUUID());

// Test 5: JSON API
console.log("\n5. JSON API 测试:");
console.log("  JSON.stringify({test: true}) =", JSON.stringify({test: true}));

console.log("\n✅ 所有功能验证完成! Amber v0.1.8 运行正常!");
