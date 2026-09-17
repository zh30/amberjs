use amberjs::typescript::{compile_typescript, TypeScriptCompiler, TypeScriptCompilerConfig};

fn main() {
    // 测试基本的 TypeScript 转译
    let ts_code = r#"
const x: number = 42;
const greet = (name: string): string => `Hello, ${name}!`;
console.log("TS Test:", x);
console.log(greet("Beejs"));
"#;

    println!("原始 TypeScript 代码:");
    println!("{}", ts_code);
    println!("\n--- 开始转译 ---\n");

    match compile_typescript(ts_code, "test.ts") {
        Ok(output) => {
            println!("✅ 转译成功!");
            println!("\n转译后的 JavaScript 代码:");
            println!("{}", output.js_code);
        }
        Err(e) => {
            println!("❌ 转译失败: {}", e);
        }
    }
}
