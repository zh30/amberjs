// Preview Cache / CacheStorage (`caches`) for the CLI path.
//
// In-process map of named caches holding Request URL/method plus Response
// status/headers/body. This is not HTTP cache (no freshness, Vary).
// Service-worker fetch intercept (Preview) can `respondWith(caches.match(...))`.

use anyhow::Result;
use rusty_v8 as v8;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::fetch::{consume_response_body_for_object, header_entries_from_value};

const CACHE_NAME_KEY: &str = "__amberCacheName";

#[derive(Clone, Debug)]
struct CachedRequest {
    url: String,
    method: String,
}

#[derive(Clone, Debug)]
struct CachedResponse {
    url: String,
    status: u16,
    status_text: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    response_type: String,
}

#[derive(Clone, Default, Debug)]
struct NamedCache {
    entries: Vec<(CachedRequest, CachedResponse)>,
}

#[derive(Default, Debug)]
struct CacheStorageBackend {
    names: Vec<String>,
    caches: HashMap<String, NamedCache>,
}

fn backend() -> &'static Mutex<CacheStorageBackend> {
    static BACKEND: OnceLock<Mutex<CacheStorageBackend>> = OnceLock::new();
    BACKEND.get_or_init(|| Mutex::new(CacheStorageBackend::default()))
}

fn strip_fragment(url: &str) -> String {
    match url.find('#') {
        Some(index) => url[..index].to_string(),
        None => url.to_string(),
    }
}

fn methods_match(stored: &str, query: &str) -> bool {
    let stored = stored.to_ascii_uppercase();
    let query = query.to_ascii_uppercase();
    if stored == query {
        return true;
    }
    matches!(
        (stored.as_str(), query.as_str()),
        ("GET", "HEAD") | ("HEAD", "GET")
    )
}

fn request_matches(stored: &CachedRequest, url: &str, method: &str) -> bool {
    stored.url == url && methods_match(&stored.method, method)
}

fn js_to_string(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> String {
    value
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default()
}

fn parse_request(
    scope: &mut v8::PinScope,
    input: v8::Local<v8::Value>,
) -> Result<(String, String), String> {
    if input.is_undefined() || input.is_null() {
        return Err("Cache request is required".to_string());
    }
    if input.is_string() {
        let url = strip_fragment(&js_to_string(scope, input));
        if url.is_empty() {
            return Err("Cache request URL must not be empty".to_string());
        }
        return Ok((url, "GET".to_string()));
    }
    let Some(obj) = input.to_object(scope) else {
        return Err("Cache request must be a URL string or Request".to_string());
    };
    let url_key = v8::String::new(scope, "url").unwrap().into();
    let method_key = v8::String::new(scope, "method").unwrap().into();
    let url = obj
        .get(scope, url_key)
        .map(|value| strip_fragment(&js_to_string(scope, value)))
        .unwrap_or_default();
    if url.is_empty() {
        return Err("Cache request URL must not be empty".to_string());
    }
    let method = obj
        .get(scope, method_key)
        .filter(|value| value.is_string())
        .map(|value| js_to_string(scope, value))
        .filter(|method| !method.is_empty())
        .unwrap_or_else(|| "GET".to_string());
    Ok((url, method))
}

fn snapshot_response(
    scope: &mut v8::PinScope,
    response_val: v8::Local<v8::Value>,
    request_url: &str,
) -> Result<CachedResponse, String> {
    let Some(obj) = response_val.to_object(scope) else {
        return Err("Cache.put requires a Response".to_string());
    };

    let status = obj
        .get(scope, v8::String::new(scope, "status").unwrap().into())
        .and_then(|value| value.to_integer(scope))
        .map(|value| value.value() as u16)
        .unwrap_or(200);
    if status == 206 {
        return Err("Cache.put does not support 206 Partial Content".to_string());
    }

    let status_text = obj
        .get(scope, v8::String::new(scope, "statusText").unwrap().into())
        .map(|value| js_to_string(scope, value))
        .unwrap_or_default();
    let url = obj
        .get(scope, v8::String::new(scope, "url").unwrap().into())
        .map(|value| js_to_string(scope, value))
        .filter(|url| !url.is_empty())
        .unwrap_or_else(|| request_url.to_string());
    let response_type = obj
        .get(scope, v8::String::new(scope, "type").unwrap().into())
        .map(|value| js_to_string(scope, value))
        .filter(|kind| !kind.is_empty())
        .unwrap_or_else(|| "default".to_string());
    let headers = obj
        .get(scope, v8::String::new(scope, "headers").unwrap().into())
        .map(|value| header_entries_from_value(scope, value))
        .unwrap_or_default();
    let body = match consume_response_body_for_object(scope, obj) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => Vec::new(),
        Err(()) => return Err("Cache.put could not read the Response body".to_string()),
    };

    Ok(CachedResponse {
        url,
        status,
        status_text,
        headers,
        body,
        response_type,
    })
}

