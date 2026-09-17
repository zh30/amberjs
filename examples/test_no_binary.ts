// Test without binary expressions
interface User {
    name: string;
    version: string;
}

function greet(user: User): string {
    return "Hello, " + user.name + "! Version: " + user.version;
}

function add(a: number, b: number): number {
    return a + b;
}

const user: User = {
    name: "Amber",
    version: "0.3.102"
};

const sum = add(1, 2);

console.log("Hello World for Amber!");
console.log(greet(user));
console.log(sum);

console.log("TypeScript support is working!");
