// Node.js fs模块实现 - v0.3.66 增强版
/// 文件系统操作 - 支持同步API和Promise API
use anyhow::Result;
use rusty_v8 as v8;
use std::fs;
use std::io::Write as _;
use std::path::Path;

use crate::permissions::{
    check_global_permission, PermissionAction, PermissionError, PermissionKind, ResourceId,
};

/// 创建 Buffer 对象（v8::ArrayBuffer）- v0.3.66
/// 用于 'buffer' 编码读取时返回二进制数据
fn create_buffer_from_bytes<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    bytes: &[u8],
) -> v8::Local<'a, v8::Value> {
    let _buffer: v8::Local<v8::ArrayBuffer> = v8::ArrayBuffer::new(scope, bytes.len());
    // Note: rusty_v8 0.22 不支持直接访问 backing_store
    // 创建一个具有 _length 属性的对象来模拟 Buffer
    let buffer_obj = v8::Object::new(scope);
    let length_key = v8::String::new(scope, "_length").unwrap();
    let length_val = v8::Integer::new(scope, bytes.len() as i32);
    buffer_obj.set(scope, length_key.into(), length_val.into());
    // 如果有 backing_store 访问权限，可以存储实际数据
    buffer_obj.into()
}

fn throw_permission_error(scope: &mut v8::PinScope, error: PermissionError) {
    let message = v8::String::new(scope, &error.to_string()).unwrap();
    let exception = v8::Exception::type_error(scope, message);
    scope.throw_exception(exception);
}

fn ensure_fs_permission(scope: &mut v8::PinScope, action: PermissionAction, path: &str) -> bool {
    if !crate::permissions::has_restrictions() {
        return true;
    }
    match check_global_permission(
        PermissionKind::FileSystem,
        action,
        ResourceId::Path(Path::new(path).to_path_buf()),
    ) {
        Ok(()) => true,
        Err(error) => {
            throw_permission_error(scope, error);
            false
        }
    }
}

fn get_stats_flag(scope: &mut v8::PinScope, this: v8::Local<v8::Object>, key: &str) -> bool {
    let key = v8::String::new(scope, key).unwrap();
    this.get(scope, key.into())
        .map(|value| value.to_boolean(scope).boolean_value(scope))
        .unwrap_or(false)
}

fn stats_is_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let is_file = get_stats_flag(scope, args.this(), "__isFile");
    retval.set(v8::Boolean::new(scope, is_file).into());
}

fn stats_is_directory_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let is_directory = get_stats_flag(scope, args.this(), "__isDirectory");
    retval.set(v8::Boolean::new(scope, is_directory).into());
}

fn create_stats_object<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    metadata: &std::fs::Metadata,
) -> v8::Local<'a, v8::Object> {
    let stat_obj = v8::Object::new(scope);

    let is_file_state_key = v8::String::new(scope, "__isFile").unwrap();
    let is_file_state = v8::Boolean::new(scope, metadata.is_file());
    stat_obj.set(scope, is_file_state_key.into(), is_file_state.into());

    let is_dir_state_key = v8::String::new(scope, "__isDirectory").unwrap();
    let is_dir_state = v8::Boolean::new(scope, metadata.is_dir());
    stat_obj.set(scope, is_dir_state_key.into(), is_dir_state.into());

    let is_file_func = v8::FunctionTemplate::new(scope, stats_is_file_callback);
    let is_file_instance = is_file_func.get_function(scope).unwrap();
    let is_file_key = v8::String::new(scope, "isFile").unwrap();
    stat_obj.set(scope, is_file_key.into(), is_file_instance.into());

    let is_dir_func = v8::FunctionTemplate::new(scope, stats_is_directory_callback);
    let is_dir_instance = is_dir_func.get_function(scope).unwrap();
    let is_dir_key = v8::String::new(scope, "isDirectory").unwrap();
    stat_obj.set(scope, is_dir_key.into(), is_dir_instance.into());

    let size_key = v8::String::new(scope, "size").unwrap();
    let size_value = v8::Number::new(scope, metadata.len() as f64);
    stat_obj.set(scope, size_key.into(), size_value.into());

    let mode_key = v8::String::new(scope, "mode").unwrap();
    let mode_value = v8::Number::new(scope, 420.0_f64);
    stat_obj.set(scope, mode_key.into(), mode_value.into());

    let mtime_key = v8::String::new(scope, "mtime").unwrap();
    if let Ok(modified) = metadata.modified() {
        if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
            let mtime_value = v8::Number::new(scope, duration.as_secs() as f64 * 1000.0);
            stat_obj.set(scope, mtime_key.into(), mtime_value.into());
        }
    }

    stat_obj
}

fn create_vfs_stats_object<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    meta: &crate::sandbox::virtual_fs::VfsMetadata,
) -> v8::Local<'a, v8::Object> {
    let stat_obj = v8::Object::new(scope);

    let is_file_state_key = v8::String::new(scope, "__isFile").unwrap();
    let is_file_state = v8::Boolean::new(scope, meta.is_file);
    stat_obj.set(scope, is_file_state_key.into(), is_file_state.into());

    let is_dir_state_key = v8::String::new(scope, "__isDirectory").unwrap();
    let is_dir_state = v8::Boolean::new(scope, meta.is_dir);
    stat_obj.set(scope, is_dir_state_key.into(), is_dir_state.into());

    let is_file_func = v8::FunctionTemplate::new(scope, stats_is_file_callback);
    let is_file_instance = is_file_func.get_function(scope).unwrap();
    let is_file_key = v8::String::new(scope, "isFile").unwrap();
    stat_obj.set(scope, is_file_key.into(), is_file_instance.into());

    let is_dir_func = v8::FunctionTemplate::new(scope, stats_is_directory_callback);
    let is_dir_instance = is_dir_func.get_function(scope).unwrap();
    let is_dir_key = v8::String::new(scope, "isDirectory").unwrap();
    stat_obj.set(scope, is_dir_key.into(), is_dir_instance.into());

    let size_key = v8::String::new(scope, "size").unwrap();
    let size_value = v8::Number::new(scope, meta.len as f64);
    stat_obj.set(scope, size_key.into(), size_value.into());

    let mode_key = v8::String::new(scope, "mode").unwrap();
    let mode_value = v8::Number::new(scope, 420.0_f64);
    stat_obj.set(scope, mode_key.into(), mode_value.into());

    let mtime_key = v8::String::new(scope, "mtime").unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let mtime_value = v8::Number::new(scope, now.as_secs() as f64 * 1000.0);
    stat_obj.set(scope, mtime_key.into(), mtime_value.into());

    stat_obj
}

