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

/// Copy file bytes into a Uint8Array that uses Buffer.prototype.
/// Callers index the result and use Buffer.toString; a length-only object drops the bytes.
fn create_buffer_from_bytes<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    bytes: &[u8],
) -> v8::Local<'a, v8::Value> {
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
        u8_arr.into()
    } else {
        v8::undefined(scope).into()
    }
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

fn stats_is_symbolic_link_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let is_link = get_stats_flag(scope, args.this(), "__isSymbolicLink");
    retval.set(v8::Boolean::new(scope, is_link).into());
}

fn system_time_ms(time: std::time::SystemTime) -> f64 {
    match time.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs_f64() * 1000.0,
        Err(err) => -(err.duration().as_secs_f64() * 1000.0),
    }
}

fn js_date<'a>(scope: &mut v8::PinScope<'a, '_>, ms: f64) -> v8::Local<'a, v8::Value> {
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, "Date").unwrap();
    if let Some(ctor_val) = global.get(scope, key.into()) {
        if let Ok(ctor) = v8::Local::<v8::Function>::try_from(ctor_val) {
            let arg = v8::Number::new(scope, ms);
            if let Some(date) = ctor.new_instance(scope, &[arg.into()]) {
                return date.into();
            }
        }
    }
    v8::Number::new(scope, ms).into()
}

fn set_number_prop(scope: &mut v8::PinScope, obj: v8::Local<v8::Object>, key: &str, value: f64) {
    let key = v8::String::new(scope, key).unwrap();
    let value = v8::Number::new(scope, value);
    obj.set(scope, key.into(), value.into());
}

fn set_date_prop(scope: &mut v8::PinScope, obj: v8::Local<v8::Object>, key: &str, ms: f64) {
    let key = v8::String::new(scope, key).unwrap();
    let value = js_date(scope, ms);
    obj.set(scope, key.into(), value);
}

fn attach_stats_methods(scope: &mut v8::PinScope, stat_obj: v8::Local<v8::Object>) {
    let is_file_func = v8::FunctionTemplate::new(scope, stats_is_file_callback);
    let is_file_instance = is_file_func.get_function(scope).unwrap();
    let is_file_key = v8::String::new(scope, "isFile").unwrap();
    stat_obj.set(scope, is_file_key.into(), is_file_instance.into());

    let is_dir_func = v8::FunctionTemplate::new(scope, stats_is_directory_callback);
    let is_dir_instance = is_dir_func.get_function(scope).unwrap();
    let is_dir_key = v8::String::new(scope, "isDirectory").unwrap();
    stat_obj.set(scope, is_dir_key.into(), is_dir_instance.into());

    let is_link_func = v8::FunctionTemplate::new(scope, stats_is_symbolic_link_callback);
    let is_link_instance = is_link_func.get_function(scope).unwrap();
    let is_link_key = v8::String::new(scope, "isSymbolicLink").unwrap();
    stat_obj.set(scope, is_link_key.into(), is_link_instance.into());
}

/// `stat` follows links, so `isSymbolicLink()` is false there.
/// `lstat` passes symlink metadata, where `file_type().is_symlink()` is true.
pub(crate) fn create_stats_object<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    metadata: &std::fs::Metadata,
) -> v8::Local<'a, v8::Object> {
    let stat_obj = v8::Object::new(scope);
    let is_symlink = metadata.file_type().is_symlink();

    let is_file_state_key = v8::String::new(scope, "__isFile").unwrap();
    let is_file_state = v8::Boolean::new(scope, metadata.is_file() && !is_symlink);
    stat_obj.set(scope, is_file_state_key.into(), is_file_state.into());

    let is_dir_state_key = v8::String::new(scope, "__isDirectory").unwrap();
    let is_dir_state = v8::Boolean::new(scope, metadata.is_dir() && !is_symlink);
    stat_obj.set(scope, is_dir_state_key.into(), is_dir_state.into());

    let is_link_state_key = v8::String::new(scope, "__isSymbolicLink").unwrap();
    let is_link_state = v8::Boolean::new(scope, is_symlink);
    stat_obj.set(scope, is_link_state_key.into(), is_link_state.into());

    attach_stats_methods(scope, stat_obj);

    set_number_prop(scope, stat_obj, "size", metadata.len() as f64);

    let (mode, uid, gid, ctime_ms) = {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let ctime_ms =
                metadata.ctime() as f64 * 1000.0 + (metadata.ctime_nsec() as f64) / 1_000_000.0;
            (
                metadata.mode() as f64,
                metadata.uid() as f64,
                metadata.gid() as f64,
                ctime_ms,
            )
        }
        #[cfg(not(unix))]
        {
            let mode = if metadata.is_dir() {
                0o040755
            } else {
                0o100644
            };
            (
                mode as f64,
                0.0,
                0.0,
                metadata.modified().ok().map(system_time_ms).unwrap_or(0.0),
            )
        }
    };
    set_number_prop(scope, stat_obj, "mode", mode);
    set_number_prop(scope, stat_obj, "uid", uid);
    set_number_prop(scope, stat_obj, "gid", gid);

    let mtime_ms = metadata.modified().ok().map(system_time_ms).unwrap_or(0.0);
    let atime_ms = metadata
        .accessed()
        .ok()
        .map(system_time_ms)
        .unwrap_or(mtime_ms);
    let birth_ms = metadata.created().ok().map(system_time_ms).unwrap_or(0.0);
    set_date_prop(scope, stat_obj, "mtime", mtime_ms);
    set_date_prop(scope, stat_obj, "atime", atime_ms);
    set_date_prop(scope, stat_obj, "ctime", ctime_ms);
    set_date_prop(scope, stat_obj, "birthtime", birth_ms);
    set_number_prop(scope, stat_obj, "mtimeMs", mtime_ms);
    set_number_prop(scope, stat_obj, "atimeMs", atime_ms);
    set_number_prop(scope, stat_obj, "ctimeMs", ctime_ms);
    set_number_prop(scope, stat_obj, "birthtimeMs", birth_ms);

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

    let is_link_state_key = v8::String::new(scope, "__isSymbolicLink").unwrap();
    stat_obj.set(
        scope,
        is_link_state_key.into(),
        v8::Boolean::new(scope, false).into(),
    );
    attach_stats_methods(scope, stat_obj);

    set_number_prop(scope, stat_obj, "size", meta.len as f64);
    let mode = if meta.is_dir { 0o040755 } else { 0o100644 };
    set_number_prop(scope, stat_obj, "mode", mode as f64);
    set_number_prop(scope, stat_obj, "uid", 0.0);
    set_number_prop(scope, stat_obj, "gid", 0.0);
    let now = system_time_ms(std::time::SystemTime::now());
    set_date_prop(scope, stat_obj, "mtime", now);
    set_date_prop(scope, stat_obj, "atime", now);
    set_date_prop(scope, stat_obj, "ctime", now);
    set_date_prop(scope, stat_obj, "birthtime", now);
    set_number_prop(scope, stat_obj, "mtimeMs", now);
    set_number_prop(scope, stat_obj, "atimeMs", now);
    set_number_prop(scope, stat_obj, "ctimeMs", now);
    set_number_prop(scope, stat_obj, "birthtimeMs", now);

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

    let append_sync_func = v8::FunctionTemplate::new(scope, fs_append_file_sync_callback);
    let append_sync_instance = append_sync_func.get_function(scope).unwrap();
    let append_sync_key = v8::String::new(scope, "appendFileSync").unwrap();
    fs_obj.set(scope, append_sync_key.into(), append_sync_instance.into());

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
    install_stable_fs_methods(scope, fs_obj);

    // 设置到全局对象
    let global = context.global(scope);
    let fs_key = v8::String::new(scope, "fs").unwrap();
    global.set(scope, fs_key.into(), fs_obj.into());

    Ok(())
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
        let len = if is_one_byte {
            s.length()
        } else {
            s.utf8_length(scope)
        };
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
    #[cfg_attr(not(unix), allow(unused_variables))] c_path: Option<*const libc::c_char>,
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum FileEncoding {
    Utf8,
    Hex,
    Base64,
    Latin1,
    /// No text encoding: readFile returns a Buffer. Not a Node write encoding.
    Buffer,
}

struct FileIoOptions {
    encoding: FileEncoding,
    flag: Option<String>,
}

impl FileIoOptions {
    fn open_mode(&self, default_append: bool) -> Result<OpenMode, String> {
        match self.flag.as_deref() {
            Some(flag) => OpenMode::parse(flag),
            None if default_append => Ok(OpenMode::append()),
            None => Ok(OpenMode::write()),
        }
    }
}

/// Node `flag` strings beyond the previous `a` / `w` / `x` subset.
#[derive(Clone, Copy, PartialEq, Eq)]
struct OpenMode {
    read: bool,
    write: bool,
    append: bool,
    create: bool,
    truncate: bool,
    exclusive: bool,
}

impl OpenMode {
    fn write() -> Self {
        Self {
            read: false,
            write: true,
            append: false,
            create: true,
            truncate: true,
            exclusive: false,
        }
    }

    fn append() -> Self {
        Self {
            read: false,
            write: true,
            append: true,
            create: true,
            truncate: false,
            exclusive: false,
        }
    }

    fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            append: false,
            create: false,
            truncate: false,
            exclusive: false,
        }
    }

    fn is_plain_write(self) -> bool {
        self.write && self.create && self.truncate && !self.append && !self.exclusive && !self.read
    }

    fn parse(flag: &str) -> Result<Self, String> {
        let flag = flag.to_ascii_lowercase();
        let mode = match flag.as_str() {
            "r" | "rs" => Self::read_only(),
            "r+" | "rs+" => Self {
                read: true,
                write: true,
                append: false,
                create: false,
                truncate: false,
                exclusive: false,
            },
            "w" => Self::write(),
            "wx" | "xw" => Self {
                exclusive: true,
                ..Self::write()
            },
            "w+" => Self {
                read: true,
                ..Self::write()
            },
            "wx+" | "xw+" => Self {
                read: true,
                exclusive: true,
                ..Self::write()
            },
            "a" | "as" => Self::append(),
            "ax" | "xa" => Self {
                exclusive: true,
                ..Self::append()
            },
            "a+" | "as+" => Self {
                read: true,
                ..Self::append()
            },
            "ax+" | "xa+" => Self {
                read: true,
                exclusive: true,
                ..Self::append()
            },
            other => {
                return Err(format!("Unknown file open flag: {other}"));
            }
        };
        Ok(mode)
    }

    fn to_open_options(self) -> std::fs::OpenOptions {
        let mut opts = std::fs::OpenOptions::new();
        opts.read(self.read);
        opts.write(self.write);
        if self.exclusive {
            opts.create_new(true);
        } else if self.create {
            opts.create(true);
        }
        if self.append {
            opts.append(true);
        } else if self.truncate && !self.exclusive {
            opts.truncate(true);
        }
        opts
    }
}

fn file_encoding_from_name(name: &str) -> Result<FileEncoding, String> {
    match name.to_ascii_lowercase().as_str() {
        "utf8" | "utf-8" | "utf8mb4" => Ok(FileEncoding::Utf8),
        "hex" => Ok(FileEncoding::Hex),
        "base64" => Ok(FileEncoding::Base64),
        "latin1" | "binary" | "ascii" => Ok(FileEncoding::Latin1),
        "buffer" | "raw" => Ok(FileEncoding::Buffer),
        other => Err(format!("Unknown encoding: {other}")),
    }
}

fn option_string(
    scope: &mut v8::PinScope,
    obj: v8::Local<v8::Object>,
    key: &str,
) -> Option<String> {
    let key = v8::String::new(scope, key)?;
    let value = obj.get(scope, key.into())?;
    if value.is_undefined() || value.is_null() {
        return None;
    }
    value
        .to_string(scope)
        .map(|text| text.to_rust_string_lossy(scope))
}

/// Node accepts a string encoding or `{ encoding, flag }`.
/// Missing encoding uses `default_encoding` (Buffer for reads, utf8 for writes).
fn parse_file_io_options(
    scope: &mut v8::PinScope,
    value: v8::Local<v8::Value>,
    default_encoding: FileEncoding,
) -> Result<FileIoOptions, String> {
    if value.is_undefined() || value.is_null() {
        return Ok(FileIoOptions {
            encoding: default_encoding,
            flag: None,
        });
    }
    if value.is_string() {
        let name = value
            .to_string(scope)
            .map(|text| text.to_rust_string_lossy(scope))
            .unwrap_or_default();
        return Ok(FileIoOptions {
            encoding: file_encoding_from_name(&name)?,
            flag: None,
        });
    }
    if value.is_object() && !value.is_function() {
        let obj = v8::Local::<v8::Object>::try_from(value)
            .map_err(|_| "Invalid file options".to_string())?;
        let encoding = match option_string(scope, obj, "encoding") {
            Some(name) => file_encoding_from_name(&name)?,
            None => default_encoding,
        };
        return Ok(FileIoOptions {
            encoding,
            flag: option_string(scope, obj, "flag"),
        });
    }
    Err("Invalid file options".to_string())
}

