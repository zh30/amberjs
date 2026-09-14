// Node.js process 模块实现
// v0.3.237: 全局未捕获异常处理器和 process 对象
// v0.3.239: 完善 nextTick 和 stdout/stderr.write()
// v0.3.240: 完善 hrtime、stdin、memory、uptime、cpuUsage

use anyhow::Result;
use rusty_v8 as v8;
use std::io::Write;
use std::sync::Mutex;
use std::time::Instant;

thread_local! {
    /// 进程启动时间（用于计算 uptime）
    static START_TIME: Instant = Instant::now();
}

thread_local! {
    /// 未捕获异常处理器
    static UNCAUGHT_EXCEPTION_HANDLERS: Mutex<Vec<v8::Global<v8::Value>>> = Mutex::new(Vec::new());
    /// 未处理的 Promise rejection 处理器
    static UNHANDLED_REJECTION_HANDLERS: Mutex<Vec<v8::Global<v8::Value>>> = Mutex::new(Vec::new());
    /// 程序是否应该退出
    static SHOULD_EXIT: Mutex<bool> = Mutex::new(false);
    /// 退出码
    static EXIT_CODE: Mutex<i32> = Mutex::new(0);
}

// v0.3.239: nextTick 队列（线程本地）
// Note: thread_local doesn't have pub(crate) on individual items, so we export the whole thing
mod next_tick_queue_mod {
    use rusty_v8 as v8;
    use std::sync::Mutex;

    pub(crate) struct NextTickCallback {
        pub(crate) callback: v8::Global<v8::Value>,
        pub(crate) args: Vec<v8::Global<v8::Value>>,
    }

    thread_local! {
        pub(crate) static NEXT_TICK_QUEUE: Mutex<Vec<NextTickCallback>> = Mutex::new(Vec::new());
    }
}

use next_tick_queue_mod::{NextTickCallback, NEXT_TICK_QUEUE};

/// v0.3.261: 添加 nextTick 回调到队列（供 runtime_minimal.rs 使用）
pub fn push_next_tick_callback(callback: v8::Global<v8::Value>, args: Vec<v8::Global<v8::Value>>) {
    NEXT_TICK_QUEUE.with(|queue| {
        let mut q = queue.lock().unwrap();
        q.push(NextTickCallback { callback, args });
    });
}

/// v0.3.261: 执行所有 pending 的 nextTick 回调
/// 必须在 V8 主线程调用，在 perform_microtask_checkpoint 之前执行
/// 这样 nextTick 回调会在 Promise microtasks 之前执行（符合 Node.js 行为）
/// 关键：使用 while 循环确保所有链式添加的 nextTick 都能执行
pub fn execute_next_tick_callbacks(scope: &mut v8::PinScope) {
    NEXT_TICK_QUEUE.with(|q| {
        let mut queue_ref = q.lock().unwrap();

        // 使用 while 循环处理链式添加的 nextTicks
        // 每次迭代取出当前所有回调，执行它们
        // 如果回调中添加了新的 nextTick，它们会在下一轮迭代中执行
        while !queue_ref.is_empty() {
            // 取出当前所有回调（使用 take 清空队列）
            let callbacks: Vec<NextTickCallback> = std::mem::take(&mut *queue_ref);

            // 释放锁以便回调中可以安全地添加新的 nextTick
            drop(queue_ref);

            // 执行当前这批回调
            for NextTickCallback { callback, args } in callbacks.into_iter() {
                let callback_local = v8::Local::new(scope, callback);
                if let Ok(func) = v8::Local::<v8::Function>::try_from(callback_local) {
                    let undefined = v8::undefined(scope);
                    // Convert Global args to Local args for the function call
                    let args_local: Vec<v8::Local<v8::Value>> =
                        args.iter().map(|g| v8::Local::new(scope, g)).collect();
                    let _ = func.call(scope, undefined.into(), &args_local);
                }
            }

            // 重新获取锁，继续处理可能添加的新回调
            queue_ref = q.lock().unwrap();
        }
    });
}

/// v0.3.261: 检查是否有 pending 的 nextTick 回调
pub fn has_pending_next_ticks() -> bool {
    NEXT_TICK_QUEUE.with(|q| {
        let queue = q.lock().unwrap();
        !queue.is_empty()
    })
}

/// v0.3.261: 清空 nextTick 队列（用于测试清理）
pub fn clear_next_tick_queue() {
    NEXT_TICK_QUEUE.with(|q| {
        let mut queue = q.lock().unwrap();
        queue.clear();
    });
}

// v0.3.242: setMaxListeners 存储
// 存储每个事件类型的最大监听器数量，0 表示无限制
thread_local! {
    static MAX_LISTENERS: Mutex<std::collections::HashMap<String, i32>> =
        Mutex::new(std::collections::HashMap::new());
}

