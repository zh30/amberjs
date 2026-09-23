// as const 类型断言测试 (v0.3.167)
// 测试 as const 语法支持

#[cfg(test)]
mod typescript_as_const_tests {
    use amberjs::typescript::compile_typescript;

    /// Test basic as const assertion (v0.3.167)
    /// as const 将表达式转换为字面量只读类型
    #[test]
    fn test_as_const_basic() {
        let ts_code = r#"
const obj = { x: 1, y: "hello" } as const;
console.log(obj);
"#;
        match compile_typescript(ts_code, "as_const_basic.ts") {
            Ok(output) => {
                println!("as const 基础转译结果:");
                println!("{}", output.js_code);
                // as const 断言应该被移除，但表达式应该保留
                assert!(
                    output.js_code.contains("const obj"),
                    "Should contain const obj: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                // 运行时应正确工作
                println!("✅ Basic as const test passed");
            }
            Err(e) => {
                panic!("Basic as const test failed: {}", e);
            }
        }
    }

    /// Test as const with array (v0.3.167)
    #[test]
    fn test_as_const_array() {
        let ts_code = r#"
const nums = [1, 2, 3] as const;
console.log(nums);
"#;
        match compile_typescript(ts_code, "as_const_array.ts") {
            Ok(output) => {
                println!("as const 数组转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const nums"),
                    "Should contain const nums: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ As const array test passed");
            }
            Err(e) => {
                panic!("As const array test failed: {}", e);
            }
        }
    }

    /// Test as const with nested object (v0.3.167)
    #[test]
    fn test_as_const_nested_object() {
        let ts_code = r#"
const nested = {
    a: {
        b: {
            c: 42
        }
    }
} as const;
console.log(nested);
"#;
        match compile_typescript(ts_code, "as_const_nested.ts") {
            Ok(output) => {
                println!("as const 嵌套对象转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const nested"),
                    "Should contain const nested: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ As const nested object test passed");
            }
            Err(e) => {
                panic!("As const nested object test failed: {}", e);
            }
        }
    }

    /// Test as const with string literal (v0.3.167)
    #[test]
    fn test_as_const_string_literal() {
        let ts_code = r#"
const greeting = "hello" as const;
console.log(greeting);
"#;
        match compile_typescript(ts_code, "as_const_string.ts") {
            Ok(output) => {
                println!("as const 字符串字面量转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const greeting"),
                    "Should contain const greeting: {}",
                    output.js_code
                );
                println!("✅ As const string literal test passed");
            }
            Err(e) => {
                panic!("As const string literal test failed: {}", e);
            }
        }
    }

    /// Test as const with number literal (v0.3.167)
    #[test]
    fn test_as_const_number_literal() {
        let ts_code = r#"
const num = 42 as const;
console.log(num);
"#;
        match compile_typescript(ts_code, "as_const_number.ts") {
            Ok(output) => {
                println!("as const 数字字面量转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const num"),
                    "Should contain const num: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("42"),
                    "Should contain 42: {}",
                    output.js_code
                );
                println!("✅ As const number literal test passed");
            }
            Err(e) => {
                panic!("As const number literal test failed: {}", e);
            }
        }
    }

    /// Test as const in function return (v0.3.167)
    #[test]
    fn test_as_const_in_function() {
        let ts_code = r#"
function getConfig() {
    return {
        timeout: 3000,
        retries: 3
    } as const;
}
console.log(getConfig());
"#;
        match compile_typescript(ts_code, "as_const_function.ts") {
            Ok(output) => {
                println!("as const 函数返回转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("function getConfig"),
                    "Should contain function: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ As const in function test passed");
            }
            Err(e) => {
                panic!("As const in function test failed: {}", e);
            }
        }
    }

    /// Test regular as type assertion still works (v0.3.167)
    #[test]
    fn test_as_type_assertion() {
        let ts_code = r#"
const value = someValue as string;
console.log(value);
"#;
        match compile_typescript(ts_code, "as_type.ts") {
            Ok(output) => {
                println!("as type 断言转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const value"),
                    "Should contain const value: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ As type assertion test passed");
            }
            Err(e) => {
                panic!("As type assertion test failed: {}", e);
            }
        }
    }

    /// Test as const with tuple (v0.3.167)
    #[test]
    fn test_as_const_tuple() {
        let ts_code = r#"
const tuple = [1, "two", true] as const;
console.log(tuple);
"#;
        match compile_typescript(ts_code, "as_const_tuple.ts") {
            Ok(output) => {
                println!("as const 元组转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const tuple"),
                    "Should contain const tuple: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ As const tuple test passed");
            }
            Err(e) => {
                panic!("As const tuple test failed: {}", e);
            }
        }
    }

    /// Test as const with enum-like object (v0.3.167)
    #[test]
    fn test_as_const_enum_like() {
        let ts_code = r##"
const Colors = {
    Red: "#ff0000",
    Green: "#00ff00",
    Blue: "#0000ff"
} as const;
console.log(Colors);
"##;
        match compile_typescript(ts_code, "as_const_enum.ts") {
            Ok(output) => {
                println!("as const 枚举对象转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const Colors"),
                    "Should contain const Colors: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("Red"),
                    "Should contain Red: {}",
                    output.js_code
                );
                println!("✅ As const enum-like test passed");
            }
            Err(e) => {
                panic!("As const enum-like test failed: {}", e);
            }
        }
    }
}
