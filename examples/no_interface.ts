// Test without interfaces
function greet(name: string): string {
    return "Hello, " + name;
}

console.log(greet("Amber"));

async function fetchData(): string {
    return "Data loaded!";
}

console.log("Test passed!");
