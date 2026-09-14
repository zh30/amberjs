// Node.js dns模块实现 - v0.3.67
/// DNS 查询 API - 支持 lookup 和 resolve
use anyhow::Result;
use rusty_v8 as v8;

use crate::permissions::{check_global_permission, PermissionAction, PermissionKind, ResourceId};

fn check_dns_network_permission(hostname: &str) -> Result<(), String> {
    check_global_permission(
        PermissionKind::Network,
        PermissionAction::Connect,
        ResourceId::Url(format!("dns://{hostname}")),
    )
    .map_err(|error| error.to_string())
}

fn throw_dns_permission_error(scope: &mut v8::PinScope, message: &str) {
    let error_message = v8::String::new(scope, message).unwrap();
    let error = v8::Exception::type_error(scope, error_message);
    scope.throw_exception(error);
}

/// DNS 记录类型
#[derive(Debug, Clone, Copy)]
pub enum DnsRecordType {
    A,     // IPv4 address
    AAAA,  // IPv6 address
    CNAME, // Canonical name
    MX,    // Mail exchange
    NS,    // Name server
    TXT,   // Text record
    SOA,   // Start of authority
    SRV,   // Service record
    PTR,   // Pointer record
}

/// 设置dns API到全局作用域
pub fn setup_dns_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let dns_obj = v8::Object::new(scope);

    // lookup - 查找主机名的IP地址
    let lookup_func = v8::FunctionTemplate::new(scope, dns_lookup_callback);
    let lookup_instance = lookup_func.get_function(scope).unwrap();
    let lookup_key = v8::String::new(scope, "lookup").unwrap();
    dns_obj.set(scope, lookup_key.into(), lookup_instance.into());

    // resolve4 - 解析 IPv4 地址
    let resolve4_func = v8::FunctionTemplate::new(scope, dns_resolve4_callback);
    let resolve4_instance = resolve4_func.get_function(scope).unwrap();
    let resolve4_key = v8::String::new(scope, "resolve4").unwrap();
    dns_obj.set(scope, resolve4_key.into(), resolve4_instance.into());

    // resolve6 - 解析 IPv6 地址
    let resolve6_func = v8::FunctionTemplate::new(scope, dns_resolve6_callback);
    let resolve6_instance = resolve6_func.get_function(scope).unwrap();
    let resolve6_key = v8::String::new(scope, "resolve6").unwrap();
    dns_obj.set(scope, resolve6_key.into(), resolve6_instance.into());

    // resolve - 通用解析
    let resolve_func = v8::FunctionTemplate::new(scope, dns_resolve_callback);
    let resolve_instance = resolve_func.get_function(scope).unwrap();
    let resolve_key = v8::String::new(scope, "resolve").unwrap();
    dns_obj.set(scope, resolve_key.into(), resolve_instance.into());

    // reverse - 反向查找
    let reverse_func = v8::FunctionTemplate::new(scope, dns_reverse_callback);
    let reverse_instance = reverse_func.get_function(scope).unwrap();
    let reverse_key = v8::String::new(scope, "reverse").unwrap();
    dns_obj.set(scope, reverse_key.into(), reverse_instance.into());

    // 设置到全局
    let global = context.global(scope);
    let dns_key = v8::String::new(scope, "dns").unwrap();
    global.set(scope, dns_key.into(), dns_obj.into());

    Ok(())
}