fn promise_resolver<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    rv: &mut v8::ReturnValue,
) -> Option<v8::Local<'a, v8::PromiseResolver>> {
    let resolver = v8::PromiseResolver::new(scope)?;
    let promise = resolver.get_promise(scope);
    rv.set(promise.into());
    Some(resolver)
}

fn resolve_value(
    scope: &mut v8::PinScope,
    resolver: v8::Local<v8::PromiseResolver>,
    value: v8::Local<v8::Value>,
) {
    resolver.resolve(scope, value);
}

fn reject_type_error(
    scope: &mut v8::PinScope,
    resolver: v8::Local<v8::PromiseResolver>,
    message: &str,
) {
    let error_message = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::type_error(scope, error_message);
    resolver.reject(scope, error);
}

fn cache_name_from_this(scope: &mut v8::PinScope, this: v8::Local<v8::Object>) -> Option<String> {
    let key = v8::String::new(scope, CACHE_NAME_KEY).unwrap().into();
    this.get(scope, key)
        .filter(|value| value.is_string())
        .map(|value| js_to_string(scope, value))
}

fn reconstruct_request<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    request: &CachedRequest,
) -> v8::Local<'a, v8::Value> {
    let context = scope.get_current_context();
    let global = context.global(scope);
    let ctor_key = v8::String::new(scope, "Request").unwrap().into();
    if let Some(ctor_val) = global.get(scope, ctor_key) {
        if let Ok(ctor) = v8::Local::<v8::Function>::try_from(ctor_val) {
            let url = v8::String::new(scope, &request.url).unwrap().into();
            let init = v8::Object::new(scope);
            let method_key = v8::String::new(scope, "method").unwrap().into();
            let method_val = v8::String::new(scope, &request.method).unwrap().into();
            init.set(scope, method_key, method_val);
            if let Some(instance) = ctor.new_instance(scope, &[url, init.into()]) {
                return instance.into();
            }
        }
    }
    v8::String::new(scope, &request.url).unwrap().into()
}

