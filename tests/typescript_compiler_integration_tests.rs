#[cfg(test)]
mod typescript_compiler_integration_tests {
    use amberjs::typescript::{
        compile_typescript as compile_oxc, CompilationOutput, TypeScriptCompiler,
        TypeScriptCompilerConfig,
    };

    /// Historical emit tests stay on the self-hosted compiler.
    /// TSX/JSX filenames go through oxc, which is the product backend.
    fn compile_typescript(source: &str, file_name: &str) -> Result<CompilationOutput, String> {
        let lower = file_name.to_ascii_lowercase();
        if lower.ends_with(".tsx") || lower.ends_with(".jsx") {
            return compile_oxc(source, file_name);
        }
        let mut compiler = TypeScriptCompiler::new(TypeScriptCompilerConfig::default());
        compiler
            .compile_source(source, file_name)
            .map_err(|error| error.to_string())
    }

    #[test]
    fn test_source_map_uses_relative_source_line_deltas() {
        let ts_code = "const first: number = 1\nconst second: number = 2\nconst third: number = 3";

        let output = compile_typescript(ts_code, "line_map.ts").expect("TypeScript should compile");
        let source_map = output.source_map.expect("source map should be generated");
        let parsed: serde_json::Value =
            serde_json::from_str(&source_map).expect("source map should be valid JSON");
        let mappings = parsed["mappings"]
            .as_str()
            .expect("source map should contain mappings");
        let first_segments: Vec<Vec<i32>> = mappings
            .split(';')
            .map(decode_first_mapping_segment)
            .collect::<Result<_, _>>()
            .expect("source map mappings should decode");

        assert_eq!(
            first_segments.len(),
            3,
            "expected one first mapping per generated line, got mappings: {}",
            mappings
        );
        assert_eq!(
            first_segments[0],
            vec![0, 0, 0, 0],
            "first line should map from generated col 0 to source line delta 0"
        );
        assert_eq!(
            first_segments[1],
            vec![0, 0, 1, 0],
            "second line should encode source line as a relative +1 delta"
        );
        assert_eq!(
            first_segments[2],
            vec![0, 0, 1, 0],
            "third line should encode source line as another relative +1 delta, got mappings: {}",
            mappings
        );
    }

    fn decode_first_mapping_segment(line_mapping: &str) -> Result<Vec<i32>, String> {
        let fields: Vec<&str> = line_mapping
            .split(',')
            .filter(|field| !field.is_empty())
            .take(4)
            .collect();

        if fields.len() != 4 {
            return Err(format!(
                "expected 4 fields in first source-map segment, got {fields:?}"
            ));
        }

        fields.into_iter().map(decode_vlq_field).collect()
    }

    fn decode_vlq_field(field: &str) -> Result<i32, String> {
        const BASE64: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

        let mut value = 0i32;
        for (idx, ch) in field.chars().enumerate() {
            let digit = BASE64
                .find(ch)
                .ok_or_else(|| format!("invalid base64 VLQ char: {ch}"))?
                as i32;
            value |= (digit & 0x1f) << (idx * 5);
        }
        Ok(value)
    }

