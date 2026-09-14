// Beejs Native Foreign Function Interface (bee:ffi)
// High-performance zero-dependency C ABI interop for JavaScript & TypeScript

use anyhow::{anyhow, Result};
use rusty_v8 as v8;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::sync::{Arc, Mutex};

/// Supported FFI primitive types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FFIType {
    Void,
    Bool,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    Ptr,
    CString,
}

impl FFIType {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "void" => Ok(FFIType::Void),
            "bool" | "boolean" => Ok(FFIType::Bool),
            "i8" | "int8" => Ok(FFIType::I8),
            "u8" | "uint8" => Ok(FFIType::U8),
            "i16" | "int16" => Ok(FFIType::I16),
            "u16" | "uint16" => Ok(FFIType::U16),
            "i32" | "int32" | "int" => Ok(FFIType::I32),
            "u32" | "uint32" | "uint" => Ok(FFIType::U32),
            "i64" | "int64" => Ok(FFIType::I64),
            "u64" | "uint64" | "usize" => Ok(FFIType::U64),
            "f32" | "float" => Ok(FFIType::F32),
            "f64" | "double" => Ok(FFIType::F64),
            "ptr" | "pointer" => Ok(FFIType::Ptr),
            "cstring" | "string" => Ok(FFIType::CString),
            _ => Err(anyhow!("Unsupported FFI type: {}", s)),
        }
    }
}

/// Function signature metadata
#[derive(Clone, Debug)]
pub struct SymbolDef {
    pub name: String,
    pub args: Vec<FFIType>,
    pub returns: FFIType,
}

/// Shared library handle
pub struct DynamicLibrary {
    #[cfg(unix)]
    handle: *mut libc::c_void,
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HMODULE,
    symbols: HashMap<String, *const ()>,
}

unsafe impl Send for DynamicLibrary {}
unsafe impl Sync for DynamicLibrary {}

impl DynamicLibrary {
    pub fn open(path: Option<&str>) -> Result<Self> {
        #[cfg(unix)]
        {
            let handle = if let Some(p) = path {
                let c_path = CString::new(p)?;
                unsafe { libc::dlopen(c_path.as_ptr(), libc::RTLD_LAZY | libc::RTLD_LOCAL) }
            } else {
                unsafe { libc::dlopen(std::ptr::null(), libc::RTLD_LAZY | libc::RTLD_LOCAL) }
            };

            if handle.is_null() {
                let err = unsafe {
                    let err_ptr = libc::dlerror();
                    if err_ptr.is_null() {
                        "Unknown dlopen error".to_string()
                    } else {
                        CStr::from_ptr(err_ptr).to_string_lossy().into_owned()
                    }
                };
                return Err(anyhow!("Failed to open library: {}", err));
            }

            Ok(Self {
                handle,
                symbols: HashMap::new(),
            })
        }

        #[cfg(windows)]
        {
            use windows_sys::Win32::System::LibraryLoader::LoadLibraryA;
            let handle = if let Some(p) = path {
                let c_path = CString::new(p)?;
                unsafe { LoadLibraryA(c_path.as_ptr() as *const u8) }
            } else {
                0
            };

            // windows-sys 0.52: HMODULE is isize, not a pointer.
            if handle == 0 {
                return Err(anyhow!("Failed to open library on Windows"));
            }

            Ok(Self {
                handle,
                symbols: HashMap::new(),
            })
        }
    }

    pub fn symbol(&mut self, name: &str) -> Result<*const ()> {
        if let Some(&ptr) = self.symbols.get(name) {
            return Ok(ptr);
        }

        #[cfg(unix)]
        {
            let c_name = CString::new(name)?;
            let sym = unsafe { libc::dlsym(self.handle, c_name.as_ptr()) };
            if sym.is_null() {
                return Err(anyhow!("Symbol '{}' not found in library", name));
            }
            let ptr = sym as *const ();
            self.symbols.insert(name.to_string(), ptr);
            Ok(ptr)
        }

        #[cfg(windows)]
        {
            use windows_sys::Win32::System::LibraryLoader::GetProcAddress;
            let c_name = CString::new(name)?;
            let sym = unsafe { GetProcAddress(self.handle, c_name.as_ptr() as *const u8) };
            if sym.is_none() {
                return Err(anyhow!("Symbol '{}' not found in library", name));
            }
            let ptr = sym.unwrap() as *const ();
            self.symbols.insert(name.to_string(), ptr);
            Ok(ptr)
        }
    }