fn throw_coded_type_error(scope: &mut v8::PinScope, code: &str, message: &str) {
    let message = v8::String::new(scope, message).unwrap();
    let exception = v8::Exception::type_error(scope, message);
    if let Some(obj) = exception.to_object(scope) {
        let code_key = v8::String::new(scope, "code").unwrap();
        let code_val = v8::String::new(scope, code).unwrap();
        obj.set(scope, code_key.into(), code_val.into());
    }
    scope.throw_exception(exception);
}

fn io_error_code(err: &std::io::Error) -> &'static str {
    if let Some(code) = err.raw_os_error() {
        let mapped = match code {
            libc::ENOENT => "ENOENT",
            libc::EACCES | libc::EPERM => "EACCES",
            libc::EEXIST => "EEXIST",
            libc::EINVAL => "EINVAL",
            libc::EISDIR => "EISDIR",
            libc::ENOTDIR => "ENOTDIR",
            libc::ENOTEMPTY => "ENOTEMPTY",
            libc::EBADF => "EBADF",
            libc::ENAMETOOLONG => "ENAMETOOLONG",
            _ => "",
        };
        if !mapped.is_empty() {
            return mapped;
        }
    }
    match err.kind() {
        std::io::ErrorKind::NotFound => "ENOENT",
        std::io::ErrorKind::PermissionDenied => "EACCES",
        std::io::ErrorKind::AlreadyExists => "EEXIST",
        std::io::ErrorKind::InvalidInput => "EINVAL",
        std::io::ErrorKind::IsADirectory => "EISDIR",
        std::io::ErrorKind::NotADirectory => "ENOTDIR",
        _ => "EIO",
    }
}

fn fs_error_value<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    syscall: &str,
    path: &str,
    err: &std::io::Error,
) -> v8::Local<'a, v8::Value> {
    let code = io_error_code(err);
    let message = format!("{code}: {err}, {syscall} '{path}'");
    let message = v8::String::new(scope, &message).unwrap();
    let exception = v8::Exception::error(scope, message);
    if let Some(obj) = exception.to_object(scope) {
        let code_key = v8::String::new(scope, "code").unwrap();
        let code_val = v8::String::new(scope, code).unwrap();
        obj.set(scope, code_key.into(), code_val.into());
        let syscall_key = v8::String::new(scope, "syscall").unwrap();
        let syscall_val = v8::String::new(scope, syscall).unwrap();
        obj.set(scope, syscall_key.into(), syscall_val.into());
        let path_key = v8::String::new(scope, "path").unwrap();
        let path_val = v8::String::new(scope, path).unwrap();
        obj.set(scope, path_key.into(), path_val.into());
        let raw = err.raw_os_error().unwrap_or(match code {
            "ENOENT" => libc::ENOENT,
            "EACCES" => libc::EACCES,
            "EEXIST" => libc::EEXIST,
            "EINVAL" => libc::EINVAL,
            "EISDIR" => libc::EISDIR,
            "ENOTDIR" => libc::ENOTDIR,
            "ENOTEMPTY" => libc::ENOTEMPTY,
            "EBADF" => libc::EBADF,
            "ENAMETOOLONG" => libc::ENAMETOOLONG,
            _ => libc::EIO,
        });
        let errno_key = v8::String::new(scope, "errno").unwrap();
        let errno_val = v8::Integer::new(scope, -raw.abs());
        obj.set(scope, errno_key.into(), errno_val.into());
    }
    exception
}

fn throw_fs_io_error(scope: &mut v8::PinScope, syscall: &str, path: &str, err: &std::io::Error) {
    let exception = fs_error_value(scope, syscall, path, err);
    scope.throw_exception(exception);
}

fn read_file_bytes(path: &str) -> std::io::Result<Vec<u8>> {
    if crate::sandbox::virtual_fs::is_enabled() {
        crate::sandbox::virtual_fs::vfs_read(Path::new(path))
    } else {
        std::fs::read(path)
    }
}

fn write_file_bytes(
    c_path: Option<*const libc::c_char>,
    path: &str,
    data: &[u8],
    mode: OpenMode,
) -> std::io::Result<()> {
    if !mode.write {
        return Err(std::io::Error::from_raw_os_error(libc::EBADF));
    }
    if crate::sandbox::virtual_fs::is_enabled() {
        let exists = crate::sandbox::virtual_fs::vfs_exists(Path::new(path));
        if mode.exclusive && exists {
            return Err(std::io::Error::from_raw_os_error(libc::EEXIST));
        }
        if mode.append {
            let mut existing = match crate::sandbox::virtual_fs::vfs_read(Path::new(path)) {
                Ok(bytes) => bytes,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
                Err(err) => return Err(err),
            };
            existing.extend_from_slice(data);
            return crate::sandbox::virtual_fs::vfs_write(Path::new(path), &existing);
        }
        if !mode.truncate && exists {
            let mut existing = crate::sandbox::virtual_fs::vfs_read(Path::new(path))?;
            let n = data.len().min(existing.len());
            existing[..n].copy_from_slice(&data[..n]);
            if data.len() > existing.len() {
                existing.extend_from_slice(&data[existing.len()..]);
            }
            return crate::sandbox::virtual_fs::vfs_write(Path::new(path), &existing);
        }
        return crate::sandbox::virtual_fs::vfs_write(Path::new(path), data);
    }
    if mode.is_plain_write() {
        return direct_write_sync(c_path, path, data);
    }
    let mut file = mode.to_open_options().open(path)?;
    file.write_all(data)
}

fn copy_array_buffer(ab: v8::Local<v8::ArrayBuffer>, offset: usize, len: usize) -> Vec<u8> {
    if len == 0 {
        return Vec::new();
    }
    let store = ab.get_backing_store();
    let ptr = store.as_ref().as_ptr() as *const u8;
    if ptr.is_null() {
        return vec![0u8; len];
    }
    unsafe { std::slice::from_raw_parts(ptr.add(offset), len).to_vec() }
}

fn copy_binary_value(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> Option<Vec<u8>> {
    if value.is_array_buffer_view() || value.is_typed_array() || value.is_data_view() {
        if let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(value) {
            let len = view.byte_length();
            let offset = view.byte_offset();
            if let Some(ab) = view.buffer(scope) {
                return Some(copy_array_buffer(ab, offset, len));
            }
            return Some(vec![0u8; len]);
        }
    }
    if value.is_array_buffer() {
        if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(value) {
            return Some(copy_array_buffer(ab, 0, ab.byte_length()));
        }
    }
    if value.is_object() && !value.is_string() {
        if let Ok(obj) = v8::Local::<v8::Object>::try_from(value) {
            let buf_key = v8::String::new(scope, "buffer")?;
            if let Some(buf_val) = obj.get(scope, buf_key.into()) {
                if let Ok(ab) = v8::Local::<v8::ArrayBuffer>::try_from(buf_val) {
                    let len_key = v8::String::new(scope, "length")?;
                    let len = obj
                        .get(scope, len_key.into())
                        .and_then(|v| v.to_integer(scope))
                        .map(|i| i.value() as usize)
                        .unwrap_or_else(|| ab.byte_length());
                    let len = len.min(ab.byte_length());
                    return Some(copy_array_buffer(ab, 0, len));
                }
            }
        }
    }
    None
}

fn string_to_file_bytes(text: &str, encoding: FileEncoding) -> Result<Vec<u8>, String> {
    match encoding {
        FileEncoding::Utf8 | FileEncoding::Buffer => Ok(text.as_bytes().to_vec()),
        FileEncoding::Hex => {
            hex::decode(text).map_err(|_| "The data argument is not valid hex".to_string())
        }
        FileEncoding::Base64 => {
            use base64::Engine;
            let cleaned: String = text.chars().filter(|ch| !ch.is_whitespace()).collect();
            base64::engine::general_purpose::STANDARD
                .decode(&cleaned)
                .or_else(|_| {
                    base64::engine::general_purpose::STANDARD_NO_PAD
                        .decode(cleaned.trim_end_matches('='))
                })
                .map_err(|_| "The data argument is not valid base64".to_string())
        }
        FileEncoding::Latin1 => Ok(text.chars().map(|ch| (ch as u32 & 0xff) as u8).collect()),
    }
}

fn value_to_file_bytes(
    scope: &mut v8::PinScope,
    value: v8::Local<v8::Value>,
    encoding: FileEncoding,
) -> Result<Vec<u8>, String> {
    if value.is_undefined() || value.is_null() {
        return Err(
            "The \"data\" argument must be of type string or an instance of Buffer, TypedArray, or DataView"
                .to_string(),
        );
    }
    if let Some(bytes) = copy_binary_value(scope, value) {
        return Ok(bytes);
    }
    if value.is_string() {
        let text = value
            .to_string(scope)
            .map(|text| text.to_rust_string_lossy(scope))
            .unwrap_or_default();
        let encoding = if encoding == FileEncoding::Buffer {
            FileEncoding::Utf8
        } else {
            encoding
        };
        return string_to_file_bytes(&text, encoding);
    }
    Err(
        "The \"data\" argument must be of type string or an instance of Buffer, TypedArray, or DataView"
            .to_string(),
    )
}

fn bytes_to_js<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    bytes: &[u8],
    encoding: FileEncoding,
) -> v8::Local<'a, v8::Value> {
    let text = match encoding {
        FileEncoding::Buffer => return create_buffer_from_bytes(scope, bytes),
        FileEncoding::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
        FileEncoding::Hex => hex::encode(bytes),
        FileEncoding::Base64 => {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(bytes)
        }
        FileEncoding::Latin1 => bytes
            .iter()
            .map(|byte| char::from_u32(u32::from(*byte)).unwrap_or('\u{FFFD}'))
            .collect(),
    };
    v8::String::new(scope, &text)
        .map(|value| value.into())
        .unwrap_or_else(|| v8::undefined(scope).into())
}

/// fs.readFileSync(path, encoding) - 读取文件
fn fs_read_file_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let mut path_buf = [0u8; 512];
    let (path, c_path) = get_path_fast(scope, args.get(0), &mut path_buf);
    // The raw pointer is only used by the Unix read fast path.
    #[cfg(not(unix))]
    let _ = c_path;

    if !ensure_fs_permission(scope, PermissionAction::Read, path.as_ref()) {
        return;
    }

    let options = match parse_file_io_options(scope, args.get(1), FileEncoding::Buffer) {
        Ok(options) => options,
        Err(message) => {
            throw_coded_type_error(scope, "ERR_UNKNOWN_ENCODING", &message);
            return;
        }
    };

    // Buffer reads stay on the direct fd path so the common case does not allocate a Vec.
    #[cfg(unix)]
    if options.encoding == FileEncoding::Buffer && !crate::sandbox::virtual_fs::is_enabled() {
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

    match read_file_bytes(path.as_ref()) {
        Ok(bytes) => {
            retval.set(bytes_to_js(scope, &bytes, options.encoding));
        }
        Err(err) => throw_fs_io_error(scope, "open", path.as_ref(), &err),
    }
}

thread_local! {
    static TLS_FS_WRITE_BUFFER: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn write_sync_inner(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
    default_append: bool,
) {
    let mut path_buf = [0u8; 512];
    let (path, c_path) = get_path_fast(scope, args.get(0), &mut path_buf);

    if !ensure_fs_permission(scope, PermissionAction::Write, path.as_ref()) {
        return;
    }

    let options = match parse_file_io_options(scope, args.get(2), FileEncoding::Utf8) {
        Ok(options) => options,
        Err(message) => {
            throw_coded_type_error(scope, "ERR_UNKNOWN_ENCODING", &message);
            return;
        }
    };
    let mode = match options.open_mode(default_append) {
        Ok(mode) => mode,
        Err(message) => {
            throw_coded_type_error(scope, "ERR_INVALID_ARG_VALUE", &message);
            return;
        }
    };
    let val = args.get(1);
    // Plain utf8 strings keep the thread-local write path. Other flags and
    // encodings open with the parsed Node flag instead of O_TRUNC.
    let utf8_fast = val.is_string()
        && matches!(options.encoding, FileEncoding::Utf8 | FileEncoding::Buffer)
        && mode.is_plain_write();

    let res = if utf8_fast {
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
    } else {
        match value_to_file_bytes(scope, val, options.encoding) {
            Ok(bytes) => {
                let direct = if mode.is_plain_write() { c_path } else { None };
                write_file_bytes(direct, path.as_ref(), &bytes, mode)
            }
            Err(message) => {
                let code = if message.contains("not valid") {
                    "ERR_INVALID_ARG_VALUE"
                } else {
                    "ERR_INVALID_ARG_TYPE"
                };
                throw_coded_type_error(scope, code, &message);
                return;
            }
        }
    };

    match res {
        Ok(()) => {
            retval.set(v8::undefined(scope).into());
        }
        Err(err) => throw_fs_io_error(scope, "open", path.as_ref(), &err),
    }
}

/// fs.writeFileSync(path, data[, options])
fn fs_write_file_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    write_sync_inner(scope, args, retval, false);
}

