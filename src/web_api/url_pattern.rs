// WHATWG URLPattern API implementation for WinterTC ECMA-429
// Provides standard URLPattern routing and matching engine for server and edge runtimes.

use anyhow::Result;
use regex::Regex;
use rusty_v8 as v8;
use std::collections::HashMap;

/// Internal compiled component pattern
#[derive(Clone, Debug)]
pub struct ComponentPattern {
    pub raw: String,
    pub regex: Regex,
    pub group_names: Vec<String>,
}

impl ComponentPattern {
    pub fn new(raw: &str, is_pathname: bool) -> Self {
        let raw_str = if raw.is_empty() { "*" } else { raw };
        let (regex_str, group_names) = compile_pattern_to_regex(raw_str, is_pathname);
        let regex =
            Regex::new(&format!("^{}$", regex_str)).unwrap_or_else(|_| Regex::new("^.*$").unwrap());
        Self {
            raw: raw_str.to_string(),
            regex,
            group_names,
        }
    }

    pub fn matches(&self, input: &str) -> Option<HashMap<String, String>> {
        if let Some(captures) = self.regex.captures(input) {
            let mut groups = HashMap::new();
            for name in &self.group_names {
                if let Some(m) = captures.name(name) {
                    groups.insert(name.clone(), m.as_str().to_string());
                }
            }
            // Capture numbered wildcard groups (0, 1, etc.)
            let mut idx = 0;
            for cap in captures.iter().skip(1) {
                if let Some(m) = cap {
                    let key = idx.to_string();
                    if !groups.contains_key(&key) {
                        groups.insert(key, m.as_str().to_string());
                    }
                    idx += 1;
                }
            }
            Some(groups)
        } else {
            None
        }
    }
}

/// Convert URLPattern glob-like syntax to Regex string
fn compile_pattern_to_regex(pattern: &str, is_pathname: bool) -> (String, Vec<String>) {
    if pattern == "*" {
        return (".*".to_string(), Vec::new());
    }

    let mut result = String::new();
    let mut group_names = Vec::new();
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c == ':' {
            // Named group :name or :name(regex)
            i += 1;
            let mut name = String::new();
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                name.push(chars[i]);
                i += 1;
            }
            if !name.is_empty() {
                group_names.push(name.clone());
                if is_pathname {
                    result.push_str(&format!("(?P<{}>[^/]+)", name));
                } else {
                    result.push_str(&format!("(?P<{}>[^/?#]+)", name));
                }
            } else {
                result.push(':');
            }
        } else if c == '*' {
            // Wildcard
            result.push_str("(.*)");
            i += 1;
        } else if c == '?' {
            // Optional previous or standard question mark in search
            if is_pathname {
                result.push('?');
            } else {
                result.push_str("\\?");
            }
            i += 1;
        } else if "\\.+*?()|[]{}^$".contains(c) {
            result.push('\\');
            result.push(c);
            i += 1;
        } else {
            result.push(c);
            i += 1;
        }
    }

    (result, group_names)
}

/// URLPattern internal model
#[derive(Clone, Debug)]
pub struct URLPatternInternal {
    pub protocol: ComponentPattern,
    pub username: ComponentPattern,
    pub password: ComponentPattern,
    pub hostname: ComponentPattern,
    pub port: ComponentPattern,
    pub pathname: ComponentPattern,
    pub search: ComponentPattern,
    pub hash: ComponentPattern,
    pub has_regexp_groups: bool,
}

