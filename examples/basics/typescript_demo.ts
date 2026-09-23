// Amber TypeScript smoke example.
// Transpiled by oxc (TypeScript 6.0 syntax, transpile-only) before V8.

let message: string = "Hello from TypeScript!";
let count: number = 42;
let isActive: boolean = true;
let numbers: number[] = [1, 2, 3, 4, 5];

function add(a: number, b: number): number {
    return a + b;
}

function greet(name: string): string {
    return "Hello, " + name + "!";
}

console.log("TypeScript Hello World for Amber!");
console.log(message);
console.log("Count: " + count);
console.log("Active: " + isActive);
console.log("Numbers: " + numbers.join(", "));
console.log("add(1, 2) = " + add(1, 2));
console.log(greet("Developer"));
console.log("TypeScript compilation works!");
