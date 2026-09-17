// Test without generic functions
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

console.log("TypeScript support is working!");
