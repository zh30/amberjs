// Background Sync API for the CLI isolate (`amber run`).
//
// `registration.sync.register(tag)` queues the tag and later dispatches a real
// `SyncEvent` (`type`, `tag`, `lastChance`, `waitUntil`) on this same isolate.
// Listeners: `onsync` / `self.onsync`, plus `dispatchEvent` when present.
// This is not a service-worker isolate, not Periodic Background Sync, and not
// a network-offline scheduler.

use anyhow::Result;
use rusty_v8 as v8;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// SyncEvent tag storage (queued until the matching `sync` event is dispatched).
static SYNC_TAGS: std::sync::OnceLock<Arc<Mutex<Vec<String>>>> = std::sync::OnceLock::new();
static PENDING_WAIT_UNTIL: AtomicUsize = AtomicUsize::new(0);

fn get_sync_tags() -> &'static Arc<Mutex<Vec<String>>> {
    SYNC_TAGS.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
}

pub fn has_pending_wait_until() -> bool {
    PENDING_WAIT_UNTIL.load(Ordering::SeqCst) > 0
}

pub fn reset_pending_wait_until() {
    PENDING_WAIT_UNTIL.store(0, Ordering::SeqCst);
    if let Ok(mut tags) = get_sync_tags().lock() {
        tags.clear();
    }
}

/// Setup Background Sync API in V8 context
pub fn setup_background_sync_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    setup_sync_event(scope, context, global)?;
    setup_sync_manager(scope, context, global)?;

    Ok(())
}

fn setup_sync_event(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    _context: &v8::Local<v8::Context>,
    global: v8::Local<v8::Object>,
) -> Result<()> {
    let sync_event_fn = v8::FunctionTemplate::new(scope, sync_event_constructor_callback);
    let sync_event_constructor = sync_event_fn.get_function(scope).unwrap();

    let sync_event_key = v8::String::new(scope, "SyncEvent").unwrap();
    global.set(scope, sync_event_key.into(), sync_event_constructor.into());

    Ok(())
}

fn setup_sync_manager(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    _context: &v8::Local<v8::Context>,
    global: v8::Local<v8::Object>,
) -> Result<()> {
    let sync_manager = v8::Object::new(scope);

    let register_fn = v8::FunctionTemplate::new(scope, sync_manager_register_callback);
    let register_key = v8::String::new(scope, "register").unwrap();
    let register_func = register_fn.get_function(scope).unwrap();
    sync_manager.set(scope, register_key.into(), register_func.into());

    let get_tags_fn = v8::FunctionTemplate::new(scope, sync_manager_get_tags_callback);
    let get_tags_key = v8::String::new(scope, "getTags").unwrap();
    let get_tags_func = get_tags_fn.get_function(scope).unwrap();
    sync_manager.set(scope, get_tags_key.into(), get_tags_func.into());

    let registration_key = v8::String::new(scope, "registration").unwrap();
    let registration = global
        .get(scope, registration_key.into())
        .and_then(|value| value.to_object(scope))
        .unwrap_or_else(|| v8::Object::new(scope));
    let sync_key = v8::String::new(scope, "sync").unwrap();
    registration.set(scope, sync_key.into(), sync_manager.into());
    global.set(scope, registration_key.into(), registration.into());

    Ok(())
}

fn option_string(
    scope: &mut v8::PinScope,
    options: v8::Local<v8::Value>,
    key: &str,
) -> Option<String> {
    let options_obj = options.to_object(scope)?;
    let key = v8::String::new(scope, key)?;
    let value = options_obj.get(scope, key.into())?;
    if value.is_undefined() || value.is_null() {
        return None;
    }
    Some(
        value
            .to_string(scope)
            .unwrap_or_else(|| v8::String::new(scope, "").unwrap())
            .to_rust_string_lossy(scope),
    )
}

fn option_bool(scope: &mut v8::PinScope, options: v8::Local<v8::Value>, key: &str) -> bool {
    let Some(options_obj) = options.to_object(scope) else {
        return false;
    };
    let Some(key) = v8::String::new(scope, key) else {
        return false;
    };
    options_obj
        .get(scope, key.into())
        .map(|value| value.to_boolean(scope).is_true())
        .unwrap_or(false)
}

