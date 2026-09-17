// Stage 93.3: Require Module - CommonJS Module Loader
// v0.3.52: Extracted from runtime_minimal.rs for better modularity
// v0.3.99: Added builtin module support for Node.js compatibility
//
// Provides require() function for loading both built-in and custom modules
// Builtin modules (os, crypto, events, etc.) are available as global objects

use anyhow::Result;
use rusty_v8 as v8;

/// Set up the require function and module system globals
/// This provides CommonJS-style module loading compatible with Node.js
pub fn setup_require_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Create module object
    let module_obj = v8::Object::new(scope);
    let module_key = v8::String::new(scope, "module").unwrap().into();
    global.set(scope, module_key, module_obj.clone().into());

    // Set module.id
    let module_id_key = v8::String::new(scope, "id").unwrap().into();
    let module_id_val = v8::String::new(scope, "<eval>").unwrap().into();
    module_obj.set(scope, module_id_key, module_id_val);

    // Set module.filename
    let module_filename_key = v8::String::new(scope, "filename").unwrap().into();
    let module_filename_val = v8::String::new(scope, "eval.js").unwrap().into();
    module_obj.set(scope, module_filename_key, module_filename_val);

    // Set module.loaded
    let module_loaded_key = v8::String::new(scope, "loaded").unwrap().into();
    let module_loaded_val = v8::Boolean::new(scope, false);
    module_obj.set(scope, module_loaded_key, module_loaded_val.into());

    // Create exports object (should be same as module.exports)
    let exports_obj = v8::Object::new(scope);

    // Set module.exports to reference exports_obj
    let module_exports_key = v8::String::new(scope, "exports").unwrap().into();
    module_obj.set(scope, module_exports_key, exports_obj.clone().into());

    // Create require function
    let require_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
        if args.length() >= 1 {
            let module_id = args.get(0);
            let requested_module_id_str = if let Some(s) = module_id.to_string(scope) {
                s.to_rust_string_lossy(scope)
            } else {
                "unknown".to_string()
            };
            let module_id_str = if let Some(builtin_name) =
                requested_module_id_str.strip_prefix("node:")
            {
                if crate::nodejs_core::commonjs_resolver::is_builtin_module(builtin_name) {
                    builtin_name.to_string()
                } else {
                    let error_msg = format!("Cannot find module '{}'", requested_module_id_str);
                    let error_str = v8::String::new(scope, &error_msg).unwrap();
                    let error_obj = v8::Exception::error(scope, error_str);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            } else if let Some(amber_name) =
                requested_module_id_str.strip_prefix("amber:")
            {
                if crate::nodejs_core::commonjs_resolver::is_builtin_module(&requested_module_id_str)
                    || crate::nodejs_core::commonjs_resolver::is_builtin_module(amber_name)
                {
                    amber_name.to_string()
                } else {
                    let error_msg = format!("Cannot find module '{}'", requested_module_id_str);
                    let error_str = v8::String::new(scope, &error_msg).unwrap();
                    let error_obj = v8::Exception::error(scope, error_str);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            } else {
                requested_module_id_str
            };

            // Return appropriate module object based on module id
            let result_obj = v8::Object::new(scope);

            match module_id_str.as_str() {
                "buffer" => {
                    // Create Buffer function template first
                    let buffer_fn_template = v8::FunctionTemplate::new(scope, |_scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                        let buffer_obj = v8::Object::new(_scope);

                        if args.length() >= 1 {
                            let first = args.get(0);
                            let bytes: Vec<u8> = if let Some(str_val) = first.to_string(_scope) {
                                str_val.to_rust_string_lossy(_scope).as_bytes().to_vec()
                            } else if first.is_number() {
                                let size = first.to_integer(_scope).unwrap().value() as usize;
                                vec![0u8; size]
                            } else {
                                vec![]
                            };

                            // Add length property
                            let length_key = v8::String::new(_scope, "length").unwrap().into();
                            let length_val = v8::Number::new(_scope, bytes.len() as f64);
                            buffer_obj.set(_scope, length_key, length_val.into());

                            // Add toString method
                            let to_string_fn = v8::Function::new(_scope, |scope: &mut v8::PinScope, _args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                                let result_str = v8::String::new(scope, "[Buffer]").unwrap();
                                retval.set(result_str.into());
                            }).unwrap();
                            let to_string_key = v8::String::new(_scope, "toString").unwrap().into();
                            buffer_obj.set(_scope, to_string_key, to_string_fn.into());
                        } else {
                            // Empty buffer
                            let length_key = v8::String::new(_scope, "length").unwrap().into();
                            let length_val = v8::Number::new(_scope, 0.0);
                            buffer_obj.set(_scope, length_key, length_val.into());
                        }

                        retval.set(buffer_obj.into());
                    });

                    // Create Buffer function instance
                    let buffer_fn = buffer_fn_template.get_function(scope).unwrap();

                    // Add Buffer.from as a static method
                    let from_fn = v8::Function::new(scope, |_scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                        let buffer_obj = v8::Object::new(_scope);

                        if args.length() >= 1 {
                            let first = args.get(0);
                            let bytes: Vec<u8> = if let Some(str_val) = first.to_string(_scope) {
                                str_val.to_rust_string_lossy(_scope).as_bytes().to_vec()
                            } else if first.is_number() {
                                let size = first.to_integer(_scope).unwrap().value() as usize;
                                vec![0u8; size]
                            } else {
                                vec![]
                            };

                            let length_key = v8::String::new(_scope, "length").unwrap().into();
                            let length_val = v8::Number::new(_scope, bytes.len() as f64);
                            buffer_obj.set(_scope, length_key, length_val.into());
                        } else {
                            let length_key = v8::String::new(_scope, "length").unwrap().into();
                            let length_val = v8::Number::new(_scope, 0.0);
                            buffer_obj.set(_scope, length_key, length_val.into());
                        }

                        retval.set(buffer_obj.into());
                    }).unwrap();
                    let from_key = v8::String::new(scope, "from").unwrap().into();
                    buffer_fn.set(scope, from_key, from_fn.into());

                    let buffer_key = v8::String::new(scope, "Buffer").unwrap().into();
                    result_obj.set(scope, buffer_key, buffer_fn.into());
                }
                "process" => {
                    let context = scope.get_current_context();
                    let global = context.global(scope);
                    let process_key = v8::String::new(scope, "process").unwrap();
                    if let Some(process_value) = global.get(scope, process_key.into()) {
                        if !process_value.is_undefined() {
                            retval.set(process_value);
                            return;
                        }
                    }

                    let error_str =
                        v8::String::new(scope, "process global is not available").unwrap();
                    let error_obj = v8::Exception::error(scope, error_str);
                    scope.throw_exception(error_obj.into());
                    return;
                }
                "path" => {
                    // Return path module with join function
                    let join_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                        let parts: Vec<String> = (0..args.length())
                            .filter_map(|i| args.get(i).to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                            .collect();
                        let result = if parts.len() > 1 {
                            parts.join("/")
                        } else if parts.len() == 1 {
                            parts[0].clone()
                        } else {
                            "".to_string()
                        };
                        let result_str = v8::String::new(scope, &result).unwrap();
                        retval.set(result_str.into());
                    }).unwrap();
                    let join_key = v8::String::new(scope, "join").unwrap().into();
                    result_obj.set(scope, join_key, join_fn.into());

                    // Add dirname function
                    let dirname_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                        let path_str = if let Some(s) = args.get(0).to_string(scope) {
                            s.to_rust_string_lossy(scope)
                        } else {
                            "/".to_string()
                        };
                        let result = std::path::Path::new(&path_str).parent()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| "/".to_string());
                        let result_str = v8::String::new(scope, &result).unwrap();
                        retval.set(result_str.into());
                    }).unwrap();
                    let dirname_key = v8::String::new(scope, "dirname").unwrap().into();
                    result_obj.set(scope, dirname_key, dirname_fn.into());

                    // Add basename function
                    let basename_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                        let path_str = if let Some(s) = args.get(0).to_string(scope) {
                            s.to_rust_string_lossy(scope)
                        } else {
                            "/".to_string()
                        };
                        let result = std::path::Path::new(&path_str).file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path_str);
                        let result_str = v8::String::new(scope, &result).unwrap();
                        retval.set(result_str.into());
                    }).unwrap();
                    let basename_key = v8::String::new(scope, "basename").unwrap().into();
                    result_obj.set(scope, basename_key, basename_fn.into());

                    // Add extname function
                    let extname_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                        let path_str = if let Some(s) = args.get(0).to_string(scope) {
                            s.to_rust_string_lossy(scope)
                        } else {
                            "".to_string()
                        };
                        let result = std::path::Path::new(&path_str).extension()
                            .map(|e| format!(".{}", e.to_string_lossy()))
                            .unwrap_or_else(|| "".to_string());
                        let result_str = v8::String::new(scope, &result).unwrap();
                        retval.set(result_str.into());
                    }).unwrap();
                    let extname_key = v8::String::new(scope, "extname").unwrap().into();
                    result_obj.set(scope, extname_key, extname_fn.into());

                    // Add resolve function
                    let resolve_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                        // Collect all path segments
                        let paths: Vec<String> = (0..args.length())
                            .filter_map(|i| args.get(i).to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
                            .collect();

                        // If no paths, return current directory
                        if paths.is_empty() {
                            let cwd = std::env::current_dir()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|_| "/".to_string());
                            retval.set(v8::String::new(scope, &cwd).unwrap().into());
                            return;
                        }

                        // If last path is absolute, use it directly
                        if let Some(last) = paths.last() {
                            if std::path::Path::new(last).is_absolute() {
                                let result_str = v8::String::new(scope, last).unwrap();
                                retval.set(result_str.into());
                                return;
                            }
                        }

                        // Start with current working directory
                        let mut result = std::env::current_dir()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|_| "/".to_string());

                        // Process each path segment
                        for path_str in paths {
                            if path_str.is_empty() {
                                continue;
                            }

                            if path_str.starts_with('/') {
                                // Absolute path segment
                                result = path_str.clone();
                            } else if path_str == "." {
                                // Current directory, do nothing
                                continue;
                            } else if path_str == ".." {
                                // Parent directory
                                if let Some(parent) = std::path::Path::new(&result).parent() {
                                    result = parent.to_string_lossy().to_string();
                                    if result.is_empty() {
                                        result = "/".to_string();
                                    }
                                }
                            } else {
                                // Regular path segment
                                if !result.ends_with('/') && !path_str.starts_with('/') {
                                    result.push('/');
                                }
                                result.push_str(&path_str);
                            }
                        }

                        // Clean up the result
                        let clean_result = std::path::Path::new(&result)
                            .to_string_lossy()
                            .to_string();

                        let result_str = v8::String::new(scope, &clean_result).unwrap();
                        retval.set(result_str.into());
                    }).unwrap();
                    let resolve_key = v8::String::new(scope, "resolve").unwrap().into();
                    result_obj.set(scope, resolve_key, resolve_fn.into());

                    // Add sep constant
                    let sep_key = v8::String::new(scope, "sep").unwrap().into();
                    let sep_val = v8::String::new(scope, "/").unwrap().into();
                    result_obj.set(scope, sep_key, sep_val);
                }
                "fs" => {
                    // Return fs module with file system methods
                    let fs_obj = v8::Object::new(scope);

                    // Add readFileSync function
                    let readfile_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                        if args.length() >= 1 {
                            if let Some(path_val) = args.get(0).to_string(scope) {
                                let path = path_val.to_rust_string_lossy(scope);
                                match std::fs::read_to_string(&path) {
                                    Ok(contents) => {
                                        let contents_val = v8::String::new(scope, &contents).unwrap();
                                        retval.set(contents_val.into());
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Error reading file: {}", e);
                                        let error_val = v8::String::new(scope, &error_msg).unwrap();
                                        retval.set(error_val.into());
                                    }
                                }
                            }
                        }
                    }).unwrap();
                    let readfile_key = v8::String::new(scope, "readFileSync").unwrap().into();
                    fs_obj.set(scope, readfile_key, readfile_fn.into());

                    // Add writeFileSync function
                    let writefile_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                        if args.length() >= 2 {
                            if let (Some(path_val), Some(data_val)) = (args.get(0).to_string(scope), args.get(1).to_string(scope)) {
                                let path = path_val.to_rust_string_lossy(scope);
                                let data = data_val.to_rust_string_lossy(scope);
                                match std::fs::write(&path, data) {
                                    Ok(_) => {
                                        let success_val = v8::undefined(scope).into();
                                        retval.set(success_val);
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Error writing file: {}", e);
                                        let error_val = v8::String::new(scope, &error_msg).unwrap();
                                        retval.set(error_val.into());
                                    }
                                }
                            }
                        }
                    }).unwrap();
                    let writefile_key = v8::String::new(scope, "writeFileSync").unwrap().into();
                    fs_obj.set(scope, writefile_key, writefile_fn.into());

                    // Add existsSync function
                    let exists_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                        if args.length() >= 1 {
                            if let Some(path_val) = args.get(0).to_string(scope) {
                                let path = path_val.to_rust_string_lossy(scope);
                                let exists = std::path::Path::new(&path).exists();
                                let exists_val = v8::Boolean::new(scope, exists);
                                retval.set(exists_val.into());
                            }
                        }
                    }).unwrap();
                    let exists_key = v8::String::new(scope, "existsSync").unwrap().into();
                    fs_obj.set(scope, exists_key, exists_fn.into());

                    // Add mkdirSync function
                    let mkdir_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                        if args.length() >= 1 {
                            if let Some(path_val) = args.get(0).to_string(scope) {
                                let path = path_val.to_rust_string_lossy(scope);
                                match std::fs::create_dir_all(&path) {
                                    Ok(_) => {
                                        retval.set(v8::undefined(scope).into());
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Error creating directory: {}", e);
                                        let error_val = v8::String::new(scope, &error_msg).unwrap();
                                        retval.set(error_val.into());
                                    }
                                }
                            }
                        }
                    }).unwrap();
                    let mkdir_key = v8::String::new(scope, "mkdirSync").unwrap().into();
                    fs_obj.set(scope, mkdir_key, mkdir_fn.into());

                    // Add readdirSync function
                    let readdir_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                        if args.length() >= 1 {
                            if let Some(path_val) = args.get(0).to_string(scope) {
                                let path = path_val.to_rust_string_lossy(scope);
                                match std::fs::read_dir(&path) {
                                    Ok(entries) => {
                                        let mut file_names = Vec::new();
                                        for entry in entries {
                                            if let Ok(entry) = entry {
                                                if let Ok(file_name) = entry.file_name().into_string() {
                                                    file_names.push(file_name);
                                                }
                                            }
                                        }
                                        let js_array = v8::Array::new(scope, file_names.len() as i32);
                                        for (i, name) in file_names.iter().enumerate() {
                                            let name_val = v8::String::new(scope, name).unwrap();
                                            js_array.set_index(scope, i as u32, name_val.into());
                                        }
                                        retval.set(js_array.into());
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Error reading directory: {}", e);
                                        let error_val = v8::String::new(scope, &error_msg).unwrap();
                                        retval.set(error_val.into());
                                    }
                                }
                            }
                        }
                    }).unwrap();
                    let readdir_key = v8::String::new(scope, "readdirSync").unwrap().into();
                    fs_obj.set(scope, readdir_key, readdir_fn.into());

                    // Add unlinkSync function
                    let unlink_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                        if args.length() >= 1 {
                            if let Some(path_val) = args.get(0).to_string(scope) {
                                let path = path_val.to_rust_string_lossy(scope);
                                match std::fs::remove_file(&path) {
                                    Ok(_) => {
                                        retval.set(v8::undefined(scope).into());
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Error deleting file: {}", e);
                                        let error_val = v8::String::new(scope, &error_msg).unwrap();
                                        let exception = v8::Exception::type_error(scope, error_val);
                                        scope.throw_exception(exception.into());
                                    }
                                }
                            }
                        }
                    }).unwrap();
                    let unlink_key = v8::String::new(scope, "unlinkSync").unwrap().into();
                    fs_obj.set(scope, unlink_key, unlink_fn.into());

                    // Add statSync function
                    let stat_fn = v8::Function::new(scope, |scope: &mut v8::PinScope, args: v8::FunctionCallbackArguments, mut retval: v8::ReturnValue| {
                        if args.length() >= 1 {
                            if let Some(path_val) = args.get(0).to_string(scope) {
                                let path = path_val.to_rust_string_lossy(scope);
                                match std::fs::metadata(&path) {
                                    Ok(metadata) => {
                                        let stat_obj = v8::Object::new(scope);
                                        let is_file_key = v8::String::new(scope, "isFile").unwrap().into();
                                        let is_file_val = v8::Boolean::new(scope, metadata.is_file());
                                        stat_obj.set(scope, is_file_key, is_file_val.into());

                                        let is_dir_key = v8::String::new(scope, "isDirectory").unwrap().into();
                                        let is_dir_val = v8::Boolean::new(scope, metadata.is_dir());
                                        stat_obj.set(scope, is_dir_key, is_dir_val.into());

                                        let size_key = v8::String::new(scope, "size").unwrap().into();
                                        let size_val = v8::Number::new(scope, metadata.len() as f64);
                                        stat_obj.set(scope, size_key, size_val.into());

                                        retval.set(stat_obj.into());
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Error getting metadata: {}", e);
                                        let error_val = v8::String::new(scope, &error_msg).unwrap();
                                        retval.set(error_val.into());
                                    }
                                }
                            }
                        }
                    }).unwrap();
                    let stat_key = v8::String::new(scope, "statSync").unwrap().into();
                    fs_obj.set(scope, stat_key, stat_fn.into());

                    // Set fs as the default export
                    let fs_default_key = v8::String::new(scope, "default").unwrap().into();
                    result_obj.set(scope, fs_default_key, fs_obj.into());
                }
                // v0.3.99: Handle builtin modules that are set as global objects
                // v0.3.281: Fixed readline support - directly return the global object
                // These modules are set up as global objects in the runtime
                "os" | "crypto" | "events" | "net" | "http" | "http2" | "https" | "util" | "url" |
                "querystring" | "dns" | "child_process" | "tcp_async" | "stream" |
                "readline" | "performance" | "diagnostics_channel" | "async_hooks" |
                "db" | "sqlite" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_db").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "vector" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_vector").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "std" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_std").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "std/dotenv" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_std_dotenv").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "std/cli" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_std_cli").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "std/fs" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_std_fs").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "std/crypto" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_std_crypto").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "std/assert" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_std_assert").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "mcp" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_mcp").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "ffi" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_ffi").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "pool" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_pool").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "wasm" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_wasm").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "replay" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_replay").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "weights" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_weights").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "security" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_security").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "permissions" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_permissions").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "kv" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_kv").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "tools" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_tools").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "sandbox" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_sandbox").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "vfs" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_vfs").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "bus" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_bus").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "grammar" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_grammar").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "checkpoint" => {
                    let global = scope.get_current_context().global(scope);
                    let k = v8::String::new(scope, "__amber_checkpoint").unwrap().into();
                    if let Some(val) = global.get(scope, k) {
                        retval.set(val);
                        return;
                    }
                }
                "ai" | "string_decoder" | "perf_hooks" => {
                    // Get the global object and directly return the module from it
                    let global = scope.get_current_context().global(scope);
                    let module_key = v8::String::new(scope, &module_id_str).unwrap().into();

                    if let Some(module_val) = global.get(scope, module_key) {
                        if !module_val.is_undefined() && !module_val.is_null() {
                            // Directly return the module object (not wrapped in { default: ... })
                            retval.set(module_val);
                            return;
                        }
                    }
                    // Fallback if module not found - return null
                    let null_val = v8::null(scope);
                    retval.set(null_val.into());
                }
                _ => {
                    // Check if module_id is a file path (absolute or relative path)
                    let module_path = std::path::Path::new(&module_id_str);

                    // Try to resolve as file path
                    if module_path.exists() && module_path.is_file() {
                        // Native .node addon support via process.dlopen
                        if module_path.extension().map_or(false, |e| e == "node") {
                            let module_obj = v8::Object::new(scope);
                            let exports_obj = v8::Object::new(scope);
                            let module_exports_key = v8::String::new(scope, "exports").unwrap().into();
                            module_obj.set(scope, module_exports_key, exports_obj.clone().into());
                            let filename_val = v8::String::new(scope, &module_path.to_string_lossy()).unwrap().into();

                            let global = scope.get_current_context().global(scope);
                            let process_key = v8::String::new(scope, "process").unwrap().into();
                            if let Some(process_val) = global.get(scope, process_key) {
                                if let Ok(process_obj) = v8::Local::<v8::Object>::try_from(process_val) {
                                    let dlopen_key = v8::String::new(scope, "dlopen").unwrap().into();
                                    if let Some(dlopen_val) = process_obj.get(scope, dlopen_key) {
                                        if let Ok(dlopen_fn) = v8::Local::<v8::Function>::try_from(dlopen_val) {
                                            let undefined = v8::undefined(scope);
                                            if dlopen_fn.call(scope, undefined.into(), &[module_obj.clone().into(), filename_val]).is_none() {
                                                return;
                                            }
                                            if let Some(exp) = module_obj.get(scope, module_exports_key) {
                                                retval.set(exp);
                                            } else {
                                                retval.set(exports_obj.into());
                                            }
                                            return;
                                        }
                                    }
                                }
                            }
                        }

                        // Read and execute the module file
                        match std::fs::read_to_string(module_path) {
                            Ok(code) => {
                                // Create new module and exports objects for this module
                                let module_obj = v8::Object::new(scope);
                                let exports_obj = v8::Object::new(scope);
                                let module_exports_key = v8::String::new(scope, "exports").unwrap().into();
                                module_obj.set(scope, module_exports_key, exports_obj.clone().into());

                                // Get directory and filename for __dirname and __filename
                                let module_dirname = module_path.parent()
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "/".to_string());
                                let module_filename = module_path.to_string_lossy().to_string();

                                // Wrap module code in CommonJS wrapper
                                let wrapper_code = format!(
                                    r#"(function(module, exports, __dirname, __filename) {{ {} }})"#,
                                    code
                                );

                                let script_source = v8::String::new(scope, &wrapper_code).unwrap();
                                let script = v8::Script::compile(scope, script_source, None).unwrap();
                                let wrapper_func_val = script.run(scope).unwrap();

                                let wrapper_func = v8::Local::<v8::Function>::try_from(wrapper_func_val).unwrap();
                                let undefined = v8::undefined(scope);
                                let dirname_val = v8::String::new(scope, &module_dirname).unwrap().into();
                                let filename_val = v8::String::new(scope, &module_filename).unwrap().into();
                                if wrapper_func.call(scope, undefined.into(), &[module_obj.clone().into(), exports_obj.clone().into(), dirname_val, filename_val]).is_none() {
                                    return;
                                }

                                let module_exports_lookup_key = v8::String::new(scope, "exports").unwrap().into();
                                if let Some(module_exports) = module_obj.get(scope, module_exports_lookup_key) {
                                    retval.set(module_exports);
                                } else {
                                    retval.set(exports_obj.into());
                                }
                                return;
                            }
                            Err(e) => {
                                let error_msg = format!("Error loading module '{}': {}", module_id_str, e);
                                let error_str = v8::String::new(scope, &error_msg).unwrap();
                                let error_obj = v8::Exception::error(scope, error_str);
                                scope.throw_exception(error_obj.into());
                                return;
                            }
                        }
                    }

                    // Check if it's a relative path that needs resolution
                    if module_id_str.starts_with("./") || module_id_str.starts_with("../") {
                        // Try adding .js extension
                        let js_path = format!("{}.js", module_id_str);
                        let js_module_path = std::path::Path::new(&js_path);
                        if js_module_path.exists() && js_module_path.is_file() {
                            match std::fs::read_to_string(js_module_path) {
                                Ok(code) => {
                                    let module_obj = v8::Object::new(scope);
                                    let exports_obj = v8::Object::new(scope);
                                    let module_exports_key = v8::String::new(scope, "exports").unwrap().into();
                                    module_obj.set(scope, module_exports_key, exports_obj.clone().into());

                                    let module_dirname = js_module_path.parent()
                                        .map(|p| p.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "/".to_string());
                                    let module_filename = js_module_path.to_string_lossy().to_string();

                                    let wrapper_code = format!(
                                        r#"(function(module, exports, __dirname, __filename) {{ {} }})"#,
                                        code
                                    );

                                    let script_source = v8::String::new(scope, &wrapper_code).unwrap();
                                    let script = v8::Script::compile(scope, script_source, None).unwrap();
                                    let wrapper_func_val = script.run(scope).unwrap();

                                    let wrapper_func = v8::Local::<v8::Function>::try_from(wrapper_func_val).unwrap();
                                    let undefined = v8::undefined(scope);
                                    let dirname_val = v8::String::new(scope, &module_dirname).unwrap().into();
                                    let filename_val = v8::String::new(scope, &module_filename).unwrap().into();
                                    if wrapper_func.call(scope, undefined.into(), &[module_obj.clone().into(), exports_obj.clone().into(), dirname_val, filename_val]).is_none() {
                                        return;
                                    }

                                    let module_exports_lookup_key = v8::String::new(scope, "exports").unwrap().into();
                                    if let Some(module_exports) = module_obj.get(scope, module_exports_lookup_key) {
                                        retval.set(module_exports);
                                    } else {
                                        retval.set(exports_obj.into());
                                    }
                                    return;
                                }
                                Err(e) => {
                                    let error_msg = format!("Error loading module '{}': {}", js_path, e);
                                    let error_str = v8::String::new(scope, &error_msg).unwrap();
                                    let error_obj = v8::Exception::error(scope, error_str);
                                    scope.throw_exception(error_obj.into());
                                    return;
                                }
                            }
                        }
                    }

                    // Throw error for unknown modules
                    let error_msg = format!("Cannot find module '{}'", module_id_str);
                    let error_str = v8::String::new(scope, &error_msg).unwrap();
                    let error_obj = v8::Exception::error(scope, error_str);
                    scope.throw_exception(error_obj.into());
                    return;
                }
            }

            retval.set(result_obj.into());
        }
    }).ok_or_else(|| anyhow::anyhow!("Failed to create require function"))?;

    // Set global objects
    let require_key = v8::String::new(scope, "require").unwrap().into();
    global.set(scope, require_key, require_fn.into());

    let exports_key = v8::String::new(scope, "exports").unwrap().into();
    global.set(scope, exports_key, exports_obj.into());

    Ok(())
}