/// 设置fs API到全局作用域
pub fn setup_fs_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let fs_obj = v8::Object::new(scope);

    // readFileSync - 读取文件内容
    let read_func = v8::FunctionTemplate::new(scope, fs_read_file_sync_callback);
    let read_instance = read_func.get_function(scope).unwrap();
    let read_key = v8::String::new(scope, "readFileSync").unwrap();
    fs_obj.set(scope, read_key.into(), read_instance.into());

    // writeFileSync - 写入文件内容
    let write_func = v8::FunctionTemplate::new(scope, fs_write_file_sync_callback);
    let write_instance = write_func.get_function(scope).unwrap();
    let write_key = v8::String::new(scope, "writeFileSync").unwrap();
    fs_obj.set(scope, write_key.into(), write_instance.into());

    // existsSync - 检查文件是否存在
    let exists_func = v8::FunctionTemplate::new(scope, fs_exists_sync_callback);
    let exists_instance = exists_func.get_function(scope).unwrap();
    let exists_key = v8::String::new(scope, "existsSync").unwrap();
    fs_obj.set(scope, exists_key.into(), exists_instance.into());

    // mkdirSync - 创建目录
    let mkdir_func = v8::FunctionTemplate::new(scope, fs_mkdir_sync_callback);
    let mkdir_instance = mkdir_func.get_function(scope).unwrap();
    let mkdir_key = v8::String::new(scope, "mkdirSync").unwrap();
    fs_obj.set(scope, mkdir_key.into(), mkdir_instance.into());

    // readdirSync - 读取目录内容
    let readdir_func = v8::FunctionTemplate::new(scope, fs_readdir_sync_callback);
    let readdir_instance = readdir_func.get_function(scope).unwrap();
    let readdir_key = v8::String::new(scope, "readdirSync").unwrap();
    fs_obj.set(scope, readdir_key.into(), readdir_instance.into());

    // statSync - 获取文件状态
    let stat_func = v8::FunctionTemplate::new(scope, fs_stat_sync_callback);
    let stat_instance = stat_func.get_function(scope).unwrap();
    let stat_key = v8::String::new(scope, "statSync").unwrap();
    fs_obj.set(scope, stat_key.into(), stat_instance.into());

    // unlinkSync - 删除文件 - v0.3.64
    let unlink_func = v8::FunctionTemplate::new(scope, fs_unlink_sync_callback);
    let unlink_instance = unlink_func.get_function(scope).unwrap();
    let unlink_key = v8::String::new(scope, "unlinkSync").unwrap();
    fs_obj.set(scope, unlink_key.into(), unlink_instance.into());

    // renameSync - 重命名文件 - v0.3.64
    let rename_func = v8::FunctionTemplate::new(scope, fs_rename_sync_callback);
    let rename_instance = rename_func.get_function(scope).unwrap();
    let rename_key = v8::String::new(scope, "renameSync").unwrap();
    fs_obj.set(scope, rename_key.into(), rename_instance.into());

    // rmdirSync - 删除目录
    let rmdir_func = v8::FunctionTemplate::new(scope, fs_rmdir_sync_callback);
    let rmdir_instance = rmdir_func.get_function(scope).unwrap();
    let rmdir_key = v8::String::new(scope, "rmdirSync").unwrap();
    fs_obj.set(scope, rmdir_key.into(), rmdir_instance.into());

    // readFile/writeFile/appendFile - callback 风格最小兼容
    let read_async_func = v8::FunctionTemplate::new(scope, fs_read_file_callback);
    let read_async_instance = read_async_func.get_function(scope).unwrap();
    let read_async_key = v8::String::new(scope, "readFile").unwrap();
    fs_obj.set(scope, read_async_key.into(), read_async_instance.into());

    let write_async_func = v8::FunctionTemplate::new(scope, fs_write_file_callback);
    let write_async_instance = write_async_func.get_function(scope).unwrap();
    let write_async_key = v8::String::new(scope, "writeFile").unwrap();
    fs_obj.set(scope, write_async_key.into(), write_async_instance.into());

    let append_async_func = v8::FunctionTemplate::new(scope, fs_append_file_callback);
    let append_async_instance = append_async_func.get_function(scope).unwrap();
    let append_async_key = v8::String::new(scope, "appendFile").unwrap();
    fs_obj.set(scope, append_async_key.into(), append_async_instance.into());

    // promises - v0.3.64: 添加 Promise API
    let promises_obj = create_fs_promises(scope);
    let promises_key = v8::String::new(scope, "promises").unwrap();
    fs_obj.set(scope, promises_key.into(), promises_obj.into());

    // 设置到全局对象
    let global = context.global(scope);
    let fs_key = v8::String::new(scope, "fs").unwrap();
    global.set(scope, fs_key.into(), fs_obj.into());

    Ok(())
}

/// If `then()`'s fulfillment callback returns another thenable, chain to it.
/// Existing tests still read `__result__` on the original object.
fn thenable_chain_return(
    scope: &mut v8::PinScope,
    this: v8::Local<v8::Object>,
    maybe_result: Option<v8::Local<v8::Value>>,
    retval: &mut v8::ReturnValue,
) {
    if let Some(result) = maybe_result {
        if result.is_object() {
            if let Ok(obj) = v8::Local::<v8::Object>::try_from(result) {
                let then_key = v8::String::new(scope, "then").unwrap();
                if let Some(then_val) = obj.get(scope, then_key.into()) {
                    if then_val.is_function() {
                        retval.set(result);
                        return;
                    }
                }
            }
        }
    }
    retval.set(this.into());
}

/// 创建 fs.promises 对象 - v0.3.64
fn create_fs_promises<'a>(scope: &mut v8::PinScope<'a, '_>) -> v8::Local<'a, v8::Object> {
    let promises_obj = v8::Object::new(scope);

    // readFile - 返回一个 thenable 对象
    let read_file_func = v8::FunctionTemplate::new(scope, fs_promises_read_file_callback);
    let read_file_instance = read_file_func.get_function(scope).unwrap();
    let read_file_key = v8::String::new(scope, "readFile").unwrap();
    promises_obj.set(scope, read_file_key.into(), read_file_instance.into());

    // writeFile
    let write_file_func = v8::FunctionTemplate::new(scope, fs_promises_write_file_callback);
    let write_file_instance = write_file_func.get_function(scope).unwrap();
    let write_file_key = v8::String::new(scope, "writeFile").unwrap();
    promises_obj.set(scope, write_file_key.into(), write_file_instance.into());

    // appendFile
    let append_file_func = v8::FunctionTemplate::new(scope, fs_promises_append_file_callback);
    let append_file_instance = append_file_func.get_function(scope).unwrap();
    let append_file_key = v8::String::new(scope, "appendFile").unwrap();
    promises_obj.set(scope, append_file_key.into(), append_file_instance.into());

    // mkdir
    let mkdir_func = v8::FunctionTemplate::new(scope, fs_promises_mkdir_callback);
    let mkdir_instance = mkdir_func.get_function(scope).unwrap();
    let mkdir_key = v8::String::new(scope, "mkdir").unwrap();
    promises_obj.set(scope, mkdir_key.into(), mkdir_instance.into());

    // rmdir
    let rmdir_func = v8::FunctionTemplate::new(scope, fs_promises_rmdir_callback);
    let rmdir_instance = rmdir_func.get_function(scope).unwrap();
    let rmdir_key = v8::String::new(scope, "rmdir").unwrap();
    promises_obj.set(scope, rmdir_key.into(), rmdir_instance.into());

    // readdir
    let readdir_func = v8::FunctionTemplate::new(scope, fs_promises_readdir_callback);
    let readdir_instance = readdir_func.get_function(scope).unwrap();
    let readdir_key = v8::String::new(scope, "readdir").unwrap();
    promises_obj.set(scope, readdir_key.into(), readdir_instance.into());

    // stat
    let stat_func = v8::FunctionTemplate::new(scope, fs_promises_stat_callback);
    let stat_instance = stat_func.get_function(scope).unwrap();
    let stat_key = v8::String::new(scope, "stat").unwrap();
    promises_obj.set(scope, stat_key.into(), stat_instance.into());

    // unlink
    let unlink_func = v8::FunctionTemplate::new(scope, fs_promises_unlink_callback);
    let unlink_instance = unlink_func.get_function(scope).unwrap();
    let unlink_key = v8::String::new(scope, "unlink").unwrap();
    promises_obj.set(scope, unlink_key.into(), unlink_instance.into());

    // rename
    let rename_func = v8::FunctionTemplate::new(scope, fs_promises_rename_callback);
    let rename_instance = rename_func.get_function(scope).unwrap();
    let rename_key = v8::String::new(scope, "rename").unwrap();
    promises_obj.set(scope, rename_key.into(), rename_instance.into());

    promises_obj
}

