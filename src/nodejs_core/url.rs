// Node.js URL模块实现
// WHATWG URL标准支持

use anyhow::Result;
use rusty_v8 as v8;
/// 设置URL API
pub fn setup_url_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global: _ = context.global(scope);
    // JIT-friendly WHATWG URL / URLSearchParams. The previous FunctionTemplate
    // constructors crossed into Rust on every `new URL()`, which was ~20x slower
    // than Node on the 20k URL microbench.
    let url_js = r#"
    (function() {
        function parseQuery(qs) {
            var out = [];
            if (qs == null || qs === '') return out;
            var s = String(qs);
            if (s.charCodeAt(0) === 63) s = s.slice(1);
            if (!s) return out;
            var parts = s.split('&');
            for (var i = 0; i < parts.length; i++) {
                var p = parts[i];
                if (!p) continue;
                var eq = p.indexOf('=');
                if (eq === -1) out.push([p, '']);
                else out.push([p.slice(0, eq), p.slice(eq + 1)]);
            }
            return out;
        }
        function URLSearchParams(init) {
            if (!(this instanceof URLSearchParams)) return new URLSearchParams(init);
            this._list = [];
            if (init == null || init === '') return;
            if (typeof init === 'string') this._list = parseQuery(init);
            else if (Array.isArray(init)) {
                for (var i = 0; i < init.length; i++) {
                    this._list.push([String(init[i][0]), String(init[i][1])]);
                }
            } else if (typeof init === 'object') {
                var keys = Object.keys(init);
                for (var k = 0; k < keys.length; k++) {
                    this._list.push([keys[k], String(init[keys[k]])]);
                }
            }
        }
        URLSearchParams.prototype.get = function(name) {
            name = String(name);
            var list = this._list;
            for (var i = 0; i < list.length; i++) {
                if (list[i][0] === name) return list[i][1];
            }
            return null;
        };
        URLSearchParams.prototype.set = function(name, value) {
            name = String(name);
            value = String(value);
            var list = this._list;
            var found = false;
            var next = [];
            for (var i = 0; i < list.length; i++) {
                if (list[i][0] === name) {
                    if (!found) { next.push([name, value]); found = true; }
                } else next.push(list[i]);
            }
            if (!found) next.push([name, value]);
            this._list = next;
        };
        URLSearchParams.prototype.append = function(name, value) {
            this._list.push([String(name), String(value)]);
        };
        URLSearchParams.prototype.delete = function(name) {
            name = String(name);
            var list = this._list;
            var next = [];
            for (var i = 0; i < list.length; i++) {
                if (list[i][0] !== name) next.push(list[i]);
            }
            this._list = next;
        };
        URLSearchParams.prototype.has = function(name) {
            return this.get(name) !== null;
        };
        URLSearchParams.prototype.toString = function() {
            var list = this._list;
            var s = '';
            for (var i = 0; i < list.length; i++) {
                if (i) s += '&';
                s += list[i][0] + '=' + list[i][1];
            }
            return s;
        };
        URLSearchParams.prototype.forEach = function(cb, thisArg) {
            var list = this._list;
            for (var i = 0; i < list.length; i++) cb.call(thisArg, list[i][1], list[i][0], this);
        };
        function parseAbs(input, base) {
            input = String(input);
            if (base && !/^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(input)) {
                var b = parseAbs(String(base), null);
                if (input.charCodeAt(0) === 47) input = b.origin + input;
                else if (input.charCodeAt(0) === 63) input = b.origin + b.pathname + input;
                else if (input.charCodeAt(0) === 35) input = b.origin + b.pathname + b.search + input;
                else {
                    var slash = b.pathname.lastIndexOf('/');
                    input = b.origin + b.pathname.slice(0, slash + 1) + input;
                }
            }
            var schemeEnd = input.indexOf('://');
            if (schemeEnd <= 0) throw new TypeError('Invalid URL');
            var protocol = input.slice(0, schemeEnd + 1);
            var rest = input.slice(schemeEnd + 3);
            var hash = '';
            var hashIdx = rest.indexOf('#');
            if (hashIdx >= 0) { hash = rest.slice(hashIdx); rest = rest.slice(0, hashIdx); }
            var search = '';
            var qIdx = rest.indexOf('?');
            if (qIdx >= 0) { search = rest.slice(qIdx); rest = rest.slice(0, qIdx); }
            var pathname = '/';
            var slashIdx = rest.indexOf('/');
            var hostport = rest;
            if (slashIdx >= 0) {
                hostport = rest.slice(0, slashIdx);
                pathname = rest.slice(slashIdx);
            }
            var hostname = hostport;
            var port = '';
            if (hostport.charCodeAt(0) === 91) {
                var end = hostport.indexOf(']');
                hostname = hostport.slice(0, end + 1);
                if (hostport.charCodeAt(end + 1) === 58) port = hostport.slice(end + 2);
            } else {
                var c = hostport.lastIndexOf(':');
                if (c >= 0) { hostname = hostport.slice(0, c); port = hostport.slice(c + 1); }
            }
            var host = port ? (hostname + ':' + port) : hostname;
            var origin = protocol + '//' + host;
            return {
                href: origin + pathname + search + hash,
                protocol: protocol,
                hostname: hostname,
                port: port,
                host: host,
                pathname: pathname,
                search: search,
                hash: hash,
                origin: origin,
                username: '',
                password: ''
            };
        }
        function URL(input, base) {
            if (!(this instanceof URL)) return new URL(input, base);
            var p = parseAbs(input, base);
            this.href = p.href;
            this.protocol = p.protocol;
            this.hostname = p.hostname;
            this.port = p.port;
            this.host = p.host;
            this.pathname = p.pathname;
            this.search = p.search;
            this.hash = p.hash;
            this.origin = p.origin;
            this.username = '';
            this.password = '';
            this.searchParams = new URLSearchParams(p.search);
        }
        URL.prototype.toString = function() { return this.href; };
        URL.prototype.toJSON = function() { return this.href; };
        URL.canParse = function(input, base) {
            try { parseAbs(input, base); return true; } catch (e) { return false; }
        };
        globalThis.URL = URL;
        globalThis.URLSearchParams = URLSearchParams;
    })();
    "#;
    if let Some(code) = v8::String::new(scope, url_js) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }
    // url对象
    let url_obj: _ = v8::Object::new(scope);
    // url.parse
    let parse_func: _ = v8::FunctionTemplate::new(scope, url_parse_callback);
    let parse_instance: _ = parse_func.get_function(scope).unwrap();
    let parse_key: _ = v8::String::new(scope, "parse").unwrap();
    url_obj.set(scope, parse_key.into(), parse_instance.into());
    // url.format
    let format_func: _ = v8::FunctionTemplate::new(scope, url_format_callback);
    let format_instance: _ = format_func.get_function(scope).unwrap();
    let format_key: _ = v8::String::new(scope, "format").unwrap();
    url_obj.set(scope, format_key.into(), format_instance.into());
    // url.resolve
    let resolve_func: _ = v8::FunctionTemplate::new(scope, url_resolve_callback);
    let resolve_instance: _ = resolve_func.get_function(scope).unwrap();
    let resolve_key: _ = v8::String::new(scope, "resolve").unwrap();
    url_obj.set(scope, resolve_key.into(), resolve_instance.into());
    let url_module_key: _ = v8::String::new(scope, "url").unwrap();
    global.set(scope, url_module_key.into(), url_obj.into());
    Ok(())
}
#[allow(dead_code)]
fn url_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let input: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let base: _ = args
        .get(1)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let url_obj: _ = v8::Object::new(scope);
    // 解析URL
    let parsed_url: _ = parse_url_string(&input, &base);
    // 设置URL属性
    if let Some(parts) = parsed_url {
        let key_href: _ = v8::String::new(scope, "href").unwrap();
        let val_href: _ = v8::String::new(scope, &parts.href).unwrap();
        url_obj.set(scope, key_href.into(), val_href.into());
        let key_protocol: _ = v8::String::new(scope, "protocol").unwrap();
        let val_protocol: _ = v8::String::new(scope, &parts.protocol).unwrap();
        url_obj.set(scope, key_protocol.into(), val_protocol.into());
        let key_hostname: _ = v8::String::new(scope, "hostname").unwrap();
        let val_hostname: _ = v8::String::new(scope, &parts.hostname).unwrap();
        url_obj.set(scope, key_hostname.into(), val_hostname.into());
        let key_port: _ = v8::String::new(scope, "port").unwrap();
        let val_port: _ = v8::String::new(scope, &parts.port).unwrap();
        url_obj.set(scope, key_port.into(), val_port.into());
        let key_pathname: _ = v8::String::new(scope, "pathname").unwrap();
        let val_pathname: _ = v8::String::new(scope, &parts.pathname).unwrap();
        url_obj.set(scope, key_pathname.into(), val_pathname.into());
        let key_search: _ = v8::String::new(scope, "search").unwrap();
        let val_search: _ = v8::String::new(scope, &parts.search).unwrap();
        url_obj.set(scope, key_search.into(), val_search.into());
        let key_hash: _ = v8::String::new(scope, "hash").unwrap();
        let val_hash: _ = v8::String::new(scope, &parts.hash).unwrap();
        url_obj.set(scope, key_hash.into(), val_hash.into());
        let key_host: _ = v8::String::new(scope, "host").unwrap();
        let val_host: _ = v8::String::new(scope, &parts.host).unwrap();
        url_obj.set(scope, key_host.into(), val_host.into());
    }
    // toString方法
    let to_string_func: _ = v8::FunctionTemplate::new(scope, url_to_string_callback);
    let to_string_instance: _ = to_string_func.get_function(scope).unwrap();
    let to_string_key: _ = v8::String::new(scope, "toString").unwrap();
    url_obj.set(scope, to_string_key.into(), to_string_instance.into());
    // toJSON方法
    let to_json_func: _ = v8::FunctionTemplate::new(scope, url_to_json_callback);
    let to_json_instance: _ = to_json_func.get_function(scope).unwrap();
    let to_json_key: _ = v8::String::new(scope, "toJSON").unwrap();
    url_obj.set(scope, to_json_key.into(), to_json_instance.into());
    // origin属性
    let origin_key: _ = v8::String::new(scope, "origin").unwrap();
    let origin_val: _ = v8::String::new(scope, "").unwrap();
    url_obj.set(scope, origin_key.into(), origin_val.into());
    retval.set(url_obj.into());
}
fn url_to_string_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let href_key: _ = v8::String::new(scope, "href").unwrap();
    let href: _ = this
        .get(scope, href_key.into())
        .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
        .unwrap_or_default();
    retval.set(v8::String::new(scope, &href).unwrap().into());
}
fn url_to_json_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let href_key: _ = v8::String::new(scope, "href").unwrap();
    let href: _ = this
        .get(scope, href_key.into())
        .unwrap_or(v8::String::new(scope, "").unwrap().into());
    retval.set(href);
}
fn search_params_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let init: _ = args.get(0);
    let params_obj: _ = v8::Object::new(scope);
    // get方法
    let get_func: _ = v8::FunctionTemplate::new(scope, search_params_get_callback);
    let get_instance: _ = get_func.get_function(scope).unwrap();
    let get_key: _ = v8::String::new(scope, "get").unwrap();
    params_obj.set(scope, get_key.into(), get_instance.into());
    // set方法
    let set_func: _ = v8::FunctionTemplate::new(scope, search_params_set_callback);
    let set_instance: _ = set_func.get_function(scope).unwrap();
    let set_key: _ = v8::String::new(scope, "set").unwrap();
    params_obj.set(scope, set_key.into(), set_instance.into());
    // append方法
    let append_func: _ = v8::FunctionTemplate::new(scope, search_params_append_callback);
    let append_instance: _ = append_func.get_function(scope).unwrap();
    let append_key: _ = v8::String::new(scope, "append").unwrap();
    params_obj.set(scope, append_key.into(), append_instance.into());
    // delete方法
    let delete_func: _ = v8::FunctionTemplate::new(scope, search_params_delete_callback);
    let delete_instance: _ = delete_func.get_function(scope).unwrap();
    let delete_key: _ = v8::String::new(scope, "delete").unwrap();
    params_obj.set(scope, delete_key.into(), delete_instance.into());
    // has方法
    let has_func: _ = v8::FunctionTemplate::new(scope, search_params_has_callback);
    let has_instance: _ = has_func.get_function(scope).unwrap();
    let has_key: _ = v8::String::new(scope, "has").unwrap();
    params_obj.set(scope, has_key.into(), has_instance.into());
    // keys方法
    let keys_func: _ = v8::FunctionTemplate::new(scope, search_params_keys_callback);
    let keys_instance: _ = keys_func.get_function(scope).unwrap();
    let keys_key: _ = v8::String::new(scope, "keys").unwrap();
    params_obj.set(scope, keys_key.into(), keys_instance.into());
    // values方法
    let values_func: _ = v8::FunctionTemplate::new(scope, search_params_values_callback);
    let values_instance: _ = values_func.get_function(scope).unwrap();
    let values_key: _ = v8::String::new(scope, "values").unwrap();
    params_obj.set(scope, values_key.into(), values_instance.into());
    // entries方法
    let entries_func: _ = v8::FunctionTemplate::new(scope, search_params_entries_callback);
    let entries_instance: _ = entries_func.get_function(scope).unwrap();
    let entries_key: _ = v8::String::new(scope, "entries").unwrap();
    params_obj.set(scope, entries_key.into(), entries_instance.into());
    // toString方法
    let to_string_func: _ = v8::FunctionTemplate::new(scope, search_params_to_string_callback);
    let to_string_instance: _ = to_string_func.get_function(scope).unwrap();
    let to_string_key: _ = v8::String::new(scope, "toString").unwrap();
    params_obj.set(scope, to_string_key.into(), to_string_instance.into());
    // _params存储
    let params_key: _ = v8::String::new(scope, "_params").unwrap();
    let params_array: _ = v8::Array::new(scope, 0);
    params_obj.set(scope, params_key.into(), params_array.into());
    // 初始化参数
    if init.is_string() {
        let query_string: _ = init.to_string(scope).unwrap().to_rust_string_lossy(scope);
        let pairs: _ = parse_query_string(&query_string);
        let params_array: _ = v8::Array::new(scope, pairs.len() as i32);
        for (i, (key, value)) in pairs.into_iter().enumerate() {
            let pair_array: _ = v8::Array::new(scope, 2);
            let val_0: _ = v8::String::new(scope, &key).unwrap().into();
            pair_array.set_index(scope, 0, val_0);
            let val_1: _ = v8::String::new(scope, &value).unwrap().into();
            pair_array.set_index(scope, 1, val_1);
            params_array.set_index(scope, i as u32, pair_array.into());
        }
        params_obj.set(scope, params_key.into(), params_array.into());
    }
    retval.set(params_obj.into());
}
fn search_params_get_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let name: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let params_key: _ = v8::String::new(scope, "_params").unwrap();
    let params_array: _ = this.get(scope, params_key.into()).unwrap();
    let mut result: v8::Local<v8::Value> = v8::null(scope).into();
    if params_array.is_array() {
        if let Ok(arr) = v8::Local::<v8::Array>::try_from(params_array) {
            for i in 0..arr.length() {
                let v: _ = arr.get_index(scope, i).unwrap();
                if let Some(pair) = if v.is_array() {
                    v8::Local::<v8::Array>::try_from(v).ok()
                } else {
                    None
                } {
                    if pair.length() >= 2 {
                        let key: _ = pair.get_index(scope, 0).unwrap();
                        let value: _ = pair.get_index(scope, 1).unwrap();
                        if key.to_string(scope).unwrap().to_rust_string_lossy(scope) == name {
                            result = value;
                            break;
                        }
                    }
                }
            }
        }
    }
    retval.set(result);
}
fn search_params_set_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let name: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let value: _ = args
        .get(1)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let params_key: _ = v8::String::new(scope, "_params").unwrap();
    let params_array: _ = this.get(scope, params_key.into()).unwrap();
    if params_array.is_array() {
        if let Ok(arr) = v8::Local::<v8::Array>::try_from(params_array) {
            let mut found = false;
            for i in 0..arr.length() {
                let v: _ = arr.get_index(scope, i).unwrap();
                if let Some(pair) = if v.is_array() {
                    v8::Local::<v8::Array>::try_from(v).ok()
                } else {
                    None
                } {
                    if pair.length() >= 2 {
                        let key: _ = pair.get_index(scope, 0).unwrap();
                        if key.to_string(scope).unwrap().to_rust_string_lossy(scope) == name {
                            let new_value: _ = v8::String::new(scope, &value).unwrap().into();
                            pair.set_index(scope, 1, new_value);
                            found = true;
                            break;
                        }
                    }
                }
            }
            if !found {
                let pair_array: _ = v8::Array::new(scope, 2);
                let name_val: _ = v8::String::new(scope, &name).unwrap().into();
                pair_array.set_index(scope, 0, name_val);
                let value_val: _ = v8::String::new(scope, &value).unwrap().into();
                pair_array.set_index(scope, 1, value_val);
                let length: _ = arr.length();
                arr.set_index(scope, length, pair_array.into());
            }
        }
    }
    retval.set(this.into());
}
fn search_params_append_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let name: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let value: _ = args
        .get(1)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let params_key: _ = v8::String::new(scope, "_params").unwrap();
    let params_array: _ = this.get(scope, params_key.into()).unwrap();
    if params_array.is_array() {
        if let Ok(arr) = v8::Local::<v8::Array>::try_from(params_array) {
            let pair_array: _ = v8::Array::new(scope, 2);
            let val_0: _ = v8::String::new(scope, &name).unwrap().into();
            pair_array.set_index(scope, 0, val_0);
            let val_1: _ = v8::String::new(scope, &value).unwrap().into();
            pair_array.set_index(scope, 1, val_1);
            let length: _ = arr.length();
            arr.set_index(scope, length, pair_array.into());
        }
    }
    retval.set(this.into());
}
fn search_params_delete_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let name: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let params_key: _ = v8::String::new(scope, "_params").unwrap();
    let params_array: _ = this.get(scope, params_key.into()).unwrap();
    if params_array.is_array() {
        if let Ok(arr) = v8::Local::<v8::Array>::try_from(params_array) {
            let new_arr = v8::Array::new(scope, 0);
            let mut new_index = 0;
            for i in 0..arr.length() {
                let v: _ = arr.get_index(scope, i).unwrap();
                if let Some(pair) = if v.is_array() {
                    v8::Local::<v8::Array>::try_from(v).ok()
                } else {
                    None
                } {
                    if pair.length() >= 2 {
                        let key: _ = pair.get_index(scope, 0).unwrap();
                        if key.to_string(scope).unwrap().to_rust_string_lossy(scope) != name {
                            new_arr.set_index(scope, new_index, pair.into());
                            new_index += 1;
                        }
                    }
                }
            }
            this.set(scope, params_key.into(), new_arr.into());
        }
    }
    retval.set(this.into());
}
fn search_params_has_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let name: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let params_key: _ = v8::String::new(scope, "_params").unwrap();
    let params_array: _ = this.get(scope, params_key.into()).unwrap();
    let mut has = false;
    if params_array.is_array() {
        if let Ok(arr) = v8::Local::<v8::Array>::try_from(params_array) {
            for i in 0..arr.length() {
                let v: _ = arr.get_index(scope, i).unwrap();
                if let Some(pair) = if v.is_array() {
                    v8::Local::<v8::Array>::try_from(v).ok()
                } else {
                    None
                } {
                    if pair.length() >= 2 {
                        let key: _ = pair.get_index(scope, 0).unwrap();
                        if key.to_string(scope).unwrap().to_rust_string_lossy(scope) == name {
                            has = true;
                            break;
                        }
                    }
                }
            }
        }
    }
    retval.set(v8::Boolean::new(scope, has).into());
}
fn search_params_keys_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let params_key: _ = v8::String::new(scope, "_params").unwrap();
    let params_array: _ = this.get(scope, params_key.into()).unwrap();
    let keys_array: _ = v8::Array::new(scope, 0);
    if params_array.is_array() {
        if let Ok(arr) = v8::Local::<v8::Array>::try_from(params_array) {
            let mut key_index = 0;
            let mut seen_keys = std::collections::HashSet::new();
            for i in 0..arr.length() {
                let v: _ = arr.get_index(scope, i).unwrap();
                if let Some(pair) = if v.is_array() {
                    v8::Local::<v8::Array>::try_from(v).ok()
                } else {
                    None
                } {
                    if pair.length() >= 2 {
                        let key: _ = pair.get_index(scope, 0).unwrap();
                        let key_str: _ = key.to_string(scope).unwrap().to_rust_string_lossy(scope);
                        if seen_keys.insert(key_str.clone()) {
                            let _val_0: _ = v8::String::new(scope, &key_str).unwrap();
                            keys_array.set_index(scope, key_index, _val_0.into());
                            key_index += 1;
                        }
                    }
                }
            }
        }
    }
    retval.set(keys_array.into());
}
fn search_params_values_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let params_key: _ = v8::String::new(scope, "_params").unwrap();
    let params_array: _ = this.get(scope, params_key.into()).unwrap();
    let values_array: _ = v8::Array::new(scope, 0);
    if params_array.is_array() {
        if let Ok(arr) = v8::Local::<v8::Array>::try_from(params_array) {
            let mut value_index = 0;
            for i in 0..arr.length() {
                let v: _ = arr.get_index(scope, i).unwrap();
                if let Some(pair) = if v.is_array() {
                    v8::Local::<v8::Array>::try_from(v).ok()
                } else {
                    None
                } {
                    if pair.length() >= 2 {
                        let value: _ = pair.get_index(scope, 1).unwrap();
                        values_array.set_index(scope, value_index, value);
                        value_index += 1;
                    }
                }
            }
        }
    }
    retval.set(values_array.into());
}
fn search_params_entries_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let params_key: _ = v8::String::new(scope, "_params").unwrap();
    let params_array: _ = this.get(scope, params_key.into()).unwrap();
    retval.set(params_array);
}
fn search_params_to_string_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let params_key: _ = v8::String::new(scope, "_params").unwrap();
    let params_array: _ = this.get(scope, params_key.into()).unwrap();
    let mut query_string = String::new();
    if params_array.is_array() {
        if let Ok(arr) = v8::Local::<v8::Array>::try_from(params_array) {
            for i in 0..arr.length() {
                let v: _ = arr.get_index(scope, i).unwrap();
                if let Some(pair) = if v.is_array() {
                    v8::Local::<v8::Array>::try_from(v).ok()
                } else {
                    None
                } {
                    if pair.length() >= 2 {
                        let key: _ = pair.get_index(scope, 0).unwrap();
                        let value: _ = pair.get_index(scope, 1).unwrap();
                        let key_str: _ = key.to_string(scope).unwrap().to_rust_string_lossy(scope);
                        let value_str: _ =
                            value.to_string(scope).unwrap().to_rust_string_lossy(scope);
                        if !query_string.is_empty() {
                            query_string.push('&');
                        }
                        query_string.push_str(&format!("{}={}", key_str, value_str));
                    }
                }
            }
        }
    }
    retval.set(v8::String::new(scope, &query_string).unwrap().into());
}
// 旧版URL API兼容函数
fn url_parse_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let url_str: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let parsed_url: _ = parse_url_string(&url_str, "");
    let url_obj: _ = v8::Object::new(scope);
    if let Some(parts) = parsed_url {
        let key_href: _ = v8::String::new(scope, "href").unwrap();
        let val_href: _ = v8::String::new(scope, &parts.href).unwrap();
        url_obj.set(scope, key_href.into(), val_href.into());
        let key_protocol: _ = v8::String::new(scope, "protocol").unwrap();
        let val_protocol: _ = v8::String::new(scope, &parts.protocol).unwrap();
        url_obj.set(scope, key_protocol.into(), val_protocol.into());
        let key_hostname: _ = v8::String::new(scope, "hostname").unwrap();
        let val_hostname: _ = v8::String::new(scope, &parts.hostname).unwrap();
        url_obj.set(scope, key_hostname.into(), val_hostname.into());
        let key_port: _ = v8::String::new(scope, "port").unwrap();
        let val_port: _ = v8::String::new(scope, &parts.port).unwrap();
        url_obj.set(scope, key_port.into(), val_port.into());
        let key_pathname: _ = v8::String::new(scope, "pathname").unwrap();
        let val_pathname: _ = v8::String::new(scope, &parts.pathname).unwrap();
        url_obj.set(scope, key_pathname.into(), val_pathname.into());
        let key_search: _ = v8::String::new(scope, "search").unwrap();
        let val_search: _ = v8::String::new(scope, &parts.search).unwrap();
        url_obj.set(scope, key_search.into(), val_search.into());
        let key_hash: _ = v8::String::new(scope, "hash").unwrap();
        let val_hash: _ = v8::String::new(scope, &parts.hash).unwrap();
        url_obj.set(scope, key_hash.into(), val_hash.into());
        let key_host: _ = v8::String::new(scope, "host").unwrap();
        let val_host: _ = v8::String::new(scope, &parts.host).unwrap();
        url_obj.set(scope, key_host.into(), val_host.into());
    }
    retval.set(url_obj.into());
}
fn url_format_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let url_obj: _ = args.get(0);
    let mut href = String::new();
    if let Some(obj) = url_obj.to_object(scope) {
        let protocol_key: _ = v8::String::new(scope, "protocol").unwrap();
        let protocol: _ = obj
            .get(scope, protocol_key.into())
            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
            .unwrap_or_default();
        let hostname_key: _ = v8::String::new(scope, "hostname").unwrap();
        let hostname: _ = obj
            .get(scope, hostname_key.into())
            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
            .unwrap_or_default();
        let pathname_key: _ = v8::String::new(scope, "pathname").unwrap();
        let pathname: _ = obj
            .get(scope, pathname_key.into())
            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
            .unwrap_or_default();
        href = format!("{}{}{}", protocol, hostname, pathname);
    }
    retval.set(v8::String::new(scope, &href).unwrap().into());
}
fn url_resolve_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let from: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let to: _ = args
        .get(1)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let result: _ = if to.starts_with('/') {
        // 绝对路径
        format!("{}{}", from, to)
    } else {
        // 相对路径
        format!("{}/{}", from.trim_end_matches('/'), to)
    };
    retval.set(v8::String::new(scope, &result).unwrap().into());
}
// 辅助数据结构
struct ParsedUrl {
    href: String,
    protocol: String,
    hostname: String,
    port: String,
    pathname: String,
    search: String,
    hash: String,
    host: String,
}
// URL解析辅助函数
fn parse_url_string(url: &str, _base: &str) -> Option<ParsedUrl> {
    // 简化的URL解析实现
    let mut parts = ParsedUrl {
        href: url.to_string(),
        protocol: "".to_string(),
        hostname: "".to_string(),
        port: "".to_string(),
        pathname: "".to_string(),
        search: "".to_string(),
        hash: "".to_string(),
        host: "".to_string(),
    };
    // 提取协议
    if let Some(colon_pos) = url.find("://") {
        parts.protocol = format!("{}:", &url[..colon_pos]);
        let remainder: _ = &url[colon_pos + 3..];
        // 提取主机和路径
        if let Some(slash_pos) = remainder.find('/') {
            let host_part: _ = &remainder[..slash_pos];
            let path_part: _ = &remainder[slash_pos..];
            parts.hostname = host_part.to_string();
            parts.host = host_part.to_string();
            parts.pathname = path_part.to_string();
        } else {
            parts.hostname = remainder.to_string();
            parts.host = remainder.to_string();
            parts.pathname = "/".to_string();
        }
    }
    Some(parts)
}
// 查询字符串解析
fn parse_query_string(query: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let parts: _ = query.split('&');
    for part in parts {
        if let Some(eq_pos) = part.find('=') {
            let key: _ = urlencoding::decode(&part[..eq_pos])
                .unwrap_or_default()
                .to_string();
            let value: _ = urlencoding::decode(&part[eq_pos + 1..])
                .unwrap_or_default()
                .to_string();
            pairs.push((key, value));
        }
    }
    pairs
}
