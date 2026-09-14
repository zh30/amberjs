// Node.js child_process模块实现
/// 子进程管理
use anyhow::Result;
use rusty_v8 as v8;
use std::process::Command;

fn string_from_v8_value(scope: &mut v8::PinScope, value: v8::Local<v8::Value>) -> String {
    value
        .to_string(scope)
        .map(|value| value.to_rust_string_lossy(scope))
        .unwrap_or_default()
}

fn string_vec_from_v8_array_value(
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

fn run_shell_command(command: &str) -> std::io::Result<std::process::Output> {
    #[cfg(windows)]
    {
        Command::new("cmd").args(["/C", command]).output()
    }

    #[cfg(not(windows))]
    {
        Command::new("sh").arg("-c").arg(command).output()
    }
}

struct ChildProcessOutput {
    stdout: String,
    stderr: String,
    exit_code: i32,
}

fn child_process_output_from_result(
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

fn child_process_output_object<'s>(
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
    let pid_key_val = v8::Integer::new(scope, 0).into();
    child_obj.set(scope, pid_key.into(), pid_key_val);

    let killed_key = v8::String::new(scope, "killed").unwrap();
    let killed_val = v8::Boolean::new(scope, false);
    child_obj.set(scope, killed_key.into(), killed_val.into());

    let exit_code_key = v8::String::new(scope, "exitCode").unwrap();
    let exit_code_val = v8::Integer::new(scope, output.exit_code);
    child_obj.set(scope, exit_code_key.into(), exit_code_val.into());

    let signal_key = v8::String::new(scope, "signal").unwrap();
    let signal_val = v8::null(scope);
    child_obj.set(scope, signal_key.into(), signal_val.into());

    let on_func: _ = v8::FunctionTemplate::new(scope, child_on_callback);
    let on_instance: _ = on_func.get_function(scope).unwrap();
    let on_key: _ = v8::String::new(scope, "on").unwrap();
    child_obj.set(scope, on_key.into(), on_instance.into());

    child_obj
}

fn child_process_error_value<'s>(
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

fn call_child_process_callback(
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

/// 设置child_process API
pub fn setup_child_process_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let cp_obj: _ = v8::Object::new(scope);
    // exec
    let exec_func: _ = v8::FunctionTemplate::new(scope, cp_exec_callback);
    let exec_instance: _ = exec_func.get_function(scope).unwrap();
    let exec_key: _ = v8::String::new(scope, "exec").unwrap();
    cp_obj.set(scope, exec_key.into(), exec_instance.into());
    // spawn
    let spawn_func: _ = v8::FunctionTemplate::new(scope, cp_spawn_callback);
    let spawn_instance: _ = spawn_func.get_function(scope).unwrap();
    let spawn_key: _ = v8::String::new(scope, "spawn").unwrap();
    cp_obj.set(scope, spawn_key.into(), spawn_instance.into());
    // execFile
    let exec_file_func: _ = v8::FunctionTemplate::new(scope, cp_exec_file_callback);
    let exec_file_instance: _ = exec_file_func.get_function(scope).unwrap();
    let exec_file_key: _ = v8::String::new(scope, "execFile").unwrap();
    cp_obj.set(scope, exec_file_key.into(), exec_file_instance.into());
    // 设置到全局
    let global: _ = context.global(scope);
    let cp_key: _ = v8::String::new(scope, "child_process").unwrap();
    global.set(scope, cp_key.into(), cp_obj.into());
    Ok(())
}
fn cp_exec_callback(
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
    let callback = if args.get(1).is_function() {
        args.get(1)
    } else {
        args.get(2)
    };
    let output = child_process_output_from_result(run_shell_command(&command));
    call_child_process_callback(scope, callback, &output);
    let child_obj = child_process_output_object(scope, &output);
    // on
    let on_func: _ = v8::FunctionTemplate::new(scope, child_on_callback);
    let on_instance: _ = on_func.get_function(scope).unwrap();
    let on_key: _ = v8::String::new(scope, "on").unwrap();
    child_obj.set(scope, on_key.into(), on_instance.into());
    retval.set(child_obj.into());
}
fn cp_spawn_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let command: String = args
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
    // Non-blocking spawn: do not wait on stdout via Command::output().
    let child_obj = v8::Object::new(scope);
    match Command::new(&command).args(&spawn_args).spawn() {
        Ok(mut child) => {
            let pid = child.id();
            let pid_key = v8::String::new(scope, "pid").unwrap();
            let pid_val: v8::Local<v8::Value> = v8::Number::new(scope, pid as f64).into();
            child_obj.set(scope, pid_key.into(), pid_val);
            // Detach wait onto a background thread so the V8 thread is not blocked.
            std::thread::spawn(move || {
                let _ = child.wait();
            });
            let killed_key = v8::String::new(scope, "killed").unwrap();
            let killed_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            child_obj.set(scope, killed_key.into(), killed_val);
            let connected_key = v8::String::new(scope, "connected").unwrap();
            let connected_val: v8::Local<v8::Value> = v8::Boolean::new(scope, false).into();
            child_obj.set(scope, connected_key.into(), connected_val);
            let on_func = v8::FunctionTemplate::new(scope, child_on_callback);
            let on_instance = on_func.get_function(scope).unwrap();
            let on_key = v8::String::new(scope, "on").unwrap();
            child_obj.set(scope, on_key.into(), on_instance.into());
        }
        Err(err) => {
            let msg = v8::String::new(scope, &format!("spawn {}", err)).unwrap();
            let exc = v8::Exception::error(scope, msg);
            scope.throw_exception(exc);
            return;
        }
    }
    retval.set(child_obj.into());
}
fn cp_exec_file_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let file: String = args
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
    let args_or_callback: v8::Local<v8::Value> = args.get(1);
    let callback = if args_or_callback.is_function() {
        args_or_callback
    } else if args.get(2).is_function() {
        args.get(2)
    } else {
        args.get(3)
    };
    let exec_args = string_vec_from_v8_array_value(scope, args_or_callback);
    let output = child_process_output_from_result(Command::new(&file).args(exec_args).output());
    call_child_process_callback(scope, callback, &output);
    let child_obj = child_process_output_object(scope, &output);
    retval.set(child_obj.into());
}
fn child_on_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let event: _ = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    let listener: _ = args.get(1);
    if !listener.is_function() {
        retval.set(this.into());
        return;
    }
    if event == "exit" || event == "close" {
        if listener.is_function() {
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
                listener_func.call(scope, this.into(), &call_args);
            }
        }
    }
    retval.set(this.into());
}
