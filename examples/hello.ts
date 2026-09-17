// TypeScript Hello World Example for Amber
// Testing TypeScript compilation support

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

console.log("🐝 TypeScript Hello World for Amber!");
console.log(greet(user));

// Arrow functions with types
const add = (a: number, b: number): number => a + b;
console.log(`\nSum: 1 + 2 = ${add(1, 2)}`);

// Generic function
function identity<T>(value: T): T {
    return value;
}

console.log(`Generic identity("test") = ${identity("test")}`);
console.log(`Generic identity(42) = ${identity(42)}`);

// Async/await
async function fetchData(): Promise<string> {
    return new Promise(resolve => {
        setTimeout(() => {
            resolve("Data loaded!");
        }, 50);
    });
}

(async () => {
    console.log("\n⏳ Loading data...");
    const data = await fetchData();
    console.log(`✅ ${data}`);
})();

console.log("\n✨ TypeScript support is working!");