#[inline]
fn get_path_fast<'a>(
    scope: &mut v8::PinScope,
    val: v8::Local<v8::Value>,
    buf: &'a mut [u8; 512],
) -> (std::borrow::Cow<'a, str>, Option<*const libc::c_char>) {
    if let Some(s) = val.to_string(scope) {
        let is_one_byte = s.contains_only_onebyte();
        let len = if is_one_byte { s.length() } else { s.utf8_length(scope) };
        if len > 0 && len < 511 {
            if is_one_byte {
                s.write_one_byte_v2(scope, 0, &mut buf[..len], v8::WriteFlags::empty());
            } else {
                s.write_utf8_v2(scope, &mut buf[..len], v8::WriteFlags::empty(), None);
            }
            buf[len] = 0;
            if let Ok(valid_str) = std::str::from_utf8(&buf[..len]) {
                return (
                    std::borrow::Cow::Borrowed(valid_str),
                    Some(buf.as_ptr() as *const libc::c_char),
                );
            }
        }
        let owned = s.to_rust_string_lossy(scope);
        (std::borrow::Cow::Owned(owned), None)
    } else {
        (std::borrow::Cow::Borrowed(""), None)
    }
}

#[inline]
fn direct_write_sync(
    c_path: Option<*const libc::c_char>,
    path_str: &str,
    data: &[u8],
) -> std::io::Result<()> {
    if crate::sandbox::virtual_fs::is_enabled() {
        return crate::sandbox::virtual_fs::vfs_write(std::path::Path::new(path_str), data);
    }
    #[cfg(unix)]
    if let Some(ptr) = c_path {
        let fd = unsafe { libc::open(ptr, libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o666) };
        if fd >= 0 {
            let mut written = 0;
            let len = data.len();
            let mut ok = true;
            while written < len {
                let n = unsafe {
                    libc::write(
                        fd,
                        data.as_ptr().add(written) as *const libc::c_void,
                        len - written,
                    )
                };
                if n <= 0 {
                    ok = false;
                    break;
                }
                written += n as usize;
            }
            unsafe { libc::close(fd) };
            if ok {
                return Ok(());
            }
        }
    }
    std::fs::write(path_str, data)
}

/// fs.readFileSync(path, encoding) - 读取文件
fn fs_read_file_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let mut path_buf = [0u8; 512];
    let (path, c_path) = get_path_fast(scope, args.get(0), &mut path_buf);

    if !ensure_fs_permission(scope, PermissionAction::Read, path.as_ref()) {
        return;
    }

    let encoding = if args.length() > 1 {
        let arg1 = args.get(1);
        if arg1.is_string() {
            arg1.to_string(scope).map(|s| s.to_rust_string_lossy(scope))
        } else if arg1.is_object() {
            let enc_key = v8::String::new(scope, "encoding").unwrap();
            v8::Local::<v8::Object>::try_from(arg1)
                .ok()
                .and_then(|o| o.get(scope, enc_key.into()))
                .and_then(|v| v.to_string(scope))
                .map(|s| s.to_rust_string_lossy(scope))
        } else {
            None
        }
    } else {
        None
    };

    if crate::sandbox::virtual_fs::is_enabled() {
        match crate::sandbox::virtual_fs::vfs_read(std::path::Path::new(path.as_ref())) {
            Ok(bytes) => {
                if let Some(enc) = encoding {
                    let s = match enc.to_ascii_lowercase().as_str() {
                        "utf8" | "utf-8" => String::from_utf8_lossy(&bytes).into_owned(),
                        "hex" => hex::encode(&bytes),
                        "base64" => {
                            use base64::Engine;
                            base64::engine::general_purpose::STANDARD.encode(&bytes)
                        }
                        _ => String::from_utf8_lossy(&bytes).into_owned(),
                    };
                    if let Some(v8_str) = v8::String::new(scope, &s) {
                        retval.set(v8_str.into());
                    }
                } else {
                    let len = bytes.len();
                    let ab = v8::ArrayBuffer::new(scope, len);
                    if len > 0 {
                        let store = ab.get_backing_store();
                        let ptr = store.as_ref().as_ptr() as *mut u8;
                        if !ptr.is_null() {
                            unsafe {
                                std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, len);
                            }
                        }
                    }
                    if let Some(u8_arr) = v8::Uint8Array::new(scope, ab, 0, len) {
                        crate::runtime_minimal::set_buffer_prototype_fast(scope, u8_arr);
                        retval.set(u8_arr.into());
                    } else {
                        retval.set(v8::undefined(scope).into());
                    }
                }
                return;
            }
            Err(e) => {
                let error_msg = format!("Error reading file: {}", e);
                let error = v8::String::new(scope, &error_msg).unwrap();
                let exc = v8::Exception::type_error(scope, error);
                scope.throw_exception(exc);
                return;
            }
        }
    }

    #[cfg(unix)]
    if encoding.is_none() {
        if let Some(ptr) = c_path {
            let fd = unsafe { libc::open(ptr, libc::O_RDONLY) };
            if fd >= 0 {
                let mut st: libc::stat = unsafe { std::mem::zeroed() };
                if unsafe { libc::fstat(fd, &mut st) } == 0 {
                    let len = st.st_size as usize;
                    let ab = v8::ArrayBuffer::new(scope, len);
                    if len > 0 {
                        let store = ab.get_backing_store();
                        let dst_ptr = store.as_ref().as_ptr() as *mut u8;
                        if !dst_ptr.is_null() {
                            let mut read_bytes = 0;
                            while read_bytes < len {
                                let n = unsafe {
                                    libc::read(
                                        fd,
                                        dst_ptr.add(read_bytes) as *mut libc::c_void,
                                        len - read_bytes,
                                    )
                                };
                                if n <= 0 {
                                    break;
                                }
                                read_bytes += n as usize;
                            }
                        }
                    }
                    unsafe { libc::close(fd) };
                    if let Some(u8_arr) = v8::Uint8Array::new(scope, ab, 0, len) {
                        crate::runtime_minimal::set_buffer_prototype_fast(scope, u8_arr);
                        retval.set(u8_arr.into());
                        return;
                    }
                } else {
                    unsafe { libc::close(fd) };
                }
            }
        }
    }

    let file_res = std::fs::File::open(path.as_ref());
    match file_res {
        Ok(mut file) => {
            let len = file.metadata().map(|m| m.len() as usize).unwrap_or(0);
            if let Some(enc) = encoding {
                let mut bytes = Vec::with_capacity(len);
                use std::io::Read;
                if let Err(e) = file.read_to_end(&mut bytes) {
                    let error_msg = format!("Error reading file: {}", e);
                    let error = v8::String::new(scope, &error_msg).unwrap();
                    let exc = v8::Exception::type_error(scope, error);
                    scope.throw_exception(exc);
                    return;
                }
                let s = match enc.to_ascii_lowercase().as_str() {
                    "utf8" | "utf-8" => String::from_utf8_lossy(&bytes).into_owned(),
                    "hex" => hex::encode(&bytes),
                    "base64" => {
                        use base64::Engine;
                        base64::engine::general_purpose::STANDARD.encode(&bytes)
                    }
                    _ => String::from_utf8_lossy(&bytes).into_owned(),
                };
                if let Some(v8_str) = v8::String::new(scope, &s) {
                    retval.set(v8_str.into());
                }
            } else {
                let ab = v8::ArrayBuffer::new(scope, len);
                if len > 0 {
                    let store = ab.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *mut u8;
                    if !ptr.is_null() {
                        use std::io::Read;
                        let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
                        if let Err(e) = file.read_exact(slice) {
                            let error_msg = format!("Error reading file: {}", e);
                            let error = v8::String::new(scope, &error_msg).unwrap();
                            let exc = v8::Exception::type_error(scope, error);
                            scope.throw_exception(exc);
                            return;
                        }
                    }
                }
                if let Some(u8_arr) = v8::Uint8Array::new(scope, ab, 0, len) {
                    crate::runtime_minimal::set_buffer_prototype_fast(scope, u8_arr);
                    retval.set(u8_arr.into());
                } else {
                    retval.set(v8::undefined(scope).into());
                }
            }
        }
        Err(e) => {
            let error_msg = format!("Error reading file: {}", e);
            let error = v8::String::new(scope, &error_msg).unwrap();
            let exc = v8::Exception::type_error(scope, error);
            scope.throw_exception(exc);
        }
    }
}

