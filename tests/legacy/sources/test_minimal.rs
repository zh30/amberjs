//! Minimal test for Amber Runtime
//! Simple test to verify core functionality

fn main() {
    println!("🚀 Amber MinimalRuntime Test");
    println!("==============================\n");

    // Test 1: Create runtime
    println!("Test 1: Creating minimal runtime...");
    match amberjs::MinimalRuntime::new() {
        Ok(runtime) => {
            println!("✅ Runtime created successfully\n");

            // Test 2: Execute simple JavaScript
            println!("Test 2: Executing '1 + 1'...");
            match runtime.execute("1 + 1") {
                Ok(result) => {
                    println!("✅ Result: {}\n", result);

                    // Test 3: Execute string concatenation
                    println!("Test 3: Executing string concatenation...");
                    match runtime.execute("'Hello' + ' ' + 'Amber'") {
                        Ok(result) => {
                            println!("✅ Result: {}\n", result);

                            // Test 4: Execute array operations
                            println!("Test 4: Executing array operations...");
                            match runtime.execute("[1, 2, 3].length") {
                                Ok(result) => {
                                    println!("✅ Result: {}\n", result);

                                    // Test 5: Execute object operations
                                    println!("Test 5: Executing object operations...");
                                    match runtime.execute("({ x: 10, y: 20 }).x") {
                                        Ok(result) => {
                                            println!("✅ Result: {}\n", result);

                                            // Test 6: Execute function
                                            println!("Test 6: Executing function...");
                                            match runtime.execute(
                                                "function add(a, b) { return a + b; } add(5, 10);",
                                            ) {
                                                Ok(result) => {
                                                    println!("✅ Result: {}\n", result);

                                                    // Test 7: Execute arrow function
                                                    println!("Test 7: Executing arrow function...");
                                                    match runtime.execute(
                                                        "const double = x => x * 2; double(21);",
                                                    ) {
                                                        Ok(result) => {
                                                            println!("✅ Result: {}\n", result);

                                                            // Test 8: Execute array methods
                                                            println!("Test 8: Executing array methods...");
                                                            match runtime.execute("[1, 2, 3, 4, 5].filter(x => x > 2).length") {
                                                                Ok(result) => {
                                                                    println!("✅ Result: {}\n", result);

                                                                    println!("🎉 All tests passed!");
                                                                    println!("==============================");
                                                                }
                                                                Err(e) => println!("❌ Test 8 failed: {}", e),
                                                            }
                                                        }
                                                        Err(e) => {
                                                            println!("❌ Test 7 failed: {}", e)
                                                        }
                                                    }
                                                }
                                                Err(e) => println!("❌ Test 6 failed: {}", e),
                                            }
                                        }
                                        Err(e) => println!("❌ Test 5 failed: {}", e),
                                    }
                                }
                                Err(e) => println!("❌ Test 4 failed: {}", e),
                            }
                        }
                        Err(e) => println!("❌ Test 3 failed: {}", e),
                    }
                }
                Err(e) => println!("❌ Test 2 failed: {}", e),
            }
        }
        Err(e) => {
            println!("❌ Failed to create runtime: {}", e);
            std::process::exit(1);
        }
    }
}