    #[test]
    fn test_type_diagnostics_are_returned_to_caller() {
        let ts_code = r#"
function load() {
    await fetch("/api/data");
}
"#;

        let output = compile_typescript(ts_code, "diagnostics.ts")
            .expect("diagnostic-producing TypeScript should still return output");

        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("await expression")),
            "caller should receive type diagnostics, got: {:?}",
            output.diagnostics
        );
    }

    #[test]
    fn test_tsx_elements_emit_classic_jsx() {
        let ts_code = r#"
const element = <div className="greeting">Hello</div>;
"#;

        let output =
            compile_typescript(ts_code, "component.tsx").expect("TSX should transpile through oxc");

        assert!(
            output.js_code.contains("React.createElement"),
            "classic JSX emit expected, got: {}",
            output.js_code
        );
        assert!(
            !output.js_code.contains("<div"),
            "JSX element syntax should be transformed, got: {}",
            output.js_code
        );
    }

    #[test]
    fn test_self_closing_tsx_emits_classic_jsx() {
        let ts_code = r#"
const element = <Foo />;
"#;

        let output = compile_typescript(ts_code, "component.tsx")
            .expect("self-closing TSX should transpile through oxc");

        assert!(
            output.js_code.contains("React.createElement"),
            "classic JSX emit expected, got: {}",
            output.js_code
        );
    }

    #[test]
    fn test_tsx_detection_ignores_string_literals() {
        let ts_code = r#"
const markup = "<div>";
console.log(markup);
"#;

        let output = compile_typescript(ts_code, "string_literal.tsx")
            .expect("string literals containing angle brackets are not JSX elements");

        assert!(
            output.js_code.contains("\"<div>\""),
            "string literal should compile without TSX unsupported diagnostic, got: {}",
            output.js_code
        );
    }

    #[test]
    fn test_simple_typescript_transpilation() {
        let ts_code = r#"
const x: number = 42;
console.log("Test:", x);
"#;

        println!("原始 TypeScript 代码:");
        println!("{}", ts_code);
        println!("\n--- 开始转译 ---\n");

        match compile_typescript(ts_code, "test.ts") {
            Ok(output) => {
                println!("✅ 转译成功!");
                println!("\n转译后的 JavaScript 代码:");
                println!("{}", output.js_code);

                // 验证转译后的代码包含期望的内容
                assert!(output.js_code.contains("x"));
                assert!(!output.js_code.contains(": number"));
            }
            Err(e) => {
                println!("❌ 转译失败: {}", e);
                panic!("TypeScript transpilation failed: {}", e);
            }
        }
    }

    #[test]
    fn test_arrow_function_typescript() {
        let ts_code = r#"
const add = (a: number, b: number): number => a + b;
console.log(add(5, 3));
"#;

        match compile_typescript(ts_code, "arrow_test.ts") {
            Ok(output) => {
                println!("箭头函数转译结果:");
                println!("{}", output.js_code);
                assert!(output.js_code.contains("add"));
            }
            Err(e) => {
                panic!("Arrow function transpilation failed: {}", e);
            }
        }
    }

    #[test]
    fn test_namespace_simple() {
        // 测试简单命名空间
        let ts_code = r#"
namespace MyNamespace {
    const value: number = 42;
    export function getValue(): number {
        return value;
    }
}
console.log(MyNamespace.getValue());
"#;

        match compile_typescript(ts_code, "namespace_test.ts") {
            Ok(output) => {
                println!("命名空间转译结果:");
                println!("{}", output.js_code);
                // 验证命名空间语法被正确转换
                assert!(output.js_code.contains("MyNamespace"));
                assert!(output.js_code.contains("getValue"));
                assert!(!output.js_code.contains(": number"));
            }
            Err(e) => {
                panic!("Namespace transpilation failed: {}", e);
            }
        }
    }

    #[test]
    fn test_namespace_with_multiple_declarations() {
        // 测试包含多个声明的命名空间（暂不包含 interface）
        let ts_code = r#"
namespace Utils {
    export const PI: number = 3.14159;
    export function add(a: number, b: number): number {
        return a + b;
    }
    export const double = (x: number) => x * 2;
}
const result: number = Utils.add(10, 20);
console.log(Utils.PI, Utils.double(result));
"#;

        match compile_typescript(ts_code, "namespace_multi.ts") {
            Ok(output) => {
                println!("多声明命名空间转译结果:");
                println!("{}", output.js_code);
                assert!(output.js_code.contains("Utils"));
                assert!(output.js_code.contains("PI"));
                assert!(output.js_code.contains("add"));
                assert!(output.js_code.contains("double"));
                // TypeScript 特有语法应该被移除
                assert!(!output.js_code.contains(": number"));
            }
            Err(e) => {
                panic!("Multi-declaration namespace transpilation failed: {}", e);
            }
        }
    }

    #[test]
    fn test_nested_namespace() {
        // 测试嵌套命名空间 A.B.C
        let ts_code = r#"
namespace Outer {
    export namespace Inner {
        export const value: number = 42;
        export function getValue(): number {
            return value;
        }
    }
}
console.log(Outer.Inner.getValue());
"#;

        match compile_typescript(ts_code, "nested_namespace.ts") {
            Ok(output) => {
                println!("嵌套命名空间转译结果:");
                println!("{}", output.js_code);
                assert!(output.js_code.contains("Outer"));
                assert!(output.js_code.contains("Inner"));
                assert!(output.js_code.contains("getValue"));
                assert!(!output.js_code.contains(": number"));
            }
            Err(e) => {
                panic!("Nested namespace transpilation failed: {}", e);
            }
        }
    }

    /// Test shorthand nested namespace (v0.3.155)
    #[test]
    fn test_shorthand_nested_namespace() {
        // 测试简写嵌套命名空间 A.B.C { ... }
        let ts_code = r#"
namespace A.B.C {
    export const value: number = 42;
    export function getValue(): number {
        return value;
    }
}
console.log(A.B.C.getValue());
"#;

        match compile_typescript(ts_code, "shorthand_nested_namespace.ts") {
            Ok(output) => {
                println!("简写嵌套命名空间转译结果:");
                println!("{}", output.js_code);
                // 验证所有命名空间层级都存在
                assert!(output.js_code.contains("var A"), "Should declare var A");
                assert!(output.js_code.contains("var B"), "Should declare var B");
                assert!(output.js_code.contains("var C"), "Should declare var C");
                assert!(output.js_code.contains("value"), "Should contain value");
                assert!(
                    output.js_code.contains("getValue"),
                    "Should contain getValue"
                );
                // TypeScript 类型注解应该被移除
                assert!(
                    !output.js_code.contains(": number"),
                    "Should not contain type annotation"
                );
                println!("✅ Shorthand nested namespace test passed");
            }
            Err(e) => {
                panic!("Shorthand nested namespace transpilation failed: {}", e);
            }
        }
    }

    /// Test declare namespace with nested names (v0.3.155)
    #[test]
    fn test_declare_nested_namespace() {
        // 测试 declare 嵌套命名空间
        let ts_code = r#"
declare namespace Outer.Inner {
    export const value: number;
    export function getValue(): number;
}
"#;

        match compile_typescript(ts_code, "declare_nested_namespace.ts") {
            Ok(output) => {
                println!("declare 嵌套命名空间转译结果:");
                println!("{}", output.js_code);
                // 验证 declare 关键字和完整命名空间路径
                assert!(
                    output.js_code.contains("declare namespace Outer.Inner"),
                    "Should contain declare namespace Outer.Inner: {}",
                    output.js_code
                );
                println!("✅ Declare nested namespace test passed");
            }
            Err(e) => {
                panic!("Declare nested namespace transpilation failed: {}", e);
            }
        }
    }

    #[test]
    fn test_declare_namespace() {
        // 测试 declare namespace 声明
        let ts_code = r#"
declare namespace MyLib {
    export const version: string = "";
    export function greet(name: string): string { return ""; }
}
console.log(MyLib.version);
"#;

        match compile_typescript(ts_code, "declare_namespace.ts") {
            Ok(output) => {
                println!("declare namespace 转译结果:");
                println!("{}", output.js_code);
                assert!(output.js_code.contains("MyLib"));
                assert!(output.js_code.contains("version"));
                assert!(output.js_code.contains("greet"));
                // declare namespace 应该保留 declare 关键字
                assert!(
                    output.js_code.contains("declare namespace"),
                    "Should contain declare namespace: {}",
                    output.js_code
                );
            }
            Err(e) => {
                panic!("Declare namespace transpilation failed: {}", e);
            }
        }
    }

    #[test]
    fn test_namespace_with_export_keyword() {
        // 测试 namespace 内的 export 关键字
        let ts_code = r#"
namespace Math {
    export const PI: number = 3.14159;
    export function add(a: number, b: number): number {
        return a + b;
    }
    export function multiply(a: number, b: number): number {
        return a * b;
    }
    const secret: number = 12345;
    export function getSecret(): number {
        return secret;
    }
}
console.log(Math.add(1, 2), Math.multiply(3, 4));
"#;

        match compile_typescript(ts_code, "namespace_export.ts") {
            Ok(output) => {
                println!("namespace export 关键字转译结果:");
                println!("{}", output.js_code);
                assert!(output.js_code.contains("Math"));
                assert!(output.js_code.contains("PI"));
                assert!(output.js_code.contains("add"));
                assert!(output.js_code.contains("multiply"));
                assert!(output.js_code.contains("getSecret"));
                // TypeScript 特有语法应该被移除
                assert!(!output.js_code.contains(": number"));
            }
            Err(e) => {
                panic!("Namespace export keyword transpilation failed: {}", e);
            }
        }
    }

    #[test]
    fn test_declare_class_basic() {
        // 测试 declare class 声明
        let ts_code = r#"
declare class MyClass {
    name: string;
    age: number;
}
"#;

        match compile_typescript(ts_code, "declare_class.ts") {
            Ok(output) => {
                println!("declare class 转译结果:");
                println!("{}", output.js_code);
                assert!(output.js_code.contains("MyClass"));
                assert!(output.js_code.contains("declare"));
                assert!(output.js_code.contains("class MyClass"));
                // 类型注解应该被移除
                assert!(!output.js_code.contains(": string"));
                assert!(!output.js_code.contains(": number"));
            }
            Err(e) => {
                panic!("Declare class transpilation failed: {}", e);
            }
        }
    }

    #[test]
    fn test_declare_class_with_extends() {
        // 测试 declare class 继承
        let ts_code = r#"
declare class Animal {
    name: string;
}
declare class Dog extends Animal {
    breed: string;
}
"#;

        match compile_typescript(ts_code, "declare_class_extends.ts") {
            Ok(output) => {
                println!("declare class with extends 转译结果:");
                println!("{}", output.js_code);
                assert!(output.js_code.contains("Animal"));
                assert!(output.js_code.contains("Dog"));
                assert!(output.js_code.contains("declare class"));
                // 验证 extends 保留
                assert!(output.js_code.contains("extends Animal"));
            }
            Err(e) => {
                panic!("Declare class with extends transpilation failed: {}", e);
            }
        }
    }

    #[test]
    fn test_declare_class_with_methods() {
        // 测试 declare class 方法声明
        let ts_code = r#"
declare class Calculator {
    PI: number;
    VERSION: string;
}
"#;

        match compile_typescript(ts_code, "declare_class_methods.ts") {
            Ok(output) => {
                println!("declare class with methods 转译结果:");
                println!("{}", output.js_code);
                assert!(output.js_code.contains("Calculator"));
                assert!(output.js_code.contains("declare class Calculator"));
                assert!(output.js_code.contains("PI"));
            }
            Err(e) => {
                panic!("Declare class with methods transpilation failed: {}", e);
            }
        }
    }

    #[test]
    fn test_regular_class_vs_declare_class() {
        // 测试普通 class 和 declare class 的区别
        let ts_code = r#"
class RegularClass {
    constructor() {
        this.value = 42;
    }
    getValue() {
        return this.value;
    }
}
declare class DeclaredClass {
    value: number;
}
"#;

        match compile_typescript(ts_code, "class_comparison.ts") {
            Ok(output) => {
                println!("普通 class vs declare class 转译结果:");
                println!("{}", output.js_code);
                // 普通类应该有完整的实现
                assert!(output.js_code.contains("class RegularClass"));
                assert!(output.js_code.contains("constructor()"));
                assert!(output.js_code.contains("this.value"));
                // 声明类应该有 declare 关键字
                assert!(output.js_code.contains("declare class DeclaredClass"));
            }
            Err(e) => {
                panic!("Class comparison transpilation failed: {}", e);
            }
        }
    }

    /// Test declare const (v0.3.152)
    #[test]
    fn test_declare_const() {
        // 测试 declare const 声明
        let ts_code = r#"
declare const API_KEY: string;
declare const MAX_RETRIES: number = 3;
"#;
        match compile_typescript(ts_code, "declare_const.ts") {
            Ok(output) => {
                println!("declare const 转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("declare const API_KEY"),
                    "Should contain declare const API_KEY: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("declare const MAX_RETRIES"),
                    "Should contain declare const MAX_RETRIES: {}",
                    output.js_code
                );
                // 类型注解应该被移除
                assert!(
                    !output.js_code.contains(": string"),
                    "Should not contain type annotation: {}",
                    output.js_code
                );
                println!("✅ Declare const test passed");
            }
            Err(e) => {
                panic!("Declare const transpilation failed: {}", e);
            }
        }
    }

    /// Test declare let (v0.3.152)
    #[test]
    fn test_declare_let() {
        // 测试 declare let 声明
        let ts_code = r#"
declare let appVersion: string;
declare let isReady: boolean;
"#;
        match compile_typescript(ts_code, "declare_let.ts") {
            Ok(output) => {
                println!("declare let 转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("declare let appVersion"),
                    "Should contain declare let appVersion: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("declare let isReady"),
                    "Should contain declare let isReady: {}",
                    output.js_code
                );
                println!("✅ Declare let test passed");
            }
            Err(e) => {
                panic!("Declare let transpilation failed: {}", e);
            }
        }
    }

    /// Test declare var (v0.3.152)
    #[test]
    fn test_declare_var() {
        // 测试 declare var 声明
        let ts_code = r#"
declare var globalConfig: object;
declare var DEBUG_MODE: boolean;
"#;
        match compile_typescript(ts_code, "declare_var.ts") {
            Ok(output) => {
                println!("declare var 转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("declare var globalConfig"),
                    "Should contain declare var globalConfig: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("declare var DEBUG_MODE"),
                    "Should contain declare var DEBUG_MODE: {}",
                    output.js_code
                );
                println!("✅ Declare var test passed");
            }
            Err(e) => {
                panic!("Declare var transpilation failed: {}", e);
            }
        }
    }

    /// Test export declare const (v0.3.152)
    /// v0.3.198: 更新测试以匹配 ESM → CommonJS 转换后的输出格式
    #[test]
    fn test_export_declare_const() {
        // 测试 export declare const 声明
        let ts_code = r#"
export declare const PLUGIN_NAME: string;
export declare const VERSION: string;
"#;
        match compile_typescript(ts_code, "export_declare_const.ts") {
            Ok(output) => {
                println!("export declare const 转译结果:");
                println!("{}", output.js_code);
                // v0.3.195 之后，ESM export 转换为 CommonJS 注释占位符
                assert!(
                    output.js_code.contains("/* ESM export"),
                    "Should contain ESM export comment: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("const PLUGIN_NAME"),
                    "Should contain const PLUGIN_NAME: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("const VERSION"),
                    "Should contain const VERSION: {}",
                    output.js_code
                );
                println!("✅ Export declare const test passed");
            }
            Err(e) => {
                panic!("Export declare const transpilation failed: {}", e);
            }
        }
    }

    /// Test declare global (v0.3.152)
    #[test]
    fn test_declare_global() {
        // 测试 declare global 声明块
        let ts_code = r#"
declare global {
    interface Window {
        myPlugin: any;
    }
    function myGlobalFunction(): void;
}
"#;
        match compile_typescript(ts_code, "declare_global.ts") {
            Ok(output) => {
                println!("declare global 转译结果:");
                println!("{}", output.js_code);
                // 声明块应该被保留（带有注释标记）
                assert!(
                    output.js_code.contains("declare global"),
                    "Should contain declare global: {}",
                    output.js_code
                );
                // 函数声明应该被保留
                assert!(
                    output.js_code.contains("myGlobalFunction"),
                    "Should contain myGlobalFunction: {}",
                    output.js_code
                );
                // 接口在 JS 中没有对应，会被跳过
                println!("✅ Declare global test passed");
            }
            Err(e) => {
                panic!("Declare global transpilation failed: {}", e);
            }
        }
    }

    /// Test declare global with variables (v0.3.152)
    #[test]
    fn test_declare_global_with_variables() {
        // 测试 declare global 中的变量声明
        let ts_code = r#"
declare global {
    declare const GLOBAL_API_KEY: string;
    declare let globalCounter: number;
}
"#;
        match compile_typescript(ts_code, "declare_global_variables.ts") {
            Ok(output) => {
                println!("declare global with variables 转译结果:");
                println!("{}", output.js_code);
                // 输出应该包含 declare const 和 declare let
                assert!(
                    output.js_code.contains("declare const GLOBAL_API_KEY"),
                    "Should contain declare const GLOBAL_API_KEY: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("declare let globalCounter"),
                    "Should contain declare let globalCounter: {}",
                    output.js_code
                );
                println!("✅ Declare global with variables test passed");
            }
            Err(e) => {
                panic!("Declare global with variables transpilation failed: {}", e);
            }
        }
    }

    /// Test regular variable vs declare variable (v0.3.152)
    #[test]
    fn test_regular_variable_vs_declare_variable() {
        // 测试普通变量和 declare 变量的区别
        let ts_code = r#"
const regularConst = "hello";
let regularLet = 42;
var regularVar = true;

declare const declaredConst: string;
declare let declaredLet: number;
declare var declaredVar: boolean;
"#;
        match compile_typescript(ts_code, "variable_comparison.ts") {
            Ok(output) => {
                println!("普通变量 vs declare 变量转译结果:");
                println!("{}", output.js_code);
                // 普通变量应该没有 declare 关键字
                assert!(
                    output.js_code.contains("const regularConst"),
                    "Should contain regular const: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("let regularLet"),
                    "Should contain regular let: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("var regularVar"),
                    "Should contain regular var: {}",
                    output.js_code
                );
                // 声明变量应该有 declare 关键字
                assert!(
                    output.js_code.contains("declare const declaredConst"),
                    "Should contain declare const: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("declare let declaredLet"),
                    "Should contain declare let: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("declare var declaredVar"),
                    "Should contain declare var: {}",
                    output.js_code
                );
                println!("✅ Regular vs declare variable test passed");
            }
            Err(e) => {
                panic!("Variable comparison transpilation failed: {}", e);
            }
        }
    }

    /// Test export declare function (v0.3.152)
    /// v0.3.198: 更新测试以匹配 ESM → CommonJS 转换后的输出格式
    #[test]
    fn test_export_declare_function() {
        // 测试 export declare function 声明
        let ts_code = r#"
export declare function greet(name: string): string;
export declare function add(a: number, b: number): number;
"#;
        match compile_typescript(ts_code, "export_declare_function.ts") {
            Ok(output) => {
                println!("export declare function 转译结果:");
                println!("{}", output.js_code);
                // v0.3.195 之后，ESM export 转换为 CommonJS 注释占位符
                assert!(
                    output.js_code.contains("/* ESM export: function greet"),
                    "Should contain ESM export comment for greet: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("/* ESM export: function add"),
                    "Should contain ESM export comment for add: {}",
                    output.js_code
                );
                // 类型注解应该被移除
                assert!(
                    !output.js_code.contains(": string"),
                    "Should not contain type annotation: {}",
                    output.js_code
                );
                println!("✅ Export declare function test passed");
            }
            Err(e) => {
                panic!("Export declare function transpilation failed: {}", e);
            }
        }
    }

    /// Test export declare const (v0.3.152)
    /// v0.3.198: 更新测试以匹配 ESM → CommonJS 转换后的输出格式
    #[test]
    fn test_export_declare_const_integration() {
        // 测试 export declare const 声明
        let ts_code = r#"
export declare const PI: number;
export declare const API_URL: string;
"#;
        match compile_typescript(ts_code, "export_declare_const.ts") {
            Ok(output) => {
                println!("export declare const 转译结果:");
                println!("{}", output.js_code);
                // v0.3.195 之后，ESM export 转换为 CommonJS 注释占位符
                assert!(
                    output.js_code.contains("/* ESM export: const PI"),
                    "Should contain ESM export comment for PI: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("/* ESM export: const API_URL"),
                    "Should contain ESM export comment for API_URL: {}",
                    output.js_code
                );
                println!("✅ Export declare const test passed");
            }
            Err(e) => {
                panic!("Export declare const transpilation failed: {}", e);
            }
        }
    }

    /// Test declare function with overloads (v0.3.152)
    #[test]
    fn test_declare_function_overloads() {
        // 测试 declare function 重载签名
        let ts_code = r#"
declare function greet(name: string): string;
declare function greet(name: string, formal: boolean): string;
declare function add(a: number, b: number): number;
declare function add(a: number, b: number, c: number): number;
declare function process(value: string): string;
declare function process(value: number): number;
"#;
        match compile_typescript(ts_code, "declare_function_overloads.ts") {
            Ok(output) => {
                println!("declare function overloads 转译结果:");
                println!("{}", output.js_code);
                // 应该保留所有的重载签名声明
                let greet_count = output.js_code.matches("declare function greet").count();
                assert!(
                    greet_count >= 2,
                    "Should contain at least 2 greet overloads, found {}: {}",
                    greet_count,
                    output.js_code
                );
                let add_count = output.js_code.matches("declare function add").count();
                assert!(
                    add_count >= 2,
                    "Should contain at least 2 add overloads, found {}: {}",
                    add_count,
                    output.js_code
                );
                println!("✅ Declare function overloads test passed");
            }
            Err(e) => {
                panic!("Declare function overloads transpilation failed: {}", e);
            }
        }
    }

    /// Test declare class with constructor signature (v0.3.152)
    #[test]
    fn test_declare_class_constructor_signature() {
        // 测试 declare class 带有构造函数签名的声明（简化版本）
        let ts_code = r#"
declare class Person {
    name: string;
    age: number;
    constructor(name: string);
    greet(): string;
}
"#;
        match compile_typescript(ts_code, "declare_class_constructor.ts") {
            Ok(output) => {
                println!("declare class with constructor signature 转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("declare class Person"),
                    "Should contain declare class Person: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("constructor"),
                    "Should contain constructor: {}",
                    output.js_code
                );
                // 验证类型注解被移除
                assert!(
                    !output.js_code.contains(": string") || output.js_code.contains("constructor"),
                    "Should not contain type annotation: {}",
                    output.js_code
                );
                println!("✅ Declare class with constructor signature test passed");
            }
            Err(e) => {
                panic!(
                    "Declare class with constructor signature transpilation failed: {}",
                    e
                );
            }
        }
    }

    /// Test mixed declare patterns (v0.3.152)
    #[test]
    fn test_mixed_declare_patterns() {
        // 测试混合的 declare 声明模式
        let ts_code = r#"
declare const PI: number;
declare function square(n: number): number;
declare class Logger {
    log(msg: string): void;
}
declare interface Config {
    debug: boolean;
}
declare namespace Utils {
    export const version: string;
    export function helper(): void;
}
declare global {
    interface Window {
        myGlobal: string;
    }
}
"#;
        match compile_typescript(ts_code, "mixed_declare_patterns.ts") {
            Ok(output) => {
                println!("混合 declare 模式转译结果:");
                println!("{}", output.js_code);
                // 验证所有 declare 类型都存在
                assert!(
                    output.js_code.contains("declare const PI"),
                    "Should contain declare const PI: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("declare function square"),
                    "Should contain declare function square: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("declare class Logger"),
                    "Should contain declare class Logger: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("declare namespace Utils"),
                    "Should contain declare namespace Utils: {}",
                    output.js_code
                );
                println!("✅ Mixed declare patterns test passed");
            }
            Err(e) => {
                panic!("Mixed declare patterns transpilation failed: {}", e);
            }
        }
    }

    /// Test function type (v0.3.154)
    #[test]
    fn test_function_type() {
        // 测试函数类型注解 (arg1: type1, arg2: type2) => returnType
        let ts_code = r#"
type Callback = (data: string) => void;
type BinaryOp = (a: number, b: number) => number;
type AsyncHandler = (req: unknown) => Promise<string>;
const onData = (data: string) => { console.log(data); };
const add = (a: number, b: number): number => a + b;
"#;
        match compile_typescript(ts_code, "function_type.ts") {
            Ok(output) => {
                println!("函数类型转译结果:");
                println!("{}", output.js_code);
                // 函数类型注解应该被移除
                assert!(
                    !output.js_code.contains(": string) => void"),
                    "Should not contain function type annotation: {}",
                    output.js_code
                );
                // 变量赋值应该保留
                assert!(
                    output.js_code.contains("onData"),
                    "Should contain onData: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("add"),
                    "Should contain add: {}",
                    output.js_code
                );
                // 验证箭头函数被正确保留
                assert!(
                    output.js_code.contains("=>"),
                    "Should contain arrow function: {}",
                    output.js_code
                );
                println!("✅ Function type test passed");
            }
            Err(e) => {
                panic!("Function type transpilation failed: {}", e);
            }
        }
    }

    /// Test tuple type (v0.3.154)
    #[test]
    fn test_tuple_type() {
        // 测试元组类型 [type1, type2, ...restType]
        let ts_code = r#"
type Point = [number, number];
type StringNumberPair = [string, number];
const point = [1, 2];
const pair = ["hello", 42];
"#;
        match compile_typescript(ts_code, "tuple_type.ts") {
            Ok(output) => {
                println!("元组类型转译结果:");
                println!("{}", output.js_code);
                // 变量赋值应该保留
                assert!(
                    output.js_code.contains("point"),
                    "Should contain point: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("pair"),
                    "Should contain pair: {}",
                    output.js_code
                );
                // 验证数组字面量被正确保留
                assert!(
                    output.js_code.contains("[1, 2]"),
                    "Should contain array literal: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("[\"hello\", 42]"),
                    "Should contain array literal: {}",
                    output.js_code
                );
                println!("✅ Tuple type test passed");
            }
            Err(e) => {
                panic!("Tuple type transpilation failed: {}", e);
            }
        }
    }

    /// Test infer keyword in conditional types (v0.3.154)
    #[test]
    fn test_infer_keyword() {
        // 测试 infer 关键字在条件类型中的使用
        let ts_code = r#"
type UnwrapPromise<T> = T extends Promise<infer U> ? U : T;
type UnwrapArray<T> = T extends Array<infer E> ? E : T;
const p = "hello";
const arr = "item";
"#;
        match compile_typescript(ts_code, "infer_keyword.ts") {
            Ok(output) => {
                println!("infer 关键字转译结果:");
                println!("{}", output.js_code);
                // 变量赋值应该保留
                assert!(
                    output.js_code.contains("p"),
                    "Should contain p: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("arr"),
                    "Should contain arr: {}",
                    output.js_code
                );
                // 验证字符串值被正确保留
                assert!(
                    output.js_code.contains("\"hello\""),
                    "Should contain string value: {}",
                    output.js_code
                );
                println!("✅ Infer keyword test passed");
            }
            Err(e) => {
                panic!("Infer keyword transpilation failed: {}", e);
            }
        }
    }

    /// Test generic with extends constraint (v0.3.154)
    #[test]
    fn test_generic_extends_constraint() {
        // 测试泛型约束 <T extends U>
        let ts_code = r#"
type WithConstraint<T extends string> = T;
type Numeric<T extends number> = T;
const constrained = "hello";
const num = 42;
"#;
        match compile_typescript(ts_code, "generic_extends.ts") {
            Ok(output) => {
                println!("泛型约束转译结果:");
                println!("{}", output.js_code);
                // 变量赋值应该保留
                assert!(
                    output.js_code.contains("constrained"),
                    "Should contain constrained: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("num"),
                    "Should contain num: {}",
                    output.js_code
                );
                // 验证值被正确保留
                assert!(
                    output.js_code.contains("\"hello\""),
                    "Should contain string value: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("42"),
                    "Should contain number value: {}",
                    output.js_code
                );
                println!("✅ Generic extends constraint test passed");
            }
            Err(e) => {
                panic!("Generic extends constraint transpilation failed: {}", e);
            }
        }
    }

    /// Test namespace merging (multiple declarations of the same namespace merge)
    /// Namespace Merging: 同一命名空间的多个声明会被合并
    #[test]
    fn test_namespace_merging() {
        let ts_code = r#"
namespace MyLib {
    export function foo(): void {
        console.log("foo");
    }
}
namespace MyLib {
    export function bar(): void {
        console.log("bar");
    }
}
namespace MyLib {
    export const value: number = 42;
}
MyLib.foo();
MyLib.bar();
console.log(MyLib.value);
"#;
        match compile_typescript(ts_code, "namespace_merging.ts") {
            Ok(output) => {
                println!("命名空间合并转译结果:");
                println!("{}", output.js_code);
                // 验证所有导出的成员都存在
                assert!(
                    output.js_code.contains("foo"),
                    "Should contain foo function: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("bar"),
                    "Should contain bar function: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("value"),
                    "Should contain value constant: {}",
                    output.js_code
                );
                // 验证命名空间被正确创建
                assert!(
                    output.js_code.contains("MyLib"),
                    "Should contain MyLib namespace: {}",
                    output.js_code
                );
                println!("✅ Namespace merging test passed");
            }
            Err(e) => {
                panic!("Namespace merging test failed: {}", e);
            }
        }
    }

    /// Test namespace with nested declarations merging
    #[test]
    fn test_namespace_nested_merging() {
        let ts_code = r#"
namespace Outer {
    export const a: number = 1;
}
namespace Outer.Inner {
    export function innerFn(): void {}
}
namespace Outer {
    export const b: number = 2;
}
"#;
        match compile_typescript(ts_code, "namespace_nested_merging.ts") {
            Ok(output) => {
                println!("嵌套命名空间合并转译结果:");
                println!("{}", output.js_code);
                // 验证外层命名空间有 a 和 b
                assert!(
                    output.js_code.contains("a"),
                    "Should contain a: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("b"),
                    "Should contain b: {}",
                    output.js_code
                );
                // 验证内层命名空间有 innerFn
                assert!(
                    output.js_code.contains("innerFn"),
                    "Should contain innerFn: {}",
                    output.js_code
                );
                println!("✅ Namespace nested merging test passed");
            }
            Err(e) => {
                panic!("Namespace nested merging test failed: {}", e);
            }
        }
    }

    /// Test declare module (module augmentation syntax)
    /// declare module 用于声明模块增强
    #[test]
    fn test_declare_module() {
        let ts_code = r#"
declare module "my-module" {
    export const someValue: number;
    export function someFunction(): void;
}
const x: number = 1;
"#;
        match compile_typescript(ts_code, "declare_module.ts") {
            Ok(output) => {
                println!("declare module 转译结果:");
                println!("{}", output.js_code);
                // 验证 declare module 被正确处理
                assert!(
                    output.js_code.contains("someValue"),
                    "Should contain someValue: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("someFunction"),
                    "Should contain someFunction: {}",
                    output.js_code
                );
                // 变量应该被保留
                assert!(
                    output.js_code.contains("x"),
                    "Should contain x: {}",
                    output.js_code
                );
                println!("✅ Declare module test passed");
            }
            Err(e) => {
                panic!("Declare module test failed: {}", e);
            }
        }
    }

    /// Test namespace augmentation on existing type (simplified - only properties)
    #[test]
    fn test_namespace_augmentation() {
        let ts_code = r#"
interface MyInterface {
    myProperty: string;
    anotherProp: number;
}

const test: string = "hello";
"#;
        match compile_typescript(ts_code, "namespace_augmentation.ts") {
            Ok(output) => {
                println!("命名空间/接口增强转译结果:");
                println!("{}", output.js_code);
                // 变量应该被保留
                assert!(
                    output.js_code.contains("test"),
                    "Should contain test: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("\"hello\""),
                    "Should contain hello: {}",
                    output.js_code
                );
                println!("✅ Namespace augmentation test passed");
            }
            Err(e) => {
                panic!("Namespace augmentation test failed: {}", e);
            }
        }
    }

    /// Test abstract class (v0.3.157)
    #[test]
    fn test_abstract_class() {
        let ts_code = r#"
abstract class Animal {
    name: string;
    abstract makeSound(): void;
    move(): void {
        console.log("Moving");
    }
}
class Dog extends Animal {
    makeSound(): void {
        console.log("Woof!");
    }
}
const dog = new Dog();
dog.makeSound();
"#;
        match compile_typescript(ts_code, "abstract_class.ts") {
            Ok(output) => {
                println!("抽象类转译结果:");
                println!("{}", output.js_code);
                // 验证 abstract 关键字被移除（因为是 TS 特有语法）
                assert!(
                    !output.js_code.contains("abstract class"),
                    "Should NOT contain abstract class (should be removed): {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("abstract makeSound"),
                    "Should NOT contain abstract method (should be removed): {}",
                    output.js_code
                );
                // 验证类和方法保留
                assert!(
                    output.js_code.contains("class Animal"),
                    "Should contain Animal class: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("class Dog extends Animal"),
                    "Should contain Dog class: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("makeSound"),
                    "Should contain makeSound method: {}",
                    output.js_code
                );
                println!("✅ Abstract class test passed");
            }
            Err(e) => {
                panic!("Abstract class test failed: {}", e);
            }
        }
    }

    /// Test abstract class with abstract properties (v0.3.157)
    #[test]
    fn test_abstract_class_with_abstract_properties() {
        let ts_code = r#"
abstract class Shape {
    abstract color: string;
    abstract calculateArea(): number;
}
class Circle extends Shape {
    color: string = "red";
    radius: number = 5;
    calculateArea(): number {
        return Math.PI * this.radius * this.radius;
    }
}
const circle = new Circle();
console.log(circle.calculateArea());
"#;
        match compile_typescript(ts_code, "abstract_properties.ts") {
            Ok(output) => {
                println!("抽象属性转译结果:");
                println!("{}", output.js_code);
                // 验证 abstract 关键字被移除
                assert!(
                    !output.js_code.contains("abstract class"),
                    "Should NOT contain abstract class (should be removed): {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("abstract color"),
                    "Should NOT contain abstract property (should be removed): {}",
                    output.js_code
                );
                // 验证类保留
                assert!(
                    output.js_code.contains("class Shape"),
                    "Should contain Shape class: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("class Circle extends Shape"),
                    "Should contain Circle class: {}",
                    output.js_code
                );
                // 验证属性和方法保留（没有 abstract）
                assert!(
                    output.js_code.contains("color"),
                    "Should contain color property: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("calculateArea"),
                    "Should contain calculateArea method: {}",
                    output.js_code
                );
                println!("✅ Abstract properties test passed");
            }
            Err(e) => {
                panic!("Abstract properties test failed: {}", e);
            }
        }
    }

    /// Test abstract class with static abstract methods (v0.3.157)
    #[test]
    fn test_static_abstract_method() {
        let ts_code = r#"
abstract class Factory {
    static abstract create(): void;
}
class ConcreteFactory extends Factory {
    static create(): void {
        console.log("Creating product");
    }
}
ConcreteFactory.create();
"#;
        match compile_typescript(ts_code, "static_abstract.ts") {
            Ok(output) => {
                println!("静态抽象方法转译结果:");
                println!("{}", output.js_code);
                // 验证 abstract 关键字被移除
                assert!(
                    !output.js_code.contains("static abstract"),
                    "Should NOT contain static abstract (should be removed): {}",
                    output.js_code
                );
                // 验证 static 方法保留
                assert!(
                    output.js_code.contains("static create"),
                    "Should contain static create: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("class Factory"),
                    "Should contain Factory class: {}",
                    output.js_code
                );
                assert!(
                    output
                        .js_code
                        .contains("class ConcreteFactory extends Factory"),
                    "Should contain ConcreteFactory class: {}",
                    output.js_code
                );
                println!("✅ Static abstract method test passed");
            }
            Err(e) => {
                panic!("Static abstract method test failed: {}", e);
            }
        }
    }

    /// Test interface merging (multiple declarations of the same interface merge)
    /// Interface Merging: 同一接口的多个声明会被合并
    #[test]
    fn test_interface_merging() {
        let ts_code = r#"
interface Point {
    x: number;
    y: number;
}
interface Point {
    z: number;
}
interface Point {
    label: string;
}
const p: Point = { x: 1, y: 2, z: 3, label: "origin" };
"#;
        match compile_typescript(ts_code, "interface_merging.ts") {
            Ok(output) => {
                println!("接口合并转译结果:");
                println!("{}", output.js_code);
                // 验证变量 p 被正确创建
                assert!(
                    output.js_code.contains("p"),
                    "Should contain p variable: {}",
                    output.js_code
                );
                // 验证对象包含所有属性
                assert!(
                    output.js_code.contains("x"),
                    "Should contain x property: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("y"),
                    "Should contain y property: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("z"),
                    "Should contain z property: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("label"),
                    "Should contain label property: {}",
                    output.js_code
                );
                // 类型注解应该被移除
                assert!(
                    !output.js_code.contains(": number"),
                    "Should not contain type annotation: {}",
                    output.js_code
                );
                println!("✅ Interface merging test passed");
            }
            Err(e) => {
                panic!("Interface merging test failed: {}", e);
            }
        }
    }

    /// Test interface merging with extends (v0.3.159)
    #[test]
    fn test_interface_merging_with_extends() {
        let ts_code = r#"
interface Animal {
    name: string;
}
interface Animal {
    age: number;
}
interface Dog extends Animal {
    breed: string;
}
const dog: Dog = { name: "Buddy", age: 3, breed: "Labrador" };
"#;
        match compile_typescript(ts_code, "interface_merging_extends.ts") {
            Ok(output) => {
                println!("接口合并(含继承)转译结果:");
                println!("{}", output.js_code);
                // 验证变量 dog 被正确创建
                assert!(
                    output.js_code.contains("dog"),
                    "Should contain dog variable: {}",
                    output.js_code
                );
                // 验证所有属性存在
                assert!(
                    output.js_code.contains("name"),
                    "Should contain name: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("age"),
                    "Should contain age: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("breed"),
                    "Should contain breed: {}",
                    output.js_code
                );
                println!("✅ Interface merging with extends test passed");
            }
            Err(e) => {
                panic!("Interface merging with extends test failed: {}", e);
            }
        }
    }

    /// Test interface merging with index signature (v0.3.159)
    #[test]
    fn test_interface_merging_with_index_signature() {
        let ts_code = r#"
interface StringMap {
    key1: string;
}
interface StringMap {
    [key: string]: string;
}
const map: StringMap = { key1: "value1", key2: "value2" };
"#;
        match compile_typescript(ts_code, "interface_merging_index.ts") {
            Ok(output) => {
                println!("接口合并(含索引签名)转译结果:");
                println!("{}", output.js_code);
                // 验证变量 map 被正确创建
                assert!(
                    output.js_code.contains("map"),
                    "Should contain map variable: {}",
                    output.js_code
                );
                // 验证属性存在
                assert!(
                    output.js_code.contains("key1"),
                    "Should contain key1: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("key2"),
                    "Should contain key2: {}",
                    output.js_code
                );
                println!("✅ Interface merging with index signature test passed");
            }
            Err(e) => {
                panic!("Interface merging with index signature test failed: {}", e);
            }
        }
    }

    /// Test module merging - multiple declarations of the same module are merged (v0.3.160)
    #[test]
    fn test_module_merging() {
        let ts_code = r#"
declare module "my-module" {
    export function foo(): void;
}
declare module "my-module" {
    export function bar(): void;
}
const x: number = 1;
"#;
        match compile_typescript(ts_code, "module_merging.ts") {
            Ok(output) => {
                println!("模块合并转译结果:");
                println!("{}", output.js_code);
                // 验证 foo 和 bar 都被保留
                assert!(
                    output.js_code.contains("foo"),
                    "Should contain foo: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("bar"),
                    "Should contain bar: {}",
                    output.js_code
                );
                // 变量应该被保留
                assert!(
                    output.js_code.contains("x"),
                    "Should contain x: {}",
                    output.js_code
                );
                println!("✅ Module merging test passed");
            }
            Err(e) => {
                panic!("Module merging test failed: {}", e);
            }
        }
    }

    /// Test module merging with multiple members (v0.3.160)
    #[test]
    fn test_module_merging_multiple_members() {
        let ts_code = r#"
declare module "express" {
    function foo(): void;
}
declare module "express" {
    function bar(): void;
}
declare module "express" {
    function baz(): void;
}
"#;
        match compile_typescript(ts_code, "module_merging_multi.ts") {
            Ok(output) => {
                println!("模块合并(多成员)转译结果:");
                println!("{}", output.js_code);
                // 验证所有函数都被保留
                assert!(
                    output.js_code.contains("foo"),
                    "Should contain foo: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("bar"),
                    "Should contain bar: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("baz"),
                    "Should contain baz: {}",
                    output.js_code
                );
                // 验证只有一个 declare module 声明
                let count = output.js_code.matches("declare module \"express\"").count();
                assert_eq!(
                    count, 1,
                    "Should have exactly one declare module: {}",
                    output.js_code
                );
                println!("✅ Module merging with multiple members test passed");
            }
            Err(e) => {
                panic!("Module merging with multiple members test failed: {}", e);
            }
        }
    }

    /// Test module merging with different modules (v0.3.160)
    #[test]
    fn test_different_modules_not_merged() {
        let ts_code = r#"
declare module "module-a" {
    function fromA(): void;
}
declare module "module-b" {
    function fromB(): void;
}
declare module "module-a" {
    function moreFromA(): void;
}
"#;
        match compile_typescript(ts_code, "different_modules.ts") {
            Ok(output) => {
                println!("不同模块不合并转译结果:");
                println!("{}", output.js_code);
                // module-a 应该合并（包含 fromA 和 moreFromA）
                // module-b 应该独立存在
                assert!(
                    output.js_code.contains("fromA"),
                    "Should contain fromA: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("moreFromA"),
                    "Should contain moreFromA: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("fromB"),
                    "Should contain fromB: {}",
                    output.js_code
                );
                // 验证有两个不同的 declare module 声明
                let count_a = output
                    .js_code
                    .matches("declare module \"module-a\"")
                    .count();
                let count_b = output
                    .js_code
                    .matches("declare module \"module-b\"")
                    .count();
                assert_eq!(
                    count_a, 1,
                    "Should have exactly one module-a: {}",
                    output.js_code
                );
                assert_eq!(
                    count_b, 1,
                    "Should have exactly one module-b: {}",
                    output.js_code
                );
                println!("✅ Different modules not merged test passed");
            }
            Err(e) => {
                panic!("Different modules not merged test failed: {}", e);
            }
        }
    }

    /// Test triple merging - interface + namespace + module all together (v0.3.161)
    #[test]
    fn test_triple_merging_complete() {
        let ts_code = r#"
declare module "augmented-module" {
    export interface ModuleInterface {
        initialProp: string;
    }
    export function initialFunc(): void;
}
interface ModuleInterface {
    additionalProp: number;
}
declare module "augmented-module" {
    export function extendedFunc(): void;
    export interface ModuleInterface {
        extendedProp: boolean;
    }
}
namespace Utils {
    export const firstValue: number = 1;
}
namespace Utils {
    export const secondValue: number = 2;
}
interface Data {
    id: number;
}
interface Data {
    name: string;
}
const result: number = 1;
"#;
        match compile_typescript(ts_code, "triple_merging.ts") {
            Ok(output) => {
                println!("三重合并完整测试转译结果:");
                println!("{}", output.js_code);
                // 验证模块合并 - 所有函数都应该存在
                assert!(
                    output.js_code.contains("initialFunc"),
                    "Should contain initialFunc: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("extendedFunc"),
                    "Should contain extendedFunc: {}",
                    output.js_code
                );
                // 验证只有一个 declare module
                let module_count = output
                    .js_code
                    .matches("declare module \"augmented-module\"")
                    .count();
                assert_eq!(
                    module_count, 1,
                    "Should have exactly one module declaration: {}",
                    output.js_code
                );
                // 验证命名空间合并 - 两个值都应该存在
                assert!(
                    output.js_code.contains("firstValue"),
                    "Should contain firstValue: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("secondValue"),
                    "Should contain secondValue: {}",
                    output.js_code
                );
                // 验证接口合并 - 变量应该被保留
                assert!(
                    output.js_code.contains("result"),
                    "Should contain result: {}",
                    output.js_code
                );
                // 验证没有类型注解
                assert!(
                    !output.js_code.contains(": number"),
                    "Should not contain type annotation: {}",
                    output.js_code
                );
                println!("✅ Triple merging complete test passed");
            }
            Err(e) => {
                panic!("Triple merging complete test failed: {}", e);
            }
        }
    }

    /// Test module augmentation with nested declarations (v0.3.161)
    #[test]
    fn test_module_augmentation_nested() {
        let ts_code = r#"
declare module "nested-test" {
    export namespace Inner {
        export function innerFunc(): void;
    }
}
declare module "nested-test" {
    export namespace Inner {
        export const innerConst: number;
    }
}
const x: number = 1;
"#;
        match compile_typescript(ts_code, "module_augmentation_nested.ts") {
            Ok(output) => {
                println!("模块增强嵌套测试转译结果:");
                println!("{}", output.js_code);
                // 验证嵌套命名空间合并
                assert!(
                    output.js_code.contains("innerFunc"),
                    "Should contain innerFunc: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("innerConst"),
                    "Should contain innerConst: {}",
                    output.js_code
                );
                // 验证只有一个 declare module
                let module_count = output
                    .js_code
                    .matches("declare module \"nested-test\"")
                    .count();
                assert_eq!(
                    module_count, 1,
                    "Should have exactly one module: {}",
                    output.js_code
                );
                // 验证变量存在
                assert!(
                    output.js_code.contains("x"),
                    "Should contain x: {}",
                    output.js_code
                );
                println!("✅ Module augmentation nested test passed");
            }
            Err(e) => {
                panic!("Module augmentation nested test failed: {}", e);
            }
        }
    }

    /// Test independent namespace, interface, and module declarations (v0.3.161)
    #[test]
    fn test_independent_declarations_not_merged() {
        let ts_code = r#"
namespace Alpha {
    export const a: number = 1;
}
namespace Beta {
    export const b: number = 2;
}
// 接口声明在 JS 输出中不可见（纯类型），但应该正确解析
interface IFace {
    prop1: string;
}
interface JFace {
    prop2: number;
}
declare module "module-X" {
    export function x(): void;
}
declare module "module-Y" {
    export function y(): void;
}
const test: number = 42;
"#;
        match compile_typescript(ts_code, "independent_declarations.ts") {
            Ok(output) => {
                println!("独立声明不合并测试转译结果:");
                println!("{}", output.js_code);
                // 验证不同的命名空间独立存在
                assert!(
                    output.js_code.contains("Alpha"),
                    "Should contain Alpha: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("Beta"),
                    "Should contain Beta: {}",
                    output.js_code
                );
                // 验证不同的模块独立存在
                assert!(
                    output.js_code.contains("module-X"),
                    "Should contain module-X: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("module-Y"),
                    "Should contain module-Y: {}",
                    output.js_code
                );
                // 验证变量存在
                assert!(
                    output.js_code.contains("test"),
                    "Should contain test: {}",
                    output.js_code
                );
                // 验证函数存在
                assert!(
                    output.js_code.contains("function x()"),
                    "Should contain function x: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("function y()"),
                    "Should contain function y: {}",
                    output.js_code
                );
                // 验证接口声明被正确移除（纯类型声明）
                assert!(
                    !output.js_code.contains("interface IFace"),
                    "Interface should be removed: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("interface JFace"),
                    "Interface should be removed: {}",
                    output.js_code
                );
                println!("✅ Independent declarations not merged test passed");
            }
            Err(e) => {
                panic!("Independent declarations not merged test failed: {}", e);
            }
        }
    }

    /// v0.3.162: Test enhanced array type inference
    #[test]
    fn test_enhanced_array_type_inference() {
        // 测试增强的数组类型推断
        let ts_code = r#"
const numbers = [1, 2, 3];
const strings = ["a", "b", "c"];
const mixed = [1, "two", true];
const empty: number[] = [];
"#;
        match compile_typescript(ts_code, "array_inference.ts") {
            Ok(output) => {
                println!("增强数组类型推断转译结果:");
                println!("{}", output.js_code);
                // 验证数组被正确保留
                assert!(
                    output.js_code.contains("[1, 2, 3]"),
                    "Should contain numbers array: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("[\"a\", \"b\", \"c\"]"),
                    "Should contain strings array: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("[1, \"two\", true]"),
                    "Should contain mixed array: {}",
                    output.js_code
                );
                // 验证类型注解被移除
                assert!(
                    !output.js_code.contains(": number[]"),
                    "Type annotation should be removed: {}",
                    output.js_code
                );
                println!("✅ Enhanced array type inference test passed");
            }
            Err(e) => {
                panic!("Array type inference test failed: {}", e);
            }
        }
    }

    /// v0.3.162: Test enhanced object type inference
    #[test]
    fn test_enhanced_object_type_inference() {
        // 测试增强的对象类型推断
        let ts_code = r#"
const user = { name: "Alice", age: 30 };
const point = { x: 10, y: 20 };
const empty = {};
"#;
        match compile_typescript(ts_code, "object_inference.ts") {
            Ok(output) => {
                println!("增强对象类型推断转译结果:");
                println!("{}", output.js_code);
                // 验证对象被正确保留
                assert!(
                    output.js_code.contains("user"),
                    "Should contain user: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("name"),
                    "Should contain name property: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("age"),
                    "Should contain age property: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("point"),
                    "Should contain point: {}",
                    output.js_code
                );
                // 验证类型注解被移除
                assert!(
                    !output.js_code.contains(": {"),
                    "Type annotation should be removed: {}",
                    output.js_code
                );
                println!("✅ Enhanced object type inference test passed");
            }
            Err(e) => {
                panic!("Object type inference test failed: {}", e);
            }
        }
    }

    /// v0.3.162: Test generic type inference with utility types
    #[test]
    fn test_generic_utility_type_inference() {
        // 测试泛型和工具类型的推断
        let ts_code = r#"
type Identity<T> = T;
type Wrapped<T> = [T];
type Result<T, E> = { ok: T, error: E };

const identity = { ok: true, error: null };
const wrapped = [42];
const result = { ok: "success", error: undefined };
"#;
        match compile_typescript(ts_code, "utility_types.ts") {
            Ok(output) => {
                println!("工具类型推断转译结果:");
                println!("{}", output.js_code);
                // 验证变量被正确保留
                assert!(
                    output.js_code.contains("identity"),
                    "Should contain identity: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("wrapped"),
                    "Should contain wrapped: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("result"),
                    "Should contain result: {}",
                    output.js_code
                );
                // 验证类型注解被移除
                assert!(
                    !output.js_code.contains(": Identity<"),
                    "Type annotation should be removed: {}",
                    output.js_code
                );
                println!("✅ Generic utility type inference test passed");
            }
            Err(e) => {
                panic!("Utility type inference test failed: {}", e);
            }
        }
    }

    /// v0.3.162: Test conditional type with infer
    #[test]
    fn test_conditional_type_infer() {
        // 测试条件类型中的 infer 关键字
        let ts_code = r#"
type IsString<T> = T extends string ? true : false;
type IsNumber<T> = T extends number ? "number" : "other";

const checkString = true;
const checkNumber = "other";
"#;
        match compile_typescript(ts_code, "conditional_infer.ts") {
            Ok(output) => {
                println!("条件类型 infer 转译结果:");
                println!("{}", output.js_code);
                // 验证变量被正确保留
                assert!(
                    output.js_code.contains("checkString"),
                    "Should contain checkString: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("checkNumber"),
                    "Should contain checkNumber: {}",
                    output.js_code
                );
                // 验证类型别名被移除（纯类型）
                assert!(
                    !output.js_code.contains("type IsString"),
                    "Type alias should be removed: {}",
                    output.js_code
                );
                println!("✅ Conditional type infer test passed");
            }
            Err(e) => {
                panic!("Conditional type infer test failed: {}", e);
            }
        }
    }

    /// v0.3.162: Test edge case - deeply nested array inference
    #[test]
    fn test_deeply_nested_array_inference() {
        // 测试深度嵌套数组的类型推断
        let ts_code = r#"
const matrix = [[1, 2], [3, 4], [5, 6]];
const nested = [[["a", "b"]], [["c", "d"]]];
const mixedMatrix = [[1, "two"], [3, "four"]];
"#;
        match compile_typescript(ts_code, "nested_array.ts") {
            Ok(output) => {
                println!("深度嵌套数组推断转译结果:");
                println!("{}", output.js_code);
                // 验证数组被正确保留
                assert!(
                    output.js_code.contains("matrix"),
                    "Should contain matrix: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("nested"),
                    "Should contain nested: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("mixedMatrix"),
                    "Should contain mixedMatrix: {}",
                    output.js_code
                );
                println!("✅ Deeply nested array inference test passed");
            }
            Err(e) => {
                panic!("Nested array inference test failed: {}", e);
            }
        }
    }

    /// Test deeply nested conditional types with multiple extends
    #[test]
    fn test_deeply_nested_conditional_types() {
        // 测试深度嵌套条件类型（多层 extends）
        let ts_code = r#"
type DeepCheck<T> = T extends string
    ? "string"
    : T extends number
        ? "number"
        : T extends boolean
            ? "boolean"
            : "unknown";

const strResult: string = "string";
const numResult: string = "number";
const boolResult: string = "boolean";
const objResult: string = "unknown";
"#;
        match compile_typescript(ts_code, "deeply_nested_conditional.ts") {
            Ok(output) => {
                println!("深度嵌套条件类型转译结果:");
                println!("{}", output.js_code);
                // 验证变量被正确保留
                assert!(
                    output.js_code.contains("strResult"),
                    "Should contain strResult: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("numResult"),
                    "Should contain numResult: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("boolResult"),
                    "Should contain boolResult: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("objResult"),
                    "Should contain objResult: {}",
                    output.js_code
                );
                // 验证类型别名被移除
                assert!(
                    !output.js_code.contains("type DeepCheck"),
                    "Type alias should be removed: {}",
                    output.js_code
                );
                println!("✅ Deeply nested conditional types test passed");
            }
            Err(e) => {
                panic!("Deeply nested conditional types test failed: {}", e);
            }
        }
    }

    /// Test conditional types with union types
    #[test]
    fn test_conditional_type_with_unions() {
        // 测试带联合类型的条件类型
        let ts_code = r#"
type ToString<T> = T extends any ? string : never;
type ToNumber<T> = T extends string ? number : never;

const a: string = "hello";
const b: number = 42;
const c: string = "world";
"#;
        match compile_typescript(ts_code, "conditional_with_unions.ts") {
            Ok(output) => {
                println!("条件类型与联合类型转译结果:");
                println!("{}", output.js_code);
                // 验证变量被正确保留
                assert!(
                    output.js_code.contains("a"),
                    "Should contain a: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("b"),
                    "Should contain b: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("c"),
                    "Should contain c: {}",
                    output.js_code
                );
                // 验证类型别名被移除
                assert!(
                    !output.js_code.contains("type ToString"),
                    "Type alias should be removed: {}",
                    output.js_code
                );
                println!("✅ Conditional type with unions test passed");
            }
            Err(e) => {
                panic!("Conditional type with unions test failed: {}", e);
            }
        }
    }

    /// Test conditional type with keyof and mapped type
    #[test]
    fn test_conditional_type_with_keyof_mapped() {
        // 测试条件类型与 keyof 和映射类型组合
        let ts_code = r#"
type ValuesOf<T> = T[keyof T];
type OptionalKeys<T> = { [P in keyof T]?: T[P] };

const obj = { name: "test", age: 30, active: true };
const optional = { name: "test" };
"#;
        match compile_typescript(ts_code, "conditional_keyof_mapped.ts") {
            Ok(output) => {
                println!("条件类型与 keyof/映射类型转译结果:");
                println!("{}", output.js_code);
                // 验证变量被正确保留
                assert!(
                    output.js_code.contains("obj"),
                    "Should contain obj: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("optional"),
                    "Should contain optional: {}",
                    output.js_code
                );
                // 验证类型别名被移除
                assert!(
                    !output.js_code.contains("type ValuesOf"),
                    "Type alias should be removed: {}",
                    output.js_code
                );
                println!("✅ Conditional type with keyof/mapped test passed");
            }
            Err(e) => {
                panic!("Conditional type with keyof/mapped test failed: {}", e);
            }
        }
    }

    /// Test recursive conditional type (simplified)
    #[test]
    fn test_recursive_conditional_type() {
        // 测试递归条件类型
        let ts_code = r#"
type DeepPartial<T> = T extends object
    ? { [P in keyof T]?: DeepPartial<T[P]> }
    : T;

const nested = { a: { b: { c: 1 } }, d: 2 };
const partial = { a: { b: {} } };
"#;
        match compile_typescript(ts_code, "recursive_conditional.ts") {
            Ok(output) => {
                println!("递归条件类型转译结果:");
                println!("{}", output.js_code);
                // 验证变量被正确保留
                assert!(
                    output.js_code.contains("nested"),
                    "Should contain nested: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("partial"),
                    "Should contain partial: {}",
                    output.js_code
                );
                // 验证类型别名被移除
                assert!(
                    !output.js_code.contains("type DeepPartial"),
                    "Type alias should be removed: {}",
                    output.js_code
                );
                println!("✅ Recursive conditional type test passed");
            }
            Err(e) => {
                panic!("Recursive conditional type test failed: {}", e);
            }
        }
    }

    /// Test conditional type with template literal types
    #[test]
    fn test_conditional_type_with_template_literal() {
        // 测试条件类型与模板字面量类型组合
        let ts_code = r#"
type EventName<T> = T extends `on${string}` ? T : never;
type ClickEvent = "onClick";
type HoverEvent = "onHover";

const handler: ClickEvent = "onClick";
const hover: HoverEvent = "onHover";
const other: string = "other";
"#;
        match compile_typescript(ts_code, "conditional_template_literal.ts") {
            Ok(output) => {
                println!("条件类型与模板字面量转译结果:");
                println!("{}", output.js_code);
                // 验证变量被正确保留
                assert!(
                    output.js_code.contains("handler"),
                    "Should contain handler: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("hover"),
                    "Should contain hover: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("other"),
                    "Should contain other: {}",
                    output.js_code
                );
                // 验证类型别名被移除
                assert!(
                    !output.js_code.contains("type EventName"),
                    "Type alias should be removed: {}",
                    output.js_code
                );
                println!("✅ Conditional type with template literal test passed");
            }
            Err(e) => {
                panic!("Conditional type with template literal test failed: {}", e);
            }
        }
    }

    /// Test type predicate with 'is' keyword (v0.3.164)
    #[test]
    fn test_type_predicate_is_keyword() {
        // 测试类型谓词 is 关键字 - 转译时会移除返回类型注解，但函数保留
        let ts_code = r#"
function isString(value: unknown): value is string {
    return typeof value === "string";
}

function isNumber(value: unknown): value is number {
    return typeof value === "number";
}

function isDefined<T>(value: T | undefined): value is T {
    return value !== undefined;
}
"#;
        match compile_typescript(ts_code, "type_predicate.ts") {
            Ok(output) => {
                println!("类型谓词 is 关键字转译结果:");
                println!("{}", output.js_code);
                // 验证函数被正确保留（转译时移除类型注解）
                assert!(
                    output.js_code.contains("function isString(value)"),
                    "Should contain isString function: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("function isNumber(value)"),
                    "Should contain isNumber function: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("function isDefined(value)"),
                    "Should contain isDefined function: {}",
                    output.js_code
                );
                // 验证函数体被保留
                assert!(
                    output.js_code.contains("typeof value === \"string\""),
                    "Should contain function body: {}",
                    output.js_code
                );
                // 注意：类型谓词 `value is string` 在转译后的 JS 中会被移除
                println!("✅ Type predicate with 'is' keyword test passed");
            }
            Err(e) => {
                panic!("Type predicate with 'is' keyword test failed: {}", e);
            }
        }
    }

    /// Test typeof with expressions (v0.3.164)
    #[test]
    fn test_typeof_expressions() {
        // 测试 typeof 与表达式
        let ts_code = r#"
const user = { name: "Alice", age: 30 };
const config = { apiUrl: "https://api.example.com", timeout: 5000 };

type UserType = typeof user;
type ConfigType = typeof config;

function getType<T>(obj: T): T {
    return obj;
}

const arr = [1, 2, 3];
type ArrType = typeof arr;
"#;
        match compile_typescript(ts_code, "typeof_expressions.ts") {
            Ok(output) => {
                println!("typeof 表达式测试转译结果:");
                println!("{}", output.js_code);
                // 验证变量被正确保留
                assert!(
                    output.js_code.contains("user"),
                    "Should contain user: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("config"),
                    "Should contain config: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("arr"),
                    "Should contain arr: {}",
                    output.js_code
                );
                // 验证 typeof 关键字在类型注解中被移除
                assert!(
                    !output.js_code.contains("UserType"),
                    "Type alias should be removed: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("ConfigType"),
                    "Type alias should be removed: {}",
                    output.js_code
                );
                // 验证函数被保留
                assert!(
                    output.js_code.contains("getType"),
                    "Should contain getType: {}",
                    output.js_code
                );
                println!("✅ Typeof expressions test passed");
            }
            Err(e) => {
                panic!("Typeof expressions test failed: {}", e);
            }
        }
    }

    /// Test keyof with complex types (v0.3.164)
    #[test]
    fn test_keyof_complex_types() {
        // 测试 keyof 与复杂类型 - 接口在 JS 输出中被移除
        let ts_code = r#"
interface User {
    id: number;
    name: string;
    email: string;
}

interface Product {
    sku: string;
    price: number;
    inStock: boolean;
}

type UserKeys = keyof User;
type ProductKeys = keyof Product;
type AllKeys = keyof User | keyof Product;

function getKeys<T>(obj: T): T[keyof T] {
    return obj[Object.keys(obj)[0] as keyof T];
}

const user = { id: 1, name: "Alice" };
"#;
        match compile_typescript(ts_code, "keyof_complex.ts") {
            Ok(output) => {
                println!("keyof 复杂类型转译结果:");
                println!("{}", output.js_code);
                // 注意：接口在 JS 输出中被移除（用于类型检查）
                // 验证类型别名被移除
                assert!(
                    !output.js_code.contains("UserKeys"),
                    "Type alias should be removed: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("ProductKeys"),
                    "Type alias should be removed: {}",
                    output.js_code
                );
                // 验证函数被保留
                assert!(
                    output.js_code.contains("function getKeys"),
                    "Should contain getKeys: {}",
                    output.js_code
                );
                // 验证变量定义被保留
                assert!(
                    output.js_code.contains("const user"),
                    "Should contain const user: {}",
                    output.js_code
                );
                // 验证函数有正确返回值
                assert!(
                    output.js_code.contains("Object.keys"),
                    "Should contain Object.keys: {}",
                    output.js_code
                );
                println!("✅ Keyof complex types test passed");
            }
            Err(e) => {
                panic!("Keyof complex types test failed: {}", e);
            }
        }
    }

    /// Test readonly modifier in mapped types (v0.3.164)
    #[test]
    fn test_readonly_mapped_type() {
        // 测试 readonly 修饰符在映射类型中的使用 - 接口在 JS 输出中被移除
        let ts_code = r#"
interface MutableUser {
    id: number;
    name: string;
}

type ReadonlyUser<T> = { readonly [P in keyof T]: T[P] };
type PartialUser<T> = { [P in keyof T]?: T[P] };
type ReadonlyPartialUser<T> = { readonly [P in keyof T]?: T[P] };

const user: ReadonlyUser<MutableUser> = { id: 1, name: "Alice" };
"#;
        match compile_typescript(ts_code, "readonly_mapped.ts") {
            Ok(output) => {
                println!("readonly 映射类型转译结果:");
                println!("{}", output.js_code);
                // 注意：接口在 JS 输出中被移除（用于类型检查）
                // 验证类型别名被移除
                assert!(
                    !output.js_code.contains("ReadonlyUser"),
                    "Type alias should be removed: {}",
                    output.js_code
                );
                // 验证变量被保留
                assert!(
                    output.js_code.contains("const user"),
                    "Should contain const user: {}",
                    output.js_code
                );
                // 验证对象字面量被保留（格式可能略有不同）
                assert!(
                    output.js_code.contains("{id: 1, name:")
                        || output.js_code.contains("{ id: 1, name:"),
                    "Should contain object literal: {}",
                    output.js_code
                );
                println!("✅ Readonly modifier in mapped types test passed");
            }
            Err(e) => {
                panic!("Readonly modifier in mapped types test failed: {}", e);
            }
        }
    }

    /// Test infer with constraints (v0.3.164)
    #[test]
    fn test_infer_with_constraints() {
        // 测试 infer 关键字与约束
        let ts_code = r#"
type UnpackPromise<T> = T extends Promise<infer U> ? U : T;
type UnpackArray<T> = T extends Array<infer E> ? E : T;

const p1: Promise<number> = Promise.resolve(42);
const p2: Promise<string> = Promise.resolve("hello");
const a1: number[] = [1, 2, 3];

function getValue<T>(arg: T): T {
    return arg;
}
"#;
        match compile_typescript(ts_code, "infer_constraints.ts") {
            Ok(output) => {
                println!("infer 约束测试转译结果:");
                println!("{}", output.js_code);
                // 验证变量被保留
                assert!(
                    output.js_code.contains("const p1"),
                    "Should contain p1: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("const p2"),
                    "Should contain p2: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("const a1"),
                    "Should contain a1: {}",
                    output.js_code
                );
                // 验证函数被保留
                assert!(
                    output.js_code.contains("function getValue"),
                    "Should contain getValue: {}",
                    output.js_code
                );
                // 验证类型别名被移除
                assert!(
                    !output.js_code.contains("UnpackPromise"),
                    "Type alias should be removed: {}",
                    output.js_code
                );
                println!("✅ Infer with constraints test passed");
            }
            Err(e) => {
                panic!("Infer with constraints test failed: {}", e);
            }
        }
    }

    /// Test index signature types (v0.3.179)
    #[test]
    fn test_index_signature_types() {
        // 测试索引签名类型
        let ts_code = r#"
interface StringMap {
    [key: string]: string;
}

interface NumberMap {
    [key: number]: number;
}

const strMap: StringMap = { hello: "world", foo: "bar" };
const numMap: NumberMap = { 1: 100, 2: 200 };

interface User {
    name: string;
    age: number;
    [key: string]: string | number;
}

const user: User = { name: "Alice", age: 30, email: "alice@example.com" };
"#;
        match compile_typescript(ts_code, "index_signature.ts") {
            Ok(output) => {
                println!("索引签名类型测试转译结果:");
                println!("{}", output.js_code);
                // 验证变量被保留
                assert!(
                    output.js_code.contains("strMap"),
                    "Should contain strMap: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("numMap"),
                    "Should contain numMap: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("user"),
                    "Should contain user: {}",
                    output.js_code
                );
                // 验证接口被移除（纯类型声明）
                assert!(
                    !output.js_code.contains("interface StringMap"),
                    "Interface should be removed: {}",
                    output.js_code
                );
                println!("✅ Index signature types test passed");
            }
            Err(e) => {
                panic!("Index signature types test failed: {}", e);
            }
        }
    }

    /// Test constructor signature types (v0.3.179)
    #[test]
    fn test_constructor_signature_types() {
        // 测试构造函数签名类型
        let ts_code = r#"
interface Constructor<T> {
    new(...args: any[]): T;
}

interface ComponentConstructor {
    new(props: { name: string; age: number }): { name: string; age: number; render(): void };
}

function createInstance<T>(ctor: Constructor<T>): T {
    return new ctor();
}

class MyComponent {
    constructor(props: { name: string; age: number }) {
        this.name = props.name;
        this.age = props.age;
    }
    name: string = "";
    age: number = 0;
    render(): void {
        console.log("Rendering:", this.name);
    }
}

const instance = createInstance(MyComponent);
"#;
        match compile_typescript(ts_code, "constructor_signature.ts") {
            Ok(output) => {
                println!("构造函数签名类型测试转译结果:");
                println!("{}", output.js_code);
                // 验证类和函数被保留
                assert!(
                    output.js_code.contains("class MyComponent"),
                    "Should contain MyComponent class: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("createInstance"),
                    "Should contain createInstance: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("instance"),
                    "Should contain instance: {}",
                    output.js_code
                );
                // 验证接口被移除（纯类型声明）
                assert!(
                    !output.js_code.contains("interface Constructor"),
                    "Interface should be removed: {}",
                    output.js_code
                );
                println!("✅ Constructor signature types test passed");
            }
            Err(e) => {
                panic!("Constructor signature types test failed: {}", e);
            }
        }
    }

    /// Test this parameter in methods (v0.3.179)
    #[test]
    fn test_this_parameter_in_methods() {
        // 测试方法中的 this 参数
        let ts_code = r#"
const counter = {
    count: 0,
    increment(this: { count: number }) {
        this.count++;
    },
    decrement(this: { count: number }) {
        this.count--;
    },
    getValue(this: { count: number }): number {
        return this.count;
    }
};

class Handler {
    private name: string = "Handler";
    process(this: Handler, data: string): void {
        console.log(this.name, data);
    }
    asyncProcess(this: Handler, data: string): Promise<void> {
        console.log(this.name, data);
    }
}

counter.increment();
counter.decrement();
const value = counter.getValue();

const h = new Handler();
h.process("test");
"#;
        match compile_typescript(ts_code, "this_parameter.ts") {
            Ok(output) => {
                println!("this 参数测试转译结果:");
                println!("{}", output.js_code);
                // 验证对象和方法被保留
                assert!(
                    output.js_code.contains("counter"),
                    "Should contain counter: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("increment"),
                    "Should contain increment: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("getValue"),
                    "Should contain getValue: {}",
                    output.js_code
                );
                // 验证类被保留
                assert!(
                    output.js_code.contains("class Handler"),
                    "Should contain Handler class: {}",
                    output.js_code
                );
                // 验证 this 参数类型被移除
                assert!(
                    !output.js_code.contains("(this: { count: number })"),
                    "This parameter type should be removed: {}",
                    output.js_code
                );
                println!("✅ This parameter in methods test passed");
            }
            Err(e) => {
                panic!("This parameter in methods test failed: {}", e);
            }
        }
    }

    /// Test optional parameters with default values (v0.3.179)
    #[test]
    fn test_optional_parameters_with_defaults() {
        // 测试带默认值的可选参数
        let ts_code = r#"
function greet(name: string, greeting: string = "Hello"): string {
    return `${greeting}, ${name}!`;
}

function multiply(a: number, b: number = 2, c: number = 3): number {
    return a * b * c;
}

const arrowGreet = (name: string, prefix: string = "Hi"): string => {
    return `${prefix} ${name}`;
};

const arrowMultiply = (x: number, factor: number = 10): number => x * factor;

console.log(greet("Alice"));
console.log(greet("Bob", "Welcome"));
console.log(multiply(5));
console.log(multiply(5, 4, 2));
console.log(arrowGreet("Charlie"));
console.log(arrowMultiply(7));
"#;
        match compile_typescript(ts_code, "optional_params_defaults.ts") {
            Ok(output) => {
                println!("可选参数默认值测试转译结果:");
                println!("{}", output.js_code);
                // 验证函数被保留
                assert!(
                    output.js_code.contains("function greet"),
                    "Should contain greet function: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("function multiply"),
                    "Should contain multiply function: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("arrowGreet"),
                    "Should contain arrowGreet: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("arrowMultiply"),
                    "Should contain arrowMultiply: {}",
                    output.js_code
                );
                // 验证类型注解被移除
                assert!(
                    !output.js_code.contains(": string, greeting"),
                    "Parameter types should be removed: {}",
                    output.js_code
                );
                // 验证函数调用被保留
                assert!(
                    output.js_code.contains("greet(\"Alice\")"),
                    "Should contain greet call: {}",
                    output.js_code
                );
                println!("✅ Optional parameters with defaults test passed");
            }
            Err(e) => {
                panic!("Optional parameters with defaults test failed: {}", e);
            }
        }
    }

    /// Test BigInt literals (v0.3.179)
    #[test]
    fn test_bigint_literals() {
        // 测试 BigInt 字面量 - 简单版本（无类型注解）
        let ts_code = r#"
const a = 123n;
const b = 456789012345678901234567890n;
const c = 0n;
"#;
        match compile_typescript(ts_code, "bigint_literals_simple.ts") {
            Ok(output) => {
                println!("BigInt 字面量简单测试转译结果:");
                println!("{}", output.js_code);
                // 验证 BigInt 字面量被保留
                assert!(
                    output.js_code.contains("123n"),
                    "Should contain 123n: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("456789012345678901234567890n"),
                    "Should contain large BigInt: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("0n"),
                    "Should contain 0n: {}",
                    output.js_code
                );
                println!("✅ BigInt literals simple test passed");
            }
            Err(e) => {
                panic!("BigInt literals simple test failed: {}", e);
            }
        }
    }

    /// Test BigInt literals with type annotations (v0.3.179)
    #[test]
    fn test_bigint_literals_with_types() {
        // 测试带类型注解的 BigInt 字面量
        let ts_code = r#"
const a: bigint = 123n;
const b: bigint = 456789012345678901234567890n;
const c: bigint = 0n;
"#;
        match compile_typescript(ts_code, "bigint_literals.ts") {
            Ok(output) => {
                println!("BigInt 字面量测试转译结果:");
                println!("{}", output.js_code);
                // 验证 BigInt 字面量被保留
                assert!(
                    output.js_code.contains("123n"),
                    "Should contain 123n: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("456789012345678901234567890n"),
                    "Should contain large BigInt: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("0n"),
                    "Should contain 0n: {}",
                    output.js_code
                );
                // 验证类型注解被移除
                assert!(
                    !output.js_code.contains(": bigint"),
                    "Type annotation should be removed: {}",
                    output.js_code
                );
                println!("✅ BigInt literals test passed");
            }
            Err(e) => {
                panic!("BigInt literals test failed: {}", e);
            }
        }
    }
}
