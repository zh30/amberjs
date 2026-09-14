// Node.js Path模块实现
/// 路径操作工具
use anyhow::Result;
use rusty_v8 as v8;
/// 设置Path API
pub fn setup_path_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let path_obj: _ = v8::Object::new(scope);
    // join
    let join_func: _ = v8::FunctionTemplate::new(scope, path_join_callback);
    let join_instance: _ = join_func.get_function(scope).unwrap();
    let join_key: _ = v8::String::new(scope, "join").unwrap();
    path_obj.set(scope, join_key.into(), join_instance.into());
    // resolve
    let resolve_func: _ = v8::FunctionTemplate::new(scope, path_resolve_callback);
    let resolve_instance: _ = resolve_func.get_function(scope).unwrap();
    let resolve_key: _ = v8::String::new(scope, "resolve").unwrap();
    path_obj.set(scope, resolve_key.into(), resolve_instance.into());
    // relative
    let relative_func: _ = v8::FunctionTemplate::new(scope, path_relative_callback);
    let relative_instance: _ = relative_func.get_function(scope).unwrap();
    let relative_key: _ = v8::String::new(scope, "relative").unwrap();
    path_obj.set(scope, relative_key.into(), relative_instance.into());
    // dirname
    let dirname_func: _ = v8::FunctionTemplate::new(scope, path_dirname_callback);
    let dirname_instance: _ = dirname_func.get_function(scope).unwrap();
    let dirname_key: _ = v8::String::new(scope, "dirname").unwrap();
    path_obj.set(scope, dirname_key.into(), dirname_instance.into());
    // basename
    let basename_func: _ = v8::FunctionTemplate::new(scope, path_basename_callback);
    let basename_instance: _ = basename_func.get_function(scope).unwrap();
    let basename_key: _ = v8::String::new(scope, "basename").unwrap();
    path_obj.set(scope, basename_key.into(), basename_instance.into());
    // extname
    let extname_func: _ = v8::FunctionTemplate::new(scope, path_extname_callback);
    let extname_instance: _ = extname_func.get_function(scope).unwrap();
    let extname_key: _ = v8::String::new(scope, "extname").unwrap();
    path_obj.set(scope, extname_key.into(), extname_instance.into());
    // parse
    let parse_func: _ = v8::FunctionTemplate::new(scope, path_parse_callback);
    let parse_instance: _ = parse_func.get_function(scope).unwrap();
    let parse_key: _ = v8::String::new(scope, "parse").unwrap();
    path_obj.set(scope, parse_key.into(), parse_instance.into());
    // format
    let format_func: _ = v8::FunctionTemplate::new(scope, path_format_callback);
    let format_instance: _ = format_func.get_function(scope).unwrap();
    let format_key: _ = v8::String::new(scope, "format").unwrap();
    path_obj.set(scope, format_key.into(), format_instance.into());
    // isAbsolute
    let is_absolute_func: _ = v8::FunctionTemplate::new(scope, path_is_absolute_callback);
    let is_absolute_instance: _ = is_absolute_func.get_function(scope).unwrap();
    let is_absolute_key: _ = v8::String::new(scope, "isAbsolute").unwrap();
    path_obj.set(scope, is_absolute_key.into(), is_absolute_instance.into());
    // normalize
    let normalize_func: _ = v8::FunctionTemplate::new(scope, path_normalize_callback);
    let normalize_instance: _ = normalize_func.get_function(scope).unwrap();
    let normalize_key: _ = v8::String::new(scope, "normalize").unwrap();
    path_obj.set(scope, normalize_key.into(), normalize_instance.into());
    // sep
    let sep: _ = if cfg!(windows) { "\\" } else { "/" };
    let key_sep: _ = v8::String::new(scope, "sep").unwrap();
    let val_sep: _ = v8::String::new(scope, sep).unwrap();
    path_obj.set(scope, key_sep.into(), val_sep.into());
    // delimiter
    let delimiter: _ = if cfg!(windows) { ";" } else { ":" };
    let key_delimiter: _ = v8::String::new(scope, "delimiter").unwrap();
    let val_delimiter: _ = v8::String::new(scope, delimiter).unwrap();
    path_obj.set(scope, key_delimiter.into(), val_delimiter.into());
    // win32
    let win32_obj: _ = v8::Object::new(scope);
    let posix_obj: _ = v8::Object::new(scope);
    // 复制所有方法到win32和posix
    for &key_str in &[
        "join",
        "resolve",
        "relative",
        "dirname",
        "basename",
        "extname",
        "parse",
        "format",
        "isAbsolute",
        "normalize",
    ] {
        let key_val: _ = v8::String::new(scope, key_str).unwrap();
        if let Some(method) = path_obj.get(scope, key_val.into()) {
            let win32_key: _ = v8::String::new(scope, key_str).unwrap();
            win32_obj.set(scope, win32_key.into(), method);
            let posix_key: _ = v8::String::new(scope, key_str).unwrap();
            posix_obj.set(scope, posix_key.into(), method);
        }
    }
    let key_sep: _ = v8::String::new(scope, "sep").unwrap();
    let val_sep: _ = v8::String::new(scope, "\\").unwrap();
    win32_obj.set(scope, key_sep.into(), val_sep.into());
    let key_delimiter: _ = v8::String::new(scope, "delimiter").unwrap();
    let val_delimiter: _ = v8::String::new(scope, ";").unwrap();
    win32_obj.set(scope, key_delimiter.into(), val_delimiter.into());
    let key_sep: _ = v8::String::new(scope, "sep").unwrap();
    let val_sep: _ = v8::String::new(scope, "/").unwrap();
    posix_obj.set(scope, key_sep.into(), val_sep.into());
    let key_delimiter: _ = v8::String::new(scope, "delimiter").unwrap();
    let val_delimiter: _ = v8::String::new(scope, ":").unwrap();
    posix_obj.set(scope, key_delimiter.into(), val_delimiter.into());
    let win32_key: _ = v8::String::new(scope, "win32").unwrap();
    path_obj.set(scope, win32_key.into(), win32_obj.into());
    let posix_key: _ = v8::String::new(scope, "posix").unwrap();
    path_obj.set(scope, posix_key.into(), posix_obj.into());
    // 设置到全局
    let global: _ = context.global(scope);
    let path_key: _ = v8::String::new(scope, "path").unwrap();
    global.set(scope, path_key.into(), path_obj.into());
    Ok(())
}
fn path_join_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let mut paths = Vec::new();
    for i in 0..args.length() {
        let arg: _ = args.get(i);
        if let Some(s) = arg.to_string(scope) {
            let arg_str: _ = s.to_rust_string_lossy(scope);
            if !arg_str.is_empty() {
                paths.push(arg_str);
            }
        }
    }
    let mut result = String::new(); // Accumulates normalized path parts
    let is_windows: _ = cfg!(windows);
    for (i, path) in paths.iter().enumerate() {
        if i > 0 {
            if is_windows && !result.ends_with('\\') {
                result.push('\\');
            } else if !is_windows && !result.ends_with('/') {
                result.push('/');
            }
        }
        result.push_str(path);
    }
    // 规范化路径
    result = normalize_path(&result, is_windows);
    retval.set(v8::String::new(scope, &result).unwrap().into());
}
fn path_resolve_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let mut paths = Vec::new();
    for i in 0..args.length() {
        let arg: _ = args.get(i);
        if let Some(s) = arg.to_string(scope) {
            paths.push(s.to_rust_string_lossy(scope));
        }
    }
    let is_windows: _ = cfg!(windows);
    let separator: _ = if is_windows { '\\' } else { '/' };

    // Walk right to left prepending each segment, stopping at the first absolute
    // one; the cwd acts as the implicit leftmost segment.
    let mut resolved = String::new();
    let mut reached_absolute = false;
    for path in paths.iter().rev() {
        if path.is_empty() {
            continue;
        }
        resolved = if resolved.is_empty() {
            path.clone()
        } else {
            format!("{path}{separator}{resolved}")
        };
        if is_absolute_path(path, is_windows) {
            reached_absolute = true;
            break;
        }
    }
    if !reached_absolute {
        let cwd: _ = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .to_string_lossy()
            .to_string();
        resolved = if resolved.is_empty() {
            cwd
        } else {
            format!("{cwd}{separator}{resolved}")
        };
    }

    let mut result: _ = normalize_path(&resolved, is_windows);
    // resolve never reports a trailing separator, except for the root itself.
    while result.len() > 1 && result.ends_with(separator) {
        result.pop();
    }
    retval.set(v8::String::new(scope, &result).unwrap().into());
}