/// dns.lookup(hostname, options, callback) 回调
fn dns_lookup_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    let hostname = args.get(0);
    let arg1 = args.get(1);
    let (options, callback) = if arg1.is_function() {
        (v8::undefined(scope).into(), arg1)
    } else {
        (arg1, args.get(2))
    };

    // 获取主机名
    let hostname_str = if hostname.is_string() {
        hostname
            .to_string(scope)
            .unwrap()
            .to_rust_string_lossy(scope)
    } else {
        return;
    };

    if let Err(error) = check_dns_network_permission(&hostname_str) {
        if callback.is_function() {
            if let Ok(callback_fn) = v8::Local::<v8::Function>::try_from(callback) {
                let undefined = v8::undefined(scope);
                let err_msg = v8::String::new(scope, &error).unwrap();
                callback_fn.call(scope, undefined.into(), &[err_msg.into()]);
            }
            return;
        }
        throw_dns_permission_error(scope, &error);
        return;
    }

    // 解析选项
    let family = extract_dns_option(scope, &options, "family", 4);

    // 检查回调函数
    if !callback.is_function() {
        return;
    }

    // 由于 V8 回调限制，我们在回调中执行同步解析
    let result = perform_dns_lookup(&hostname_str, family);

    let callback_fn = v8::Local::<v8::Function>::try_from(callback).unwrap();

    match result {
        Ok(addresses) => {
            // 预先创建所有 V8 值
            let undefined = v8::undefined(scope);
            let null_val = v8::null(scope);

            // 成功，创建结果参数
            let result_arr = v8::Array::new(scope, addresses.len() as i32);

            for (i, addr) in addresses.iter().enumerate() {
                let addr_obj = v8::Object::new(scope);
                let addr_key = v8::String::new(scope, "address").unwrap();
                let family_key = v8::String::new(scope, "family").unwrap();

                let addr_val = v8::String::new(scope, addr).unwrap();
                let family_val = v8::Integer::new(scope, family as i32);

                addr_obj.set(scope, addr_key.into(), addr_val.into());
                addr_obj.set(scope, family_key.into(), family_val.into());

                result_arr.set_index(scope, i as u32, addr_obj.into());
            }

            // 调用回调
            callback_fn.call(
                scope,
                undefined.into(),
                &[null_val.into(), result_arr.into()],
            );
        }
        Err(err) => {
            // 错误，创建错误对象
            let err_obj = v8::Object::new(scope);
            let code_key = v8::String::new(scope, "code").unwrap();
            let message_key = v8::String::new(scope, "message").unwrap();

            let code_val = v8::String::new(scope, "ENOTFOUND").unwrap();
            let message_val = v8::String::new(scope, &err).unwrap();

            err_obj.set(scope, code_key.into(), code_val.into());
            err_obj.set(scope, message_key.into(), message_val.into());

            // 调用回调
            let undefined = v8::undefined(scope);
            callback_fn.call(scope, undefined.into(), &[err_obj.into()]);
        }
    }
}

/// dns.resolve4(hostname, callback) 回调
fn dns_resolve4_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    let hostname = args.get(0);
    let callback = args.get(1);

    let hostname_str = if hostname.is_string() {
        hostname
            .to_string(scope)
            .unwrap()
            .to_rust_string_lossy(scope)
    } else {
        return;
    };

    if !callback.is_function() {
        return;
    }

    if let Err(error) = check_dns_network_permission(&hostname_str) {
        let callback_fn = v8::Local::<v8::Function>::try_from(callback).unwrap();
        let undefined = v8::undefined(scope);
        let err_msg = v8::String::new(scope, &error).unwrap();
        callback_fn.call(scope, undefined.into(), &[err_msg.into()]);
        return;
    }

    let result = perform_dns_lookup(&hostname_str, 4);
    let callback_fn = v8::Local::<v8::Function>::try_from(callback).unwrap();

    let undefined = v8::undefined(scope);
    let null_val = v8::null(scope);

    match result {
        Ok(addresses) => {
            let result_arr = v8::Array::new(scope, addresses.len() as i32);
            for (i, addr) in addresses.iter().enumerate() {
                let addr_val = v8::String::new(scope, addr).unwrap();
                result_arr.set_index(scope, i as u32, addr_val.into());
            }
            callback_fn.call(
                scope,
                undefined.into(),
                &[null_val.into(), result_arr.into()],
            );
        }
        Err(err) => {
            let err_msg = v8::String::new(scope, &err).unwrap();
            callback_fn.call(scope, undefined.into(), &[err_msg.into()]);
        }
    }
}