impl URLPatternInternal {
    pub fn new(
        protocol: &str,
        username: &str,
        password: &str,
        hostname: &str,
        port: &str,
        pathname: &str,
        search: &str,
        hash: &str,
    ) -> Self {
        let p_protocol = ComponentPattern::new(protocol, false);
        let p_username = ComponentPattern::new(username, false);
        let p_password = ComponentPattern::new(password, false);
        let p_hostname = ComponentPattern::new(hostname, false);
        let p_port = ComponentPattern::new(port, false);
        let p_pathname = ComponentPattern::new(pathname, true);
        let p_search = ComponentPattern::new(search, false);
        let p_hash = ComponentPattern::new(hash, false);

        let has_regexp = !p_pathname.group_names.is_empty()
            || !p_hostname.group_names.is_empty()
            || !p_search.group_names.is_empty();

        Self {
            protocol: p_protocol,
            username: p_username,
            password: p_password,
            hostname: p_hostname,
            port: p_port,
            pathname: p_pathname,
            search: p_search,
            hash: p_hash,
            has_regexp_groups: has_regexp,
        }
    }

    pub fn test(&self, url_str: &str, base_url: Option<&str>) -> bool {
        self.exec(url_str, base_url).is_some()
    }

    pub fn exec(
        &self,
        url_str: &str,
        base_url: Option<&str>,
    ) -> Option<HashMap<String, (String, HashMap<String, String>)>> {
        // Parse input url
        let parsed_url = if let Ok(u) = url::Url::parse(url_str) {
            u
        } else if let Some(base) = base_url {
            if let Ok(b) = url::Url::parse(base) {
                b.join(url_str).ok()?
            } else {
                url::Url::parse(&format!("https://dummy.local{}", url_str)).ok()?
            }
        } else if url_str.starts_with('/') {
            url::Url::parse(&format!("https://dummy.local{}", url_str)).ok()?
        } else {
            return None;
        };

        let proto = parsed_url.scheme();
        let user = parsed_url.username();
        let pass = parsed_url.password().unwrap_or("");
        let host = parsed_url.host_str().unwrap_or("");
        let port = parsed_url.port().map(|p| p.to_string()).unwrap_or_default();
        let path = parsed_url.path();
        let query = parsed_url
            .query()
            .map(|q| format!("?{}", q))
            .unwrap_or_default();
        let fragment = parsed_url
            .fragment()
            .map(|f| format!("#{}", f))
            .unwrap_or_default();

        let m_proto = self.protocol.matches(proto)?;
        let m_user = self.username.matches(user)?;
        let m_pass = self.password.matches(pass)?;
        let m_host = self.hostname.matches(host)?;
        let m_port = self.port.matches(&port)?;
        let m_path = self.pathname.matches(path)?;
        let m_search = self.search.matches(&query)?;
        let m_hash = self.hash.matches(&fragment)?;

        let mut res = HashMap::new();
        res.insert("protocol".to_string(), (proto.to_string(), m_proto));
        res.insert("username".to_string(), (user.to_string(), m_user));
        res.insert("password".to_string(), (pass.to_string(), m_pass));
        res.insert("hostname".to_string(), (host.to_string(), m_host));
        res.insert("port".to_string(), (port, m_port));
        res.insert("pathname".to_string(), (path.to_string(), m_path));
        res.insert("search".to_string(), (query, m_search));
        res.insert("hash".to_string(), (fragment, m_hash));

        Some(res)
    }
}

/// Setup URLPattern API in V8 context
pub fn setup_url_pattern_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    let constructor_fn = v8::Function::new(scope, url_pattern_constructor)
        .ok_or_else(|| anyhow::anyhow!("Failed to create URLPattern constructor"))?;

    let prototype = v8::Object::new(scope);

    // Methods: test and exec
    let test_fn = v8::Function::new(scope, url_pattern_test).unwrap();
    let test_key = v8::String::new(scope, "test").unwrap();
    prototype.set(scope, test_key.into(), test_fn.into());

    let exec_fn = v8::Function::new(scope, url_pattern_exec).unwrap();
    let exec_key = v8::String::new(scope, "exec").unwrap();
    prototype.set(scope, exec_key.into(), exec_fn.into());

    // Link constructor and prototype
    let proto_key = v8::String::new(scope, "prototype").unwrap();
    constructor_fn.set(scope, proto_key.into(), prototype.into());

    let constructor_key = v8::String::new(scope, "constructor").unwrap();
    prototype.set(scope, constructor_key.into(), constructor_fn.into());

    // Symbol.toStringTag
    let to_string_tag_sym = v8::Symbol::get_to_string_tag(scope);
    let tag_val = v8::String::new(scope, "URLPattern").unwrap();
    prototype.set(scope, to_string_tag_sym.into(), tag_val.into());

    let name_key = v8::String::new(scope, "URLPattern").unwrap();
    global.set(scope, name_key.into(), constructor_fn.into());

    Ok(())
}

