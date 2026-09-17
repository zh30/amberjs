#[test]
fn test_debug_output() {
    use amberjs::Runtime;
    use tempfile::{NamedTempFile, TempDir};
    use std::io::Write;
    
    let runtime = Runtime::new(67108864, 1073741824, false).unwrap();
    
    let temp_dir = TempDir::new().unwrap();
    
    let module_file = temp_dir.path().join("math.js");
    std::fs::write(&module_file, "
        module.exports = {
            add: (a, b) => a + b,
            multiply: (a, b) => a * b
        };
    ").unwrap();
    
    let main_file = temp_dir.path().join("main.js");
    std::fs::write(&main_file, "
        const math = require('./math.js');
        console.log(math.add(5, 3));
        console.log(math.multiply(4, 7));
        math.add(5, 3)
    ").unwrap();
    
    let result = runtime.execute_file(&main_file);
    println!("Result: {:?}", result);
    if let Ok(output) = result {
        println!("Output: '{}'", output);
        println!("Output bytes: {:?}", output.as_bytes());
    }
}
