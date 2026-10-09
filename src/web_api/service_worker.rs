// ServiceWorker API boundary for Web standard compatibility.
//
// `navigator.serviceWorker.register` starts the script on a WorkerHost isolate.
// install / activate run in that script, `registration.scope` is the recorded
// scope string, and `clients.claim()` publishes `navigator.serviceWorker.controller`.
// Page and worker exchange JSON `postMessage`. `event.waitUntil(promise)` on
// install/activate extends that phase until the promise settles (reject fails
// registration). CacheStorage is Preview in-process storage (see
// `cache_storage.rs`). Preview fetch intercept: an activated SW with a `fetch`
// listener can `FetchEvent.respondWith` for page `fetch()`. Push is not
// implemented. G16 Stable still does not include intercept.

use anyhow::Result;
use rusty_v8 as v8;
use std::sync::{Arc, Mutex};

// ServiceWorker state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceWorkerState {
    Parsing,    // 0: Script is being parsed
    Installing, // 1: Script is being installed
    Installed,  // 2: Installation completed, waiting for activation
    Activating, // 3: Service worker is being activated
    Activated,  // 4: Service worker is active and can handle events
    Redundant,  // 5: Service worker has been replaced
}

impl ServiceWorkerState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ServiceWorkerState::Parsing => "parsing",
            ServiceWorkerState::Installing => "installing",
            ServiceWorkerState::Installed => "installed",
            ServiceWorkerState::Activating => "activating",
            ServiceWorkerState::Activated => "activated",
            ServiceWorkerState::Redundant => "redundant",
        }
    }
}

/// ServiceWorker registration info
#[derive(Debug, Clone)]
pub struct ServiceWorkerRegistrationInfo {
    pub scope: String,
    pub script_url: String,
    pub state: ServiceWorkerState,
    pub listeners: Arc<Mutex<Vec<(String, v8::Global<v8::Function>)>>>,
}

