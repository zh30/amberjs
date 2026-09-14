/// AbortController API implementation
/// v0.3.291: Enhanced with proper signal.aborted flag and event handling
use anyhow::Result;
use rusty_v8 as v8;

pub fn setup_abort_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    // v0.3.291: Create AbortController template
    let abort_controller_template = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            // Create all strings for this instance
            let signal_key = v8::String::new(scope, "signal").unwrap();
            let aborted_key = v8::String::new(scope, "aborted").unwrap();
            let listeners_key = v8::String::new(scope, "_abortListeners").unwrap();
            let add_event_listener_key = v8::String::new(scope, "addEventListener").unwrap();
            let abort_key = v8::String::new(scope, "abort").unwrap();

            let controller_obj: v8::Local<v8::Object> = v8::Object::new(scope);

            // Create signal object
            let signal_obj: v8::Local<v8::Object> = v8::Object::new(scope);
            let listeners_array: v8::Local<v8::Array> = v8::Array::new(scope, 0);
            let false_bool: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            signal_obj.set(scope, aborted_key.into(), false_bool);
            signal_obj.set(scope, listeners_key.into(), listeners_array.into());

            // Create addEventListener for signal
            let add_listener_fn = v8::FunctionTemplate::new(
                scope,
                |_scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _rv: v8::ReturnValue| {
                    if args.length() > 1 {
                        let listener = args.get(1);
                        let signal = args.this();
                        let lkey = v8::String::new(_scope, "_abortListeners").unwrap();
                        let listeners_val = signal.get(_scope, lkey.into());

                        let listeners_array = if let Some(lv) = listeners_val {
                            if lv.is_array() {
                                v8::Local::<v8::Array>::try_from(lv).unwrap()
                            } else {
                                v8::Array::new(_scope, 0)
                            }
                        } else {
                            v8::Array::new(_scope, 0)
                        };

                        let length = listeners_array.length();
                        listeners_array.set_index(_scope, length, listener);
                        signal.set(_scope, lkey.into(), listeners_array.into());
                    }
                },
            );
            let add_listener_instance = add_listener_fn.get_function(scope).unwrap();
            signal_obj.set(
                scope,
                add_event_listener_key.into(),
                add_listener_instance.into(),
            );

            // Create abort function for controller
            let abort_fn = v8::FunctionTemplate::new(
                scope,
                |_scope: &mut v8::PinScope,
                 args: v8::FunctionCallbackArguments,
                 _rv: v8::ReturnValue| {
                    let controller_obj = args.this();

                    // Get signal object
                    let sig_key = v8::String::new(_scope, "signal").unwrap();
                    if let Some(signal_val) = controller_obj.get(_scope, sig_key.into()) {
                        if let Ok(signal_obj) = v8::Local::<v8::Object>::try_from(signal_val) {
                            // Set aborted = true
                            let aborted_key = v8::String::new(_scope, "aborted").unwrap();
                            let true_bool: v8::Local<v8::Value> =
                                v8::Boolean::new(_scope, true).into();
                            signal_obj.set(_scope, aborted_key.into(), true_bool);

                            // Set reason property (v1.5.0)
                            let reason = if args.length() > 0 && !args.get(0).is_undefined() {
                                args.get(0)
                            } else {
                                let msg =
                                    v8::String::new(_scope, "The operation was aborted").unwrap();
                                v8::Exception::error(_scope, msg)
                            };
                            let reason_key = v8::String::new(_scope, "reason").unwrap();
                            signal_obj.set(_scope, reason_key.into(), reason);

                            // Trigger listeners
                            let list_key = v8::String::new(_scope, "_abortListeners").unwrap();
                            let event_type_key = v8::String::new(_scope, "type").unwrap();
                            let abort_type = v8::String::new(_scope, "abort").unwrap();

                            if let Some(listeners_val) = signal_obj.get(_scope, list_key.into()) {
                                if let Ok(listeners_array) =
                                    v8::Local::<v8::Array>::try_from(listeners_val)
                                {
                                    let event_obj = v8::Object::new(_scope);
                                    event_obj.set(_scope, event_type_key.into(), abort_type.into());

                                    for i in 0..listeners_array.length() {
                                        if let Some(listener) = listeners_array.get_index(_scope, i)
                                        {
                                            if listener.is_function() {
                                                let listener_fn =
                                                    v8::Local::<v8::Function>::try_from(listener)
                                                        .unwrap();
                                                let _ = listener_fn.call(
                                                    _scope,
                                                    signal_obj.into(),
                                                    &[event_obj.into()],
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                },
            );
            let abort_instance = abort_fn.get_function(scope).unwrap();

            controller_obj.set(scope, signal_key.into(), signal_obj.into());
            controller_obj.set(scope, abort_key.into(), abort_instance.into());

            retval.set(controller_obj.into());
        },
    );

    let abort_controller_constructor: v8::Local<v8::Function> =
        abort_controller_template.get_function(scope).unwrap();

    // Create AbortSignal as a separate global object (v0.3.340)
    let abort_signal_obj: v8::Local<v8::Object> = v8::Object::new(scope);
    let aborted_key = v8::String::new(scope, "aborted").unwrap();
    let false_bool: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
    abort_signal_obj.set(scope, aborted_key.into(), false_bool);

    let global: v8::Local<v8::Object> = context.global(scope);
    let abort_controller_key: v8::Local<v8::String> =
        v8::String::new(scope, "AbortController").unwrap();
    let abort_controller_val: v8::Local<v8::Value> = abort_controller_constructor.into();
    global.set(
        scope,
        abort_controller_key.into(),
        abort_controller_val.into(),
    );

    // Expose AbortSignal as global (v0.3.340)
    let abort_signal_key: v8::Local<v8::String> = v8::String::new(scope, "AbortSignal").unwrap();
    global.set(scope, abort_signal_key.into(), abort_signal_obj.into());

    // Inject AbortSignal static methods: timeout, any, abort (v1.5.0)
    let helper_js = r#"
    (function() {
        if (typeof AbortSignal === 'undefined') return;

        AbortSignal.timeout = function(ms) {
            const controller = new AbortController();
            const delay = typeof ms === 'number' ? ms : 0;
            const timer = setTimeout(() => {
                const err = new Error('The operation was aborted due to timeout');
                err.name = 'TimeoutError';
                controller.abort(err);
            }, delay);
            if (timer && typeof timer.unref === 'function') timer.unref();
            return controller.signal;
        };

        AbortSignal.any = function(signals) {
            const controller = new AbortController();
            const sigList = Array.isArray(signals) ? signals : Array.from(signals || []);
            for (const s of sigList) {
                if (!s) continue;
                if (s.aborted) {
                    controller.abort(s.reason);
                    return controller.signal;
                }
                if (typeof s.addEventListener === 'function') {
                    s.addEventListener('abort', () => {
                        controller.abort(s.reason);
                    });
                }
            }
            return controller.signal;
        };

        AbortSignal.abort = function(reason) {
            const controller = new AbortController();
            controller.abort(reason);
            return controller.signal;
        };
    })();
    "#;
    if let Some(code) = v8::String::new(scope, helper_js) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }

    Ok(())
}