    pub fn close(&mut self) {
        #[cfg(unix)]
        {
            if !self.handle.is_null() {
                unsafe { libc::dlclose(self.handle) };
                self.handle = std::ptr::null_mut();
            }
        }
        #[cfg(windows)]
        {
            // windows-sys 0.52: FreeLibrary is in Foundation, not LibraryLoader.
            use windows_sys::Win32::Foundation::FreeLibrary;
            if self.handle != 0 {
                unsafe { FreeLibrary(self.handle) };
                self.handle = 0;
            }
        }
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        self.close();
    }
}

// Global registry of opened libraries
type LibRegistry = Arc<Mutex<HashMap<usize, DynamicLibrary>>>;

fn get_lib_registry() -> LibRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<LibRegistry> = OnceLock::new();
    REGISTRY
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

static NEXT_LIB_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);

/// Initialize the `bee:ffi` subsystem in V8 context
pub fn setup_ffi_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Native helper for dlopen
    let dlopen_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let path_opt = if args.length() > 0 && !args.get(0).is_null_or_undefined() {
                let s = args.get(0).to_rust_string_lossy(scope);
                if s.is_empty() {
                    None
                } else {
                    Some(s)
                }
            } else {
                None
            };

            let lib = match DynamicLibrary::open(path_opt.as_deref()) {
                Ok(l) => l,
                Err(e) => {
                    let msg = v8::String::new(scope, &format!("FFI Error: {}", e)).unwrap();
                    let exc = v8::Exception::error(scope, msg);
                    scope.throw_exception(exc);
                    return;
                }
            };

            let lib_id = NEXT_LIB_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            {
                let registry = get_lib_registry();
                let mut guard = registry.lock().unwrap();
                guard.insert(lib_id, lib);
            }

            rv.set(v8::Integer::new(scope, lib_id as i32).into());
        },
    )
    .unwrap();

    // Native helper for calling FFI symbols
    let call_symbol_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            if args.length() < 4 {
                let msg =
                    v8::String::new(scope, "callSymbol requires (libId, symbol, args, returns)")
                        .unwrap();
                let exc = v8::Exception::type_error(scope, msg);
                scope.throw_exception(exc);
                return;
            }

            let lib_id = args.get(0).int32_value(scope).unwrap_or(0) as usize;
            let symbol_name = args.get(1).to_rust_string_lossy(scope);
            let returns_str = args.get(3).to_rust_string_lossy(scope);
            let returns_type = match FFIType::from_str(&returns_str) {
                Ok(t) => t,
                Err(e) => {
                    let msg = v8::String::new(scope, &e.to_string()).unwrap();
                    let exc = v8::Exception::type_error(scope, msg);
                    scope.throw_exception(exc);
                    return;
                }
            };

            let fn_ptr = {
                let registry = get_lib_registry();
                let mut guard = registry.lock().unwrap();
                let lib = match guard.get_mut(&lib_id) {
                    Some(l) => l,
                    None => {
                        let msg =
                            v8::String::new(scope, "Invalid or closed library handle").unwrap();
                        let exc = v8::Exception::error(scope, msg);
                        scope.throw_exception(exc);
                        return;
                    }
                };
                match lib.symbol(&symbol_name) {
                    Ok(p) => p,
                    Err(e) => {
                        let msg = v8::String::new(scope, &e.to_string()).unwrap();
                        let exc = v8::Exception::error(scope, msg);
                        scope.throw_exception(exc);
                        return;
                    }
                }
            };

            // Parse argument types and runtime values
            let arg_types_val = args.get(2);
            let mut arg_types = Vec::new();
            if arg_types_val.is_array() {
                if let Ok(arr) = v8::Local::<v8::Array>::try_from(arg_types_val) {
                    for i in 0..arr.length() {
                        let item = arr.get_index(scope, i).unwrap();
                        let s = item.to_rust_string_lossy(scope);
                        if let Ok(t) = FFIType::from_str(&s) {
                            arg_types.push(t);
                        }
                    }
                }
            }

            // Extract call arguments passed after the 4th parameter
            let call_args_offset = 4;
            let num_call_args = if args.length() > call_args_offset {
                args.length() - call_args_offset
            } else {
                0
            };

            // Fast path dispatch for common C ABI patterns
            match (arg_types.len(), returns_type) {
                // 0 args -> void / scalar
                (0, FFIType::Void) => {
                    let func: extern "C" fn() = unsafe { std::mem::transmute(fn_ptr) };
                    func();
                    rv.set(v8::undefined(scope).into());
                }
                (0, FFIType::I32) => {
                    let func: extern "C" fn() -> i32 = unsafe { std::mem::transmute(fn_ptr) };
                    rv.set(v8::Integer::new(scope, func()).into());
                }
                (0, FFIType::U32) => {
                    let func: extern "C" fn() -> u32 = unsafe { std::mem::transmute(fn_ptr) };
                    rv.set(v8::Integer::new_from_unsigned(scope, func()).into());
                }
                (0, FFIType::F64) => {
                    let func: extern "C" fn() -> f64 = unsafe { std::mem::transmute(fn_ptr) };
                    rv.set(v8::Number::new(scope, func()).into());
                }
                // 1 arg: (f64) -> f64 (e.g. cos, sin, sqrt, abs, ceil, floor)
                (1, FFIType::F64) if arg_types[0] == FFIType::F64 => {
                    let a0 = if num_call_args > 0 {
                        args.get(call_args_offset)
                            .number_value(scope)
                            .unwrap_or(0.0)
                    } else {
                        0.0
                    };
                    let func: extern "C" fn(f64) -> f64 = unsafe { std::mem::transmute(fn_ptr) };
                    rv.set(v8::Number::new(scope, func(a0)).into());
                }
                // 1 arg: (f32) -> f32
                (1, FFIType::F32) if arg_types[0] == FFIType::F32 => {
                    let a0 = if num_call_args > 0 {
                        args.get(call_args_offset)
                            .number_value(scope)
                            .unwrap_or(0.0) as f32
                    } else {
                        0.0
                    };
                    let func: extern "C" fn(f32) -> f32 = unsafe { std::mem::transmute(fn_ptr) };
                    rv.set(v8::Number::new(scope, func(a0) as f64).into());
                }
                // 1 arg: (i32) -> i32 (e.g. abs)
                (1, FFIType::I32) if arg_types[0] == FFIType::I32 => {
                    let a0 = if num_call_args > 0 {
                        args.get(call_args_offset).int32_value(scope).unwrap_or(0)
                    } else {
                        0
                    };
                    let func: extern "C" fn(i32) -> i32 = unsafe { std::mem::transmute(fn_ptr) };
                    rv.set(v8::Integer::new(scope, func(a0)).into());
                }
                // 1 arg: (ptr / cstring) -> usize / u64 / i32 (e.g. strlen)
                (1, FFIType::U64)
                    if arg_types[0] == FFIType::CString || arg_types[0] == FFIType::Ptr =>
                {
                    if num_call_args > 0 && args.get(call_args_offset).is_string() {
                        let rust_str = args.get(call_args_offset).to_rust_string_lossy(scope);
                        let c_str = CString::new(rust_str).unwrap_or_default();
                        let func: extern "C" fn(*const libc::c_char) -> usize =
                            unsafe { std::mem::transmute(fn_ptr) };
                        let res = func(c_str.as_ptr());
                        rv.set(v8::Number::new(scope, res as f64).into());
                    } else if num_call_args > 0 {
                        let p = args.get(call_args_offset).integer_value(scope).unwrap_or(0)
                            as *const libc::c_char;
                        let func: extern "C" fn(*const libc::c_char) -> usize =
                            unsafe { std::mem::transmute(fn_ptr) };
                        let res = func(p);
                        rv.set(v8::Number::new(scope, res as f64).into());
                    } else {
                        rv.set(v8::Integer::new(scope, 0).into());
                    }
                }
                // 1 arg: (ptr) -> void
                (1, FFIType::Void) if arg_types[0] == FFIType::Ptr => {
                    let p = if num_call_args > 0 {
                        args.get(call_args_offset).integer_value(scope).unwrap_or(0)
                            as *mut libc::c_void
                    } else {
                        std::ptr::null_mut()
                    };
                    let func: extern "C" fn(*mut libc::c_void) =
                        unsafe { std::mem::transmute(fn_ptr) };
                    func(p);
                    rv.set(v8::undefined(scope).into());
                }
                // 2 args: (f64, f64) -> f64 (e.g. pow, atan2, hypot)
                (2, FFIType::F64)
                    if arg_types[0] == FFIType::F64 && arg_types[1] == FFIType::F64 =>
                {
                    let a0 = if num_call_args > 0 {
                        args.get(call_args_offset)
                            .number_value(scope)
                            .unwrap_or(0.0)
                    } else {
                        0.0
                    };
                    let a1 = if num_call_args > 1 {
                        args.get(call_args_offset + 1)
                            .number_value(scope)
                            .unwrap_or(0.0)
                    } else {
                        0.0
                    };
                    let func: extern "C" fn(f64, f64) -> f64 =
                        unsafe { std::mem::transmute(fn_ptr) };
                    rv.set(v8::Number::new(scope, func(a0, a1)).into());
                }
                // 2 args: (i32, i32) -> i32
                (2, FFIType::I32)
                    if arg_types[0] == FFIType::I32 && arg_types[1] == FFIType::I32 =>
                {
                    let a0 = if num_call_args > 0 {
                        args.get(call_args_offset).int32_value(scope).unwrap_or(0)
                    } else {
                        0
                    };
                    let a1 = if num_call_args > 1 {
                        args.get(call_args_offset + 1)
                            .int32_value(scope)
                            .unwrap_or(0)
                    } else {
                        0
                    };
                    let func: extern "C" fn(i32, i32) -> i32 =
                        unsafe { std::mem::transmute(fn_ptr) };
                    rv.set(v8::Integer::new(scope, func(a0, a1)).into());
                }
                // 2 args: (cstring, cstring) -> i32 (e.g. strcmp)
                (2, FFIType::I32)
                    if arg_types[0] == FFIType::CString && arg_types[1] == FFIType::CString =>
                {
                    let s0 = args.get(call_args_offset).to_rust_string_lossy(scope);
                    let s1 = args.get(call_args_offset + 1).to_rust_string_lossy(scope);
                    let c0 = CString::new(s0).unwrap_or_default();
                    let c1 = CString::new(s1).unwrap_or_default();
                    let func: extern "C" fn(*const libc::c_char, *const libc::c_char) -> i32 =
                        unsafe { std::mem::transmute(fn_ptr) };
                    rv.set(v8::Integer::new(scope, func(c0.as_ptr(), c1.as_ptr())).into());
                }
                // 3 args: (ptr, i32, usize) -> ptr (e.g. memset)
                (3, FFIType::Ptr)
                    if arg_types[0] == FFIType::Ptr && arg_types[1] == FFIType::I32 =>
                {
                    let p = args.get(call_args_offset).integer_value(scope).unwrap_or(0)
                        as *mut libc::c_void;
                    let val = args
                        .get(call_args_offset + 1)
                        .int32_value(scope)
                        .unwrap_or(0);
                    let n = args
                        .get(call_args_offset + 2)
                        .integer_value(scope)
                        .unwrap_or(0) as usize;
                    let func: extern "C" fn(
                        *mut libc::c_void,
                        libc::c_int,
                        usize,
                    ) -> *mut libc::c_void = unsafe { std::mem::transmute(fn_ptr) };
                    let ret_p = func(p, val, n);
                    rv.set(v8::BigInt::new_from_u64(scope, ret_p as u64).into());
                }
                // General fallback: integer register arguments
                _ => {
                    let mut int_args = [0usize; 6];
                    let limit = (num_call_args.max(0).min(6) as usize).min(6);
                    for i in 0..limit {
                        let arg_val = args.get(call_args_offset + i as i32);
                        if arg_val.is_number() {
                            int_args[i] = arg_val.integer_value(scope).unwrap_or(0) as usize;
                        } else if arg_val.is_big_int() {
                            if let Ok(bi) = v8::Local::<v8::BigInt>::try_from(arg_val) {
                                let (val, _) = bi.u64_value();
                                int_args[i] = val as usize;
                            }
                        } else if arg_val.is_string() {
                            let s = arg_val.to_rust_string_lossy(scope);
                            let c = CString::new(s).unwrap_or_default();
                            int_args[i] = c.into_raw() as usize;
                        }
                    }

                    let func: extern "C" fn(usize, usize, usize, usize, usize, usize) -> usize =
                        unsafe { std::mem::transmute(fn_ptr) };
                    let res = func(
                        int_args[0],
                        int_args[1],
                        int_args[2],
                        int_args[3],
                        int_args[4],
                        int_args[5],
                    );

                    match returns_type {
                        FFIType::Void => rv.set(v8::undefined(scope).into()),
                        FFIType::Bool => rv.set(v8::Boolean::new(scope, res != 0).into()),
                        FFIType::I8 => rv.set(v8::Integer::new(scope, res as i8 as i32).into()),
                        FFIType::U8 => rv.set(v8::Integer::new(scope, res as u8 as i32).into()),
                        FFIType::I16 => rv.set(v8::Integer::new(scope, res as i16 as i32).into()),
                        FFIType::U16 => rv.set(v8::Integer::new(scope, res as u16 as i32).into()),
                        FFIType::I32 => rv.set(v8::Integer::new(scope, res as i32).into()),
                        FFIType::U32 => {
                            rv.set(v8::Integer::new_from_unsigned(scope, res as u32).into())
                        }
                        FFIType::I64 => rv.set(v8::BigInt::new_from_i64(scope, res as i64).into()),
                        FFIType::U64 | FFIType::Ptr => {
                            rv.set(v8::BigInt::new_from_u64(scope, res as u64).into())
                        }
                        FFIType::F32 => rv.set(v8::Number::new(scope, (res as f32) as f64).into()),
                        FFIType::F64 => rv.set(v8::Number::new(scope, res as f64).into()),
                        FFIType::CString => {
                            let ptr = res as *const libc::c_char;
                            if ptr.is_null() {
                                rv.set(v8::null(scope).into());
                            } else {
                                let c_str = unsafe { CStr::from_ptr(ptr) };
                                let v8_str =
                                    v8::String::new(scope, &c_str.to_string_lossy()).unwrap();
                                rv.set(v8_str.into());
                            }
                        }
                    }
                }
            }
        },
    )
    .unwrap();

    // Native helper for closeLibrary
    let close_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let lib_id = args.get(0).int32_value(scope).unwrap_or(0) as usize;
            let registry = get_lib_registry();
            let mut guard = registry.lock().unwrap();
            let removed = guard.remove(&lib_id).is_some();
            rv.set(v8::Boolean::new(scope, removed).into());
        },
    )
    .unwrap();

    // Native pointer read/write utilities
    let read_ptr_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let raw_ptr = if args.get(0).is_big_int() {
                if let Ok(bi) = v8::Local::<v8::BigInt>::try_from(args.get(0)) {
                    let (val, _) = bi.u64_value();
                    val as usize
                } else {
                    0
                }
            } else {
                args.get(0).integer_value(scope).unwrap_or(0) as usize
            };

            let offset = args.get(1).integer_value(scope).unwrap_or(0) as usize;
            let type_str = args.get(2).to_rust_string_lossy(scope);
            let ptr = (raw_ptr + offset) as *const u8;

            if ptr.is_null() {
                rv.set(v8::null(scope).into());
                return;
            }

            match type_str.to_ascii_lowercase().as_str() {
                "i8" => {
                    rv.set(v8::Integer::new(scope, unsafe { *(ptr as *const i8) } as i32).into())
                }
                "u8" => rv.set(v8::Integer::new(scope, unsafe { *ptr } as i32).into()),
                "i16" => {
                    rv.set(v8::Integer::new(scope, unsafe { *(ptr as *const i16) } as i32).into())
                }
                "u16" => {
                    rv.set(v8::Integer::new(scope, unsafe { *(ptr as *const u16) } as i32).into())
                }
                "i32" => rv.set(v8::Integer::new(scope, unsafe { *(ptr as *const i32) }).into()),
                "u32" => rv.set(
                    v8::Integer::new_from_unsigned(scope, unsafe { *(ptr as *const u32) }).into(),
                ),
                "i64" => {
                    rv.set(v8::BigInt::new_from_i64(scope, unsafe { *(ptr as *const i64) }).into())
                }
                "u64" | "ptr" => {
                    rv.set(v8::BigInt::new_from_u64(scope, unsafe { *(ptr as *const u64) }).into())
                }
                "f32" => {
                    rv.set(v8::Number::new(scope, unsafe { *(ptr as *const f32) } as f64).into())
                }
                "f64" => rv.set(v8::Number::new(scope, unsafe { *(ptr as *const f64) }).into()),
                _ => rv.set(v8::undefined(scope).into()),
            }
        },
    )
    .unwrap();

    let write_ptr_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let raw_ptr = if args.get(0).is_big_int() {
                if let Ok(bi) = v8::Local::<v8::BigInt>::try_from(args.get(0)) {
                    let (val, _) = bi.u64_value();
                    val as usize
                } else {
                    0
                }
            } else {
                args.get(0).integer_value(scope).unwrap_or(0) as usize
            };

            let offset = args.get(1).integer_value(scope).unwrap_or(0) as usize;
            let type_str = args.get(2).to_rust_string_lossy(scope);
            let ptr = (raw_ptr + offset) as *mut u8;

            if ptr.is_null() {
                rv.set(v8::Boolean::new(scope, false).into());
                return;
            }

            let val = args.get(3);
            match type_str.to_ascii_lowercase().as_str() {
                "i8" => unsafe { *(ptr as *mut i8) = val.int32_value(scope).unwrap_or(0) as i8 },
                "u8" => unsafe { *ptr = val.int32_value(scope).unwrap_or(0) as u8 },
                "i16" => unsafe { *(ptr as *mut i16) = val.int32_value(scope).unwrap_or(0) as i16 },
                "u16" => unsafe { *(ptr as *mut u16) = val.int32_value(scope).unwrap_or(0) as u16 },
                "i32" => unsafe { *(ptr as *mut i32) = val.int32_value(scope).unwrap_or(0) },
                "u32" => unsafe { *(ptr as *mut u32) = val.uint32_value(scope).unwrap_or(0) },
                "i64" => {
                    let num = if val.is_big_int() {
                        v8::Local::<v8::BigInt>::try_from(val)
                            .map(|bi| bi.i64_value().0)
                            .unwrap_or(0)
                    } else {
                        val.integer_value(scope).unwrap_or(0)
                    };
                    unsafe { *(ptr as *mut i64) = num };
                }
                "u64" | "ptr" => {
                    let num = if val.is_big_int() {
                        v8::Local::<v8::BigInt>::try_from(val)
                            .map(|bi| bi.u64_value().0)
                            .unwrap_or(0)
                    } else {
                        val.integer_value(scope).unwrap_or(0) as u64
                    };
                    unsafe { *(ptr as *mut u64) = num };
                }
                "f32" => unsafe {
                    *(ptr as *mut f32) = val.number_value(scope).unwrap_or(0.0) as f32
                },
                "f64" => unsafe { *(ptr as *mut f64) = val.number_value(scope).unwrap_or(0.0) },
                _ => {
                    rv.set(v8::Boolean::new(scope, false).into());
                    return;
                }
            }

            rv.set(v8::Boolean::new(scope, true).into());
        },
    )
    .unwrap();

    let read_cstring_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let raw_ptr = if args.get(0).is_big_int() {
                if let Ok(bi) = v8::Local::<v8::BigInt>::try_from(args.get(0)) {
                    let (val, _) = bi.u64_value();
                    val as *const libc::c_char
                } else {
                    std::ptr::null()
                }
            } else {
                args.get(0).integer_value(scope).unwrap_or(0) as *const libc::c_char
            };

            if raw_ptr.is_null() {
                rv.set(v8::null(scope).into());
                return;
            }

            let c_str = unsafe { CStr::from_ptr(raw_ptr) };
            let s = v8::String::new(scope, &c_str.to_string_lossy()).unwrap();
            rv.set(s.into());
        },
    )
    .unwrap();

    let ptr_fn = v8::Function::new(
        scope,
        |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut rv: v8::ReturnValue| {
            let arg = args.get(0);
            if arg.is_array_buffer_view() {
                if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(arg) {
                    let byte_offset = view.byte_offset();
                    if let Some(ab) = view.buffer(scope) {
                        let store = ab.get_backing_store();
                        if let Some(data) = store.data() {
                            let ptr =
                                unsafe { (data.as_ptr() as *mut u8).add(byte_offset) as usize };
                            rv.set(v8::BigInt::new_from_u64(scope, ptr as u64).into());
                            return;
                        }
                    }
                }
            } else if arg.is_array_buffer() {
                if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(arg) {
                    let store = ab.get_backing_store();
                    if let Some(data) = store.data() {
                        let ptr = data.as_ptr() as usize;
                        rv.set(v8::BigInt::new_from_u64(scope, ptr as u64).into());
                        return;
                    }
                }
            }
            rv.set(v8::BigInt::new_from_u64(scope, 0).into());
        },
    )
    .unwrap();

    // Attach native binding bag
    let native_obj = v8::Object::new(scope);
    let k_dlopen = v8::String::new(scope, "dlopen").unwrap();
    let k_call_sym = v8::String::new(scope, "callSymbol").unwrap();
    let k_close = v8::String::new(scope, "close").unwrap();
    let k_read = v8::String::new(scope, "read").unwrap();
    let k_write = v8::String::new(scope, "write").unwrap();
    let k_read_str = v8::String::new(scope, "readCString").unwrap();
    let k_ptr = v8::String::new(scope, "ptr").unwrap();

    native_obj.set(scope, k_dlopen.into(), dlopen_fn.into());
    native_obj.set(scope, k_call_sym.into(), call_symbol_fn.into());
    native_obj.set(scope, k_close.into(), close_fn.into());
    native_obj.set(scope, k_read.into(), read_ptr_fn.into());
    native_obj.set(scope, k_write.into(), write_ptr_fn.into());
    native_obj.set(scope, k_read_str.into(), read_cstring_fn.into());
    native_obj.set(scope, k_ptr.into(), ptr_fn.into());

    let k_bee_ffi_native = v8::String::new(scope, "__bee_ffi_native").unwrap();
    global.set(scope, k_bee_ffi_native.into(), native_obj.into());

    // Inject high-level user-friendly JavaScript wrapper
    let js_code = r#"
    (function() {
        const native = globalThis.__bee_ffi_native;

        const FFIType = Object.freeze({
            void: 'void',
            bool: 'bool',
            boolean: 'bool',
            i8: 'i8',
            int8: 'i8',
            u8: 'u8',
            uint8: 'u8',
            i16: 'i16',
            int16: 'i16',
            u16: 'u16',
            uint16: 'u16',
            i32: 'i32',
            int32: 'i32',
            int: 'i32',
            u32: 'u32',
            uint32: 'u32',
            uint: 'u32',
            i64: 'i64',
            int64: 'i64',
            u64: 'u64',
            uint64: 'u64',
            f32: 'f32',
            float: 'f32',
            f64: 'f64',
            double: 'f64',
            ptr: 'ptr',
            pointer: 'ptr',
            cstring: 'cstring',
            string: 'cstring'
        });

        class DynamicLibraryWrapper {
            constructor(path, options = {}) {
                this.path = path || null;
                this.libId = native.dlopen(this.path);
                this.symbols = {};

                const symbolDefs = options.symbols || {};
                for (const [name, def] of Object.entries(symbolDefs)) {
                    const argTypes = def.args || [];
                    const retType = def.returns || 'void';

                    this.symbols[name] = (...callArgs) => {
                        return native.callSymbol(this.libId, name, argTypes, retType, ...callArgs);
                    };
                }
            }

            close() {
                if (this.libId !== null) {
                    const closed = native.close(this.libId);
                    this.libId = null;
                    return closed;
                }
                return false;
            }
        }

        function dlopen(path, options) {
            return new DynamicLibraryWrapper(path, options);
        }

        const ffi = {
            dlopen,
            FFIType,
            ptr: native.ptr,
            read: native.read,
            write: native.write,
            readCString: native.readCString,
            version: '1.4.0'
        };

        globalThis.__bee_ffi = ffi;
        globalThis.ffi = ffi;
    })();
    "#;

    let code_str = v8::String::new(scope, js_code).unwrap();
    let script = v8::Script::compile(scope, code_str, None)
        .ok_or_else(|| anyhow!("Failed to compile ffi bootstrap script"))?;
    script
        .run(scope)
        .ok_or_else(|| anyhow!("Failed to run ffi bootstrap script"))?;

    Ok(())
}