impl ServiceWorkerRegistrationInfo {
    pub fn new(scope: String, script_url: String) -> Self {
        Self {
            scope,
            script_url,
            state: ServiceWorkerState::Parsing,
            listeners: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

/// Setup ServiceWorker API in V8 context
pub fn setup_service_worker_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Setup ServiceWorkerGlobalScope (self) - v0.3.328
    setup_service_worker_global_scope(scope, context, global)?;

    // Setup Event classes (Event, ExtendableEvent) for lifecycle events
    setup_service_worker_events(scope, context)?;

    // Setup navigator.serviceWorker
    setup_navigator_service_worker(scope, context, global)?;

    // Setup Cache and CacheStorage globals (Preview in-process backend)
    crate::web_api::cache_storage::setup_cache_api(scope, context, global)?;

    // Setup Push API (v0.3.326)
    setup_push_api(scope, context, global)?;

    install_service_worker_registration(scope)?;

    Ok(())
}

/// Replace the fail-closed `register` with a WorkerHost-backed registration.
///
/// The script runs on its own isolate. Control frames (`install`, `activate`,
/// `message`) travel through the same postMessage channel as dedicated workers.
fn install_service_worker_registration(
    scope: &mut v8::ContextScope<v8::HandleScope>,
) -> Result<()> {
    let wrapper = r#"
const { parentPort, workerData } = require('worker_threads');
const listeners = {};
globalThis.addEventListener = function(type, fn) {
  if (typeof type !== 'string' || typeof fn !== 'function') return;
  (listeners[type] = listeners[type] || []).push(fn);
};
globalThis.removeEventListener = function(type, fn) {
  const list = listeners[type];
  if (!list) return;
  listeners[type] = list.filter((cb) => cb !== fn);
};
function dispatch(type, event) {
  const list = (listeners[type] || []).slice();
  for (let i = 0; i < list.length; i++) {
    try {
      list[i].call(globalThis, event);
    } catch (err) {
      parentPort.postMessage({
        __amberSw: 'error',
        message: err && err.message ? String(err.message) : String(err)
      });
    }
  }
}
function createExtendableEvent(type) {
  const extenders = [];
  return {
    type: type,
    waitUntil(promise) {
      if (arguments.length === 0) {
        throw new TypeError('waitUntil requires a promise');
      }
      extenders.push(Promise.resolve(promise));
    },
    __amberSettle() {
      return Promise.all(extenders);
    }
  };
}
function finishLifecycle(signal, event) {
  event.__amberSettle().then(
    function() {
      parentPort.postMessage({ __amberSw: signal });
    },
    function(err) {
      parentPort.postMessage({
        __amberSw: 'error',
        message: err && err.message ? String(err.message) : String(err)
      });
    }
  );
}
globalThis.skipWaiting = function() {
  parentPort.postMessage({ __amberSw: 'skipWaiting' });
};
globalThis.clients = {
  claim() {
    parentPort.postMessage({ __amberSw: 'claim' });
  }
};
globalThis.__amber_before_user_message = function(data) {
  if (!data || typeof data !== 'object' || typeof data.__amberSw !== 'string') {
    return undefined;
  }
  if (data.__amberSw === 'install') {
    const event = createExtendableEvent('install');
    dispatch('install', event);
    finishLifecycle('installed', event);
    return { __amberConsume: true };
  }
  if (data.__amberSw === 'activate') {
    const event = createExtendableEvent('activate');
    dispatch('activate', event);
    finishLifecycle('activated', event);
    return { __amberConsume: true };
  }
  if (data.__amberSw === 'message') {
    dispatch('message', {
      type: 'message',
      data: data.data,
      source: {
        postMessage(payload) {
          parentPort.postMessage({ __amberSw: 'reply', data: payload });
        }
      }
    });
    return { __amberRewrite: true, value: data.data };
  }
  if (data.__amberSw === 'fetch') {
    const fetchId = data.id;
    let responded = false;
    let responsePromise = null;
    const headerMap = {};
    const headerPairs = Array.isArray(data.headers) ? data.headers : [];
    for (let i = 0; i < headerPairs.length; i++) {
      const pair = headerPairs[i];
      if (pair && pair.length >= 2) headerMap[String(pair[0])] = String(pair[1]);
    }
    const request = {
      url: String(data.url || ''),
      method: String(data.method || 'GET'),
      headers: headerMap
    };
    const event = {
      type: 'fetch',
      request: request,
      requestUrl: request.url,
      respondWith(promise) {
        if (arguments.length === 0) {
          throw new TypeError('respondWith requires a Response or Promise');
        }
        if (responded) {
          throw new TypeError('respondWith has already been called');
        }
        responded = true;
        responsePromise = promise;
      }
    };
    dispatch('fetch', event);
    if (!responded) {
      parentPort.postMessage({ __amberSw: 'fetchPass', id: fetchId });
      return { __amberConsume: true };
    }
    Promise.resolve(responsePromise).then(
      function(response) {
        if (!response || typeof response !== 'object') {
          parentPort.postMessage({
            __amberSw: 'fetchError',
            id: fetchId,
            message: 'respondWith must resolve to a Response'
          });
          return;
        }
        let body = '';
        try {
          if (typeof response.text === 'function') {
            const text = response.text();
            body = typeof text === 'string' ? text : String(text);
          }
        } catch (err) {
          parentPort.postMessage({
            __amberSw: 'fetchError',
            id: fetchId,
            message: err && err.message ? String(err.message) : String(err)
          });
          return;
        }
        const headers = [];
        if (response.headers && typeof response.headers.forEach === 'function') {
          response.headers.forEach(function(value, name) {
            headers.push([String(name), String(value)]);
          });
        }
        parentPort.postMessage({
          __amberSw: 'fetchResponse',
          id: fetchId,
          status: typeof response.status === 'number' ? response.status : 200,
          statusText: response.statusText != null ? String(response.statusText) : '',
          url: response.url != null ? String(response.url) : request.url,
          ok: response.ok !== false,
          type: response.type != null ? String(response.type) : 'default',
          headers: headers,
          body: body
        });
      },
      function(err) {
        parentPort.postMessage({
          __amberSw: 'fetchError',
          id: fetchId,
          message: err && err.message ? String(err.message) : String(err)
        });
      }
    );
    return { __amberConsume: true };
  }
  return undefined;
};
globalThis.__amber_worker_listening = true;
(0, eval)(workerData.source);
parentPort.postMessage({
  __amberSw: 'ready',
  hasFetch: !!(listeners.fetch && listeners.fetch.length)
});
"#;
    let wrapper_json = serde_json::to_string(wrapper).unwrap_or_else(|_| "\"\"".to_string());
    let installer = format!(
        r#"(function() {{
  const SW_WRAPPER = {wrapper_json};
  const container = (typeof navigator !== 'undefined') ? navigator.serviceWorker : null;
  if (!container || typeof __amber_spawn_worker !== 'function') return;

  const registrations = [];
  let controller = null;
  let readyResolve = null;
  const readyPromise = new Promise((resolve) => {{ readyResolve = resolve; }});
  try {{ container.ready = readyPromise; }} catch (_) {{}}
  container.onmessage = null;
  const containerListeners = {{}};
  container.addEventListener = function(type, fn) {{
    if (typeof type !== 'string' || typeof fn !== 'function') return;
    (containerListeners[type] = containerListeners[type] || []).push(fn);
  }};
  container.removeEventListener = function(type, fn) {{
    const list = containerListeners[type];
    if (!list) return;
    containerListeners[type] = list.filter((cb) => cb !== fn);
  }};
  function dispatchContainer(event) {{
    if (typeof container.onmessage === 'function') {{
      try {{ container.onmessage(event); }} catch (_) {{}}
    }}
    const list = (containerListeners.message || []).slice();
    for (let i = 0; i < list.length; i++) {{
      try {{ list[i].call(container, event); }} catch (_) {{}}
    }}
  }}
  Object.defineProperty(container, 'controller', {{
    configurable: true,
    enumerable: true,
    get() {{ return controller; }}
  }});

  let nativeFetch = null;
  const pendingFetches = {{}};
  let fetchSeq = 1;

  function rememberNativeFetch() {{
    const current = globalThis.fetch;
    if (typeof current === 'function' && !current.__amberSwIntercept) {{
      nativeFetch = current;
    }}
  }}

  function headerPairs(headers) {{
    const out = [];
    if (!headers) return out;
    if (typeof headers.forEach === 'function') {{
      headers.forEach(function(value, name) {{
        out.push([String(name), String(value)]);
      }});
      return out;
    }}
    if (typeof headers === 'object') {{
      const keys = Object.keys(headers);
      for (let i = 0; i < keys.length; i++) {{
        out.push([keys[i], String(headers[keys[i]])]);
      }}
    }}
    return out;
  }}

  function snapshotRequest(input, init) {{
    let url = '';
    let method = 'GET';
    let headers = [];
    let body = null;
    if (typeof input === 'string') {{
      url = input;
    }} else if (input && typeof input === 'object') {{
      url = input.url != null ? String(input.url) : String(input);
      if (input.method) method = String(input.method);
      headers = headerPairs(input.headers);
      if (typeof input.body === 'string') body = input.body;
    }} else if (input != null) {{
      url = String(input);
    }}
    if (init && typeof init === 'object') {{
      if (init.method) method = String(init.method);
      if (init.headers) headers = headerPairs(init.headers);
      if (typeof init.body === 'string') body = init.body;
    }}
    return {{ url: url, method: method, headers: headers, body: body }};
  }}

  function reconstructResponse(data) {{
    const init = {{
      status: typeof data.status === 'number' ? data.status : 200,
      statusText: data.statusText != null ? String(data.statusText) : '',
      headers: data.headers || []
    }};
    const response = new Response(data.body != null ? data.body : '', init);
    if (data.url) {{
      try {{ response.url = String(data.url); }} catch (_) {{}}
    }}
    if (data.type) {{
      try {{ response.type = String(data.type); }} catch (_) {{}}
    }}
    return response;
  }}

  function interceptingRegistration() {{
    for (let i = 0; i < registrations.length; i++) {{
      const rec = registrations[i];
      if (rec.__amberHasFetch && rec.active && rec.active.state === 'activated' && rec.__amberWorkerId) {{
        return rec;
      }}
    }}
    return null;
  }}

  function interceptedFetch(input, init) {{
    const target = interceptingRegistration();
    const native = nativeFetch;
    if (!target || typeof native !== 'function') {{
      if (typeof native === 'function') return native(input, init);
      throw new TypeError('fetch is not available');
    }}
    const id = fetchSeq++;
    const req = snapshotRequest(input, init);
    return new Promise(function(resolve, reject) {{
      pendingFetches[id] = {{ resolve: resolve, reject: reject, input: input, init: init }};
      __amber_worker_post(target.__amberWorkerId, JSON.stringify({{
        __amberSw: 'fetch',
        id: id,
        url: req.url,
        method: req.method,
        headers: req.headers,
        body: req.body
      }}));
    }});
  }}
  interceptedFetch.__amberSwIntercept = true;

  function applyFetchIntercept() {{
    rememberNativeFetch();
    if (interceptingRegistration()) {{
      globalThis.fetch = interceptedFetch;
    }} else if (nativeFetch) {{
      globalThis.fetch = nativeFetch;
    }}
  }}

  function settlePendingFetch(data) {{
    const pending = pendingFetches[data.id];
    if (!pending) return;
    delete pendingFetches[data.id];
    if (data.__amberSw === 'fetchPass') {{
      try {{
        pending.resolve(nativeFetch(pending.input, pending.init));
      }} catch (err) {{
        pending.reject(err);
      }}
      return;
    }}
    if (data.__amberSw === 'fetchError') {{
      pending.reject(new Error(data.message || 'ServiceWorker fetch intercept failed'));
      return;
    }}
    try {{
      pending.resolve(reconstructResponse(data));
    }} catch (err) {{
      pending.reject(err);
    }}
  }}

  function defaultScope(scriptUrl) {{
    if (typeof scriptUrl === 'string' && scriptUrl.indexOf('data:') === 0) return '/';
    const path = String(scriptUrl || '').split('?')[0];
    const slash = path.lastIndexOf('/');
    if (slash < 0) return './';
    return path.slice(0, slash + 1);
  }}

  container.register = function(scriptURL, options) {{
    if (typeof scriptURL !== 'string' || scriptURL.length === 0) {{
      throw new TypeError('ServiceWorker registration requires a script URL');
    }}
    if (options && options.type != null && options.type !== 'classic') {{
      throw new TypeError('ServiceWorker type "' + options.type + '" is not supported');
    }}
    if (options && options.scope !== undefined && typeof options.scope !== 'string') {{
      throw new TypeError('ServiceWorker scope must be a string');
    }}
    if (options && options.scope === '') {{
      throw new TypeError('ServiceWorker scope must not be empty');
    }}
    let resolved;
    try {{
      resolved = __amber_resolve_script_file(scriptURL);
    }} catch (err) {{
      return Promise.reject(err);
    }}
    const scope = (options && typeof options.scope === 'string') ? options.scope : defaultScope(resolved.url);
    const worker = {{
      scriptURL: resolved.url,
      state: 'installing'
    }};
    const registration = {{
      scope: scope,
      installing: worker,
      waiting: null,
      active: null
    }};
    let resolveReg;
    let rejectReg;
    const promise = new Promise((resolve, reject) => {{
      resolveReg = resolve;
      rejectReg = reject;
    }});
    let settled = false;
    let skipWaiting = false;
    let claimed = false;
    let id = 0;
    registration.__amberHasFetch = false;
    registration.__amberWorkerId = 0;

    function markRedundant() {{
      worker.state = 'redundant';
      if (registration.installing === worker) registration.installing = null;
      if (registration.waiting === worker) registration.waiting = null;
      if (registration.active === worker) registration.active = null;
      if (controller === worker) controller = null;
    }}

    function fail(err) {{
      if (settled) return;
      settled = true;
      markRedundant();
      registration.__amberHasFetch = false;
      try {{ __amber_worker_terminate(id); }} catch (_) {{}}
      applyFetchIntercept();
      const error = (err instanceof Error) ? err : new Error(String(err && err.message || err));
      rejectReg(error);
    }}

    function finishResolve() {{
      if (settled) return;
      settled = true;
      if (claimed) controller = worker;
      if (worker.state === 'activated' && readyResolve) {{
        const resolveReady = readyResolve;
        readyResolve = null;
        resolveReady(registration);
      }}
      resolveReg(registration);
      applyFetchIntercept();
    }}

    worker.postMessage = function(data) {{
      if (worker.state === 'redundant') {{
        throw new Error('Cannot postMessage to a redundant service worker');
      }}
      __amber_worker_post(id, JSON.stringify({{ __amberSw: 'message', data: data }}));
    }};

    registration.unregister = function() {{
      const pending = !settled;
      settled = true;
      try {{ __amber_worker_terminate(id); }} catch (_) {{}}
      markRedundant();
      registration.__amberHasFetch = false;
      applyFetchIntercept();
      if (pending) {{
        rejectReg(new Error('ServiceWorker was unregistered'));
      }}
      return Promise.resolve(true);
    }};

    id = __amber_spawn_worker(SW_WRAPPER, resolved.url, JSON.stringify({{ source: resolved.source }}));
    registration.__amberWorkerId = id;
    globalThis.__amber_workers[id] = {{
      emit(event) {{
        if (event === 'exit') fail(new Error('ServiceWorker stopped before it finished installing'));
      }},
      onmessage(event) {{
        const data = event && event.data;
        if (!data || typeof data.__amberSw !== 'string') return;
        if (data.__amberSw === 'error') {{
          fail(new Error(data.message || 'ServiceWorker script error'));
          return;
        }}
        if (data.__amberSw === 'ready') {{
          registration.__amberHasFetch = !!data.hasFetch;
          __amber_worker_post(id, JSON.stringify({{ __amberSw: 'install' }}));
          return;
        }}
        if (data.__amberSw === 'fetchResponse' || data.__amberSw === 'fetchPass' || data.__amberSw === 'fetchError') {{
          settlePendingFetch(data);
          return;
        }}
        if (data.__amberSw === 'skipWaiting') {{
          skipWaiting = true;
          return;
        }}
        if (data.__amberSw === 'claim') {{
          claimed = true;
          return;
        }}
        if (data.__amberSw === 'installed') {{
          worker.state = 'installed';
          registration.installing = null;
          registration.waiting = worker;
          if (skipWaiting) {{
            worker.state = 'activating';
            registration.waiting = null;
            registration.active = worker;
            __amber_worker_post(id, JSON.stringify({{ __amberSw: 'activate' }}));
          }} else {{
            finishResolve();
          }}
          return;
        }}
        if (data.__amberSw === 'activated') {{
          worker.state = 'activated';
          registration.installing = null;
          registration.waiting = null;
          registration.active = worker;
          finishResolve();
          applyFetchIntercept();
          return;
        }}
        if (data.__amberSw === 'reply') {{
          dispatchContainer({{ data: data.data }});
        }}
      }},
      onerror(err) {{ fail(err); }}
    }};
    registrations.push(registration);
    return promise;
  }};
}})();
"#
    );

    let source = v8::String::new(scope, &installer).unwrap();
    if let Some(script) = v8::Script::compile(scope, source, None) {
        let _ = script.run(scope);
    }
    Ok(())
}

