// Simple TypeScript Example for Amber
// Testing basic TypeScript compilation

// Basic types
let message: string = "Hello from TypeScript!";
let count: number = 42;
let isActive: boolean = true;

// Array types
let numbers: number[] = [1, 2, 3, 4, 5];
let names: string[] = ["Amber", "TypeScript", "Rust"];

// Function with types
function add(a: number, b: number): number {
    return a + b;
}

function greet(name: string): string {
    return "Hello, " + name + "!";
}

// Object type
interface User {
    name: string;
    age: number;
}

const user: User = {
    name: "Amber User",
    age: 1
};

// Execute
console.log("🐝 TypeScript Hello World for Amber!");
console.log(message);
console.log("Count: " + count);
console.log("Active: " + isActive);
console.log("\nNumbers: " + numbers.join(", "));
console.log("Names: " + names.join(", "));
console.log("\nadd(1, 2) = " + add(1, 2));
console.log(greet("Developer"));
console.log("\nUser: " + user.name + ", age " + user.age);
console.log("\n✨ TypeScript compilation works!");
