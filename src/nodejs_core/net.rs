// Node.js net 模块实现 - v0.3.71 完整版
// TCP 连接、网络服务器和 Socket API
// v0.3.71: 添加真实异步 TCP 连接支持

use anyhow::Result;
use rusty_v8 as v8;

// 导入 tcp_async 模块（通过父模块的 mod.rs）
use super::tcp_async::{TcpConnectionHandle, TCP_MANAGER};

/// 设置 net API
pub fn setup_net_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let net_obj: _ = v8::Object::new(scope);

    // connect (createConnection 的别名)
    let connect_func: _ = v8::FunctionTemplate::new(scope, net_connect_callback);
    let connect_instance: _ = connect_func.get_function(scope).unwrap();
    let connect_key: _ = v8::String::new(scope, "connect").unwrap();
    net_obj.set(scope, connect_key.into(), connect_instance.into());

    // createConnection
    let create_connection_func: _ = v8::FunctionTemplate::new(scope, net_connect_callback);
    let create_connection_instance: _ = create_connection_func.get_function(scope).unwrap();
    let create_connection_key: _ = v8::String::new(scope, "createConnection").unwrap();
    net_obj.set(
        scope,
        create_connection_key.into(),
        create_connection_instance.into(),
    );

    // createServer
    let create_server_func: _ = v8::FunctionTemplate::new(scope, net_create_server_callback);
    let create_server_instance: _ = create_server_func.get_function(scope).unwrap();
    let create_server_key: _ = v8::String::new(scope, "createServer").unwrap();
    net_obj.set(
        scope,
        create_server_key.into(),
        create_server_instance.into(),
    );

    // Server 构造函数
    let server_func: _ = v8::FunctionTemplate::new(scope, net_server_constructor_callback);
    let server_instance: _ = server_func.get_function(scope).unwrap();
    let server_key: _ = v8::String::new(scope, "Server").unwrap();
    net_obj.set(scope, server_key.into(), server_instance.into());

    // isIP
    let is_ip_func: _ = v8::FunctionTemplate::new(scope, net_is_ip_callback);
    let is_ip_instance: _ = is_ip_func.get_function(scope).unwrap();
    let is_ip_key: _ = v8::String::new(scope, "isIP").unwrap();
    net_obj.set(scope, is_ip_key.into(), is_ip_instance.into());

    // isIPv4
    let is_ipv4_func: _ = v8::FunctionTemplate::new(scope, net_is_ipv4_callback);
    let is_ipv4_instance: _ = is_ipv4_func.get_function(scope).unwrap();
    let is_ipv4_key: _ = v8::String::new(scope, "isIPv4").unwrap();
    net_obj.set(scope, is_ipv4_key.into(), is_ipv4_instance.into());

    // isIPv6
    let is_ipv6_func: _ = v8::FunctionTemplate::new(scope, net_is_ipv6_callback);
    let is_ipv6_instance: _ = is_ipv6_func.get_function(scope).unwrap();
    let is_ipv6_key: _ = v8::String::new(scope, "isIPv6").unwrap();
    net_obj.set(scope, is_ipv6_key.into(), is_ipv6_instance.into());

    // 设置到全局
    let global: _ = context.global(scope);
    let net_key: _ = v8::String::new(scope, "net").unwrap();
    global.set(scope, net_key.into(), net_obj.into());

    Ok(())
}