/// 设置 process 全局对象
pub fn setup_process_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // 创建 process 对象
    let process_obj = v8::Object::new(scope);

    // process.version - Node.js 版本
    let version_key = v8::String::new(scope, "version").unwrap();
    let version_val = v8::String::new(scope, "v20.0.0").unwrap();
    process_obj.set(scope, version_key.into(), version_val.into());

    // process.platform - 平台信息
    let platform_key = v8::String::new(scope, "platform").unwrap();
    #[cfg(target_os = "macos")]
    let platform_val = v8::String::new(scope, "darwin").unwrap();
    #[cfg(target_os = "linux")]
    let platform_val = v8::String::new(scope, "linux").unwrap();
    #[cfg(target_os = "windows")]
    let platform_val = v8::String::new(scope, "win32").unwrap();
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let platform_val = v8::String::new(scope, "unknown").unwrap();
    process_obj.set(scope, platform_key.into(), platform_val.into());

    // process.arch - 架构信息
    let arch_key = v8::String::new(scope, "arch").unwrap();
    #[cfg(target_arch = "x86_64")]
    let arch_val = v8::String::new(scope, "x64").unwrap();
    #[cfg(target_arch = "aarch64")]
    let arch_val = v8::String::new(scope, "arm64").unwrap();
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    let arch_val = v8::String::new(scope, "unknown").unwrap();
    process_obj.set(scope, arch_key.into(), arch_val.into());

    // process.argv - 命令行参数
    let argv_key = v8::String::new(scope, "argv").unwrap();
    let argv_array = v8::Array::new(scope, 0);
    // 默认参数
    let arg0 = v8::String::new(scope, "bee").unwrap();
    argv_array.set_index(scope, 0, arg0.into());
    let arg1 = v8::String::new(scope, "script.js").unwrap();
    argv_array.set_index(scope, 1, arg1.into());
    process_obj.set(scope, argv_key.into(), argv_array.into());

    // process.execPath - 可执行文件路径
    let exec_path_key = v8::String::new(scope, "execPath").unwrap();
    let exec_path_val = v8::String::new(scope, "/usr/local/bin/bee").unwrap();
    process_obj.set(scope, exec_path_key.into(), exec_path_val.into());

    // process.cwd() - 获取当前工作目录
    let cwd_func = v8::FunctionTemplate::new(scope, process_cwd_callback);
    let cwd_instance = cwd_func.get_function(scope).unwrap();
    let cwd_key = v8::String::new(scope, "cwd").unwrap();
    process_obj.set(scope, cwd_key.into(), cwd_instance.into());

    // v0.3.239: process.nextTick() - 微任务队列优先级
    let next_tick_func = v8::FunctionTemplate::new(scope, process_next_tick_callback);
    let next_tick_instance = next_tick_func.get_function(scope).unwrap();
    let next_tick_key = v8::String::new(scope, "nextTick").unwrap();
    process_obj.set(scope, next_tick_key.into(), next_tick_instance.into());

    // process.exit() - 退出程序
    let exit_func = v8::FunctionTemplate::new(scope, process_exit_callback);
    let exit_instance = exit_func.get_function(scope).unwrap();
    let exit_key = v8::String::new(scope, "exit").unwrap();
    process_obj.set(scope, exit_key.into(), exit_instance.into());

    // process.on() - 事件监听
    let on_func = v8::FunctionTemplate::new(scope, process_on_callback);
    let on_instance = on_func.get_function(scope).unwrap();
    let on_key = v8::String::new(scope, "on").unwrap();
    process_obj.set(scope, on_key.into(), on_instance.into());

    // process.off() - 移除事件监听
    let off_func = v8::FunctionTemplate::new(scope, process_off_callback);
    let off_instance = off_func.get_function(scope).unwrap();
    let off_key = v8::String::new(scope, "off").unwrap();
    process_obj.set(scope, off_key.into(), off_instance.into());

    // process.removeListener() - 移除特定监听器
    let remove_listener_func = v8::FunctionTemplate::new(scope, process_remove_listener_callback);
    let remove_listener_instance = remove_listener_func.get_function(scope).unwrap();
    let remove_listener_key = v8::String::new(scope, "removeListener").unwrap();
    process_obj.set(
        scope,
        remove_listener_key.into(),
        remove_listener_instance.into(),
    );

    // v0.3.242: process.setMaxListeners() - 设置最大监听器数量
    let set_max_listeners_func =
        v8::FunctionTemplate::new(scope, process_set_max_listeners_callback);
    let set_max_listeners_instance = set_max_listeners_func.get_function(scope).unwrap();
    let set_max_listeners_key = v8::String::new(scope, "setMaxListeners").unwrap();
    process_obj.set(
        scope,
        set_max_listeners_key.into(),
        set_max_listeners_instance.into(),
    );

    // v0.3.242: process.getMaxListeners() - 获取最大监听器数量
    let get_max_listeners_func =
        v8::FunctionTemplate::new(scope, process_get_max_listeners_callback);
    let get_max_listeners_instance = get_max_listeners_func.get_function(scope).unwrap();
    let get_max_listeners_key = v8::String::new(scope, "getMaxListeners").unwrap();
    process_obj.set(
        scope,
        get_max_listeners_key.into(),
        get_max_listeners_instance.into(),
    );

    // process.dlopen() - 加载原生 C/C++ .node 扩展模块
    let dlopen_func = v8::FunctionTemplate::new(scope, process_dlopen_callback);
    let dlopen_instance = dlopen_func.get_function(scope).unwrap();
    let dlopen_key = v8::String::new(scope, "dlopen").unwrap();
    process_obj.set(scope, dlopen_key.into(), dlopen_instance.into());

    // process.pid - 进程 ID
    let pid_key = v8::String::new(scope, "pid").unwrap();
    let pid_val = v8::Integer::new(scope, std::process::id() as i32);
    process_obj.set(scope, pid_key.into(), pid_val.into());

    // process.ppid - 父进程 ID
    let ppid_key = v8::String::new(scope, "ppid").unwrap();
    let ppid_val = v8::Integer::new(scope, 1); // 默认 1
    process_obj.set(scope, ppid_key.into(), ppid_val.into());

    // process.env - 环境变量
    let env_key = v8::String::new(scope, "env").unwrap();
    let env_obj = v8::Object::new(scope);
    // 添加常见环境变量
    let home_key = v8::String::new(scope, "HOME").unwrap();
    let home_val =
        v8::String::new(scope, std::env::var("HOME").unwrap_or_default().as_str()).unwrap();
    env_obj.set(scope, home_key.into(), home_val.into());
    let path_key = v8::String::new(scope, "PATH").unwrap();
    let path_val =
        v8::String::new(scope, std::env::var("PATH").unwrap_or_default().as_str()).unwrap();
    env_obj.set(scope, path_key.into(), path_val.into());
    process_obj.set(scope, env_key.into(), env_obj.into());

    // process.release - 发布信息
    let release_key = v8::String::new(scope, "release").unwrap();
    let release_obj = v8::Object::new(scope);
    let name_key = v8::String::new(scope, "name").unwrap();
    let name_val = v8::String::new(scope, "bee").unwrap();
    release_obj.set(scope, name_key.into(), name_val.into());
    process_obj.set(scope, release_key.into(), release_obj.into());

    // v0.3.239: process.stdout - 标准输出（带 write() 方法）
    let stdout_key = v8::String::new(scope, "stdout").unwrap();
    let stdout_obj = v8::Object::new(scope);
    let stdout_write_func = v8::FunctionTemplate::new(scope, stdout_write_callback);
    let stdout_write_instance = stdout_write_func.get_function(scope).unwrap();
    let stdout_write_key = v8::String::new(scope, "write").unwrap();
    stdout_obj.set(scope, stdout_write_key.into(), stdout_write_instance.into());
    process_obj.set(scope, stdout_key.into(), stdout_obj.into());

    // v0.3.239: process.stderr - 标准错误（带 write() 方法）
    let stderr_key = v8::String::new(scope, "stderr").unwrap();
    let stderr_obj = v8::Object::new(scope);
    let stderr_write_func = v8::FunctionTemplate::new(scope, stderr_write_callback);
    let stderr_write_instance = stderr_write_func.get_function(scope).unwrap();
    let stderr_write_key = v8::String::new(scope, "write").unwrap();
    stderr_obj.set(scope, stderr_write_key.into(), stderr_write_instance.into());
    process_obj.set(scope, stderr_key.into(), stderr_obj.into());

    // v0.3.240: process.stdin - 标准输入
    let stdin_key = v8::String::new(scope, "stdin").unwrap();
    let stdin_obj = v8::Object::new(scope);
    // stdin.fd - 标准输入的文件描述符 (0)
    let stdin_fd_key = v8::String::new(scope, "fd").unwrap();
    let stdin_fd_val = v8::Integer::new(scope, 0);
    stdin_obj.set(scope, stdin_fd_key.into(), stdin_fd_val.into());
    // stdin.read() - 读取输入（同步版本返回 null）
    let stdin_read_func = v8::FunctionTemplate::new(scope, stdin_read_callback);
    let stdin_read_instance = stdin_read_func.get_function(scope).unwrap();
    let stdin_read_key = v8::String::new(scope, "read").unwrap();
    stdin_obj.set(scope, stdin_read_key.into(), stdin_read_instance.into());
    process_obj.set(scope, stdin_key.into(), stdin_obj.into());

    // v0.3.240: process.hrtime() - 高精度时间
    let hrtime_func = v8::FunctionTemplate::new(scope, process_hrtime_callback);
    let hrtime_instance = hrtime_func.get_function(scope).unwrap();
    let hrtime_key = v8::String::new(scope, "hrtime").unwrap();
    process_obj.set(scope, hrtime_key.into(), hrtime_instance.into());

    // v0.3.240: process.memory() - 内存使用统计
    let memory_func = v8::FunctionTemplate::new(scope, process_memory_callback);
    let memory_instance = memory_func.get_function(scope).unwrap();
    let memory_key = v8::String::new(scope, "memory").unwrap();
    process_obj.set(scope, memory_key.into(), memory_instance.into());

    // v0.3.240: process.uptime() - 进程运行时间
    let uptime_func = v8::FunctionTemplate::new(scope, process_uptime_callback);
    let uptime_instance = uptime_func.get_function(scope).unwrap();
    let uptime_key = v8::String::new(scope, "uptime").unwrap();
    process_obj.set(scope, uptime_key.into(), uptime_instance.into());

    // v0.3.240: process.cpuUsage() - CPU 使用统计
    let cpu_usage_func = v8::FunctionTemplate::new(scope, process_cpu_usage_callback);
    let cpu_usage_instance = cpu_usage_func.get_function(scope).unwrap();
    let cpu_usage_key = v8::String::new(scope, "cpuUsage").unwrap();
    process_obj.set(scope, cpu_usage_key.into(), cpu_usage_instance.into());

    // v0.3.243: process.kill(pid, signal) - 向进程发送信号
    let kill_func = v8::FunctionTemplate::new(scope, process_kill_callback);
    let kill_instance = kill_func.get_function(scope).unwrap();
    let kill_key = v8::String::new(scope, "kill").unwrap();
    process_obj.set(scope, kill_key.into(), kill_instance.into());

    // 将 process 对象设置为全局
    let process_key = v8::String::new(scope, "process").unwrap();
    global.set(scope, process_key.into(), process_obj.into());

    Ok(())
}

