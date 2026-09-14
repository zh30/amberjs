//! Minimal N-API (napi) host for `process.dlopen`.
#![allow(non_camel_case_types)]
//!
//! Exports the C symbols a hello addon needs and invokes `napi_register_module_v1`.
//! This is not a Node ABI compatibility commitment.

use rusty_v8 as v8;
use std::cell::Cell;
use std::ffi::{c_char, c_void, CStr};
use std::os::raw::c_int;

pub type napi_env = *mut NapiEnv;
pub type napi_value = *mut v8::Global<v8::Value>;
pub type napi_callback_info = *mut NapiCallbackInfo;
pub type napi_status = c_int;
#[allow(non_camel_case_types)]
pub type napi_callback = Option<unsafe extern "C" fn(napi_env, napi_callback_info) -> napi_value>;

pub const NAPI_OK: napi_status = 0;
pub const NAPI_GENERIC_FAILURE: napi_status = 1;

pub struct NapiEnv {
    _private: u8,
}

pub struct NapiCallbackInfo {
    #[allow(dead_code)]
    env: napi_env,
}

thread_local! {
    static CURRENT_SCOPE: Cell<*mut ()> = const { Cell::new(std::ptr::null_mut()) };
    static CURRENT_ENV: Cell<napi_env> = const { Cell::new(std::ptr::null_mut()) };
}

fn set_current_scope(scope: &mut v8::PinScope) {
    let ptr = scope as *mut v8::PinScope as *mut ();
    CURRENT_SCOPE.with(|cell| cell.set(ptr));
}

fn clear_current_scope() {
    CURRENT_SCOPE.with(|cell| cell.set(std::ptr::null_mut()));
}

fn with_scope<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut v8::PinScope) -> R,
{
    let ptr = CURRENT_SCOPE.with(|cell| cell.get());
    if ptr.is_null() {
        return None;
    }
    Some(f(unsafe { &mut *(ptr as *mut v8::PinScope) }))
}

fn box_value(scope: &mut v8::PinScope, local: v8::Local<v8::Value>) -> napi_value {
    Box::into_raw(Box::new(v8::Global::new(scope, local)))
}

fn unbox_value<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: napi_value,
) -> Option<v8::Local<'s, v8::Value>> {
    if value.is_null() {
        return None;
    }
    let global = unsafe { &*value };
    Some(v8::Local::new(scope, global))
}

fn retain_napi_c_abi() {
    let fns: &[*const ()] = &[
        napi_create_string_utf8 as *const (),
        napi_create_function as *const (),
        napi_set_named_property as *const (),
        napi_get_undefined as *const (),
        napi_get_cb_info as *const (),
        napi_create_object as *const (),
    ];
    std::hint::black_box(fns);
}

/// Load `napi_register_module_v1` from `filename` and assign the result to `module.exports`.
pub fn load_napi_addon(scope: &mut v8::PinScope, module: v8::Local<v8::Object>, filename: &str) {
    retain_napi_c_abi();
    set_current_scope(scope);
    let env = Box::into_raw(Box::new(NapiEnv { _private: 0 }));
    CURRENT_ENV.with(|cell| cell.set(env));

    let exports_obj = v8::Object::new(scope);
    let exports = box_value(scope, exports_obj.into());

    let register = match resolve_register(filename) {
        Ok(ptr) => ptr,
        Err(msg) => {
            clear_current_scope();
            let s = v8::String::new(scope, &msg).unwrap();
            let err = v8::Exception::error(scope, s);
            scope.throw_exception(err);
            return;
        }
    };

    let result = unsafe { register(env, exports) };
    let result_local = unbox_value(scope, result).unwrap_or(exports_obj.into());

    let exports_key = v8::String::new(scope, "exports").unwrap();
    let _ = module.set(scope, exports_key.into(), result_local);

    clear_current_scope();
}

type RegisterFn = unsafe extern "C" fn(napi_env, napi_value) -> napi_value;

fn resolve_register(filename: &str) -> Result<RegisterFn, String> {
    #[cfg(unix)]
    unsafe {
        use std::ffi::CString;
        let c_path = CString::new(filename).map_err(|e| e.to_string())?;
        let handle = libc::dlopen(c_path.as_ptr(), libc::RTLD_LAZY | libc::RTLD_GLOBAL);
        if handle.is_null() {
            let err_ptr = libc::dlerror();
            let err_msg = if !err_ptr.is_null() {
                CStr::from_ptr(err_ptr).to_string_lossy().into_owned()
            } else {
                "unknown dlopen error".to_string()
            };
            return Err(format!(
                "Failed to load native module '{}': {}",
                filename, err_msg
            ));
        }
        let napi_sym = CString::new("napi_register_module_v1").unwrap();
        let ptr = libc::dlsym(handle, napi_sym.as_ptr());
        if ptr.is_null() {
            libc::dlclose(handle);
            return Err(format!(
                "Native module '{}' does not export napi_register_module_v1",
                filename
            ));
        }
        Ok(std::mem::transmute(ptr))
    }

    #[cfg(windows)]
    unsafe {
        use std::ffi::CString;
        use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryA};
        let c_path = CString::new(filename).map_err(|e| e.to_string())?;
        let handle = LoadLibraryA(c_path.as_ptr() as *const u8);
        // windows-sys 0.52: HMODULE is isize, not a pointer.
        if handle == 0 {
            return Err(format!("Failed to load native module '{}'", filename));
        }
        let proc = GetProcAddress(handle, b"napi_register_module_v1\0".as_ptr());
        let Some(proc) = proc else {
            return Err(format!(
                "Native module '{}' does not export napi_register_module_v1",
                filename
            ));
        };
        Ok(std::mem::transmute(proc))
    }
}

