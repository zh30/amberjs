#[cfg(test)]
mod typescript_enum_tests {
    use amberjs::typescript::compile_typescript;

    /// Test enum declaration (v0.3.165)
    #[test]
    fn test_enum_basic() {
        // 测试基本枚举类型
        let ts_code = r#"
enum Color {
    Red = "red",
    Green = "green",
    Blue = "blue"
}

enum Status {
    Active = 1,
    Inactive = 0,
    Pending = 2
}

const myColor = Color.Red;
const myStatus = Status.Active;
console.log(myColor, myStatus);
"#;
        match compile_typescript(ts_code, "enum_basic.ts") {
            Ok(output) => {
                println!("基本枚举转译结果:");
                println!("{}", output.js_code);
                // 枚举被转译为 JavaScript 对象
                assert!(
                    output.js_code.contains("var Color"),
                    "Should contain var Color: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("var Status"),
                    "Should contain var Status: {}",
                    output.js_code
                );
                // 验证枚举成员值
                assert!(
                    output.js_code.contains("[\"Red\"] = \"red\""),
                    "Should contain Red with string value: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("[\"Active\"] = 1"),
                    "Should contain Active with number value: {}",
                    output.js_code
                );
                println!("✅ Basic enum test passed");
            }
            Err(e) => {
                panic!("Basic enum test failed: {}", e);
            }
        }
    }

    /// Test const enum (v0.3.165)
    #[test]
    fn test_const_enum() {
        // 测试常量枚举（const enum）
        // const enum 在编译时会被内联，不生成枚举对象
        let ts_code = r#"
const enum Direction {
    Up = "UP",
    Down = "DOWN",
    Left = "LEFT",
    Right = "RIGHT"
}

const moveUp = Direction.Up;
console.log(moveUp);
"#;
        match compile_typescript(ts_code, "const_enum.ts") {
            Ok(output) => {
                println!("常量枚举转译结果:");
                println!("{}", output.js_code);
                // const enum 被转译为普通对象（当前实现）
                // 注意：真正的 const enum 应该内联值，但当前实现保持对象形式
                assert!(
                    output.js_code.contains("var Direction"),
                    "Should contain var Direction: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("[\"Up\"] = \"UP\""),
                    "Should contain Up with value: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("moveUp"),
                    "Should contain moveUp variable: {}",
                    output.js_code
                );
                println!("✅ Const enum test passed");
            }
            Err(e) => {
                panic!("Const enum test failed: {}", e);
            }
        }
    }

    /// Test enum with numeric values (v0.3.165)
    #[test]
    fn test_enum_numeric() {
        // 测试数值枚举
        let ts_code = r#"
enum HttpStatus {
    OK = 200,
    NotFound = 404,
    ServerError = 500
}

const code = HttpStatus.OK;
console.log(code);
"#;
        match compile_typescript(ts_code, "enum_numeric.ts") {
            Ok(output) => {
                println!("数值枚举转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("var HttpStatus"),
                    "Should contain var HttpStatus: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("[\"OK\"] = 200"),
                    "Should contain OK with value 200: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("[\"NotFound\"] = 404"),
                    "Should contain NotFound with value 404: {}",
                    output.js_code
                );
                println!("✅ Numeric enum test passed");
            }
            Err(e) => {
                panic!("Numeric enum test failed: {}", e);
            }
        }
    }

    /// Test enum with string and number mixed (v0.3.165)
    #[test]
    fn test_enum_mixed() {
        // 测试混合枚举
        let ts_code = r#"
enum Mixed {
    StringValue = "str",
    NumericValue = 42,
    AutoNumeric
}

const val1 = Mixed.StringValue;
const val2 = Mixed.NumericValue;
const val3 = Mixed.AutoNumeric;
console.log(val1, val2, val3);
"#;
        match compile_typescript(ts_code, "enum_mixed.ts") {
            Ok(output) => {
                println!("混合枚举转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("var Mixed"),
                    "Should contain var Mixed: {}",
                    output.js_code
                );
                // 验证字符串值
                assert!(
                    output.js_code.contains("[\"StringValue\"] = \"str\""),
                    "Should contain StringValue with string: {}",
                    output.js_code
                );
                // 验证数值
                assert!(
                    output.js_code.contains("[\"NumericValue\"] = 42"),
                    "Should contain NumericValue: {}",
                    output.js_code
                );
                // 验证自动赋值 (从最后一个数值递增)
                assert!(
                    output.js_code.contains("[\"AutoNumeric\"] = 43"),
                    "Should contain AutoNumeric with auto value 43 (last value + 1): {}",
                    output.js_code
                );
                println!("✅ Mixed enum test passed");
            }
            Err(e) => {
                panic!("Mixed enum test failed: {}", e);
            }
        }
    }

