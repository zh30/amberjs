// Node.js child_process模块实现
/// 子进程管理
///
/// Stable surfaces:
/// - G25 sync: `execSync` / `spawnSync` (see CHILD_PROCESS_CONTRACT.md)
/// - G47 async: narrow `exec` / `execFile` with host-thread run + later-turn callback
///   (see CHILD_PROCESS_ASYNC_CONTRACT.md). Preview `spawn` stays sync-blocking.
use anyhow::Result;
use once_cell::sync::Lazy;
use rusty_v8 as v8;
use std::collections::{HashMap, VecDeque};
use std::process::Command;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;

/// Cross-thread completed async jobs (job id + captured output).
static ASYNC_COMPLETED: Lazy<Mutex<VecDeque<(u64, ChildProcessOutput)>>> =
    Lazy::new(|| Mutex::new(VecDeque::new()));
/// In-flight host threads started by `exec` / `execFile`.
static ASYNC_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
static ASYNC_JOB_SEQ: AtomicU64 = AtomicU64::new(1);

thread_local! {
    /// Callbacks for async jobs; V8 Globals stay on the isolate thread.
    static ASYNC_CALLBACKS: std::cell::RefCell<HashMap<u64, v8::Global<v8::Function>>> =
        std::cell::RefCell::new(HashMap::new());
}

/// True while an async child is running or its result awaits delivery.
pub fn has_pending_child_process_work() -> bool {
    if ASYNC_IN_FLIGHT.load(Ordering::SeqCst) > 0 {
        return true;
    }
    ASYNC_COMPLETED
        .lock()
        .map(|q| !q.is_empty())
        .unwrap_or(false)
}

/// Drain completed async jobs into Node-shaped callbacks (V8 thread only).
pub fn pump_pending_child_process_jobs(scope: &mut v8::PinScope) -> usize {
    let mut batch = Vec::new();
    if let Ok(mut q) = ASYNC_COMPLETED.lock() {
        while let Some(item) = q.pop_front() {
            batch.push(item);
        }
    }
    let n = batch.len();
    for (job_id, output) in batch {
        let callback = ASYNC_CALLBACKS.with(|map| map.borrow_mut().remove(&job_id));
        if let Some(callback_global) = callback {
            let callback_local = v8::Local::new(scope, callback_global);
            call_child_process_callback(scope, callback_local.into(), &output);
        }
    }
    n
}

fn enqueue_async_job(
    scope: &mut v8::PinScope,
    callback: v8::Local<v8::Function>,
    work: AsyncCpWork,
) {
    let job_id = ASYNC_JOB_SEQ.fetch_add(1, Ordering::Relaxed);
    ASYNC_CALLBACKS.with(|map| {
        map.borrow_mut()
            .insert(job_id, v8::Global::new(scope, callback));
    });
    ASYNC_IN_FLIGHT.fetch_add(1, Ordering::SeqCst);
    thread::spawn(move || {
        let output = match work {
            AsyncCpWork::Shell { command } => {
                child_process_output_from_result(run_shell_command(&command))
            }
            AsyncCpWork::ExecFile { file, args } => {
                child_process_output_from_result(Command::new(&file).args(args).output())
            }
        };
        if let Ok(mut q) = ASYNC_COMPLETED.lock() {
            q.push_back((job_id, output));
        }
        ASYNC_IN_FLIGHT.fetch_sub(1, Ordering::SeqCst);
    });
}

enum AsyncCpWork {
    Shell { command: String },
    ExecFile { file: String, args: Vec<String> },
}