fn create_sync_event<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    event_type: &str,
    tag: &str,
    last_chance: bool,
    is_trusted: bool,
) -> v8::Local<'a, v8::Object> {
    let event_obj = v8::Object::new(scope);

    let type_key = v8::String::new(scope, "type").unwrap();
    let type_val = v8::String::new(scope, event_type).unwrap();
    event_obj.set(scope, type_key.into(), type_val.into());

    let internal_type_key = v8::String::new(scope, "_type").unwrap();
    event_obj.set(scope, internal_type_key.into(), type_val.into());

    let tag_key = v8::String::new(scope, "tag").unwrap();
    let tag_val = v8::String::new(scope, tag).unwrap();
    event_obj.set(scope, tag_key.into(), tag_val.into());

    let last_chance_key = v8::String::new(scope, "lastChance").unwrap();
    let last_chance_val = v8::Boolean::new(scope, last_chance);
    event_obj.set(scope, last_chance_key.into(), last_chance_val.into());

    let bubbles_key = v8::String::new(scope, "bubbles").unwrap();
    event_obj.set(
        scope,
        bubbles_key.into(),
        v8::Boolean::new(scope, false).into(),
    );

    let cancelable_key = v8::String::new(scope, "cancelable").unwrap();
    event_obj.set(
        scope,
        cancelable_key.into(),
        v8::Boolean::new(scope, true).into(),
    );

    let trusted_key = v8::String::new(scope, "isTrusted").unwrap();
    event_obj.set(
        scope,
        trusted_key.into(),
        v8::Boolean::new(scope, is_trusted).into(),
    );

    let time_stamp_key = v8::String::new(scope, "timeStamp").unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as f64;
    event_obj.set(
        scope,
        time_stamp_key.into(),
        v8::Number::new(scope, now).into(),
    );

    let wait_until_fn = v8::FunctionTemplate::new(scope, sync_event_wait_until_callback);
    let wait_until_func = wait_until_fn.get_function(scope).unwrap();
    let wait_until_key = v8::String::new(scope, "waitUntil").unwrap();
    event_obj.set(scope, wait_until_key.into(), wait_until_func.into());

    event_obj
}

fn sync_event_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let event_type = if args.length() > 0 {
        args.get(0)
            .to_string(scope)
            .unwrap_or_else(|| v8::String::new(scope, "sync").unwrap())
            .to_rust_string_lossy(scope)
    } else {
        "sync".to_string()
    };

    let (tag_value, last_chance_value) = if args.length() > 1 {
        let options = args.get(1);
        (
            option_string(scope, options, "tag").unwrap_or_else(|| String::from("default-sync")),
            option_bool(scope, options, "lastChance"),
        )
    } else {
        (String::from("default-sync"), false)
    };

    let event_obj = create_sync_event(scope, &event_type, &tag_value, last_chance_value, false);
    rv.set(event_obj.into());
}

fn sync_event_wait_until_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    if args.length() > 0 {
        let promise = args.get(0);
        let resolver = v8::PromiseResolver::new(scope).unwrap();
        let undefined: v8::Local<v8::Value> = v8::undefined(scope).into();

        if promise.is_promise() {
            PENDING_WAIT_UNTIL.fetch_add(1, Ordering::SeqCst);

            let done_func = v8::FunctionTemplate::new(scope, sync_event_wait_until_done_callback)
                .get_function(scope)
                .unwrap();
            let then_key = v8::String::new(scope, "then").unwrap();
            let mut attached_handler = false;

            if let Ok(promise_obj) = v8::Local::<v8::Object>::try_from(promise) {
                if let Some(then_value) = promise_obj.get(scope, then_key.into()) {
                    if let Ok(then_func) = v8::Local::<v8::Function>::try_from(then_value) {
                        let done_value: v8::Local<v8::Value> = done_func.into();
                        let args = [done_value, done_value];
                        if then_func.call(scope, promise, &args).is_some() {
                            attached_handler = true;
                        }
                    }
                }
            }

            if !attached_handler {
                decrement_pending_wait_until();
            }

            rv.set(promise);
        } else {
            resolver.resolve(scope, undefined).unwrap();
            rv.set(resolver.into());
        }
    } else {
        let error = v8::String::new(scope, "waitUntil requires a promise").unwrap();
        let exception = v8::Exception::type_error(scope, error);
        scope.throw_exception(exception);
    }
}

fn sync_event_wait_until_done_callback(
    _scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    decrement_pending_wait_until();
}

fn decrement_pending_wait_until() {
    let _ = PENDING_WAIT_UNTIL.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
        Some(count.saturating_sub(1))
    });
}

fn enqueue_sync_tag(tag: &str) -> bool {
    let mut tags = get_sync_tags().lock().unwrap();
    if tags.iter().any(|existing| existing == tag) {
        false
    } else {
        tags.push(tag.to_string());
        true
    }
}

fn take_sync_tag(tag: &str) {
    let mut tags = get_sync_tags().lock().unwrap();
    tags.retain(|existing| existing != tag);
}

fn reject_register(
    scope: &mut v8::PinScope,
    resolver: v8::Local<v8::PromiseResolver>,
    message: &str,
) {
    let message = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::type_error(scope, message);
    resolver.reject(scope, error).unwrap();
}

fn schedule_sync_fire_microtask(scope: &mut v8::PinScope, tag: &str) {
    let resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = resolver.get_promise(scope);
    let fire_fn = v8::FunctionTemplate::new(scope, fire_queued_sync_callback)
        .get_function(scope)
        .unwrap();
    let then_key = v8::String::new(scope, "then").unwrap();
    let promise_value: v8::Local<v8::Value> = promise.into();
    if let Ok(promise_obj) = v8::Local::<v8::Object>::try_from(promise_value) {
        if let Some(then_value) = promise_obj.get(scope, then_key.into()) {
            if let Ok(then_func) = v8::Local::<v8::Function>::try_from(then_value) {
                let fire_value: v8::Local<v8::Value> = fire_fn.into();
                let _ = then_func.call(scope, promise_value, &[fire_value]);
            }
        }
    }
    let tag_val = v8::String::new(scope, tag).unwrap();
    resolver.resolve(scope, tag_val.into()).unwrap();
}