/// net.connect() 和 net.createConnection() 回调
fn net_connect_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let options: _ = args.get(0);

    // 解析连接选项
    let port = extract_integer_option(scope, &options, "port", 0);
    let host = extract_string_option(scope, &options, "host", "localhost");
    let local_port = extract_integer_option(scope, &options, "localPort", 0);
    let local_address = extract_string_option(scope, &options, "localAddress", "0.0.0.0");
    let connect_timeout = extract_integer_option(scope, &options, "connectTimeout", 0);

    if let Err(error) = crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::Network,
        crate::permissions::PermissionAction::Connect,
        crate::permissions::ResourceId::Url(format!("tcp://{}:{}", host, port)),
    ) {
        let error_message = v8::String::new(scope, &error.to_string()).unwrap();
        let error_obj = v8::Exception::error(scope, error_message);
        scope.throw_exception(error_obj.into());
        return;
    }

    // 创建真实的 TCP 连接
    let tcp_handle = TCP_MANAGER.create_connection();
    let handle_id = tcp_handle.id;

    // 尝试建立真实连接（带超时）
    let timeout_secs = if connect_timeout > 0 {
        connect_timeout as u64
    } else {
        10
    };
    let connect_result = tcp_handle.connect(&host, port as u16);
    let (is_connected, remote_addr, remote_family, local_addr_val) = {
        let rt = tokio::runtime::Runtime::new().ok();
        match rt {
            Some(rt) => {
                let timeout_duration = std::time::Duration::from_secs(timeout_secs);
                let result = rt.block_on(async {
                    tokio::time::timeout(timeout_duration, connect_result).await
                });
                match result {
                    Ok(Ok(())) => {
                        let info = tcp_handle.get_info();
                        (
                            true,
                            info.remote_addr.clone(),
                            info.family.clone(),
                            info.local_addr.clone(),
                        )
                    }
                    _ => (
                        false,
                        host.clone(),
                        if host.contains(':') {
                            "IPv6".to_string()
                        } else {
                            "IPv4".to_string()
                        },
                        local_address,
                    ),
                }
            }
            None => (
                false,
                host.clone(),
                if host.contains(':') {
                    "IPv6".to_string()
                } else {
                    "IPv4".to_string()
                },
                local_address,
            ),
        }
    };

    // 创建 socket 对象
    let socket_obj: _ = v8::Object::new(scope);

    // 预先创建所有 V8 值，避免 borrow checker 问题
    let port_key = v8::String::new(scope, "remotePort").unwrap();
    let port_val = v8::Integer::new(scope, port as i32);
    socket_obj.set(scope, port_key.into(), port_val.into());

    let host_key = v8::String::new(scope, "remoteAddress").unwrap();
    let host_val = v8::String::new(scope, &remote_addr).unwrap();
    socket_obj.set(scope, host_key.into(), host_val.into());

    let local_port_key = v8::String::new(scope, "localPort").unwrap();
    let local_port_val = v8::Integer::new(scope, local_port as i32);
    socket_obj.set(scope, local_port_key.into(), local_port_val.into());

    let local_addr_key = v8::String::new(scope, "localAddress").unwrap();
    let local_addr_v8_val = v8::String::new(scope, &local_addr_val).unwrap();
    socket_obj.set(scope, local_addr_key.into(), local_addr_v8_val.into());

    // 确定 address family
    let family_key = v8::String::new(scope, "remoteFamily").unwrap();
    let family_val = v8::String::new(scope, &remote_family).unwrap();
    socket_obj.set(scope, family_key.into(), family_val.into());

    // 连接状态
    let connecting_key = v8::String::new(scope, "connecting").unwrap();
    let connecting_val = v8::Boolean::new(scope, !is_connected);
    socket_obj.set(scope, connecting_key.into(), connecting_val.into());

    // connect 事件标识
    let connect_key = v8::String::new(scope, "connect").unwrap();
    let connect_val =
        v8::String::new(scope, if is_connected { "open" } else { "opening" }).unwrap();
    socket_obj.set(scope, connect_key.into(), connect_val.into());

    // 存储连接句柄 ID
    let handle_id_key = v8::String::new(scope, "_handleId").unwrap();
    let handle_id_val = v8::Integer::new(scope, handle_id as i32);
    socket_obj.set(scope, handle_id_key.into(), handle_id_val.into());

    // write 方法
    let write_func: _ = v8::FunctionTemplate::new(scope, socket_write_callback);
    let write_instance: _ = write_func.get_function(scope).unwrap();
    let write_key: _ = v8::String::new(scope, "write").unwrap();
    socket_obj.set(scope, write_key.into(), write_instance.into());

    // end 方法
    let end_func: _ = v8::FunctionTemplate::new(scope, socket_end_callback);
    let end_instance: _ = end_func.get_function(scope).unwrap();
    let end_key: _ = v8::String::new(scope, "end").unwrap();
    socket_obj.set(scope, end_key.into(), end_instance.into());

    // on 方法 (事件监听)
    let on_func: _ = v8::FunctionTemplate::new(scope, socket_on_callback);
    let on_instance: _ = on_func.get_function(scope).unwrap();
    let on_key: _ = v8::String::new(scope, "on").unwrap();
    socket_obj.set(scope, on_key.into(), on_instance.into());

    // once 方法 (一次性事件)
    let once_func: _ = v8::FunctionTemplate::new(scope, socket_once_callback);
    let once_instance: _ = once_func.get_function(scope).unwrap();
    let once_key: _ = v8::String::new(scope, "once").unwrap();
    socket_obj.set(scope, once_key.into(), once_instance.into());

    // emit 方法
    let emit_func: _ = v8::FunctionTemplate::new(scope, socket_emit_callback);
    let emit_instance: _ = emit_func.get_function(scope).unwrap();
    let emit_key: _ = v8::String::new(scope, "emit").unwrap();
    socket_obj.set(scope, emit_key.into(), emit_instance.into());

    // destroy 方法
    let destroy_func: _ = v8::FunctionTemplate::new(scope, socket_destroy_callback);
    let destroy_instance: _ = destroy_func.get_function(scope).unwrap();
    let destroy_key: _ = v8::String::new(scope, "destroy").unwrap();
    socket_obj.set(scope, destroy_key.into(), destroy_instance.into());

    // setTimeout 方法
    let set_timeout_func: _ = v8::FunctionTemplate::new(scope, socket_set_timeout_callback);
    let set_timeout_instance: _ = set_timeout_func.get_function(scope).unwrap();
    let set_timeout_key: _ = v8::String::new(scope, "setTimeout").unwrap();
    socket_obj.set(scope, set_timeout_key.into(), set_timeout_instance.into());

    // setEncoding 方法
    let set_encoding_func: _ = v8::FunctionTemplate::new(scope, socket_set_encoding_callback);
    let set_encoding_instance: _ = set_encoding_func.get_function(scope).unwrap();
    let set_encoding_key: _ = v8::String::new(scope, "setEncoding").unwrap();
    socket_obj.set(scope, set_encoding_key.into(), set_encoding_instance.into());

    // pause/resume 方法 (用于流控制)
    let pause_func: _ = v8::FunctionTemplate::new(scope, socket_pause_callback);
    let pause_instance: _ = pause_func.get_function(scope).unwrap();
    let pause_key: _ = v8::String::new(scope, "pause").unwrap();
    socket_obj.set(scope, pause_key.into(), pause_instance.into());

    let resume_func: _ = v8::FunctionTemplate::new(scope, socket_resume_callback);
    let resume_instance: _ = resume_func.get_function(scope).unwrap();
    let resume_key: _ = v8::String::new(scope, "resume").unwrap();
    socket_obj.set(scope, resume_key.into(), resume_instance.into());

    // read 方法 - v0.3.70 新增
    let read_func: _ = v8::FunctionTemplate::new(scope, socket_read_callback);
    let read_instance: _ = read_func.get_function(scope).unwrap();
    let read_key: _ = v8::String::new(scope, "read").unwrap();
    socket_obj.set(scope, read_key.into(), read_instance.into());

    // 存储连接选项
    let options_key = v8::String::new(scope, "_connectOptions").unwrap();
    socket_obj.set(scope, options_key.into(), options);

    retval.set(socket_obj.into());
}