fn pending_child_process_object<'s>(scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Object> {
    let child_obj = v8::Object::new(scope);

    let stdout_key = v8::String::new(scope, "stdout").unwrap();
    let stdout_val = v8::undefined(scope);
    child_obj.set(scope, stdout_key.into(), stdout_val.into());

    let stderr_key = v8::String::new(scope, "stderr").unwrap();
    let stderr_val = v8::undefined(scope);
    child_obj.set(scope, stderr_key.into(), stderr_val.into());

    let pid_key = v8::String::new(scope, "pid").unwrap();
    let pid_val = v8::Integer::new(scope, 0);
    child_obj.set(scope, pid_key.into(), pid_val.into());

    let killed_key = v8::String::new(scope, "killed").unwrap();
    let killed_val = v8::Boolean::new(scope, false);
    child_obj.set(scope, killed_key.into(), killed_val.into());

    // Not exited yet — Stable async contract does not pin exit events.
    let exit_code_key = v8::String::new(scope, "exitCode").unwrap();
    let exit_code_val = v8::null(scope);
    child_obj.set(scope, exit_code_key.into(), exit_code_val.into());

    let signal_key = v8::String::new(scope, "signal").unwrap();
    let signal_val = v8::null(scope);
    child_obj.set(scope, signal_key.into(), signal_val.into());

    let on_template = v8::FunctionTemplate::new(scope, child_process_on_pending_callback);
    let on_func = on_template.get_function(scope).unwrap();
    let on_key = v8::String::new(scope, "on").unwrap();
    child_obj.set(scope, on_key.into(), on_func.into());

    child_obj
}

/// Preview/Stable return value: `on` is chainable and does not fire sync.
pub fn child_process_on_pending_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    retval.set(args.this().into());
}

fn async_noop_callback(
    _scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    _retval: v8::ReturnValue,
) {
}

fn make_async_noop_function<'s>(scope: &mut v8::PinScope<'s, '_>) -> v8::Local<'s, v8::Function> {
    v8::Function::new(scope, async_noop_callback).unwrap()
}

pub fn string_from_v8_value(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> String {
    value
        .to_string(scope)
        .map(|value| value.to_rust_string_lossy(scope))
        .unwrap_or_default()
}

pub fn string_vec_from_v8_array_value(
    scope: &mut v8::PinScope,
    value: v8::Local<v8::Value>,
) -> Vec<String> {
    if !value.is_array() {
        return Vec::new();
    }

    let Ok(array) = v8::Local::<v8::Array>::try_from(value) else {
        return Vec::new();
    };

    let mut values = Vec::new();
    for index in 0..array.length() {
        if let Some(value) = array.get_index(scope, index) {
            values.push(string_from_v8_value(scope, value));
        }
    }
    values
}

pub fn run_shell_command(command: &str) -> std::io::Result<std::process::Output> {
    #[cfg(windows)]
    {
        Command::new("cmd").args(["/C", command]).output()
    }

    #[cfg(not(windows))]
    {
        Command::new("sh").arg("-c").arg(command).output()
    }
}

pub struct ChildProcessOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub fn child_process_output_from_result(
    output: std::io::Result<std::process::Output>,
) -> ChildProcessOutput {
    match output {
        Ok(output) => ChildProcessOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(1),
        },
        Err(error) => ChildProcessOutput {
            stdout: String::new(),
            stderr: error.to_string(),
            exit_code: 1,
        },
    }
}

pub fn child_process_output_object<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    output: &ChildProcessOutput,
) -> v8::Local<'s, v8::Object> {
    let child_obj = v8::Object::new(scope);

    let stdout_key = v8::String::new(scope, "stdout").unwrap();
    let stdout_val = v8::String::new(scope, &output.stdout).unwrap();
    child_obj.set(scope, stdout_key.into(), stdout_val.into());

    let stderr_key = v8::String::new(scope, "stderr").unwrap();
    let stderr_val = v8::String::new(scope, &output.stderr).unwrap();
    child_obj.set(scope, stderr_key.into(), stderr_val.into());

    let pid_key = v8::String::new(scope, "pid").unwrap();
    let pid_val = v8::Integer::new(scope, 0);
    child_obj.set(scope, pid_key.into(), pid_val.into());

    let killed_key = v8::String::new(scope, "killed").unwrap();
    let killed_val = v8::Boolean::new(scope, false);
    child_obj.set(scope, killed_key.into(), killed_val.into());

    let exit_code_key = v8::String::new(scope, "exitCode").unwrap();
    let exit_code_val = v8::Integer::new(scope, output.exit_code);
    child_obj.set(scope, exit_code_key.into(), exit_code_val.into());

    let signal_key = v8::String::new(scope, "signal").unwrap();
    let signal_val = v8::null(scope);
    child_obj.set(scope, signal_key.into(), signal_val.into());

    let on_template = v8::FunctionTemplate::new(scope, child_process_on_callback);
    let on_func = on_template.get_function(scope).unwrap();
    let on_key = v8::String::new(scope, "on").unwrap();
    child_obj.set(scope, on_key.into(), on_func.into());

    child_obj
}

