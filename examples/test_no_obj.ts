// Test without object literal
interface User {
    name: string;
    version: string;
}

function greet(user: User): string {
    return "Hello";
}

function add(a: number, b: number): number {
    return a + b;
}

console.log("Hello World for Amber!");
console.log(greet({name: "Test", version: "1.0"}));
console.log(add(1, 2));

console.log("TypeScript support is working!");