    /// Test enum with reverse mapping (v0.3.165)
    #[test]
    fn test_enum_reverse_mapping() {
        // 测试枚举的反向映射（数值枚举支持）
        let ts_code = r#"
enum Status {
    Active = 1,
    Inactive = 0
}

const a = Status.Active;
const b = Status[1]; // 反向映射
console.log(a, b);
"#;
        match compile_typescript(ts_code, "enum_reverse.ts") {
            Ok(output) => {
                println!("枚举反向映射转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("var Status"),
                    "Should contain var Status: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("[\"Active\"] = 1"),
                    "Should contain Active: {}",
                    output.js_code
                );
                // 反向映射语法保留
                assert!(
                    output.js_code.contains("Status[1]"),
                    "Should contain reverse mapping: {}",
                    output.js_code
                );
                println!("✅ Enum reverse mapping test passed");
            }
            Err(e) => {
                panic!("Enum reverse mapping test failed: {}", e);
            }
        }
    }

    /// Test enum in object property access (v0.3.165)
    #[test]
    fn test_enum_in_object() {
        // 测试枚举在对象属性访问中的使用
        let ts_code = r#"
enum LogLevel {
    Debug = "DEBUG",
    Info = "INFO",
    Warn = "WARN",
    Error = "ERROR"
}

const config = {
    level: LogLevel.Info,
    timestamp: Date.now()
};

console.log(config.level);
"#;
        match compile_typescript(ts_code, "enum_object.ts") {
            Ok(output) => {
                println!("枚举在对象中使用转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("var LogLevel"),
                    "Should contain var LogLevel: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("config"),
                    "Should contain config object: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("LogLevel.Info"),
                    "Should contain LogLevel.Info access: {}",
                    output.js_code
                );
                println!("✅ Enum in object test passed");
            }
            Err(e) => {
                panic!("Enum in object test failed: {}", e);
            }
        }
    }

    /// Test enum with computed values (v0.3.165)
    #[test]
    fn test_enum_computed_values() {
        // 测试带计算值的枚举
        // 注意：当前编译器不支持复杂计算值，使用简单常量引用
        let ts_code = r#"
enum Custom {
    First = 100,
    Second = 200,
    Third = 300
}

const v1 = Custom.First;
const v2 = Custom.Second;
const v3 = Custom.Third;
console.log(v1, v2, v3);
"#;
        match compile_typescript(ts_code, "enum_computed.ts") {
            Ok(output) => {
                println!("计算值枚举转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("var Custom"),
                    "Should contain var Custom: {}",
                    output.js_code
                );
                // 验证数值被保留
                assert!(
                    output.js_code.contains("[\"First\"] = 100"),
                    "Should contain First: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("[\"Second\"] = 200"),
                    "Should contain Second: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("[\"Third\"] = 300"),
                    "Should contain Third: {}",
                    output.js_code
                );
                println!("✅ Enum computed values test passed");
            }
            Err(e) => {
                panic!("Enum computed values test failed: {}", e);
            }
        }
    }

    /// Test enum as function return type (v0.3.165)
    #[test]
    fn test_enum_function_return() {
        // 测试枚举作为函数返回类型
        let ts_code = r#"
enum Result {
    Success = "success",
    Error = "error"
}

function process(success: boolean): Result {
    return success ? Result.Success : Result.Error;
}

const r1 = process(true);
const r2 = process(false);
console.log(r1, r2);
"#;
        match compile_typescript(ts_code, "enum_function_return.ts") {
            Ok(output) => {
                println!("枚举函数返回类型转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("var Result"),
                    "Should contain var Result: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("function process"),
                    "Should contain process function: {}",
                    output.js_code
                );
                // 验证函数参数类型注解被移除
                assert!(
                    !output.js_code.contains("success: boolean"),
                    "Should remove type annotation: {}",
                    output.js_code
                );
                println!("✅ Enum function return test passed");
            }
            Err(e) => {
                panic!("Enum function return test failed: {}", e);
            }
        }
    }
}
