// Node.js dns模块实现 - v0.3.67
/// DNS 查询 API - 支持 lookup / resolve / reverse / getServers
use anyhow::Result;
use rusty_v8 as v8;
use std::net::IpAddr;

/// Real PTR via OS `getnameinfo(NI_NAMEREQD)`. Never echoes the IP as a hostname.
fn reverse_lookup(ip: &str) -> Result<Vec<String>, String> {
    let addr: IpAddr = ip
        .parse()
        .map_err(|_| format!("Error: dns.reverse {} - invalid IP address", ip))?;
    reverse_lookup_addr(addr, ip)
}

#[cfg(unix)]
fn reverse_lookup_addr(addr: IpAddr, ip: &str) -> Result<Vec<String>, String> {
    use std::ffi::CStr;

    let mut hostbuf = [0i8; libc::NI_MAXHOST as usize];
    let mut servbuf = [0i8; 32];

    let rc = match addr {
        IpAddr::V4(v4) => {
            let mut sa: libc::sockaddr_in = unsafe { std::mem::zeroed() };
            sa.sin_family = libc::AF_INET as _;
            sa.sin_port = 0;
            sa.sin_addr = libc::in_addr {
                s_addr: u32::from(v4).to_be(),
            };
            unsafe {
                libc::getnameinfo(
                    &sa as *const _ as *const libc::sockaddr,
                    std::mem::size_of_val(&sa) as libc::socklen_t,
                    hostbuf.as_mut_ptr(),
                    hostbuf.len() as libc::socklen_t,
                    servbuf.as_mut_ptr(),
                    servbuf.len() as libc::socklen_t,
                    libc::NI_NAMEREQD,
                )
            }
        }
        IpAddr::V6(v6) => {
            let mut sa: libc::sockaddr_in6 = unsafe { std::mem::zeroed() };
            sa.sin6_family = libc::AF_INET6 as _;
            sa.sin6_port = 0;
            sa.sin6_flowinfo = 0;
            sa.sin6_addr = libc::in6_addr {
                s6_addr: v6.octets(),
            };
            sa.sin6_scope_id = 0;
            unsafe {
                libc::getnameinfo(
                    &sa as *const _ as *const libc::sockaddr,
                    std::mem::size_of_val(&sa) as libc::socklen_t,
                    hostbuf.as_mut_ptr(),
                    hostbuf.len() as libc::socklen_t,
                    servbuf.as_mut_ptr(),
                    servbuf.len() as libc::socklen_t,
                    libc::NI_NAMEREQD,
                )
            }
        }
    };

    if rc != 0 {
        let detail = unsafe {
            let ptr = libc::gai_strerror(rc);
            if ptr.is_null() {
                format!("getnameinfo failed ({rc})")
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        };
        return Err(format!("Error: dns.reverse {} - {}", ip, detail));
    }

    let hostname = unsafe { CStr::from_ptr(hostbuf.as_ptr()) }
        .to_string_lossy()
        .into_owned();
    // Belt-and-suspenders: refuse IP-echo (the old Preview stub).
    if hostname.is_empty() || hostname == ip {
        return Err(format!("Error: dns.reverse {} - no PTR hostname", ip));
    }
    Ok(vec![hostname])
}

/// windows-sys 0.52 WinSock does not declare `gai_strerrorA`. The export is in
/// ws2_32 and formats the code `getnameinfo` returns (same role as `libc::gai_strerror`).
#[cfg(windows)]
#[link(name = "ws2_32")]
extern "system" {
    fn gai_strerrorA(ecode: i32) -> *const i8;
}

#[cfg(windows)]
fn reverse_lookup_addr(addr: IpAddr, ip: &str) -> Result<Vec<String>, String> {
    use std::ffi::CStr;
    use windows_sys::Win32::Networking::WinSock::{
        getnameinfo, AF_INET, AF_INET6, NI_MAXHOST, NI_NAMEREQD, SOCKADDR, SOCKADDR_IN,
        SOCKADDR_IN6,
    };

    let mut hostbuf = [0u8; NI_MAXHOST as usize];
    let mut servbuf = [0u8; 32];

    let rc = match addr {
        IpAddr::V4(v4) => {
            let mut sa: SOCKADDR_IN = unsafe { std::mem::zeroed() };
            sa.sin_family = AF_INET;
            sa.sin_port = 0;
            unsafe {
                sa.sin_addr.S_un.S_addr = u32::from(v4).to_be();
                getnameinfo(
                    &sa as *const _ as *const SOCKADDR,
                    std::mem::size_of_val(&sa) as i32,
                    hostbuf.as_mut_ptr(),
                    hostbuf.len() as u32,
                    servbuf.as_mut_ptr(),
                    servbuf.len() as u32,
                    NI_NAMEREQD as i32,
                )
            }
        }
        IpAddr::V6(v6) => {
            let mut sa: SOCKADDR_IN6 = unsafe { std::mem::zeroed() };
            sa.sin6_family = AF_INET6;
            sa.sin6_port = 0;
            sa.sin6_flowinfo = 0;
            unsafe {
                sa.sin6_addr.u.Byte = v6.octets();
                getnameinfo(
                    &sa as *const _ as *const SOCKADDR,
                    std::mem::size_of_val(&sa) as i32,
                    hostbuf.as_mut_ptr(),
                    hostbuf.len() as u32,
                    servbuf.as_mut_ptr(),
                    servbuf.len() as u32,
                    NI_NAMEREQD as i32,
                )
            }
        }
    };

    if rc != 0 {
        let detail = unsafe {
            let ptr = gai_strerrorA(rc);
            if ptr.is_null() {
                format!("getnameinfo failed ({rc})")
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        };
        return Err(format!("Error: dns.reverse {} - {}", ip, detail));
    }

    let end = hostbuf
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(hostbuf.len());
    let hostname = String::from_utf8_lossy(&hostbuf[..end]).into_owned();
    if hostname.is_empty() || hostname == ip {
        return Err(format!("Error: dns.reverse {} - no PTR hostname", ip));
    }
    Ok(vec![hostname])
}

/// System resolver list. Never invents `8.8.8.8`.
fn read_dns_servers() -> Vec<String> {
    #[cfg(unix)]
    {
        read_resolv_conf_nameservers("/etc/resolv.conf")
    }
    #[cfg(windows)]
    {
        read_windows_dns_servers()
    }
}

#[cfg(unix)]
fn read_resolv_conf_nameservers(path: &str) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut servers = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        if parts.next() != Some("nameserver") {
            continue;
        }
        if let Some(server) = parts.next() {
            if !server.is_empty() {
                servers.push(server.to_string());
            }
        }
    }
    servers
}

