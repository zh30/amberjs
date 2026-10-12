---
title: "Server 模式"
nav: "amber serve"
subtitle: "通过 HTTP/1.1 提供 fetch 风格的处理函数，可选 rustls TLS。"
status: "stable"
id: "server-mode"
---

普通 `amber serve` 是 **Stable**（G41），`amber serve --https` 是 **Stable**（G7）。契约分别见 [`SERVE_HTTP_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/SERVE_HTTP_CONTRACT.md) 和 [`SERVE_HTTPS_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/SERVE_HTTPS_CONTRACT.md)。

## 处理函数

导出一个函数、一个带 `fetch` 方法的对象，或一个 `default` 导出。处理函数接收 `Request` 并返回 `Response`。

```js
// server.js
module.exports = {
  fetch(request) {
    const url = new URL(request.url);
    if (url.pathname === "/health") {
      return Response.json({ ok: true });
    }
    return new Response(`Hello from ${url.pathname}`);
  },
};
```

```bash
amber serve --port 3000 server.js
curl http://localhost:3000/health
```

| 参数 | 默认值 | 含义 |
| :--- | :--- | :--- |
| `--port`、`-p` | `3000` | 端口。`0` 会选择空闲端口并打印出来。 |
| `--host` | `localhost` | 绑定地址 |
| `--https` | 关闭 | 启用 TLS |
| `--cert`、`--key` | — | 未加密的 PEM 文件，`--https` 时必填 |

不传脚本时，`amber serve` 返回一个 JSON 健康检查文档。

## HTTPS

```bash
amber serve --https --cert cert.pem --key key.pem --port 8443 server.js
```

证书文件中叶子证书要放在最前面。私钥可以是 PKCS#8、PKCS#1 RSA 或 SEC1。证书或私钥缺失、无法读取时，会在监听之前以退出码 2 退出。

## 限制

两种模式共享相同的帧限制：

- 每个连接一个 HTTP/1.1 请求，不支持 keep-alive。
- 请求头最多 16 KiB、64 个字段，请求体最多 1 MiB，读写超时 10 秒。
- 不支持 chunked 请求体、流式响应、HTTP/2、WebSocket 升级和静态文件服务。

`amber serve` 是处理函数桥接，不是通用 Web 服务器。生产流量请在前面加一层反向代理。

## Node `http` 服务器

如果更习惯 `require('http').createServer`，窄范围的 Node `http` 服务器是另一份 Stable 契约（tentative G37）。它支持 keep-alive，但每个响应会缓冲到 `end()` 才发送。`http.request` 客户端、`Agent`、`https` 和 `http2` 不受支持。详见 [Node.js 兼容性](/docs/node-compat)。