/// process.cwd() 回调
fn process_cwd_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "/".to_string());
    let cwd_str = v8::String::new(scope, &cwd).unwrap();
    retval.set(cwd_str.into());
}

/// process.dlopen(module, filename, [flags]) - 原生扩展模块动态链接加载
pub fn process_dlopen_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if args.length() < 2 {
        let msg = v8::String::new(
            scope,
            "process.dlopen requires at least module and filename arguments",
        )
        .unwrap();
        let err = v8::Exception::type_error(scope, msg);
        scope.throw_exception(err);
        return;
    }

    let module_val = args.get(0);
    let filename_val = args.get(1);

    if !module_val.is_object() || !filename_val.is_string() {
        let msg = v8::String::new(scope, "Invalid arguments to process.dlopen").unwrap();
        let err = v8::Exception::type_error(scope, msg);
        scope.throw_exception(err);
        return;
    }

    let filename = filename_val.to_rust_string_lossy(scope);
    let path = std::path::Path::new(&filename);

    if !path.exists() {
        let msg = v8::String::new(
            scope,
            &format!("Cannot find native addon module '{}'", filename),
        )
        .unwrap();
        let err = v8::Exception::error(scope, msg);
        scope.throw_exception(err);
        return;
    }

    let module_obj = match v8::Local::<v8::Object>::try_from(module_val) {
        Ok(obj) => obj,
        Err(_) => {
            let msg = v8::String::new(scope, "process.dlopen module must be an object").unwrap();
            let err = v8::Exception::type_error(scope, msg);
            scope.throw_exception(err);
            return;
        }
    };

    crate::napi::load_napi_addon(scope, module_obj, &filename);
    retval.set(v8::undefined(scope).into());
}