/// Fire after `register` settles and its `.then` handlers run, so `getTags()`
/// can still observe the queued tag. `setTimeout(fn, 0)` is a later timer turn
/// (after nextTick + microtasks). Fall back to a microtask if timers are absent.
fn schedule_sync_fire(scope: &mut v8::PinScope, tag: &str) {
    let context = scope.get_current_context();
    let global = context.global(scope);
    let timeout_key = v8::String::new(scope, "setTimeout").unwrap();
    let Some(timeout_value) = global.get(scope, timeout_key.into()) else {
        schedule_sync_fire_microtask(scope, tag);
        return;
    };
    let Ok(timeout_fn) = v8::Local::<v8::Function>::try_from(timeout_value) else {
        schedule_sync_fire_microtask(scope, tag);
        return;
    };

    let fire_fn = v8::FunctionTemplate::new(scope, fire_queued_sync_callback)
        .get_function(scope)
        .unwrap();
    let delay = v8::Number::new(scope, 0.0);
    let tag_val = v8::String::new(scope, tag).unwrap();
    if timeout_fn
        .call(
            scope,
            global.into(),
            &[fire_fn.into(), delay.into(), tag_val.into()],
        )
        .is_none()
    {
        schedule_sync_fire_microtask(scope, tag);
    }
}

fn call_named_function(
    scope: &mut v8::PinScope,
    receiver: v8::Local<v8::Object>,
    name: &str,
    event: v8::Local<v8::Value>,
) {
    let Some(key) = v8::String::new(scope, name) else {
        return;
    };
    let Some(value) = receiver.get(scope, key.into()) else {
        return;
    };
    let Ok(function) = v8::Local::<v8::Function>::try_from(value) else {
        return;
    };
    let _ = function.call(scope, receiver.into(), &[event]);
}

fn dispatch_sync_event(scope: &mut v8::PinScope, event: v8::Local<v8::Value>) {
    let context = scope.get_current_context();
    let global = context.global(scope);

    call_named_function(scope, global, "onsync", event);

    let self_key = v8::String::new(scope, "self").unwrap();
    if let Some(self_value) = global.get(scope, self_key.into()) {
        if let Ok(self_obj) = v8::Local::<v8::Object>::try_from(self_value) {
            if self_obj != global {
                call_named_function(scope, self_obj, "onsync", event);
            }
        }
    }

    let dispatch_key = v8::String::new(scope, "dispatchEvent").unwrap();
    if let Some(dispatch_value) = global.get(scope, dispatch_key.into()) {
        if let Ok(dispatch_fn) = v8::Local::<v8::Function>::try_from(dispatch_value) {
            let _ = dispatch_fn.call(scope, global.into(), &[event]);
        }
    }
}

fn fire_queued_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _rv: v8::ReturnValue,
) {
    let tag = args
        .get(0)
        .to_string(scope)
        .map(|value| value.to_rust_string_lossy(scope))
        .unwrap_or_default();
    if tag.is_empty() {
        return;
    }

    let event = create_sync_event(scope, "sync", &tag, false, true);
    dispatch_sync_event(scope, event.into());
    take_sync_tag(&tag);
}

fn sync_manager_register_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = promise_resolver.get_promise(scope);
    rv.set(promise.into());

    if args.length() == 0 {
        reject_register(
            scope,
            promise_resolver,
            "Background Sync register requires a tag",
        );
        return;
    }

    let tag = args
        .get(0)
        .to_string(scope)
        .unwrap_or_else(|| v8::String::new(scope, "").unwrap())
        .to_rust_string_lossy(scope);
    if tag.is_empty() {
        reject_register(
            scope,
            promise_resolver,
            "Background Sync register requires a non-empty tag",
        );
        return;
    }

    let newly_queued = enqueue_sync_tag(&tag);
    let undefined: v8::Local<v8::Value> = v8::undefined(scope).into();
    promise_resolver.resolve(scope, undefined).unwrap();
    if newly_queued {
        schedule_sync_fire(scope, &tag);
    }
}

fn sync_manager_get_tags_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let sync_tags = get_sync_tags();
    let tags = sync_tags.lock().unwrap();

    let tags_array = v8::Array::new(scope, tags.len() as i32);
    for (i, tag) in tags.iter().enumerate() {
        let tag_val = v8::String::new(scope, tag).unwrap();
        tags_array.set_index(scope, i as u32, tag_val.into());
    }

    let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = promise_resolver.get_promise(scope);
    rv.set(promise.into());
    promise_resolver.resolve(scope, tags_array.into()).unwrap();
}
