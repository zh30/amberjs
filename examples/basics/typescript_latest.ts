// Amber TypeScript 6.0 syntax smoke (transpile-only via oxc).
// Keep this executable without React or extra packages.

function id<const T>(value: T): T {
    return value;
}

const point = { x: 20, y: 22 } satisfies { x: number; y: number };

let disposed = false;
{
    using resource = {
        [Symbol.dispose]() {
            disposed = true;
        },
    };
    void resource;
}

console.log("const type param:", id(point.x) + point.y);
console.log("using disposed:", disposed);
console.log("TypeScript 6.0 syntax works!");
