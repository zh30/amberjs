// Final test - copy of hello.ts without async parts
interface User {
    name: string;
    version: string;
    features: string[];
}

function greet(user: User): string {
    return `Hello, ${user.name}! Version: ${user.version}`;
}

const user: User = {
    name: "Amber TypeScript",
    version: "0.3.102",
    features: ["fast", "type-safe", "AI-ready"]
};

console.log("TypeScript Hello World for Amber!");
console.log(greet(user));

// Arrow functions with types
const add = (a: number, b: number): number => {
    return a + b;
};
console.log("Sum: 1 + 2 = " + add(1, 2));

// Generic function
function identity<T>(value: T): T {
    return value;
}
console.log("Generic identity(\"test\") = " + identity("test"));
console.log("Generic identity(42) = " + identity(42));

console.log("TypeScript support is working!");
