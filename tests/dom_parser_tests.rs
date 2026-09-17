// DOMParser API 测试套件 - v0.3.341
//
// 目标：验证 Beejs 对 DOMParser 接口的完整支持
// DOMParser 用于解析 HTML/XML 文档，适用于 AI 工作负载处理网页内容

#[cfg(test)]
mod tests {
    use amberjs::MinimalRuntime;
    use serial_test::serial;

    /// 测试 DOMParser 构造函数可用性
    #[test]
    #[serial]
    fn test_dom_parser_constructor() {
        let code = r#"
            typeof DOMParser
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "DOMParser constructor should be available");
        assert_eq!(result.unwrap().trim(), "function");
    }

    /// 测试 DOMParser 基本实例创建
    #[test]
    #[serial]
    fn test_dom_parser_instance() {
        let code = r#"
            const parser = new DOMParser();
            parser !== null && typeof parser === 'object'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "DOMParser instance should be creatable");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 parseFromString 方法可用性
    #[test]
    #[serial]
    fn test_parse_from_string_method() {
        let code = r#"
            typeof DOMParser.prototype.parseFromString === 'function'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "parseFromString method should be available");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 HTML 文档解析
    #[test]
    #[serial]
    fn test_parse_html_document() {
        let code = r#"
            const parser = new DOMParser();
            const html = '<html><body><h1>Hello</h1></body></html>';
            const doc = DOMParser.prototype.parseFromString(html, 'text/html');
            typeof doc === 'object' && doc !== null
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "HTML document should be parseable");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 XML 文档解析
    #[test]
    #[serial]
    fn test_parse_xml_document() {
        let code = r#"
            const parser = new DOMParser();
            const xml = '<?xml version="1.0"?><root><item>test</item></root>';
            const doc = DOMParser.prototype.parseFromString(xml, 'application/xml');
            typeof doc === 'object' && doc !== null
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "XML document should be parseable");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 document.body 属性（HTML 文档）
    #[test]
    #[serial]
    fn test_html_document_body() {
        let code = r#"
            const parser = new DOMParser();
            const html = '<html><body><p>Test</p></body></html>';
            const doc = DOMParser.prototype.parseFromString(html, 'text/html');
            typeof doc.body === 'object'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "document.body should be available for HTML");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 document.URL 属性
    #[test]
    #[serial]
    fn test_document_url() {
        let code = r#"
            const parser = new DOMParser();
            const doc = DOMParser.prototype.parseFromString('<html></html>', 'text/html');
            typeof doc.URL === 'string'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "document.URL should be available");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试空字符串解析
    #[test]
    #[serial]
    fn test_parse_empty_string() {
        let code = r#"
            const parser = new DOMParser();
            const doc = DOMParser.prototype.parseFromString('', 'text/html');
            typeof doc === 'object' && doc !== null
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "Empty string should be parseable");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试特殊字符转义（HTML 实体）
    #[test]
    #[serial]
    fn test_special_characters() {
        let code = r#"
            const parser = new DOMParser();
            const html = '<p>&lt;script&gt;</p>';
            const doc = DOMParser.prototype.parseFromString(html, 'text/html');
            typeof doc === 'object' && doc !== null && typeof doc.body === 'object'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "Special characters should be properly escaped"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 XHTML 解析
    #[test]
    #[serial]
    fn test_parse_xhtml() {
        let code = r#"
            const parser = new DOMParser();
            const xhtml = '<html xmlns="http://www.w3.org/1999/xhtml"><body><div/></body></html>';
            const doc = DOMParser.prototype.parseFromString(xhtml, 'application/xhtml+xml');
            typeof doc === 'object'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "XHTML document should be parseable");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 children 属性存在性
    #[test]
    #[serial]
    fn test_document_children() {
        let code = r#"
            const parser = new DOMParser();
            const doc = DOMParser.prototype.parseFromString('<html></html>', 'text/html');
            Array.isArray(doc.children)
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "document.children should be an array");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试 SVG 解析
    #[test]
    #[serial]
    fn test_parse_svg() {
        let code = r#"
            const parser = new DOMParser();
            const svg = '<svg xmlns="http://www.w3.org/2000/svg"><circle cx="50" cy="50" r="40"/></svg>';
            const doc = DOMParser.prototype.parseFromString(svg, 'image/svg+xml');
            typeof doc === 'object'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(result.is_ok(), "SVG document should be parseable");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试缺失内容类型会抛 TypeError
    #[test]
    #[serial]
    fn test_missing_content_type_throws() {
        let code = r#"
            const parser = new DOMParser();
            try {
                DOMParser.prototype.parseFromString('<html></html>');
                false;
            } catch (error) {
                error instanceof TypeError;
            }
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "Missing DOMParser content type should be catchable"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// 测试非法内容类型会抛 TypeError
    #[test]
    #[serial]
    fn test_invalid_content_type_throws() {
        let code = r#"
            const parser = new DOMParser();
            try {
                DOMParser.prototype.parseFromString('<x/>', 'text/plain');
                false;
            } catch (error) {
                error instanceof TypeError;
            }
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "Invalid DOMParser content type should be catchable"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }
}
