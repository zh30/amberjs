//! Pins docs/DOMPARSER_CONTRACT.md (G32).
//!
//! Stable surface: read-only HTML (scraper) / XML (roxmltree) parse tree with
//! query helpers. Not a live browser DOM. XML query is tag/#id/descendant-limited.

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

    /// Real HTML parse: instance method + getElementById / querySelector / textContent
    #[test]
    #[serial]
    fn test_html_query_and_text_content() {
        let code = r#"
            const parser = new DOMParser();
            const doc = parser.parseFromString(
              '<html><body><div id="main"><p class="x">Hello</p><p class="x">World</p></div></body></html>',
              'text/html'
            );
            const main = doc.getElementById('main');
            const first = doc.querySelector('p.x');
            const all = doc.querySelectorAll('p.x');
            const byTag = doc.getElementsByTagName('p');
            (
              main !== null &&
              main.tagName === 'DIV' &&
              first !== null &&
              first.textContent === 'Hello' &&
              all.length === 2 &&
              byTag.length === 2 &&
              doc.body.querySelector('#main p') !== null &&
              typeof doc.body.innerHTML === 'string' &&
              doc.body.innerHTML.includes('Hello')
            )
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "HTML query APIs should work: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// Real XML parse: documentElement + getElementsByTagName + #id selector
    #[test]
    #[serial]
    fn test_xml_document_element_and_query() {
        let code = r#"
            const parser = new DOMParser();
            const doc = parser.parseFromString(
              '<?xml version="1.0"?><root><item id="a">one</item><item id="b">two</item></root>',
              'application/xml'
            );
            const root = doc.documentElement;
            const items = doc.getElementsByTagName('item');
            const a = doc.getElementById('a');
            const viaSel = doc.querySelector('item#b');
            (
              root !== null &&
              root.tagName === 'root' &&
              items.length === 2 &&
              a !== null &&
              a.textContent === 'one' &&
              viaSel !== null &&
              viaSel.textContent === 'two'
            )
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "XML query APIs should work: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// Malformed XML returns a parsererror document (does not throw)
    #[test]
    #[serial]
    fn test_xml_parsererror_document() {
        let code = r#"
            const parser = new DOMParser();
            const doc = parser.parseFromString('<root><unclosed>', 'application/xml');
            const el = doc.documentElement;
            el !== null && el.tagName === 'parsererror'
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "Malformed XML should yield parsererror: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// getAttribute on parsed elements
    #[test]
    #[serial]
    fn test_element_get_attribute() {
        let code = r#"
            const parser = new DOMParser();
            const doc = parser.parseFromString(
              '<html><body><a id="link" href="/x" class="c">Go</a></body></html>',
              'text/html'
            );
            const a = doc.getElementById('link');
            a !== null &&
              a.getAttribute('href') === '/x' &&
              a.getAttribute('class') === 'c' &&
              a.getAttribute('missing') === null
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "getAttribute should work: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// Contract Limit: XML query rejects class / attribute selectors (tag/#id/descendant only)
    #[test]
    #[serial]
    fn test_xml_query_rejects_unsupported_selector() {
        let code = r#"
            const parser = new DOMParser();
            const doc = parser.parseFromString(
              '<?xml version="1.0"?><root><item class="x">one</item></root>',
              'application/xml'
            );
            let classOk = false;
            let attrOk = false;
            try {
              doc.querySelector('.x');
            } catch (error) {
              // Class tokens hit the per-part unsupported path (no tag/#id/descendant phrase).
              classOk = error instanceof SyntaxError &&
                String(error.message).includes('unsupported selector');
            }
            try {
              doc.querySelector('[class]');
            } catch (error) {
              attrOk = error instanceof SyntaxError &&
                String(error.message).includes('tag/#id/descendant');
            }
            classOk && attrOk
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "XML unsupported selectors should throw SyntaxError: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// Contract Limit: parse tree is read-only (no appendChild; query still works)
    #[test]
    #[serial]
    fn test_parse_tree_has_no_append_child() {
        let code = r#"
            const parser = new DOMParser();
            const doc = parser.parseFromString(
              '<html><body><div id="main">Hi</div></body></html>',
              'text/html'
            );
            typeof doc.appendChild === 'undefined' &&
              typeof doc.body.appendChild === 'undefined' &&
              doc.getElementById('main') !== null
        "#;

        let mut runtime = MinimalRuntime::new().expect("Failed to create runtime");
        let result = runtime.execute_code(code);
        assert!(
            result.is_ok(),
            "Read-only parse tree pin failed: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap().trim(), "true");
    }
}