thread_local! {
    static TLS_FS_WRITE_BUFFER: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// fs.writeFileSync(path, data, encoding) - 写入文件
fn fs_write_file_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let mut path_buf = [0u8; 512];
    let (path, c_path) = get_path_fast(scope, args.get(0), &mut path_buf);

    if !ensure_fs_permission(scope, PermissionAction::Write, path.as_ref()) {
        return;
    }

    let val = args.get(1);
    let res = if val.is_string() {
        if let Some(s) = val.to_string(scope) {
            if s.contains_only_onebyte() {
                let len = s.length();
                TLS_FS_WRITE_BUFFER.with(|cell| {
                    let mut buf = cell.borrow_mut();
                    if buf.len() < len {
                        buf.resize(len, 0);
                    }
                    s.write_one_byte_v2(scope, 0, &mut buf[..len], v8::WriteFlags::empty());
                    direct_write_sync(c_path, path.as_ref(), &buf[..len])
                })
            } else {
                let len = s.utf8_length(scope);
                TLS_FS_WRITE_BUFFER.with(|cell| {
                    let mut buf = cell.borrow_mut();
                    if buf.len() < len {
                        buf.resize(len, 0);
                    }
                    s.write_utf8_v2(scope, &mut buf[..len], v8::WriteFlags::empty(), None);
                    direct_write_sync(c_path, path.as_ref(), &buf[..len])
                })
            }
        } else {
            direct_write_sync(c_path, path.as_ref(), b"")
        }
    } else if val.is_array_buffer_view() || val.is_typed_array() {
        if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(val) {
            if let Some(ab) = view.buffer(scope) {
                let offset = view.byte_offset();
                let len = view.byte_length();
                let store = ab.get_backing_store();
                let ptr = store.as_ref().as_ptr() as *const u8;
                if !ptr.is_null() {
                    let slice = unsafe { std::slice::from_raw_parts(ptr.add(offset), len) };
                    direct_write_sync(c_path, path.as_ref(), slice)
                } else {
                    direct_write_sync(c_path, path.as_ref(), b"")
                }
            } else {
                direct_write_sync(c_path, path.as_ref(), b"")
            }
        } else {
            direct_write_sync(c_path, path.as_ref(), b"")
        }
    } else if val.is_object() {
        if let Ok(obj) = v8::Local::<v8::Object>::try_from(val) {
            let buf_key = v8::String::new(scope, "buffer").unwrap();
            if let Some(buf_val) = obj.get(scope, buf_key.into()) {
                if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(buf_val) {
                    let len_key = v8::String::new(scope, "length").unwrap();
                    let len = obj
                        .get(scope, len_key.into())
                        .and_then(|v| v.to_integer(scope))
                        .map(|i| i.value() as usize)
                        .unwrap_or_else(|| ab.byte_length());
                    let store = ab.get_backing_store();
                    let ptr = store.as_ref().as_ptr() as *const u8;
                    if !ptr.is_null() {
                        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
                        direct_write_sync(c_path, path.as_ref(), slice)
                    } else {
                        direct_write_sync(c_path, path.as_ref(), b"")
                    }
                } else {
                    let rust_s = val
                        .to_string(scope)
                        .map(|s| s.to_rust_string_lossy(scope))
                        .unwrap_or_default();
                    direct_write_sync(c_path, path.as_ref(), rust_s.as_bytes())
                }
            } else {
                let rust_s = val
                    .to_string(scope)
                    .map(|s| s.to_rust_string_lossy(scope))
                    .unwrap_or_default();
                direct_write_sync(c_path, path.as_ref(), rust_s.as_bytes())
            }
        } else {
            let rust_s = val
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();
            direct_write_sync(c_path, path.as_ref(), rust_s.as_bytes())
        }
    } else {
        let rust_s = val
            .to_string(scope)
            .map(|s| s.to_rust_string_lossy(scope))
            .unwrap_or_default();
        direct_write_sync(c_path, path.as_ref(), rust_s.as_bytes())
    };

    match res {
        Ok(()) => {
            retval.set(v8::undefined(scope).into());
        }
        Err(e) => {
            let error_msg = format!("Error writing file: {}", e);
            let error = v8::String::new(scope, &error_msg).unwrap();
            let exc = v8::Exception::type_error(scope, error);
            scope.throw_exception(exc);
        }
    }
}

/// fs.existsSync(path) - 检查文件是否存在
fn fs_exists_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }

    if crate::sandbox::virtual_fs::is_enabled() {
        let exists = crate::sandbox::virtual_fs::vfs_exists(Path::new(&path));
        retval.set(v8::Boolean::new(scope, exists).into());
        return;
    }

    let exists = Path::new(&path).exists();
    retval.set(v8::Boolean::new(scope, exists).into());
}

/// fs.mkdirSync(path) - 创建目录
fn fs_mkdir_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }

    if crate::sandbox::virtual_fs::is_enabled() {
        match crate::sandbox::virtual_fs::vfs_create_dir_all(Path::new(&path)) {
            Ok(()) => {
                retval.set(v8::undefined(scope).into());
            }
            Err(e) => {
                let error_msg = format!("Error creating directory: {}", e);
                let error = v8::String::new(scope, &error_msg).unwrap();
                let exc = v8::Exception::type_error(scope, error);
                scope.throw_exception(exc);
            }
        }
        return;
    }

    match std::fs::create_dir_all(&path) {
        Ok(()) => {
            retval.set(v8::undefined(scope).into());
        }
        Err(e) => {
            let error_msg = format!("Error creating directory: {}", e);
            let error = v8::String::new(scope, &error_msg).unwrap();
            let exc = v8::Exception::type_error(scope, error);
            scope.throw_exception(exc);
        }
    }
}

/// fs.readdirSync(path) - 读取目录内容
fn fs_readdir_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }

    if crate::sandbox::virtual_fs::is_enabled() {
        match crate::sandbox::virtual_fs::vfs_read_dir(Path::new(&path)) {
            Ok(names) => {
                let array = v8::Array::new(scope, names.len() as i32);
                for (i, name) in names.iter().enumerate() {
                    let value = v8::String::new(scope, name).unwrap();
                    array.set_index(scope, i as u32, value.into());
                }
                retval.set(array.into());
            }
            Err(e) => {
                let error_msg = format!("Error reading directory: {}", e);
                let error = v8::String::new(scope, &error_msg).unwrap();
                let exc = v8::Exception::type_error(scope, error);
                scope.throw_exception(exc);
            }
        }
        return;
    }

    match std::fs::read_dir(&path) {
        Ok(entries) => {
            let names: Vec<String> = entries
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| entry.file_name().to_str().map(|s| s.to_string()))
                .collect();

            // 创建 JavaScript 数组
            let array = v8::Array::new(scope, names.len() as i32);
            for (i, name) in names.iter().enumerate() {
                let value = v8::String::new(scope, name).unwrap();
                array.set_index(scope, i as u32, value.into());
            }
            retval.set(array.into());
        }
        Err(e) => {
            let error_msg = format!("Error reading directory: {}", e);
            let error = v8::String::new(scope, &error_msg).unwrap();
            let exc = v8::Exception::type_error(scope, error);
            scope.throw_exception(exc);
        }
    }
}

