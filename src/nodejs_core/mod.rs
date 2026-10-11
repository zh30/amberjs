pub mod ai;
pub mod assert;
pub mod async_hooks;
pub mod buffer;
pub mod child_process;
pub mod child_process_fork;
pub mod commonjs_resolver;
pub mod crypto;
pub mod diagnostics_channel;
pub mod dns; // v0.3.67: DNS lookup and resolve API
pub mod events;
pub mod fast_path;
/// Node.js 核心 API 兼容层。
/// 兼容性进度以 tests/conformance/scorecard.md 的符合性分数为准，
/// 不要在此处或注释中宣称全面兼容。
pub mod fs;
pub mod fs_watch; // G64: fs.watch / watchFile (notify + poll)
pub mod http;
pub mod http2;
pub mod https;
pub mod net;
pub mod os;
pub mod path;
pub mod perf_hooks;
pub mod performance; // v0.3.275: Performance API (performance.now, performance.mark, etc.)
pub mod process; // v0.3.237: process 对象和未捕获异常处理器
pub mod querystring;
pub mod readline;
pub mod require; // v0.3.54: CommonJS module loader extracted to独立模块
pub mod stream;
pub mod string_decoder;
pub mod tcp_async; // v0.3.71: Async TCP connection module
pub mod timers; // v0.3.244: Timer API (setTimeout, setInterval, setImmediate)
pub mod tls;
pub mod tty;
pub mod url;
pub mod util; // v0.3.277: Readline API (createInterface, Interface.question, etc.)
pub mod vm;
pub mod worker_threads;
pub mod zlib;
use anyhow::Result;
use rusty_v8 as v8;
/// 设置所有Node.js核心API
pub fn setup_nodejs_core_apis(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    // 设置全局对象
    setup_globals(scope, context)?;
    // 设置核心模块
    fs::setup_fs_api(scope, context)?;
    crypto::setup_crypto_api(scope, context)?;
    stream::setup_stream_api(scope, context)?;
    events::setup_events_api(scope, context)?;
    // G64: after EventEmitter so FSWatcher can inherit EE.prototype.
    fs_watch::install_fs_watch_api(scope, context)?;
    net::setup_net_api(scope, context)?;
    http::setup_http_api(scope, context)?;
    http2::setup_http2_api(scope, context)?;
    https::setup_https_api(scope, context)?;
    tls::setup_tls_api(scope, context)?;
    dns::setup_dns_api(scope, context)?; // v0.3.67: DNS support
    buffer::setup_buffer_api(scope, context)?;
    path::setup_path_api(scope, context)?;
    os::setup_os_api(scope, context)?;
    util::setup_util_api(scope, context)?;
    url::setup_url_api(scope, context)?;
    querystring::setup_querystring_api(scope, context)?;
    child_process::setup_child_process_api(scope, context)?;
    process::setup_process_api(scope, context)?; // v0.3.237: process 对象
                                                 // Library install order puts process after child_process; retry fork-child IPC install.
    child_process_fork::install_fork_child_if_needed(scope, context)?;
    timers::setup_timers_api(scope, context)?; // v0.3.244: Timer API
    performance::setup_performance_api(scope, context)?; // v0.3.275: Performance API
    readline::setup_readline_api(scope, context)?; // v0.3.277: Readline API
    assert::setup_assert_api(scope, context)?;
    zlib::setup_zlib_api(scope, context)?;
    vm::setup_vm_api(scope, context)?;
    tty::setup_tty_api(scope, context)?;
    worker_threads::setup_worker_threads_api(scope, context)?;
    diagnostics_channel::setup_diagnostics_channel_api(scope, context)?;
    async_hooks::setup_async_hooks_api(scope, context)?;
    string_decoder::setup_string_decoder_api(scope, context)?;
    perf_hooks::setup_perf_hooks_api(scope, context)?;
    ai::setup_ai_api(scope, context)?;
    crate::database::setup_db_api(scope, context)?;
    crate::std_lib::setup_std_api(scope, context)?;
    // v0.3.54: 设置 CommonJS require 模块（必须最后设置，因为它依赖其他模块）
    require::setup_require_api(scope, context)?;
    Ok(())
}
/// 设置Node.js全局对象
fn setup_globals(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global: _ = context.global(scope);
    // 设置global对象
    let global_obj: _ = v8::Object::new(scope);
    let global_key: _ = v8::String::new(scope, "global").unwrap();
    global.set(scope, global_key.into(), global_obj.into());
    // 设置GLOBAL别名
    let global_alias_key: _ = v8::String::new(scope, "GLOBAL").unwrap();
    global.set(scope, global_alias_key.into(), global_obj.into());
    // 设置__dirname
    let dirname_key: _ = v8::String::new(scope, "__dirname").unwrap();
    let dirname_val: _ = v8::String::new(scope, "/").unwrap();
    global.set(scope, dirname_key.into(), dirname_val.into());
    // 设置__filename
    let filename_key: _ = v8::String::new(scope, "__filename").unwrap();
    let filename_val: _ = v8::String::new(scope, "main.js").unwrap();
    global.set(scope, filename_key.into(), filename_val.into());
    Ok(())
}