/// v0.3.239: process.nextTick() 回调
/// v0.3.261: 重构 - 只将回调添加到队列，由事件循环在正确时机执行
/// 正确的执行顺序: nextTick -> microtasks (Promises) -> timers -> setImmediate
fn process_next_tick_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _retval: v8::ReturnValue,
) {
    let callback = args.get(0);

    if !callback.is_function() {
        let error =
            v8::String::new(scope, "process.nextTick: callback must be a function").unwrap();
        let error_obj = v8::Exception::type_error(scope, error);
        scope.throw_exception(error_obj.into());
        return;
    }

    // 收集额外参数
    let callback_args: Vec<v8::Global<v8::Value>> = (1..args.length())
        .map(|i| args.get(i))
        .filter(|v| !v.is_undefined())
        .map(|v| v8::Global::new(scope, v))
        .collect();

    // 保存到 nextTick 队列
    // 回调将在事件循环的 execute_next_tick_callbacks 中执行
    let callback_global = v8::Global::new(scope, callback);
    NEXT_TICK_QUEUE.with(|queue| {
        let mut q = queue.lock().unwrap();
        q.push(NextTickCallback {
            callback: callback_global,
            args: callback_args,
        });
    });
}

// 获取 context 的辅助函数 - v0.3.265: 预留用于未来使用
#[allow(dead_code)]
fn _context<'s>(scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Context> {
    scope.get_current_context()
}

/// v0.3.239: stdout.write() 回调
fn stdout_write_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let data = args.get(0);

    // 将 V8 值转换为字符串并输出到 stdout
    let output = if let Some(str_val) = data.to_string(scope) {
        str_val.to_rust_string_lossy(scope)
    } else if data.is_null_or_undefined() {
        String::new()
    } else {
        String::from("[object]")
    };

    // 输出到 stdout 并刷新
    let mut stdout = std::io::stdout();
    let _ = write!(stdout, "{}", output);
    let _ = stdout.flush();

    // 返回 true（表示写入成功）
    let result = v8::Boolean::new(scope, true);
    retval.set(result.into());
}

/// v0.3.239: stderr.write() 回调
fn stderr_write_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let data = args.get(0);

    // 将 V8 值转换为字符串并输出到 stderr
    let output = if let Some(str_val) = data.to_string(scope) {
        str_val.to_rust_string_lossy(scope)
    } else if data.is_null_or_undefined() {
        String::new()
    } else {
        String::from("[object]")
    };

    // 输出到 stderr 并刷新
    let mut stderr = std::io::stderr();
    let _ = write!(stderr, "{}", output);
    let _ = stderr.flush();

    // 返回 true（表示写入成功）
    let result = v8::Boolean::new(scope, true);
    retval.set(result.into());
}