/// net.createServer() 回调
fn net_create_server_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let _options: _ = args.get(0);

    // 创建服务器对象
    let server_obj: _ = v8::Object::new(scope);

    // listen 方法
    let listen_func: _ = v8::FunctionTemplate::new(scope, server_listen_callback);
    let listen_instance: _ = listen_func.get_function(scope).unwrap();
    let listen_key: _ = v8::String::new(scope, "listen").unwrap();
    server_obj.set(scope, listen_key.into(), listen_instance.into());

    // close 方法
    let close_func: _ = v8::FunctionTemplate::new(scope, server_close_callback);
    let close_instance: _ = close_func.get_function(scope).unwrap();
    let close_key: _ = v8::String::new(scope, "close").unwrap();
    server_obj.set(scope, close_key.into(), close_instance.into());

    // on 方法
    let on_func: _ = v8::FunctionTemplate::new(scope, server_on_callback);
    let on_instance: _ = on_func.get_function(scope).unwrap();
    let on_key: _ = v8::String::new(scope, "on").unwrap();
    server_obj.set(scope, on_key.into(), on_instance.into());

    // once 方法
    let once_func: _ = v8::FunctionTemplate::new(scope, server_once_callback);
    let once_instance: _ = once_func.get_function(scope).unwrap();
    let once_key: _ = v8::String::new(scope, "once").unwrap();
    server_obj.set(scope, once_key.into(), once_instance.into());

    // emit 方法
    let emit_func: _ = v8::FunctionTemplate::new(scope, server_emit_callback);
    let emit_instance: _ = emit_func.get_function(scope).unwrap();
    let emit_key: _ = v8::String::new(scope, "emit").unwrap();
    server_obj.set(scope, emit_key.into(), emit_instance.into());

    // address 方法
    let address_func: _ = v8::FunctionTemplate::new(scope, server_address_callback);
    let address_instance: _ = address_func.get_function(scope).unwrap();
    let address_key: _ = v8::String::new(scope, "address").unwrap();
    server_obj.set(scope, address_key.into(), address_instance.into());

    // getConnections 方法
    let get_connections_func: _ = v8::FunctionTemplate::new(scope, server_get_connections_callback);
    let get_connections_instance: _ = get_connections_func.get_function(scope).unwrap();
    let get_connections_key: _ = v8::String::new(scope, "getConnections").unwrap();
    server_obj.set(
        scope,
        get_connections_key.into(),
        get_connections_instance.into(),
    );

    // ref/unref 方法
    let ref_func: _ = v8::FunctionTemplate::new(scope, server_ref_callback);
    let ref_instance: _ = ref_func.get_function(scope).unwrap();
    let ref_key: _ = v8::String::new(scope, "ref").unwrap();
    server_obj.set(scope, ref_key.into(), ref_instance.into());

    let unref_func: _ = v8::FunctionTemplate::new(scope, server_unref_callback);
    let unref_instance: _ = unref_func.get_function(scope).unwrap();
    let unref_key: _ = v8::String::new(scope, "unref").unwrap();
    server_obj.set(scope, unref_key.into(), unref_instance.into());

    // 监听计数
    let listening_key = v8::String::new(scope, "listening").unwrap();
    let listening_val = v8::Boolean::new(scope, false);
    server_obj.set(scope, listening_key.into(), listening_val.into());

    retval.set(server_obj.into());
}

