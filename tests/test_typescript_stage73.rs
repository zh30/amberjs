// Stage 73 TypeScript 转译测试
// 验证箭头函数和类型标注转译功能

#[cfg(test)]
mod tests {
    use amberjs::typescript::compile_typescript;

    #[test]
    fn test_simple_arrow_function() {
        let code = r#"
const double = (x: number) => x * 2;
console.log(double(5));
"#;

        match compile_typescript(code, "test.ts") {
            Ok(output) => {
                println!("✅ 转译成功!");
                println!("原始代码: {}", code);
                println!("转译后: {}", output.js_code);

                // 验证类型标注被移除
                assert!(!output.js_code.contains(": number"));
                assert!(output.js_code.contains("double"));
            }
            Err(e) => {
                panic!("❌ 转译失败: {}", e);
            }
        }
    }

    #[test]
    fn test_multi_param_arrow() {
        let code = r#"
const add = (a: number, b: number): number => a + b;
console.log(add(10, 20));
"#;

        match compile_typescript(code, "test.ts") {
            Ok(output) => {
                println!("✅ 多参数箭头函数转译成功!");
                println!("转译后: {}", output.js_code);

                // 验证类型标注被移除
                assert!(!output.js_code.contains(": number"));
                assert!(!output.js_code.contains(": number =>"));
            }
            Err(e) => {
                panic!("❌ 转译失败: {}", e);
            }
        }
    }

    #[test]
    fn test_no_param_arrow() {
        let code = r#"
const getAnswer = () => 42;
console.log(getAnswer());
"#;

        match compile_typescript(code, "test.ts") {
            Ok(output) => {
                println!("✅ 无参数箭头函数转译成功!");
                println!("转译后: {}", output.js_code);
                assert!(output.js_code.contains("getAnswer"));
            }
            Err(e) => {
                panic!("❌ 转译失败: {}", e);
            }
        }
    }

    #[test]
    fn test_function_with_types() {
        let code = r#"
function greet(name: string): string {
    return `Hello, ${name}!`;
}
console.log(greet("Amber"));
"#;

        match compile_typescript(code, "test.ts") {
            Ok(output) => {
                println!("✅ 函数类型标注转译成功!");
                println!("转译后: {}", output.js_code);

                // 验证类型标注被移除
                assert!(!output.js_code.contains(": string"));
                assert!(!output.js_code.contains(": string {"));
            }
            Err(e) => {
                panic!("❌ 转译失败: {}", e);
            }
        }
    }
}
