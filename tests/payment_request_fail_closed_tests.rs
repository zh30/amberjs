use serial_test::serial;

fn run_with_web_api(source: &str) -> String {
    amberjs::initialize_v8().expect("V8 should initialize");

    let mut isolate = rusty_v8::Isolate::new(Default::default());
    isolate.set_microtasks_policy(rusty_v8::MicrotasksPolicy::Explicit);

    rusty_v8::scope!(let scope, &mut isolate);
    let context = rusty_v8::Context::new(scope, Default::default());
    let scope = &mut rusty_v8::ContextScope::new(scope, context);

    amberjs::web_api::init_web_api(scope, &context).expect("web APIs should initialize");

    let source = rusty_v8::String::new(scope, source).unwrap();
    let script = rusty_v8::Script::compile(scope, source, None).unwrap();
    script.run(scope).unwrap();
    scope.perform_microtask_checkpoint();

    let global = context.global(scope);
    let result_key = rusty_v8::String::new(scope, "__paymentTestResult").unwrap();
    global
        .get(scope, result_key.into())
        .unwrap()
        .to_rust_string_lossy(scope)
}

#[test]
#[serial]
fn payment_request_show_rejects_instead_of_mock_success() {
    let result = run_with_web_api(
        r#"
        globalThis.__paymentTestResult = 'pending';
        const request = new PaymentRequest(
          [{ supportedMethods: 'basic-card' }],
          { total: { label: 'Total', amount: { currency: 'USD', value: '1.00' } } }
        );
        request.show().then(
          response => {
            globalThis.__paymentTestResult =
              `resolved:${response.methodName}:${response.details && response.details.status}`;
          },
          error => {
            globalThis.__paymentTestResult =
              `rejected:${String(error && error.message ? error.message : error)}`;
          }
        );
    "#,
    );

    assert!(
        result.contains("rejected:") && result.contains("not supported"),
        "PaymentRequest.show must fail closed, not return mock success: {result}"
    );
}

#[test]
#[serial]
fn payment_request_can_make_payment_is_false_without_handler() {
    let result = run_with_web_api(
        r#"
        globalThis.__paymentTestResult = 'pending';
        const request = new PaymentRequest(
          [{ supportedMethods: 'basic-card' }],
          { total: { label: 'Total', amount: { currency: 'USD', value: '1.00' } } }
        );
        request.canMakePayment().then(
          value => {
            globalThis.__paymentTestResult = String(value);
          },
          error => {
            globalThis.__paymentTestResult =
              `rejected:${String(error && error.message ? error.message : error)}`;
          }
        );
    "#,
    );

    assert_eq!(
        result, "false",
        "PaymentRequest.canMakePayment must not advertise a fake available handler"
    );
}

#[test]
#[serial]
fn payment_response_constructor_fails_closed() {
    let result = run_with_web_api(
        r#"
        globalThis.__paymentTestResult = 'pending';
        try {
          const response = new PaymentResponse();
          globalThis.__paymentTestResult = `constructed:${response.requestId}`;
        } catch (error) {
          globalThis.__paymentTestResult =
            `threw:${String(error && error.message ? error.message : error)}`;
        }
    "#,
    );

    assert!(
        result.contains("threw:") && result.contains("not supported"),
        "PaymentResponse must not be user-constructible fake response: {result}"
    );
}

#[test]
#[serial]
fn payment_address_constructor_fails_closed() {
    let result = run_with_web_api(
        r#"
        globalThis.__paymentTestResult = 'pending';
        try {
          const address = new PaymentAddress();
          globalThis.__paymentTestResult = `constructed:${address.country}`;
        } catch (error) {
          globalThis.__paymentTestResult =
            `threw:${String(error && error.message ? error.message : error)}`;
        }
    "#,
    );

    assert!(
        result.contains("threw:") && result.contains("not supported"),
        "PaymentAddress must not be user-constructible fake address: {result}"
    );
}

#[test]
#[serial]
fn payment_request_abort_rejects_without_active_payment_ui() {
    let result = run_with_web_api(
        r#"
        globalThis.__paymentTestResult = 'pending';
        const request = new PaymentRequest(
          [{ supportedMethods: 'basic-card' }],
          { total: { label: 'Total', amount: { currency: 'USD', value: '1.00' } } }
        );
        request.abort().then(
          () => {
            globalThis.__paymentTestResult = 'resolved';
          },
          error => {
            globalThis.__paymentTestResult =
              `rejected:${String(error && error.message ? error.message : error)}`;
          }
        );
    "#,
    );

    assert!(
        result.contains("rejected:") && result.contains("not supported"),
        "PaymentRequest.abort must fail closed when no payment UI exists: {result}"
    );
}
