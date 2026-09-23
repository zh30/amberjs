// Amber JIT Optimizer Demo
// This script demonstrates the JIT optimization capabilities

// Simple code - should be optimized quickly
console.log("=== Simple Code Example ===");
let x = 1;
let y = 2;
let z = x + y;
console.log("Result:", z);

// Medium complexity - loops and conditions
console.log("\n=== Medium Complexity Example ===");
function calculateSum(n) {
    let sum = 0;
    for (let i = 0; i < n; i++) {
        if (i % 2 === 0) {
            sum += i;
        }
    }
    return sum;
}

console.log("Sum of even numbers up to 100:", calculateSum(100));

// Complex code - nested loops and recursion
console.log("\n=== Complex Code Example ===");
function fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

console.log("Fibonacci of 10:", fibonacci(10));

// Demonstrate hot path detection
console.log("\n=== Hot Path Example ===");
for (let i = 0; i < 20; i++) {
    let result = i * i;
    if (i % 5 === 0) {
        console.log(`Hot path: ${i}^2 = ${result}`);
    }
}

console.log("\n=== JIT Optimization Demo Complete ===");
console.log("The JIT optimizer will analyze this code and apply appropriate optimizations.");
