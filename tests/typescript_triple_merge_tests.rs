// 三重合并测试 - TypeScript 编译增强
// 测试 interface + namespace 同名合并（module 关键字有解析复杂性）

#[cfg(test)]
mod tests {
    use amberjs::typescript::compile_typescript;

    /// 测试1: 基础三重合并 - interface + namespace
    /// 同名的 interface 和 namespace 会被合并
    #[test]
    fn test_triple_merge_interface_namespace() {
        let ts_code = r#"
interface Foo {
    bar: string;
}
namespace Foo {
    export function baz(): void {}
}
const result = Foo;
"#;
        let result = compile_typescript(ts_code, "test.ts");
        assert!(
            result.is_ok(),
            "Triple merge should compile successfully, error: {:?}",
            result.err()
        );
        let output = result.unwrap();
        // 验证代码能够编译，且 Foo 命名空间被保留
        assert!(
            output.js_code.contains("Foo") || output.js_code.contains("var Foo"),
            "Should preserve Foo declaration: {}",
            output.js_code
        );
        println!("✅ Test 1: Triple merge interface + namespace");
    }

    /// 测试2: 多个同名的 interface + namespace 合并
    #[test]
    fn test_multiple_triple_merge() {
        let ts_code = r#"
interface Point {
    x: number;
}
namespace Point {
    export function create(x: number): any {
        return { x: x, y: 0 };
    }
}
interface Point {
    y: number;
}
namespace Point {
    export function distance(p1: any, p2: any): number {
        return 0;
    }
}
const p = Point.create(1);
const d = Point.distance(p, p);
"#;
        let result = compile_typescript(ts_code, "test.ts");
        assert!(
            result.is_ok(),
            "Multiple triple merge should compile, error: {:?}",
            result.err()
        );
        let output = result.unwrap();
        // 验证 Point 命名空间被正确保留
        assert!(
            output.js_code.contains("Point"),
            "Should preserve Point declaration: {}",
            output.js_code
        );
        println!("✅ Test 2: Multiple triple merge with same name");
        println!("Output: {}", output.js_code);
    }

    /// 测试3: namespace 多次合并
    #[test]
    fn test_namespace_multiple_merge() {
        let ts_code = r#"
namespace Utils {
    export const version = "1.0";
}
namespace Utils {
    export function help(): string {
        return "help";
    }
}
const v = Utils.version;
"#;
        let result = compile_typescript(ts_code, "test.ts");
        assert!(result.is_ok(), "Namespace merge should compile");
        let output = result.unwrap();
        assert!(
            output.js_code.contains("Utils"),
            "Should preserve Utils namespace: {}",
            output.js_code
        );
        println!("✅ Test 3: Namespace multiple merge");
    }

    /// 测试4: interface 多次合并
    #[test]
    fn test_interface_multiple_merge() {
        let ts_code = r#"
interface Config {
    debug: boolean;
}
interface Config {
    timeout: number;
}
const config: Config = { debug: true, timeout: 30 };
"#;
        let result = compile_typescript(ts_code, "test.ts");
        assert!(result.is_ok(), "Interface merge should compile");
        println!("✅ Test 4: Interface multiple merge");
    }

    /// 测试5: interface + namespace + declare 组合
    #[test]
    fn test_triple_merge_with_declare() {
        let ts_code = r#"
declare interface Window {
    myPlugin: any;
}
declare namespace Window {
    export function init(): void;
}
"#;
        let result = compile_typescript(ts_code, "declare.ts");
        assert!(
            result.is_ok(),
            "Declare triple merge should compile, error: {:?}",
            result.err()
        );
        let output = result.unwrap();
        // Declare 声明应该被转译
        println!("✅ Test 5: Triple merge with declare");
        println!("Output: {}", output.js_code);
    }

    /// 测试6: 嵌套命名空间合并
    #[test]
    fn test_nested_namespace_merge() {
        let ts_code = r#"
namespace Outer {
    export const a = 1;
}
namespace Outer {
    export const b = 2;
}
namespace Outer.Inner {
    export const c = 3;
}
"#;
        let result = compile_typescript(ts_code, "test.ts");
        assert!(result.is_ok(), "Nested namespace merge should compile");
        println!("✅ Test 6: Nested namespace merge");
    }

    /// 测试7: interface 继承合并
    #[test]
    fn test_interface_extends_merge() {
        let ts_code = r#"
interface Base {
    id: number;
}
interface Derived extends Base {
    name: string;
}
interface Derived {
    age: number;
}
const obj: Derived = { id: 1, name: "test", age: 30 };
"#;
        let result = compile_typescript(ts_code, "test.ts");
        assert!(result.is_ok(), "Interface extends merge should compile");
        println!("✅ Test 7: Interface extends merge");
    }
}