/// Setup ServiceWorkerGlobalScope (self) - v0.3.328: Global scope support
fn setup_service_worker_global_scope(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    _context: &v8::Local<v8::Context>,
    global: v8::Local<v8::Object>,
) -> Result<()> {
    // In ServiceWorker, `self` refers to the global scope (ServiceWorkerGlobalScope)
    // This allows access to addEventListener, skipWaiting, clients, etc.
    let self_key = v8::String::new(scope, "self").unwrap();

    // Create ServiceWorkerGlobalScope object with standard properties
    let sw_scope = v8::Object::new(scope);

    // addEventListener method (for event handling)
    let add_event_listener_fn = v8::FunctionTemplate::new(scope, sw_add_event_listener_callback);
    let add_event_key = v8::String::new(scope, "addEventListener").unwrap();
    let add_event_func = add_event_listener_fn.get_function(scope).unwrap();
    sw_scope.set(scope, add_event_key.into(), add_event_func.into());

    // removeEventListener method
    let remove_event_listener_fn =
        v8::FunctionTemplate::new(scope, sw_remove_event_listener_callback);
    let remove_event_key = v8::String::new(scope, "removeEventListener").unwrap();
    let remove_event_func = remove_event_listener_fn.get_function(scope).unwrap();
    sw_scope.set(scope, remove_event_key.into(), remove_event_func.into());

    // skipWaiting method - allows the service worker to skip the waiting state
    let skip_waiting_fn = v8::FunctionTemplate::new(scope, sw_skip_waiting_callback);
    let skip_waiting_key = v8::String::new(scope, "skipWaiting").unwrap();
    let skip_waiting_func = skip_waiting_fn.get_function(scope).unwrap();
    sw_scope.set(scope, skip_waiting_key.into(), skip_waiting_func.into());

    // registration property (points to ServiceWorkerRegistration)
    let registration_key = v8::String::new(scope, "registration").unwrap();
    let undefined_val: v8::Local<v8::Value> = v8::undefined(scope).into();
    sw_scope.set(scope, registration_key.into(), undefined_val);

    // scope property - the path scope this SW controls
    let scope_prop_key = v8::String::new(scope, "scope").unwrap();
    let scope_val = v8::String::new(scope, "./").unwrap();
    sw_scope.set(scope, scope_prop_key.into(), scope_val.into());

    // Set self to point to global scope (circular reference like in browsers)
    // This allows self.addEventListener, self.skipWaiting, etc.
    global.set(scope, self_key.into(), global.into());

    Ok(())
}

