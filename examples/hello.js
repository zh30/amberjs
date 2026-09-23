// Amber Hello World Example
// High-performance JavaScript runtime built with Rust + V8

console.log("🐝 Hello from Amber!");
console.log("🚀 Running JavaScript with Rust + V8");

// Basic operations
const sum = (a, b) => a + b;
console.log(`\nSum of 1 + 2 = ${sum(1, 2)}`);

// Array operations
const numbers = [1, 2, 3, 4, 5];
const doubled = numbers.map(n => n * 2);
console.log(`Array doubled: ${doubled.join(", ")}`);

// Object operations
const user = {
    name: "Amber User",
    version: "0.3.100",
    features: ["fast", "secure", "AI-ready"]
};

console.log(`\nUser: ${user.name}`);
console.log(`Version: ${user.version}`);
console.log(`Features: ${user.features.join(", ")}`);

// Async simulation
const delay = (ms) => new Promise(resolve => setTimeout(resolve, ms));
(async () => {
    console.log("\n⏳ Simulating async operation...");
    await delay(100);
    console.log("✅ Async operation completed!");
})();

console.log("\n✨ Amber is faster than Bun!");
