// Background Sync API implementation for Web standard
// v0.3.327: SyncManager and SyncEvent APIs for background synchronization
// Background Sync allows background tasks to be registered and executed when network is available

use anyhow::Result;
use rusty_v8 as v8;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// SyncEvent tag storage (internal use)
static SYNC_TAGS: std::sync::OnceLock<Arc<Mutex<Vec<String>>>> = std::sync::OnceLock::new();
static PENDING_WAIT_UNTIL: AtomicUsize = AtomicUsize::new(0);

/// Get or initialize the sync tags storage
fn get_sync_tags() -> &'static Arc<Mutex<Vec<String>>> {
    SYNC_TAGS.get_or_init(|| Arc::new(Mutex::new(Vec::new())))
}

pub fn has_pending_wait_until() -> bool {
    PENDING_WAIT_UNTIL.load(Ordering::SeqCst) > 0
}

pub fn reset_pending_wait_until() {
    PENDING_WAIT_UNTIL.store(0, Ordering::SeqCst);
}

/// Setup Background Sync API in V8 context
pub fn setup_background_sync_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Setup SyncEvent constructor
    setup_sync_event(scope, context, global)?;

    // Setup SyncManager (accessible via registration.sync)
    setup_sync_manager(scope, context, global)?;

    Ok(())
}

/// Setup SyncEvent constructor
fn setup_sync_event(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    _context: &v8::Local<v8::Context>,
    global: v8::Local<v8::Object>,
) -> Result<()> {
    let sync_event_fn = v8::FunctionTemplate::new(scope, sync_event_constructor_callback);
    let sync_event_constructor = sync_event_fn.get_function(scope).unwrap();

    // Register constructor globally
    let sync_event_key = v8::String::new(scope, "SyncEvent").unwrap();
    global.set(scope, sync_event_key.into(), sync_event_constructor.into());

    Ok(())
}

/// Setup SyncManager (for registration.sync)
fn setup_sync_manager(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    _context: &v8::Local<v8::Context>,
    global: v8::Local<v8::Object>,
) -> Result<()> {
    // SyncManager is accessed via registration.sync
    // For now, we create a minimal sync manager object
    let sync_manager = v8::Object::new(scope);

    // register method - registers a sync with a tag
    let register_fn = v8::FunctionTemplate::new(scope, sync_manager_register_callback);
    let register_key = v8::String::new(scope, "register").unwrap();
    let register_func = register_fn.get_function(scope).unwrap();
    sync_manager.set(scope, register_key.into(), register_func.into());

    // getTags method - returns all registered sync tags
    let get_tags_fn = v8::FunctionTemplate::new(scope, sync_manager_get_tags_callback);
    let get_tags_key = v8::String::new(scope, "getTags").unwrap();
    let get_tags_func = get_tags_fn.get_function(scope).unwrap();
    sync_manager.set(scope, get_tags_key.into(), get_tags_func.into());

    // Store in global for registration.sync access pattern
    let registration_key = v8::String::new(scope, "registration").unwrap();
    let registration = v8::Object::new(scope);
    let sync_key = v8::String::new(scope, "sync").unwrap();
    registration.set(scope, sync_key.into(), sync_manager.into());
    global.set(scope, registration_key.into(), registration.into());

    Ok(())
}

