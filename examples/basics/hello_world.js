// Module: examples/basics/hello_world.js
// Hello World example for Amber runtime
console.log("Hello from Amber!");
console.log("This is a JavaScript/TypeScript runtime built with Rust and V8");

// Basic arithmetic
const a = 10;
const b = 20;
console.log(`Sum: ${a} + ${b} = ${a + b}`);

// Function example
function greet(name) {
    return `Hello, ${name}!`;
}

console.log(greet("Amber"));

// Object example
const user = {
    name: "Developer",
    role: "JavaScript Engineer",
    language: "TypeScript"
};

console.log("User:", user);

// Array example
const numbers = [1, 2, 3, 4, 5];
console.log("Numbers:", numbers);
console.log("Sum of numbers:", numbers.reduce((sum, n) => sum + n, 0));

