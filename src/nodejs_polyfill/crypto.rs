/// crypto polyfill
use rusty_v8 as v8;
pub fn register(scope: &mut v8::PinScope, global: &v8::Local<v8::Object>) {
    let crypto_key: _ = v8::String::new(scope, "crypto").unwrap();
    let crypto_obj: _ = v8::Object::new(scope);
    // Random bytes
    let random_fn: _ = v8::FunctionTemplate::new(scope, random_bytes).get_function(scope).unwrap();
    let random_key: _ = v8::String::new(scope, "randomBytes").unwrap().into();
    crypto_obj.set(scope, random_key, random_fn.into());
    global.set(scope, crypto_key.into(), crypto_obj.into());
}
fn random_bytes(scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue) {
    let size: _ = args.get(0).int32_value(scope).unwrap_or(0) as usize;
    // Use a simple random generator
    use rand::Rng;
use std::collections::{HashMap, BTreeMap};
    let mut rng = rand::thread_rng();
    // Generate random data directly into a string representation for now
    let mut random_string = String::new();
    for _ in 0..size {
        random_string.push(rng.gen::<u8>() as char);
    }
    // Return as string for simplicity (not ideal but works)
    retval.set(v8::String::new(scope, &random_string).unwrap().into());
}