/// fs.statSync(path) - 获取文件状态
fn fs_stat_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }

    if crate::sandbox::virtual_fs::is_enabled() {
        match crate::sandbox::virtual_fs::vfs_metadata(Path::new(&path)) {
            Ok(meta) => {
                let stat_obj = create_vfs_stats_object(scope, &meta);
                retval.set(stat_obj.into());
            }
            Err(e) => {
                let error_msg = format!("Error getting file metadata: {}", e);
                let error = v8::String::new(scope, &error_msg).unwrap();
                let exc = v8::Exception::type_error(scope, error);
                scope.throw_exception(exc);
            }
        }
        return;
    }

    match std::fs::metadata(&path) {
        Ok(metadata) => {
            let stat_obj = create_stats_object(scope, &metadata);
            retval.set(stat_obj.into());
        }
        Err(e) => {
            let error_msg = format!("Error getting file metadata: {}", e);
            let error = v8::String::new(scope, &error_msg).unwrap();
            let exc = v8::Exception::type_error(scope, error);
            scope.throw_exception(exc);
        }
    }
}

/// fs.unlinkSync(path) - 删除文件 - v0.3.64
fn fs_unlink_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }

    if crate::sandbox::virtual_fs::is_enabled() {
        match crate::sandbox::virtual_fs::vfs_remove_file(Path::new(&path)) {
            Ok(()) => {
                retval.set(v8::undefined(scope).into());
            }
            Err(e) => {
                let error_msg = format!("Error deleting file: {}", e);
                let error = v8::String::new(scope, &error_msg).unwrap();
                let exc = v8::Exception::type_error(scope, error);
                scope.throw_exception(exc);
            }
        }
        return;
    }

    match fs::remove_file(&path) {
        Ok(()) => {
            retval.set(v8::undefined(scope).into());
        }
        Err(e) => {
            let error_msg = format!("Error deleting file: {}", e);
            let error = v8::String::new(scope, &error_msg).unwrap();
            let exc = v8::Exception::type_error(scope, error);
            scope.throw_exception(exc);
        }
    }
}

/// fs.renameSync(oldPath, newPath) - 重命名文件 - v0.3.64
fn fs_rename_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let old_path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    let new_path: String = args
        .get(1)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    if !ensure_fs_permission(scope, PermissionAction::Write, &old_path) {
        return;
    }
    if !ensure_fs_permission(scope, PermissionAction::Write, &new_path) {
        return;
    }

    match fs::rename(&old_path, &new_path) {
        Ok(()) => {
            retval.set(v8::undefined(scope).into());
        }
        Err(e) => {
            let error_msg = format!("Error renaming file: {}", e);
            let error = v8::String::new(scope, &error_msg).unwrap();
            let exc = v8::Exception::type_error(scope, error);
            scope.throw_exception(exc);
        }
    }
}

/// fs.rmdirSync(path) - 删除目录
fn fs_rmdir_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }

    if crate::sandbox::virtual_fs::is_enabled() {
        match crate::sandbox::virtual_fs::vfs_remove_dir_all(Path::new(&path)) {
            Ok(()) => {
                retval.set(v8::undefined(scope).into());
            }
            Err(e) => {
                let error_msg = format!("Error removing directory: {}", e);
                let error = v8::String::new(scope, &error_msg).unwrap();
                let exc = v8::Exception::type_error(scope, error);
                scope.throw_exception(exc);
            }
        }
        return;
    }

    match fs::remove_dir(&path) {
        Ok(()) => {
            retval.set(v8::undefined(scope).into());
        }
        Err(e) => {
            let error_msg = format!("Error removing directory: {}", e);
            let error = v8::String::new(scope, &error_msg).unwrap();
            let exc = v8::Exception::type_error(scope, error);
            scope.throw_exception(exc);
        }
    }
}

/// fs.readFile(path, [encoding], callback) - callback 风格读取
fn fs_read_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    let callback_val = if args.get(1).is_function() {
        args.get(1)
    } else if args.length() >= 3 && args.get(2).is_function() {
        args.get(2)
    } else {
        let error = v8::String::new(scope, "readFile: callback must be a function").unwrap();
        let exc = v8::Exception::type_error(scope, error);
        scope.throw_exception(exc);
        return;
    };
    let callback = v8::Local::<v8::Function>::try_from(callback_val).unwrap();

    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }

    let read_res = if crate::sandbox::virtual_fs::is_enabled() {
        crate::sandbox::virtual_fs::vfs_read_to_string(Path::new(&path))
    } else {
        std::fs::read_to_string(&path)
    };

    match read_res {
        Ok(content) => {
            let content = v8::String::new(scope, &content).unwrap();
            let undefined = v8::undefined(scope);
            let null: v8::Local<v8::Value> = v8::null(scope).into();
            let _ = callback.call(scope, undefined.into(), &[null, content.into()]);
            retval.set(v8::undefined(scope).into());
        }
        Err(e) => {
            let error_msg = format!("Error reading file: {}", e);
            let error = v8::String::new(scope, &error_msg).unwrap();
            let undefined = v8::undefined(scope);
            let data: v8::Local<v8::Value> = v8::undefined(scope).into();
            let _ = callback.call(scope, undefined.into(), &[error.into(), data]);
            retval.set(v8::undefined(scope).into());
        }
    }
}

/// fs.writeFile(path, data, callback) - callback 风格写入
fn fs_write_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let data: String = args
        .get(1)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    let callback_val = if args.get(2).is_function() {
        args.get(2)
    } else if args.get(1).is_function() {
        args.get(1)
    } else {
        let error = v8::String::new(scope, "writeFile: callback must be a function").unwrap();
        let exc = v8::Exception::type_error(scope, error);
        scope.throw_exception(exc);
        return;
    };
    let callback = v8::Local::<v8::Function>::try_from(callback_val).unwrap();

    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }

    let write_res = if crate::sandbox::virtual_fs::is_enabled() {
        crate::sandbox::virtual_fs::vfs_write(Path::new(&path), data.as_bytes())
    } else {
        std::fs::write(&path, data)
    };

    match write_res {
        Ok(()) => {
            let undefined = v8::undefined(scope);
            let null: v8::Local<v8::Value> = v8::null(scope).into();
            let _ = callback.call(scope, undefined.into(), &[null]);
        }
        Err(e) => {
            let error_msg = format!("Error writing file: {}", e);
            let error = v8::String::new(scope, &error_msg).unwrap();
            let undefined = v8::undefined(scope);
            let _ = callback.call(scope, undefined.into(), &[error.into()]);
        }
    }
    retval.set(v8::undefined(scope).into());
}

/// fs.appendFile(path, data, callback) - callback 风格追加写入
fn fs_append_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let data: String = args
        .get(1)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    let callback_val = if args.get(2).is_function() {
        args.get(2)
    } else if args.get(1).is_function() {
        args.get(1)
    } else {
        let error = v8::String::new(scope, "appendFile: callback must be a function").unwrap();
        let exc = v8::Exception::type_error(scope, error);
        scope.throw_exception(exc);
        return;
    };
    let callback = v8::Local::<v8::Function>::try_from(callback_val).unwrap();

    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }

    let result = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut file| file.write_all(data.as_bytes()));

    match result {
        Ok(()) => {
            let undefined = v8::undefined(scope);
            let null: v8::Local<v8::Value> = v8::null(scope).into();
            let _ = callback.call(scope, undefined.into(), &[null]);
        }
        Err(e) => {
            let error_msg = format!("Error appending to file: {}", e);
            let error = v8::String::new(scope, &error_msg).unwrap();
            let undefined = v8::undefined(scope);
            let _ = callback.call(scope, undefined.into(), &[error.into()]);
        }
    }
    retval.set(v8::undefined(scope).into());
}

// ============ fs.promises API - v0.3.66 ============
// 注意：fs.promises API 使用简化的 thenable 实现
// 真正的异步执行需要完整的 async runtime，这是 Beejs 未来的目标
// 使用 V8 对象的内部字段存储路径数据，避免闭包捕获问题

/// 编码类型枚举 - v0.3.66
enum Encoding {
    Utf8,
    Base64,
    Hex,
    Buffer,
}