/// fs.appendFileSync(path, data[, options])
fn fs_append_file_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    write_sync_inner(scope, args, retval, true);
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

fn option_truthy(scope: &mut v8::PinScope, value: v8::Local<v8::Value>, key: &str) -> bool {
    if !value.is_object() || value.is_function() || value.is_null() {
        return false;
    }
    let Ok(obj) = v8::Local::<v8::Object>::try_from(value) else {
        return false;
    };
    let Some(key) = v8::String::new(scope, key) else {
        return false;
    };
    obj.get(scope, key.into())
        .map(|flag| flag.to_boolean(scope).is_true())
        .unwrap_or(false)
}

/// Default is one level. `{ recursive: true }` creates parents and ignores an
/// existing directory. An existing file, or a non-recursive collision, is EEXIST.
fn mkdir_at(path: &str, recursive: bool) -> std::io::Result<()> {
    if crate::sandbox::virtual_fs::is_enabled() {
        let p = Path::new(path);
        if crate::sandbox::virtual_fs::vfs_exists(p) {
            if recursive {
                if let Ok(meta) = crate::sandbox::virtual_fs::vfs_metadata(p) {
                    if meta.is_file {
                        return Err(std::io::Error::from_raw_os_error(libc::EEXIST));
                    }
                    return Ok(());
                }
            }
            return Err(std::io::Error::from_raw_os_error(libc::EEXIST));
        }
        if recursive {
            return crate::sandbox::virtual_fs::vfs_create_dir_all(p);
        }
        if let Some(parent) = p.parent() {
            if !parent.as_os_str().is_empty()
                && parent != Path::new("/")
                && !crate::sandbox::virtual_fs::vfs_exists(parent)
            {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "parent directory does not exist",
                ));
            }
        }
        return crate::sandbox::virtual_fs::vfs_create_dir(p);
    }
    let p = Path::new(path);
    if recursive {
        if p.is_file() {
            return Err(std::io::Error::from_raw_os_error(libc::EEXIST));
        }
        return std::fs::create_dir_all(p);
    }
    std::fs::create_dir(p)
}

/// fs.mkdirSync(path[, options])
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

    let recursive = option_truthy(scope, args.get(1), "recursive");
    match mkdir_at(&path, recursive) {
        Ok(()) => retval.set(v8::undefined(scope).into()),
        Err(err) => throw_fs_io_error(scope, "mkdir", &path, &err),
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

    let with_file_types = option_truthy(scope, args.get(1), "withFileTypes");
    match read_directory(&path, with_file_types) {
        Ok(entries) => retval.set(directory_entries_to_js(scope, &entries, with_file_types)),
        Err(err) => throw_fs_io_error(scope, "scandir", &path, &err),
    }
}

struct DirEntryInfo {
    name: String,
    is_file: bool,
    is_dir: bool,
    is_symlink: bool,
}

fn read_directory(path: &str, with_file_types: bool) -> std::io::Result<Vec<DirEntryInfo>> {
    if crate::sandbox::virtual_fs::is_enabled() {
        // vfs_read_dir reports a non-directory file as missing (ENOENT); a
        // directory listing call must distinguish ENOTDIR.
        if let Ok(meta) = crate::sandbox::virtual_fs::vfs_metadata(Path::new(path)) {
            if !meta.is_dir {
                return Err(std::io::Error::from_raw_os_error(libc::ENOTDIR));
            }
        }
        let names = crate::sandbox::virtual_fs::vfs_read_dir(Path::new(path))?;
        let mut entries = Vec::with_capacity(names.len());
        for name in names {
            let child = Path::new(path).join(&name);
            let meta = crate::sandbox::virtual_fs::vfs_metadata(&child).ok();
            entries.push(DirEntryInfo {
                name,
                is_file: meta.map(|m| m.is_file).unwrap_or(true),
                is_dir: meta.map(|m| m.is_dir).unwrap_or(false),
                is_symlink: false,
            });
        }
        return Ok(entries);
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let (is_file, is_dir, is_symlink) = if with_file_types {
            match entry.file_type() {
                Ok(kind) => (kind.is_file(), kind.is_dir(), kind.is_symlink()),
                Err(_) => (false, false, false),
            }
        } else {
            (false, false, false)
        };
        entries.push(DirEntryInfo {
            name,
            is_file,
            is_dir,
            is_symlink,
        });
    }
    Ok(entries)
}

fn dirent_is_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let is_file = get_stats_flag(scope, args.this(), "__isFile");
    retval.set(v8::Boolean::new(scope, is_file).into());
}

fn dirent_is_directory_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let is_dir = get_stats_flag(scope, args.this(), "__isDirectory");
    retval.set(v8::Boolean::new(scope, is_dir).into());
}

fn dirent_is_symbolic_link_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let is_link = get_stats_flag(scope, args.this(), "__isSymbolicLink");
    retval.set(v8::Boolean::new(scope, is_link).into());
}

fn dirent_entry_to_js<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    entry: &DirEntryInfo,
) -> v8::Local<'a, v8::Value> {
    let obj = v8::Object::new(scope);
    let name_key = v8::String::new(scope, "name").unwrap();
    let name_val = v8::String::new(scope, &entry.name).unwrap();
    obj.set(scope, name_key.into(), name_val.into());
    let file_key = v8::String::new(scope, "__isFile").unwrap();
    obj.set(
        scope,
        file_key.into(),
        v8::Boolean::new(scope, entry.is_file).into(),
    );
    let dir_key = v8::String::new(scope, "__isDirectory").unwrap();
    obj.set(
        scope,
        dir_key.into(),
        v8::Boolean::new(scope, entry.is_dir).into(),
    );
    let link_key = v8::String::new(scope, "__isSymbolicLink").unwrap();
    obj.set(
        scope,
        link_key.into(),
        v8::Boolean::new(scope, entry.is_symlink).into(),
    );
    let is_file = v8::FunctionTemplate::new(scope, dirent_is_file_callback)
        .get_function(scope)
        .unwrap();
    let is_dir = v8::FunctionTemplate::new(scope, dirent_is_directory_callback)
        .get_function(scope)
        .unwrap();
    let is_link = v8::FunctionTemplate::new(scope, dirent_is_symbolic_link_callback)
        .get_function(scope)
        .unwrap();
    obj.set(
        scope,
        v8::String::new(scope, "isFile").unwrap().into(),
        is_file.into(),
    );
    obj.set(
        scope,
        v8::String::new(scope, "isDirectory").unwrap().into(),
        is_dir.into(),
    );
    obj.set(
        scope,
        v8::String::new(scope, "isSymbolicLink").unwrap().into(),
        is_link.into(),
    );
    obj.into()
}

fn directory_entries_to_js<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    entries: &[DirEntryInfo],
    with_file_types: bool,
) -> v8::Local<'a, v8::Value> {
    let array = v8::Array::new(scope, entries.len() as i32);
    for (i, entry) in entries.iter().enumerate() {
        let value = if with_file_types {
            dirent_entry_to_js(scope, entry)
        } else {
            v8::String::new(scope, &entry.name).unwrap().into()
        };
        array.set_index(scope, i as u32, value);
    }
    array.into()
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

    match stat_metadata(&path, true) {
        Ok(metadata) => retval.set(metadata_to_stats(scope, &metadata).into()),
        Err(err) => throw_fs_io_error(scope, "stat", &path, &err),
    }
}

enum StatMeta {
    Os(std::fs::Metadata),
    Vfs(crate::sandbox::virtual_fs::VfsMetadata),
}

fn stat_metadata(path: &str, follow: bool) -> std::io::Result<StatMeta> {
    if crate::sandbox::virtual_fs::is_enabled() {
        return crate::sandbox::virtual_fs::vfs_metadata(Path::new(path)).map(StatMeta::Vfs);
    }
    if follow {
        std::fs::metadata(path).map(StatMeta::Os)
    } else {
        std::fs::symlink_metadata(path).map(StatMeta::Os)
    }
}

fn metadata_to_stats<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    metadata: &StatMeta,
) -> v8::Local<'a, v8::Object> {
    match metadata {
        StatMeta::Os(metadata) => create_stats_object(scope, metadata),
        StatMeta::Vfs(metadata) => create_vfs_stats_object(scope, metadata),
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

    match unlink_path(&path) {
        Ok(()) => retval.set(v8::undefined(scope).into()),
        Err(err) => throw_fs_io_error(scope, "unlink", &path, &err),
    }
}

fn unlink_path(path: &str) -> std::io::Result<()> {
    if crate::sandbox::virtual_fs::is_enabled() {
        crate::sandbox::virtual_fs::vfs_remove_file(Path::new(path))
    } else {
        fs::remove_file(path)
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
        Ok(()) => retval.set(v8::undefined(scope).into()),
        Err(err) => throw_fs_io_error(scope, "rename", &old_path, &err),
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

    match rmdir_path(&path) {
        Ok(()) => retval.set(v8::undefined(scope).into()),
        Err(err) => throw_fs_io_error(scope, "rmdir", &path, &err),
    }
}

fn rmdir_path(path: &str) -> std::io::Result<()> {
    if crate::sandbox::virtual_fs::is_enabled() {
        return crate::sandbox::virtual_fs::vfs_remove_dir_all(Path::new(path));
    }
    fs::remove_dir(path)
}

fn callback_index(args: &v8::FunctionCallbackArguments, option_index: i32) -> Option<i32> {
    if args.get(option_index).is_function() {
        Some(option_index)
    } else if args.get(option_index + 1).is_function() {
        Some(option_index + 1)
    } else {
        None
    }
}

/// fs.readFile(path[, options], callback)
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

    let Some(callback_at) = callback_index(&args, 1) else {
        throw_coded_type_error(
            scope,
            "ERR_INVALID_ARG_TYPE",
            "readFile: callback must be a function",
        );
        return;
    };
    let options_val = if callback_at == 1 {
        v8::undefined(scope).into()
    } else {
        args.get(1)
    };
    let Ok(callback) = v8::Local::<v8::Function>::try_from(args.get(callback_at)) else {
        return;
    };
    let options = match parse_file_io_options(scope, options_val, FileEncoding::Buffer) {
        Ok(options) => options,
        Err(message) => {
            throw_coded_type_error(scope, "ERR_UNKNOWN_ENCODING", &message);
            return;
        }
    };

    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }

    // The callback runs on a later nextTick turn, not inside this call.
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback: v8::Global::new(scope, callback),
            shape: FsCallbackShape::Value,
        },
        FsWork::ReadFile {
            path,
            encoding: options.encoding,
        },
    );
    retval.set(v8::undefined(scope).into());
}

fn write_callback_inner(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
    default_append: bool,
    name: &str,
) {
    let path: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let Some(callback_at) = callback_index(&args, 2) else {
        throw_coded_type_error(
            scope,
            "ERR_INVALID_ARG_TYPE",
            &format!("{name}: callback must be a function"),
        );
        return;
    };
    let options_val = if callback_at == 2 {
        v8::undefined(scope).into()
    } else {
        args.get(2)
    };
    let Ok(callback) = v8::Local::<v8::Function>::try_from(args.get(callback_at)) else {
        return;
    };
    let options = match parse_file_io_options(scope, options_val, FileEncoding::Utf8) {
        Ok(options) => options,
        Err(message) => {
            throw_coded_type_error(scope, "ERR_UNKNOWN_ENCODING", &message);
            return;
        }
    };
    let mode = match options.open_mode(default_append) {
        Ok(mode) => mode,
        Err(message) => {
            throw_coded_type_error(scope, "ERR_INVALID_ARG_VALUE", &message);
            return;
        }
    };
    let bytes = match value_to_file_bytes(scope, args.get(1), options.encoding) {
        Ok(bytes) => bytes,
        Err(message) => {
            let code = if message.contains("not valid") {
                "ERR_INVALID_ARG_VALUE"
            } else {
                "ERR_INVALID_ARG_TYPE"
            };
            throw_coded_type_error(scope, code, &message);
            return;
        }
    };

    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }

    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback: v8::Global::new(scope, callback),
            shape: FsCallbackShape::Void,
        },
        FsWork::WriteFile { path, bytes, mode },
    );
    retval.set(v8::undefined(scope).into());
}

/// fs.writeFile(path, data[, options], callback)
fn fs_write_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    write_callback_inner(scope, args, retval, false, "writeFile");
}

/// fs.appendFile(path, data[, options], callback)
fn fs_append_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    write_callback_inner(scope, args, retval, true, "appendFile");
}

// ============ fs.promises API ============
// Callbacks and fs.promises complete on a later process.nextTick turn.
// The calling turn only validates arguments and permissions.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};

const COPYFILE_EXCL: i32 = 1;
// POSIX access(2) bits. Node exposes these numbers on every platform.
// Windows `libc` does not define R_OK / W_OK / X_OK.
const F_OK: i32 = 0;
const X_OK: i32 = 1;
const W_OK: i32 = 2;
const R_OK: i32 = 4;

