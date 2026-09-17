// StringDecoder 模块测试 - v0.3.48
// 测试 StringDecoder 功能

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_string_decoder_module_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof string_decoder");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_string_decoder_constructor_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof string_decoder.StringDecoder");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_string_decoder_default_encoding() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code("const decoder = new string_decoder.StringDecoder(); decoder._encoding");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "utf8");
}

#[test]
#[serial]
fn test_string_decoder_custom_encoding() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        "const decoder = new string_decoder.StringDecoder('utf8'); decoder._encoding",
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "utf8");
}

#[test]
#[serial]
fn test_string_decoder_write_method_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code("const decoder = new string_decoder.StringDecoder(); typeof decoder.write");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_string_decoder_end_method_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code("const decoder = new string_decoder.StringDecoder(); typeof decoder.end");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_string_decoder_write_simple() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code("const decoder = new string_decoder.StringDecoder(); decoder.write('hello')");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "hello");
}

#[test]
#[serial]
fn test_string_decoder_write_empty() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code("const decoder = new string_decoder.StringDecoder(); decoder.write('')");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "");
}

#[test]
#[serial]
fn test_string_decoder_end_simple() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime
        .execute_code("const decoder = new string_decoder.StringDecoder(); decoder.end('world')");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "world");
}

#[test]
#[serial]
fn test_string_decoder_end_empty() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result =
        runtime.execute_code("const decoder = new string_decoder.StringDecoder(); decoder.end('')");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "");
}

#[test]
#[serial]
fn test_string_decoder_write_then_end() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        "const decoder = new string_decoder.StringDecoder(); decoder.write('hello') + decoder.end('world')"
    );
    assert!(result.is_ok());
    // Result should be combined output
    let binding = result.unwrap();
    let output = binding.trim();
    assert!(output.contains("hello") && output.contains("world"));
}

#[test]
#[serial]
fn test_string_decoder_utf8_characters() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        "const decoder = new string_decoder.StringDecoder(); decoder.write('你好世界')",
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "你好世界");
}

#[test]
#[serial]
fn test_string_decoder_emoji_characters() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        "const decoder = new string_decoder.StringDecoder(); decoder.write('🚀 Beejs')",
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "🚀 Beejs");
}

#[test]
#[serial]
fn test_string_decoder_multiple_writes() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code(
        "const decoder = new string_decoder.StringDecoder(); const r1 = decoder.write('hello'); const r2 = decoder.write(' '); const r3 = decoder.write('world'); r1 + r2 + r3"
    );
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    assert!(output.contains("hello") && output.contains("world"));
}
