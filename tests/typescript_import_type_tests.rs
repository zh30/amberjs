#[cfg(test)]
mod typescript_import_type_tests {
    use amberjs::typescript::compile_typescript;

    /// Test import type (v0.3.166)
    /// import type 用于仅导入类型，在编译时会被移除
    #[test]
    fn test_import_type_basic() {
        let ts_code = r#"
import type { User } from "./types";

interface User {
    id: number;
    name: string;
}

const user = { id: 1, name: "test" };
console.log(user);
"#;
        match compile_typescript(ts_code, "import_type_basic.ts") {
            Ok(output) => {
                println!("import type 基础转译结果:");
                println!("{}", output.js_code);
                // import type 语句应该被移除
                assert!(
                    !output.js_code.contains("import type"),
                    "Should remove import type: {}",
                    output.js_code
                );
                // 运行时代码应该保留
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                // 变量声明应该保留
                assert!(
                    output.js_code.contains("const user"),
                    "Should contain user variable: {}",
                    output.js_code
                );
                println!("✅ Basic import type test passed");
            }
            Err(e) => {
                panic!("Basic import type test failed: {}", e);
            }
        }
    }

    /// Test import type with default export (v0.3.166)
    #[test]
    fn test_import_type_default() {
        let ts_code = r#"
import type MyClass from "./my-class";

class MyClass {
    static value = 42;
}

const instance = new MyClass();
console.log(MyClass.value);
"#;
        match compile_typescript(ts_code, "import_type_default.ts") {
            Ok(output) => {
                println!("import type 默认导入转译结果:");
                println!("{}", output.js_code);
                // import type 应该被移除
                assert!(
                    !output.js_code.contains("import type"),
                    "Should remove import type: {}",
                    output.js_code
                );
                // 类定义应该保留
                assert!(
                    output.js_code.contains("class MyClass"),
                    "Should contain class MyClass: {}",
                    output.js_code
                );
                println!("✅ Import type default test passed");
            }
            Err(e) => {
                panic!("Import type default test failed: {}", e);
            }
        }
    }

    /// Test export type alias (v0.3.166)
    /// export type 用于导出类型，在编译时应该移除
    #[test]
    fn test_export_type_alias() {
        let ts_code = r#"
export type { User };

type User = {
    id: number;
    name: string;
};

const user = { id: 1, name: "export" };
console.log(user);
"#;
        match compile_typescript(ts_code, "export_type.ts") {
            Ok(output) => {
                println!("export type 转译结果:");
                println!("{}", output.js_code);
                // export type { User } 应该被移除
                assert!(
                    !output.js_code.contains("export type"),
                    "Should remove export type: {}",
                    output.js_code
                );
                // 但 type User = ... 应该被转译为注释或保留
                // 当前实现会移除所有类型声明，只保留运行时代码
                // 运行时代码应该保留
                assert!(
                    output.js_code.contains("const user"),
                    "Should contain user variable: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ Export type test passed");
            }
            Err(e) => {
                panic!("Export type test failed: {}", e);
            }
        }
    }

    /// Test mixed import and import type (v0.3.166)
    #[test]
    fn test_mixed_import_and_type() {
        let ts_code = r#"
import { func } from "./utils";
import type { TypeOnly } from "./types";

function func(x) {
    return String(x);
}

const value = { data: "test" };
console.log(func(42), value);
"#;
        match compile_typescript(ts_code, "mixed_import.ts") {
            Ok(output) => {
                println!("混合导入转译结果:");
                println!("{}", output.js_code);
                // 本地 function func 会遮蔽 import，oxc 可能去掉未使用的 import。
                // 这里只要求类型导入被擦掉，运行时代码还在。
                // import type 应该被移除
                assert!(
                    !output.js_code.contains("import type"),
                    "Should remove import type: {}",
                    output.js_code
                );
                // 运行时代码应该保留
                assert!(
                    output.js_code.contains("function func"),
                    "Should contain func function: {}",
                    output.js_code
                );
                println!("✅ Mixed import test passed");
            }
            Err(e) => {
                panic!("Mixed import test failed: {}", e);
            }
        }
    }

    /// Test import side effect (v0.3.166)
    /// import "module" 用于副作用，不应该被移除
    #[test]
    fn test_import_side_effect() {
        let ts_code = r#"
import "./side-effect";

console.log("hello");
"#;
        match compile_typescript(ts_code, "import_side_effect.ts") {
            Ok(output) => {
                println!("副作用导入转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("import") && output.js_code.contains("./side-effect"),
                    "Should keep side-effect import: {}",
                    output.js_code
                );
                // 运行时代码应该保留
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ Side-effect import test passed");
            }
            Err(e) => {
                panic!("Side-effect import test failed: {}", e);
            }
        }
    }

    /// Test import type with interface (v0.3.166)
    #[test]
    fn test_import_type_with_interface() {
        let ts_code = r#"
import type { Foo } from "./foo";

interface Foo {
    bar: string;
}

const foo = { bar: "test" };
console.log(foo);
"#;
        match compile_typescript(ts_code, "import_type_interface.ts") {
            Ok(output) => {
                println!("import type with interface 转译结果:");
                println!("{}", output.js_code);
                // import type 应该被移除
                assert!(
                    !output.js_code.contains("import type"),
                    "Should remove import type: {}",
                    output.js_code
                );
                // 运行时代码应该保留
                assert!(
                    output.js_code.contains("const foo"),
                    "Should contain foo variable: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ Import type with interface test passed");
            }
            Err(e) => {
                panic!("Import type with interface test failed: {}", e);
            }
        }
    }

    /// Test regular import still works (v0.3.166)
    #[test]
    fn test_regular_import() {
        let ts_code = r#"
import { something } from "./module";

const result = something;
console.log(result);
"#;
        match compile_typescript(ts_code, "regular_import.ts") {
            Ok(output) => {
                println!("普通导入转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("import")
                        && output.js_code.contains("./module")
                        && output.js_code.contains("something"),
                    "Should keep regular ESM import: {}",
                    output.js_code
                );
                // 运行时代码应该保留
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ Regular import test passed");
            }
            Err(e) => {
                panic!("Regular import test failed: {}", e);
            }
        }
    }
}
