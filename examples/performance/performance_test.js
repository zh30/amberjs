// Performance test for Amber runtime
// This script tests various JavaScript operations

console.log("Starting Amber Performance Test...");

// Test 1: Simple arithmetic
console.time("Arithmetic Test");
for (let i = 0; i < 1000000; i++) {
    const result = (i * 2 + 10) / 5 - 3;
}
console.timeEnd("Arithmetic Test");

// Test 2: String operations
console.time("String Test");
let str = "";
for (let i = 0; i < 100000; i++) {
    str += "test" + i + " ";
}
console.timeEnd("String Test");

// Test 3: Array operations
console.time("Array Test");
const arr = [];
for (let i = 0; i < 100000; i++) {
    arr.push(i);
}
arr.sort((a, b) => a - b);
console.timeEnd("Array Test");

// Test 4: Object operations
console.time("Object Test");
const obj = {};
for (let i = 0; i < 100000; i++) {
    obj[`key${i}`] = i * 2;
}
console.timeEnd("Object Test");

// Test 5: Function calls
console.time("Function Test");
function fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}
const fibResult = fibonacci(10);
console.timeEnd("Function Test");

// Test 6: Async operations
console.time("Async Test");
Promise.resolve().then(() => {
    for (let i = 0; i < 100000; i++) {
        Math.random();
    }
});
console.timeEnd("Async Test");

console.log("Performance test completed!");
console.log("Fibonacci result:", fibResult);
