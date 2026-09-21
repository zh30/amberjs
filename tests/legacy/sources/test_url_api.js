//! URL API 测试用例
//! 测试 URL Web API 的完整功能

console.log("=== Amber URL API 测试套件 ===\n");

// 测试 1: URL 对象创建
console.log("测试 1: URL 对象创建");
try {
    const url = new URL('https://example.com:8080/path/to/resource?query=value&foo=bar#hash');
    console.log("✓ URL 对象创建成功");
    console.log("  href:", url.href);
    console.log("  protocol:", url.protocol);
    console.log("  host:", url.host);
    console.log("  pathname:", url.pathname);
    console.log("  search:", url.search);
    console.log("  hash:", url.hash);
    console.log("  port:", url.port);
    console.log("  hostname:", url.hostname);
} catch (e) {
    console.log("✗ 错误:", e.message);
}

console.log();

// 测试 2: 相对 URL
console.log("测试 2: 相对 URL");
try {
    const url = new URL('/path/to/page?param=value', 'https://base.example.com');
    console.log("✓ 相对 URL 解析成功");
    console.log("  href:", url.href);
    console.log("  origin:", url.origin);
    console.log("  pathname:", url.pathname);
    console.log("  search:", url.search);
} catch (e) {
    console.log("✗ 错误:", e.message);
}

console.log();

// 测试 3: URL 属性修改
console.log("测试 3: URL 属性修改");
try {
    const url = new URL('https://example.com/path');
    console.log("  修改前 href:", url.href);
    url.pathname = '/newpath';
    url.searchParams.append('key', 'value');
    console.log("  修改后 href:", url.href);
    console.log("  ✓ 属性修改成功");
} catch (e) {
    console.log("✗ 错误:", e.message);
}

console.log();

// 测试 4: URLSearchParams
console.log("测试 4: URLSearchParams");
try {
    const url = new URL('https://example.com?a=1&b=2&c=3');
    const params = url.searchParams;
    console.log("✓ URLSearchParams 创建成功");
    console.log("  a:", params.get('a'));
    console.log("  b:", params.get('b'));
    console.log("  所有参数:", params.toString());
} catch (e) {
    console.log("✗ 错误:", e.message);
}

console.log();

// 测试 5: 不同协议
console.log("测试 5: 不同协议");
try {
    const httpUrl = new URL('http://example.com');
    const httpsUrl = new URL('https://example.com');
    const ftpUrl = new URL('ftp://example.com');
    console.log("✓ HTTP URL:", httpUrl.protocol);
    console.log("✓ HTTPS URL:", httpsUrl.protocol);
    console.log("✓ FTP URL:", ftpUrl.protocol);
} catch (e) {
    console.log("✗ 错误:", e.message);
}

console.log("\n=== 测试完成 ===");