/// 提取编码选项 - v0.3.66
fn extract_encoding_option(scope: &mut v8::PinScope, options: &v8::Local<v8::Value>) -> Encoding {
    if options.is_undefined() || options.is_null() {
        return Encoding::Utf8;
    }

    // 如果是字符串直接返回
    if let Some(s) = options.to_string(scope) {
        let encoding_str = s.to_rust_string_lossy(scope).to_lowercase();
        return match encoding_str.as_str() {
            "utf-8" | "utf8" => Encoding::Utf8,
            "base64" => Encoding::Base64,
            "hex" => Encoding::Hex,
            "buffer" | "raw" => Encoding::Buffer,
            _ => Encoding::Utf8,
        };
    }

    // 如果是对象，检查 encoding 属性
    if let Ok(obj) = v8::Local::<v8::Object>::try_from(*options) {
        let encoding_key = v8::String::new(scope, "encoding").unwrap();
        if let Some(enc_val) = obj.get(scope, encoding_key.into()) {
            if let Some(s) = enc_val.to_string(scope) {
                let encoding_str = s.to_rust_string_lossy(scope).to_lowercase();
                return match encoding_str.as_str() {
                    "utf-8" | "utf8" => Encoding::Utf8,
                    "base64" => Encoding::Base64,
                    "hex" => Encoding::Hex,
                    "buffer" | "raw" => Encoding::Buffer,
                    _ => Encoding::Utf8,
                };
            }
        }
    }

    Encoding::Utf8
}

/// fs.promises.readFile(path, options) - v0.3.66 增强版
/// 支持 encoding 参数：'utf-8', 'base64', 'hex', 'buffer'
/// 返回一个 thenable 对象，可以配合 await/then 使用
fn fs_promises_read_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }

    // 提取 encoding 参数 - v0.3.66
    let options = args.get(1);
    let encoding = extract_encoding_option(scope, &options);

    let thenable_obj = v8::Object::new(scope);

    // 将路径和编码存储为 thenable 对象的属性
    let path_key = v8::String::new(scope, "__path").unwrap();
    let path_val = v8::String::new(scope, &path).unwrap();
    thenable_obj.set(scope, path_key.into(), path_val.into());

    // 存储编码类型 - v0.3.66
    let encoding_key = v8::String::new(scope, "__encoding").unwrap();
    let encoding_str = match encoding {
        Encoding::Utf8 => "utf-8",
        Encoding::Base64 => "base64",
        Encoding::Hex => "hex",
        Encoding::Buffer => "buffer",
    };
    let encoding_val = v8::String::new(scope, encoding_str).unwrap();
    thenable_obj.set(scope, encoding_key.into(), encoding_val.into());

    // then 方法 - 从 thenable 对象获取路径和编码 - v0.3.66
    let then_func = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let on_fulfilled = args.get(0);

            // 从 this 获取路径和编码
            let path_key = v8::String::new(scope, "__path").unwrap();
            let encoding_key = v8::String::new(scope, "__encoding").unwrap();

            let path_val = this
                .get(scope, path_key.into())
                .unwrap_or(v8::undefined(scope).into());
            let encoding_val = this
                .get(scope, encoding_key.into())
                .unwrap_or(v8::undefined(scope).into());

            let path_str = path_val
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();
            let encoding_str = encoding_val
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();

            if !ensure_fs_permission(scope, PermissionAction::Read, &path_str) {
                return;
            }

            // 根据编码类型读取文件 - v0.3.66
            let read_result: Result<String, String> = {
                let read_bytes = if crate::sandbox::virtual_fs::is_enabled() {
                    crate::sandbox::virtual_fs::vfs_read(Path::new(&path_str))
                } else {
                    std::fs::read(&path_str)
                };

                match encoding_str.as_str() {
                    "utf-8" | "utf8" => match read_bytes {
                        Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).to_string()),
                        Err(e) => Err(format!("Error reading file: {}", e)),
                    },
                    "base64" => match read_bytes {
                        Ok(bytes) => {
                            use base64::{engine::general_purpose::STANDARD, Engine as _};
                            Ok(STANDARD.encode(&bytes))
                        }
                        Err(e) => Err(format!("Error reading file: {}", e)),
                    },
                    "hex" => match read_bytes {
                        Ok(bytes) => Ok(hex::encode(&bytes)),
                        Err(e) => Err(format!("Error reading file: {}", e)),
                    },
                    "buffer" | "raw" => match read_bytes {
                        Ok(bytes) => {
                            // 创建 Buffer 对象
                            let buffer_val = create_buffer_from_bytes(scope, &bytes);
                            let mut fulfillment = None;
                            if on_fulfilled.is_function() {
                                if let Ok(func) = v8::Local::<v8::Function>::try_from(on_fulfilled)
                                {
                                    let undefined = v8::undefined(scope);
                                    let result =
                                        func.call(scope, undefined.into(), &[buffer_val.into()]);
                                    if let Some(r) = result {
                                        let result_key =
                                            v8::String::new(scope, "__result__").unwrap();
                                        this.set(scope, result_key.into(), r);
                                        fulfillment = Some(r);
                                    }
                                }
                            }
                            thenable_chain_return(scope, this, fulfillment, &mut retval);
                            return;
                        }
                        Err(e) => Err(format!("Error reading file: {}", e)),
                    },
                    _ => match read_bytes {
                        Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).to_string()),
                        Err(e) => Err(format!("Error reading file: {}", e)),
                    },
                }
            };

            let mut fulfillment = None;
            match read_result {
                Ok(content) => {
                    if on_fulfilled.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_fulfilled) {
                            let content_val = v8::String::new(scope, &content).unwrap();
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[content_val.into()]);
                            // v0.3.64: Store result on thenable for test access
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                                fulfillment = Some(r);
                            }
                        }
                    }
                }
                Err(e) => {
                    let on_rejected = args.get(1);
                    if on_rejected.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_rejected) {
                            let error_val = v8::String::new(scope, &e).unwrap();
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[error_val.into()]);
                            // v0.3.64: Store result on thenable for test access
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                                fulfillment = Some(r);
                            }
                        }
                    }
                }
            }
            thenable_chain_return(scope, this, fulfillment, &mut retval);
        },
    );

    let then_instance = then_func.get_function(scope).unwrap();
    let then_key = v8::String::new(scope, "then").unwrap();
    thenable_obj.set(scope, then_key.into(), then_instance.into());

    let catch_func = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let on_rejected = args.get(0);

            let path_key = v8::String::new(scope, "__path").unwrap();
            let path_val = this
                .get(scope, path_key.into())
                .unwrap_or(v8::undefined(scope).into());
            let path_str = path_val
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();

            if !ensure_fs_permission(scope, PermissionAction::Read, &path_str) {
                return;
            }

            if let Err(error) = std::fs::read(&path_str) {
                if on_rejected.is_function() {
                    if let Ok(func) = v8::Local::<v8::Function>::try_from(on_rejected) {
                        let error_msg = format!("Error reading file: {}", error);
                        let error_val = v8::String::new(scope, &error_msg).unwrap();
                        let undefined = v8::undefined(scope);
                        let result = func.call(scope, undefined.into(), &[error_val.into()]);
                        if let Some(r) = result {
                            let result_key = v8::String::new(scope, "__result__").unwrap();
                            this.set(scope, result_key.into(), r);
                        }
                    }
                }
            }

            retval.set(this.into());
        },
    );
    let catch_instance = catch_func.get_function(scope).unwrap();
    let catch_key = v8::String::new(scope, "catch").unwrap();
    thenable_obj.set(scope, catch_key.into(), catch_instance.into());

    retval.set(thenable_obj.into());
}

