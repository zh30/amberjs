// Watch mode test - try editing this file and see it reload!
let counter = 0;

console.log("🔄 Watch mode test started");
console.log(`Initial counter: ${counter}`);

// Simulate some work
const data = {
    timestamp: new Date().toISOString(),
    message: "Amber hot reload is working!"
};

console.log("Data:", JSON.stringify(data, null, 2));
console.log("\nEdit this file and save to trigger hot reload!");
