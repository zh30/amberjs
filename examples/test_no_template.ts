// Test without template strings
interface User {
    name: string;
    version: string;
}

function greet(user: User): string {
    return "Hello, " + user.name + "! Version: " + user.version;
}

const user: User = {
    name: "Amber",
    version: "0.3.102"
};

console.log("Hello World for Amber!");
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
console.log("Generic test = " + identity("test"));
console.log("Generic 42 = " + identity(42));

console.log("TypeScript support is working!");