/// SyncEvent constructor callback
fn sync_event_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let event_obj = v8::Object::new(scope);

    // Get event type from first argument (should be 'sync')
    let event_type = if args.length() > 0 {
        args.get(0)
            .to_string(scope)
            .unwrap_or_else(|| v8::String::new(scope, "sync").unwrap())
            .to_rust_string_lossy(scope)
    } else {
        "sync".to_string()
    };

    // Get options from second argument
    let tag_value = if args.length() > 1 {
        let options = args.get(1);
        if let Some(options_obj) = options.to_object(scope) {
            let tag_key = v8::String::new(scope, "tag").unwrap();
            if let Some(tag) = options_obj.get(scope, tag_key.into()) {
                tag.to_string(scope)
                    .unwrap_or_else(|| v8::String::new(scope, "").unwrap())
                    .to_rust_string_lossy(scope)
            } else {
                String::from("default-sync")
            }
        } else {
            String::from("default-sync")
        }
    } else {
        String::from("default-sync")
    };

    // Get lastChance option (default false)
    let last_chance_value = if args.length() > 1 {
        let options = args.get(1);
        if let Some(options_obj) = options.to_object(scope) {
            let lc_key = v8::String::new(scope, "lastChance").unwrap();
            if let Some(lc) = options_obj.get(scope, lc_key.into()) {
                lc.to_boolean(scope).is_true()
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };

    // Set type property
    let type_key = v8::String::new(scope, "type").unwrap();
    let type_val = v8::String::new(scope, &event_type).unwrap();
    event_obj.set(scope, type_key.into(), type_val.into());

    // Store internal type for reference
    let internal_type_key = v8::String::new(scope, "_type").unwrap();
    event_obj.set(scope, internal_type_key.into(), type_val.into());

    // Set tag property
    let tag_key = v8::String::new(scope, "tag").unwrap();
    let tag_val = v8::String::new(scope, &tag_value).unwrap();
    event_obj.set(scope, tag_key.into(), tag_val.into());

    // Set lastChance property
    let last_chance_key = v8::String::new(scope, "lastChance").unwrap();
    let last_chance_val = v8::Boolean::new(scope, last_chance_value);
    event_obj.set(scope, last_chance_key.into(), last_chance_val.into());

    // Set bubbles (false for sync events)
    let bubbles_key = v8::String::new(scope, "bubbles").unwrap();
    let bubbles_val = v8::Boolean::new(scope, false);
    event_obj.set(scope, bubbles_key.into(), bubbles_val.into());

    // Set cancelable (true for sync events)
    let cancelable_key = v8::String::new(scope, "cancelable").unwrap();
    let cancelable_val = v8::Boolean::new(scope, true);
    event_obj.set(scope, cancelable_key.into(), cancelable_val.into());

    // Set timestamp using SystemTime (milliseconds since epoch)
    let time_stamp_key = v8::String::new(scope, "timeStamp").unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as f64;
    let time_stamp_val = v8::Number::new(scope, now);
    event_obj.set(scope, time_stamp_key.into(), time_stamp_val.into());

    // Add waitUntil method directly to the event object
    let wait_until_fn = v8::FunctionTemplate::new(scope, sync_event_wait_until_callback);
    let wait_until_func = wait_until_fn.get_function(scope).unwrap();
    let wait_until_key = v8::String::new(scope, "waitUntil").unwrap();
    event_obj.set(scope, wait_until_key.into(), wait_until_func.into());

    rv.set(event_obj.into());
}

/// SyncEvent.waitUntil() callback
fn sync_event_wait_until_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // waitUntil() extends the event lifetime until the promise resolves/rejects
    // For a full implementation, this would track pending promises
    // and prevent the sync from being considered complete until all resolve

    if args.length() > 0 {
        // Get the promise from arguments
        let promise = args.get(0);

        // Create a new promise resolver that will wrap the provided promise
        let resolver = v8::PromiseResolver::new(scope).unwrap();

        // Resolve with the result of the original promise
        // For now, we just return a resolved promise to support .then() chaining
        let undefined: v8::Local<v8::Value> = v8::undefined(scope).into();

        // Check if the argument is a promise
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
            // If not a promise, create a resolved promise
            resolver.resolve(scope, undefined).unwrap();
            rv.set(resolver.into());
        }
    } else {
        // No argument provided
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

/// SyncManager.register() callback
fn sync_manager_register_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
    let promise = promise_resolver.get_promise(scope);
    rv.set(promise.into());

    let message = if args.length() == 0 {
        "Background Sync register requires a tag"
    } else {
        "Background Sync registration is not supported yet"
    };
    let message = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::error(scope, message);
    promise_resolver.reject(scope, error).unwrap();
}

/// SyncManager.getTags() callback
fn sync_manager_get_tags_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    // Get stored tags
    let sync_tags = get_sync_tags();
    let tags = sync_tags.lock().unwrap();

    // Create a JS array with the tags
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