struct OpenedFile {
    file: std::fs::File,
}

thread_local! {
    static FD_TABLE: RefCell<HashMap<i32, OpenedFile>> = RefCell::new(HashMap::new());
    static NEXT_FD: Cell<i32> = const { Cell::new(3) };
    static FS_JOBS: RefCell<HashMap<u32, (FsDelivery, FsWork)>> = RefCell::new(HashMap::new());
    static FS_JOB_SEQ: Cell<u32> = const { Cell::new(1) };
}

enum FsCallbackShape {
    Value,
    Void,
    Count,
}

enum FsDelivery {
    Callback {
        callback: v8::Global<v8::Function>,
        shape: FsCallbackShape,
    },
    Promise(v8::Global<v8::PromiseResolver>),
}

enum FsWork {
    ReadFile {
        path: String,
        encoding: FileEncoding,
    },
    WriteFile {
        path: String,
        bytes: Vec<u8>,
        mode: OpenMode,
    },
    Mkdir {
        path: String,
        recursive: bool,
    },
    Rmdir {
        path: String,
    },
    Readdir {
        path: String,
        with_file_types: bool,
    },
    Stat {
        path: String,
        follow: bool,
    },
    Unlink {
        path: String,
    },
    Rename {
        from: String,
        to: String,
    },
    CopyFile {
        from: String,
        to: String,
        exclusive: bool,
    },
    Rm {
        path: String,
        recursive: bool,
        force: bool,
    },
    Realpath {
        path: String,
    },
    Chmod {
        path: String,
        mode: u32,
    },
    Access {
        path: String,
        mode: i32,
    },
    Open {
        path: String,
        mode: OpenMode,
        perm: u32,
    },
    ReadFd {
        fd: i32,
        offset: usize,
        length: usize,
        position: Option<u64>,
        buffer: v8::Global<v8::Uint8Array>,
    },
    WriteFd {
        fd: i32,
        bytes: Vec<u8>,
        position: Option<u64>,
    },
    Close {
        fd: i32,
    },
    Cp {
        from: String,
        to: String,
        recursive: bool,
        force: bool,
        error_on_exist: bool,
    },
    Opendir {
        path: String,
    },
    Statfs {
        path: String,
    },
    DirRead {
        dir: i32,
    },
    DirClose {
        dir: i32,
    },
}

enum JobFail {
    Io {
        syscall: &'static str,
        path: String,
        err: std::io::Error,
    },
}

fn arg_text(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> String {
    value
        .to_string(scope)
        .map(|text| text.to_rust_string_lossy(scope))
        .unwrap_or_default()
}

fn arg_i32(scope: &mut v8::PinScope, value: v8::Local<v8::Value>, default: i32) -> i32 {
    if value.is_undefined() || value.is_null() {
        return default;
    }
    value.int32_value(scope).unwrap_or(default)
}

fn arg_usize(scope: &mut v8::PinScope, value: v8::Local<v8::Value>, default: usize) -> usize {
    if value.is_undefined() || value.is_null() {
        return default;
    }
    value
        .uint32_value(scope)
        .map(|n| n as usize)
        .unwrap_or(default)
}

fn arg_position(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> Option<u64> {
    if value.is_undefined() || value.is_null() {
        return None;
    }
    value.number_value(scope).map(|n| n.max(0.0) as u64)
}

fn trailing_callback(
    scope: &mut v8::PinScope,
    args: &v8::FunctionCallbackArguments,
) -> Option<v8::Global<v8::Function>> {
    let len = args.length();
    if len <= 0 {
        return None;
    }
    let last = args.get(len - 1);
    if !last.is_function() {
        return None;
    }
    let func = v8::Local::<v8::Function>::try_from(last).ok()?;
    Some(v8::Global::new(scope, func))
}

fn enqueue_fs_job(scope: &mut v8::PinScope, delivery: FsDelivery, work: FsWork) {
    let id = FS_JOB_SEQ.with(|seq| {
        let id = seq.get();
        seq.set(id.wrapping_add(1));
        id
    });
    FS_JOBS.with(|jobs| {
        jobs.borrow_mut().insert(id, (delivery, work));
    });
    let trampoline = v8::Function::new(scope, fs_job_trampoline).unwrap();
    let id_val = v8::Number::new(scope, f64::from(id));
    let trampoline_val: v8::Local<v8::Value> = trampoline.into();
    let id_value: v8::Local<v8::Value> = id_val.into();
    crate::nodejs_core::process::push_next_tick_callback(
        v8::Global::new(scope, trampoline_val),
        vec![v8::Global::new(scope, id_value)],
    );
}

fn start_promise<'a>(scope: &mut v8::PinScope<'a, '_>, work: FsWork) -> v8::Local<'a, v8::Value> {
    let Some(resolver) = v8::PromiseResolver::new(scope) else {
        return v8::undefined(scope).into();
    };
    let promise = resolver.get_promise(scope);
    enqueue_fs_job(
        scope,
        FsDelivery::Promise(v8::Global::new(scope, resolver)),
        work,
    );
    promise.into()
}

fn fs_job_trampoline(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _retval: v8::ReturnValue,
) {
    let id = args.get(0).number_value(scope).unwrap_or(0.0) as u32;
    let job = FS_JOBS.with(|jobs| jobs.borrow_mut().remove(&id));
    if let Some((delivery, work)) = job {
        run_fs_job(scope, delivery, work);
    }
}

fn broker_denial<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    action: PermissionAction,
    path: &str,
) -> Option<v8::Local<'a, v8::Value>> {
    if !crate::permissions::has_restrictions() {
        return None;
    }
    match check_global_permission(
        PermissionKind::FileSystem,
        action,
        ResourceId::Path(Path::new(path).to_path_buf()),
    ) {
        Ok(()) => None,
        Err(err) => {
            let message = v8::String::new(scope, &err.to_string()).unwrap();
            Some(v8::Exception::error(scope, message))
        }
    }
}

fn work_denial<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    work: &FsWork,
) -> Option<v8::Local<'a, v8::Value>> {
    match work {
        FsWork::ReadFile { path, .. }
        | FsWork::Readdir { path, .. }
        | FsWork::Stat { path, .. }
        | FsWork::Realpath { path }
        | FsWork::Access { path, .. } => broker_denial(scope, PermissionAction::Read, path),
        FsWork::WriteFile { path, .. }
        | FsWork::Mkdir { path, .. }
        | FsWork::Rmdir { path }
        | FsWork::Unlink { path }
        | FsWork::Rm { path, .. }
        | FsWork::Chmod { path, .. } => broker_denial(scope, PermissionAction::Write, path),
        FsWork::Rename { from, to } => broker_denial(scope, PermissionAction::Write, from)
            .or_else(|| broker_denial(scope, PermissionAction::Write, to)),
        FsWork::CopyFile { from, to, .. } => broker_denial(scope, PermissionAction::Read, from)
            .or_else(|| broker_denial(scope, PermissionAction::Write, to)),
        FsWork::Cp { from, to, .. } => broker_denial(scope, PermissionAction::Read, from)
            .or_else(|| broker_denial(scope, PermissionAction::Write, to)),
        FsWork::Opendir { path } | FsWork::Statfs { path } => {
            broker_denial(scope, PermissionAction::Read, path)
        }
        FsWork::DirRead { .. } | FsWork::DirClose { .. } => None,
        FsWork::Open { path, mode, .. } => {
            if mode.read {
                if let Some(err) = broker_denial(scope, PermissionAction::Read, path) {
                    return Some(err);
                }
            }
            if mode.write {
                broker_denial(scope, PermissionAction::Write, path)
            } else {
                None
            }
        }
        FsWork::ReadFd { .. } | FsWork::WriteFd { .. } | FsWork::Close { .. } => None,
    }
}

fn run_fs_job(scope: &mut v8::PinScope, delivery: FsDelivery, work: FsWork) {
    if let Some(denied) = work_denial(scope, &work) {
        deliver_job(scope, delivery, Err(denied));
        return;
    }
    let executed = execute_work(scope, &work);
    let result = executed.map_err(|fail| {
        let JobFail::Io { syscall, path, err } = fail;
        fs_error_value(scope, syscall, &path, &err)
    });
    let extra = work_buffer(scope, &work);
    deliver_job_with_extra(scope, delivery, result, extra);
}

fn work_buffer<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    work: &FsWork,
) -> Option<v8::Local<'a, v8::Value>> {
    match work {
        FsWork::ReadFd { buffer, .. } => Some(v8::Local::new(scope, buffer).into()),
        _ => None,
    }
}

fn deliver_job(
    scope: &mut v8::PinScope,
    delivery: FsDelivery,
    result: Result<v8::Local<v8::Value>, v8::Local<v8::Value>>,
) {
    deliver_job_with_extra(scope, delivery, result, None);
}

fn deliver_job_with_extra(
    scope: &mut v8::PinScope,
    delivery: FsDelivery,
    result: Result<v8::Local<v8::Value>, v8::Local<v8::Value>>,
    extra: Option<v8::Local<v8::Value>>,
) {
    let undefined = v8::undefined(scope);
    match delivery {
        FsDelivery::Callback { callback, shape } => {
            let callback = v8::Local::new(scope, callback);
            match result {
                Ok(value) => {
                    let null: v8::Local<v8::Value> = v8::null(scope).into();
                    match shape {
                        FsCallbackShape::Void => {
                            let _ = callback.call(scope, undefined.into(), &[null]);
                        }
                        FsCallbackShape::Value => {
                            let _ = callback.call(scope, undefined.into(), &[null, value]);
                        }
                        FsCallbackShape::Count => {
                            let buffer = extra.unwrap_or_else(|| v8::undefined(scope).into());
                            let _ = callback.call(scope, undefined.into(), &[null, value, buffer]);
                        }
                    }
                }
                Err(error) => {
                    let _ = callback.call(scope, undefined.into(), &[error]);
                }
            }
        }
        FsDelivery::Promise(resolver) => {
            let resolver = v8::Local::new(scope, resolver);
            match result {
                Ok(value) => {
                    let _ = resolver.resolve(scope, value);
                }
                Err(error) => {
                    let _ = resolver.reject(scope, error);
                }
            }
        }
    }
}