/// dns.resolve6(hostname, callback) 回调
fn dns_resolve6_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    let hostname = args.get(0);
    let callback = args.get(1);

    let hostname_str = if hostname.is_string() {
        hostname
            .to_string(scope)
            .unwrap()
            .to_rust_string_lossy(scope)
    } else {
        return;
    };

    if !callback.is_function() {
        return;
    }

    if let Err(error) = check_dns_network_permission(&hostname_str) {
        let callback_fn = v8::Local::<v8::Function>::try_from(callback).unwrap();
        let undefined = v8::undefined(scope);
        let err_msg = v8::String::new(scope, &error).unwrap();
        callback_fn.call(scope, undefined.into(), &[err_msg.into()]);
        return;
    }

    let result = perform_dns_lookup(&hostname_str, 6);
    let callback_fn = v8::Local::<v8::Function>::try_from(callback).unwrap();

    let undefined = v8::undefined(scope);
    let null_val = v8::null(scope);

    match result {
        Ok(addresses) => {
            let result_arr = v8::Array::new(scope, addresses.len() as i32);
            for (i, addr) in addresses.iter().enumerate() {
                let addr_val = v8::String::new(scope, addr).unwrap();
                result_arr.set_index(scope, i as u32, addr_val.into());
            }
            callback_fn.call(
                scope,
                undefined.into(),
                &[null_val.into(), result_arr.into()],
            );
        }
        Err(err) => {
            let err_msg = v8::String::new(scope, &err).unwrap();
            callback_fn.call(scope, undefined.into(), &[err_msg.into()]);
        }
    }
}

/// dns.resolve(hostname, rrtype, callback) 回调
fn dns_resolve_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    let hostname = args.get(0);
    let rrtype = args.get(1);
    let callback = args.get(2);

    let hostname_str = if hostname.is_string() {
        hostname
            .to_string(scope)
            .unwrap()
            .to_rust_string_lossy(scope)
    } else {
        return;
    };

    // 解析记录类型
    let record_type = if rrtype.is_number() {
        let fam = rrtype.to_int32(scope).unwrap().value();
        match fam {
            1 => DnsRecordType::A,
            28 => DnsRecordType::AAAA,
            _ => DnsRecordType::A,
        }
    } else if rrtype.is_string() {
        let rt = rrtype.to_string(scope).unwrap().to_rust_string_lossy(scope);
        match rt.to_lowercase().as_str() {
            "a" | "ipv4" => DnsRecordType::A,
            "aaaa" | "ipv6" => DnsRecordType::AAAA,
            "cname" => DnsRecordType::CNAME,
            "mx" => DnsRecordType::MX,
            "ns" => DnsRecordType::NS,
            "txt" => DnsRecordType::TXT,
            _ => DnsRecordType::A,
        }
    } else {
        DnsRecordType::A
    };

    if !callback.is_function() {
        return;
    }

    if let Err(error) = check_dns_network_permission(&hostname_str) {
        let callback_fn = v8::Local::<v8::Function>::try_from(callback).unwrap();
        let undefined = v8::undefined(scope);
        let err_msg = v8::String::new(scope, &error).unwrap();
        callback_fn.call(scope, undefined.into(), &[err_msg.into()]);
        return;
    }

    let family = match record_type {
        DnsRecordType::A
        | DnsRecordType::CNAME
        | DnsRecordType::MX
        | DnsRecordType::NS
        | DnsRecordType::TXT => 4,
        DnsRecordType::AAAA => 6,
        _ => 4,
    };

    let result = perform_dns_lookup(&hostname_str, family);
    let callback_fn = v8::Local::<v8::Function>::try_from(callback).unwrap();

    let undefined = v8::undefined(scope);
    let null_val = v8::null(scope);

    match result {
        Ok(addresses) => {
            let result_arr = v8::Array::new(scope, addresses.len() as i32);
            for (i, addr) in addresses.iter().enumerate() {
                let addr_val = v8::String::new(scope, addr).unwrap();
                result_arr.set_index(scope, i as u32, addr_val.into());
            }
            callback_fn.call(
                scope,
                undefined.into(),
                &[null_val.into(), result_arr.into()],
            );
        }
        Err(err) => {
            let err_msg = v8::String::new(scope, &err).unwrap();
            callback_fn.call(scope, undefined.into(), &[err_msg.into()]);
        }
    }
}