fn reconstruct_response<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    response: &CachedResponse,
) -> v8::Local<'a, v8::Value> {
    let context = scope.get_current_context();
    let global = context.global(scope);
    let body = {
        let buffer = v8::ArrayBuffer::new(scope, response.body.len());
        let store = buffer.get_backing_store();
        let ptr = store.as_ref().as_ptr() as *mut u8;
        if !response.body.is_empty() && !ptr.is_null() {
            unsafe {
                std::ptr::copy_nonoverlapping(response.body.as_ptr(), ptr, response.body.len());
            }
        }
        v8::Uint8Array::new(scope, buffer, 0, response.body.len())
            .unwrap()
            .into()
    };
    let init = v8::Object::new(scope);
    let status_key = v8::String::new(scope, "status").unwrap().into();
    let status_val = v8::Integer::new_from_unsigned(scope, response.status as u32).into();
    init.set(scope, status_key, status_val);
    let status_text_key = v8::String::new(scope, "statusText").unwrap().into();
    let status_text_val = v8::String::new(scope, &response.status_text)
        .unwrap()
        .into();
    init.set(scope, status_text_key, status_text_val);

    let headers_init = v8::Array::new(scope, response.headers.len() as i32);
    for (index, (name, value)) in response.headers.iter().enumerate() {
        let pair = v8::Array::new(scope, 2);
        let name_val = v8::String::new(scope, name).unwrap().into();
        let value_val = v8::String::new(scope, value).unwrap().into();
        pair.set_index(scope, 0, name_val);
        pair.set_index(scope, 1, value_val);
        headers_init.set_index(scope, index as u32, pair.into());
    }
    let headers_key = v8::String::new(scope, "headers").unwrap().into();
    init.set(scope, headers_key, headers_init.into());

    let ctor_key = v8::String::new(scope, "Response").unwrap().into();
    if let Some(ctor_val) = global.get(scope, ctor_key) {
        if let Ok(ctor) = v8::Local::<v8::Function>::try_from(ctor_val) {
            if let Some(instance) = ctor.new_instance(scope, &[body, init.into()]) {
                let url_key = v8::String::new(scope, "url").unwrap().into();
                let url_val = v8::String::new(scope, &response.url).unwrap().into();
                instance.set(scope, url_key, url_val);
                let type_key = v8::String::new(scope, "type").unwrap().into();
                let type_val = v8::String::new(scope, &response.response_type)
                    .unwrap()
                    .into();
                instance.set(scope, type_key, type_val);
                return instance.into();
            }
        }
    }
    v8::undefined(scope).into()
}

fn attach_cache_fetch_helpers(scope: &mut v8::PinScope, cache_obj: v8::Local<v8::Object>) {
    let source = r#"(function(cache) {
  cache.add = function(request) {
    if (typeof fetch !== 'function') {
      return Promise.reject(new TypeError('Cache.add requires fetch'));
    }
    const req = request;
    return fetch(req).then(function(response) {
      if (!response || response.ok === false) {
        throw new TypeError('Cache.add failed because the response is not ok');
      }
      return cache.put(req, response);
    });
  };
  cache.addAll = function(requests) {
    if (requests == null || typeof requests[Symbol.iterator] !== 'function') {
      return Promise.reject(new TypeError('Cache.addAll requires an iterable'));
    }
    const list = Array.from(requests);
    if (typeof fetch !== 'function') {
      return Promise.reject(new TypeError('Cache.addAll requires fetch'));
    }
    return Promise.all(list.map(function(item) {
      return fetch(item).then(function(response) {
        if (!response || response.ok === false) {
          throw new TypeError('Cache.addAll failed because a response is not ok');
        }
        return [item, response];
      });
    })).then(function(pairs) {
      let chain = Promise.resolve();
      pairs.forEach(function(pair) {
        chain = chain.then(function() { return cache.put(pair[0], pair[1]); });
      });
      return chain;
    });
  };
})"#;
    let Some(code) = v8::String::new(scope, source) else {
        return;
    };
    let Some(script) = v8::Script::compile(scope, code, None) else {
        return;
    };
    let Some(result) = script.run(scope) else {
        return;
    };
    let Ok(func) = v8::Local::<v8::Function>::try_from(result) else {
        return;
    };
    let recv: v8::Local<v8::Value> = v8::undefined(scope).into();
    let _ = func.call(scope, recv, &[cache_obj.into()]);
}

fn create_cache_object<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    name: &str,
) -> v8::Local<'a, v8::Object> {
    let cache_obj = v8::Object::new(scope);
    let name_key = v8::String::new(scope, CACHE_NAME_KEY).unwrap();
    let name_val = v8::String::new(scope, name).unwrap();
    cache_obj.set(scope, name_key.into(), name_val.into());

    let match_fn = v8::FunctionTemplate::new(scope, cache_match_callback)
        .get_function(scope)
        .unwrap();
    let match_key = v8::String::new(scope, "match").unwrap();
    cache_obj.set(scope, match_key.into(), match_fn.into());

    let put_fn = v8::FunctionTemplate::new(scope, cache_put_callback)
        .get_function(scope)
        .unwrap();
    let put_key = v8::String::new(scope, "put").unwrap();
    cache_obj.set(scope, put_key.into(), put_fn.into());

    let delete_fn = v8::FunctionTemplate::new(scope, cache_delete_callback)
        .get_function(scope)
        .unwrap();
    let delete_key = v8::String::new(scope, "delete").unwrap();
    cache_obj.set(scope, delete_key.into(), delete_fn.into());

    let keys_fn = v8::FunctionTemplate::new(scope, cache_keys_callback)
        .get_function(scope)
        .unwrap();
    let keys_key = v8::String::new(scope, "keys").unwrap();
    cache_obj.set(scope, keys_key.into(), keys_fn.into());

    attach_cache_fetch_helpers(scope, cache_obj);
    cache_obj
}