fn execute_work<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    work: &FsWork,
) -> Result<v8::Local<'a, v8::Value>, JobFail> {
    match work {
        FsWork::ReadFile { path, encoding } => {
            let bytes = read_file_bytes(path).map_err(|err| JobFail::Io {
                syscall: "open",
                path: path.clone(),
                err,
            })?;
            Ok(bytes_to_js(scope, &bytes, *encoding))
        }
        FsWork::WriteFile { path, bytes, mode } => {
            write_file_bytes(None, path, bytes, *mode).map_err(|err| JobFail::Io {
                syscall: "open",
                path: path.clone(),
                err,
            })?;
            Ok(v8::undefined(scope).into())
        }
        FsWork::Mkdir { path, recursive } => {
            mkdir_at(path, *recursive).map_err(|err| JobFail::Io {
                syscall: "mkdir",
                path: path.clone(),
                err,
            })?;
            Ok(v8::undefined(scope).into())
        }
        FsWork::Rmdir { path } => {
            rmdir_path(path).map_err(|err| JobFail::Io {
                syscall: "rmdir",
                path: path.clone(),
                err,
            })?;
            Ok(v8::undefined(scope).into())
        }
        FsWork::Readdir {
            path,
            with_file_types,
        } => {
            let entries = read_directory(path, *with_file_types).map_err(|err| JobFail::Io {
                syscall: "scandir",
                path: path.clone(),
                err,
            })?;
            Ok(directory_entries_to_js(scope, &entries, *with_file_types))
        }
        FsWork::Stat { path, follow } => {
            let metadata = stat_metadata(path, *follow).map_err(|err| JobFail::Io {
                syscall: if *follow { "stat" } else { "lstat" },
                path: path.clone(),
                err,
            })?;
            Ok(metadata_to_stats(scope, &metadata).into())
        }
        FsWork::Unlink { path } => {
            unlink_path(path).map_err(|err| JobFail::Io {
                syscall: "unlink",
                path: path.clone(),
                err,
            })?;
            Ok(v8::undefined(scope).into())
        }
        FsWork::Rename { from, to } => {
            std::fs::rename(from, to).map_err(|err| JobFail::Io {
                syscall: "rename",
                path: from.clone(),
                err,
            })?;
            Ok(v8::undefined(scope).into())
        }
        FsWork::CopyFile {
            from,
            to,
            exclusive,
        } => {
            copy_path(from, to, *exclusive).map_err(|err| JobFail::Io {
                syscall: "copyfile",
                path: from.clone(),
                err,
            })?;
            Ok(v8::undefined(scope).into())
        }
        FsWork::Rm {
            path,
            recursive,
            force,
        } => {
            rm_path(path, *recursive, *force).map_err(|err| JobFail::Io {
                syscall: "rm",
                path: path.clone(),
                err,
            })?;
            Ok(v8::undefined(scope).into())
        }
        FsWork::Realpath { path } => {
            let resolved = realpath_of(path).map_err(|err| JobFail::Io {
                syscall: "realpath",
                path: path.clone(),
                err,
            })?;
            Ok(v8::String::new(scope, &resolved)
                .map(|value| value.into())
                .unwrap_or_else(|| v8::undefined(scope).into()))
        }
        FsWork::Chmod { path, mode } => {
            chmod_path(path, *mode).map_err(|err| JobFail::Io {
                syscall: "chmod",
                path: path.clone(),
                err,
            })?;
            Ok(v8::undefined(scope).into())
        }
        FsWork::Access { path, mode } => {
            access_path(path, *mode).map_err(|err| JobFail::Io {
                syscall: "access",
                path: path.clone(),
                err,
            })?;
            Ok(v8::undefined(scope).into())
        }
        FsWork::Open { path, mode, perm } => {
            let fd = open_fd(path, *mode, *perm).map_err(|err| JobFail::Io {
                syscall: "open",
                path: path.clone(),
                err,
            })?;
            Ok(v8::Integer::new(scope, fd).into())
        }
        FsWork::ReadFd {
            fd,
            offset,
            length,
            position,
            buffer,
        } => {
            let bytes = read_fd(*fd, *position, *length).map_err(|err| JobFail::Io {
                syscall: "read",
                path: fd.to_string(),
                err,
            })?;
            let view = v8::Local::new(scope, buffer);
            let _ = write_view_bytes(scope, view.into(), *offset, &bytes);
            Ok(v8::Integer::new(scope, bytes.len() as i32).into())
        }
        FsWork::WriteFd {
            fd,
            bytes,
            position,
        } => {
            let n = write_fd(*fd, *position, bytes).map_err(|err| JobFail::Io {
                syscall: "write",
                path: fd.to_string(),
                err,
            })?;
            Ok(v8::Integer::new(scope, n as i32).into())
        }
        FsWork::Close { fd } => {
            close_fd(*fd).map_err(|err| JobFail::Io {
                syscall: "close",
                path: fd.to_string(),
                err,
            })?;
            Ok(v8::undefined(scope).into())
        }
        FsWork::Cp {
            from,
            to,
            recursive,
            force,
            error_on_exist,
        } => {
            cp_path(from, to, *recursive, *force, *error_on_exist).map_err(|err| JobFail::Io {
                syscall: "cp",
                path: from.clone(),
                err,
            })?;
            Ok(v8::undefined(scope).into())
        }
        FsWork::Opendir { path } => {
            let entries = read_directory(path, true).map_err(|err| JobFail::Io {
                syscall: "opendir",
                path: path.clone(),
                err,
            })?;
            // Snapshot semantics: the entries are captured at opendir time —
            // read() replays this list, it does not see later changes.
            let id = dir_register(path.clone(), entries).map_err(|err| JobFail::Io {
                syscall: "opendir",
                path: path.clone(),
                err,
            })?;
            Ok(dir_to_js(scope, id, path))
        }
        FsWork::Statfs { path } => {
            let info = statfs_info(path).map_err(|err| JobFail::Io {
                syscall: "statfs",
                path: path.clone(),
                err,
            })?;
            Ok(statfs_to_js(scope, &info))
        }
        FsWork::DirRead { dir } => match dir_read_next(*dir) {
            Ok(entry) => match entry {
                Some(entry) => Ok(dirent_entry_to_js(scope, &entry)),
                None => Ok(v8::null(scope).into()),
            },
            Err(err) => Err(JobFail::Io {
                syscall: "readdir",
                path: dir.to_string(),
                err,
            }),
        },
        FsWork::DirClose { dir } => {
            dir_close(*dir).map_err(|err| JobFail::Io {
                syscall: "close",
                path: dir.to_string(),
                err,
            })?;
            Ok(v8::undefined(scope).into())
        }
    }
}

fn copy_path(from: &str, to: &str, exclusive: bool) -> std::io::Result<()> {
    if crate::sandbox::virtual_fs::is_enabled() {
        if exclusive && crate::sandbox::virtual_fs::vfs_exists(Path::new(to)) {
            return Err(std::io::Error::from_raw_os_error(libc::EEXIST));
        }
        let bytes = crate::sandbox::virtual_fs::vfs_read(Path::new(from))?;
        return crate::sandbox::virtual_fs::vfs_write(Path::new(to), &bytes);
    }
    if exclusive && Path::new(to).exists() {
        return Err(std::io::Error::from_raw_os_error(libc::EEXIST));
    }
    std::fs::copy(from, to).map(|_| ())
}

fn rm_path(path: &str, recursive: bool, force: bool) -> std::io::Result<()> {
    if crate::sandbox::virtual_fs::is_enabled() {
        let p = Path::new(path);
        if !crate::sandbox::virtual_fs::vfs_exists(p) {
            return if force {
                Ok(())
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "no such file or directory",
                ))
            };
        }
        if let Ok(meta) = crate::sandbox::virtual_fs::vfs_metadata(p) {
            if meta.is_dir {
                if !recursive {
                    return Err(std::io::Error::from_raw_os_error(libc::EISDIR));
                }
                return crate::sandbox::virtual_fs::vfs_remove_dir_all(p);
            }
        }
        return crate::sandbox::virtual_fs::vfs_remove_file(p);
    }
    let p = Path::new(path);
    let meta = match std::fs::symlink_metadata(p) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound && force => return Ok(()),
        Err(err) => return Err(err),
    };
    if meta.is_dir() {
        if !recursive {
            return Err(std::io::Error::from_raw_os_error(libc::EISDIR));
        }
        std::fs::remove_dir_all(p)
    } else {
        std::fs::remove_file(p)
    }
}

fn realpath_of(path: &str) -> std::io::Result<String> {
    if crate::sandbox::virtual_fs::is_enabled() {
        if crate::sandbox::virtual_fs::vfs_exists(Path::new(path)) {
            return Ok(path.to_string());
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no such file or directory",
        ));
    }
    std::fs::canonicalize(path).map(|path| path.to_string_lossy().into_owned())
}

fn chmod_path(path: &str, mode: u32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        return std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
        Ok(())
    }
}

fn access_path(path: &str, mode: i32) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let c_path = std::ffi::CString::new(path).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains a nul")
        })?;
        let rc = unsafe { libc::access(c_path.as_ptr(), mode) };
        if rc == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
    #[cfg(not(unix))]
    {
        let _ = mode;
        if Path::new(path).exists() {
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no such file or directory",
            ))
        }
    }
}

fn open_fd(
    path: &str,
    mode: OpenMode,
    #[cfg_attr(not(unix), allow(unused_variables))] perm: u32,
) -> std::io::Result<i32> {
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut opts = mode.to_open_options();
    #[cfg(unix)]
    if mode.create || mode.exclusive {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(perm);
    }
    let file = opts.open(path)?;
    let fd = NEXT_FD.with(|next| {
        let fd = next.get();
        next.set(fd.saturating_add(1));
        fd
    });
    FD_TABLE.with(|table| {
        table.borrow_mut().insert(fd, OpenedFile { file });
    });
    Ok(fd)
}

fn read_fd(fd: i32, position: Option<u64>, length: usize) -> std::io::Result<Vec<u8>> {
    FD_TABLE.with(|table| {
        let mut table = table.borrow_mut();
        let handle = table
            .get_mut(&fd)
            .ok_or_else(|| std::io::Error::from_raw_os_error(libc::EBADF))?;
        if let Some(position) = position {
            handle.file.seek(SeekFrom::Start(position))?;
        }
        let mut buf = vec![0u8; length];
        let n = handle.file.read(&mut buf)?;
        buf.truncate(n);
        Ok(buf)
    })
}

fn write_fd(fd: i32, position: Option<u64>, bytes: &[u8]) -> std::io::Result<usize> {
    FD_TABLE.with(|table| {
        let mut table = table.borrow_mut();
        let handle = table
            .get_mut(&fd)
            .ok_or_else(|| std::io::Error::from_raw_os_error(libc::EBADF))?;
        if let Some(position) = position {
            handle.file.seek(SeekFrom::Start(position))?;
        }
        handle.file.write_all(bytes)?;
        Ok(bytes.len())
    })
}

fn close_fd(fd: i32) -> std::io::Result<()> {
    let removed = FD_TABLE.with(|table| table.borrow_mut().remove(&fd));
    if removed.is_some() {
        Ok(())
    } else {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
}

fn write_view_bytes(
    scope: &mut v8::PinScope,
    value: v8::Local<v8::Value>,
    offset: usize,
    data: &[u8],
) -> bool {
    let Ok(view) = v8::Local::<v8::ArrayBufferView>::try_from(value) else {
        return false;
    };
    let Some(buffer) = view.buffer(scope) else {
        return false;
    };
    let store = buffer.get_backing_store();
    let ptr = store.as_ref().as_ptr() as *mut u8;
    if ptr.is_null() {
        return data.is_empty();
    }
    let start = view.byte_offset() + offset;
    if start + data.len() > buffer.byte_length() {
        return false;
    }
    if !data.is_empty() {
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr.add(start), data.len());
        }
    }
    true
}

fn view_byte_length(_scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> usize {
    v8::Local::<v8::ArrayBufferView>::try_from(value)
        .map(|view| view.byte_length())
        .unwrap_or(0)
}

fn require_callback(
    scope: &mut v8::PinScope,
    args: &v8::FunctionCallbackArguments,
    name: &str,
) -> Option<v8::Global<v8::Function>> {
    if let Some(callback) = trailing_callback(scope, args) {
        return Some(callback);
    }
    throw_coded_type_error(
        scope,
        "ERR_INVALID_ARG_TYPE",
        &format!("{name}: callback must be a function"),
    );
    None
}

fn fs_promises_read_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    let encoding = match parse_file_io_options(scope, args.get(1), FileEncoding::Buffer) {
        Ok(options) => options.encoding,
        Err(message) => {
            throw_coded_type_error(scope, "ERR_UNKNOWN_ENCODING", &message);
            return;
        }
    };
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    retval.set(start_promise(scope, FsWork::ReadFile { path, encoding }));
}

fn fs_promises_write_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    schedule_promise_write(scope, args, retval, false);
}

fn fs_promises_append_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    schedule_promise_write(scope, args, retval, true);
}

fn schedule_promise_write(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
    default_append: bool,
) {
    let path = arg_text(scope, args.get(0));
    let options = match parse_file_io_options(scope, args.get(2), FileEncoding::Utf8) {
        Ok(options) => options,
        Err(message) => {
            throw_coded_type_error(scope, "ERR_UNKNOWN_ENCODING", &message);
            return;
        }
    };
    let mode = match options.open_mode(default_append) {
        Ok(mode) => mode,
        Err(message) => {
            throw_coded_type_error(scope, "ERR_INVALID_ARG_VALUE", &message);
            return;
        }
    };
    let bytes = match value_to_file_bytes(scope, args.get(1), options.encoding) {
        Ok(bytes) => bytes,
        Err(message) => {
            let code = if message.contains("not valid") {
                "ERR_INVALID_ARG_VALUE"
            } else {
                "ERR_INVALID_ARG_TYPE"
            };
            throw_coded_type_error(scope, code, &message);
            return;
        }
    };
    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }
    retval.set(start_promise(
        scope,
        FsWork::WriteFile { path, bytes, mode },
    ));
}

fn fs_promises_mkdir_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }
    let recursive = option_truthy(scope, args.get(1), "recursive");
    retval.set(start_promise(scope, FsWork::Mkdir { path, recursive }));
}

fn fs_promises_rmdir_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }
    retval.set(start_promise(scope, FsWork::Rmdir { path }));
}

fn fs_promises_readdir_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    let with_file_types = option_truthy(scope, args.get(1), "withFileTypes");
    retval.set(start_promise(
        scope,
        FsWork::Readdir {
            path,
            with_file_types,
        },
    ));
}

fn fs_promises_stat_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    retval.set(start_promise(scope, FsWork::Stat { path, follow: true }));
}

fn fs_promises_unlink_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }
    retval.set(start_promise(scope, FsWork::Unlink { path }));
}

fn fs_promises_rename_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let from = arg_text(scope, args.get(0));
    let to = arg_text(scope, args.get(1));
    if !ensure_fs_permission(scope, PermissionAction::Write, &from)
        || !ensure_fs_permission(scope, PermissionAction::Write, &to)
    {
        return;
    }
    retval.set(start_promise(scope, FsWork::Rename { from, to }));
}

fn fs_lstat_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    match stat_metadata(&path, false) {
        Ok(metadata) => retval.set(metadata_to_stats(scope, &metadata).into()),
        Err(err) => throw_fs_io_error(scope, "lstat", &path, &err),
    }
}

fn fs_copy_file_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let from = arg_text(scope, args.get(0));
    let to = arg_text(scope, args.get(1));
    let exclusive = arg_i32(scope, args.get(2), 0) & COPYFILE_EXCL != 0;
    if !ensure_fs_permission(scope, PermissionAction::Read, &from)
        || !ensure_fs_permission(scope, PermissionAction::Write, &to)
    {
        return;
    }
    match copy_path(&from, &to, exclusive) {
        Ok(()) => retval.set(v8::undefined(scope).into()),
        Err(err) => throw_fs_io_error(scope, "copyfile", &from, &err),
    }
}