fn url_pattern_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();

    let mut protocol = "*".to_string();
    let mut username = "*".to_string();
    let mut password = "*".to_string();
    let mut hostname = "*".to_string();
    let mut port = "*".to_string();
    let mut pathname = "*".to_string();
    let mut search = "*".to_string();
    let mut hash = "*".to_string();

    let arg0 = args.get(0);
    let arg1 = args.get(1);

    let base_url = if !arg1.is_undefined() {
        arg1.to_string(scope).map(|s| s.to_rust_string_lossy(scope))
    } else {
        None
    };

    if arg0.is_string() {
        let pattern_str = arg0.to_string(scope).unwrap().to_rust_string_lossy(scope);
        if let Ok(u) = url::Url::parse(&pattern_str) {
            protocol = u.scheme().to_string();
            username = u.username().to_string();
            if let Some(p) = u.password() {
                password = p.to_string();
            }
            if let Some(h) = u.host_str() {
                hostname = h.to_string();
            }
            if let Some(p) = u.port() {
                port = p.to_string();
            }
            pathname = u.path().to_string();
            if let Some(q) = u.query() {
                search = format!("?{}", q);
            }
            if let Some(f) = u.fragment() {
                hash = format!("#{}", f);
            }
        } else if let Some(base) = &base_url {
            if let Ok(b) = url::Url::parse(base) {
                if let Ok(joined) = b.join(&pattern_str) {
                    protocol = joined.scheme().to_string();
                    if let Some(h) = joined.host_str() {
                        hostname = h.to_string();
                    }
                    pathname = joined.path().to_string();
                }
            } else {
                pathname = pattern_str;
            }
        } else {
            pathname = pattern_str;
        }
    } else if arg0.is_object() {
        let obj: v8::Local<v8::Object> = v8::Local::cast(arg0);

        macro_rules! get_comp {
            ($field:ident, $key:literal) => {
                let k = v8::String::new(scope, $key).unwrap();
                if let Some(val) = obj.get(scope, k.into()) {
                    if !val.is_undefined() {
                        if let Some(s) = val.to_string(scope) {
                            $field = s.to_rust_string_lossy(scope);
                        }
                    }
                }
            };
        }

        get_comp!(protocol, "protocol");
        get_comp!(username, "username");
        get_comp!(password, "password");
        get_comp!(hostname, "hostname");
        get_comp!(port, "port");
        get_comp!(pathname, "pathname");
        get_comp!(search, "search");
        get_comp!(hash, "hash");
    }

    let pattern = URLPatternInternal::new(
        &protocol, &username, &password, &hostname, &port, &pathname, &search, &hash,
    );

    // Store pattern properties on JS object
    macro_rules! set_prop {
        ($key:literal, $val:expr) => {
            let k = v8::String::new(scope, $key).unwrap();
            let v = v8::String::new(scope, $val).unwrap();
            this.set(scope, k.into(), v.into());
        };
    }

    set_prop!("protocol", &pattern.protocol.raw);
    set_prop!("username", &pattern.username.raw);
    set_prop!("password", &pattern.password.raw);
    set_prop!("hostname", &pattern.hostname.raw);
    set_prop!("port", &pattern.port.raw);
    set_prop!("pathname", &pattern.pathname.raw);
    set_prop!("search", &pattern.search.raw);
    set_prop!("hash", &pattern.hash.raw);

    let k_has_regexp = v8::String::new(scope, "hasRegExpGroups").unwrap();
    let v_has_regexp = v8::Boolean::new(scope, pattern.has_regexp_groups);
    this.set(scope, k_has_regexp.into(), v_has_regexp.into());

    retval.set(this.into());
}

