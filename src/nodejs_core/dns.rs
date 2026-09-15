// Node.js dns模块实现 - v0.3.67
/// DNS 查询 API - 支持 lookup 和 resolve
use anyhow::Result;
use rusty_v8 as v8;

pub fn setup_dns_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Create dns object
    let dns_obj = v8::Object::new(scope);

    // dns.lookup(hostname, [options]) - Look up a hostname
    let lookup_key = v8::String::new(scope, "lookup").unwrap();
    let lookup_instance = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let hostname = args
                .get(0)
                .to_string(_scope)
                .map(|s| s.to_rust_string_lossy(_scope))
                .unwrap_or_default();
            let callback = if args.get(1).is_function() {
                Some(args.get(1))
            } else if args.get(2).is_function() {
                Some(args.get(2))
            } else {
                None
            };

            if hostname.is_empty() {
                let error_msg = "Error: hostname is required";
                if let Some(cb) = callback {
                    if let Ok(func) = v8::Local::<v8::Function>::try_from(cb) {
                        let undefined = v8::undefined(_scope);
                        let err_val = v8::String::new(_scope, error_msg).unwrap();
                        func.call(_scope, undefined.into(), &[err_val.into()]);
                    }
                }
                retval.set(v8::String::new(_scope, error_msg).unwrap().into());
                return;
            }
            if let Err(error) = crate::permissions::check_global_permission(
                crate::permissions::PermissionKind::Network,
                crate::permissions::PermissionAction::Connect,
                crate::permissions::ResourceId::Name(hostname.clone()),
            ) {
                let error_msg = error.to_string();
                if let Some(cb) = callback {
                    if let Ok(func) = v8::Local::<v8::Function>::try_from(cb) {
                        let undefined = v8::undefined(_scope);
                        let err_val = v8::String::new(_scope, &error_msg).unwrap();
                        func.call(_scope, undefined.into(), &[err_val.into()]);
                    }
                }
                retval.set(v8::String::new(_scope, &error_msg).unwrap().into());
                return;
            }

            // Use standard library for DNS lookup
            // Try different formats to handle localhost and regular hostnames
            let result = std::net::ToSocketAddrs::to_socket_addrs(&hostname)
                .or_else(|_| std::net::ToSocketAddrs::to_socket_addrs(&format!("{}:0", hostname)));

            match result {
                Ok(addrs) => {
                    // Extract IP addresses only (without port)
                    let mut addresses: Vec<String> = addrs
                        .map(|addr| {
                            if addr.is_ipv4() {
                                format!("{}", addr.ip())
                            } else {
                                format!("{}", addr.ip())
                            }
                        })
                        .collect();
                    addresses.sort();
                    addresses.dedup();

                    // Return first address as string for compatibility
                    if let Some(ip) = addresses.first() {
                        if let Some(cb) = callback {
                            if let Ok(func) = v8::Local::<v8::Function>::try_from(cb) {
                                let undefined = v8::undefined(_scope);
                                let null_val = v8::null(_scope);
                                let addr_val = v8::String::new(_scope, ip).unwrap();
                                let family_val = v8::Integer::new(_scope, 4);
                                func.call(
                                    _scope,
                                    undefined.into(),
                                    &[null_val.into(), addr_val.into(), family_val.into()],
                                );
                            }
                        }
                        retval.set(v8::String::new(_scope, ip).unwrap().into());
                    } else {
                        if let Some(cb) = callback {
                            if let Ok(func) = v8::Local::<v8::Function>::try_from(cb) {
                                let undefined = v8::undefined(_scope);
                                let err_val =
                                    v8::String::new(_scope, "Error: no addresses").unwrap();
                                func.call(_scope, undefined.into(), &[err_val.into()]);
                            }
                        }
                        retval.set(v8::null(_scope).into());
                    }
                }
                Err(e) => {
                    let error_msg = format!("Error: dns.lookup {} - {}", hostname, e);
                    if let Some(cb) = callback {
                        if let Ok(func) = v8::Local::<v8::Function>::try_from(cb) {
                            let undefined = v8::undefined(_scope);
                            let err_val = v8::String::new(_scope, &error_msg).unwrap();
                            func.call(_scope, undefined.into(), &[err_val.into()]);
                        }
                    }
                    retval.set(v8::String::new(_scope, &error_msg).unwrap().into());
                }
            }
        },
    )
    .get_function(scope)
    .unwrap();
    dns_obj.set(scope, lookup_key.into(), lookup_instance.into());

    // dns.resolve(hostname, [rrtype]) - Resolve a hostname
    let resolve_key = v8::String::new(scope, "resolve").unwrap();
    let resolve_instance = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let hostname = args
                .get(0)
                .to_string(_scope)
                .map(|s| s.to_rust_string_lossy(_scope))
                .unwrap_or_default();
            let _rrtype = args
                .get(1)
                .to_string(_scope)
                .map(|s| s.to_rust_string_lossy(_scope))
                .unwrap_or_else(|| "A".to_string());
            // Note: rrtype parameter is accepted for API compatibility but full record-type
            // specific resolution would require a DNS crate like trust-dns or c-ares

            if hostname.is_empty() {
                retval.set(
                    v8::String::new(_scope, "Error: hostname is required")
                        .unwrap()
                        .into(),
                );
                return;
            }
            if let Err(error) = crate::permissions::check_global_permission(
                crate::permissions::PermissionKind::Network,
                crate::permissions::PermissionAction::Connect,
                crate::permissions::ResourceId::Name(hostname.clone()),
            ) {
                retval.set(v8::String::new(_scope, &error.to_string()).unwrap().into());
                return;
            }

            // Perform DNS lookup based on record type
            // Note: Full DNS resolution with different record types requires a DNS crate
            // For now, use standard library lookup which handles A/AAAA records
            let result = std::net::ToSocketAddrs::to_socket_addrs(&hostname)
                .or_else(|_| std::net::ToSocketAddrs::to_socket_addrs(&format!("{}:0", hostname)));

            match result {
                Ok(addrs) => {
                    // Extract IP addresses only (without port)
                    let addresses: Vec<String> =
                        addrs.map(|addr| format!("{}", addr.ip())).collect();
                    // Create array of addresses
                    let arr = v8::Array::new(_scope, addresses.len() as i32);
                    for (i, addr) in addresses.iter().enumerate() {
                        let addr_str = v8::String::new(_scope, addr).unwrap();
                        arr.set_index(_scope, i as u32, addr_str.into());
                    }
                    retval.set(arr.into());
                }
                Err(e) => {
                    let error_msg = format!("Error: dns.resolve {} - {}", hostname, e);
                    retval.set(v8::String::new(_scope, &error_msg).unwrap().into());
                }
            }
        },
    )
    .get_function(scope)
    .unwrap();
    dns_obj.set(scope, resolve_key.into(), resolve_instance.into());

    // dns.resolve4(hostname) - Resolve IPv4 addresses
    let resolve4_key = v8::String::new(scope, "resolve4").unwrap();
    let resolve4_instance = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let hostname = args
                .get(0)
                .to_string(_scope)
                .map(|s| s.to_rust_string_lossy(_scope))
                .unwrap_or_default();

            if hostname.is_empty() {
                retval.set(
                    v8::String::new(_scope, "Error: hostname is required")
                        .unwrap()
                        .into(),
                );
                return;
            }
            if let Err(error) = crate::permissions::check_global_permission(
                crate::permissions::PermissionKind::Network,
                crate::permissions::PermissionAction::Connect,
                crate::permissions::ResourceId::Name(hostname.clone()),
            ) {
                retval.set(v8::String::new(_scope, &error.to_string()).unwrap().into());
                return;
            }

            let result = std::net::ToSocketAddrs::to_socket_addrs(&hostname)
                .or_else(|_| std::net::ToSocketAddrs::to_socket_addrs(&format!("{}:0", hostname)));

            match result {
                Ok(addrs) => {
                    let v4_addresses: Vec<String> = addrs
                        .filter(|addr| addr.is_ipv4())
                        .map(|addr| format!("{}", addr.ip()))
                        .collect();

                    let arr = v8::Array::new(_scope, v4_addresses.len() as i32);
                    for (i, addr) in v4_addresses.iter().enumerate() {
                        let addr_str = v8::String::new(_scope, addr).unwrap();
                        arr.set_index(_scope, i as u32, addr_str.into());
                    }
                    retval.set(arr.into());
                }
                Err(e) => {
                    let error_msg = format!("Error: dns.resolve4 {} - {}", hostname, e);
                    retval.set(v8::String::new(_scope, &error_msg).unwrap().into());
                }
            }
        },
    )
    .get_function(scope)
    .unwrap();
    dns_obj.set(scope, resolve4_key.into(), resolve4_instance.into());

    // dns.resolve6(hostname) - Resolve IPv6 addresses
    let resolve6_key = v8::String::new(scope, "resolve6").unwrap();
    let resolve6_instance = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let hostname = args
                .get(0)
                .to_string(_scope)
                .map(|s| s.to_rust_string_lossy(_scope))
                .unwrap_or_default();

            if hostname.is_empty() {
                retval.set(
                    v8::String::new(_scope, "Error: hostname is required")
                        .unwrap()
                        .into(),
                );
                return;
            }
            if let Err(error) = crate::permissions::check_global_permission(
                crate::permissions::PermissionKind::Network,
                crate::permissions::PermissionAction::Connect,
                crate::permissions::ResourceId::Name(hostname.clone()),
            ) {
                retval.set(v8::String::new(_scope, &error.to_string()).unwrap().into());
                return;
            }

            let result = std::net::ToSocketAddrs::to_socket_addrs(&hostname)
                .or_else(|_| std::net::ToSocketAddrs::to_socket_addrs(&format!("{}:0", hostname)));

            match result {
                Ok(addrs) => {
                    let v6_addresses: Vec<String> = addrs
                        .filter(|addr| addr.is_ipv6())
                        .map(|addr| format!("{}", addr.ip()))
                        .collect();

                    let arr = v8::Array::new(_scope, v6_addresses.len() as i32);
                    for (i, addr) in v6_addresses.iter().enumerate() {
                        let addr_str = v8::String::new(_scope, addr).unwrap();
                        arr.set_index(_scope, i as u32, addr_str.into());
                    }
                    retval.set(arr.into());
                }
                Err(e) => {
                    let error_msg = format!("Error: dns.resolve6 {} - {}", hostname, e);
                    retval.set(v8::String::new(_scope, &error_msg).unwrap().into());
                }
            }
        },
    )
    .get_function(scope)
    .unwrap();
    dns_obj.set(scope, resolve6_key.into(), resolve6_instance.into());

    // dns.reverse(ip) - PTR record lookup (reverse DNS)
    let reverse_key = v8::String::new(scope, "reverse").unwrap();
    let reverse_instance = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let ip = args
                .get(0)
                .to_string(_scope)
                .map(|s| s.to_rust_string_lossy(_scope))
                .unwrap_or_default();

            if ip.is_empty() {
                retval.set(
                    v8::String::new(_scope, "Error: IP address is required")
                        .unwrap()
                        .into(),
                );
                return;
            }
            if let Err(error) = crate::permissions::check_global_permission(
                crate::permissions::PermissionKind::Network,
                crate::permissions::PermissionAction::Connect,
                crate::permissions::ResourceId::Name(ip.clone()),
            ) {
                retval.set(v8::String::new(_scope, &error.to_string()).unwrap().into());
                return;
            }

            // For PTR records, we return the IP as hostname for compatibility
            // Full PTR lookup would require a DNS resolver crate
            retval.set(v8::String::new(_scope, &ip).unwrap().into());
        },
    )
    .get_function(scope)
    .unwrap();
    dns_obj.set(scope, reverse_key.into(), reverse_instance.into());

    // dns.getServers() - Get DNS servers (mock for compatibility)
    let get_servers_key = v8::String::new(scope, "getServers").unwrap();
    let get_servers_instance = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let servers = v8::Array::new(_scope, 1);
            let dns_server = v8::String::new(_scope, "8.8.8.8").unwrap();
            servers.set_index(_scope, 0, dns_server.into());
            retval.set(servers.into());
        },
    )
    .get_function(scope)
    .unwrap();
    dns_obj.set(scope, get_servers_key.into(), get_servers_instance.into());

    // Set dns as global
    let dns_key = v8::String::new(scope, "dns").unwrap();
    global.set(scope, dns_key.into(), dns_obj.into());

    Ok(())
}