fn fs_rm_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }
    let recursive = option_truthy(scope, args.get(1), "recursive");
    let force = option_truthy(scope, args.get(1), "force");
    match rm_path(&path, recursive, force) {
        Ok(()) => retval.set(v8::undefined(scope).into()),
        Err(err) => throw_fs_io_error(scope, "rm", &path, &err),
    }
}

fn fs_realpath_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    match realpath_of(&path) {
        Ok(resolved) => {
            let value = v8::String::new(scope, &resolved).unwrap();
            retval.set(value.into());
        }
        Err(err) => throw_fs_io_error(scope, "realpath", &path, &err),
    }
}

fn fs_chmod_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    let mode = arg_i32(scope, args.get(1), 0o666) as u32;
    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }
    match chmod_path(&path, mode) {
        Ok(()) => retval.set(v8::undefined(scope).into()),
        Err(err) => throw_fs_io_error(scope, "chmod", &path, &err),
    }
}

fn fs_access_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    let mode = if args.get(1).is_undefined() || args.get(1).is_null() {
        0
    } else {
        arg_i32(scope, args.get(1), 0)
    };
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    match access_path(&path, mode) {
        Ok(()) => retval.set(v8::undefined(scope).into()),
        Err(err) => throw_fs_io_error(scope, "access", &path, &err),
    }
}

fn fs_open_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    let flags = if args.get(1).is_undefined() {
        "r".to_string()
    } else {
        arg_text(scope, args.get(1))
    };
    let mode = match OpenMode::parse(if flags.is_empty() { "r" } else { &flags }) {
        Ok(mode) => mode,
        Err(message) => {
            throw_coded_type_error(scope, "ERR_INVALID_ARG_VALUE", &message);
            return;
        }
    };
    let perm = arg_i32(scope, args.get(2), 0o666) as u32;
    if mode.read && !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    if mode.write && !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }
    match open_fd(&path, mode, perm) {
        Ok(fd) => retval.set(v8::Integer::new(scope, fd).into()),
        Err(err) => throw_fs_io_error(scope, "open", &path, &err),
    }
}

fn fs_read_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let fd = arg_i32(scope, args.get(0), -1);
    let buffer = args.get(1);
    let offset = arg_usize(scope, args.get(2), 0);
    let available = view_byte_length(scope, buffer);
    let length = arg_usize(scope, args.get(3), available.saturating_sub(offset));
    let position = arg_position(scope, args.get(4));
    match read_fd(fd, position, length) {
        Ok(bytes) => {
            let _ = write_view_bytes(scope, buffer, offset, &bytes);
            retval.set(v8::Integer::new(scope, bytes.len() as i32).into());
        }
        Err(err) => throw_fs_io_error(scope, "read", &fd.to_string(), &err),
    }
}

fn fs_write_fd_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let fd = arg_i32(scope, args.get(0), -1);
    let data = args.get(1);
    let (bytes, position) = if data.is_string() {
        let text = arg_text(scope, data);
        let position = arg_position(scope, args.get(2));
        (text.into_bytes(), position)
    } else {
        let offset = arg_usize(scope, args.get(2), 0);
        let available = view_byte_length(scope, data);
        let length = arg_usize(scope, args.get(3), available.saturating_sub(offset));
        let position = arg_position(scope, args.get(4));
        let owned = copy_binary_value(scope, data).unwrap_or_default();
        let end = (offset + length).min(owned.len());
        let start = offset.min(owned.len());
        (owned[start..end].to_vec(), position)
    };
    match write_fd(fd, position, &bytes) {
        Ok(n) => retval.set(v8::Integer::new(scope, n as i32).into()),
        Err(err) => throw_fs_io_error(scope, "write", &fd.to_string(), &err),
    }
}

fn fs_close_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let fd = arg_i32(scope, args.get(0), -1);
    match close_fd(fd) {
        Ok(()) => retval.set(v8::undefined(scope).into()),
        Err(err) => throw_fs_io_error(scope, "close", &fd.to_string(), &err),
    }
}

fn fs_copy_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let Some(callback) = require_callback(scope, &args, "copyFile") else {
        return;
    };
    let from = arg_text(scope, args.get(0));
    let to = arg_text(scope, args.get(1));
    let mode_arg = if args.length() >= 4 || (args.length() == 3 && !args.get(2).is_function()) {
        args.get(2)
    } else {
        v8::undefined(scope).into()
    };
    let exclusive = arg_i32(scope, mode_arg, 0) & COPYFILE_EXCL != 0;
    if !ensure_fs_permission(scope, PermissionAction::Read, &from)
        || !ensure_fs_permission(scope, PermissionAction::Write, &to)
    {
        return;
    }
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback,
            shape: FsCallbackShape::Void,
        },
        FsWork::CopyFile {
            from,
            to,
            exclusive,
        },
    );
    retval.set(v8::undefined(scope).into());
}

fn fs_rm_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let Some(callback) = require_callback(scope, &args, "rm") else {
        return;
    };
    let path = arg_text(scope, args.get(0));
    let options = if args.length() >= 3 && !args.get(1).is_function() {
        args.get(1)
    } else {
        v8::undefined(scope).into()
    };
    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }
    let recursive = option_truthy(scope, options, "recursive");
    let force = option_truthy(scope, options, "force");
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback,
            shape: FsCallbackShape::Void,
        },
        FsWork::Rm {
            path,
            recursive,
            force,
        },
    );
    retval.set(v8::undefined(scope).into());
}

fn fs_realpath_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let Some(callback) = require_callback(scope, &args, "realpath") else {
        return;
    };
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback,
            shape: FsCallbackShape::Value,
        },
        FsWork::Realpath { path },
    );
    retval.set(v8::undefined(scope).into());
}

fn fs_chmod_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let Some(callback) = require_callback(scope, &args, "chmod") else {
        return;
    };
    let path = arg_text(scope, args.get(0));
    let mode = arg_i32(scope, args.get(1), 0o666) as u32;
    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback,
            shape: FsCallbackShape::Void,
        },
        FsWork::Chmod { path, mode },
    );
    retval.set(v8::undefined(scope).into());
}

fn fs_access_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let Some(callback) = require_callback(scope, &args, "access") else {
        return;
    };
    let path = arg_text(scope, args.get(0));
    let mode = if args.length() >= 3 && !args.get(1).is_function() {
        arg_i32(scope, args.get(1), 0)
    } else {
        0
    };
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback,
            shape: FsCallbackShape::Void,
        },
        FsWork::Access { path, mode },
    );
    retval.set(v8::undefined(scope).into());
}

fn fs_open_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let Some(callback) = require_callback(scope, &args, "open") else {
        return;
    };
    let path = arg_text(scope, args.get(0));
    let flags = if args.length() >= 3 && !args.get(1).is_function() {
        arg_text(scope, args.get(1))
    } else {
        "r".to_string()
    };
    let mode = match OpenMode::parse(if flags.is_empty() { "r" } else { &flags }) {
        Ok(mode) => mode,
        Err(message) => {
            throw_coded_type_error(scope, "ERR_INVALID_ARG_VALUE", &message);
            return;
        }
    };
    let perm = if args.length() >= 4 && !args.get(2).is_function() {
        arg_i32(scope, args.get(2), 0o666) as u32
    } else {
        0o666
    };
    if mode.read && !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    if mode.write && !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback,
            shape: FsCallbackShape::Value,
        },
        FsWork::Open { path, mode, perm },
    );
    retval.set(v8::undefined(scope).into());
}

fn fs_read_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let Some(callback) = require_callback(scope, &args, "read") else {
        return;
    };
    let fd = arg_i32(scope, args.get(0), -1);
    let buffer = args.get(1);
    let Ok(view) = v8::Local::<v8::Uint8Array>::try_from(buffer) else {
        throw_coded_type_error(
            scope,
            "ERR_INVALID_ARG_TYPE",
            "read: buffer must be a Uint8Array",
        );
        return;
    };
    let offset = arg_usize(scope, args.get(2), 0);
    let available = view.byte_length();
    let length = if args.length() >= 5 && !args.get(3).is_function() {
        arg_usize(scope, args.get(3), available.saturating_sub(offset))
    } else {
        available.saturating_sub(offset)
    };
    let position = if args.length() >= 6 && !args.get(4).is_function() {
        arg_position(scope, args.get(4))
    } else {
        None
    };
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback,
            shape: FsCallbackShape::Count,
        },
        FsWork::ReadFd {
            fd,
            offset,
            length,
            position,
            buffer: v8::Global::new(scope, view),
        },
    );
    retval.set(v8::undefined(scope).into());
}

fn fs_write_fd_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let Some(callback) = require_callback(scope, &args, "write") else {
        return;
    };
    let fd = arg_i32(scope, args.get(0), -1);
    let data = args.get(1);
    let (bytes, position) = if data.is_string() {
        (
            arg_text(scope, data).into_bytes(),
            arg_position(scope, args.get(2)),
        )
    } else {
        let offset = arg_usize(scope, args.get(2), 0);
        let available = view_byte_length(scope, data);
        let length = arg_usize(scope, args.get(3), available.saturating_sub(offset));
        let position = arg_position(scope, args.get(4));
        let owned = copy_binary_value(scope, data).unwrap_or_default();
        let end = (offset + length).min(owned.len());
        let start = offset.min(owned.len());
        (owned[start..end].to_vec(), position)
    };
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback,
            shape: FsCallbackShape::Count,
        },
        FsWork::WriteFd {
            fd,
            bytes,
            position,
        },
    );
    retval.set(v8::undefined(scope).into());
}

fn fs_close_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let Some(callback) = require_callback(scope, &args, "close") else {
        return;
    };
    let fd = arg_i32(scope, args.get(0), -1);
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback,
            shape: FsCallbackShape::Void,
        },
        FsWork::Close { fd },
    );
    retval.set(v8::undefined(scope).into());
}

fn fs_lstat_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let Some(callback) = require_callback(scope, &args, "lstat") else {
        return;
    };
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback,
            shape: FsCallbackShape::Value,
        },
        FsWork::Stat {
            path,
            follow: false,
        },
    );
    retval.set(v8::undefined(scope).into());
}

fn fs_mkdir_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let Some(callback) = require_callback(scope, &args, "mkdir") else {
        return;
    };
    let path = arg_text(scope, args.get(0));
    let options = if args.length() >= 3 && !args.get(1).is_function() {
        args.get(1)
    } else {
        v8::undefined(scope).into()
    };
    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }
    let recursive = option_truthy(scope, options, "recursive");
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback,
            shape: FsCallbackShape::Void,
        },
        FsWork::Mkdir { path, recursive },
    );
    retval.set(v8::undefined(scope).into());
}

fn promise_method(scope: &mut v8::PinScope, mut retval: v8::ReturnValue, work: FsWork) {
    retval.set(start_promise(scope, work));
}

fn fs_promises_lstat_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    promise_method(
        scope,
        retval,
        FsWork::Stat {
            path,
            follow: false,
        },
    );
}

fn fs_promises_copy_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    let from = arg_text(scope, args.get(0));
    let to = arg_text(scope, args.get(1));
    let exclusive = arg_i32(scope, args.get(2), 0) & COPYFILE_EXCL != 0;
    if !ensure_fs_permission(scope, PermissionAction::Read, &from)
        || !ensure_fs_permission(scope, PermissionAction::Write, &to)
    {
        return;
    }
    promise_method(
        scope,
        retval,
        FsWork::CopyFile {
            from,
            to,
            exclusive,
        },
    );
}

fn fs_promises_rm_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }
    let options = args.get(1);
    let recursive = option_truthy(scope, options, "recursive");
    let force = option_truthy(scope, options, "force");
    promise_method(
        scope,
        retval,
        FsWork::Rm {
            path,
            recursive,
            force,
        },
    );
}

fn fs_promises_realpath_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    promise_method(scope, retval, FsWork::Realpath { path });
}

fn fs_promises_chmod_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    let mode = arg_i32(scope, args.get(1), 0o666) as u32;
    if !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }
    promise_method(scope, retval, FsWork::Chmod { path, mode });
}

fn fs_promises_access_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    let mode = arg_i32(scope, args.get(1), 0);
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    promise_method(scope, retval, FsWork::Access { path, mode });
}

fn fs_promises_open_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    let flags = if args.get(1).is_undefined() {
        "r".to_string()
    } else {
        arg_text(scope, args.get(1))
    };
    let mode = match OpenMode::parse(if flags.is_empty() { "r" } else { &flags }) {
        Ok(mode) => mode,
        Err(message) => {
            throw_coded_type_error(scope, "ERR_INVALID_ARG_VALUE", &message);
            return;
        }
    };
    let perm = arg_i32(scope, args.get(2), 0o666) as u32;
    if mode.read && !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    if mode.write && !ensure_fs_permission(scope, PermissionAction::Write, &path) {
        return;
    }
    promise_method(scope, retval, FsWork::Open { path, mode, perm });
}