pub fn child_process_error_value<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    exit_code: i32,
) -> v8::Local<'s, v8::Value> {
    if exit_code == 0 {
        return v8::null(scope).into();
    }

    let message = format!("Command failed with exit code {}", exit_code);
    let message_val = v8::String::new(scope, &message).unwrap();
    let error_val = v8::Exception::error(scope, message_val);
    if let Ok(error_obj) = v8::Local::<v8::Object>::try_from(error_val) {
        let code_key = v8::String::new(scope, "code").unwrap();
        let code_val = v8::Integer::new(scope, exit_code);
        error_obj.set(scope, code_key.into(), code_val.into());
    }
    error_val
}

pub fn call_child_process_callback(
    scope: &mut v8::PinScope,
    callback_value: v8::Local<v8::Value>,
    output: &ChildProcessOutput,
) {
    if !callback_value.is_function() {
        return;
    }

    let Ok(callback) = v8::Local::<v8::Function>::try_from(callback_value) else {
        return;
    };

    let error = child_process_error_value(scope, output.exit_code);
    let stdout = v8::String::new(scope, &output.stdout).unwrap();
    let stderr = v8::String::new(scope, &output.stderr).unwrap();
    let undefined = v8::undefined(scope);
    let callback_args: [v8::Local<v8::Value>; 3] = [error, stdout.into(), stderr.into()];
    let _ = callback.call(scope, undefined.into(), &callback_args);
}

pub fn child_process_on_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this = args.this();
    let event = string_from_v8_value(scope, args.get(0));
    let listener = args.get(1);
    if !listener.is_function() {
        retval.set(this.into());
        return;
    }

    if event == "exit" || event == "close" {
        if let Ok(listener_func) = v8::Local::<v8::Function>::try_from(listener) {
            let exit_code_key = v8::String::new(scope, "exitCode").unwrap();
            let exit_code = this
                .get(scope, exit_code_key.into())
                .unwrap_or_else(|| v8::null(scope).into());
            let signal_key = v8::String::new(scope, "signal").unwrap();
            let signal = this
                .get(scope, signal_key.into())
                .unwrap_or_else(|| v8::null(scope).into());
            let call_args: [v8::Local<v8::Value>; 2] = [exit_code, signal];
            let _ = listener_func.call(scope, this.into(), &call_args);
        }
    }

    retval.set(this.into());
}

pub fn child_process_bytes_to_v8_value<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    bytes: &[u8],
    encoding: Option<&str>,
) -> v8::Local<'s, v8::Value> {
    if let Some(enc) = encoding {
        if enc != "buffer" {
            let s = String::from_utf8_lossy(bytes);
            return v8::String::new(scope, &s).unwrap().into();
        }
    }
    let ab = v8::ArrayBuffer::new(scope, bytes.len());
    if !bytes.is_empty() {
        let bs = ab.get_backing_store();
        unsafe {
            let ptr = bs.as_ref().as_ptr() as *mut u8;
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        }
    }
    let ctx = scope.get_current_context();
    let global = ctx.global(scope);
    let buffer_key = v8::String::new(scope, "Buffer").unwrap();
    if let Some(buf_ctor) = global.get(scope, buffer_key.into()) {
        if let Ok(buf_fn) = v8::Local::<v8::Function>::try_from(buf_ctor) {
            let from_key = v8::String::new(scope, "from").unwrap();
            if let Some(from_val) = buf_fn.get(scope, from_key.into()) {
                if let Ok(from_fn) = v8::Local::<v8::Function>::try_from(from_val) {
                    let fargs = [ab.into()];
                    if let Some(wrapped) = from_fn.call(scope, buf_ctor, &fargs) {
                        return wrapped;
                    }
                }
            }
        }
    }
    ab.into()
}

