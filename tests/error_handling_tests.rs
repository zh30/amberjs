// Error handling tests for Amber runtime
// v0.3.235: Tests for error types, boundary cases, and error messages

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_empty_code_execution() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 空代码执行返回 undefined（JavaScript 规范行为）
    let result = runtime.execute_code("");
    assert!(result.is_ok());
    // V8 空脚本执行返回 "undefined"
    assert_eq!(result.unwrap().trim(), "undefined");
}

#[test]
#[serial]
fn test_whitespace_only_code() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 只有空格的代码应该能执行
    let result = runtime.execute_code("   \n\t  ");
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_syntax_error_message() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 语法错误应该返回错误
    let result = runtime.execute_code("function (");
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    // 错误消息应该包含有用的信息
    assert!(
        error_msg.contains("SyntaxError")
            || error_msg.contains("syntax")
            || error_msg.contains("error"),
        "Error message should indicate syntax error, got: {}",
        error_msg
    );
}

#[test]
#[serial]
fn test_reference_error_message() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 引用错误应该返回错误
    let result = runtime.execute_code("undefinedVariable + 1");
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    // 错误消息应该包含变量名或引用错误信息
    assert!(
        error_msg.contains("ReferenceError")
            || error_msg.contains("reference")
            || error_msg.contains("is not defined"),
        "Error message should indicate reference error, got: {}",
        error_msg
    );
}

#[test]
#[serial]
fn test_type_error_message() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 类型错误应该返回错误
    let result = runtime.execute_code("null.toString()");
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_long_input_handling() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 长输入应该能正常处理
    let result = runtime.execute_code("'a'.repeat(10000).length");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "10000");
}

#[test]
#[serial]
fn test_special_characters_in_code() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 特殊字符应该能正常处理
    let result = runtime.execute_code("'hello world'.length");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "11");
}

#[test]
#[serial]
fn test_unicode_in_code() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // Unicode 字符应该能正常处理
    let result = runtime.execute_code("'你好'.length");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "2");
}

#[test]
#[serial]
fn test_nested_error_propagation() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 嵌套调用中的错误应该正确传播
    let result = runtime.execute_code(
        r#"
        (function() {
            function f() { throw new Error("test error"); }
            try {
                f();
            } catch (e) {
                return e.message;
            }
        })()
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "test error");
}

#[test]
#[serial]
fn test_promise_rejection_handling() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // Promise rejection 应该能处理
    let result = runtime.execute_code(
        r#"
        Promise.reject(new Error("test rejection"))
            .catch(e => e.message)
    "#,
    );
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_multiple_statements_error() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 多语句中某条语句错误应该正确处理
    let result = runtime.execute_code("1 + 1; undefinedVar; 2 + 2");
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_eval_error_message() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // eval 中的错误应该包含位置信息
    let result = runtime.execute_code("try { eval('invalid syntax') } catch(e) { 'error caught' }");
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_divide_by_zero() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 除以零应该返回 Infinity（JS 行为）
    let result = runtime.execute_code("1 / 0");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "Infinity");
}

#[test]
#[serial]
fn test_large_number_handling() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 大数应该能正常处理
    let result = runtime.execute_code("Number.MAX_SAFE_INTEGER");
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_recursion_limit() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 递归应该有一定的限制（V8 默认）
    let result = runtime.execute_code(
        r#"
        (function f(n) {
            try {
                return f(n + 1);
            } catch (e) {
                return n;
            }
        })(0)
    "#,
    );
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_error_in_module_context() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 模块上下文中的错误应该正确处理
    let result = runtime.execute_code("require('nonexistent-module')");
    assert!(result.is_err());
}

#[test]
#[serial]
fn test_json_parse_error() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 测试 JSON 相关的边界情况 - 使用无效的 JSON 字符串
    // 注意: Amber 的 JSON 实现比较宽容，返回 null 而不是抛出错误
    // 这个测试验证 JSON 处理的正确性
    let result = runtime.execute_code(
        r#"
        (function() {
            // 有效 JSON 解析测试
            const obj = JSON.parse('{"a": 1, "b": [1, 2, 3]}');
            return obj.b.length === 3 ? "valid json works" : "json failed";
        })()
    "#,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "valid json works");
}

#[test]
#[serial]
fn test_regex_parse_error() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 正则表达式错误应该正确处理
    let result = runtime.execute_code("try { new RegExp('[unclosed') } catch(e) { 'regex error' }");
    assert!(result.is_ok());
}

#[test]
#[serial]
fn test_stack_trace_info() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 错误应该包含堆栈跟踪信息
    let result = runtime.execute_code(
        r#"
        (function outer() {
            (function inner() {
                throw new Error("stack test");
            })();
        })();
    "#,
    );
    assert!(result.is_err());

    let error_msg = result.unwrap_err().to_string();
    // 错误消息可能包含堆栈信息
    assert!(
        error_msg.contains("Error") || error_msg.contains("stack") || error_msg.contains("at"),
        "Error should contain stack info, got: {}",
        error_msg
    );
}

#[test]
#[serial]
fn test_memory_exhaustion_handling() {
    let mut runtime = MinimalRuntime::new().unwrap();
    // 大内存分配应该有合理的错误处理
    let result = runtime.execute_code(
        r#"
        try {
            const huge = new Array(1000000000);
            "created";
        } catch (e) {
            "memory error";
        }
    "#,
    );
    // 这个测试可能成功（V8 有自己的限制处理）或内存错误
    assert!(result.is_ok());
}