fn fs_promises_read_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let fd = arg_i32(scope, args.get(0), -1);
    let Ok(view) = v8::Local::<v8::Uint8Array>::try_from(args.get(1)) else {
        throw_coded_type_error(
            scope,
            "ERR_INVALID_ARG_TYPE",
            "read: buffer must be a Uint8Array",
        );
        return;
    };
    let offset = arg_usize(scope, args.get(2), 0);
    let length = arg_usize(
        scope,
        args.get(3),
        view.byte_length().saturating_sub(offset),
    );
    let position = arg_position(scope, args.get(4));
    retval.set(start_promise(
        scope,
        FsWork::ReadFd {
            fd,
            offset,
            length,
            position,
            buffer: v8::Global::new(scope, view),
        },
    ));
}

fn fs_promises_write_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let fd = arg_i32(scope, args.get(0), -1);
    let data = args.get(1);
    let (bytes, position) = if data.is_string() {
        (
            arg_text(scope, data).into_bytes(),
            arg_position(scope, args.get(2)),
        )
    } else {
        let offset = arg_usize(scope, args.get(2), 0);
        let available = view_byte_length(scope, data);
        let length = arg_usize(scope, args.get(3), available.saturating_sub(offset));
        let position = arg_position(scope, args.get(4));
        let owned = copy_binary_value(scope, data).unwrap_or_default();
        let end = (offset + length).min(owned.len());
        let start = offset.min(owned.len());
        (owned[start..end].to_vec(), position)
    };
    retval.set(start_promise(
        scope,
        FsWork::WriteFd {
            fd,
            bytes,
            position,
        },
    ));
}

fn fs_promises_close_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let fd = arg_i32(scope, args.get(0), -1);
    retval.set(start_promise(scope, FsWork::Close { fd }));
}

fn set_constant(scope: &mut v8::PinScope, obj: v8::Local<v8::Object>, name: &str, value: i32) {
    let key = v8::String::new(scope, name).unwrap();
    let value = v8::Integer::new(scope, value);
    obj.set(scope, key.into(), value.into());
}

fn install_stable_fs_methods(scope: &mut v8::PinScope, fs_obj: v8::Local<v8::Object>) {
    macro_rules! bind_fn {
        ($obj:expr, $name:expr, $cb:path) => {{
            let func = v8::FunctionTemplate::new(scope, $cb)
                .get_function(scope)
                .unwrap();
            let key = v8::String::new(scope, $name).unwrap();
            $obj.set(scope, key.into(), func.into());
        }};
    }

    bind_fn!(fs_obj, "lstatSync", fs_lstat_sync_callback);
    bind_fn!(fs_obj, "lstat", fs_lstat_callback);
    bind_fn!(fs_obj, "copyFileSync", fs_copy_file_sync_callback);
    bind_fn!(fs_obj, "copyFile", fs_copy_file_callback);
    bind_fn!(fs_obj, "rmSync", fs_rm_sync_callback);
    bind_fn!(fs_obj, "rm", fs_rm_callback);
    bind_fn!(fs_obj, "realpathSync", fs_realpath_sync_callback);
    bind_fn!(fs_obj, "realpath", fs_realpath_callback);
    bind_fn!(fs_obj, "chmodSync", fs_chmod_sync_callback);
    bind_fn!(fs_obj, "chmod", fs_chmod_callback);
    bind_fn!(fs_obj, "accessSync", fs_access_sync_callback);
    bind_fn!(fs_obj, "access", fs_access_callback);
    bind_fn!(fs_obj, "openSync", fs_open_sync_callback);
    bind_fn!(fs_obj, "open", fs_open_callback);
    bind_fn!(fs_obj, "readSync", fs_read_sync_callback);
    bind_fn!(fs_obj, "read", fs_read_callback);
    bind_fn!(fs_obj, "writeSync", fs_write_fd_sync_callback);
    bind_fn!(fs_obj, "write", fs_write_fd_callback);
    bind_fn!(fs_obj, "closeSync", fs_close_sync_callback);
    bind_fn!(fs_obj, "close", fs_close_callback);
    bind_fn!(fs_obj, "mkdir", fs_mkdir_callback);
    // G76 carve: recursive copy, Dir iteration, statvfs.
    bind_fn!(fs_obj, "cp", fs_cp_callback);
    bind_fn!(fs_obj, "cpSync", fs_cp_sync_callback);
    bind_fn!(fs_obj, "opendir", fs_opendir_callback);
    bind_fn!(fs_obj, "opendirSync", fs_opendir_sync_callback);
    bind_fn!(fs_obj, "statfs", fs_statfs_callback);
    bind_fn!(fs_obj, "statfsSync", fs_statfs_sync_callback);

    let constants = v8::Object::new(scope);
    set_constant(scope, constants, "F_OK", F_OK);
    set_constant(scope, constants, "R_OK", R_OK);
    set_constant(scope, constants, "W_OK", W_OK);
    set_constant(scope, constants, "X_OK", X_OK);
    set_constant(scope, constants, "O_RDONLY", libc::O_RDONLY as i32);
    set_constant(scope, constants, "O_WRONLY", libc::O_WRONLY as i32);
    set_constant(scope, constants, "O_RDWR", libc::O_RDWR as i32);
    set_constant(scope, constants, "O_APPEND", libc::O_APPEND as i32);
    set_constant(scope, constants, "O_CREAT", libc::O_CREAT as i32);
    set_constant(scope, constants, "O_EXCL", libc::O_EXCL as i32);
    set_constant(scope, constants, "O_TRUNC", libc::O_TRUNC as i32);
    set_constant(scope, constants, "COPYFILE_EXCL", COPYFILE_EXCL);
    let constants_key = v8::String::new(scope, "constants").unwrap();
    fs_obj.set(scope, constants_key.into(), constants.into());

    let promises_key = v8::String::new(scope, "promises").unwrap();
    if let Some(promises_val) = fs_obj.get(scope, promises_key.into()) {
        if let Ok(promises) = v8::Local::<v8::Object>::try_from(promises_val) {
            bind_fn!(promises, "lstat", fs_promises_lstat_callback);
            bind_fn!(promises, "copyFile", fs_promises_copy_file_callback);
            bind_fn!(promises, "rm", fs_promises_rm_callback);
            bind_fn!(promises, "realpath", fs_promises_realpath_callback);
            bind_fn!(promises, "chmod", fs_promises_chmod_callback);
            bind_fn!(promises, "access", fs_promises_access_callback);
            bind_fn!(promises, "open", fs_promises_open_callback);
            bind_fn!(promises, "read", fs_promises_read_callback);
            bind_fn!(promises, "write", fs_promises_write_callback);
            bind_fn!(promises, "close", fs_promises_close_callback);
            bind_fn!(promises, "cp", fs_promises_cp_callback);
            bind_fn!(promises, "opendir", fs_promises_opendir_callback);
            bind_fn!(promises, "statfs", fs_promises_statfs_callback);
        }
    }
}

// --- G76 additive carve: fs.cp / fs.opendir / fs.statfs -------------------
//
// Narrow surface (see docs/FS_CONTRACT.md "G76 carve"):
//   cp(src, dest, {recursive, force, errorOnExist}) — recursive copy; dir
//     without `recursive` is EISDIR; dest inside src is EINVAL.
//   opendir(path) -> Dir — snapshot directory listing; Dir exposes
//     path, read(cb)/readSync(), close(cb)/closeSync(). No async iterator.
//   statfs(path) -> {bsize, frsize, blocks, bfree, bavail, files, ffree,
//     favail, namemax} — libc::statvfs on the real host path (VFS overlay
//     is not consulted). Unix only; other platforms fail ENOSYS-flavoured.

struct DirHandle {
    entries: Vec<DirEntryInfo>,
    index: usize,
}

struct StatfsInfo {
    bsize: u64,
    frsize: u64,
    blocks: u64,
    bfree: u64,
    bavail: u64,
    files: u64,
    ffree: u64,
    favail: u64,
    namemax: u64,
}

thread_local! {
    static DIR_TABLE: RefCell<HashMap<i32, DirHandle>> = RefCell::new(HashMap::new());
    static NEXT_DIR_ID: Cell<i32> = const { Cell::new(1) };
}

/// Live `Dir` snapshots are retained until `close` — the VM has no GC
/// finalizer for them, so the table is capped to bound the leak.
const MAX_OPEN_DIRS: usize = 256;

fn dir_register(_path: String, entries: Vec<DirEntryInfo>) -> std::io::Result<i32> {
    let id = NEXT_DIR_ID.with(|next| {
        let id = next.get();
        next.set(id.saturating_add(1));
        id
    });
    DIR_TABLE.with(|table| {
        let mut table = table.borrow_mut();
        if table.len() >= MAX_OPEN_DIRS {
            return Err(std::io::Error::from_raw_os_error(libc::EMFILE));
        }
        table.insert(id, DirHandle { entries, index: 0 });
        Ok(id)
    })
}

fn dir_read_next(id: i32) -> std::io::Result<Option<DirEntryInfo>> {
    DIR_TABLE.with(|table| {
        let mut table = table.borrow_mut();
        let handle = table
            .get_mut(&id)
            .ok_or_else(|| std::io::Error::from_raw_os_error(libc::EBADF))?;
        if handle.index >= handle.entries.len() {
            return Ok(None);
        }
        let entry = DirEntryInfo {
            name: handle.entries[handle.index].name.clone(),
            is_file: handle.entries[handle.index].is_file,
            is_dir: handle.entries[handle.index].is_dir,
            is_symlink: handle.entries[handle.index].is_symlink,
        };
        handle.index += 1;
        Ok(Some(entry))
    })
}

fn dir_close(id: i32) -> std::io::Result<()> {
    let removed = DIR_TABLE.with(|table| table.borrow_mut().remove(&id));
    if removed.is_some() {
        Ok(())
    } else {
        Err(std::io::Error::from_raw_os_error(libc::EBADF))
    }
}

fn dir_id_of(scope: &mut v8::PinScope, this: v8::Local<v8::Object>) -> i32 {
    let key = v8::String::new(scope, "__dirId").unwrap();
    this.get(scope, key.into())
        .map(|v| v.int32_value(scope).unwrap_or(-1))
        .unwrap_or(-1)
}

fn dir_to_js<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    id: i32,
    path: &str,
) -> v8::Local<'a, v8::Value> {
    let obj = v8::Object::new(scope);
    let path_key = v8::String::new(scope, "path").unwrap();
    let path_val = v8::String::new(scope, path).unwrap();
    obj.set(scope, path_key.into(), path_val.into());
    let id_key = v8::String::new(scope, "__dirId").unwrap();
    obj.set(scope, id_key.into(), v8::Integer::new(scope, id).into());
    macro_rules! bind_dir_fn {
        ($name:expr, $cb:path) => {{
            let func = v8::FunctionTemplate::new(scope, $cb)
                .get_function(scope)
                .unwrap();
            let key = v8::String::new(scope, $name).unwrap();
            obj.set(scope, key.into(), func.into());
        }};
    }
    bind_dir_fn!("read", dir_read_callback);
    bind_dir_fn!("readSync", dir_read_sync_callback);
    bind_dir_fn!("close", dir_close_callback);
    bind_dir_fn!("closeSync", dir_close_sync_callback);
    obj.into()
}

fn dir_read_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let id = dir_id_of(scope, args.this());
    match dir_read_next(id) {
        Ok(Some(entry)) => retval.set(dirent_entry_to_js(scope, &entry)),
        Ok(None) => retval.set(v8::null(scope).into()),
        Err(err) => throw_fs_io_error(scope, "readdir", &id.to_string(), &err),
    }
}

fn dir_read_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let Some(callback) = require_callback(scope, &args, "dir.read") else {
        return;
    };
    let id = dir_id_of(scope, args.this());
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback,
            shape: FsCallbackShape::Value,
        },
        FsWork::DirRead { dir: id },
    );
    retval.set(v8::undefined(scope).into());
}

fn dir_close_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let id = dir_id_of(scope, args.this());
    match dir_close(id) {
        Ok(()) => retval.set(v8::undefined(scope).into()),
        Err(err) => throw_fs_io_error(scope, "close", &id.to_string(), &err),
    }
}

fn dir_close_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let Some(callback) = require_callback(scope, &args, "dir.close") else {
        return;
    };
    let id = dir_id_of(scope, args.this());
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback,
            shape: FsCallbackShape::Void,
        },
        FsWork::DirClose { dir: id },
    );
    retval.set(v8::undefined(scope).into());
}

/// Lexically normalize an absolute-ish path (resolve `.` / `..`) for the
/// dest-inside-src guard; never touches the filesystem.
fn normalize_lexical(path: &str) -> std::path::PathBuf {
    use std::path::Component;
    let mut out = std::path::PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn absolutize(path: &str) -> std::path::PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        normalize_lexical(path)
    } else {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        normalize_lexical(&cwd.join(path).to_string_lossy())
    }
}