pub fn cp_exec_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let command = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    if let Err(error) = crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::Process,
        crate::permissions::PermissionAction::Execute,
        crate::permissions::ResourceId::Name(crate::permissions::process_command_name(&command)),
    ) {
        let error_message = v8::String::new(scope, &error.to_string()).unwrap();
        let error_obj = v8::Exception::error(scope, error_message);
        scope.throw_exception(error_obj.into());
        return;
    }
    let callback_val = if args.get(1).is_function() {
        args.get(1)
    } else {
        args.get(2)
    };
    let callback = v8::Local::<v8::Function>::try_from(callback_val)
        .unwrap_or_else(|_| make_async_noop_function(scope));
    enqueue_async_job(scope, callback, AsyncCpWork::Shell { command });
    retval.set(pending_child_process_object(scope).into());
}

pub fn cp_spawn_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let command = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    if let Err(error) = crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::Process,
        crate::permissions::PermissionAction::Execute,
        crate::permissions::ResourceId::Name(crate::permissions::process_command_name(&command)),
    ) {
        let error_message = v8::String::new(scope, &error.to_string()).unwrap();
        let error_obj = v8::Exception::error(scope, error_message);
        scope.throw_exception(error_obj.into());
        return;
    }
    let spawn_args = string_vec_from_v8_array_value(scope, args.get(1));
    let output = Command::new(&command).args(spawn_args).output();
    let output = child_process_output_from_result(output);
    let child_obj = child_process_output_object(scope, &output);
    retval.set(child_obj.into());
}

pub fn cp_exec_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let file = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    if let Err(error) = crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::Process,
        crate::permissions::PermissionAction::Execute,
        crate::permissions::ResourceId::Name(crate::permissions::process_command_name(&file)),
    ) {
        let error_message = v8::String::new(scope, &error.to_string()).unwrap();
        let error_obj = v8::Exception::error(scope, error_message);
        scope.throw_exception(error_obj.into());
        return;
    }
    let args_or_callback = args.get(1);
    let callback_val = if args_or_callback.is_function() {
        args_or_callback
    } else if args.get(2).is_function() {
        args.get(2)
    } else {
        args.get(3)
    };
    let exec_args = if args_or_callback.is_array() {
        string_vec_from_v8_array_value(scope, args_or_callback)
    } else {
        Vec::new()
    };
    let callback = v8::Local::<v8::Function>::try_from(callback_val)
        .unwrap_or_else(|_| make_async_noop_function(scope));
    enqueue_async_job(
        scope,
        callback,
        AsyncCpWork::ExecFile {
            file,
            args: exec_args,
        },
    );
    retval.set(pending_child_process_object(scope).into());
}

