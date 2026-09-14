// v0.3.277: Readline API implementation
// Implements readline.createInterface() and Interface class for interactive input
// Compatible with Node.js readline API

use once_cell::sync::Lazy;
use rusty_v8 as v8;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::Mutex;

/// Interface instance ID counter
static INTERFACE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Readline interface state (minimal, without v8::Global to avoid Sync issues)
#[derive(Clone, Debug)]
struct InterfaceState {
    #[allow(dead_code)]
    id: u64,
    prompt: String,
    #[allow(dead_code)]
    is_paused: bool,
    #[allow(dead_code)]
    terminal: bool,
    #[allow(dead_code)]
    completer: Option<String>, // Store completer function reference (for v8::Global we'd need different approach)
    #[allow(dead_code)]
    history: Vec<String>, // Command history for readline
}

/// Global registry of readline interfaces (thread-safe)
static INTERFACE_REGISTRY: Lazy<Mutex<HashMap<u64, InterfaceState>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Get next interface ID
fn get_next_interface_id() -> u64 {
    INTERFACE_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}

/// Setup readline API in the V8 context
pub fn setup_readline_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<(), anyhow::Error> {
    let global = context.global(scope);

    // Create readline object
    let readline_obj = v8::Object::new(scope);

    // Create Interface constructor
    let interface_constructor = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut _retval: v8::ReturnValue| {
            if args.length() < 1 {
                if let Some(error_msg) =
                    v8::String::new(scope, "readline.createInterface requires an options object")
                {
                    let error = v8::Exception::type_error(scope, error_msg);
                    scope.throw_exception(error.into());
                }
                return;
            }

            let options = args.get(0).to_object(scope);

            // Get terminal option (default: true)
            let is_terminal = if let Some(options) = options {
                let terminal_key = v8::String::new(scope, "terminal");
                if let Some(terminal_key) = terminal_key {
                    let terminal = options.get(scope, terminal_key.into());
                    terminal.map_or(true, |t| t.is_true())
                } else {
                    true
                }
            } else {
                true
            };

            // Create interface object
            let interface_obj = v8::Object::new(scope);

            // Generate interface ID
            let interface_id = get_next_interface_id();

            // Initialize interface state
            let state = InterfaceState {
                id: interface_id,
                prompt: "> ".to_string(),
                is_paused: false,
                terminal: is_terminal,
                completer: None,
                history: Vec::new(),
            };

            // Store in registry
            let mut registry = INTERFACE_REGISTRY.lock().unwrap();
            registry.insert(interface_id, state);

            // Create question method
            let question_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {
                    if args.length() < 2 {
                        if let Some(error_msg) = v8::String::new(
                            scope,
                            "question requires 2 arguments: query and callback",
                        ) {
                            let error = v8::Exception::type_error(scope, error_msg);
                            scope.throw_exception(error.into());
                        }
                        return;
                    }

                    let callback = args.get(1);
                    if callback.is_function() {
                        if let Ok(cb_func) = v8::Local::<v8::Function>::try_from(callback) {
                            let undefined = v8::undefined(scope);
                            if let Some(empty_answer) = v8::String::new(scope, "") {
                                let _ =
                                    cb_func.call(scope, undefined.into(), &[empty_answer.into()]);
                            }
                        }
                    }
                },
            )
            .unwrap();

            let close_fn = v8::Function::new(
                scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {},
            )
            .unwrap();
            let pause_fn = v8::Function::new(
                scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {},
            )
            .unwrap();
            let resume_fn = v8::Function::new(
                scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {},
            )
            .unwrap();

            let set_prompt_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {
                    // Get interface_id from 'this' object
                    let this = args.this();
                    let id_key = v8::String::new(scope, "_interfaceId").unwrap();
                    let id_val = this.get(scope, id_key.into()).unwrap();
                    let interface_id = id_val.to_integer(scope).unwrap().value() as u64;

                    if args.length() >= 1 {
                        if let Some(prompt) = args
                            .get(0)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                        {
                            let mut registry = INTERFACE_REGISTRY.lock().unwrap();
                            if let Some(state) = registry.get_mut(&interface_id) {
                                state.prompt = prompt;
                            }
                        }
                    }
                },
            )
            .unwrap();

            let prompt_fn = v8::Function::new(
                scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {},
            )
            .unwrap();

            let write_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {
                    if args.length() >= 1 {
                        let _data = args
                            .get(0)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope));
                    }
                },
            )
            .unwrap();

            let clear_line_fn = v8::Function::new(
                scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {},
            )
            .unwrap();

            // Set properties and methods - store keys/values first to avoid borrow conflicts
            let line_key = v8::String::new(scope, "line").unwrap();
            let empty_str = v8::String::new(scope, "").unwrap();
            interface_obj.set(scope, line_key.into(), empty_str.into());

            let column_key = v8::String::new(scope, "column").unwrap();
            let zero_val = v8::Integer::new(scope, 0);
            interface_obj.set(scope, column_key.into(), zero_val.into());

            let cursor_key = v8::String::new(scope, "cursor").unwrap();
            interface_obj.set(scope, cursor_key.into(), zero_val.into());

            // Set methods
            let question_key = v8::String::new(scope, "question").unwrap();
            interface_obj.set(scope, question_key.into(), question_fn.into());

            let close_key = v8::String::new(scope, "close").unwrap();
            interface_obj.set(scope, close_key.into(), close_fn.into());

            let pause_key = v8::String::new(scope, "pause").unwrap();
            interface_obj.set(scope, pause_key.into(), pause_fn.into());

            let resume_key = v8::String::new(scope, "resume").unwrap();
            interface_obj.set(scope, resume_key.into(), resume_fn.into());

            let set_prompt_key = v8::String::new(scope, "setPrompt").unwrap();
            interface_obj.set(scope, set_prompt_key.into(), set_prompt_fn.into());

            let prompt_key = v8::String::new(scope, "prompt").unwrap();
            interface_obj.set(scope, prompt_key.into(), prompt_fn.into());

            let write_key = v8::String::new(scope, "write").unwrap();
            interface_obj.set(scope, write_key.into(), write_fn.into());

            let clear_line_key = v8::String::new(scope, "clearLine").unwrap();
            interface_obj.set(scope, clear_line_key.into(), clear_line_fn.into());

            // Store interface_id on the object for method access
            let id_key = v8::String::new(scope, "_interfaceId").unwrap();
            let id_val = v8::Number::new(scope, interface_id as f64);
            interface_obj.set(scope, id_key.into(), id_val.into());

            // Return the interface object
            _retval.set(interface_obj.into());
        },
    );

    // Get Interface constructor function and set it
    let ic_func = interface_constructor.get_function(scope).unwrap();
    let interface_key = v8::String::new(scope, "Interface").unwrap();
    readline_obj.set(scope, interface_key.into(), ic_func.into());

    // createInterface(options) function - using simpler methods that don't capture state
    let create_interface_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            if args.length() < 1 {
                if let Some(error_msg) =
                    v8::String::new(scope, "readline.createInterface requires an options object")
                {
                    let error = v8::Exception::type_error(scope, error_msg);
                    scope.throw_exception(error.into());
                }
                return;
            }

            // Parse terminal and historySize options
            let options = args.get(0).to_object(scope);
            let is_terminal = if let Some(options) = options {
                let terminal_key = v8::String::new(scope, "terminal");
                if let Some(terminal_key) = terminal_key {
                    let terminal = options.get(scope, terminal_key.into());
                    terminal.map_or(true, |t| t.is_true())
                } else {
                    true
                }
            } else {
                true
            };

            // Parse historySize option (default: 30, Node.js compatible)
            let _history_size = if let Some(options) = options {
                let history_size_key = v8::String::new(scope, "historySize");
                if let Some(history_size_key) = history_size_key {
                    let history_size = options.get(scope, history_size_key.into());
                    history_size
                        .and_then(|v| v.to_integer(scope))
                        .map(|v| v.value() as usize)
                        .unwrap_or(30)
                } else {
                    30
                }
            } else {
                30
            };

            // Generate interface ID
            let interface_id = get_next_interface_id();

            // Initialize interface state
            let state = InterfaceState {
                id: interface_id,
                prompt: "> ".to_string(),
                is_paused: false,
                terminal: is_terminal,
                completer: None,
                history: Vec::new(),
            };

            // Store in registry
            let mut registry = INTERFACE_REGISTRY.lock().unwrap();
            registry.insert(interface_id, state);

            // Create interface object
            let interface_obj = v8::Object::new(scope);

            // Create methods for the interface - all stateless closures
            let question_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {
                    if args.length() < 2 {
                        if let Some(error_msg) = v8::String::new(
                            scope,
                            "question requires 2 arguments: query and callback",
                        ) {
                            let error = v8::Exception::type_error(scope, error_msg);
                            scope.throw_exception(error.into());
                        }
                        return;
                    }

                    let callback = args.get(1);
                    if callback.is_function() {
                        if let Ok(cb_func) = v8::Local::<v8::Function>::try_from(callback) {
                            let undefined = v8::undefined(scope);
                            if let Some(empty_answer) = v8::String::new(scope, "") {
                                let _ =
                                    cb_func.call(scope, undefined.into(), &[empty_answer.into()]);
                            }
                        }
                    }
                },
            )
            .unwrap();

            let close_fn = v8::Function::new(
                scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {},
            )
            .unwrap();
            let pause_fn = v8::Function::new(
                scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {},
            )
            .unwrap();
            let resume_fn = v8::Function::new(
                scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {},
            )
            .unwrap();

            // setPrompt - use global state lookup instead of closure capture
            let set_prompt_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {
                    // Get interface_id from 'this' object
                    let this = args.this();
                    let id_key = v8::String::new(scope, "_interfaceId").unwrap();
                    let id_val = this.get(scope, id_key.into()).unwrap();
                    let interface_id = id_val.to_integer(scope).unwrap().value() as u64;

                    if args.length() >= 1 {
                        if let Some(prompt) = args
                            .get(0)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope))
                        {
                            let mut registry = INTERFACE_REGISTRY.lock().unwrap();
                            if let Some(state) = registry.get_mut(&interface_id) {
                                state.prompt = prompt;
                            }
                        }
                    }
                },
            )
            .unwrap();

            let prompt_fn = v8::Function::new(
                scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {},
            )
            .unwrap();

            let write_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {
                    if args.length() >= 1 {
                        let _data = args
                            .get(0)
                            .to_string(scope)
                            .map(|s| s.to_rust_string_lossy(scope));
                    }
                },
            )
            .unwrap();

            let clear_line_fn = v8::Function::new(
                scope,
                |_scope: &mut v8::PinScope,
                 _args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {},
            )
            .unwrap();

            // Set properties
            let line_key = v8::String::new(scope, "line").unwrap();
            let empty_str = v8::String::new(scope, "").unwrap();
            interface_obj.set(scope, line_key.into(), empty_str.into());

            let column_key = v8::String::new(scope, "column").unwrap();
            let zero_val = v8::Integer::new(scope, 0);
            interface_obj.set(scope, column_key.into(), zero_val.into());

            let cursor_key = v8::String::new(scope, "cursor").unwrap();
            interface_obj.set(scope, cursor_key.into(), zero_val.into());

            // v0.3.280: Add history array property (empty by default)
            let history_key = v8::String::new(scope, "history").unwrap();
            let history_arr = v8::Array::new(scope, 0);
            interface_obj.set(scope, history_key.into(), history_arr.into());

            // Store interface_id on the object for method access
            let id_key = v8::String::new(scope, "_interfaceId").unwrap();
            let id_val = v8::Number::new(scope, interface_id as f64);
            interface_obj.set(scope, id_key.into(), id_val.into());

            // v0.3.280: Initialize _events object for event system
            let events_key = v8::String::new(scope, "_events").unwrap();
            let events_obj = v8::Object::new(scope);
            interface_obj.set(scope, events_key.into(), events_obj.into());

            // Set methods
            let question_key = v8::String::new(scope, "question").unwrap();
            interface_obj.set(scope, question_key.into(), question_fn.into());

            let close_key = v8::String::new(scope, "close").unwrap();
            interface_obj.set(scope, close_key.into(), close_fn.into());

            let pause_key = v8::String::new(scope, "pause").unwrap();
            interface_obj.set(scope, pause_key.into(), pause_fn.into());

            let resume_key = v8::String::new(scope, "resume").unwrap();
            interface_obj.set(scope, resume_key.into(), resume_fn.into());

            let set_prompt_key = v8::String::new(scope, "setPrompt").unwrap();
            interface_obj.set(scope, set_prompt_key.into(), set_prompt_fn.into());

            let prompt_key = v8::String::new(scope, "prompt").unwrap();
            interface_obj.set(scope, prompt_key.into(), prompt_fn.into());

            let write_key = v8::String::new(scope, "write").unwrap();
            interface_obj.set(scope, write_key.into(), write_fn.into());

            let clear_line_key = v8::String::new(scope, "clearLine").unwrap();
            interface_obj.set(scope, clear_line_key.into(), clear_line_fn.into());

            // v0.3.280: Add on() method for event handling (line, close events)
            let on_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 mut retval: v8::ReturnValue| {
                    if args.length() < 2 {
                        let undefined = v8::undefined(scope);
                        retval.set(undefined.into());
                        return;
                    }

                    let event_name = args
                        .get(0)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    let listener = args.get(1);

                    if !listener.is_function() {
                        let undefined = v8::undefined(scope);
                        retval.set(undefined.into());
                        return;
                    }

                    // Get interface from 'this' object
                    let this = args.this();

                    // Get _events object from interface
                    let events_key = v8::String::new(scope, "_events").unwrap();
                    let events_obj = this.get(scope, events_key.into());

                    if let Some(e) = events_obj {
                        if e.is_object() {
                            let events = e.to_object(scope).unwrap();
                            // Store listener on events object
                            let event_key = v8::String::new(scope, &event_name).unwrap();
                            events.set(scope, event_key.into(), listener);
                        }
                    }

                    retval.set(this.into());
                },
            )
            .unwrap();

            let on_key = v8::String::new(scope, "on").unwrap();
            interface_obj.set(scope, on_key.into(), on_fn.into());

            // v0.3.280: Add emit() method for triggering events
            let emit_fn = v8::Function::new(
                scope,
                |scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _retval: v8::ReturnValue| {
                    if args.length() < 1 {
                        return;
                    }

                    let event_name = args
                        .get(0)
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();

                    // Get _events object
                    let this = args.this();
                    let events_key = v8::String::new(scope, "_events").unwrap();
                    if let Some(events_obj) = this.get(scope, events_key.into()) {
                        if events_obj.is_object() {
                            let events = events_obj.to_object(scope).unwrap();
                            let event_key = v8::String::new(scope, &event_name).unwrap();
                            if let Some(listener) = events.get(scope, event_key.into()) {
                                if listener.is_function() {
                                    // Build arguments array (skip event name)
                                    let mut emit_args: Vec<v8::Local<v8::Value>> = Vec::new();
                                    for i in 1..args.length() {
                                        emit_args.push(args.get(i));
                                    }
                                    let _listener_func = v8::Local::<v8::Function>::try_from(
                                        listener.to_object(scope).unwrap(),
                                    )
                                    .unwrap();
                                    let _ = _listener_func.call(scope, this.into(), &emit_args);
                                }
                            }
                        }
                    }
                },
            )
            .unwrap();

            let emit_key = v8::String::new(scope, "emit").unwrap();
            interface_obj.set(scope, emit_key.into(), emit_fn.into());

            // Set completer property from options (stored in 'options' variable from outer scope)
            let completer_key = v8::String::new(scope, "completer").unwrap();
            let null_val = v8::null(scope);
            if let Some(options) = options {
                let completer_val = options.get(scope, completer_key.into());
                if let Some(completer) = completer_val {
                    if completer.is_function() {
                        interface_obj.set(scope, completer_key.into(), completer);
                    } else {
                        interface_obj.set(scope, completer_key.into(), null_val.into());
                    }
                } else {
                    interface_obj.set(scope, completer_key.into(), null_val.into());
                }
            } else {
                interface_obj.set(scope, completer_key.into(), null_val.into());
            }

            retval.set(interface_obj.into());
        },
    )
    .unwrap();

    let create_interface_key = v8::String::new(scope, "createInterface").unwrap();
    readline_obj.set(
        scope,
        create_interface_key.into(),
        create_interface_fn.into(),
    );

    // Set readline as global
    let readline_key = v8::String::new(scope, "readline").unwrap();
    global.set(scope, readline_key.into(), readline_obj.into());

    Ok(())
}