fn cp_path(
    from: &str,
    to: &str,
    recursive: bool,
    force: bool,
    error_on_exist: bool,
) -> std::io::Result<()> {
    // ERR_FS_CP_EINVAL equivalent: cannot copy into a subdirectory of self.
    let src_abs = absolutize(from);
    let dest_abs = absolutize(to);
    if dest_abs == src_abs || dest_abs.starts_with(&src_abs) {
        return Err(std::io::Error::from_raw_os_error(libc::EINVAL));
    }
    if crate::sandbox::virtual_fs::is_enabled() {
        let meta = crate::sandbox::virtual_fs::vfs_metadata(Path::new(from))?;
        if meta.is_dir {
            if !recursive {
                return Err(std::io::Error::from_raw_os_error(libc::EISDIR));
            }
            cp_dir_vfs(from, to, force, error_on_exist)
        } else {
            cp_file_vfs(from, to, force, error_on_exist)
        }
    } else {
        // metadata() follows symlinks: cp dereferences them (verbatimSymlinks
        // is out of this carve).
        let meta = std::fs::metadata(from)?;
        // Lexical containment misses symlinked paths: `cp link-to-src
        // real-src/dst` must also fail EINVAL. Resolve the deepest existing
        // ancestor of dest so a not-yet-created tail still compares real.
        let src_real = realpath_deepest(from)?;
        let dst_real = realpath_deepest(to)?;
        if dst_real == src_real || dst_real.starts_with(&src_real) {
            return Err(std::io::Error::from_raw_os_error(libc::EINVAL));
        }
        if meta.is_dir() {
            if !recursive {
                return Err(std::io::Error::from_raw_os_error(libc::EISDIR));
            }
            cp_dir_real(from, to, force, error_on_exist)
        } else {
            cp_file_real(from, to, force, error_on_exist)
        }
    }
}

/// Canonicalize the deepest existing ancestor of `path`, then re-attach the
/// missing tail. Lets the containment guard see through symlinks even when
/// the destination itself does not exist yet.
fn realpath_deepest(path: &str) -> std::io::Result<std::path::PathBuf> {
    let mut cursor = std::path::PathBuf::from(path);
    loop {
        match std::fs::canonicalize(&cursor) {
            Ok(real) => {
                let tail = Path::new(path)
                    .strip_prefix(&cursor)
                    .unwrap_or_else(|_| Path::new(""));
                return Ok(real.join(tail));
            }
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    if !cursor.pop() {
                        return Err(err);
                    }
                } else {
                    return Err(err);
                }
            }
        }
    }
}

/// Recursive `cp` expands child paths under a root grant; the broker may
/// deny paths beneath an allowed root, so every copied entry is checked.
fn cp_entry_allowed(action: PermissionAction, path: &Path) -> std::io::Result<()> {
    check_global_permission(
        PermissionKind::FileSystem,
        action,
        ResourceId::Path(path.to_path_buf()),
    )
    .map_err(|_| std::io::Error::from_raw_os_error(libc::EACCES))
}

fn cp_file_real(from: &str, to: &str, force: bool, error_on_exist: bool) -> std::io::Result<()> {
    cp_entry_allowed(PermissionAction::Read, Path::new(from))?;
    cp_entry_allowed(PermissionAction::Write, Path::new(to))?;
    if let Ok(dest) = std::fs::symlink_metadata(to) {
        if dest.is_dir() {
            return Err(std::io::Error::from_raw_os_error(libc::EISDIR));
        }
        if !force {
            if error_on_exist {
                return Err(std::io::Error::from_raw_os_error(libc::EEXIST));
            }
            return Ok(());
        }
    }
    std::fs::copy(from, to)?;
    Ok(())
}

fn cp_dir_real(from: &str, to: &str, force: bool, error_on_exist: bool) -> std::io::Result<()> {
    cp_entry_allowed(PermissionAction::Read, Path::new(from))?;
    cp_entry_allowed(PermissionAction::Write, Path::new(to))?;
    if let Ok(dest) = std::fs::symlink_metadata(to) {
        if !dest.is_dir() {
            // Node: copying a dir onto a non-dir is always an error —
            // `force` only governs same-type overwrites.
            return Err(std::io::Error::from_raw_os_error(libc::ENOTDIR));
        }
    } else {
        std::fs::create_dir_all(to)?;
    }
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        let child_src = entry.path();
        let child_dst = Path::new(to).join(&name);
        let child_src_str = child_src.to_string_lossy();
        let child_dst_str = child_dst.to_string_lossy();
        // metadata() follows links → symlinked dirs recurse as real dirs.
        let meta = std::fs::metadata(&child_src)?;
        if meta.is_dir() {
            cp_dir_real(&child_src_str, &child_dst_str, force, error_on_exist)?;
        } else {
            cp_file_real(&child_src_str, &child_dst_str, force, error_on_exist)?;
        }
    }
    Ok(())
}

fn cp_file_vfs(from: &str, to: &str, force: bool, error_on_exist: bool) -> std::io::Result<()> {
    cp_entry_allowed(PermissionAction::Read, Path::new(from))?;
    cp_entry_allowed(PermissionAction::Write, Path::new(to))?;
    debug_assert!(crate::sandbox::virtual_fs::is_enabled());
    if let Ok(dest) = crate::sandbox::virtual_fs::vfs_metadata(Path::new(to)) {
        if dest.is_dir {
            return Err(std::io::Error::from_raw_os_error(libc::EISDIR));
        }
        if !force {
            if error_on_exist {
                return Err(std::io::Error::from_raw_os_error(libc::EEXIST));
            }
            return Ok(());
        }
    }
    let bytes = crate::sandbox::virtual_fs::vfs_read(Path::new(from))?;
    crate::sandbox::virtual_fs::vfs_write(Path::new(to), &bytes)
}

fn cp_dir_vfs(from: &str, to: &str, force: bool, error_on_exist: bool) -> std::io::Result<()> {
    cp_entry_allowed(PermissionAction::Read, Path::new(from))?;
    cp_entry_allowed(PermissionAction::Write, Path::new(to))?;
    if let Ok(dest) = crate::sandbox::virtual_fs::vfs_metadata(Path::new(to)) {
        if !dest.is_dir {
            // Same as the real path: dir onto non-dir is ENOTDIR regardless
            // of `force`.
            return Err(std::io::Error::from_raw_os_error(libc::ENOTDIR));
        }
    } else {
        crate::sandbox::virtual_fs::vfs_create_dir_all(Path::new(to))?;
    }
    for name in crate::sandbox::virtual_fs::vfs_read_dir(Path::new(from))? {
        let child_src = Path::new(from).join(&name);
        let child_dst = Path::new(to).join(&name);
        let meta = crate::sandbox::virtual_fs::vfs_metadata(&child_src)?;
        if meta.is_dir {
            cp_dir_vfs(
                &child_src.to_string_lossy(),
                &child_dst.to_string_lossy(),
                force,
                error_on_exist,
            )?;
        } else {
            cp_file_vfs(
                &child_src.to_string_lossy(),
                &child_dst.to_string_lossy(),
                force,
                error_on_exist,
            )?;
        }
    }
    Ok(())
}

fn statfs_info(path: &str) -> std::io::Result<StatfsInfo> {
    // statvfs runs against the real host filesystem. Under the sandbox VFS
    // there is no host mapping and answering would leak host stats — fail
    // closed instead.
    if crate::sandbox::virtual_fs::is_enabled() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "statfs is unavailable under the sandbox VFS",
        ));
    }
    #[cfg(unix)]
    {
        let c_path = std::ffi::CString::new(path)
            .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?;
        let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) } != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(StatfsInfo {
            bsize: stat.f_bsize as u64,
            frsize: stat.f_frsize as u64,
            blocks: stat.f_blocks as u64,
            bfree: stat.f_bfree as u64,
            bavail: stat.f_bavail as u64,
            files: stat.f_files as u64,
            ffree: stat.f_ffree as u64,
            favail: stat.f_favail as u64,
            namemax: stat.f_namemax as u64,
        })
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "statfs is unix-only",
        ))
    }
}

fn statfs_to_js<'a>(
    scope: &mut v8::PinScope<'a, '_>,
    info: &StatfsInfo,
) -> v8::Local<'a, v8::Value> {
    let obj = v8::Object::new(scope);
    macro_rules! set_num {
        ($name:expr, $val:expr) => {{
            let key = v8::String::new(scope, $name).unwrap();
            let val = v8::Number::new(scope, $val as f64);
            obj.set(scope, key.into(), val.into());
        }};
    }
    set_num!("bsize", info.bsize);
    set_num!("frsize", info.frsize);
    set_num!("blocks", info.blocks);
    set_num!("bfree", info.bfree);
    set_num!("bavail", info.bavail);
    set_num!("files", info.files);
    set_num!("ffree", info.ffree);
    set_num!("favail", info.favail);
    set_num!("namemax", info.namemax);
    obj.into()
}

/// `{recursive, force, errorOnExist}` option bag for fs.cp. Anything else
/// (filter, dereference, verbatimSymlinks, preserveTimestamps) is out of the
/// G76 carve.
fn cp_options(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> (bool, bool, bool) {
    let get_bool = |name: &str, default: bool| -> bool {
        v8::Local::<v8::Object>::try_from(value)
            .ok()
            .and_then(|obj| {
                let key = v8::String::new(scope, name).unwrap();
                obj.get(scope, key.into())
            })
            .map(|v| v.to_boolean(scope).boolean_value(scope))
            .unwrap_or(default)
    };
    (
        get_bool("recursive", false),
        get_bool("force", true),
        get_bool("errorOnExist", false),
    )
}

fn fs_cp_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let from = arg_text(scope, args.get(0));
    let to = arg_text(scope, args.get(1));
    let (recursive, force, error_on_exist) = cp_options(scope, args.get(2));
    if !ensure_fs_permission(scope, PermissionAction::Read, &from)
        || !ensure_fs_permission(scope, PermissionAction::Write, &to)
    {
        return;
    }
    match cp_path(&from, &to, recursive, force, error_on_exist) {
        Ok(()) => retval.set(v8::undefined(scope).into()),
        Err(err) => throw_fs_io_error(scope, "cp", &from, &err),
    }
}

fn fs_opendir_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    match read_directory(&path, true).and_then(|entries| dir_register(path.clone(), entries)) {
        Ok(id) => {
            retval.set(dir_to_js(scope, id, &path));
        }
        Err(err) => throw_fs_io_error(scope, "opendir", &path, &err),
    }
}

fn fs_statfs_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    match statfs_info(&path) {
        Ok(info) => retval.set(statfs_to_js(scope, &info)),
        Err(err) => throw_fs_io_error(scope, "statfs", &path, &err),
    }
}

fn fs_cp_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let Some(callback) = require_callback(scope, &args, "cp") else {
        return;
    };
    let from = arg_text(scope, args.get(0));
    let to = arg_text(scope, args.get(1));
    let (recursive, force, error_on_exist) = cp_options(scope, args.get(2));
    if !ensure_fs_permission(scope, PermissionAction::Read, &from)
        || !ensure_fs_permission(scope, PermissionAction::Write, &to)
    {
        return;
    }
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback,
            shape: FsCallbackShape::Void,
        },
        FsWork::Cp {
            from,
            to,
            recursive,
            force,
            error_on_exist,
        },
    );
    retval.set(v8::undefined(scope).into());
}

fn fs_opendir_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let Some(callback) = require_callback(scope, &args, "opendir") else {
        return;
    };
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback,
            shape: FsCallbackShape::Value,
        },
        FsWork::Opendir { path },
    );
    retval.set(v8::undefined(scope).into());
}

fn fs_statfs_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let Some(callback) = require_callback(scope, &args, "statfs") else {
        return;
    };
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    enqueue_fs_job(
        scope,
        FsDelivery::Callback {
            callback,
            shape: FsCallbackShape::Value,
        },
        FsWork::Statfs { path },
    );
    retval.set(v8::undefined(scope).into());
}

fn fs_promises_cp_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    let from = arg_text(scope, args.get(0));
    let to = arg_text(scope, args.get(1));
    let (recursive, force, error_on_exist) = cp_options(scope, args.get(2));
    if !ensure_fs_permission(scope, PermissionAction::Read, &from)
        || !ensure_fs_permission(scope, PermissionAction::Write, &to)
    {
        return;
    }
    promise_method(
        scope,
        retval,
        FsWork::Cp {
            from,
            to,
            recursive,
            force,
            error_on_exist,
        },
    );
}

fn fs_promises_opendir_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    promise_method(scope, retval, FsWork::Opendir { path });
}

fn fs_promises_statfs_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    retval: v8::ReturnValue,
) {
    let path = arg_text(scope, args.get(0));
    if !ensure_fs_permission(scope, PermissionAction::Read, &path) {
        return;
    }
    promise_method(scope, retval, FsWork::Statfs { path });
}
