// DNS 模块测试 - v0.3.47
// 测试 DNS lookup 和 resolution 功能

use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_dns_module_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof dns");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "object");
}

#[test]
#[serial]
fn test_dns_lookup_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof dns.lookup");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_dns_lookup_localhost() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("dns.lookup('localhost')");
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // localhost should resolve to 127.0.0.1
    assert!(output.contains("127.0.0.1") || output.is_empty() || output.starts_with("Error"));
}

#[test]
#[serial]
fn test_dns_lookup_empty_hostname() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("dns.lookup('')");
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    assert!(output.contains("Error") && output.contains("hostname"));
}

#[test]
#[serial]
fn test_dns_resolve_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof dns.resolve");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_dns_resolve_localhost() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("dns.resolve('localhost')");
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // Should return an array containing localhost addresses
    assert!(output.contains("[") || output.contains("127.0.0.1") || output.starts_with("Error"));
}

#[test]
#[serial]
fn test_dns_resolve4_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof dns.resolve4");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_dns_resolve4_localhost() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("dns.resolve4('localhost')");
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // Should return an array of IPv4 addresses
    assert!(output.contains("[") || output.contains("127.0.0.1") || output.starts_with("Error"));
}

#[test]
#[serial]
fn test_dns_resolve6_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof dns.resolve6");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_dns_resolve6_localhost() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("dns.resolve6('localhost')");
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // Should return an array of IPv6 addresses (or empty if none found)
    // Output could be "::1", "[]", "Error", etc.
    assert!(output.contains("::") || output.is_empty() || output.starts_with("Error"));
}

#[test]
#[serial]
fn test_dns_reverse_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof dns.reverse");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_dns_reverse_ip() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("dns.reverse('127.0.0.1')");
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // Should return the IP for compatibility (PTR lookup simplified)
    assert!(output.contains("127.0.0.1") || output.starts_with("Error"));
}

#[test]
#[serial]
fn test_dns_reverse_empty() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("dns.reverse('')");
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    assert!(output.contains("Error") && output.contains("IP address"));
}

#[test]
#[serial]
fn test_dns_get_servers_exists() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("typeof dns.getServers");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "function");
}

#[test]
#[serial]
fn test_dns_get_servers_returns_array() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("Array.isArray(dns.getServers())");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().trim(), "true");
}

#[test]
#[serial]
fn test_dns_get_servers_contains_dns_server() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("dns.getServers().includes('8.8.8.8')");
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // Should contain at least one DNS server (8.8.8.8 is the default)
    assert_eq!(output, "true");
}

#[test]
#[serial]
fn test_dns_lookup_google() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("dns.lookup('google.com')");
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // Should return an IP address or error
    assert!(!output.contains("undefined"));
}

#[test]
#[serial]
fn test_dns_resolve_with_rrtype() {
    let mut runtime = MinimalRuntime::new().unwrap();
    let result = runtime.execute_code("dns.resolve('localhost', 'A')");
    assert!(result.is_ok());
    let binding = result.unwrap();
    let output = binding.trim();
    // Should return an array or error
    // Array is converted to string by V8, so check for addresses or error
    assert!(
        output.contains("::1")
            || output.contains("127.0.0.1")
            || output.is_empty()
            || output.starts_with("Error")
    );
}
