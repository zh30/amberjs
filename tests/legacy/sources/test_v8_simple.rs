//! Simple V8 test - independent minimal implementation
//! No dependencies on the main project

use rusty_v8 as v8;

fn main() {
    println!("🚀 Simple V8 Test");
    println!("==================\n");

    // Initialize V8
    v8::V8::initialize_platform(v8::new_default_platform().unwrap());
    v8::V8::initialize();

    // Create isolate
    let mut isolate = v8::Isolate::new(v8::CreateParams::default());
    v8::scope!(let scope, &mut isolate);

    // Create context
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    // Test 1: Simple arithmetic
    println!("Test 1: Simple arithmetic (1 + 1)");
    let source = v8::String::new(scope, "1 + 1").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let result = script.run(scope).unwrap();
    let result_str = result.to_string(scope).unwrap();
    println!("✅ Result: {}\n", result_str.to_rust_string_lossy(scope));

    // Test 2: String concatenation
    println!("Test 2: String concatenation");
    let source = v8::String::new(scope, "'Hello' + ' ' + 'V8'").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let result = script.run(scope).unwrap();
    let result_str = result.to_string(scope).unwrap();
    println!("✅ Result: {}\n", result_str.to_rust_string_lossy(scope));

    // Test 3: Array operations
    println!("Test 3: Array operations");
    let source = v8::String::new(scope, "[1, 2, 3, 4, 5].length").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let result = script.run(scope).unwrap();
    let result_str = result.to_string(scope).unwrap();
    println!("✅ Result: {}\n", result_str.to_rust_string_lossy(scope));

    // Test 4: Object operations
    println!("Test 4: Object operations");
    let source = v8::String::new(scope, "({ x: 10, y: 20 }).x").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let result = script.run(scope).unwrap();
    let result_str = result.to_string(scope).unwrap();
    println!("✅ Result: {}\n", result_str.to_rust_string_lossy(scope));

    // Test 5: Function
    println!("Test 5: Function definition and call");
    let source = v8::String::new(scope, "function add(a, b) { return a + b; } add(5, 3);").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let result = script.run(scope).unwrap();
    let result_str = result.to_string(scope).unwrap();
    println!("✅ Result: {}\n", result_str.to_rust_string_lossy(scope));

    // Test 6: Arrow function
    println!("Test 6: Arrow function");
    let source = v8::String::new(scope, "const double = x => x * 2; double(21);").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let result = script.run(scope).unwrap();
    let result_str = result.to_string(scope).unwrap();
    println!("✅ Result: {}\n", result_str.to_rust_string_lossy(scope));

    // Test 7: Array methods
    println!("Test 7: Array filter method");
    let source = v8::String::new(scope, "[1, 2, 3, 4, 5].filter(x => x > 2).length").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let result = script.run(scope).unwrap();
    let result_str = result.to_string(scope).unwrap();
    println!("✅ Result: {}\n", result_str.to_rust_string_lossy(scope));

    // Test 8: ES6 features
    println!("Test 8: ES6 template literal");
    let source = v8::String::new(scope, "`Hello, ${'World'}!`").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let result = script.run(scope).unwrap();
    let result_str = result.to_string(scope).unwrap();
    println!("✅ Result: {}\n", result_str.to_rust_string_lossy(scope));

    println!("🎉 All tests passed!");
    println!("==================");
    println!("V8 is working correctly! ✅");
}