fn utf8_from_ptr_len(str_ptr: *const c_char, length: usize) -> String {
    if str_ptr.is_null() {
        return String::new();
    }
    if length == usize::MAX {
        unsafe { CStr::from_ptr(str_ptr).to_string_lossy().into_owned() }
    } else {
        let slice = unsafe { std::slice::from_raw_parts(str_ptr as *const u8, length) };
        String::from_utf8_lossy(slice).into_owned()
    }
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_string_utf8(
    _env: napi_env,
    str_ptr: *const c_char,
    length: usize,
    result: *mut napi_value,
) -> napi_status {
    let Some(()) = with_scope(|scope| {
        let text = utf8_from_ptr_len(str_ptr, length);
        let local = v8::String::new(scope, &text).unwrap();
        if !result.is_null() {
            *result = box_value(scope, local.into());
        }
    }) else {
        return NAPI_GENERIC_FAILURE;
    };
    NAPI_OK
}

#[no_mangle]
pub unsafe extern "C" fn napi_get_undefined(
    _env: napi_env,
    result: *mut napi_value,
) -> napi_status {
    let Some(()) = with_scope(|scope| {
        let undef = v8::undefined(scope);
        if !result.is_null() {
            *result = box_value(scope, undef.into());
        }
    }) else {
        return NAPI_GENERIC_FAILURE;
    };
    NAPI_OK
}

#[no_mangle]
pub unsafe extern "C" fn napi_set_named_property(
    _env: napi_env,
    object: napi_value,
    utf8name: *const c_char,
    value: napi_value,
) -> napi_status {
    let Some(ok) = with_scope(|scope| {
        let Some(obj_val) = unbox_value(scope, object) else {
            return false;
        };
        let Ok(obj) = v8::Local::<v8::Object>::try_from(obj_val) else {
            return false;
        };
        let Some(prop_val) = unbox_value(scope, value) else {
            return false;
        };
        let name = if utf8name.is_null() {
            String::new()
        } else {
            CStr::from_ptr(utf8name).to_string_lossy().into_owned()
        };
        let key = v8::String::new(scope, &name).unwrap();
        obj.set(scope, key.into(), prop_val);
        true
    }) else {
        return NAPI_GENERIC_FAILURE;
    };
    if ok {
        NAPI_OK
    } else {
        NAPI_GENERIC_FAILURE
    }
}

fn napi_function_trampoline(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
) {
    set_current_scope(scope);
    let env = CURRENT_ENV.with(|cell| cell.get());
    let callback: napi_callback = v8::Local::<v8::External>::try_from(args.data())
        .ok()
        .and_then(|ext| {
            let ptr = ext.value() as usize;
            if ptr == 0 {
                None
            } else {
                Some(unsafe { std::mem::transmute(ptr) })
            }
        });
    let info = Box::into_raw(Box::new(NapiCallbackInfo { env }));
    let ret = if let Some(callback) = callback {
        unsafe { callback(env, info) }
    } else {
        std::ptr::null_mut()
    };
    if let Some(local) = unbox_value(scope, ret) {
        rv.set(local);
    }
    clear_current_scope();
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_function(
    env: napi_env,
    utf8name: *const c_char,
    length: usize,
    cb: napi_callback,
    _data: *mut c_void,
    result: *mut napi_value,
) -> napi_status {
    let name = utf8name_to_string(utf8name, length);
    CURRENT_ENV.with(|cell| cell.set(env));
    let Some(ok) = with_scope(|scope| {
        let cb_ptr = cb.map(|f| f as usize).unwrap_or(0) as *mut c_void;
        let external = v8::External::new(scope, cb_ptr);
        let func = v8::Function::builder(napi_function_trampoline)
            .data(external.into())
            .build(scope);
        let Some(func) = func else {
            return false;
        };
        let _ = name;
        if !result.is_null() {
            *result = box_value(scope, func.into());
        }
        true
    }) else {
        return NAPI_GENERIC_FAILURE;
    };
    if ok {
        NAPI_OK
    } else {
        NAPI_GENERIC_FAILURE
    }
}

fn utf8name_to_string(ptr: *const c_char, length: usize) -> String {
    if ptr.is_null() {
        return String::new();
    }
    utf8_from_ptr_len(ptr, length)
}

#[no_mangle]
pub unsafe extern "C" fn napi_get_cb_info(
    _env: napi_env,
    _cbinfo: napi_callback_info,
    argc: *mut usize,
    _argv: *mut napi_value,
    this_arg: *mut napi_value,
    data: *mut *mut c_void,
) -> napi_status {
    if !argc.is_null() {
        *argc = 0;
    }
    if !this_arg.is_null() {
        *this_arg = std::ptr::null_mut();
    }
    if !data.is_null() {
        *data = std::ptr::null_mut();
    }
    NAPI_OK
}

#[no_mangle]
pub unsafe extern "C" fn napi_create_object(
    _env: napi_env,
    result: *mut napi_value,
) -> napi_status {
    let Some(()) = with_scope(|scope| {
        let obj = v8::Object::new(scope);
        if !result.is_null() {
            *result = box_value(scope, obj.into());
        }
    }) else {
        return NAPI_GENERIC_FAILURE;
    };
    NAPI_OK
}
