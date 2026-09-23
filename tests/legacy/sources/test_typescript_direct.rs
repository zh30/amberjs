// 直接测试 TypeScript 编译器
// 编译命令: rustc --edition 2021 -L target/debug/deps test_typescript_direct.rs --extern amberjs=target/debug/libamberjs.rlib

fn main() {
    // 模拟测试用例
    let test_cases = vec![
        ("简单箭头函数", "const double = (x: number) => x * 2;"),
        (
            "多参数箭头函数",
            "const add = (a: number, b: number): number => a + b;",
        ),
        ("无参数箭头函数", "const getAnswer = () => 42;"),
        (
            "函数类型标注",
            "function greet(name: string): string { return `Hello, ${name}!`; }",
        ),
    ];

    println!("🚀 TypeScript 编译器测试\n");

    for (name, code) in test_cases {
        println!("📝 测试: {}", name);
        println!("   输入: {}", code);

        // 这里我们需要调用 amberjs::typescript::compile_typescript
        // 但由于这是在独立的二进制文件中，我们需要其他方法
        // 或者我们可以使用amberjs CLI本身

        println!();
    }
}