pub fn cp_exec_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let command = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    if let Err(error) = crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::Process,
        crate::permissions::PermissionAction::Execute,
        crate::permissions::ResourceId::Name(crate::permissions::process_command_name(&command)),
    ) {
        let error_message = v8::String::new(scope, &error.to_string()).unwrap();
        let error_obj = v8::Exception::error(scope, error_message);
        scope.throw_exception(error_obj.into());
        return;
    }
    let mut encoding: Option<String> = None;
    if args.length() > 1 && args.get(1).is_object() {
        if let Ok(opts_obj) = v8::Local::<v8::Object>::try_from(args.get(1)) {
            let enc_key = v8::String::new(scope, "encoding").unwrap();
            if let Some(enc_val) = opts_obj.get(scope, enc_key.into()) {
                if let Some(s) = enc_val.to_string(scope) {
                    encoding = Some(s.to_rust_string_lossy(scope));
                }
            }
        }
    }
    match run_shell_command(&command) {
        Ok(output) => {
            if !output.status.success() {
                let exit_code = output.status.code().unwrap_or(1);
                let stderr_str = String::from_utf8_lossy(&output.stderr);
                let msg = format!("Command failed: {}\n{}", command, stderr_str);
                let msg_val = v8::String::new(scope, &msg).unwrap();
                let err_val = v8::Exception::error(scope, msg_val);
                if let Ok(err_obj) = v8::Local::<v8::Object>::try_from(err_val) {
                    let status_key = v8::String::new(scope, "status").unwrap();
                    let status_val = v8::Integer::new(scope, exit_code);
                    err_obj.set(scope, status_key.into(), status_val.into());
                    let stdout_key = v8::String::new(scope, "stdout").unwrap();
                    let stdout_val =
                        child_process_bytes_to_v8_value(scope, &output.stdout, encoding.as_deref());
                    err_obj.set(scope, stdout_key.into(), stdout_val);
                    let stderr_key = v8::String::new(scope, "stderr").unwrap();
                    let stderr_val =
                        child_process_bytes_to_v8_value(scope, &output.stderr, encoding.as_deref());
                    err_obj.set(scope, stderr_key.into(), stderr_val);
                }
                scope.throw_exception(err_val);
                return;
            }
            let val = child_process_bytes_to_v8_value(scope, &output.stdout, encoding.as_deref());
            retval.set(val);
        }
        Err(e) => {
            let msg = format!("Command failed: {}: {}", command, e);
            let msg_val = v8::String::new(scope, &msg).unwrap();
            let err_val = v8::Exception::error(scope, msg_val);
            scope.throw_exception(err_val);
        }
    }
}

pub fn cp_spawn_sync_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let command = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    if let Err(error) = crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::Process,
        crate::permissions::PermissionAction::Execute,
        crate::permissions::ResourceId::Name(crate::permissions::process_command_name(&command)),
    ) {
        let error_message = v8::String::new(scope, &error.to_string()).unwrap();
        let error_obj = v8::Exception::error(scope, error_message);
        scope.throw_exception(error_obj.into());
        return;
    }
    let mut cmd_args: Vec<String> = Vec::new();
    let mut encoding: Option<String> = None;
    let arg1 = args.get(1);
    let arg2 = args.get(2);
    if arg1.is_array() {
        cmd_args = string_vec_from_v8_array_value(scope, arg1);
        if arg2.is_object() {
            if let Ok(opts) = v8::Local::<v8::Object>::try_from(arg2) {
                let enc_k = v8::String::new(scope, "encoding").unwrap();
                if let Some(enc_v) = opts.get(scope, enc_k.into()) {
                    if let Some(s) = enc_v.to_string(scope) {
                        encoding = Some(s.to_rust_string_lossy(scope));
                    }
                }
            }
        }
    } else if arg1.is_object() {
        if let Ok(opts) = v8::Local::<v8::Object>::try_from(arg1) {
            let enc_k = v8::String::new(scope, "encoding").unwrap();
            if let Some(enc_v) = opts.get(scope, enc_k.into()) {
                if let Some(s) = enc_v.to_string(scope) {
                    encoding = Some(s.to_rust_string_lossy(scope));
                }
            }
        }
    }
    let res_obj = v8::Object::new(scope);
    match Command::new(&command).args(&cmd_args).output() {
        Ok(output) => {
            let status_key = v8::String::new(scope, "status").unwrap();
            let status_val = v8::Integer::new(scope, output.status.code().unwrap_or(0));
            res_obj.set(scope, status_key.into(), status_val.into());

            let signal_key = v8::String::new(scope, "signal").unwrap();
            let signal_val = v8::null(scope);
            res_obj.set(scope, signal_key.into(), signal_val.into());

            let pid_key = v8::String::new(scope, "pid").unwrap();
            let pid_val = v8::Integer::new(scope, 0);
            res_obj.set(scope, pid_key.into(), pid_val.into());

            let stdout_val =
                child_process_bytes_to_v8_value(scope, &output.stdout, encoding.as_deref());
            let stdout_key = v8::String::new(scope, "stdout").unwrap();
            res_obj.set(scope, stdout_key.into(), stdout_val);

            let stderr_val =
                child_process_bytes_to_v8_value(scope, &output.stderr, encoding.as_deref());
            let stderr_key = v8::String::new(scope, "stderr").unwrap();
            res_obj.set(scope, stderr_key.into(), stderr_val);

            let output_arr = v8::Array::new(scope, 3);
            let null_elem = v8::null(scope);
            output_arr.set_index(scope, 0, null_elem.into());
            output_arr.set_index(scope, 1, stdout_val);
            output_arr.set_index(scope, 2, stderr_val);
            let output_key = v8::String::new(scope, "output").unwrap();
            res_obj.set(scope, output_key.into(), output_arr.into());

            let error_key = v8::String::new(scope, "error").unwrap();
            let undef_val = v8::undefined(scope);
            res_obj.set(scope, error_key.into(), undef_val.into());
        }
        Err(e) => {
            let status_key = v8::String::new(scope, "status").unwrap();
            let status_val = v8::Integer::new(scope, 1);
            res_obj.set(scope, status_key.into(), status_val.into());

            let signal_key = v8::String::new(scope, "signal").unwrap();
            let null_signal = v8::null(scope);
            res_obj.set(scope, signal_key.into(), null_signal.into());

            let pid_key = v8::String::new(scope, "pid").unwrap();
            let pid_val = v8::Integer::new(scope, 0);
            res_obj.set(scope, pid_key.into(), pid_val.into());

            let empty_str = v8::String::new(scope, "").unwrap();
            let stdout_key = v8::String::new(scope, "stdout").unwrap();
            res_obj.set(scope, stdout_key.into(), empty_str.into());

            let stderr_val = v8::String::new(scope, &e.to_string()).unwrap();
            let stderr_key = v8::String::new(scope, "stderr").unwrap();
            res_obj.set(scope, stderr_key.into(), stderr_val.into());

            let err_msg = v8::String::new(scope, &e.to_string()).unwrap();
            let err_obj = v8::Exception::error(scope, err_msg);
            let error_key = v8::String::new(scope, "error").unwrap();
            res_obj.set(scope, error_key.into(), err_obj);
        }
    }
    retval.set(res_obj.into());
}