/// ServiceWorkerGlobalScope.addEventListener callback
fn sw_add_event_listener_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // Placeholder - in a full implementation, this would register event listeners
    // that persist across fetch events
    rv.set(v8::undefined(scope).into());
}

/// ServiceWorkerGlobalScope.removeEventListener callback
fn sw_remove_event_listener_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    rv.set(v8::undefined(scope).into());
}

/// ServiceWorkerGlobalScope.skipWaiting callback
fn sw_skip_waiting_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // skipWaiting() makes the service worker skip the waiting state
    // and immediately activate
    rv.set(v8::undefined(scope).into());
}

/// Setup ServiceWorker lifecycle event classes
fn setup_service_worker_events(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // InstallEvent constructor
    let install_event_fn = v8::FunctionTemplate::new(scope, install_event_constructor_callback);
    let install_event_constructor = install_event_fn.get_function(scope).unwrap();

    // ActivateEvent constructor
    let activate_event_fn = v8::FunctionTemplate::new(scope, activate_event_constructor_callback);
    let activate_event_constructor = activate_event_fn.get_function(scope).unwrap();

    // FetchEvent constructor
    let fetch_event_fn = v8::FunctionTemplate::new(scope, fetch_event_constructor_callback);
    let fetch_event_constructor = fetch_event_fn.get_function(scope).unwrap();

    // Register constructors globally
    let install_event_key = v8::String::new(scope, "InstallEvent").unwrap();
    global.set(
        scope,
        install_event_key.into(),
        install_event_constructor.into(),
    );

    let activate_event_key = v8::String::new(scope, "ActivateEvent").unwrap();
    global.set(
        scope,
        activate_event_key.into(),
        activate_event_constructor.into(),
    );

    let fetch_event_key = v8::String::new(scope, "FetchEvent").unwrap();
    global.set(
        scope,
        fetch_event_key.into(),
        fetch_event_constructor.into(),
    );

    Ok(())
}

