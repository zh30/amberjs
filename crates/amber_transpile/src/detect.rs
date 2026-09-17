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

static USING_BINDING: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)\b(?:await\s+)?using\s+[A-Za-z_$]").expect("using regex"));

static CONST_TYPE_PARAM: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<const\s+[A-Za-z_$]").expect("const type param regex"));

static DECORATOR_LINE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?m)^\s*@[A-Za-z_$]").expect("decorator regex"));

// Lowercase tags, self-closing components, or closing tags. Avoids `Foo<Bar>`.
static JSX_ELEMENT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"</[A-Za-z]|<[a-z][A-Za-z0-9]*[\s/>]|<[A-Z][A-Za-z0-9]*[\s/]").expect("jsx regex")
});

/// True when `code` contains JSX that should be parsed as TSX.
pub fn looks_like_jsx_source(code: &str) -> bool {
    JSX_ELEMENT.is_match(code)
}

/// True when `code` contains TypeScript-only syntax that JS engines cannot parse.
pub fn looks_like_typescript_source(code: &str) -> bool {
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
        || code.contains("this:")
        || code.contains("keyof ")
        || code.contains("infer ")
        || code.contains("implements ")
        || code.contains(": string")
        || code.contains(": number")
        || code.contains(": boolean")
        || code.contains(": Promise<")
}