/// Install global `caches` (CacheStorage singleton).
pub fn setup_cache_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    _context: &v8::Local<v8::Context>,
    global: v8::Local<v8::Object>,
) -> Result<()> {
    let cache_storage_obj = v8::Object::new(scope);

    let open_fn = v8::FunctionTemplate::new(scope, cache_storage_open_callback);
    let open_key = v8::String::new(scope, "open").unwrap();
    cache_storage_obj.set(
        scope,
        open_key.into(),
        open_fn.get_function(scope).unwrap().into(),
    );

    let keys_fn = v8::FunctionTemplate::new(scope, cache_storage_keys_callback);
    let keys_key = v8::String::new(scope, "keys").unwrap();
    cache_storage_obj.set(
        scope,
        keys_key.into(),
        keys_fn.get_function(scope).unwrap().into(),
    );

    let has_fn = v8::FunctionTemplate::new(scope, cache_storage_has_callback);
    let has_key = v8::String::new(scope, "has").unwrap();
    cache_storage_obj.set(
        scope,
        has_key.into(),
        has_fn.get_function(scope).unwrap().into(),
    );

    let delete_fn = v8::FunctionTemplate::new(scope, cache_storage_delete_callback);
    let delete_key = v8::String::new(scope, "delete").unwrap();
    cache_storage_obj.set(
        scope,
        delete_key.into(),
        delete_fn.get_function(scope).unwrap().into(),
    );

    let cache_storage_key = v8::String::new(scope, "caches").unwrap();
    global.set(scope, cache_storage_key.into(), cache_storage_obj.into());
    Ok(())
}

fn cache_storage_open_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(resolver) = promise_resolver(scope, &mut rv) else {
        return;
    };
    if args.length() < 1 {
        reject_type_error(scope, resolver, "caches.open requires a cache name");
        return;
    }
    let name = js_to_string(scope, args.get(0));
    if name.is_empty() {
        reject_type_error(scope, resolver, "caches.open requires a cache name");
        return;
    }

    {
        let mut store = backend().lock().unwrap();
        if !store.caches.contains_key(&name) {
            store.names.push(name.clone());
            store.caches.insert(name.clone(), NamedCache::default());
        }
    }

    let cache_obj = create_cache_object(scope, &name);
    resolve_value(scope, resolver, cache_obj.into());
}

fn cache_storage_keys_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(resolver) = promise_resolver(scope, &mut rv) else {
        return;
    };
    let names = backend().lock().unwrap().names.clone();
    let array = v8::Array::new(scope, names.len() as i32);
    for (index, name) in names.iter().enumerate() {
        let value = v8::String::new(scope, name).unwrap().into();
        array.set_index(scope, index as u32, value);
    }
    resolve_value(scope, resolver, array.into());
}

fn cache_storage_has_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(resolver) = promise_resolver(scope, &mut rv) else {
        return;
    };
    let name = js_to_string(scope, args.get(0));
    let found = backend().lock().unwrap().caches.contains_key(&name);
    resolve_value(scope, resolver, v8::Boolean::new(scope, found).into());
}

fn cache_storage_delete_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(resolver) = promise_resolver(scope, &mut rv) else {
        return;
    };
    let name = js_to_string(scope, args.get(0));
    let mut store = backend().lock().unwrap();
    let deleted = store.caches.remove(&name).is_some();
    if deleted {
        store.names.retain(|existing| existing != &name);
    }
    resolve_value(scope, resolver, v8::Boolean::new(scope, deleted).into());
}

