// Simple async function test without arrow functions
interface User {
    name: string;
    version: string;
}

function greet(user: User): string {
    return "Hello, " + user.name;
}

const user: User = {
    name: "Amber",
    version: "0.3.102"
};

console.log(greet(user));

async function fetchData(): string {
    return "Data loaded!";
}

console.log("Test passed!");
