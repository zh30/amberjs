use serial_test::serial;

fn run_with_web_api(source: &str) -> String {
    beejs::initialize_v8().expect("V8 should initialize");

    let mut isolate = rusty_v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(rusty_v8::MicrotasksPolicy::Explicit);

    rusty_v8::scope!(let scope, &mut isolate);
    let context = rusty_v8::Context::new(scope, Default::default());
    let scope = &mut rusty_v8::ContextScope::new(scope, context);

    beejs::web_api::init_web_api(scope, &context).expect("web APIs should initialize");

    let source = rusty_v8::String::new(scope, source).unwrap();
    let script = rusty_v8::Script::compile(scope, source, None).unwrap();
    script.run(scope).unwrap();
    scope.perform_microtask_checkpoint();

    let global = context.global(scope);
    let result_key = rusty_v8::String::new(scope, "__notificationTestResult").unwrap();
    global
        .get(scope, result_key.into())
        .unwrap()
        .to_rust_string_lossy(scope)
}

#[test]
#[serial]
fn notification_request_permission_does_not_auto_grant() {
    let result = run_with_web_api(
        r#"
        globalThis.__notificationTestResult = 'pending';
        const hasStaticRequest = typeof Notification.requestPermission === 'function';
        const leakedGlobalRequest = typeof requestPermission;
        const before = Notification.permission;
        if (!hasStaticRequest) {
          globalThis.__notificationTestResult = `missing:${leakedGlobalRequest}:${before}`;
        } else {
          Notification.requestPermission().then(value => {
            globalThis.__notificationTestResult =
              `${hasStaticRequest}:${leakedGlobalRequest}:${before}:${Notification.permission}:${value}`;
          });
        }
    "#,
    );

    assert_eq!(
        result, "true:undefined:default:default:default",
        "Notification.requestPermission must not leak globally or auto-grant"
    );
}

#[test]
#[serial]
fn notification_constructor_requires_permission() {
    let result = run_with_web_api(
        r#"
        globalThis.__notificationTestResult = 'pending';
        try {
          const notification = new Notification('hello', { body: 'world' });
          globalThis.__notificationTestResult = `constructed:${notification.title}:${Notification.permission}`;
        } catch (error) {
          globalThis.__notificationTestResult =
            `threw:${String(error && error.message ? error.message : error)}:${Notification.permission}`;
        }
    "#,
    );

    assert!(
        result.starts_with("threw:")
            && result.contains("permission")
            && result.ends_with(":default"),
        "Notification constructor must fail closed until permission/backend exists: {result}"
    );
}
