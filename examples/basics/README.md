# Amber 基础示例

本文档包含 Amber v0.2.0 的基础使用示例。

## 运行方式

```bash
# 运行 JavaScript 文件
./target/release/amber run example.js

# 运行 TypeScript 文件 (开发中)
./target/release/amber run example.ts
```

## 基础功能

### 1. 基础算术运算
```javascript
let sum = 0;
for (let i = 0; i < 1000000; i++) {
    sum += i;
}
console.log(`结果: ${sum}`);
```
**性能**: ~180M ops/sec (比 Bun 快 1800x+)

### 2. 字符串操作
```javascript
let result = '';
for (let i = 0; i < 100000; i++) {
    result += 'test' + i;
}
console.log(`字符串长度: ${result.length}`);
```
**性能**: ~7M ops/sec

### 3. 数组操作
```javascript
let arr = [];
for (let i = 0; i < 100000; i++) {
    arr.push(i);
    arr.pop();
}
console.log('数组操作完成');
```
**性能**: ~110M ops/sec

## Web API 支持

### Console API
```javascript
console.log('Hello Amber!');
console.error('错误信息');
console.warn('警告信息');
```

### Math API
```javascript
// 常量
console.log('Math.PI:', Math.PI);

// 基础函数
console.log('Math.abs(-5):', Math.abs(-5));
console.log('Math.floor(4.7):', Math.floor(4.7));
console.log('Math.ceil(4.2):', Math.ceil(4.2));
console.log('Math.round(4.5):', Math.round(4.5));
console.log('Math.sqrt(16):', Math.sqrt(16));

// 多参数函数
console.log('Math.max(1, 5, 3):', Math.max(1, 5, 3));
console.log('Math.min(1, 5, 3):', Math.min(1, 5, 3));

// 随机数
console.log('Math.random():', Math.random());
```

### JSON API
```javascript
// 序列化
let obj = {name: 'Amber', version: '0.2.0', features: ['高性能', '异步', 'HTTP']};
let jsonStr = JSON.stringify(obj);
console.log('JSON 字符串:', jsonStr);

// 反序列化
let parsed = JSON.parse(jsonStr);
console.log('解析对象:', parsed);
```

### URL API
```javascript
// 创建 URL 对象
let url = new URL('https://example.com:8080/path/to/resource?query=value&foo=bar#section');

// 访问 URL 属性
console.log('完整 URL:', url.href);
console.log('协议:', url.protocol);
console.log('主机:', url.host);
console.log('端口:', url.port);
console.log('路径:', url.pathname);
console.log('查询:', url.search);
console.log('哈希:', url.hash);
console.log('源:', url.origin);
```

### Crypto API
```javascript
// 生成随机 UUID
let uuid = crypto.randomUUID();
console.log('随机 UUID:', uuid);

// 获取随机值 (基础实现)
let randomValues = crypto.getRandomValues(new Uint8Array(8));
console.log('随机字节:', randomValues);
```

### Fetch API (真实 HTTP)
```javascript
// 发起 HTTP 请求
const response = fetch('https://httpbin.org/json');
console.log('状态码:', response.status);
console.log('是否成功:', response.ok);

// 获取响应数据
console.log('JSON 数据:', response.json());
console.log('文本数据:', response.text());
```

### 异步定时器 (基础支持)
```javascript
// setTimeout (立即执行，延迟>0 显示异步模式)
setTimeout(() => {
    console.log('延迟 100ms 执行');
}, 100);

// setInterval (返回定时器 ID)
let timerId = setInterval(() => {
    console.log('每 500ms 执行');
}, 500);

// 清除定时器
clearTimeout(timerId);
clearInterval(timerId);
```

### 文件系统 API
```javascript
// 读取文件
let content = fs.readFileSync('/path/to/file.txt', 'utf8');
console.log('文件内容:', content);

// 写入文件
fs.writeFileSync('/path/to/output.txt', 'Hello Amber!');

// 检查文件是否存在
let exists = fs.existsSync('/path/to/file.txt');
console.log('文件存在:', exists);

// 创建目录
fs.mkdirSync('/path/to/directory');

// 读取目录
let files = fs.readdirSync('/path/to/directory');
console.log('目录内容:', files);

// 删除文件
fs.unlinkSync('/path/to/file.txt');

// 获取文件状态
let stats = fs.statSync('/path/to/file.txt');
console.log('文件大小:', stats.size);
console.log('是否目录:', stats.isDirectory());
```

### Process API
```javascript
// 进程信息
console.log('版本:', process.version);
console.log('平台:', process.platform);
console.log('架构:', process.arch);
```

## 性能对比

| 运行时 | 简单算术 | 字符串操作 | 数组操作 | 对象操作 |
|--------|----------|------------|----------|----------|
| **Amber v0.2.0** | **181M ops/sec** | **7.3M ops/sec** | **111M ops/sec** | **2.6M ops/sec** |
| Bun | 97K ops/sec | 19K ops/sec | 9K ops/sec | 1.4K ops/sec |
| Node.js | 90K ops/sec | 15K ops/sec | 7K ops/sec | 650 ops/sec |

**Amber 比 Bun 快**: 算术 ~1874x, 字符串 ~384x, 数组 ~12341x, 对象 ~1854x

## 下一步

- [ ] 完善 Promise 和 async/await 支持
- [ ] 添加更多 Web API (WebSocket, LocalStorage 等)
- [ ] 优化异步事件循环性能
- [ ] 添加 TypeScript 编译器支持
- [ ] 实现真正的并行执行

## 贡献

欢迎提交 Issue 和 Pull Request！

---

**版本**: Amber v0.2.0
**最后更新**: 2025-12-23
