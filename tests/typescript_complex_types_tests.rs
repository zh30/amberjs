// 复杂类型注解测试 (v0.3.169)
// 测试对象类型、交叉类型、泛型类型等复杂类型注解的处理

#[cfg(test)]
mod typescript_complex_types_tests {
    use amberjs::typescript::compile_typescript;

    /// Test nested object type in satisfies (v0.3.169)
    /// 嵌套对象类型应该正确处理
    #[test]
    fn test_nested_object_type() {
        let ts_code = r#"
const config = {
    database: {
        host: "localhost",
        port: 5432
    }
} satisfies { database: { host: string; port: number } };
console.log(config);
"#;
        match compile_typescript(ts_code, "nested_object.ts") {
            Ok(output) => {
                println!("嵌套对象类型转译结果:");
                println!("{}", output.js_code);
                // 应该保留对象结构，移除类型注解
                assert!(
                    output.js_code.contains("database"),
                    "Should contain database: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("host"),
                    "Should contain host: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("port"),
                    "Should contain port: {}",
                    output.js_code
                );
                // 类型注解应该被移除
                assert!(
                    !output.js_code.contains("string"),
                    "Should not contain string type annotation: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("number"),
                    "Should not contain number type annotation: {}",
                    output.js_code
                );
                println!("✅ Nested object type test passed");
            }
            Err(e) => {
                panic!("Nested object type test failed: {}", e);
            }
        }
    }

    /// Test intersection type (v0.3.169)
    /// 交叉类型 A & B 应该被正确处理
    #[test]
    fn test_intersection_type() {
        let ts_code = r#"
interface A {
    a: string;
}
interface B {
    b: number;
}
const value = { a: "hello" } satisfies A & B;
console.log(value);
"#;
        match compile_typescript(ts_code, "intersection.ts") {
            Ok(output) => {
                println!("交叉类型转译结果:");
                println!("{}", output.js_code);
                // 应该保留对象，移除交叉类型注解
                assert!(
                    output.js_code.contains("a:"),
                    "Should contain a property: {}",
                    output.js_code
                );
                println!("✅ Intersection type test passed");
            }
            Err(e) => {
                panic!("Intersection type test failed: {}", e);
            }
        }
    }

    /// Test generic function type (v0.3.169)
    /// 泛型函数类型应该被正确处理
    #[test]
    fn test_generic_function_type() {
        let ts_code = r#"
function identity<T>(value: T): T {
    return value;
}
const result = identity(42);
console.log(result);
"#;
        match compile_typescript(ts_code, "generic_function.ts") {
            Ok(output) => {
                println!("泛型函数转译结果:");
                println!("{}", output.js_code);
                // 应该保留函数结构，移除类型注解
                assert!(
                    output.js_code.contains("function identity"),
                    "Should contain identity function: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("return value"),
                    "Should contain return statement: {}",
                    output.js_code
                );
                // 泛型参数应该被移除
                assert!(
                    !output.js_code.contains("<T>"),
                    "Should not contain generic parameter: {}",
                    output.js_code
                );
                println!("✅ Generic function type test passed");
            }
            Err(e) => {
                panic!("Generic function type test failed: {}", e);
            }
        }
    }

    /// Test mapped type (v0.3.169)
    /// 映射类型应该被正确处理
    #[test]
    fn test_mapped_type() {
        let ts_code = r#"
type Readonly<T> = { readonly [P in keyof T]: T[P] };
const obj = { x: 1, y: 2 } satisfies Readonly<{ x: number; y: number }>;
console.log(obj);
"#;
        match compile_typescript(ts_code, "mapped_type.ts") {
            Ok(output) => {
                println!("映射类型转译结果:");
                println!("{}", output.js_code);
                // 应该保留对象，移除类型注解
                assert!(
                    output.js_code.contains("x: 1"),
                    "Should contain x property: {}",
                    output.js_code
                );
                println!("✅ Mapped type test passed");
            }
            Err(e) => {
                panic!("Mapped type test failed: {}", e);
            }
        }
    }

    /// Test conditional type (v0.3.169)
    /// 条件类型应该被正确处理
    #[test]
    fn test_conditional_type() {
        let ts_code = r#"
type IsString<T> = T extends string ? true : false;
const result = IsString<string>;
console.log(result);
"#;
        match compile_typescript(ts_code, "conditional_type.ts") {
            Ok(output) => {
                println!("条件类型转译结果:");
                println!("{}", output.js_code);
                // 类型别名应该被移除
                assert!(
                    !output.js_code.contains("type IsString"),
                    "Should not contain type alias: {}",
                    output.js_code
                );
                println!("✅ Conditional type test passed");
            }
            Err(e) => {
                panic!("Conditional type test failed: {}", e);
            }
        }
    }

