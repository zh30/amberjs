// 内建字符串类型测试 (v0.3.199)
// 测试 TypeScript 内建字符串操作类型: Uppercase, Lowercase, Capitalize, Uncapitalize

#[cfg(test)]
mod typescript_intrinsic_string_types_tests {
    use amberjs::typescript::compile_typescript;

    /// Test basic Uppercase intrinsic type (v0.3.199)
    #[test]
    fn test_uppercase_basic() {
        let ts_code = r#"
type TU1 = Uppercase<'hello'>;
const result: TU1 = "HELLO";
console.log(result);
"#;
        match compile_typescript(ts_code, "uppercase_basic.ts") {
            Ok(output) => {
                println!("Uppercase 基础转译结果:");
                println!("{}", output.js_code);
                // 类型别名应该被移除
                assert!(
                    !output.js_code.contains("type TU1"),
                    "Should not contain type alias: {}",
                    output.js_code
                );
                // 保留 const 声明和 console.log
                assert!(
                    output.js_code.contains("const result"),
                    "Should contain const result: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ Uppercase basic test passed");
            }
            Err(e) => {
                panic!("Uppercase basic test failed: {}", e);
            }
        }
    }

    /// Test Lowercase intrinsic type (v0.3.199)
    #[test]
    fn test_lowercase_basic() {
        let ts_code = r#"
type TL1 = Lowercase<'HELLO'>;
const result: TL1 = "hello";
console.log(result);
"#;
        match compile_typescript(ts_code, "lowercase_basic.ts") {
            Ok(output) => {
                println!("Lowercase 基础转译结果:");
                println!("{}", output.js_code);
                assert!(
                    !output.js_code.contains("type TL1"),
                    "Should not contain type alias: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("const result"),
                    "Should contain const result: {}",
                    output.js_code
                );
                println!("✅ Lowercase basic test passed");
            }
            Err(e) => {
                panic!("Lowercase basic test failed: {}", e);
            }
        }
    }

    /// Test Capitalize intrinsic type (v0.3.199)
    #[test]
    fn test_capitalize_basic() {
        let ts_code = r#"
type TC1 = Capitalize<'hello'>;
const result: TC1 = "Hello";
console.log(result);
"#;
        match compile_typescript(ts_code, "capitalize_basic.ts") {
            Ok(output) => {
                println!("Capitalize 基础转译结果:");
                println!("{}", output.js_code);
                assert!(
                    !output.js_code.contains("type TC1"),
                    "Should not contain type alias: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("const result"),
                    "Should contain const result: {}",
                    output.js_code
                );
                println!("✅ Capitalize basic test passed");
            }
            Err(e) => {
                panic!("Capitalize basic test failed: {}", e);
            }
        }
    }

    /// Test Uncapitalize intrinsic type (v0.3.199)
    #[test]
    fn test_uncapitalize_basic() {
        let ts_code = r#"
type TN1 = Uncapitalize<'Hello'>;
const result: TN1 = "hello";
console.log(result);
"#;
        match compile_typescript(ts_code, "uncapitalize_basic.ts") {
            Ok(output) => {
                println!("Uncapitalize 基础转译结果:");
                println!("{}", output.js_code);
                assert!(
                    !output.js_code.contains("type TN1"),
                    "Should not contain type alias: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("const result"),
                    "Should contain const result: {}",
                    output.js_code
                );
                println!("✅ Uncapitalize basic test passed");
            }
            Err(e) => {
                panic!("Uncapitalize basic test failed: {}", e);
            }
        }
    }

    /// Test intrinsic types with union types (v0.3.199)
    #[test]
    fn test_intrinsic_with_union() {
        let ts_code = r#"
type TUnion = 'foo' | 'bar';
type TUpper = Uppercase<TUnion>;
const result: TUpper = "FOO";
console.log(result);
"#;
        match compile_typescript(ts_code, "intrinsic_union.ts") {
            Ok(output) => {
                println!("内建类型与联合类型转译结果:");
                println!("{}", output.js_code);
                // 类型别名应该被移除
                assert!(
                    !output.js_code.contains("type TUnion"),
                    "Should not contain type TUnion: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("type TUpper"),
                    "Should not contain type TUpper: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("const result"),
                    "Should contain const result: {}",
                    output.js_code
                );
                println!("✅ Intrinsic with union test passed");
            }
            Err(e) => {
                panic!("Intrinsic with union test failed: {}", e);
            }
        }
    }