/// v0.3.240: stdin.read() 回调
/// 同步读取不支持，返回 null（需要异步运行时支持）
fn stdin_read_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // 同步模式下无法读取 stdin，返回 null
    let null_val = v8::null(scope);
    retval.set(null_val.into());
}

/// v0.3.240: process.hrtime() 回调 - 高精度时间
fn process_hrtime_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // 获取当前时间（纳秒精度）
    let now = std::time::UNIX_EPOCH.elapsed().unwrap();
    let seconds = now.as_secs() as f64;
    let nanos = now.subsec_nanos() as f64;

    // 如果传入了 previous timestamp，计算差值
    if args.length() > 0 {
        let prev = args.get(0);
        if prev.is_array() {
            let prev_array = v8::Local::<v8::Array>::try_from(prev).unwrap();
            if prev_array.length() >= 2 {
                let prev_sec = prev_array
                    .get_index(scope, 0)
                    .unwrap()
                    .to_number(scope)
                    .unwrap()
                    .value();
                let prev_nano = prev_array
                    .get_index(scope, 1)
                    .unwrap()
                    .to_number(scope)
                    .unwrap()
                    .value();

                let diff_sec = seconds - prev_sec;
                let diff_nano = nanos - prev_nano;

                let diff_sec_num = v8::Number::new(scope, diff_sec);
                let diff_nano_num = v8::Number::new(scope, diff_nano);
                let result = v8::Array::new(scope, 2);
                result.set_index(scope, 0, diff_sec_num.into());
                result.set_index(scope, 1, diff_nano_num.into());
                retval.set(result.into());
                return;
            }
        }
    }

    // 返回 [seconds, nanoseconds] 数组
    let seconds_num = v8::Number::new(scope, seconds);
    let nanos_num = v8::Number::new(scope, nanos);
    let result = v8::Array::new(scope, 2);
    result.set_index(scope, 0, seconds_num.into());
    result.set_index(scope, 1, nanos_num.into());
    retval.set(result.into());
}

/// v0.3.241: process.memory() 回调 - 真实的 V8 堆内存统计
/// 使用 V8 HeapStatistics API 获取真实的堆内存使用情况
fn process_memory_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // 使用 V8 API 获取真实的堆统计信息
    let heap_stats = scope.get_heap_statistics();
    {
        let result = v8::Object::new(scope);

        // heapUsed - 已使用的堆内存（字节）
        let heap_used_key = v8::String::new(scope, "heapUsed").unwrap();
        let heap_used_val = v8::Number::new(scope, heap_stats.used_heap_size() as f64);
        result.set(scope, heap_used_key.into(), heap_used_val.into());

        // heapTotal - 总堆内存（字节）
        let heap_total_key = v8::String::new(scope, "heapTotal").unwrap();
        let heap_total_val = v8::Number::new(scope, heap_stats.total_heap_size() as f64);
        result.set(scope, heap_total_key.into(), heap_total_val.into());

        // external - 外部内存（字节）
        let external_key = v8::String::new(scope, "external").unwrap();
        let external_val = v8::Number::new(scope, heap_stats.external_memory() as f64);
        result.set(scope, external_key.into(), external_val.into());

        let rss_key = v8::String::new(scope, "rss").unwrap();
        let rss_val = v8::Number::new(scope, heap_stats.total_physical_size() as f64);
        result.set(scope, rss_key.into(), rss_val.into());

        let array_buffers_key = v8::String::new(scope, "arrayBuffers").unwrap();
        let array_buffers_obj = v8::Object::new(scope);
        let used_key = v8::String::new(scope, "used").unwrap();
        let used_val = v8::Number::new(scope, 0.0);
        array_buffers_obj.set(scope, used_key.into(), used_val.into());
        result.set(scope, array_buffers_key.into(), array_buffers_obj.into());

        retval.set(result.into());
    }
}

/// v0.3.240: process.uptime() 回调 - 进程运行时间
fn process_uptime_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let uptime = START_TIME.with(|start| start.elapsed().as_secs_f64());
    retval.set(v8::Number::new(scope, uptime).into());
}

// v0.3.241: process.cpuUsage() 回调 - 真实的 CPU 使用统计
// 使用平台特定的 API 获取用户和系统 CPU 时间

// 线程本地存储初始 CPU 时间
thread_local! {
    static START_CPU_TIME: std::time::Duration = std::time::Duration::ZERO;
}

