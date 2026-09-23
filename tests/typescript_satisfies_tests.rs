// satisfies 操作符测试 (v0.3.168)
// 测试 satisfies 语法支持 - 类型检查但不改变推断类型

#[cfg(test)]
mod typescript_satisfies_tests {
    use amberjs::typescript::compile_typescript;

    /// Test basic satisfies with simple type (v0.3.168)
    /// satisfies 检查类型兼容性，但保留原始推断类型
    #[test]
    fn test_satisfies_basic_number() {
        let ts_code = r#"
const x = 1 satisfies number;
console.log(x);
"#;
        match compile_typescript(ts_code, "satisfies_basic.ts") {
            Ok(output) => {
                println!("satisfies 基础转译结果:");
                println!("{}", output.js_code);
                // satisfies 断言应该被移除，但表达式应该保留
                assert!(
                    output.js_code.contains("const x"),
                    "Should contain const x: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                // 验证 satisfies 被移除
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Basic satisfies test passed");
            }
            Err(e) => {
                panic!("Basic satisfies test failed: {}", e);
            }
        }
    }

    /// Test satisfies with string literal (v0.3.168)
    #[test]
    fn test_satisfies_string_literal() {
        let ts_code = r#"
const greeting = "hello" satisfies string;
console.log(greeting);
"#;
        match compile_typescript(ts_code, "satisfies_string.ts") {
            Ok(output) => {
                println!("satisfies 字符串字面量转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const greeting"),
                    "Should contain const greeting: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Satisfies string literal test passed");
            }
            Err(e) => {
                panic!("Satisfies string literal test failed: {}", e);
            }
        }
    }

