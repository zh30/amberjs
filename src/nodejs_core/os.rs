// Node.js OS模块实现
// 操作系统信息

use anyhow::Result;
use rusty_v8 as v8;
use std::env;
/// 设置OS API
pub fn setup_os_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let os_obj: _ = v8::Object::new(scope);
    // arch
    let arch_func: _ = v8::FunctionTemplate::new(scope, os_arch_callback);
    let arch_instance: _ = arch_func.get_function(scope).unwrap();
    let arch_key: _ = v8::String::new(scope, "arch").unwrap();
    os_obj.set(scope, arch_key.into(), arch_instance.into());
    // platform
    let platform_func: _ = v8::FunctionTemplate::new(scope, os_platform_callback);
    let platform_instance: _ = platform_func.get_function(scope).unwrap();
    let platform_key: _ = v8::String::new(scope, "platform").unwrap();
    os_obj.set(scope, platform_key.into(), platform_instance.into());
    // type
    let type_func: _ = v8::FunctionTemplate::new(scope, os_type_callback);
    let type_instance: _ = type_func.get_function(scope).unwrap();
    let type_key: _ = v8::String::new(scope, "type").unwrap();
    os_obj.set(scope, type_key.into(), type_instance.into());
    // release
    let release_func: _ = v8::FunctionTemplate::new(scope, os_release_callback);
    let release_instance: _ = release_func.get_function(scope).unwrap();
    let release_key: _ = v8::String::new(scope, "release").unwrap();
    os_obj.set(scope, release_key.into(), release_instance.into());
    // hostname
    let hostname_func: _ = v8::FunctionTemplate::new(scope, os_hostname_callback);
    let hostname_instance: _ = hostname_func.get_function(scope).unwrap();
    let hostname_key: _ = v8::String::new(scope, "hostname").unwrap();
    os_obj.set(scope, hostname_key.into(), hostname_instance.into());
    // loadavg
    let loadavg_func: _ = v8::FunctionTemplate::new(scope, os_loadavg_callback);
    let loadavg_instance: _ = loadavg_func.get_function(scope).unwrap();
    let loadavg_key: _ = v8::String::new(scope, "loadavg").unwrap();
    os_obj.set(scope, loadavg_key.into(), loadavg_instance.into());
    // uptime
    let uptime_func: _ = v8::FunctionTemplate::new(scope, os_uptime_callback);
    let uptime_instance: _ = uptime_func.get_function(scope).unwrap();
    let uptime_key: _ = v8::String::new(scope, "uptime").unwrap();
    os_obj.set(scope, uptime_key.into(), uptime_instance.into());
    // cpus
    let cpus_func: _ = v8::FunctionTemplate::new(scope, os_cpus_callback);
    let cpus_instance: _ = cpus_func.get_function(scope).unwrap();
    let cpus_key: _ = v8::String::new(scope, "cpus").unwrap();
    os_obj.set(scope, cpus_key.into(), cpus_instance.into());
    // freemem
    let freemem_func: _ = v8::FunctionTemplate::new(scope, os_freemem_callback);
    let freemem_instance: _ = freemem_func.get_function(scope).unwrap();
    let freemem_key: _ = v8::String::new(scope, "freemem").unwrap();
    os_obj.set(scope, freemem_key.into(), freemem_instance.into());
    // totalmem
    let totalmem_func: _ = v8::FunctionTemplate::new(scope, os_totalmem_callback);
    let totalmem_instance: _ = totalmem_func.get_function(scope).unwrap();
    let totalmem_key: _ = v8::String::new(scope, "totalmem").unwrap();
    os_obj.set(scope, totalmem_key.into(), totalmem_instance.into());
    // homedir
    let homedir_func: _ = v8::FunctionTemplate::new(scope, os_homedir_callback);
    let homedir_instance: _ = homedir_func.get_function(scope).unwrap();
    let homedir_key: _ = v8::String::new(scope, "homedir").unwrap();
    os_obj.set(scope, homedir_key.into(), homedir_instance.into());
    // tmpdir
    let tmpdir_func: _ = v8::FunctionTemplate::new(scope, os_tmpdir_callback);
    let tmpdir_instance: _ = tmpdir_func.get_function(scope).unwrap();
    let tmpdir_key: _ = v8::String::new(scope, "tmpdir").unwrap();
    os_obj.set(scope, tmpdir_key.into(), tmpdir_instance.into());
    // tmpDir (别名)
    let tmpdir_alias_func: _ = v8::FunctionTemplate::new(scope, os_tmpdir_callback);
    let tmpdir_alias_instance: _ = tmpdir_alias_func.get_function(scope).unwrap();
    let tmpdir_alias_key: _ = v8::String::new(scope, "tmpDir").unwrap();
    os_obj.set(scope, tmpdir_alias_key.into(), tmpdir_alias_instance.into());
    // networkInterfaces
    let network_interfaces_func: _ =
        v8::FunctionTemplate::new(scope, os_network_interfaces_callback);
    let network_interfaces_instance: _ = network_interfaces_func.get_function(scope).unwrap();
    let network_interfaces_key: _ = v8::String::new(scope, "networkInterfaces").unwrap();
    os_obj.set(
        scope,
        network_interfaces_key.into(),
        network_interfaces_instance.into(),
    );
    // constants
    let constants_obj: _ = v8::Object::new(scope);
    // OS常量
    let key_uv_udp_reuseaddr: _ = v8::String::new(scope, "UV_UDP_REUSEADDR").unwrap();
    let val_uv_udp_reuseaddr: _ = v8::Integer::new(scope, 4);
    constants_obj.set(
        scope,
        key_uv_udp_reuseaddr.into(),
        val_uv_udp_reuseaddr.into(),
    );
    // Signal常量
    let signals_obj: _ = v8::Object::new(scope);
    let key_sighup: _ = v8::String::new(scope, "SIGHUP").unwrap();
    let val_sighup: _ = v8::Integer::new(scope, 1);
    signals_obj.set(scope, key_sighup.into(), val_sighup.into());
    let key_sigint: _ = v8::String::new(scope, "SIGINT").unwrap();
    let val_sigint: _ = v8::Integer::new(scope, 2);
    signals_obj.set(scope, key_sigint.into(), val_sigint.into());
    let key_sigquit: _ = v8::String::new(scope, "SIGQUIT").unwrap();
    let val_sigquit: _ = v8::Integer::new(scope, 3);
    signals_obj.set(scope, key_sigquit.into(), val_sigquit.into());
    let key_sigill: _ = v8::String::new(scope, "SIGILL").unwrap();
    let val_sigill: _ = v8::Integer::new(scope, 4);
    signals_obj.set(scope, key_sigill.into(), val_sigill.into());
    let key_sigtrap: _ = v8::String::new(scope, "SIGTRAP").unwrap();
    let val_sigtrap: _ = v8::Integer::new(scope, 5);
    signals_obj.set(scope, key_sigtrap.into(), val_sigtrap.into());
    let key_sigabrt: _ = v8::String::new(scope, "SIGABRT").unwrap();
    let val_sigabrt: _ = v8::Integer::new(scope, 6);
    signals_obj.set(scope, key_sigabrt.into(), val_sigabrt.into());
    let key_sigiot: _ = v8::String::new(scope, "SIGIOT").unwrap();
    let val_sigiot: _ = v8::Integer::new(scope, 6);
    signals_obj.set(scope, key_sigiot.into(), val_sigiot.into());
    let key_sigbus: _ = v8::String::new(scope, "SIGBUS").unwrap();
    let val_sigbus: _ = v8::Integer::new(scope, 7);
    signals_obj.set(scope, key_sigbus.into(), val_sigbus.into());
    let key_sigfpe: _ = v8::String::new(scope, "SIGFPE").unwrap();
    let val_sigfpe: _ = v8::Integer::new(scope, 8);
    signals_obj.set(scope, key_sigfpe.into(), val_sigfpe.into());
    let key_sigkill: _ = v8::String::new(scope, "SIGKILL").unwrap();
    let val_sigkill: _ = v8::Integer::new(scope, 9);
    signals_obj.set(scope, key_sigkill.into(), val_sigkill.into());
    let key_sigusr1: _ = v8::String::new(scope, "SIGUSR1").unwrap();
    let val_sigusr1: _ = v8::Integer::new(scope, 10);
    signals_obj.set(scope, key_sigusr1.into(), val_sigusr1.into());
    let key_sigusr2: _ = v8::String::new(scope, "SIGUSR2").unwrap();
    let val_sigusr2: _ = v8::Integer::new(scope, 12);
    signals_obj.set(scope, key_sigusr2.into(), val_sigusr2.into());
    let key_sigpipe: _ = v8::String::new(scope, "SIGPIPE").unwrap();
    let val_sigpipe: _ = v8::Integer::new(scope, 13);
    signals_obj.set(scope, key_sigpipe.into(), val_sigpipe.into());
    let key_sigalrm: _ = v8::String::new(scope, "SIGALRM").unwrap();
    let val_sigalrm: _ = v8::Integer::new(scope, 14);
    signals_obj.set(scope, key_sigalrm.into(), val_sigalrm.into());
    let key_sigterm: _ = v8::String::new(scope, "SIGTERM").unwrap();
    let val_sigterm: _ = v8::Integer::new(scope, 15);
    signals_obj.set(scope, key_sigterm.into(), val_sigterm.into());
    let key_sigchld: _ = v8::String::new(scope, "SIGCHLD").unwrap();
    let val_sigchld: _ = v8::Integer::new(scope, 17);
    signals_obj.set(scope, key_sigchld.into(), val_sigchld.into());
    let key_sigcld: _ = v8::String::new(scope, "SIGCLD").unwrap();
    let val_sigcld: _ = v8::Integer::new(scope, 17);
    signals_obj.set(scope, key_sigcld.into(), val_sigcld.into());
    let key_sigcont: _ = v8::String::new(scope, "SIGCONT").unwrap();
    let val_sigcont: _ = v8::Integer::new(scope, 18);
    signals_obj.set(scope, key_sigcont.into(), val_sigcont.into());
    let key_sigstop: _ = v8::String::new(scope, "SIGSTOP").unwrap();
    let val_sigstop: _ = v8::Integer::new(scope, 19);
    signals_obj.set(scope, key_sigstop.into(), val_sigstop.into());
    let key_sigtstp: _ = v8::String::new(scope, "SIGTSTP").unwrap();
    let val_sigtstp: _ = v8::Integer::new(scope, 20);
    signals_obj.set(scope, key_sigtstp.into(), val_sigtstp.into());
    let key_sigttin: _ = v8::String::new(scope, "SIGTTIN").unwrap();
    let val_sigttin: _ = v8::Integer::new(scope, 21);
    signals_obj.set(scope, key_sigttin.into(), val_sigttin.into());
    let key_sigttou: _ = v8::String::new(scope, "SIGTTOU").unwrap();
    let val_sigttou: _ = v8::Integer::new(scope, 22);
    signals_obj.set(scope, key_sigttou.into(), val_sigttou.into());
    let key_sigurg: _ = v8::String::new(scope, "SIGURG").unwrap();
    let val_sigurg: _ = v8::Integer::new(scope, 23);
    signals_obj.set(scope, key_sigurg.into(), val_sigurg.into());
    let key_sigxcpu: _ = v8::String::new(scope, "SIGXCPU").unwrap();
    let val_sigxcpu: _ = v8::Integer::new(scope, 24);
    signals_obj.set(scope, key_sigxcpu.into(), val_sigxcpu.into());
    let key_sigxfsz: _ = v8::String::new(scope, "SIGXFSZ").unwrap();
    let val_sigxfsz: _ = v8::Integer::new(scope, 25);
    signals_obj.set(scope, key_sigxfsz.into(), val_sigxfsz.into());
    let key_sigvtalrm: _ = v8::String::new(scope, "SIGVTALRM").unwrap();
    let val_sigvtalrm: _ = v8::Integer::new(scope, 26);
    signals_obj.set(scope, key_sigvtalrm.into(), val_sigvtalrm.into());
    let key_sigprof: _ = v8::String::new(scope, "SIGPROF").unwrap();
    let val_sigprof: _ = v8::Integer::new(scope, 27);
    signals_obj.set(scope, key_sigprof.into(), val_sigprof.into());
    let key_sigwinch: _ = v8::String::new(scope, "SIGWINCH").unwrap();
    let val_sigwinch: _ = v8::Integer::new(scope, 28);
    signals_obj.set(scope, key_sigwinch.into(), val_sigwinch.into());
    let key_sigio: _ = v8::String::new(scope, "SIGIO").unwrap();
    let val_sigio: _ = v8::Integer::new(scope, 29);
    signals_obj.set(scope, key_sigio.into(), val_sigio.into());
    let key_sigpoll: _ = v8::String::new(scope, "SIGPOLL").unwrap();
    let val_sigpoll: _ = v8::Integer::new(scope, 29);
    signals_obj.set(scope, key_sigpoll.into(), val_sigpoll.into());
    let key_sigpwr: _ = v8::String::new(scope, "SIGPWR").unwrap();
    let val_sigpwr: _ = v8::Integer::new(scope, 30);
    signals_obj.set(scope, key_sigpwr.into(), val_sigpwr.into());
    let key_sigsys: _ = v8::String::new(scope, "SIGSYS").unwrap();
    let val_sigsys: _ = v8::Integer::new(scope, 31);
    signals_obj.set(scope, key_sigsys.into(), val_sigsys.into());
    let key_sigunused: _ = v8::String::new(scope, "SIGUNUSED").unwrap();
    let val_sigunused: _ = v8::Integer::new(scope, 31);
    signals_obj.set(scope, key_sigunused.into(), val_sigunused.into());
    let signals_key: _ = v8::String::new(scope, "signals").unwrap();
    constants_obj.set(scope, signals_key.into(), signals_obj.into());
    let constants_key: _ = v8::String::new(scope, "constants").unwrap();
    os_obj.set(scope, constants_key.into(), constants_obj.into());
    // EOL常量
    let eol: _ = if cfg!(windows) { "\r\n" } else { "\n" };
    let key_eol: _ = v8::String::new(scope, "EOL").unwrap();
    let val_eol: _ = v8::String::new(scope, eol).unwrap();
    os_obj.set(scope, key_eol.into(), val_eol.into());
    // arch()的常量版本
    let arch_constants_obj: _ = v8::Object::new(scope);
    let key_arm: _ = v8::String::new(scope, "arm").unwrap();
    let val_arm: _ = v8::String::new(scope, "arm").unwrap();
    arch_constants_obj.set(scope, key_arm.into(), val_arm.into());
    let key_arm64: _ = v8::String::new(scope, "arm64").unwrap();
    let val_arm64: _ = v8::String::new(scope, "arm64").unwrap();
    arch_constants_obj.set(scope, key_arm64.into(), val_arm64.into());
    let key_ia32: _ = v8::String::new(scope, "ia32").unwrap();
    let val_ia32: _ = v8::String::new(scope, "ia32").unwrap();
    arch_constants_obj.set(scope, key_ia32.into(), val_ia32.into());
    let key_mips: _ = v8::String::new(scope, "mips").unwrap();
    let val_mips: _ = v8::String::new(scope, "mips").unwrap();
    arch_constants_obj.set(scope, key_mips.into(), val_mips.into());
    let key_mipsel: _ = v8::String::new(scope, "mipsel").unwrap();
    let val_mipsel: _ = v8::String::new(scope, "mipsel").unwrap();
    arch_constants_obj.set(scope, key_mipsel.into(), val_mipsel.into());
    let key_ppc: _ = v8::String::new(scope, "ppc").unwrap();
    let val_ppc: _ = v8::String::new(scope, "ppc").unwrap();
    arch_constants_obj.set(scope, key_ppc.into(), val_ppc.into());
    let key_ppc64: _ = v8::String::new(scope, "ppc64").unwrap();
    let val_ppc64: _ = v8::String::new(scope, "ppc64").unwrap();
    arch_constants_obj.set(scope, key_ppc64.into(), val_ppc64.into());
    let key_riscv64: _ = v8::String::new(scope, "riscv64").unwrap();
    let val_riscv64: _ = v8::String::new(scope, "riscv64").unwrap();
    arch_constants_obj.set(scope, key_riscv64.into(), val_riscv64.into());
    let key_s390: _ = v8::String::new(scope, "s390").unwrap();
    let val_s390: _ = v8::String::new(scope, "s390").unwrap();
    arch_constants_obj.set(scope, key_s390.into(), val_s390.into());
    let key_s390x: _ = v8::String::new(scope, "s390x").unwrap();
    let val_s390x: _ = v8::String::new(scope, "s390x").unwrap();
    arch_constants_obj.set(scope, key_s390x.into(), val_s390x.into());
    let key_x64: _ = v8::String::new(scope, "x64").unwrap();
    let val_x64: _ = v8::String::new(scope, "x64").unwrap();
    arch_constants_obj.set(scope, key_x64.into(), val_x64.into());
    let key_x86: _ = v8::String::new(scope, "x86").unwrap();
    let val_x86: _ = v8::String::new(scope, "x86").unwrap();
    arch_constants_obj.set(scope, key_x86.into(), val_x86.into());
    let arch_key: _ = v8::String::new(scope, "arch").unwrap();
    os_obj.set(scope, arch_key.into(), arch_constants_obj.into());
    // platform()的常量版本
    let platform_constants_obj: _ = v8::Object::new(scope);
    let key_aix: _ = v8::String::new(scope, "aix").unwrap();
    let val_aix: _ = v8::String::new(scope, "aix").unwrap();
    platform_constants_obj.set(scope, key_aix.into(), val_aix.into());
    let key_darwin: _ = v8::String::new(scope, "darwin").unwrap();
    let val_darwin: _ = v8::String::new(scope, "darwin").unwrap();
    platform_constants_obj.set(scope, key_darwin.into(), val_darwin.into());
    let key_freebsd: _ = v8::String::new(scope, "freebsd").unwrap();
    let val_freebsd: _ = v8::String::new(scope, "freebsd").unwrap();
    platform_constants_obj.set(scope, key_freebsd.into(), val_freebsd.into());
    let key_linux: _ = v8::String::new(scope, "linux").unwrap();
    let val_linux: _ = v8::String::new(scope, "linux").unwrap();
    platform_constants_obj.set(scope, key_linux.into(), val_linux.into());
    let key_openbsd: _ = v8::String::new(scope, "openbsd").unwrap();
    let val_openbsd: _ = v8::String::new(scope, "openbsd").unwrap();
    platform_constants_obj.set(scope, key_openbsd.into(), val_openbsd.into());
    let key_sunos: _ = v8::String::new(scope, "sunos").unwrap();
    let val_sunos: _ = v8::String::new(scope, "sunos").unwrap();
    platform_constants_obj.set(scope, key_sunos.into(), val_sunos.into());
    let key_win32: _ = v8::String::new(scope, "win32").unwrap();
    let val_win32: _ = v8::String::new(scope, "win32").unwrap();
    platform_constants_obj.set(scope, key_win32.into(), val_win32.into());
    let key_android: _ = v8::String::new(scope, "android").unwrap();
    let val_android: _ = v8::String::new(scope, "android").unwrap();
    platform_constants_obj.set(scope, key_android.into(), val_android.into());
    let key_cygwin: _ = v8::String::new(scope, "cygwin").unwrap();
    let val_cygwin: _ = v8::String::new(scope, "cygwin").unwrap();
    platform_constants_obj.set(scope, key_cygwin.into(), val_cygwin.into());
    let key_netbsd: _ = v8::String::new(scope, "netbsd").unwrap();
    let val_netbsd: _ = v8::String::new(scope, "netbsd").unwrap();
    platform_constants_obj.set(scope, key_netbsd.into(), val_netbsd.into());
    let platform_key: _ = v8::String::new(scope, "platform").unwrap();
    os_obj.set(scope, platform_key.into(), platform_constants_obj.into());
    // 设置到全局
    let global: _ = context.global(scope);
    let os_key: _ = v8::String::new(scope, "os").unwrap();
    global.set(scope, os_key.into(), os_obj.into());
    Ok(())
}
fn os_arch_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let arch: _ = std::env::consts::ARCH;
    retval.set(v8::String::new(scope, arch).unwrap().into());
}
fn os_platform_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let platform: _ = std::env::consts::OS;
    retval.set(v8::String::new(scope, platform).unwrap().into());
}
fn os_type_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let os_type: _ = if cfg!(windows) {
        "Windows_NT"
    } else if cfg!(target_os = "macos") {
        "Darwin"
    } else {
        "Linux"
    };
    retval.set(v8::String::new(scope, os_type).unwrap().into());
}
fn os_release_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let release = sys_info::os_release().unwrap_or_else(|_| {
        if cfg!(target_os = "macos") {
            "Darwin".to_string()
        } else if cfg!(target_os = "windows") {
            "Windows_NT".to_string()
        } else {
            "Linux".to_string()
        }
    });
    retval.set(v8::String::new(scope, &release).unwrap().into());
}
fn os_hostname_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if let Ok(hostname) = env::var("HOSTNAME") {
        retval.set(v8::String::new(scope, &hostname).unwrap().into());
    } else if let Ok(computer_name) = env::var("COMPUTERNAME") {
        retval.set(v8::String::new(scope, &computer_name).unwrap().into());
    } else {
        retval.set(v8::String::new(scope, "localhost").unwrap().into());
    }
}
fn os_loadavg_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let loadavg = v8::Array::new(scope, 3);
    let (a, b, c) = sys_info::loadavg()
        .map(|l| (l.one, l.five, l.fifteen))
        .unwrap_or((0.0, 0.0, 0.0));
    let v0: v8::Local<v8::Value> = v8::Number::new(scope, a).into();
    let v1: v8::Local<v8::Value> = v8::Number::new(scope, b).into();
    let v2: v8::Local<v8::Value> = v8::Number::new(scope, c).into();
    loadavg.set_index(scope, 0, v0);
    loadavg.set_index(scope, 1, v1);
    loadavg.set_index(scope, 2, v2);
    retval.set(loadavg.into());
}
fn os_uptime_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let uptime = {
        #[cfg(windows)]
        {
            // sys_info::boottime is cfg(not(windows)); GetTickCount64 is ms since boot.
            use windows_sys::Win32::System::SystemInformation::GetTickCount64;
            unsafe { GetTickCount64() / 1000 }
        }
        #[cfg(not(windows))]
        {
            match sys_info::boottime() {
                Ok(bt) => {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    now.saturating_sub(bt.tv_sec as u64)
                }
                Err(_) => 0,
            }
        }
    };
    retval.set(v8::Number::new(scope, uptime as f64).into());
}
fn os_cpus_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let cpu_count = num_cpus::get();
    let model = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim().to_string())
        })
        .or_else(|| {
            // macOS: sysctl
            std::process::Command::new("sysctl")
                .args(["-n", "machdep.cpu.brand_string"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| format!("{} CPU", std::env::consts::ARCH));
    let cpus_array = v8::Array::new(scope, cpu_count as i32);
    for i in 0..cpu_count {
        let cpu_obj = v8::Object::new(scope);
        let model_key = v8::String::new(scope, "model").unwrap();
        let model_val = v8::String::new(scope, &model).unwrap();
        cpu_obj.set(scope, model_key.into(), model_val.into());
        let key_speed = v8::String::new(scope, "speed").unwrap();
        let val_speed = v8::Number::new(scope, 0.0);
        cpu_obj.set(scope, key_speed.into(), val_speed.into());
        let times_obj = v8::Object::new(scope);
        for (name, val) in [
            ("user", 0.0),
            ("nice", 0.0),
            ("sys", 0.0),
            ("idle", 0.0),
            ("irq", 0.0),
        ] {
            let key = v8::String::new(scope, name).unwrap();
            let num = v8::Number::new(scope, val);
            times_obj.set(scope, key.into(), num.into());
        }
        let times_key = v8::String::new(scope, "times").unwrap();
        cpu_obj.set(scope, times_key.into(), times_obj.into());
        cpus_array.set_index(scope, i as u32, cpu_obj.into());
    }
    retval.set(cpus_array.into());
}
fn os_freemem_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let freemem = sys_info::mem_info().map(|m| m.avail * 1024).unwrap_or(0);
    retval.set(v8::Number::new(scope, freemem as f64).into());
}
fn os_totalmem_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let totalmem = sys_info::mem_info().map(|m| m.total * 1024).unwrap_or(0);
    retval.set(v8::Number::new(scope, totalmem as f64).into());
}
fn os_homedir_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    if let Some(home_dir) = dirs::home_dir() {
        retval.set(
            v8::String::new(scope, &home_dir.to_string_lossy())
                .unwrap()
                .into(),
        );
    } else {
        retval.set(v8::String::new(scope, "/home/user").unwrap().into());
    }
}
fn os_tmpdir_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let tmpdir: _ = if cfg!(windows) {
        env::var("TEMP").unwrap_or_else(|_| "C:\\Windows\\Temp".to_string())
    } else {
        "/tmp".to_string()
    };
    retval.set(v8::String::new(scope, &tmpdir).unwrap().into());
}
fn os_network_interfaces_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // 简化的网络接口
    let interfaces_obj: _ = v8::Object::new(scope);
    // Loopback接口
    let loopback_array: _ = v8::Array::new(scope, 1);
    let loopback_obj: _ = v8::Object::new(scope);
    let key_address: _ = v8::String::new(scope, "address").unwrap();
    let val_address: _ = v8::String::new(scope, "127.0.0.1").unwrap();
    loopback_obj.set(scope, key_address.into(), val_address.into());
    let key_netmask: _ = v8::String::new(scope, "netmask").unwrap();
    let val_netmask: _ = v8::String::new(scope, "255.0.0.0").unwrap();
    loopback_obj.set(scope, key_netmask.into(), val_netmask.into());
    let key_family: _ = v8::String::new(scope, "family").unwrap();
    let val_family: _ = v8::String::new(scope, "IPv4").unwrap();
    loopback_obj.set(scope, key_family.into(), val_family.into());
    let key_internal: _ = v8::String::new(scope, "internal").unwrap();
    let val_internal: _ = v8::Boolean::new(scope, true);
    loopback_obj.set(scope, key_internal.into(), val_internal.into());
    loopback_array.set_index(scope, 0, loopback_obj.into());
    let _key_2: _ = v8::String::new(scope, "lo").unwrap();
    interfaces_obj.set(scope, _key_2.into(), loopback_array.into());
    let _key_3: _ = v8::String::new(scope, "lo0").unwrap();
    interfaces_obj.set(scope, _key_3.into(), loopback_array.into());
    retval.set(interfaces_obj.into());
}