/// InstallEvent constructor
#[allow(unused_mut)]
fn install_event_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    create_service_worker_event(scope, args, "install", rv);
}

/// ActivateEvent constructor
#[allow(unused_mut)]
fn activate_event_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    create_service_worker_event(scope, args, "activate", rv);
}

/// FetchEvent constructor
fn fetch_event_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let event_obj = v8::Object::new(scope);

    let event_type = if args.length() > 0 {
        args.get(0)
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_else(|| "fetch".to_string())
    } else {
        "fetch".to_string()
    };

    // FetchEvent follows the DOM constructor shape: new FetchEvent(type, init).
    // Older Amber tests also passed a URL as the first argument, so keep that
    // fallback when no init object is provided.
    let request_url = if args.length() > 1 {
        let init = args.get(1);
        if init.is_object() {
            let init_obj = init.to_object(scope).unwrap();
            let request_url_key = v8::String::new(scope, "requestUrl").unwrap();
            init_obj
                .get(scope, request_url_key.into())
                .and_then(|value| value.to_string(scope))
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default()
        } else {
            String::new()
        }
    } else if event_type != "fetch" {
        event_type.clone()
    } else {
        String::new()
    };

    // Store internal properties - extract values first to avoid scope borrow issues
    let type_key = v8::String::new(scope, "_type").unwrap();
    let type_val = v8::String::new(scope, &event_type).unwrap();
    event_obj.set(scope, type_key.into(), type_val.into());

    let type_prop_key = v8::String::new(scope, "type").unwrap();
    event_obj.set(scope, type_prop_key.into(), type_val.into());

    let request_url_val = v8::String::new(scope, &request_url).unwrap();
    let request_url_key = v8::String::new(scope, "requestUrl").unwrap();
    event_obj.set(scope, request_url_key.into(), request_url_val.into());

    let bubbles_false = v8::Boolean::new(scope, false);
    let bubbles_key = v8::String::new(scope, "bubbles").unwrap();
    event_obj.set(scope, bubbles_key.into(), bubbles_false.into());

    let cancelable_true = v8::Boolean::new(scope, true);
    let cancelable_key = v8::String::new(scope, "cancelable").unwrap();
    event_obj.set(scope, cancelable_key.into(), cancelable_true.into());

    let respond_with_fn = v8::Function::new(scope, fetch_event_respond_with_callback).unwrap();
    let respond_with_key = v8::String::new(scope, "respondWith").unwrap();
    event_obj.set(scope, respond_with_key.into(), respond_with_fn.into());

    rv.set(event_obj.into());
}