/// 设置child_process API
pub fn setup_child_process_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let cp_obj = v8::Object::new(scope);

    // exec
    let exec_func = v8::FunctionTemplate::new(scope, cp_exec_callback);
    let exec_instance = exec_func.get_function(scope).unwrap();
    let exec_key = v8::String::new(scope, "exec").unwrap();
    cp_obj.set(scope, exec_key.into(), exec_instance.into());

    // spawn
    let spawn_func = v8::FunctionTemplate::new(scope, cp_spawn_callback);
    let spawn_instance = spawn_func.get_function(scope).unwrap();
    let spawn_key = v8::String::new(scope, "spawn").unwrap();
    cp_obj.set(scope, spawn_key.into(), spawn_instance.into());

    // execFile
    let exec_file_func = v8::FunctionTemplate::new(scope, cp_exec_file_callback);
    let exec_file_instance = exec_file_func.get_function(scope).unwrap();
    let exec_file_key = v8::String::new(scope, "execFile").unwrap();
    cp_obj.set(scope, exec_file_key.into(), exec_file_instance.into());

    // execSync
    let exec_sync_func = v8::FunctionTemplate::new(scope, cp_exec_sync_callback);
    let exec_sync_instance = exec_sync_func.get_function(scope).unwrap();
    let exec_sync_key = v8::String::new(scope, "execSync").unwrap();
    cp_obj.set(scope, exec_sync_key.into(), exec_sync_instance.into());

    // spawnSync
    let spawn_sync_func = v8::FunctionTemplate::new(scope, cp_spawn_sync_callback);
    let spawn_sync_instance = spawn_sync_func.get_function(scope).unwrap();
    let spawn_sync_key = v8::String::new(scope, "spawnSync").unwrap();
    cp_obj.set(scope, spawn_sync_key.into(), spawn_sync_instance.into());

    // 设置到全局
    let global = context.global(scope);
    let cp_key = v8::String::new(scope, "child_process").unwrap();
    global.set(scope, cp_key.into(), cp_obj.into());

    Ok(())
}