fn get_pattern_from_this(
    scope: &mut v8::PinScope,
    this: v8::Local<v8::Object>,
) -> URLPatternInternal {
    macro_rules! read_prop {
        ($key:literal) => {{
            let k = v8::String::new(scope, $key).unwrap();
            this.get(scope, k.into())
                .and_then(|v| v.to_string(scope))
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_else(|| "*".to_string())
        }};
    }

    let proto = read_prop!("protocol");
    let user = read_prop!("username");
    let pass = read_prop!("password");
    let host = read_prop!("hostname");
    let port = read_prop!("port");
    let path = read_prop!("pathname");
    let search = read_prop!("search");
    let hash = read_prop!("hash");

    URLPatternInternal::new(&proto, &user, &pass, &host, &port, &path, &search, &hash)
}

fn parse_url_args(
    scope: &mut v8::PinScope,
    args: &v8::FunctionCallbackArguments,
) -> (String, Option<String>) {
    let arg0 = args.get(0);
    let input = if arg0.is_string() {
        arg0.to_string(scope).unwrap().to_rust_string_lossy(scope)
    } else if arg0.is_object() {
        let obj: v8::Local<v8::Object> = v8::Local::cast(arg0);
        let k_url = v8::String::new(scope, "url").unwrap();
        if let Some(u) = obj
            .get(scope, k_url.into())
            .and_then(|v| v.to_string(scope))
        {
            u.to_rust_string_lossy(scope)
        } else {
            let k_path = v8::String::new(scope, "pathname").unwrap();
            obj.get(scope, k_path.into())
                .and_then(|v| v.to_string(scope))
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default()
        }
    } else {
        String::new()
    };

    let arg1 = args.get(1);
    let base = if !arg1.is_undefined() {
        arg1.to_string(scope).map(|s| s.to_rust_string_lossy(scope))
    } else {
        None
    };

    (input, base)
}

fn url_pattern_test(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let pattern = get_pattern_from_this(scope, this);
    let (input, base) = parse_url_args(scope, &args);

    let matched = pattern.test(&input, base.as_deref());
    retval.set(v8::Boolean::new(scope, matched).into());
}

fn url_pattern_exec(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let pattern = get_pattern_from_this(scope, this);
    let (input, base) = parse_url_args(scope, &args);

    if let Some(matches) = pattern.exec(&input, base.as_deref()) {
        let result_obj = v8::Object::new(scope);

        // inputs array: [input, base]
        let inputs_arr = v8::Array::new(scope, 2);
        let in_str = v8::String::new(scope, &input).unwrap();
        inputs_arr.set_index(scope, 0, in_str.into());
        if let Some(b) = &base {
            let b_str = v8::String::new(scope, b).unwrap();
            inputs_arr.set_index(scope, 1, b_str.into());
        }
        let k_inputs = v8::String::new(scope, "inputs").unwrap();
        result_obj.set(scope, k_inputs.into(), inputs_arr.into());

        for (comp_name, (comp_input, comp_groups)) in matches {
            let comp_obj = v8::Object::new(scope);
            let k_in = v8::String::new(scope, "input").unwrap();
            let v_in = v8::String::new(scope, &comp_input).unwrap();
            comp_obj.set(scope, k_in.into(), v_in.into());

            let groups_obj = v8::Object::new(scope);
            for (g_name, g_val) in comp_groups {
                let gk = v8::String::new(scope, &g_name).unwrap();
                let gv = v8::String::new(scope, &g_val).unwrap();
                groups_obj.set(scope, gk.into(), gv.into());
            }
            let k_groups = v8::String::new(scope, "groups").unwrap();
            comp_obj.set(scope, k_groups.into(), groups_obj.into());

            let comp_key = v8::String::new(scope, &comp_name).unwrap();
            result_obj.set(scope, comp_key.into(), comp_obj.into());
        }

        retval.set(result_obj.into());
    } else {
        retval.set(v8::null(scope).into());
    }
}