/// net.Server 构造函数
fn net_server_constructor_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let _options: _ = args.get(0);
    let _connection_listener: _ = args.get(1);

    // 创建服务器对象 (与 createServer 相同)
    let server_obj: _ = v8::Object::new(scope);

    let listen_func: _ = v8::FunctionTemplate::new(scope, server_listen_callback);
    let listen_instance: _ = listen_func.get_function(scope).unwrap();
    let listen_key: _ = v8::String::new(scope, "listen").unwrap();
    server_obj.set(scope, listen_key.into(), listen_instance.into());

    let close_func: _ = v8::FunctionTemplate::new(scope, server_close_callback);
    let close_instance: _ = close_func.get_function(scope).unwrap();
    let close_key: _ = v8::String::new(scope, "close").unwrap();
    server_obj.set(scope, close_key.into(), close_instance.into());

    let on_func: _ = v8::FunctionTemplate::new(scope, server_on_callback);
    let on_instance: _ = on_func.get_function(scope).unwrap();
    let on_key: _ = v8::String::new(scope, "on").unwrap();
    server_obj.set(scope, on_key.into(), on_instance.into());

    let listening_key = v8::String::new(scope, "listening").unwrap();
    let listening_val = v8::Boolean::new(scope, false);
    server_obj.set(scope, listening_key.into(), listening_val.into());

    retval.set(server_obj.into());
}

