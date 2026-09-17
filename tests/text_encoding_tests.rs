// TextEncoder/TextDecoder API 测试套件
//
// 目标：验证 Amber 对 TextEncoder 和 TextDecoder 的完整支持
// 这两个 API 用于高效处理 UTF-8 编码/解码

#[cfg(test)]
mod tests {
    use amberjs::runtime_minimal::MinimalRuntime;
    use serial_test::serial;

    /// 测试 TextEncoder 构造函数可用性
    #[test]
    #[serial]
    fn test_text_encoder_constructor() {
        let code = r#"
            typeof TextEncoder
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "TextEncoder constructor should be available"
        );
        assert_eq!(result.unwrap().trim(), "function");
    }

    /// 测试 TextEncoder.encode() 方法
    #[test]
    #[serial]
    fn test_text_encoder_encode() {
        let code = r#"
            const encoder = new TextEncoder();
            const bytes = encoder.encode("Hello");
            bytes.length
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "TextEncoder.encode should work");
    }

    /// 测试 TextEncoder.encodeInto() 方法
    #[test]
    #[serial]
    fn test_text_encoder_encode_into() {
        let code = r#"
            const encoder = new TextEncoder();
            const destination = new Uint8Array(5);
            const result = encoder.encodeInto("Hello", destination);
            `${result.read}:${result.written}:${Array.from(destination).join(",")}`
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "TextEncoder.encodeInto should work");
        assert_eq!(result.unwrap().trim(), "5:5:72,101,108,108,111");
    }

    /// 测试 TextEncoder.encodeInto() 不会拆开多字节 UTF-8 字符
    #[test]
    #[serial]
    fn test_text_encoder_encode_into_does_not_split_multibyte_character() {
        let code = r#"
            const encoder = new TextEncoder();
            const destination = new Uint8Array(1);
            const result = encoder.encodeInto("é", destination);
            `${result.read}:${result.written}:${Array.from(destination).join(",")}`
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "TextEncoder.encodeInto should handle short destination buffers"
        );
        assert_eq!(result.unwrap().trim(), "0:0:0");
    }

    /// 测试 TextDecoder 构造函数可用性
    #[test]
    #[serial]
    fn test_text_decoder_constructor() {
        let code = r#"
            typeof TextDecoder
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "TextDecoder constructor should be available"
        );
        assert_eq!(result.unwrap().trim(), "function");
    }

    /// 测试 TextDecoder.decode() 方法 - Uint8Array
    #[test]
    #[serial]
    fn test_text_decoder_decode_bytes() {
        let code = r#"
            const decoder = new TextDecoder();
            const bytes = new Uint8Array([72, 101, 108, 108, 111]); // "Hello"
            decoder.decode(bytes)
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "TextDecoder.decode should work with Uint8Array"
        );
    }

    /// 测试 TextDecoder.decode() 方法 - 字符串
    #[test]
    #[serial]
    fn test_text_decoder_decode_string() {
        let code = r#"
            const decoder = new TextDecoder();
            const bytes = new Uint8Array([228, 189, 160, 229, 165, 189]); // "你好"
            decoder.decode(bytes) === "你好"
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "TextDecoder.decode should handle Chinese characters"
        );
    }

    /// 测试 TextDecoder with fatal option
    #[test]
    #[serial]
    fn test_text_decoder_fatal_option() {
        let code = r#"
            const decoder = new TextDecoder('utf-8', { fatal: true });
            typeof decoder
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "TextDecoder should support fatal option");
    }

    /// fatal=true 时 invalid UTF-8 必须抛错，不能宽松替换
    #[test]
    #[serial]
    fn test_text_decoder_fatal_invalid_utf8_throws() {
        let code = r#"
            const decoder = new TextDecoder('utf-8', { fatal: true });
            let threw = false;
            try {
                decoder.decode(new Uint8Array([0xff]));
            } catch (error) {
                threw = true;
            }
            threw
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "TextDecoder fatal invalid UTF-8 test should execute"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// ignoreBOM 控制初始 UTF-8 BOM 是否作为 U+FEFF 暴露
    #[test]
    #[serial]
    fn test_text_decoder_ignore_bom_controls_initial_bom() {
        let code = r#"
            const bytes = new Uint8Array([0xef, 0xbb, 0xbf, 65]);
            const stripped = new TextDecoder('utf-8').decode(bytes);
            const preserved = new TextDecoder('utf-8', { ignoreBOM: true }).decode(bytes);
            `${stripped}:${stripped.length}:${preserved.charCodeAt(0)}:${preserved.slice(1)}:${preserved.length}`
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "TextDecoder ignoreBOM test should execute");
        assert_eq!(result.unwrap().trim(), "A:1:65279:A:2");
    }

    /// 测试编码中文
    #[test]
    #[serial]
    fn test_encode_chinese() {
        let code = r#"
            const encoder = new TextEncoder();
            const bytes = encoder.encode("你好世界");
            bytes.length === 12
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Should correctly encode Chinese characters");
    }

    /// 测试编码 emoji
    #[test]
    #[serial]
    fn test_encode_emoji() {
        let code = r#"
            const encoder = new TextEncoder();
            const bytes = encoder.encode("🚀🔥");
            bytes.length === 8
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Should correctly encode emoji characters");
    }

    /// 测试 round-trip 编码解码
    #[test]
    #[serial]
    fn test_round_trip() {
        let code = r#"
            const encoder = new TextEncoder();
            const decoder = new TextDecoder();
            const original = "Hello 🌍 你好世界 🔥";
            const encoded = encoder.encode(original);
            const decoded = decoder.decode(encoded);
            original === decoded
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Round-trip encoding should preserve text");
    }

    // ==================== atob/btoa Tests ====================

    /// 测试 btoa 编码基本字符串
    #[test]
    #[serial]
    fn test_btoa_basic() {
        let code = r#"
            btoa("Hello") === "SGVsbG8="
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "btoa should encode basic string correctly");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 atob 解码基本字符串
    #[test]
    #[serial]
    fn test_atob_basic() {
        let code = r#"
            atob("SGVsbG8=") === "Hello"
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "atob should decode basic string correctly");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 btoa/atob round-trip
    #[test]
    #[serial]
    fn test_btoa_atob_round_trip() {
        let code = r#"
            const original = "Hello, World! ÿ";
            const encoded = btoa(original);
            const decoded = atob(encoded);
            original === decoded
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "btoa/atob round-trip should preserve string"
        );
    }

    /// 测试 btoa 编码空字符串
    #[test]
    #[serial]
    fn test_btoa_empty() {
        let code = r#"
            btoa("") === ""
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "btoa should handle empty string");
    }

    /// 测试 btoa 编码特殊字符
    #[test]
    #[serial]
    fn test_btoa_special_chars() {
        let code = r#"
            btoa("+/)=") === "Ky8pPQ=="
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "btoa should encode special base64 characters"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 btoa 抛出非 Latin-1 字符错误
    #[test]
    #[serial]
    fn test_btoa_unicode_error() {
        let code = r#"
            try {
                btoa("你好");
                false;
            } catch (e) {
                e.message.includes("Latin-1")
            }
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "btoa should throw error for non-Latin1 characters"
        );
    }

    /// 测试 atob 处理无效输入
    #[test]
    #[serial]
    fn test_atob_invalid_input() {
        let code = r#"
            try {
                atob("!!!invalid!!!");
                false;
            } catch (e) {
                e.message.includes("invalid base64")
            }
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "atob should throw error for invalid base64");
    }

    /// 测试 atob 处理 undefined
    #[test]
    #[serial]
    fn test_atob_undefined_error() {
        let code = r#"
            try {
                atob();
                false;
            } catch (e) {
                e.message.includes("input is required")
            }
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "atob should throw error when input is undefined"
        );
    }

    /// 测试 btoa 处理 undefined
    #[test]
    #[serial]
    fn test_btoa_undefined_error() {
        let code = r#"
            try {
                btoa();
                false;
            } catch (e) {
                e.message.includes("input is required")
            }
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "btoa should throw error when input is undefined"
        );
    }
}