    /// Test intrinsic types in generic context (v0.3.199)
    #[test]
    fn test_intrinsic_in_generic() {
        let ts_code = r#"
function toUppercase<T extends string>(s: T): Uppercase<T> {
    return s.toUpperCase() as Uppercase<T>;
}
const result = toUppercase("hello");
console.log(result);
"#;
        match compile_typescript(ts_code, "intrinsic_generic.ts") {
            Ok(output) => {
                println!("内建类型在泛型中转译结果:");
                println!("{}", output.js_code);
                assert!(
                    output.js_code.contains("function toUppercase"),
                    "Should contain function: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                // 类型注解应该被移除
                assert!(
                    !output.js_code.contains(": Uppercase<T>"),
                    "Should remove type annotation: {}",
                    output.js_code
                );
                println!("✅ Intrinsic in generic test passed");
            }
            Err(e) => {
                panic!("Intrinsic in generic test failed: {}", e);
            }
        }
    }

    /// Test combined intrinsic types (v0.3.199)
    #[test]
    fn test_combined_intrinsic_types() {
        let ts_code = r#"
type Cases<T extends string> = `${Uppercase<T>} ${Lowercase<T>} ${Capitalize<T>} ${Uncapitalize<T>}`;
type Result = Cases<'bar'>;
const output: string = "BAR bar Bar bar";
console.log(output);
"#;
        match compile_typescript(ts_code, "combined_intrinsic.ts") {
            Ok(output) => {
                println!("组合内建类型转译结果:");
                println!("{}", output.js_code);
                // 类型别名应该被移除
                assert!(
                    !output.js_code.contains("type Cases"),
                    "Should not contain type Cases: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("type Result"),
                    "Should not contain type Result: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ Combined intrinsic types test passed");
            }
            Err(e) => {
                panic!("Combined intrinsic types test failed: {}", e);
            }
        }
    }

    /// Test intrinsic types with template literal (v0.3.199)
    #[test]
    fn test_intrinsic_with_template_literal() {
        let ts_code = r#"
type TemplateWithUpper = `${Uppercase<'hello'>}_suffix`;
const result: string = "HELLO_suffix";
console.log(result);
"#;
        match compile_typescript(ts_code, "intrinsic_template.ts") {
            Ok(output) => {
                println!("内建类型与模板字面量转译结果:");
                println!("{}", output.js_code);
                // 类型别名应该被移除
                assert!(
                    !output.js_code.contains("type TemplateWithUpper"),
                    "Should not contain type TemplateWithUpper: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("const result"),
                    "Should contain const result: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ Intrinsic with template literal test passed");
            }
            Err(e) => {
                panic!("Intrinsic with template literal test failed: {}", e);
            }
        }
    }

    /// Test all intrinsic types in single file (v0.3.199)
    #[test]
    fn test_all_intrinsic_types() {
        let ts_code = r#"
type Upper = Uppercase<'hello'>;
type Lower = Lowercase<'WORLD'>;
type Cap = Capitalize<'hello'>;
type Uncap = Uncapitalize<'WORLD'>;

const u: string = "HELLO";
const l: string = "world";
const c: string = "Hello";
const uc: string = "wORLD";

console.log(u, l, c, uc);
"#;
        match compile_typescript(ts_code, "all_intrinsic.ts") {
            Ok(output) => {
                println!("所有内建类型转译结果:");
                println!("{}", output.js_code);
                // 所有类型别名应该被移除
                assert!(
                    !output.js_code.contains("type Upper"),
                    "Should not contain type Upper: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("type Lower"),
                    "Should not contain type Lower: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("type Cap"),
                    "Should not contain type Cap: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("type Uncap"),
                    "Should not contain type Uncap: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ All intrinsic types test passed");
            }
            Err(e) => {
                panic!("All intrinsic types test failed: {}", e);
            }
        }
    }

    /// Test template literal with intrinsic types without warnings (v0.3.219)
    #[test]
    fn test_template_literal_intrinsic_no_warnings() {
        // v0.3.219: 验证模板字面量中的内建字符串类型不会产生警告
        let ts_code = r#"
type UpperTemplate = `PREFIX_${Uppercase<'hello'>}_SUFFIX`;
type LowerTemplate = `prefix_${Lowercase<'WORLD'>}_suffix`;
type CapTemplate = `Prefix_${Capitalize<'hello'>}_Suffix`;
type UncapTemplate = `pREFIX_${Uncapitalize<'Hello'>}_sUFFIX`;
type Combined = `${Uppercase<'a'>}${Lowercase<'B'>}${Capitalize<'c'>}${Uncapitalize<'D'>}`;

const t1 = "PREFIX_HELLO_SUFFIX";
const t2 = "prefix_world_suffix";
const t3 = "Prefix_Hello_Suffix";
const t4 = "pREFIX_hello_sUFFIX";
const t5 = "AaBbCcDd";

console.log(t1, t2, t3, t4, t5);
"#;
        match compile_typescript(ts_code, "template_intrinsic_no_warnings.ts") {
            Ok(output) => {
                println!("模板字面量内建类型无警告测试结果:");
                println!("{}", output.js_code);

                // 验证没有警告（diagnostics 列表应该为空或只包含非错误信息）
                let has_warnings = output
                    .diagnostics
                    .iter()
                    .any(|d| d.message.contains("has invalid type definition"));
                assert!(
                    !has_warnings,
                    "Should not have invalid type definition warnings, got: {:?}",
                    output.diagnostics
                );

                // 验证代码保留
                assert!(
                    output.js_code.contains("const t1"),
                    "Should contain const t1: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );

                // 验证类型别名被移除
                assert!(
                    !output.js_code.contains("type UpperTemplate"),
                    "Should not contain type UpperTemplate: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("type Combined"),
                    "Should not contain type Combined: {}",
                    output.js_code
                );

                println!("✅ Template literal intrinsic types without warnings test passed");
            }
            Err(e) => {
                panic!("Template literal intrinsic no warnings test failed: {}", e);
            }
        }
    }

    /// Test Trim intrinsic type (v0.3.222)
    #[test]
    fn test_trim_basic() {
        let ts_code = r#"
type TTrim1 = Trim<'  hello  '>;
const result: TTrim1 = 'hello';
console.log(result);
"#;
        match compile_typescript(ts_code, "trim_basic.ts") {
            Ok(output) => {
                println!("Trim 基础转译结果:");
                println!("{}", output.js_code);
                assert!(
                    !output.js_code.contains("type TTrim1"),
                    "Should not contain type alias: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("const result"),
                    "Should contain const result: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ Trim basic test passed");
            }
            Err(e) => {
                panic!("Trim basic test failed: {}", e);
            }
        }
    }

    /// Test TrimLeft intrinsic type (v0.3.222)
    #[test]
    fn test_trim_left_basic() {
        let ts_code = r#"
type TTrimLeft1 = TrimLeft<'  hello  '>;
const result: TTrimLeft1 = 'hello  ';
console.log(result);
"#;
        match compile_typescript(ts_code, "trim_left_basic.ts") {
            Ok(output) => {
                println!("TrimLeft 基础转译结果:");
                println!("{}", output.js_code);
                assert!(
                    !output.js_code.contains("type TTrimLeft1"),
                    "Should not contain type alias: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("const result"),
                    "Should contain const result: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ TrimLeft basic test passed");
            }
            Err(e) => {
                panic!("TrimLeft basic test failed: {}", e);
            }
        }
    }

    /// Test TrimRight intrinsic type (v0.3.222)
    #[test]
    fn test_trim_right_basic() {
        let ts_code = r#"
type TTrimRight1 = TrimRight<'  hello  '>;
const result: TTrimRight1 = '  hello';
console.log(result);
"#;
        match compile_typescript(ts_code, "trim_right_basic.ts") {
            Ok(output) => {
                println!("TrimRight 基础转译结果:");
                println!("{}", output.js_code);
                assert!(
                    !output.js_code.contains("type TTrimRight1"),
                    "Should not contain type alias: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("const result"),
                    "Should contain const result: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ TrimRight basic test passed");
            }
            Err(e) => {
                panic!("TrimRight basic test failed: {}", e);
            }
        }
    }

    /// Test Trim types with union types (v0.3.222)
    #[test]
    fn test_trim_with_union() {
        let ts_code = r#"
type TUnion = '  foo  ' | '  bar  ';
type TTrim = Trim<TUnion>;
const result: string = 'foo';
console.log(result);
"#;
        match compile_typescript(ts_code, "trim_union.ts") {
            Ok(output) => {
                println!("Trim 与联合类型转译结果:");
                println!("{}", output.js_code);
                assert!(
                    !output.js_code.contains("type TTrim"),
                    "Should not contain type TTrim: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("const result"),
                    "Should contain const result: {}",
                    output.js_code
                );
                println!("✅ Trim with union test passed");
            }
            Err(e) => {
                panic!("Trim with union test failed: {}", e);
            }
        }
    }

    /// Test all Trim types in single file (v0.3.222)
    #[test]
    fn test_all_trim_types() {
        let ts_code = r#"
type TrimStr = Trim<'  hello  '>;
type TrimLeftStr = TrimLeft<'  hello  '>;
type TrimRightStr = TrimRight<'  hello  '>;

const t: string = 'hello';
const tl: string = 'hello  ';
const tr: string = '  hello';

console.log(t, tl, tr);
"#;
        match compile_typescript(ts_code, "all_trim.ts") {
            Ok(output) => {
                println!("所有 Trim 类型转译结果:");
                println!("{}", output.js_code);
                assert!(
                    !output.js_code.contains("type TrimStr"),
                    "Should not contain type TrimStr: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("type TrimLeftStr"),
                    "Should not contain type TrimLeftStr: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("type TrimRightStr"),
                    "Should not contain type TrimRightStr: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );
                println!("✅ All Trim types test passed");
            }
            Err(e) => {
                panic!("All Trim types test failed: {}", e);
            }
        }
    }

    /// Test Capitalize/Uncapitalize in conditional types (v0.3.223)
    #[test]
    fn test_capitalize_uncapitalize_in_conditional_types() {
        // v0.3.223: 验证 Capitalize/Uncapitalize 在条件类型中的处理
        let ts_code = r#"
type ToCapitalize<T extends string> = T extends string ? Capitalize<T> : never;
type ToUncapitalize<T extends string> = T extends string ? Uncapitalize<T> : never;

type CapResult = ToCapitalize<'hello'>;
type UncapResult = ToUncapitalize<'HELLO'>;

const cap: string = "Hello";
const uncap: string = "hELLO";

console.log(cap, uncap);
"#;
        match compile_typescript(ts_code, "capitalize_conditional.ts") {
            Ok(output) => {
                println!("Capitalize/Uncapitalize 在条件类型中的转译结果:");
                println!("{}", output.js_code);

                // 验证类型别名被移除
                assert!(
                    !output.js_code.contains("type ToCapitalize"),
                    "Should not contain type ToCapitalize: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("type ToUncapitalize"),
                    "Should not contain type ToUncapitalize: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("type CapResult"),
                    "Should not contain type CapResult: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("type UncapResult"),
                    "Should not contain type UncapResult: {}",
                    output.js_code
                );

                // 验证代码保留
                assert!(
                    output.js_code.contains("const cap"),
                    "Should contain const cap: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );

                println!("✅ Capitalize/Uncapitalize in conditional types test passed");
            }
            Err(e) => {
                panic!(
                    "Capitalize/Uncapitalize in conditional types test failed: {}",
                    e
                );
            }
        }
    }

    /// Test deeply nested intrinsic string types (v0.3.223)
    #[test]
    fn test_deeply_nested_intrinsic_types() {
        // v0.3.223: 验证深层嵌套的内建字符串类型处理
        let ts_code = r#"
type Nested1 = Uppercase<Lowercase<'HELLO'>>;
type Nested2 = Capitalize<Uncapitalize<'Hello'>>;
type Nested3 = Trim<Uppercase<'  hello  '>>;

const n1: string = "hello";
const n2: string = "Hello";
const n3: string = "HELLO";

console.log(n1, n2, n3);
"#;
        match compile_typescript(ts_code, "nested_intrinsic.ts") {
            Ok(output) => {
                println!("深层嵌套内建字符串类型转译结果:");
                println!("{}", output.js_code);

                // 验证类型别名被移除
                assert!(
                    !output.js_code.contains("type Nested1"),
                    "Should not contain type Nested1: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("type Nested2"),
                    "Should not contain type Nested2: {}",
                    output.js_code
                );
                assert!(
                    !output.js_code.contains("type Nested3"),
                    "Should not contain type Nested3: {}",
                    output.js_code
                );

                // 验证代码保留
                assert!(
                    output.js_code.contains("const n1"),
                    "Should contain const n1: {}",
                    output.js_code
                );
                assert!(
                    output.js_code.contains("console.log"),
                    "Should contain console.log: {}",
                    output.js_code
                );

                println!("✅ Deeply nested intrinsic types test passed");
            }
            Err(e) => {
                panic!("Deeply nested intrinsic types test failed: {}", e);
            }
        }
    }
}