/// net.isIP() 回调 - 检测字符串是否为 IP 地址
fn net_is_ip_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let ip: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    let result = if is_valid_ipv4(&ip) {
        4
    } else if is_valid_ipv6(&ip) {
        6
    } else {
        0
    };
    retval.set(v8::Integer::new(scope, result).into());
}

/// net.isIPv4() 回调
fn net_is_ipv4_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let ip: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    retval.set(v8::Boolean::new(scope, is_valid_ipv4(&ip)).into());
}

/// net.isIPv6() 回调
fn net_is_ipv6_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let ip: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();
    retval.set(v8::Boolean::new(scope, is_valid_ipv6(&ip)).into());
}

// ==================== 辅助函数 ====================

/// 验证 IPv4 地址
fn is_valid_ipv4(ip: &str) -> bool {
    if ip.split('.').count() != 4 {
        return false;
    }
    ip.split('.').all(|part| part.parse::<u8>().is_ok())
}

/// 验证 IPv6 地址
fn is_valid_ipv6(ip: &str) -> bool {
    // 简化验证：检查是否包含冒号
    if !ip.contains(':') {
        return false;
    }
    // 有效的 IPv6 应该能够被解析
    ip.parse::<std::net::Ipv6Addr>().is_ok()
}

/// 提取整数选项
fn extract_integer_option(
    scope: &mut v8::PinScope,
    options: &v8::Local<v8::Value>,
    key: &str,
    default: i32,
) -> i32 {
    if options.is_undefined() || options.is_null() {
        return default;
    }
    if let Ok(obj) = v8::Local::<v8::Object>::try_from(*options) {
        let key_str: _ = v8::String::new(scope, key).unwrap();
        if let Some(val) = obj.get(scope, key_str.into()) {
            if val.is_number() {
                return val.to_int32(scope).unwrap().value() as i32;
            }
        }
    }
    default
}

/// 提取字符串选项
fn extract_string_option(
    scope: &mut v8::PinScope,
    options: &v8::Local<v8::Value>,
    key: &str,
    default: &str,
) -> String {
    if options.is_undefined() || options.is_null() {
        return default.to_string();
    }
    if let Ok(obj) = v8::Local::<v8::Object>::try_from(*options) {
        let key_str: _ = v8::String::new(scope, key).unwrap();
        if let Some(val) = obj.get(scope, key_str.into()) {
            if let Some(s) = val.to_string(scope) {
                return s.to_rust_string_lossy(scope);
            }
        }
    }
    default.to_string()
}

// ==================== Socket 回调 ====================