fn is_absolute_path(path: &str, is_windows: bool) -> bool {
    if is_windows {
        let bytes: _ = path.as_bytes();
        path.starts_with('\\')
            || path.starts_with('/')
            || (bytes.len() >= 3 && bytes[1] == b':' && (bytes[2] == b'\\' || bytes[2] == b'/'))
    } else {
        path.starts_with('/')
    }
}
fn path_relative_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let from: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let to: _ = args
        .get(1)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let is_windows: _ = cfg!(windows);
    // 简化的相对路径计算
    let from_parts: Vec<&str> = if is_windows {
        from.split('\\').filter(|s| !s.is_empty()).collect()
    } else {
        from.split('/').filter(|s| !s.is_empty()).collect()
    };
    let to_parts: Vec<&str> = if is_windows {
        to.split('\\').filter(|s| !s.is_empty()).collect()
    } else {
        to.split('/').filter(|s| !s.is_empty()).collect()
    };
    let mut result = String::new(); // Accumulates normalized path parts
    let mut i = 0;
    // 找到共同前缀
    while i < from_parts.len() && i < to_parts.len() && from_parts[i] == to_parts[i] {
        i += 1;
    }
    // 添加向上一级
    for _ in i..from_parts.len() {
        result.push_str("../");
    }
    // 添加剩余的to路径
    for part in &to_parts[i..] {
        result.push_str(part);
        if part != to_parts.last().unwrap() {
            result.push('/');
        }
    }
    if result.is_empty() {
        result = ".".to_string();
    }
    retval.set(v8::String::new(scope, &result).unwrap().into());
}
fn path_dirname_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let is_windows: _ = cfg!(windows);
    #[allow(unused_assignments)]
    let mut result = String::new();
    if is_windows {
        // Windows路径处理
        if path.len() == 2 && path.chars().nth(1) == Some(':') {
            result = path; // C: -> C:
        } else if path.len() > 2 && path.chars().nth(1) == Some(':') {
            result = path[..2].to_string(); // C:\path -> C:\
        } else if let Some(last_slash) = path.rfind('\\') {
            if last_slash == 0 {
                result = "\\".to_string();
            } else {
                result = path[..last_slash].to_string();
            }
        } else {
            result = ".".to_string();
        }
    } else {
        // POSIX路径处理
        if path == "/" {
            result = "/".to_string();
        } else if let Some(last_slash) = path.rfind('/') {
            if last_slash == 0 {
                result = "/".to_string();
            } else {
                result = path[..last_slash].to_string();
            }
        } else {
            result = ".".to_string();
        }
    }
    retval.set(v8::String::new(scope, &result).unwrap().into());
}
fn path_basename_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let ext: _ = args
        .get(1)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let is_windows: _ = cfg!(windows);
    let separator: _ = if is_windows { '\\' } else { '/' };
    let result: _ = if let Some(last_sep) = path.rfind(separator) {
        let basename: _ = &path[last_sep + 1..];
        if !ext.is_empty() && basename.ends_with(&ext) {
            &basename[..basename.len() - ext.len()]
        } else {
            basename
        }
    } else {
        if !ext.is_empty() && path.ends_with(&ext) {
            &path[..path.len() - ext.len()]
        } else {
            &path
        }
    };
    retval.set(v8::String::new(scope, result).unwrap().into());
}
fn path_extname_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let is_windows: _ = cfg!(windows);
    let separator: _ = if is_windows { '\\' } else { '/' };
    let basename: _ = match path.rfind(separator) {
        Some(last_sep) => &path[last_sep + 1..],
        None => path.as_str(),
    };
    // A dot that opens the basename marks a dotfile, not an extension, so
    // `.gitignore` and `..` have no extension while `..a` has `.a`.
    let result: _ = match basename.rfind('.') {
        Some(dot_pos) if dot_pos > 0 && basename != ".." => &basename[dot_pos..],
        _ => "",
    };
    retval.set(v8::String::new(scope, result).unwrap().into());
}
fn path_parse_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let is_windows: _ = cfg!(windows);
    let separator: _ = if is_windows { '\\' } else { '/' };
    let result: _ = v8::Object::new(scope);
    // root
    let root: String = if is_windows {
        if path.len() > 1 && path.chars().nth(1) == Some(':') {
            path[..3].to_string() // C:\
        } else if path.starts_with("\\\\") {
            "\\\\".to_string()
        } else {
            "".to_string()
        }
    } else {
        if path.starts_with('/') {
            "/".to_string()
        } else {
            "".to_string()
        }
    };
    // dir
    let dir: String = if let Some(last_sep) = path.rfind(separator) {
        if last_sep == 0 {
            if is_windows {
                "\\".to_string()
            } else {
                "/".to_string()
            }
        } else {
            path[..last_sep].to_string()
        }
    } else {
        "".to_string()
    };
    // base
    let base: &str = if let Some(last_sep) = path.rfind(separator) {
        &path[last_sep + 1..]
    } else {
        &path
    };
    // ext
    let ext: &str = if let Some(dot_pos) = base.rfind('.') {
        &base[dot_pos..]
    } else {
        ""
    };
    // name
    let name: &str = if !base.is_empty() && ext.len() < base.len() {
        &base[..base.len() - ext.len()]
    } else {
        base
    };
    let key_root: _ = v8::String::new(scope, "root").unwrap();
    let val_root: _ = v8::String::new(scope, &root).unwrap();
    result.set(scope, key_root.into(), val_root.into());
    let key_dir: _ = v8::String::new(scope, "dir").unwrap();
    let val_dir: _ = v8::String::new(scope, &dir).unwrap();
    result.set(scope, key_dir.into(), val_dir.into());
    let key_base: _ = v8::String::new(scope, "base").unwrap();
    let val_base: _ = v8::String::new(scope, base).unwrap();
    result.set(scope, key_base.into(), val_base.into());
    let key_ext: _ = v8::String::new(scope, "ext").unwrap();
    let val_ext: _ = v8::String::new(scope, ext).unwrap();
    result.set(scope, key_ext.into(), val_ext.into());
    let key_name: _ = v8::String::new(scope, "name").unwrap();
    let val_name: _ = v8::String::new(scope, name).unwrap();
    result.set(scope, key_name.into(), val_name.into());
    retval.set(result.into());
}
fn path_format_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path_obj: _ = args.get(0);
    if let Some(obj) = path_obj.to_object(scope) {
        let root_key: _ = v8::String::new(scope, "root").unwrap();
        let root: _ = obj
            .get(scope, root_key.into())
            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
            .unwrap_or_default();
        let dir_key: _ = v8::String::new(scope, "dir").unwrap();
        let dir: _ = obj
            .get(scope, dir_key.into())
            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
            .unwrap_or_default();
        let base_key: _ = v8::String::new(scope, "base").unwrap();
        let base: _ = obj
            .get(scope, base_key.into())
            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
            .unwrap_or_default();
        let name_key: _ = v8::String::new(scope, "name").unwrap();
        let name: _ = obj
            .get(scope, name_key.into())
            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
            .unwrap_or_default();
        let ext_key: _ = v8::String::new(scope, "ext").unwrap();
        let ext: _ = obj
            .get(scope, ext_key.into())
            .and_then(|v| v.to_string(scope).map(|s| s.to_rust_string_lossy(scope)))
            .unwrap_or_default();
        let file_part: _ = if !base.is_empty() {
            base.clone()
        } else {
            format!("{}{}", name, ext)
        };
        let result: _ = if !dir.is_empty() {
            format!("{}/{}", dir, file_part)
        } else if !root.is_empty() {
            format!("{}{}", root, file_part)
        } else {
            file_part
        };
        retval.set(v8::String::new(scope, &result).unwrap().into());
    } else {
        retval.set(v8::null(scope).into());
    }
}
fn path_is_absolute_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let is_windows: _ = cfg!(windows);
    let is_absolute: _ = if is_windows {
        // Windows: C:\path or \\server\share
        (path.len() > 1 && path.chars().nth(1) == Some(':')) || path.starts_with("\\\\")
    } else {
        // POSIX: /path
        path.starts_with('/')
    };
    retval.set(v8::Boolean::new(scope, is_absolute).into());
}
fn path_normalize_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let is_windows: _ = cfg!(windows);
    let result: _ = normalize_path(&path, is_windows);
    retval.set(v8::String::new(scope, &result).unwrap().into());
}
// 辅助函数：规范化路径
fn normalize_path(path: &str, is_windows: bool) -> String {
    let separator: _ = if is_windows { '\\' } else { '/' };
    let other_separator: _ = if is_windows { '/' } else { '\\' };
    let separator_str: _ = separator.to_string();
    let replaced_path: _ = path.replace(other_separator, &separator_str);

    // The root stays in front of the normalized body: a bare separator on POSIX,
    // and the drive prefix (`C:\`) on Windows.
    let bytes: _ = replaced_path.as_bytes();
    let root_len: usize = if is_windows && bytes.len() >= 3 && bytes[1] == b':' && bytes[2] == b'\\'
    {
        3
    } else if replaced_path.starts_with(separator) {
        1
    } else {
        0
    };
    let root: _ = &replaced_path[..root_len];
    let is_absolute: _ = root_len > 0;
    let body: _ = &replaced_path[root_len..];
    let keeps_trailing_separator: _ = !body.is_empty() && body.ends_with(separator);

    let mut stack: Vec<&str> = Vec::new();
    for part in body
        .split(separator)
        .filter(|part| !part.is_empty() && *part != ".")
    {
        if part != ".." {
            stack.push(part);
        } else if stack.last().is_some_and(|last| *last != "..") {
            stack.pop();
        } else if !is_absolute {
            // A relative path may legitimately climb above where it started;
            // an absolute one cannot go above its root.
            stack.push("..");
        }
    }

    let mut result: _ = format!("{root}{}", stack.join(&separator_str));
    if keeps_trailing_separator && !result.ends_with(separator) {
        result.push(separator);
    }
    if result.is_empty() {
        ".".to_string()
    } else {
        result
    }
}