#[cfg(windows)]
fn read_windows_dns_servers() -> Vec<String> {
    use windows_sys::Win32::Foundation::ERROR_BUFFER_OVERFLOW;
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetNetworkParams, FIXED_INFO_W2KSP1, IP_ADDR_STRING,
    };

    let mut size: u32 = 0;
    let first = unsafe { GetNetworkParams(std::ptr::null_mut(), &mut size) };
    if first != ERROR_BUFFER_OVERFLOW && size == 0 {
        return Vec::new();
    }
    let mut buf = vec![0u8; size as usize];
    let rc = unsafe { GetNetworkParams(buf.as_mut_ptr() as *mut FIXED_INFO_W2KSP1, &mut size) };
    if rc != 0 {
        return Vec::new();
    }
    let info = unsafe { &*(buf.as_ptr() as *const FIXED_INFO_W2KSP1) };
    let mut servers = Vec::new();
    // Head is the embedded list node. `Next` is `*mut` and null after the last server,
    // which does not unify with `as *const _` under windows-sys 0.52.
    let mut node: *const IP_ADDR_STRING = &info.DnsServerList;
    while !node.is_null() {
        let entry = unsafe { &*node };
        let bytes = entry.IpAddress.String;
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        let s = String::from_utf8_lossy(&bytes[..end]).trim().to_string();
        if !s.is_empty() {
            servers.push(s);
        }
        node = entry.Next.cast_const();
    }
    servers
}

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

    // dns.reverse(ip) - real PTR via OS getnameinfo(NI_NAMEREQD)
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

            match reverse_lookup(&ip) {
                Ok(hostnames) => {
                    let arr = v8::Array::new(_scope, hostnames.len() as i32);
                    for (i, name) in hostnames.iter().enumerate() {
                        let name_str = v8::String::new(_scope, name).unwrap();
                        arr.set_index(_scope, i as u32, name_str.into());
                    }
                    retval.set(arr.into());
                }
                Err(error_msg) => {
                    retval.set(v8::String::new(_scope, &error_msg).unwrap().into());
                }
            }
        },
    )
    .get_function(scope)
    .unwrap();
    dns_obj.set(scope, reverse_key.into(), reverse_instance.into());

    // dns.getServers() - system resolver list (resolv.conf / GetNetworkParams)
    let get_servers_key = v8::String::new(scope, "getServers").unwrap();
    let get_servers_instance = v8::FunctionTemplate::new(
        scope,
        |_scope: &mut v8::PinScope,
         _args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let servers = read_dns_servers();
            let arr = v8::Array::new(_scope, servers.len() as i32);
            for (i, server) in servers.iter().enumerate() {
                let s = v8::String::new(_scope, server).unwrap();
                arr.set_index(_scope, i as u32, s.into());
            }
            retval.set(arr.into());
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
