//! Conservative TypeScript-only sniff for `execute_code` / `eval`.
//!
//! `import` / `export` are valid JavaScript and must not force a transpile.

use once_cell::sync::Lazy;
use regex::Regex;

static INTERFACE_DECLARATION: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?m)^\s*(?:export\s+)?interface\s+[A-Za-z_$][A-Za-z0-9_$]*(?:\s+extends\s+[^{]+)?\s*\{",
    )
    .expect("interface regex")
});

static TYPE_ALIAS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)^\s*(?:export\s+)?type\s+[A-Za-z_$][A-Za-z0-9_$]*(?:\s*<[^=;\n]+>)?\s*=")
        .expect("type alias regex")
});

// Require `=` so prose like "using the Test262Error class" (test262 harness
// comments) does not look like Explicit Resource Management and force oxc,
// which then rejects non-strict `with` statements.
static USING_BINDING: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?m)\b(?:await\s+)?using\s+[A-Za-z_$][A-Za-z0-9_$]*\s*=").expect("using regex")
});

static CONST_TYPE_PARAM: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<const\s+[A-Za-z_$]").expect("const type param regex"));

static DECORATOR_LINE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^\s*@[A-Za-z_$]").expect("decorator regex"));

// Lowercase tags, self-closing components, or closing tags. Avoids `Foo<Bar>`.
static JSX_ELEMENT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"</[A-Za-z]|<[a-z][A-Za-z0-9]*[\s/>]|<[A-Z][A-Za-z0-9]*[\s/]").expect("jsx regex")
});

// Binding annotations only — not JSDoc `{name?: string}` prose.
static BINDING_TYPE_ANN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?m)(?::\s*(?:string|number|boolean|bigint|symbol|any|unknown|never|void|object|Promise\s*<)|\bas\s+const\b)",
    )
    .expect("binding type ann regex")
});

/// Strip `//` and `/* */` comments so JSDoc / harness prose cannot trip TS sniff.
fn strip_comments(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    let bytes = code.as_bytes();
    let mut i = 0;
    let n = bytes.len();
    while i < n {
        if bytes[i] == b'/' && i + 1 < n && bytes[i + 1] == b'/' {
            i += 2;
            while i < n && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if bytes[i] == b'/' && i + 1 < n && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < n && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i = (i + 2).min(n);
            continue;
        }
        // Preserve string/template contents without stripping their internals.
        if bytes[i] == b'\'' || bytes[i] == b'"' {
            let quote = bytes[i];
            out.push(quote as char);
            i += 1;
            while i < n {
                let c = bytes[i];
                out.push(c as char);
                i += 1;
                if c == b'\\' && i < n {
                    out.push(bytes[i] as char);
                    i += 1;
                    continue;
                }
                if c == quote {
                    break;
                }
            }
            continue;
        }
        if bytes[i] == b'`' {
            out.push('`');
            i += 1;
            while i < n {
                let c = bytes[i];
                out.push(c as char);
                i += 1;
                if c == b'\\' && i < n {
                    out.push(bytes[i] as char);
                    i += 1;
                    continue;
                }
                if c == b'`' {
                    break;
                }
            }
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// True when `code` contains JSX that should be parsed as TSX.
pub fn looks_like_jsx_source(code: &str) -> bool {
    JSX_ELEMENT.is_match(code)
}

/// True when `code` contains TypeScript-only syntax that JS engines cannot parse.
pub fn looks_like_typescript_source(code: &str) -> bool {
    // Comment-strip first: test262 harness JSDoc contains `: string` / `using the…`.
    let code = strip_comments(code);
    let code = code.as_str();
    looks_like_jsx_source(code)
        || INTERFACE_DECLARATION.is_match(code)
        || TYPE_ALIAS.is_match(code)
        || USING_BINDING.is_match(code)
        || CONST_TYPE_PARAM.is_match(code)
        || DECORATOR_LINE.is_match(code)
        || code.contains("enum ")
        || code.contains(" satisfies ")
        || code.contains(" as const")
        || code.contains("declare global")
        || code.contains("declare module ")
        || code.contains("declare namespace ")
        || code.contains("import type")
        || code.contains("export type")
        || code.contains("abstract class")
        || code.contains("export abstract")
        || code.contains("keyof ")
        || code.contains("infer ")
        || code.contains("implements ")
        || BINDING_TYPE_ANN.is_match(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_prose_is_not_typescript() {
        let sta = "/*---\nusing the Test262Error class\n---*/\nfunction f(){}\n";
        assert!(!looks_like_typescript_source(sta));
        let jsdoc =
            "/**\n * @param {{name?: string|symbol, length?: number}}\n */\nwith (o) { x = 1; }\n";
        assert!(!looks_like_typescript_source(jsdoc));
    }

    #[test]
    fn real_using_and_annotations_still_match() {
        assert!(looks_like_typescript_source("using x = resource();\n"));
        assert!(looks_like_typescript_source("let x: string = 'a';\n"));
    }
}
