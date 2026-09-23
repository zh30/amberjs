// This example demonstrates the inline cache functionality of Amber.
// The amberjs.getProperty function is provided by the runtime and uses the inline cache.

const obj = { name: "Amber", type: "runtime", version: "0.1.0" };

// Accessing the 'name' property multiple times should benefit from the inline cache
console.log("Name:", amberjs.getProperty(obj, "name"));
console.log("Name again:", amberjs.getProperty(obj, "name"));

// Accessing a different property
console.log("Type:", amberjs.getProperty(obj, "type"));

// Accessing a non-existent property
console.log("Author:", amberjs.getProperty(obj, "author"));

console.log("Amber inline cache example completed.");
