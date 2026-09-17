// Simple test to verify core functionality
use amberjs::runtime_minimal::MinimalRuntime;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🐝 Testing Beejs Core Functionality");

    // Test 1: Create runtime
    let mut runtime = MinimalRuntime::new()?;
    println!("✅ Runtime created successfully");

    // Test 2: Execute simple code
    let result = runtime.execute_code("1 + 1")?;
    assert_eq!(result.trim(), "2");
    println!("✅ Simple arithmetic: 1 + 1 = {}", result.trim());

    // Test 3: Execute with console.log
    let result = runtime.execute_code("console.log('Hello from Beejs!'); 42;")?;
    assert_eq!(result.trim(), "42");
    println!("✅ Console.log test passed");

    // Test 4: Execute complex code
    let result = runtime.execute_code("let x = 5; let y = 10; x + y;")?;
    assert_eq!(result.trim(), "15");
    println!("✅ Complex expression: 5 + 10 = {}", result.trim());

    println!();
    println!("🎉 All core tests passed!");
    println!("Beejs is working correctly!");

    Ok(())
}
