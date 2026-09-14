// Node.js tty 核心模块实现
// 提供终端信息检测与 Stream 支持

use anyhow::Result;
use rusty_v8 as v8;

/// 判断文件描述符是否为 TTY
fn is_terminal_fd(fd: i32) -> bool {
    #[cfg(unix)]
    {
        if fd < 0 {
            return false;
        }
        unsafe { libc::isatty(fd) == 1 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// tty.isatty(fd) 回调
fn tty_isatty_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if args.length() == 0 {
        let res = v8::Boolean::new(scope, false);
        retval.set(res.into());
        return;
    }

    let fd_val = args.get(0);
    if !fd_val.is_number() {
        let res = v8::Boolean::new(scope, false);
        retval.set(res.into());
        return;
    }

    let fd = fd_val.int32_value(scope).unwrap_or(-1);
    let is_tty = is_terminal_fd(fd);
    let res = v8::Boolean::new(scope, is_tty);
    retval.set(res.into());
}

/// tty.ReadStream 构造函数
fn tty_read_stream_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let fd = if args.length() > 0 && args.get(0).is_number() {
        args.get(0).int32_value(scope).unwrap_or(0)
    } else {
        0
    };

    let fd_key = v8::String::new(scope, "fd").unwrap();
    let is_tty_key = v8::String::new(scope, "isTTY").unwrap();
    let is_raw_key = v8::String::new(scope, "isRaw").unwrap();

    let is_tty = is_terminal_fd(fd);
    let fd_val = v8::Integer::new(scope, fd);
    let is_tty_val = v8::Boolean::new(scope, is_tty);
    let is_raw_val = v8::Boolean::new(scope, false);

    this.set(scope, fd_key.into(), fd_val.into());
    this.set(scope, is_tty_key.into(), is_tty_val.into());
    this.set(scope, is_raw_key.into(), is_raw_val.into());

    // setRawMode 方法
    let set_raw_mode_key = v8::String::new(scope, "setRawMode").unwrap();
    let set_raw_mode_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let this = args.this();
            let is_raw = if args.length() > 0 {
                args.get(0).boolean_value(scope)
            } else {
                false
            };
            let is_raw_key = v8::String::new(scope, "isRaw").unwrap();
            let is_raw_val = v8::Boolean::new(scope, is_raw);
            this.set(scope, is_raw_key.into(), is_raw_val.into());
            retval.set(this.into());
        },
    );
    let set_raw_mode_val = set_raw_mode_fn.get_function(scope).unwrap();
    this.set(scope, set_raw_mode_key.into(), set_raw_mode_val.into());

    retval.set(this.into());
}

/// tty.WriteStream 构造函数
fn tty_write_stream_constructor(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let fd = if args.length() > 0 && args.get(0).is_number() {
        args.get(0).int32_value(scope).unwrap_or(1)
    } else {
        1
    };

    let fd_key = v8::String::new(scope, "fd").unwrap();
    let is_tty_key = v8::String::new(scope, "isTTY").unwrap();
    let columns_key = v8::String::new(scope, "columns").unwrap();
    let rows_key = v8::String::new(scope, "rows").unwrap();

    let is_tty = is_terminal_fd(fd);
    let fd_val = v8::Integer::new(scope, fd);
    let is_tty_val = v8::Boolean::new(scope, is_tty);
    let columns_val = v8::Integer::new(scope, 80);
    let rows_val = v8::Integer::new(scope, 24);

    this.set(scope, fd_key.into(), fd_val.into());
    this.set(scope, is_tty_key.into(), is_tty_val.into());
    this.set(scope, columns_key.into(), columns_val.into());
    this.set(scope, rows_key.into(), rows_val.into());

    // getColorDepth
    let get_color_depth_key = v8::String::new(scope, "getColorDepth").unwrap();
    let get_color_depth_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let depth = v8::Integer::new(scope, 8);
            retval.set(depth.into());
        },
    );
    let get_color_depth_val = get_color_depth_fn.get_function(scope).unwrap();
    this.set(
        scope,
        get_color_depth_key.into(),
        get_color_depth_val.into(),
    );

    // hasColors
    let has_colors_key = v8::String::new(scope, "hasColors").unwrap();
    let has_colors_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let res = v8::Boolean::new(scope, true);
            retval.set(res.into());
        },
    );
    let has_colors_val = has_colors_fn.get_function(scope).unwrap();
    this.set(scope, has_colors_key.into(), has_colors_val.into());

    // getWindowSize
    let get_window_size_key = v8::String::new(scope, "getWindowSize").unwrap();
    let get_window_size_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let arr = v8::Array::new(scope, 2);
            let col = v8::Integer::new(scope, 80);
            let row = v8::Integer::new(scope, 24);
            arr.set_index(scope, 0, col.into());
            arr.set_index(scope, 1, row.into());
            retval.set(arr.into());
        },
    );
    let get_window_size_val = get_window_size_fn.get_function(scope).unwrap();
    this.set(
        scope,
        get_window_size_key.into(),
        get_window_size_val.into(),
    );

    retval.set(this.into());
}

/// 设置 TTY API 到全局上下文与模块
pub fn setup_tty_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let tty_obj = v8::Object::new(scope);

    // isatty 函数
    let isatty_key = v8::String::new(scope, "isatty").unwrap();
    let isatty_tmpl = v8::FunctionTemplate::new(scope, tty_isatty_callback);
    let isatty_fn = isatty_tmpl.get_function(scope).unwrap();
    tty_obj.set(scope, isatty_key.into(), isatty_fn.into());

    // ReadStream 类
    let read_stream_key = v8::String::new(scope, "ReadStream").unwrap();
    let read_stream_tmpl = v8::FunctionTemplate::new(scope, tty_read_stream_constructor);
    let read_stream_fn = read_stream_tmpl.get_function(scope).unwrap();
    tty_obj.set(scope, read_stream_key.into(), read_stream_fn.into());

    // WriteStream 类
    let write_stream_key = v8::String::new(scope, "WriteStream").unwrap();
    let write_stream_tmpl = v8::FunctionTemplate::new(scope, tty_write_stream_constructor);
    let write_stream_fn = write_stream_tmpl.get_function(scope).unwrap();
    tty_obj.set(scope, write_stream_key.into(), write_stream_fn.into());

    // 设置为全局 tty
    let global = context.global(scope);
    let tty_global_key = v8::String::new(scope, "tty").unwrap();
    global.set(scope, tty_global_key.into(), tty_obj.into());

    Ok(())
}