    /// Test template literal type (v0.3.169)
    /// 模板字面量类型应该被正确处理
    #[test]
    fn test_template_literal_type() {
        let ts_code = r#"
type Email = `${string}@${string}.${string}`;
const email = "user@example.com" satisfies Email;
console.log(email);
"#;
        match compile_typescript(ts_code, "template_literal.ts") {
            Ok(output) => {
                println!("模板字面量类型转译结果:");
                println!("{}", output.js_code);
                // 应该保留原始表达式
                assert!(
                    output.js_code.contains("user@example.com"),
                    "Should contain email value: {}",
                    output.js_code
                );
                println!("✅ Template literal type test passed");
            }
            Err(e) => {
                panic!("Template literal type test failed: {}", e);
            }
        }
    }

    /// Test indexed access type (v0.3.169)
    /// 索引访问类型应该被正确处理
    #[test]
    fn test_indexed_access_type() {
        let ts_code = r#"
type Person = { name: string; age: number };
type Name = Person["name"];
const name: Name = "Alice";
console.log(name);
"#;
        match compile_typescript(ts_code, "indexed_access.ts") {
            Ok(output) => {
                println!("索引访问类型转译结果:");
                println!("{}", output.js_code);
                // 类型注解应该被移除
                assert!(
                    !output.js_code.contains("Person[\"name\"]"),
                    "Should not contain indexed access type: {}",
                    output.js_code
                );
                println!("✅ Indexed access type test passed");
            }
            Err(e) => {
                panic!("Indexed access type test failed: {}", e);
            }
        }
    }

    /// Test infer type (v0.3.169)
    /// infer 类型应该被正确处理
    #[test]
    fn test_infer_type() {
        let ts_code = r#"
type ReturnType<T> = T extends (...args: any[]) => infer R ? R : never;
type Num = ReturnType<() => number>;
console.log(Num);
"#;
        match compile_typescript(ts_code, "infer_type.ts") {
            Ok(output) => {
                println!("infer 类型转译结果:");
                println!("{}", output.js_code);
                // 类型定义应该被移除
                assert!(
                    !output.js_code.contains("infer"),
                    "Should not contain infer keyword: {}",
                    output.js_code
                );
                println!("✅ Infer type test passed");
            }
            Err(e) => {
                panic!("Infer type test failed: {}", e);
            }
        }
    }

    /// Test constructor type (v0.3.169)
    /// 构造函数类型应该被正确处理
    #[test]
    fn test_constructor_type() {
        let ts_code = r#"
class Container<T> {
    value: T;
    constructor(val: T) {
        this.value = val;
    }
}
const instance = new Container(42);
console.log(instance.value);
"#;
        match compile_typescript(ts_code, "constructor_type.ts") {
            Ok(output) => {
                println!("构造函数类型转译结果:");
                println!("{}", output.js_code);
                // 应该保留类结构，移除类型注解
                assert!(
                    output.js_code.contains("class Container"),
                    "Should contain Container class: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("constructor"),
                    "Should contain constructor: {}",
                    output.js_code
                );
                // 泛型参数应该被移除
                assert!(
                    !output.js_code.contains("<T>"),
                    "Should not contain generic parameter: {}",
                    output.js_code
                );
                println!("✅ Constructor type test passed");
            }
            Err(e) => {
                panic!("Constructor type test failed: {}", e);
            }
        }
    }

    /// Test optional and readonly modifiers (v0.3.169)
    /// 可选和 readonly 修饰符应该被正确处理
    #[test]
    fn test_modifiers() {
        let ts_code = r#"
interface Config {
    host: string;
    port?: number;
    readonly id: string;
}
const config: Config = { host: "localhost", port: 8080, id: "123" };
console.log(config);
"#;
        match compile_typescript(ts_code, "modifiers.ts") {
            Ok(output) => {
                println!("修饰符转译结果:");
                println!("{}", output.js_code);
                // 应该保留对象结构，移除类型注解和修饰符
                assert!(
                    output.js_code.contains("host"),
                    "Should contain host: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("port"),
                    "Should contain port: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("8080"),
                    "Should contain port value: {}",
                    output.js_code
                );
                // 类型注解和修饰符应该被移除
                assert!(
                    !output.js_code.contains("string"),
                    "Should not contain string type: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("number"),
                    "Should not contain number type: {}",
                    output.js_code
                );
                println!("✅ Modifiers test passed");
            }
            Err(e) => {
                panic!("Modifiers test failed: {}", e);
            }
        }
    }
}