fn cache_match_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(resolver) = promise_resolver(scope, &mut rv) else {
        return;
    };
    let Some(name) = cache_name_from_this(scope, args.this()) else {
        reject_type_error(scope, resolver, "Cache.match called on an invalid Cache");
        return;
    };
    let (url, method) = match parse_request(scope, args.get(0)) {
        Ok(parsed) => parsed,
        Err(message) => {
            reject_type_error(scope, resolver, &message);
            return;
        }
    };
    let matched = {
        let store = backend().lock().unwrap();
        store.caches.get(&name).and_then(|cache| {
            cache
                .entries
                .iter()
                .find(|(request, _)| request_matches(request, &url, &method))
                .map(|(_, response)| response.clone())
        })
    };
    match matched {
        Some(response) => {
            let value = reconstruct_response(scope, &response);
            resolve_value(scope, resolver, value);
        }
        None => resolve_value(scope, resolver, v8::undefined(scope).into()),
    }
}

fn cache_put_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(resolver) = promise_resolver(scope, &mut rv) else {
        return;
    };
    let Some(name) = cache_name_from_this(scope, args.this()) else {
        reject_type_error(scope, resolver, "Cache.put called on an invalid Cache");
        return;
    };
    let (url, method) = match parse_request(scope, args.get(0)) {
        Ok(parsed) => parsed,
        Err(message) => {
            reject_type_error(scope, resolver, &message);
            return;
        }
    };
    if !method.eq_ignore_ascii_case("GET") && !method.eq_ignore_ascii_case("HEAD") {
        reject_type_error(
            scope,
            resolver,
            "Cache.put only stores GET or HEAD requests",
        );
        return;
    }
    let response = match snapshot_response(scope, args.get(1), &url) {
        Ok(response) => response,
        Err(message) => {
            reject_type_error(scope, resolver, &message);
            return;
        }
    };
    {
        let mut store = backend().lock().unwrap();
        if !store.caches.contains_key(&name) {
            store.names.push(name.clone());
            store.caches.insert(name.clone(), NamedCache::default());
        }
        let cache = store.caches.get_mut(&name).unwrap();
        cache
            .entries
            .retain(|(request, _)| !request_matches(request, &url, &method));
        cache.entries.push((
            CachedRequest {
                url,
                method: method.to_ascii_uppercase(),
            },
            response,
        ));
    }
    resolve_value(scope, resolver, v8::undefined(scope).into());
}

fn cache_delete_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(resolver) = promise_resolver(scope, &mut rv) else {
        return;
    };
    let Some(name) = cache_name_from_this(scope, args.this()) else {
        reject_type_error(scope, resolver, "Cache.delete called on an invalid Cache");
        return;
    };
    let (url, method) = match parse_request(scope, args.get(0)) {
        Ok(parsed) => parsed,
        Err(message) => {
            reject_type_error(scope, resolver, &message);
            return;
        }
    };
    let deleted = {
        let mut store = backend().lock().unwrap();
        match store.caches.get_mut(&name) {
            Some(cache) => {
                let before = cache.entries.len();
                cache
                    .entries
                    .retain(|(request, _)| !request_matches(request, &url, &method));
                cache.entries.len() != before
            }
            None => false,
        }
    };
    resolve_value(scope, resolver, v8::Boolean::new(scope, deleted).into());
}

fn cache_keys_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let Some(resolver) = promise_resolver(scope, &mut rv) else {
        return;
    };
    let Some(name) = cache_name_from_this(scope, args.this()) else {
        reject_type_error(scope, resolver, "Cache.keys called on an invalid Cache");
        return;
    };
    let requests = backend()
        .lock()
        .unwrap()
        .caches
        .get(&name)
        .map(|cache| {
            cache
                .entries
                .iter()
                .map(|(request, _)| request.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let array = v8::Array::new(scope, requests.len() as i32);
    for (index, request) in requests.iter().enumerate() {
        let value = reconstruct_request(scope, request);
        array.set_index(scope, index as u32, value);
    }
    resolve_value(scope, resolver, array.into());
}