/// dns.reverse(ip, callback) 回调
fn dns_reverse_callback(
    scope: &mut v8::PinScope,
    args: v8::FunctionCallbackArguments,
    mut _retval: v8::ReturnValue,
) {
    let ip = args.get(0);
    let callback = args.get(1);

    let ip_str = if ip.is_string() {
        ip.to_string(scope).unwrap().to_rust_string_lossy(scope)
    } else {
        return;
    };

    if !callback.is_function() {
        return;
    }

    // 反向 DNS 查询（模拟）
    let result = perform_dns_reverse(&ip_str);
    let callback_fn = v8::Local::<v8::Function>::try_from(callback).unwrap();

    let undefined = v8::undefined(scope);
    let null_val = v8::null(scope);

    match result {
        Ok(hostnames) => {
            let result_arr = v8::Array::new(scope, hostnames.len() as i32);
            for (i, hostname) in hostnames.iter().enumerate() {
                let hostname_val = v8::String::new(scope, hostname).unwrap();
                result_arr.set_index(scope, i as u32, hostname_val.into());
            }
            callback_fn.call(
                scope,
                undefined.into(),
                &[null_val.into(), result_arr.into()],
            );
        }
        Err(err) => {
            let err_msg = v8::String::new(scope, &err).unwrap();
            callback_fn.call(scope, undefined.into(), &[err_msg.into()]);
        }
    }
}

/// 执行 DNS 查找
fn perform_dns_lookup(hostname: &str, family: i32) -> Result<Vec<String>, String> {
    // 处理 localhost
    if hostname == "localhost" || hostname == "127.0.0.1" {
        return Ok(vec!["127.0.0.1".to_string()]);
    }

    // 使用系统 DNS 解析
    let port = 80;
    let addr_format = format!("{}:{}", hostname, port);

    match std::net::ToSocketAddrs::to_socket_addrs(&addr_format) {
        Ok(addrs) => {
            let addrs_vec: Vec<_> = addrs.collect();
            let mut results = Vec::new();
            for addr in &addrs_vec {
                if family == 4 && addr.is_ipv4() {
                    results.push(addr.to_string());
                } else if family == 6 && addr.is_ipv6() {
                    results.push(addr.to_string());
                } else if family == 0 {
                    results.push(addr.to_string());
                }
            }
            if results.is_empty() {
                for addr in &addrs_vec {
                    results.push(addr.to_string());
                }
            }
            Ok(results)
        }
        Err(err) => Err(format!("ENOTFOUND getaddrinfo {}", err)),
    }
}

/// 执行 DNS 反向查找
fn perform_dns_reverse(ip: &str) -> Result<Vec<String>, String> {
    if ip == "127.0.0.1" || ip == "::1" {
        return Ok(vec!["localhost".to_string()]);
    }

    // Do not invent PTR records. Fail closed until a real reverse lookup is wired.
    Err(format!("ENOTFOUND Cannot reverse resolve: {}", ip))
}

/// 从选项对象中提取 DNS 选项
fn extract_dns_option(
    scope: &mut v8::PinScope,
    options: &v8::Local<v8::Value>,
    key: &str,
    default: i32,
) -> i32 {
    if options.is_undefined() || options.is_null() {
        return default;
    }

    if let Ok(obj) = v8::Local::<v8::Object>::try_from(*options) {
        let key_str = v8::String::new(scope, key).unwrap();
        if let Some(val) = obj.get(scope, key_str.into()) {
            if val.is_number() {
                return val.to_int32(scope).unwrap().value() as i32;
            }
        }
    }

    default
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dns_lookup_localhost() {
        let result = perform_dns_lookup("localhost", 4);
        assert!(result.is_ok());
        let addrs = result.unwrap();
        assert!(!addrs.is_empty());
        assert!(addrs.contains(&"127.0.0.1".to_string()));
    }

    #[test]
    fn test_dns_reverse_localhost() {
        let result = perform_dns_reverse("127.0.0.1");
        assert!(result.is_ok());
        let hostnames = result.unwrap();
        assert!(hostnames.contains(&"localhost".to_string()));
    }
}