fn process_cpu_usage_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let mut prev_user = 0.0;
    let mut prev_system = 0.0;

    // 如果传入了 previous value，计算差值
    if args.length() > 0 {
        let prev = args.get(0);
        if prev.is_object() {
            let prev_obj = v8::Local::<v8::Object>::try_from(prev).unwrap();
            let user_key = v8::String::new(scope, "user").unwrap();
            let system_key = v8::String::new(scope, "system").unwrap();

            if let Some(user_val) = prev_obj.get(scope, user_key.into()) {
                if user_val.is_number() {
                    prev_user = user_val.to_number(scope).unwrap().value();
                }
            }
            if let Some(system_val) = prev_obj.get(scope, system_key.into()) {
                if system_val.is_number() {
                    prev_system = system_val.to_number(scope).unwrap().value();
                }
            }
        }
    }

    // 获取当前 CPU 时间（微秒）
    let (current_user, current_system) = get_cpu_times();

    let result = v8::Object::new(scope);

    // 如果传入了 previous value，返回差值
    let user_value = if prev_user > 0.0 {
        (current_user as f64) - prev_user
    } else {
        current_user as f64
    };

    let system_value = if prev_system > 0.0 {
        (current_system as f64) - prev_system
    } else {
        current_system as f64
    };

    let user_key = v8::String::new(scope, "user").unwrap();
    let user_val = v8::Number::new(scope, user_value);
    result.set(scope, user_key.into(), user_val.into());

    let system_key = v8::String::new(scope, "system").unwrap();
    let system_val = v8::Number::new(scope, system_value);
    result.set(scope, system_key.into(), system_val.into());

    retval.set(result.into());
}

/// 获取当前进程的 CPU 使用时间（微秒）
/// 返回 (user_time, system_time)
#[cfg(target_os = "linux")]
fn get_cpu_times() -> (u64, u64) {
    use std::fs;

    // 读取 /proc/self/stat 获取进程统计信息
    if let Ok(stat) = fs::read_to_string("/proc/self/stat") {
        let parts: Vec<&str> = stat.split_whitespace().collect();
        if parts.len() >= 16 {
            // utime (index 14) - 用户态时间（时钟滴答数）
            // stime (index 15) - 内核态时间（时钟滴答数）
            if let (Ok(utime), Ok(stime)) = (parts[14].parse::<u64>(), parts[15].parse::<u64>()) {
                // 将时钟滴答数转换为微秒
                let clk_tck = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
                let usec_per_tick = 1_000_000 / clk_tck as u64;
                return (utime * usec_per_tick, stime * usec_per_tick);
            }
        }
    }

    // 回退到基于时间的估算
    fallback_cpu_time()
}

#[cfg(target_os = "macos")]
fn get_cpu_times() -> (u64, u64) {
    use libc::getrusage;
    use libc::rusage;
    use std::mem::zeroed;

    const RUSAGE_SELF: i32 = 0;

    let mut usage: rusage = unsafe { zeroed() };
    unsafe {
        if getrusage(RUSAGE_SELF, &mut usage) == 0 {
            // tv_sec 和 tv_usec 转换为微秒
            let user_usec =
                (usage.ru_utime.tv_sec as u64) * 1_000_000 + usage.ru_utime.tv_usec as u64;
            let system_usec =
                (usage.ru_stime.tv_sec as u64) * 1_000_000 + usage.ru_stime.tv_usec as u64;
            return (user_usec, system_usec);
        }
    }

    fallback_cpu_time()
}

#[cfg(target_os = "freebsd")]
fn get_cpu_times() -> (u64, u64) {
    use std::fs;

    // FreeBSD 使用 procfs
    if let Ok(stat) = fs::read_to_string("/proc/curproc/status") {
        for line in stat.lines() {
            if line.starts_with("utime:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let (Ok(user), Ok(sys)) = (
                        parts[1].parse::<u64>(),
                        parts.get(2).and_then(|s| s.parse::<u64>().ok()),
                    ) {
                        return (user * 1000, sys * 1000); // 转换为微秒
                    }
                }
            }
        }
    }

    fallback_cpu_time()
}

/// 回退到基于时间的 CPU 使用估算
fn fallback_cpu_time() -> (u64, u64) {
    // 使用当前时间作为近似值
    let now = std::time::Instant::now();
    let elapsed = now.elapsed();
    let micros = elapsed.as_secs() * 1_000_000 + elapsed.subsec_micros() as u64;
    (micros, 0)
}

#[cfg(target_family = "windows")]
fn get_cpu_times() -> (u64, u64) {
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};

    fn filetime_to_micros(time: FILETIME) -> u64 {
        let ticks = ((time.dwHighDateTime as u64) << 32) | (time.dwLowDateTime as u64);
        ticks / 10
    }

    let mut creation_time = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut exit_time = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut kernel_time = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut user_time = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };

    unsafe {
        if GetProcessTimes(
            GetCurrentProcess(),
            &mut creation_time,
            &mut exit_time,
            &mut kernel_time,
            &mut user_time,
        ) != 0
        {
            return (
                filetime_to_micros(user_time),
                filetime_to_micros(kernel_time),
            );
        }
    }

    (0, 0)
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_family = "windows"
)))]
fn get_cpu_times() -> (u64, u64) {
    // 其他平台回退到零
    (0, 0)
}

/// process.exit() 回调
fn process_exit_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    _retval: v8::ReturnValue,
) {
    let code = args.get(0).int32_value(scope).unwrap_or(0);

    // 设置退出状态
    SHOULD_EXIT.with(|exit| {
        let mut should_exit = exit.lock().unwrap();
        *should_exit = true;
    });
    EXIT_CODE.with(|exit_code| {
        let mut code_ref = exit_code.lock().unwrap();
        *code_ref = code;
    });

    // 注意: 实际退出需要在 Rust 层面处理
    // 这里只是标记退出状态
    eprintln!("[bee] Process exiting with code: {}", code);
}