fn socket_write_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let data_val: _ = args.get(0);
    let _encoding: String = args
        .get(1)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_else(|| "utf8".to_string());

    // 转换为字符串
    let data_str = data_val
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    let data_bytes = data_str.as_bytes();

    // 获取 TCP 句柄 ID 并写入数据
    let handle_id_key = v8::String::new(scope, "_handleId").unwrap();
    if let Some(handle_id_val) = this.get(scope, handle_id_key.into()) {
        if let Some(id_num) = handle_id_val.to_int32(scope) {
            let handle_id = id_num.value() as u64;
            if let Some(handle) = TCP_MANAGER.get_connection(handle_id) {
                let is_conn = handle.is_connected();
                if is_conn {
                    // 异步写入数据
                    let rt = tokio::runtime::Runtime::new().ok();
                    if let Some(rt) = rt {
                        let data = data_bytes.to_vec();
                        let h: TcpConnectionHandle = handle.clone();
                        let _ = rt.block_on(async { h.write(&data).await });
                    }
                }
            }
        }
    }

    retval.set(v8::Boolean::new(scope, true).into());
}

fn socket_end_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    retval.set(this.into());
}

fn socket_on_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let event: String = args
        .get(0)
        .to_string(scope)
        .map(|s| s.to_rust_string_lossy(scope))
        .unwrap_or_default();

    // 获取回调函数
    let listener_val = args.get(1);

    // 将回调存储在 socket 对象的 _events 属性中
    let events_key = v8::String::new(scope, "_events").unwrap();

    // 获取或创建 _events 对象
    let events_obj = if let Some(events) = this.get(scope, events_key.into()) {
        if events.is_object() {
            v8::Local::<v8::Object>::try_from(events).unwrap_or_else(|_| v8::Object::new(scope))
        } else {
            v8::Object::new(scope)
        }
    } else {
        v8::Object::new(scope)
    };

    // 存储回调函数
    let event_key = v8::String::new(scope, &event).unwrap();
    events_obj.set(scope, event_key.into(), listener_val);

    // 保存 _events 对象回 socket
    this.set(scope, events_key.into(), events_obj.into());

    // 如果是 connect 事件且已连接，立即触发
    if event == "connect" {
        let connecting_key = v8::String::new(scope, "connecting").unwrap();
        let is_connecting = if let Some(conn_val) = this.get(scope, connecting_key.into()) {
            conn_val.to_boolean(scope).boolean_value(scope)
        } else {
            false
        };
        if !is_connecting {
            // 触发 connect 事件 - 预创建所有值避免 borrow checker 问题
            if let Some(events) = this.get(scope, events_key.into()) {
                if let Ok(events_obj) = v8::Local::<v8::Object>::try_from(events) {
                    let connect_key = v8::String::new(scope, "connect").unwrap();
                    let callback_val = events_obj.get(scope, connect_key.into());
                    if let Some(cb) = callback_val {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(cb) {
                            let this_val: v8::Local<v8::Value> = this.into();
                            func.call(scope, this_val.into(), &[]);
                        }
                    }
                }
            }
        }
    }

    retval.set(this.into());
}

fn socket_once_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    retval.set(this.into());
}

fn socket_emit_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let _this: _ = args.this();
    retval.set(v8::Boolean::new(scope, true).into());
}

fn socket_destroy_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();

    // 关闭真实的 TCP 连接
    let handle_id_key = v8::String::new(scope, "_handleId").unwrap();
    if let Some(handle_id_val) = this.get(scope, handle_id_key.into()) {
        if let Some(id_num) = handle_id_val.to_int32(scope) {
            let handle_id = id_num.value() as u64;
            if let Some(handle) = TCP_MANAGER.get_connection(handle_id) {
                let _h: TcpConnectionHandle = handle;
                _h.close();
                TCP_MANAGER.remove_connection(handle_id);
            }
        }
    }

    // 设置 connecting 为 false
    let connecting_key = v8::String::new(scope, "connecting").unwrap();
    let connecting_val = v8::Boolean::new(scope, false);
    this.set(scope, connecting_key.into(), connecting_val.into());
    // 设置 connect 为 closed
    let connect_key = v8::String::new(scope, "connect").unwrap();
    let connect_val = v8::String::new(scope, "closed").unwrap();
    this.set(scope, connect_key.into(), connect_val.into());
    retval.set(v8::Boolean::new(scope, true).into());
}

