// URL API implementation for Web standard
/// Provides URL, URLSearchParams API
use anyhow::Result;
use rusty_v8 as v8;
use std::collections::HashMap;
/// URL class implementation
#[derive(Debug, Clone)]
pub struct Url {
    pub href: String,
    pub protocol: String,
    pub host: String,
    pub hostname: String,
    pub port: String,
    pub pathname: String,
    pub search: String,
    pub hash: String,
    pub origin: String,
    pub username: String,
    pub password: String,
}
impl Url {
    /// Parse URL string
    pub fn parse(url_str: &str, _base: Option<&str>) -> Result<Self> {
        // Simple URL parsing - in production would use url crate
        let (
            href,
            protocol,
            host,
            hostname,
            port,
            pathname,
            search,
            hash,
            origin,
            username,
            password,
        ) = if url_str.contains("://") {
            let parts: Vec<&str> = url_str.split("://").collect();
            let protocol: _ = parts[0].to_string();
            let rest: _ = parts.get(1).unwrap_or(&"");
            let (host_part, pathname, search, hash) = if let Some(path_start) = rest.find('/') {
                let (host_path, rest_path) = rest.split_at(path_start);
                let (path_part, hash_part) = if let Some(hash_pos) = rest_path.find('#') {
                    let (p, h) = rest_path.split_at(hash_pos);
                    (p, h.to_string())
                } else {
                    (rest_path, "".to_string())
                };
                let (search_part, path_part) = if let Some(search_pos) = path_part.find('?') {
                    let (p, s) = path_part.split_at(search_pos);
                    (s.to_string(), p.to_string())
                } else {
                    ("".to_string(), path_part.to_string())
                };
                (host_path.to_string(), path_part, search_part, hash_part)
            } else {
                (
                    rest.to_string(),
                    "/".to_string(),
                    "".to_string(),
                    "".to_string(),
                )
            };
            let (hostname, port) = if let Some(port_pos) = host_part.find(':') {
                let (h, p) = host_part.split_at(port_pos);
                (h.to_string(), p[1..].to_string()) // Skip the ':' prefix
            } else {
                (host_part.clone(), "".to_string())
            };
            let origin: _ = format!("{}://{}", protocol, host_part);
            (
                url_str.to_string(),
                protocol,
                host_part,
                hostname,
                port,
                pathname,
                search,
                hash,
                origin,
                "".to_string(), // username
                "".to_string(), // password
            )
        } else {
            // Relative URL - would need base URL
            (
                url_str.to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                url_str.to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
                "".to_string(),
            )
        };
        Ok(Self {
            href,
            protocol,
            host,
            hostname,
            port,
            pathname,
            search,
            hash,
            origin,
            username,
            password,
        })
    }
    /// Get search params
    pub fn search_params(&self) -> UrlSearchParams {
        UrlSearchParams::new(&self.search)
    }
}
/// URLSearchParams implementation
#[derive(Debug, Clone)]
pub struct UrlSearchParams {
    params: HashMap<String, Vec<String>>,
}
impl UrlSearchParams {
    pub fn new(search: &str) -> Self {
        let mut params = HashMap::new();
        // Remove leading '?' if present
        let search: _ = search.trim_start_matches('?');
        for pair in search.split('&') {
            if pair.is_empty() {
                continue;
            }
            let (key, value) = if let Some(eq_pos) = pair.find('=') {
                let (k, v) = pair.split_at(eq_pos);
                (k.to_string(), v[1..].to_string()) // Skip the '=' prefix
            } else {
                (pair.to_string(), "".to_string())
            };
            params.entry(key).or_insert_with(Vec::new).push(value);
        }
        Self { params }
    }
    /// Get value by key
    pub fn get(&self, key: &str) -> Option<&String> {
        self.params.get(key).and_then(|v| v.first())
    }
    /// Set value
    pub fn set(&mut self, key: String, value: String) {
        self.params.insert(key, vec![value]);
    }
    /// Append value
    pub fn append(&mut self, key: String, value: String) {
        self.params.entry(key).or_insert_with(Vec::new).push(value);
    }
    /// Delete value
    pub fn delete(&mut self, key: &str) {
        self.params.remove(key);
    }
    /// Get all keys
    pub fn keys(&self) -> Vec<String> {
        self.params.keys().cloned().collect()
    }
    /// Get all values
    pub fn values(&self) -> Vec<String> {
        self.params.values().flatten().cloned().collect()
    }
    /// Check if has key
    pub fn has(&self, key: &str) -> bool {
        self.params.contains_key(key)
    }
    /// Get entries
    pub fn entries(&self) -> Vec<(String, String)> {
        self.params
            .iter()
            .flat_map(|(k, v)| v.iter().map(|val| (k.clone(), val.clone())))
            .collect()
    }
    /// Convert to string
    pub fn to_string(&self) -> String {
        let mut result = String::new();
        let mut first = true;
        for (key, values) in &self.params {
            for value in values {
                if !first {
                    result.push('&');
                }
                first = false;
                result.push_str(key);
                result.push('=');
                result.push_str(value);
            }
        }
        result
    }
}
/// Setup URL API in V8 context
pub fn setup_url_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    _context: &v8::Local<v8::Context>,
) -> Result<()> {
    // rusty_v8 does not expose WHATWG URL on the global. Install a JIT-friendly
    // JS implementation. Do not wrap it again in BeeURL (see runtime_minimal).
    let url_js = include_str!("url_fast.js");
    if let Some(code) = v8::String::new(scope, url_js) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }
    Ok(())
}
/// URL constructor callback
#[allow(dead_code)]
fn url_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let url_str: _ = args
        .get(0)
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);
    if let Ok(url) = Url::parse(&url_str, None) {
        let url_obj: _ = v8::Object::new(scope);
        // Store URL data
        let href_key: _ = v8::String::new(scope, "href").unwrap();
        let href_val: _ = v8::String::new(scope, &url.href).unwrap();
        url_obj.set(scope, href_key.into(), href_val.into());
        let protocol_key: _ = v8::String::new(scope, "protocol").unwrap();
        let protocol_val: _ = v8::String::new(scope, &url.protocol).unwrap();
        url_obj.set(scope, protocol_key.into(), protocol_val.into());
        let host_key: _ = v8::String::new(scope, "host").unwrap();
        let host_val: _ = v8::String::new(scope, &url.host).unwrap();
        url_obj.set(scope, host_key.into(), host_val.into());
        let hostname_key: _ = v8::String::new(scope, "hostname").unwrap();
        let hostname_val: _ = v8::String::new(scope, &url.hostname).unwrap();
        url_obj.set(scope, hostname_key.into(), hostname_val.into());
        let port_key: _ = v8::String::new(scope, "port").unwrap();
        let port_val: _ = v8::String::new(scope, &url.port).unwrap();
        url_obj.set(scope, port_key.into(), port_val.into());
        let pathname_key: _ = v8::String::new(scope, "pathname").unwrap();
        let pathname_val: _ = v8::String::new(scope, &url.pathname).unwrap();
        url_obj.set(scope, pathname_key.into(), pathname_val.into());
        let search_key: _ = v8::String::new(scope, "search").unwrap();
        let search_val: _ = v8::String::new(scope, &url.search).unwrap();
        url_obj.set(scope, search_key.into(), search_val.into());
        let hash_key: _ = v8::String::new(scope, "hash").unwrap();
        let hash_val: _ = v8::String::new(scope, &url.hash).unwrap();
        url_obj.set(scope, hash_key.into(), hash_val.into());
        let origin_key: _ = v8::String::new(scope, "origin").unwrap();
        let origin_val: _ = v8::String::new(scope, &url.origin).unwrap();
        url_obj.set(scope, origin_key.into(), origin_val.into());
        retval.set(url_obj.into());
    } else {
        let error: _ = v8::String::new(scope, "Invalid URL").unwrap();
        let error_obj: _ = v8::Exception::error(scope, error);
        scope.throw_exception(error_obj.into());
    }
}
/// URLSearchParams constructor callback
#[allow(dead_code)]
fn url_search_params_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let _init: _ = args.get(0);
    let search_params_obj: _ = v8::Object::new(scope);
    // Add methods to prototype
    let proto: _ = v8::Object::new(scope);
    let get_key: _ = v8::String::new(scope, "get").unwrap();
    let get_func: _ = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut _rv: v8::ReturnValue| {
            let _name: _ = args
                .get(0)
                .to_string(scope)
                .unwrap()
                .to_rust_string_lossy(scope);
            _rv.set(v8::String::new(scope, "").unwrap().into());
        },
    );
    let get_func_instance: _ = get_func.get_function(scope).unwrap();
    proto.set(scope, get_key.into(), get_func_instance.into());
    let set_key: _ = v8::String::new(scope, "set").unwrap();
    let set_func: _ = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope, _args: v8::FunctionCallbackArguments, _rv: v8::ReturnValue| {},
    );
    let set_func_instance: _ = set_func.get_function(scope).unwrap();
    proto.set(scope, set_key.into(), set_func_instance.into());
    search_params_obj.set_prototype(scope, proto.into());
    retval.set(search_params_obj.into());
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_url_parse_absolute() {
        let url: _ = Url::parse(
            "https://example.com:8080/path/to/page?query=value#hash",
            None,
        )
        .unwrap();
        assert_eq!(url.protocol, "https");
        assert_eq!(url.host, "example.com:8080");
        assert_eq!(url.hostname, "example.com");
        assert_eq!(url.port, "8080");
        assert_eq!(url.pathname, "/path/to/page");
        assert_eq!(url.search, "?query=value");
        assert_eq!(url.hash, "#hash");
        assert_eq!(url.origin, "https://example.com:8080");
    }
    #[test]
    fn test_url_parse_relative() {
        let url: _ = Url::parse("/path/to/page", None).unwrap();
        assert_eq!(url.pathname, "/path/to/page");
    }
    #[test]
    fn test_url_search_params() {
        let mut params = UrlSearchParams::new("?key1=value1&key2=value2&key1=value3");
        assert_eq!(params.get("key1"), Some(&"value1".to_string()));
        assert_eq!(params.get("key2"), Some(&"value2".to_string()));
        params.set("key1".to_string(), "new_value".to_string());
        assert_eq!(params.get("key1"), Some(&"new_value".to_string()));
        let entries: _ = params.entries();
        assert!(entries.len() >= 1);
    }
    #[test]
    fn test_url_search_params_operations() {
        let mut params = UrlSearchParams::new("");
        params.append("key".to_string(), "value1".to_string());
        params.append("key".to_string(), "value2".to_string());
        assert_eq!(params.get("key"), Some(&"value1".to_string()));
        assert!(params.has("key"));
        params.delete("key");
        assert!(!params.has("key"));
    }
}