    /// Test satisfies with array type (v0.3.168)
    #[test]
    fn test_satisfies_array_type() {
        let ts_code = r#"
const nums = [1, 2, 3] satisfies number[];
console.log(nums);
"#;
        match compile_typescript(ts_code, "satisfies_array.ts") {
            Ok(output) => {
                println!("satisfies 数组类型转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const nums"),
                    "Should contain const nums: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains('1')
                        && output.js_code.contains('2')
                        && output.js_code.contains('3'),
                    "Should contain array elements: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Satisfies array type test passed");
            }
            Err(e) => {
                panic!("Satisfies array type test failed: {}", e);
            }
        }
    }

    /// Test satisfies with object type (v0.3.168)
    #[test]
    fn test_satisfies_object_type() {
        let ts_code = r#"
const point = { x: 10, y: 20 } satisfies { x: number; y: number };
console.log(point);
"#;
        match compile_typescript(ts_code, "satisfies_object.ts") {
            Ok(output) => {
                println!("satisfies 对象类型转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const point"),
                    "Should contain const point: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("x: 10"),
                    "Should contain x property: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("y: 20"),
                    "Should contain y property: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Satisfies object type test passed");
            }
            Err(e) => {
                panic!("Satisfies object type test failed: {}", e);
            }
        }
    }

    /// Test satisfies with generic type (v0.3.168)
    #[test]
    fn test_satisfies_generic_type() {
        let ts_code = r#"
const strings = ["a", "b", "c"] satisfies Array<string>;
console.log(strings);
"#;
        match compile_typescript(ts_code, "satisfies_generic.ts") {
            Ok(output) => {
                println!("satisfies 泛型类型转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const strings"),
                    "Should contain const strings: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Satisfies generic type test passed");
            }
            Err(e) => {
                panic!("Satisfies generic type test failed: {}", e);
            }
        }
    }

    /// Test satisfies in function return (v0.3.168)
    #[test]
    fn test_satisfies_in_function() {
        let ts_code = r#"
function getConfig() {
    return {
        timeout: 3000,
        retries: 3
    } satisfies { timeout: number; retries: number };
}
console.log(getConfig());
"#;
        match compile_typescript(ts_code, "satisfies_function.ts") {
            Ok(output) => {
                println!("satisfies 函数返回转译结果:");
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
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Satisfies in function test passed");
            }
            Err(e) => {
                panic!("Satisfies in function test failed: {}", e);
            }
        }
    }

    /// Test satisfies with boolean literal (v0.3.168)
    #[test]
    fn test_satisfies_boolean() {
        let ts_code = r#"
const flag = true satisfies boolean;
console.log(flag);
"#;
        match compile_typescript(ts_code, "satisfies_boolean.ts") {
            Ok(output) => {
                println!("satisfies 布尔类型转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const flag"),
                    "Should contain const flag: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("true"),
                    "Should contain true: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Satisfies boolean test passed");
            }
            Err(e) => {
                panic!("Satisfies boolean test failed: {}", e);
            }
        }
    }

    /// Test satisfies with complex nested object (v0.3.168)
    #[test]
    fn test_satisfies_nested_object() {
        let ts_code = r#"
const nested = {
    user: {
        name: "Alice",
        age: 30
    }
} satisfies { user: { name: string; age: number } };
console.log(nested);
"#;
        match compile_typescript(ts_code, "satisfies_nested.ts") {
            Ok(output) => {
                println!("satisfies 嵌套对象转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const nested"),
                    "Should contain const nested: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("user"),
                    "Should contain user property: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Satisfies nested object test passed");
            }
            Err(e) => {
                panic!("Satisfies nested object test failed: {}", e);
            }
        }
    }

    /// Test satisfies with union type (v0.3.168)
    #[test]
    fn test_satisfies_union_type() {
        let ts_code = r#"
const value = "hello" satisfies string | number;
console.log(value);
"#;
        match compile_typescript(ts_code, "satisfies_union.ts") {
            Ok(output) => {
                println!("satisfies 联合类型转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const value"),
                    "Should contain const value: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Satisfies union type test passed");
            }
            Err(e) => {
                panic!("Satisfies union type test failed: {}", e);
            }
        }
    }

    /// Test satisfies in array element (v0.3.168)
    #[test]
    fn test_satisfies_in_array() {
        let ts_code = r#"
const items = [
    { id: 1, name: "A" } satisfies { id: number; name: string },
    { id: 2, name: "B" } satisfies { id: number; name: string }
];
console.log(items);
"#;
        match compile_typescript(ts_code, "satisfies_array_elements.ts") {
            Ok(output) => {
                println!("satisfies 数组元素转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const items"),
                    "Should contain const items: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Satisfies in array test passed");
            }
            Err(e) => {
                panic!("Satisfies in array test failed: {}", e);
            }
        }
    }

    /// Test multiple satisfies in same scope (v0.3.168)
    #[test]
    fn test_multiple_satisfies() {
        let ts_code = r#"
const a = 1 satisfies number;
const b = "hello" satisfies string;
const c = [1, 2, 3] satisfies number[];
console.log(a, b, c);
"#;
        match compile_typescript(ts_code, "satisfies_multiple.ts") {
            Ok(output) => {
                println!("satisfies 多重转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const a"),
                    "Should contain const a: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("const b"),
                    "Should contain const b: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("const c"),
                    "Should contain const c: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Multiple satisfies test passed");
            }
            Err(e) => {
                panic!("Multiple satisfies test failed: {}", e);
            }
        }
    }

    /// Test satisfies with tuple type (v0.3.168)
    #[test]
    fn test_satisfies_tuple() {
        let ts_code = r#"
const tuple = [1, "two", true] satisfies [number, string, boolean];
console.log(tuple);
"#;
        match compile_typescript(ts_code, "satisfies_tuple.ts") {
            Ok(output) => {
                println!("satisfies 元组类型转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const tuple"),
                    "Should contain const tuple: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Satisfies tuple test passed");
            }
            Err(e) => {
                panic!("Satisfies tuple test failed: {}", e);
            }
        }
    }

    /// Test satisfies with interface type (v0.3.168)
    #[test]
    fn test_satisfies_interface() {
        let ts_code = r#"
interface User {
    name: string;
    age: number;
}

const user = { name: "Bob", age: 25 } satisfies User;
console.log(user);
"#;
        match compile_typescript(ts_code, "satisfies_interface.ts") {
            Ok(output) => {
                println!("satisfies 接口类型转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const user"),
                    "Should contain const user: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Satisfies interface test passed");
            }
            Err(e) => {
                panic!("Satisfies interface test failed: {}", e);
            }
        }
    }

    /// Test satisfies with type alias (v0.3.168)
    #[test]
    fn test_satisfies_type_alias() {
        let ts_code = r#"
type Point = { x: number; y: number };

const point = { x: 5, y: 10 } satisfies Point;
console.log(point);
"#;
        match compile_typescript(ts_code, "satisfies_type_alias.ts") {
            Ok(output) => {
                println!("satisfies 类型别名转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const point"),
                    "Should contain const point: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Satisfies type alias test passed");
            }
            Err(e) => {
                panic!("Satisfies type alias test failed: {}", e);
            }
        }
    }

    /// Test satisfies with Map type (v0.3.168)
    #[test]
    fn test_satisfies_map_type() {
        let ts_code = r#"
const map = new Map([["key", 42]]) satisfies Map<string, number>;
console.log(map);
"#;
        match compile_typescript(ts_code, "satisfies_map.ts") {
            Ok(output) => {
                println!("satisfies Map 类型转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const map"),
                    "Should contain const map: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Satisfies Map type test passed");
            }
            Err(e) => {
                panic!("Satisfies Map type test failed: {}", e);
            }
        }
    }

    /// Test satisfies with Promise type (v0.3.168)
    #[test]
    fn test_satisfies_promise() {
        let ts_code = r#"
const promise = Promise.resolve(42) satisfies Promise<number>;
console.log(promise);
"#;
        match compile_typescript(ts_code, "satisfies_promise.ts") {
            Ok(output) => {
                println!("satisfies Promise 类型转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const promise"),
                    "Should contain const promise: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Satisfies Promise test passed");
            }
            Err(e) => {
                panic!("Satisfies Promise test failed: {}", e);
            }
        }
    }

    /// Test satisfies preserves expression value (v0.3.168)
    /// 确保 satisfies 不会改变原始表达式的值
    #[test]
    fn test_satisfies_preserves_value() {
        let ts_code = r#"
const result = (1 + 2) * 3 satisfies number;
console.log(result);
"#;
        match compile_typescript(ts_code, "satisfies_expression.ts") {
            Ok(output) => {
                println!("satisfies 表达式转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const result"),
                    "Should contain const result: {}",
                    output.js_code
                );
                // 表达式应该被保留（不会被计算）
                assert!(
                    output.js_code.contains("(1 + 2) * 3") || output.js_code.contains("1 + 2 * 3"),
                    "Should contain expression: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Satisfies preserves value test passed");
            }
            Err(e) => {
                panic!("Satisfies preserves value test failed: {}", e);
            }
        }
    }

    /// Test mixed as const and satisfies (v0.3.168)
    #[test]
    fn test_mixed_as_const_and_satisfies() {
        let ts_code = r#"
const config = { debug: true } as const satisfies { debug: boolean };
console.log(config);
"#;
        match compile_typescript(ts_code, "satisfies_as_const.ts") {
            Ok(output) => {
                println!("satisfies 和 as const 混合转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const config"),
                    "Should contain const config: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Mixed as const and satisfies test passed");
            }
            Err(e) => {
                panic!("Mixed as const and satisfies test failed: {}", e);
            }
        }
    }

    /// Test satisfies in ternary expression (v0.3.168)
    #[test]
    fn test_satisfies_in_ternary() {
        let ts_code = r#"
const value = true ? 1 satisfies number : 2 satisfies number;
console.log(value);
"#;
        match compile_typescript(ts_code, "satisfies_ternary.ts") {
            Ok(output) => {
                println!("satisfies 三元表达式转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("const value"),
                    "Should contain const value: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("satisfies"),
                    "satisfies should be removed: {}",
                    output.js_code
                );
                println!("✅ Satisfies in ternary test passed");
            }
            Err(e) => {
                panic!("Satisfies in ternary test failed: {}", e);
            }
        }
    }
}
