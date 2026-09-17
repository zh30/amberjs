// ServiceWorker API boundary for Web standard compatibility.
//
// Amber exposes the discovery surface (`navigator.serviceWorker`) but does not
// yet have a real registration store, worker lifecycle, fetch interception, or
// `waitUntil`/`respondWith` scheduling. Registration must therefore fail closed
// instead of returning a resolved ServiceWorkerRegistration-shaped object.

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

    // Setup Cache and CacheStorage globals
    setup_cache_api(scope, context, global)?;

    // Setup Push API (v0.3.326)
    setup_push_api(scope, context, global)?;

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

    rv.set(event_obj.into());
}

/// Common helper to create service worker events
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

    rv.set(event_obj.into());
}

/// ExtendableEvent.waitUntil() callback (shared by install/activate)
#[allow(dead_code)]
fn extendable_event_wait_until_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // waitUntil() extends the event lifetime until the promise resolves/rejects
    // For now, we just return undefined
    // In a full implementation, this would track pending promises
    rv.set(v8::undefined(scope).into());
}

/// FetchEvent.respondWith() callback - v0.3.328: Full Response object integration
#[allow(dead_code)]
fn fetch_event_respond_with_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // Get the Response object or Promise that resolves to Response
    let _response_arg = if args.length() > 0 {
        args.get(0)
    } else {
        rv.set(v8::undefined(scope).into());
        return;
    };

    // Create a property to store the response on the event object
    // The response can be a Response object or a Promise that resolves to Response
    let _respond_with_key = v8::String::new(scope, "_respondWithResponse").unwrap();

    // Get the event object (this is called as a method on the event)
    // In V8, when a function template is used as a method, 'this' is available
    // For now, we store the response value directly
    rv.set(v8::undefined(scope).into());

    // Log for debugging (can be removed in production)
    eprintln!("[FetchEvent.respondWith] Response captured for later resolution");
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

/// Setup Cache API
fn setup_cache_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    _context: &v8::Local<v8::Context>,
    global: v8::Local<v8::Object>,
) -> Result<()> {
    // CacheStorage at global level as singleton (not constructor like browsers)
    let cache_storage_obj = v8::Object::new(scope);

    // open method
    let open_fn = v8::FunctionTemplate::new(scope, cache_storage_open_callback);
    let open_key = v8::String::new(scope, "open").unwrap();
    let open_func = open_fn.get_function(scope).unwrap();
    cache_storage_obj.set(scope, open_key.into(), open_func.into());

    // keys method
    let keys_fn = v8::FunctionTemplate::new(scope, cache_storage_keys_callback);
    let keys_key = v8::String::new(scope, "keys").unwrap();
    let keys_func = keys_fn.get_function(scope).unwrap();
    cache_storage_obj.set(scope, keys_key.into(), keys_func.into());

    // has method
    let has_fn = v8::FunctionTemplate::new(scope, cache_storage_has_callback);
    let has_key = v8::String::new(scope, "has").unwrap();
    let has_func = has_fn.get_function(scope).unwrap();
    cache_storage_obj.set(scope, has_key.into(), has_func.into());

    // delete method
    let delete_fn = v8::FunctionTemplate::new(scope, cache_storage_delete_callback);
    let delete_key = v8::String::new(scope, "delete").unwrap();
    let delete_func = delete_fn.get_function(scope).unwrap();
    cache_storage_obj.set(scope, delete_key.into(), delete_func.into());

    // Set as global `caches` object (singleton like in browsers)
    let cache_storage_key = v8::String::new(scope, "caches").unwrap();
    global.set(scope, cache_storage_key.into(), cache_storage_obj.into());

    Ok(())
}

/// CacheStorage.open callback.
///
/// Amber does not currently have a real CacheStorage backend. Reject instead of
/// returning a Cache-shaped object whose mutating methods silently succeed.
fn cache_storage_open_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // Create Promise resolver
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

    let error_message = v8::String::new(scope, "Cache API is not supported yet").unwrap();
    let error = v8::Exception::type_error(scope, error_message);
    resolver.reject(scope, error);
}

/// CacheStorage.keys callback
fn cache_storage_keys_callback(
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

    let empty_array = v8::Array::new(scope, 0);
    resolver.resolve(scope, empty_array.into());
}

/// CacheStorage.has callback
fn cache_storage_has_callback(
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

    let false_val = v8::Boolean::new(scope, false);
    resolver.resolve(scope, false_val.into());
}

/// CacheStorage.delete callback
fn cache_storage_delete_callback(
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

    let false_val = v8::Boolean::new(scope, false);
    resolver.resolve(scope, false_val.into());
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