/// process.on() 回调
fn process_on_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let event_name = args.get(0);
    let handler = args.get(1);

    if !handler.is_function() {
        let error = v8::String::new(scope, "Event handler must be a function").unwrap();
        retval.set(error.into());
        return;
    }

    let event_name_str = event_name
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);

    match event_name_str.as_str() {
        "uncaughtException" => {
            // 添加到未捕获异常处理器列表
            let handler_global = v8::Global::new(scope, handler);
            UNCAUGHT_EXCEPTION_HANDLERS.with(|handlers| {
                let mut h = handlers.lock().unwrap();
                h.push(handler_global);
            });
            retval.set(scope.get_current_context().global(scope).into());
        }
        "unhandledRejection" => {
            // 添加到未处理的 Promise rejection 处理器列表
            let handler_global = v8::Global::new(scope, handler);
            UNHANDLED_REJECTION_HANDLERS.with(|handlers| {
                let mut h = handlers.lock().unwrap();
                h.push(handler_global);
            });
            retval.set(scope.get_current_context().global(scope).into());
        }
        _ => {
            retval.set(scope.get_current_context().global(scope).into());
        }
    }
}

/// process.off() 回调
fn process_off_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let event_name = args.get(0);
    let handler = args.get(1);

    if !handler.is_function() {
        retval.set(scope.get_current_context().global(scope).into());
        return;
    }

    let event_name_str = event_name
        .to_string(scope)
        .unwrap()
        .to_rust_string_lossy(scope);

    match event_name_str.as_str() {
        "uncaughtException" => {
            // 移除未捕获异常处理器
            // 注意: 这是一个简化实现，实际需要更复杂的比较逻辑
            retval.set(scope.get_current_context().global(scope).into());
        }
        "unhandledRejection" => {
            // 移除未处理的 Promise rejection 处理器
            retval.set(scope.get_current_context().global(scope).into());
        }
        _ => {
            retval.set(scope.get_current_context().global(scope).into());
        }
    }
}

/// process.removeListener() 回调
fn process_remove_listener_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let _event_name = args.get(0);
    let _handler = args.get(1);

    retval.set(scope.get_current_context().global(scope).into());
}

/// v0.3.242: process.setMaxListeners() 回调
/// 设置指定事件的最大监听器数量
/// n 为 0 表示无限制
/// v0.3.243: Fix - process.setMaxListeners(n) with single arg sets global default
fn process_set_max_listeners_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Determine event name and n value
    // If first arg is a number, treat it as n (set global default)
    // If first arg is a string, treat it as event name, second arg is n
    let (event_name, n) = if args.length() > 0 {
        let first = args.get(0);
        if first.is_number() {
            // Single argument: setMaxListeners(n) - sets "__default__"
            let n = first.int32_value(scope).unwrap_or(0);
            (String::from("__default__"), n)
        } else if first.is_string() || first.is_null_or_undefined() {
            // First arg is event name
            let name = first.to_string(scope).unwrap().to_rust_string_lossy(scope);
            let n = if args.length() > 1 {
                args.get(1).int32_value(scope).unwrap_or(0)
            } else {
                0
            };
            (name, n)
        } else {
            (String::from("__default__"), 0)
        }
    } else {
        (String::from("__default__"), 0)
    };

    // Validate n (must be >= 0)
    let max_listeners = if n < 0 {
        0 // 负数视为 0（无限制）
    } else {
        n
    };

    // 存储到线程本地
    MAX_LISTENERS.with(|map| {
        let mut map = map.lock().unwrap();
        map.insert(event_name, max_listeners);
    });

    // 返回 process 对象（支持链式调用）
    retval.set(args.this().into());
}

/// v0.3.242: process.getMaxListeners() 回调
/// 获取指定事件的最大监听器数量
fn process_get_max_listeners_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // 获取事件名称
    let event_name = if args.length() > 0 {
        let name = args.get(0);
        if name.is_string() || name.is_null_or_undefined() {
            name.to_string(scope).unwrap().to_rust_string_lossy(scope)
        } else {
            String::from("__default__")
        }
    } else {
        String::from("__default__")
    };

    // 从线程本地获取
    let max_listeners = MAX_LISTENERS.with(|map| {
        let map = map.lock().unwrap();
        map.get(&event_name).copied().unwrap_or(10) // 默认值是 10
    });

    retval.set(v8::Integer::new(scope, max_listeners).into());
}

/// v0.3.243: process.kill(pid, signal) 回调
/// 向指定进程发送信号
fn process_kill_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // 获取 PID
    let pid = args.get(0).int32_value(scope).unwrap_or(0);

    // 获取信号（可以是字符串或数字）
    let signal = if args.length() > 1 {
        let sig_arg = args.get(1);
        if sig_arg.is_number() {
            sig_arg.int32_value(scope).unwrap_or(15) as u32
        } else if let Some(str_val) = sig_arg.to_string(scope) {
            let sig_str = str_val.to_rust_string_lossy(scope);
            signal_name_to_number(&sig_str)
        } else {
            15 // 默认 SIGTERM
        }
    } else {
        15 // 默认信号
    };

    // 发送信号
    let result = if pid > 0 {
        send_signal_to_process(pid as u32, signal)
    } else {
        false
    };

    retval.set(v8::Boolean::new(scope, result).into());
}