/// Common helper to create service worker events (InstallEvent / ActivateEvent).
/// Attaches a real `waitUntil` that tracks promises the same way as the global
/// `ExtendableEvent` constructor. The WorkerHost SW path builds its own
/// ExtendableEvent-shaped object in the wrapper; these ctors are for page-side
/// `new InstallEvent` / `new ActivateEvent`.
fn create_service_worker_event(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    event_type: &str,
    mut rv: v8::ReturnValue,
) {
    let event_obj = v8::Object::new(scope);

    // Get event type from arguments (usually same as event name)
    let event_type_str = if args.length() > 0 {
        args.get(0)
            .to_string(scope)
            .unwrap_or_else(|| v8::String::new(scope, event_type).unwrap())
            .to_rust_string_lossy(scope)
    } else {
        event_type.to_string()
    };

    // Store internal properties - extract values first to avoid scope borrow issues
    let type_key = v8::String::new(scope, "_type").unwrap();
    let type_val = v8::String::new(scope, &event_type_str).unwrap();
    event_obj.set(scope, type_key.into(), type_val.into());

    let type_prop_key = v8::String::new(scope, "type").unwrap();
    event_obj.set(scope, type_prop_key.into(), type_val.into());

    let bubbles_false = v8::Boolean::new(scope, false);
    let bubbles_key = v8::String::new(scope, "bubbles").unwrap();
    event_obj.set(scope, bubbles_key.into(), bubbles_false.into());

    let cancelable_true = v8::Boolean::new(scope, true);
    let cancelable_key = v8::String::new(scope, "cancelable").unwrap();
    event_obj.set(scope, cancelable_key.into(), cancelable_true.into());

    let wait_until_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, rv: v8::ReturnValue| {
            // Reuse the global ExtendableEvent.waitUntil semantics (pending counter).
            crate::web_api::events::extendable_event_wait_until_for_sw(scope, args, rv);
        },
    )
    .unwrap();
    let wait_until_key = v8::String::new(scope, "waitUntil").unwrap();
    event_obj.set(scope, wait_until_key.into(), wait_until_fn.into());

    rv.set(event_obj.into());
}

/// Page-side `FetchEvent.respondWith`. Live intercept uses the WorkerHost
/// wrapper's respondWith; this stores the argument on constructed events.
fn fetch_event_respond_with_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    if args.length() == 0 {
        let message = v8::String::new(scope, "respondWith requires a Response or Promise").unwrap();
        let error = v8::Exception::type_error(scope, message);
        scope.throw_exception(error.into());
        return;
    }
    let this = args.this();
    let key = v8::String::new(scope, "_respondWithResponse").unwrap();
    this.set(scope, key.into(), args.get(0));
    rv.set(v8::undefined(scope).into());
}

/// FetchEvent.clientId property getter - v0.3.328: Track client origin
#[allow(dead_code)]
fn fetch_event_client_id_getter(
    scope: &mut v8::PinScope,
    _args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // Return 'unknown' for now as we don't have client tracking in this context
    let client_id = v8::String::new(scope, "unknown").unwrap();
    rv.set(client_id.into());
}

/// Setup navigator.serviceWorker
fn setup_navigator_service_worker(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    _context: &v8::Local<v8::Context>,
    global: v8::Local<v8::Object>,
) -> Result<()> {
    let service_worker_container = v8::Object::new(scope);

    // register method
    let register_fn = v8::FunctionTemplate::new(scope, service_worker_register_callback);
    let register_key = v8::String::new(scope, "register").unwrap();
    let register_func = register_fn.get_function(scope).unwrap();
    service_worker_container.set(scope, register_key.into(), register_func.into());

    // ready property - use undefined for now (no active worker)
    let undefined: v8::Local<v8::Value> = v8::undefined(scope).into();
    let ready_key = v8::String::new(scope, "ready").unwrap();
    service_worker_container.set(scope, ready_key.into(), undefined);

    // Add to navigator (create navigator if it doesn't exist)
    let navigator_key = v8::String::new(scope, "navigator").unwrap();
    let navigator = if let Some(nav) = global
        .get(scope, navigator_key.into())
        .and_then(|v| v.to_object(scope))
    {
        nav
    } else {
        // Create navigator object if it doesn't exist
        let new_navigator = v8::Object::new(scope);
        global.set(scope, navigator_key.into(), new_navigator.into());
        new_navigator
    };
    let service_worker_key = v8::String::new(scope, "serviceWorker").unwrap();
    navigator.set(
        scope,
        service_worker_key.into(),
        service_worker_container.into(),
    );

    Ok(())
}

/// ServiceWorker registration callback
fn service_worker_register_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let url_val = args.get(0);
    if !url_val.is_string() {
        let error =
            v8::String::new(scope, "ServiceWorker registration requires a script URL").unwrap();
        let exception = v8::Exception::type_error(scope, error);
        scope.throw_exception(exception);
        return;
    }

    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    rv.set(promise.into());

    let error_message =
        v8::String::new(scope, "ServiceWorker registration is not supported yet").unwrap();
    let error = v8::Exception::type_error(scope, error_message);
    resolver.reject(scope, error);
}

// =====================================================
// Push API (v0.3.326)
// Provides PushManager, PushSubscription, and PushEvent
// =====================================================

