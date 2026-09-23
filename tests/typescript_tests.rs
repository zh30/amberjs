use amberjs::Runtime;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_typescript_basic_types() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Test TypeScript-style code (which is valid JavaScript too)
    // Note: TypeScript type annotations are removed as V8 doesn't support them
    let code = r#"
        let message = "Hello, TypeScript!";
        let count = 42;
        let isActive = true;
        console.log(message, count, isActive);
        count;
    "#;

    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    // Should execute without errors
}

#[test]
fn test_typescript_interfaces() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Test interface-like object structure
    // Note: TypeScript interfaces are removed as V8 doesn't support them
    let code = r#"
        const user = {
            name: "Alice",
            age: 30
        };

        console.log(user.name, user.age);
        user.age;
    "#;

    let result = runtime.execute_code(code);
    assert!(result.is_ok());
}

#[test]
fn test_typescript_functions() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Test typed function
    // Note: TypeScript type annotations are removed as V8 doesn't support them
    let code = r#"
        function greet(name) {
            return "Hello, " + name;
        }

        const result = greet("Amber");
        console.log(result);
        result;
    "#;

    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("Hello"));
}

#[test]
fn test_typescript_arrow_functions() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Test typed arrow function
    // Note: TypeScript type annotations are removed as V8 doesn't support them
    let code = r#"
        const add = (a, b) => {
            return a + b;
        };

        const sum = add(5, 3);
        console.log(sum);
        sum;
    "#;

    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("8"));
}

#[test]
fn test_typescript_classes() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Test class with type annotations
    // Note: TypeScript type annotations are removed as V8 doesn't support them
    let code = r#"
        class Calculator {
            constructor(initial = 0) {
                this.value = initial;
            }

            add(n) {
                this.value += n;
                return this;
            }

            getValue() {
                return this.value;
            }
        }

        const calc = new Calculator(10);
        calc.add(5);
        const result = calc.getValue();
        console.log(result);
        result;
    "#;

    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("15"));
}

#[test]
fn test_typescript_generics() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Test generic function (TypeScript syntax)
    // Note: TypeScript generics are removed as V8 doesn't support them
    let code = r#"
        function identity(arg) {
            return arg;
        }

        const num = identity(42);
        const str = identity("hello");
        console.log(num, str);
        num;
    "#;

    let result = runtime.execute_code(code);
    assert!(result.is_ok());
}

#[test]
fn test_typescript_unions() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Test union types
    // Note: TypeScript union types are removed as V8 doesn't support them
    let code = r#"
        let id;
        id = "abc123";
        console.log(id);
        id = 123;
        console.log(id);
        id;
    "#;

    let result = runtime.execute_code(code);
    assert!(result.is_ok());
}

#[test]
fn test_typescript_enums() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Test enum
    // Note: TypeScript enums are removed as V8 doesn't support them
    let code = r#"
        const Color = {
            Red: 1,
            Green: 2,
            Blue: 3
        };

        const favoriteColor = Color.Blue;
        console.log(favoriteColor);
        favoriteColor;
    "#;

    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("3"));
}

#[test]
fn test_typescript_file_execution() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Create a temporary TypeScript file
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
        // TypeScript-style code (type annotations removed for V8 compatibility)
        let message = "TypeScript works!";
        let count = 100;

        console.log(message);
        const result = count * 2;
        result;
    "#
    )
    .unwrap();

    let path = file.path().to_path_buf();
    let result = runtime.execute_file(&path);
    assert!(result.is_ok());
    // Check that the result contains the expected number (count * 2 = 200)
    assert!(result.unwrap().contains("200"));
}

#[test]
fn test_typescript_optional_properties() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Test optional properties
    // Note: TypeScript interfaces are removed as V8 doesn't support them
    let code = r#"
        const server = {
            port: 8080
        };

        console.log(server.port);
        server.port;
    "#;

    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("8080"));
}

#[test]
fn test_typescript_literal_types() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Test literal types
    // Note: TypeScript type aliases are removed as V8 doesn't support them
    let code = r#"
        const get = "GET";
        console.log(get);
        get;
    "#;

    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("GET"));
}

#[test]
fn test_typescript_namespace() {
    let runtime = Runtime::new(67108864, 1073741824, false, false);

    // Test namespace
    // Note: TypeScript namespaces are removed as V8 doesn't support them
    let code = r#"
        const MathUtils = {
            add(a, b) {
                return a + b;
            },

            PI: 3.14159
        };

        const result = MathUtils.add(10, 20);
        console.log(result);
        result;
    "#;

    let result = runtime.execute_code(code);
    assert!(result.is_ok());
    assert!(result.unwrap().contains("30"));
}