/// 将信号名称转换为信号编号
fn signal_name_to_number(signal_name: &str) -> u32 {
    match signal_name.to_uppercase().as_str() {
        "SIGHUP" | "HUP" => 1,
        "SIGINT" | "INT" => 2,
        "SIGQUIT" | "QUIT" => 3,
        "SIGILL" | "ILL" => 4,
        "SIGTRAP" | "TRAP" => 5,
        "SIGABRT" | "ABRT" => 6,
        "SIGFPE" | "FPE" => 8,
        "SIGKILL" | "KILL" => 9,
        "SIGUSR1" | "USR1" => 10,
        "SIGSEGV" | "SEGV" => 11,
        "SIGUSR2" | "USR2" => 12,
        "SIGPIPE" | "PIPE" => 13,
        "SIGALRM" | "ALRM" => 14,
        "SIGTERM" | "TERM" => 15,
        "SIGCHLD" | "CHLD" => 17,
        "SIGCONT" | "CONT" => 18,
        "SIGSTOP" | "STOP" => 19,
        "SIGTSTP" | "TSTP" => 20,
        "SIGTTIN" | "TTIN" => 21,
        "SIGTTOU" | "TTOU" => 22,
        "SIGBUS" | "BUS" => 10, // FreeBSD uses 10, Linux uses 7
        _ => 15,                // 默认 SIGTERM
    }
}

/// 向进程发送信号
fn send_signal_to_process(pid: u32, signal: u32) -> bool {
    #[cfg(target_family = "unix")]
    {
        use libc::kill;

        unsafe { kill(pid as libc::pid_t, signal as libc::c_int) == 0 }
    }
    #[cfg(target_family = "windows")]
    {
        // Windows 不支持 Unix 信号，这里简化处理
        // 对于当前进程，标记退出
        if pid == std::process::id() {
            if signal == 15 || signal == 2 || signal == 1 {
                // SIGTERM, SIGINT, SIGHUP
                // 标记退出（实际退出由运行时处理）
                SHOULD_EXIT.with(|exit| {
                    *exit.lock().unwrap() = true;
                });
                EXIT_CODE.with(|code| {
                    *code.lock().unwrap() = 128 + signal as i32;
                });
                return true;
            }
        }
        false
    }
    #[cfg(not(any(target_family = "unix", target_family = "windows")))]
    {
        false
    }
}

/// 触发未捕获异常事件
pub fn emit_uncaught_exception(scope: &mut v8::PinScope, error: &v8::Local<v8::Value>) {
    UNCAUGHT_EXCEPTION_HANDLERS.with(|handlers| {
        let handlers = handlers.lock().unwrap();
        for handler in handlers.iter() {
            let handler_val = v8::Local::new(scope, handler);
            // 转换为 Function 类型
            if handler_val.is_function() {
                if let Ok(handler_func) = v8::Local::<v8::Function>::try_from(handler_val) {
                    let this = scope.get_current_context().global(scope);
                    let result = handler_func.call(scope, this.into(), &[*error]);
                    if result.is_none() {
                        // 处理器执行失败，忽略
                    }
                }
            }
        }
    });
}

/// 触发未处理的 Promise rejection 事件
pub fn emit_unhandled_rejection(
    scope: &mut v8::PinScope,
    reason: &v8::Local<v8::Value>,
    promise: &v8::Local<v8::Value>,
) {
    UNHANDLED_REJECTION_HANDLERS.with(|handlers| {
        let handlers = handlers.lock().unwrap();
        for handler in handlers.iter() {
            let handler_val = v8::Local::new(scope, handler);
            // 转换为 Function 类型
            if handler_val.is_function() {
                if let Ok(handler_func) = v8::Local::<v8::Function>::try_from(handler_val) {
                    let this = scope.get_current_context().global(scope);
                    let result = handler_func.call(scope, this.into(), &[*reason, *promise]);
                    if result.is_none() {
                        // 处理器执行失败，忽略
                    }
                }
            }
        }
    });
}

/// 检查是否应该退出
pub fn should_exit() -> bool {
    SHOULD_EXIT.with(|exit| *exit.lock().unwrap())
}

/// 获取退出码
pub fn get_exit_code() -> i32 {
    EXIT_CODE.with(|code| *code.lock().unwrap())
}

/// 重置状态（用于测试）
#[cfg(test)]
pub fn reset_process_state() {
    UNCAUGHT_EXCEPTION_HANDLERS.with(|handlers| {
        let mut h = handlers.lock().unwrap();
        h.clear();
    });
    UNHANDLED_REJECTION_HANDLERS.with(|handlers| {
        let mut h = handlers.lock().unwrap();
        h.clear();
    });
    SHOULD_EXIT.with(|exit| {
        *exit.lock().unwrap() = false;
    });
    EXIT_CODE.with(|code| {
        *code.lock().unwrap() = 0;
    });
    // v0.3.242: 重置 setMaxListeners 状态
    MAX_LISTENERS.with(|map| {
        let mut m = map.lock().unwrap();
        m.clear();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reset_process_state() {
        reset_process_state();
        assert!(!should_exit());
        assert_eq!(get_exit_code(), 0);
    }
}