/// Setup Push API - PushManager, PushSubscription, PushEvent
fn setup_push_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    _context: &v8::Local<v8::Context>,
    global: v8::Local<v8::Object>,
) -> Result<()> {
    // Setup PushEvent constructor
    let push_event_fn = v8::FunctionTemplate::new(scope, push_event_constructor_callback);
    let push_event_key = v8::String::new(scope, "PushEvent").unwrap();
    let push_event_constructor = push_event_fn.get_function(scope).unwrap();
    global.set(scope, push_event_key.into(), push_event_constructor.into());

    // Setup PushManager as a constructor function
    let push_manager_fn = v8::FunctionTemplate::new(scope, push_manager_constructor_callback);
    let push_manager_constructor = push_manager_fn.get_function(scope).unwrap();

    // Create prototype object with methods
    let push_manager_proto = v8::Object::new(scope);

    // subscribe method
    let subscribe_fn = v8::FunctionTemplate::new(scope, push_manager_subscribe_callback);
    let subscribe_key = v8::String::new(scope, "subscribe").unwrap();
    let subscribe_func = subscribe_fn.get_function(scope).unwrap();
    push_manager_proto.set(scope, subscribe_key.into(), subscribe_func.into());

    // getSubscription method
    let get_sub_fn = v8::FunctionTemplate::new(scope, push_manager_get_subscription_callback);
    let get_sub_key = v8::String::new(scope, "getSubscription").unwrap();
    let get_sub_func = get_sub_fn.get_function(scope).unwrap();
    push_manager_proto.set(scope, get_sub_key.into(), get_sub_func.into());

    // permissionState method
    let perm_state_fn = v8::FunctionTemplate::new(scope, push_manager_permission_state_callback);
    let perm_state_key = v8::String::new(scope, "permissionState").unwrap();
    let perm_state_func = perm_state_fn.get_function(scope).unwrap();
    push_manager_proto.set(scope, perm_state_key.into(), perm_state_func.into());

    // Register globally first
    let push_manager_key = v8::String::new(scope, "PushManager").unwrap();
    global.set(
        scope,
        push_manager_key.into(),
        push_manager_constructor.into(),
    );

    // Store prototype globally so JavaScript can access it
    let push_manager_proto_key = v8::String::new(scope, "pushManagerProto").unwrap();
    global.set(
        scope,
        push_manager_proto_key.into(),
        push_manager_proto.into(),
    );

    // Use JavaScript to set up the prototype chain
    // This changes the [[Prototype]] of the constructor's .prototype object
    let set_proto_js = v8::String::new(
        scope,
        "Object.setPrototypeOf(PushManager.prototype, pushManagerProto)",
    )
    .unwrap();
    if let Some(proto_script) = v8::Script::compile(scope, set_proto_js, None) {
        let _ = proto_script.run(scope);
    }

    // Setup PushSubscription as a constructor function
    let push_subscription_fn =
        v8::FunctionTemplate::new(scope, push_subscription_constructor_callback);
    let push_subscription_constructor = push_subscription_fn.get_function(scope).unwrap();

    // Create prototype object with methods
    let push_subscription_proto = v8::Object::new(scope);

    // getKey method
    let get_key_fn = v8::FunctionTemplate::new(scope, push_subscription_get_key_callback);
    let get_key_key = v8::String::new(scope, "getKey").unwrap();
    let get_key_func = get_key_fn.get_function(scope).unwrap();
    push_subscription_proto.set(scope, get_key_key.into(), get_key_func.into());

    // toJSON method
    let to_json_fn = v8::FunctionTemplate::new(scope, push_subscription_to_json_callback);
    let to_json_key = v8::String::new(scope, "toJSON").unwrap();
    let to_json_func = to_json_fn.get_function(scope).unwrap();
    push_subscription_proto.set(scope, to_json_key.into(), to_json_func.into());

    // unsubscribe method
    let unsubscribe_fn = v8::FunctionTemplate::new(scope, push_subscription_unsubscribe_callback);
    let unsubscribe_key = v8::String::new(scope, "unsubscribe").unwrap();
    let unsubscribe_func = unsubscribe_fn.get_function(scope).unwrap();
    push_subscription_proto.set(scope, unsubscribe_key.into(), unsubscribe_func.into());

    // Register globally
    let push_subscription_key = v8::String::new(scope, "PushSubscription").unwrap();
    global.set(
        scope,
        push_subscription_key.into(),
        push_subscription_constructor.into(),
    );

    // Store prototype globally so JavaScript can access it
    let push_subscription_proto_key = v8::String::new(scope, "pushSubscriptionProto").unwrap();
    global.set(
        scope,
        push_subscription_proto_key.into(),
        push_subscription_proto.into(),
    );

    // Use JavaScript to set up the prototype chain for PushSubscription
    let set_sub_proto_js = v8::String::new(
        scope,
        "Object.setPrototypeOf(PushSubscription.prototype, pushSubscriptionProto)",
    )
    .unwrap();
    if let Some(proto_script) = v8::Script::compile(scope, set_sub_proto_js, None) {
        let _ = proto_script.run(scope);
    }

    Ok(())
}

/// PushManager constructor - mainly for prototype access
fn push_manager_constructor_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // PushManager is not meant to be instantiated directly
    // It provides static methods: subscribe(), getSubscription(), permissionState()
    // Methods are set on the function template's prototype in setup_push_api
    // Return undefined since PushManager shouldn't be called with 'new'
    rv.set(v8::undefined(scope).into());
}

/// PushSubscription constructor boundary.
fn push_subscription_constructor_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let error_message =
        v8::String::new(scope, "PushSubscription construction is not supported yet").unwrap();
    let error = v8::Exception::type_error(scope, error_message);
    scope.throw_exception(error);
    rv.set(v8::undefined(scope).into());
}