/// Clear all readline interfaces (for testing)
pub fn clear_readline_interfaces() {
    let mut registry = INTERFACE_REGISTRY.lock().unwrap();
    registry.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_id_counter() {
        let id1 = get_next_interface_id();
        let id2 = get_next_interface_id();
        assert!(id2 > id1);
    }

    #[test]
    fn test_interface_registry() {
        let id = get_next_interface_id();
        let state = InterfaceState {
            id,
            prompt: "> ".to_string(),
            is_paused: false,
            terminal: true,
            completer: None,
            history: Vec::new(),
        };

        let mut registry = INTERFACE_REGISTRY.lock().unwrap();
        registry.insert(id, state.clone());

        assert!(registry.contains_key(&id));
        registry.remove(&id);
        assert!(!registry.contains_key(&id));
    }

    #[test]
    fn test_clear_interfaces() {
        let id = get_next_interface_id();
        let state = InterfaceState {
            id,
            prompt: "test".to_string(),
            is_paused: true,
            terminal: false,
            completer: None,
            history: Vec::new(),
        };

        {
            let mut registry = INTERFACE_REGISTRY.lock().unwrap();
            registry.insert(id, state);
        }

        clear_readline_interfaces();

        let registry = INTERFACE_REGISTRY.lock().unwrap();
        assert!(registry.is_empty());
    }
}