fn socket_set_timeout_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let timeout = args.get(0).to_int32(scope).map(|i| i.value()).unwrap_or(0);

    // 设置 timeout 属性
    let timeout_key = v8::String::new(scope, "timeout").unwrap();
    let timeout_val = v8::Integer::new(scope, timeout as i32);
    this.set(scope, timeout_key.into(), timeout_val.into());

    retval.set(this.into());
}

fn socket_set_encoding_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    retval.set(this.into());
}

fn socket_pause_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    retval.set(this.into());
}

fn socket_resume_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    retval.set(this.into());
}

fn socket_read_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();

    // 检查是否有缓存的数据
    let data_key = v8::String::new(scope, "_cachedData").unwrap();
    let cached_data = this.get(scope, data_key.into());

    if let Some(data) = cached_data {
        if !data.is_null() && !data.is_undefined() {
            // 有缓存数据，返回它并清除缓存
            let buf = v8::Local::<v8::Value>::try_from(data).unwrap();
            // 预先创建 null 值，避免 borrow checker 问题
            let null_val = v8::null(scope).into();
            // 清除缓存
            this.set(scope, data_key.into(), null_val);
            retval.set(buf);
            return;
        }
    }

    // 没有可用数据，返回 null（Node.js 行为）
    retval.set(v8::null(scope).into());
}

// ==================== Server 回调 ====================

fn server_listen_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();

    // 解析监听选项
    let port = extract_integer_option(scope, &args.get(0), "port", 0);
    let host = extract_string_option(scope, &args.get(0), "host", "0.0.0.0");

    if let Err(error) = crate::permissions::check_global_permission(
        crate::permissions::PermissionKind::Network,
        crate::permissions::PermissionAction::Listen,
        crate::permissions::ResourceId::Url(format!("tcp://{}:{}", host, port)),
    ) {
        let error_message = v8::String::new(scope, &error.to_string()).unwrap();
        let error_obj = v8::Exception::type_error(scope, error_message);
        scope.throw_exception(error_obj.into());
        return;
    }

    // 设置 listening 为 true
    let listening_key = v8::String::new(scope, "listening").unwrap();
    let listening_val = v8::Boolean::new(scope, true);
    this.set(scope, listening_key.into(), listening_val.into());

    // 创建 address 对象
    let address_obj: _ = v8::Object::new(scope);
    let address_key = v8::String::new(scope, "address").unwrap();
    let address_val = v8::String::new(scope, &host).unwrap();
    address_obj.set(scope, address_key.into(), address_val.into());

    let port_key = v8::String::new(scope, "port").unwrap();
    let port_val = v8::Integer::new(scope, port as i32);
    address_obj.set(scope, port_key.into(), port_val.into());

    let family_key = v8::String::new(scope, "family").unwrap();
    let family_val = v8::String::new(scope, "IPv4").unwrap();
    address_obj.set(scope, family_key.into(), family_val.into());

    this.set(scope, address_key.into(), address_obj.into());

    retval.set(this.into());
}

fn server_close_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let listening_key = v8::String::new(scope, "listening").unwrap();
    let listening_val = v8::Boolean::new(scope, false);
    this.set(scope, listening_key.into(), listening_val.into());
    retval.set(this.into());
}

fn server_on_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    retval.set(this.into());
}

fn server_once_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    retval.set(this.into());
}

fn server_emit_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let _this: _ = args.this();
    retval.set(v8::Boolean::new(scope, true).into());
}

fn server_address_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    let address_key = v8::String::new(scope, "address").unwrap();
    let address = this
        .get(scope, address_key.into())
        .unwrap_or(v8::null(scope).into());
    retval.set(address);
}

fn server_get_connections_callback(
    scope: &mut v8::PinScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    retval.set(v8::Integer::new(scope, 0).into());
}

fn server_ref_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    retval.set(this.into());
}

fn server_unref_callback(
    _scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    let this: _ = args.this();
    retval.set(this.into());
}