/// PushSubscription.getKey() boundary.
///
/// No real PushSubscription instances exist until Amber has a push service,
/// subscription store, and key generation backend. Direct prototype calls must
/// not return fixed key material.
fn push_subscription_get_key_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let error_message = v8::String::new(scope, "PushSubscription is not supported yet").unwrap();
    let error = v8::Exception::type_error(scope, error_message);
    scope.throw_exception(error);
}

/// PushSubscription.toJSON() boundary.
fn push_subscription_to_json_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let error_message = v8::String::new(scope, "PushSubscription is not supported yet").unwrap();
    let error = v8::Exception::type_error(scope, error_message);
    scope.throw_exception(error);
}

/// PushSubscription.unsubscribe() boundary.
fn push_subscription_unsubscribe_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let resolver = match v8::PromiseResolver::new(scope) {
        Some(r) => r,
        None => {
            let error = v8::String::new(scope, "Failed to create promise resolver").unwrap();
            scope.throw_exception(error.into());
            return;
        }
    };
    let promise = resolver.get_promise(scope);
    rv.set(promise.into());

    let error_message = v8::String::new(scope, "PushSubscription is not supported yet").unwrap();
    let error = v8::Exception::type_error(scope, error_message);
    resolver.reject(scope, error);
}

/// PushManager.subscribe() - requests a push subscription
fn push_manager_subscribe_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let resolver = match v8::PromiseResolver::new(scope) {
        Some(r) => r,
        None => {
            let error = v8::String::new(scope, "Failed to create promise resolver").unwrap();
            scope.throw_exception(error.into());
            return;
        }
    };
    let promise = resolver.get_promise(scope);
    rv.set(promise.into());

    let error_message = v8::String::new(scope, "Push subscription is not supported yet").unwrap();
    let error = v8::Exception::type_error(scope, error_message);
    resolver.reject(scope, error);
}

/// PushManager.getSubscription() - returns existing subscription or null
fn push_manager_get_subscription_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // Create a promise that resolves to the subscription or null
    let resolver = match v8::PromiseResolver::new(scope) {
        Some(r) => r,
        None => {
            let error = v8::String::new(scope, "Failed to create promise resolver").unwrap();
            scope.throw_exception(error.into());
            return;
        }
    };
    let promise = resolver.get_promise(scope);
    rv.set(promise.into());

    // For demo purposes, return null (no active subscription)
    let null_val: v8::Local<v8::Value> = v8::null(scope).into();
    resolver.resolve(scope, null_val);
}

/// PushManager.permissionState() - returns the permission state
fn push_manager_permission_state_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // Create a promise that resolves to the permission state
    let resolver = match v8::PromiseResolver::new(scope) {
        Some(r) => r,
        None => {
            let error = v8::String::new(scope, "Failed to create promise resolver").unwrap();
            scope.throw_exception(error.into());
            return;
        }
    };
    let promise = resolver.get_promise(scope);
    rv.set(promise.into());

    // Return 'prompt' as default (user hasn't been asked yet)
    let prompt_str = v8::String::new(scope, "prompt").unwrap();
    resolver.resolve(scope, prompt_str.into());
}

/// PushEvent constructor - extends ExtendableEvent
fn push_event_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let event_obj = v8::Object::new(scope);

    // Get event type (usually 'push')
    let event_type = if args.length() > 0 {
        args.get(0)
            .to_string(scope)
            .unwrap_or_else(|| v8::String::new(scope, "push").unwrap())
            .to_rust_string_lossy(scope)
    } else {
        "push".to_string()
    };

    // type property
    let type_key = v8::String::new(scope, "type").unwrap();
    let type_val = v8::String::new(scope, &event_type).unwrap();
    event_obj.set(scope, type_key.into(), type_val.into());

    // bubbles: false
    let bubbles_false = v8::Boolean::new(scope, false);
    let bubbles_key = v8::String::new(scope, "bubbles").unwrap();
    event_obj.set(scope, bubbles_key.into(), bubbles_false.into());

    // cancelable: false (PushEvent is not cancelable)
    let cancelable_false = v8::Boolean::new(scope, false);
    let cancelable_key = v8::String::new(scope, "cancelable").unwrap();
    event_obj.set(scope, cancelable_key.into(), cancelable_false.into());

    // data property - can be passed in options
    let data_key = v8::String::new(scope, "data").unwrap();
    let null_val: v8::Local<v8::Value> = v8::null(scope).into();
    event_obj.set(scope, data_key.into(), null_val);

    // If data is provided in options (second argument), extract it
    if args.length() > 1 {
        let options = args.get(1);
        if let Some(options_obj) = options.to_object(scope) {
            let data_in_options = options_obj.get(scope, data_key.into());
            if let Some(data_val) = data_in_options {
                event_obj.set(scope, data_key.into(), data_val);
            }
        }
    }

    // waitUntil method (inherited from ExtendableEvent via prototype chain)
    // For now, add it directly to support basic usage
    let wait_until_fn = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut rv: v8::ReturnValue| {
            rv.set(v8::undefined(_scope).into());
        },
    );
    let wait_until_key = v8::String::new(scope, "waitUntil").unwrap();
    let wait_until_func = wait_until_fn.get_function(scope).unwrap();
    event_obj.set(scope, wait_until_key.into(), wait_until_func.into());

    rv.set(event_obj.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_worker_state_values() {
        assert_eq!(ServiceWorkerState::Parsing as u8, 0);
        assert_eq!(ServiceWorkerState::Installing as u8, 1);
        assert_eq!(ServiceWorkerState::Activated as u8, 4);
    }
}