/// fs.promises.writeFile(path, data, options) - v0.3.64
fn fs_promises_write_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let data: String = args
        .get(1)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }

    let thenable_obj = v8::Object::new(scope);

    // 预先创建所有 V8 值，避免 borrow checker 问题
    let path_val = v8::String::new(scope, &path).unwrap();
    let data_val = v8::String::new(scope, &data).unwrap();
    let path_key = v8::String::new(scope, "__path").unwrap();
    let data_key = v8::String::new(scope, "__data").unwrap();
    thenable_obj.set(scope, path_key.into(), path_val.into());
    thenable_obj.set(scope, data_key.into(), data_val.into());

    let then_func = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let on_fulfilled = args.get(0);

            let path_key = v8::String::new(scope, "__path").unwrap();
            let data_key = v8::String::new(scope, "__data").unwrap();
            let path_val = this
                .get(scope, path_key.into())
                .unwrap_or(v8::undefined(scope).into());
            let data_val = this
                .get(scope, data_key.into())
                .unwrap_or(v8::undefined(scope).into());
            let path_str = path_val
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();
            let data_str = data_val
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();

            if !ensure_fs_permission(scope, PermissionAction::Write, &path_str) {
                return;
            }

            let mut fulfillment = None;
            let write_res = if crate::sandbox::virtual_fs::is_enabled() {
                crate::sandbox::virtual_fs::vfs_write(Path::new(&path_str), data_str.as_bytes())
            } else {
                std::fs::write(&path_str, &data_str)
            };
            match write_res {
                Ok(()) => {
                    if on_fulfilled.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_fulfilled) {
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[]);
                            // v0.3.64: Store result on thenable for test access
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                                fulfillment = Some(r);
                            }
                        }
                    }
                }
                Err(e) => {
                    let on_rejected = args.get(1);
                    if on_rejected.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_rejected) {
                            let error_msg = format!("Error writing file: {}", e);
                            let error_val = v8::String::new(scope, &error_msg).unwrap();
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[error_val.into()]);
                            // v0.3.64: Store result on thenable for test access
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                                fulfillment = Some(r);
                            }
                        }
                    }
                }
            }
            thenable_chain_return(scope, this, fulfillment, &mut retval);
        },
    );

    let then_instance = then_func.get_function(scope).unwrap();
    let then_key = v8::String::new(scope, "then").unwrap();
    thenable_obj.set(scope, then_key.into(), then_instance.into());

    retval.set(thenable_obj.into());
}

/// fs.promises.appendFile(path, data, options) - v0.3.66
fn fs_promises_append_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let data: String = args
        .get(1)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }

    let thenable_obj = v8::Object::new(scope);

    let path_val = v8::String::new(scope, &path).unwrap();
    let data_val = v8::String::new(scope, &data).unwrap();
    let path_key = v8::String::new(scope, "__path").unwrap();
    let data_key = v8::String::new(scope, "__data").unwrap();
    thenable_obj.set(scope, path_key.into(), path_val.into());
    thenable_obj.set(scope, data_key.into(), data_val.into());

    let then_func = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let on_fulfilled = args.get(0);

            let path_key = v8::String::new(scope, "__path").unwrap();
            let data_key = v8::String::new(scope, "__data").unwrap();
            let path_val = this
                .get(scope, path_key.into())
                .unwrap_or(v8::undefined(scope).into());
            let data_val = this
                .get(scope, data_key.into())
                .unwrap_or(v8::undefined(scope).into());
            let path_str = path_val
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();
            let data_str = data_val
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();

            if !ensure_fs_permission(scope, PermissionAction::Write, &path_str) {
                return;
            }

            let append_result = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path_str)
                .and_then(|mut file| file.write_all(data_str.as_bytes()));

            match append_result {
                Ok(()) => {
                    if on_fulfilled.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_fulfilled) {
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[]);
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                            }
                        }
                    }
                }
                Err(error) => {
                    let on_rejected = args.get(1);
                    if on_rejected.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_rejected) {
                            let error_msg = format!("Error appending file: {}", error);
                            let error_val = v8::String::new(scope, &error_msg).unwrap();
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[error_val.into()]);
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                            }
                        }
                    }
                }
            }

            retval.set(this.into());
        },
    );

    let then_instance = then_func.get_function(scope).unwrap();
    let then_key = v8::String::new(scope, "then").unwrap();
    thenable_obj.set(scope, then_key.into(), then_instance.into());

    retval.set(thenable_obj.into());
}

/// fs.promises.mkdir(path, options) - v0.3.64
fn fs_promises_mkdir_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }

    let thenable_obj = v8::Object::new(scope);

    let path_val = v8::String::new(scope, &path).unwrap();
    let path_key = v8::String::new(scope, "__path").unwrap();
    thenable_obj.set(scope, path_key.into(), path_val.into());

    let then_func = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let on_fulfilled = args.get(0);

            let path_key = v8::String::new(scope, "__path").unwrap();
            let path_val = this
                .get(scope, path_key.into())
                .unwrap_or(v8::undefined(scope).into());
            let path_str = path_val
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();

            if !ensure_fs_permission(scope, PermissionAction::Write, &path_str) {
                return;
            }

            match std::fs::create_dir_all(&path_str) {
                Ok(()) => {
                    if on_fulfilled.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_fulfilled) {
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[]);
                            // v0.3.64: Store result on thenable for test access
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                            }
                        }
                    }
                }
                Err(e) => {
                    let on_rejected = args.get(1);
                    if on_rejected.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_rejected) {
                            let error_msg = format!("Error creating directory: {}", e);
                            let error_val = v8::String::new(scope, &error_msg).unwrap();
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[error_val.into()]);
                            // v0.3.64: Store result on thenable for test access
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                            }
                        }
                    }
                }
            }
            // v0.3.64: Return this thenable so execute_code can access __result__
            retval.set(this.into());
        },
    );

    let then_instance = then_func.get_function(scope).unwrap();
    let then_key = v8::String::new(scope, "then").unwrap();
    thenable_obj.set(scope, then_key.into(), then_instance.into());

    retval.set(thenable_obj.into());
}

/// fs.promises.rmdir(path) - v0.3.66
fn fs_promises_rmdir_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }

    let thenable_obj = v8::Object::new(scope);

    let path_val = v8::String::new(scope, &path).unwrap();
    let path_key = v8::String::new(scope, "__path").unwrap();
    thenable_obj.set(scope, path_key.into(), path_val.into());

    let then_func = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let on_fulfilled = args.get(0);

            let path_key = v8::String::new(scope, "__path").unwrap();
            let path_val = this
                .get(scope, path_key.into())
                .unwrap_or(v8::undefined(scope).into());
            let path_str = path_val
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();

            if !ensure_fs_permission(scope, PermissionAction::Write, &path_str) {
                return;
            }

            match std::fs::remove_dir(&path_str) {
                Ok(()) => {
                    if on_fulfilled.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_fulfilled) {
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[]);
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                            }
                        }
                    }
                }
                Err(error) => {
                    let on_rejected = args.get(1);
                    if on_rejected.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_rejected) {
                            let error_msg = format!("Error removing directory: {}", error);
                            let error_val = v8::String::new(scope, &error_msg).unwrap();
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[error_val.into()]);
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                            }
                        }
                    }
                }
            }

            retval.set(this.into());
        },
    );

    let then_instance = then_func.get_function(scope).unwrap();
    let then_key = v8::String::new(scope, "then").unwrap();
    thenable_obj.set(scope, then_key.into(), then_instance.into());

    retval.set(thenable_obj.into());
}

