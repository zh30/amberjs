// Navigator API implementation for Web Standard and WinterTC ECMA-429 Section 7
// Provides standard navigator object with userAgent, hardwareConcurrency, language, and platform.

use anyhow::Result;
use rusty_v8 as v8;

pub fn setup_navigator_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    let navigator_key = v8::String::new(scope, "navigator").unwrap();
    let navigator_obj = if let Some(val) = global.get(scope, navigator_key.into()) {
        if val.is_object() {
            v8::Local::cast(val)
        } else {
            let obj = v8::Object::new(scope);
            global.set(scope, navigator_key.into(), obj.into());
            obj
        }
    } else {
        let obj = v8::Object::new(scope);
        global.set(scope, navigator_key.into(), obj.into());
        obj
    };

    // 1. userAgent conforming to ECMA-429 Section 7 & RFC 7231: product token
    let user_agent_key = v8::String::new(scope, "userAgent").unwrap();
    let version = env!("CARGO_PKG_VERSION");
    let user_agent_str = format!("Amber/{}", version);
    let user_agent_val = v8::String::new(scope, &user_agent_str).unwrap();
    navigator_obj.set(scope, user_agent_key.into(), user_agent_val.into());

    // 2. hardwareConcurrency
    let concurrency_key = v8::String::new(scope, "hardwareConcurrency").unwrap();
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let concurrency_val = v8::Integer::new(scope, cores as i32);
    navigator_obj.set(scope, concurrency_key.into(), concurrency_val.into());

    // 3. language and languages
    let language_key = v8::String::new(scope, "language").unwrap();
    let language_val = v8::String::new(scope, "en-US").unwrap();
    navigator_obj.set(scope, language_key.into(), language_val.into());

    let languages_key = v8::String::new(scope, "languages").unwrap();
    let languages_arr = v8::Array::new(scope, 2);
    let l0 = v8::String::new(scope, "en-US").unwrap();
    let l1 = v8::String::new(scope, "en").unwrap();
    languages_arr.set_index(scope, 0, l0.into());
    languages_arr.set_index(scope, 1, l1.into());
    navigator_obj.set(scope, languages_key.into(), languages_arr.into());

    // 4. platform
    let platform_key = v8::String::new(scope, "platform").unwrap();
    let platform_str = if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "MacIntel"
        } else {
            "MacIntel"
        }
    } else if cfg!(target_os = "linux") {
        if cfg!(target_arch = "aarch64") {
            "Linux aarch64"
        } else {
            "Linux x86_64"
        }
    } else if cfg!(target_os = "windows") {
        "Win32"
    } else {
        "Unknown"
    };
    let platform_val = v8::String::new(scope, platform_str).unwrap();
    navigator_obj.set(scope, platform_key.into(), platform_val.into());

    // 5. onLine
    let online_key = v8::String::new(scope, "onLine").unwrap();
    let online_val = v8::Boolean::new(scope, true);
    navigator_obj.set(scope, online_key.into(), online_val.into());

    Ok(())
}