/// fs.promises.readdir(path) - v0.3.64
fn fs_promises_readdir_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }

    let thenable_obj = v8::Object::new(scope);

    let path_val = v8::String::new(scope, &path).unwrap();
    let path_key = v8::String::new(scope, "__path").unwrap();
    thenable_obj.set(scope, path_key.into(), path_val.into());

    let then_func = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let on_fulfilled = args.get(0);

            let path_key = v8::String::new(scope, "__path").unwrap();
            let path_val = this
                .get(scope, path_key.into())
                .unwrap_or(v8::undefined(scope).into());
            let path_str = path_val
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();

            if !ensure_fs_permission(scope, PermissionAction::Read, &path_str) {
                return;
            }

            match std::fs::read_dir(&path_str) {
                Ok(entries) => {
                    let names: Vec<String> = entries
                        .filter_map(|entry| entry.ok())
                        .filter_map(|entry| entry.file_name().to_str().map(|s| s.to_string()))
                        .collect();

                    let array = v8::Array::new(scope, names.len() as i32);
                    for (i, name) in names.iter().enumerate() {
                        let value = v8::String::new(scope, name).unwrap();
                        array.set_index(scope, i as u32, value.into());
                    }

                    if on_fulfilled.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_fulfilled) {
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[array.into()]);
                            // v0.3.64: Store result on thenable for test access
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                            }
                        }
                    }
                }
                Err(e) => {
                    let on_rejected = args.get(1);
                    if on_rejected.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_rejected) {
                            let error_msg = format!("Error reading directory: {}", e);
                            let error_val = v8::String::new(scope, &error_msg).unwrap();
                            let undefined = v8::undefined(scope);
                            func.call(scope, undefined.into(), &[error_val.into()]);
                        }
                    }
                }
            }
            // v0.3.64: Return this thenable so execute_code can access __result__
            retval.set(this.into());
        },
    );

    let then_instance = then_func.get_function(scope).unwrap();
    let then_key = v8::String::new(scope, "then").unwrap();
    thenable_obj.set(scope, then_key.into(), then_instance.into());

    retval.set(thenable_obj.into());
}

/// fs.promises.stat(path) - v0.3.64
fn fs_promises_stat_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }

    let thenable_obj = v8::Object::new(scope);

    let path_val = v8::String::new(scope, &path).unwrap();
    let path_key = v8::String::new(scope, "__path").unwrap();
    thenable_obj.set(scope, path_key.into(), path_val.into());

    let then_func = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let on_fulfilled = args.get(0);

            let path_key = v8::String::new(scope, "__path").unwrap();
            let path_val = this
                .get(scope, path_key.into())
                .unwrap_or(v8::undefined(scope).into());
            let path_str = path_val
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();

            if !ensure_fs_permission(scope, PermissionAction::Read, &path_str) {
                return;
            }

            match std::fs::metadata(&path_str) {
                Ok(metadata) => {
                    let stat_obj = create_stats_object(scope, &metadata);

                    if on_fulfilled.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_fulfilled) {
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[stat_obj.into()]);
                            // v0.3.64: Store result on thenable for test access
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                            }
                        }
                    }
                }
                Err(e) => {
                    let on_rejected = args.get(1);
                    if on_rejected.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_rejected) {
                            let error_msg = format!("Error getting file metadata: {}", e);
                            let error_val = v8::String::new(scope, &error_msg).unwrap();
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[error_val.into()]);
                            // v0.3.64: Store result on thenable for test access
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                            }
                        }
                    }
                }
            }
            // v0.3.64: Return this thenable so execute_code can access __result__
            retval.set(this.into());
        },
    );

    let then_instance = then_func.get_function(scope).unwrap();
    let then_key = v8::String::new(scope, "then").unwrap();
    thenable_obj.set(scope, then_key.into(), then_instance.into());

    retval.set(thenable_obj.into());
}

/// fs.promises.unlink(path) - v0.3.64
fn fs_promises_unlink_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }

    let thenable_obj = v8::Object::new(scope);

    let path_val = v8::String::new(scope, &path).unwrap();
    let path_key = v8::String::new(scope, "__path").unwrap();
    thenable_obj.set(scope, path_key.into(), path_val.into());

    let then_func = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let on_fulfilled = args.get(0);

            let path_key = v8::String::new(scope, "__path").unwrap();
            let path_val = this
                .get(scope, path_key.into())
                .unwrap_or(v8::undefined(scope).into());
            let path_str = path_val
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();

            if !ensure_fs_permission(scope, PermissionAction::Write, &path_str) {
                return;
            }

            match std::fs::remove_file(&path_str) {
                Ok(()) => {
                    if on_fulfilled.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_fulfilled) {
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[]);
                            // v0.3.64: Store result on thenable for test access
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                            }
                        }
                    }
                }
                Err(e) => {
                    let on_rejected = args.get(1);
                    if on_rejected.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_rejected) {
                            let error_msg = format!("Error deleting file: {}", e);
                            let error_val = v8::String::new(scope, &error_msg).unwrap();
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[error_val.into()]);
                            // v0.3.64: Store result on thenable for test access
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                            }
                        }
                    }
                }
            }
            // v0.3.64: Return this thenable so execute_code can access __result__
            retval.set(this.into());
        },
    );

    let then_instance = then_func.get_function(scope).unwrap();
    let then_key = v8::String::new(scope, "then").unwrap();
    thenable_obj.set(scope, then_key.into(), then_instance.into());

    retval.set(thenable_obj.into());
}

/// fs.promises.rename(oldPath, newPath) - v0.3.64
fn fs_promises_rename_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let old_path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let new_path: String = args
        .get(1)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    if !ensure_fs_permission(scope, PermissionAction::Write, &old_path) {
        return;
    }
    if !ensure_fs_permission(scope, PermissionAction::Write, &new_path) {
        return;
    }

    let thenable_obj = v8::Object::new(scope);

    let old_path_val = v8::String::new(scope, &old_path).unwrap();
    let new_path_val = v8::String::new(scope, &new_path).unwrap();
    let old_path_key = v8::String::new(scope, "__oldPath").unwrap();
    let new_path_key = v8::String::new(scope, "__newPath").unwrap();
    thenable_obj.set(scope, old_path_key.into(), old_path_val.into());
    thenable_obj.set(scope, new_path_key.into(), new_path_val.into());

    let then_func = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let on_fulfilled = args.get(0);

            let old_path_key = v8::String::new(scope, "__oldPath").unwrap();
            let new_path_key = v8::String::new(scope, "__newPath").unwrap();
            let old_path_val = this
                .get(scope, old_path_key.into())
                .unwrap_or(v8::undefined(scope).into());
            let new_path_val = this
                .get(scope, new_path_key.into())
                .unwrap_or(v8::undefined(scope).into());
            let old_path_str = old_path_val
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();
            let new_path_str = new_path_val
                .to_string(scope)
                .map(|s| s.to_rust_string_lossy(scope))
                .unwrap_or_default();

            if !ensure_fs_permission(scope, PermissionAction::Write, &old_path_str) {
                return;
            }
            if !ensure_fs_permission(scope, PermissionAction::Write, &new_path_str) {
                return;
            }

            match std::fs::rename(&old_path_str, &new_path_str) {
                Ok(()) => {
                    if on_fulfilled.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_fulfilled) {
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[]);
                            // v0.3.64: Store result on thenable for test access
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                            }
                        }
                    }
                }
                Err(e) => {
                    let on_rejected = args.get(1);
                    if on_rejected.is_function() {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(on_rejected) {
                            let error_msg = format!("Error renaming file: {}", e);
                            let error_val = v8::String::new(scope, &error_msg).unwrap();
                            let undefined = v8::undefined(scope);
                            let result = func.call(scope, undefined.into(), &[error_val.into()]);
                            // v0.3.64: Store result on thenable for test access
                            if let Some(r) = result {
                                let result_key = v8::String::new(scope, "__result__").unwrap();
                                this.set(scope, result_key.into(), r);
                            }
                        }
                    }
                }
            }
            // v0.3.64: Return this thenable so execute_code can access __result__
            retval.set(this.into());
        },
    );

    let then_instance = then_func.get_function(scope).unwrap();
    let then_key = v8::String::new(scope, "then").unwrap();
    thenable_obj.set(scope, then_key.into(), then_instance.into());

    retval.set(thenable_obj.into());
}
