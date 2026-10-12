---
title: "Server mode"
nav: "amber serve"
subtitle: "Serve a fetch-style handler over HTTP/1.1, with optional rustls TLS."
status: "stable"
id: "server-mode"
---

Plain `amber serve` is **Stable** (G41) and `amber serve --https` is **Stable** (G7). Their contracts are [`SERVE_HTTP_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/SERVE_HTTP_CONTRACT.md) and [`SERVE_HTTPS_CONTRACT.md`](https://github.com/zh30/amberjs/blob/main/docs/SERVE_HTTPS_CONTRACT.md).

## A handler

Export a function, an object with `fetch`, or a `default` export. The handler receives a `Request` and returns a `Response`.

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

| Flag | Default | Meaning |
| :--- | :--- | :--- |
| `--port`, `-p` | `3000` | Port. `0` picks a free port and prints it. |
| `--host` | `localhost` | Bind address |
| `--https` | off | Turn on TLS |
| `--cert`, `--key` | — | Unencrypted PEM files, required with `--https` |

With no script, `amber serve` answers with a JSON health document.

## HTTPS

```bash
amber serve --https --cert cert.pem --key key.pem --port 8443 server.js
```

The certificate file lists the leaf certificate first. Keys may be PKCS#8, PKCS#1 RSA, or SEC1. Missing or unreadable material exits with code 2 before the server listens.

## Limits

Both modes share the same framing limits:

- One HTTP/1.1 request per connection. No keep-alive.
- Headers up to 16 KiB and 64 fields, bodies up to 1 MiB, 10 second read and write timeouts.
- No chunked bodies, streaming responses, HTTP/2, WebSocket upgrades, or static file serving.

`amber serve` is a handler bridge, not a general web server. Put a reverse proxy in front of it for production traffic.

## Node `http` servers

If you prefer `require('http').createServer`, the narrow Node `http` server is a separate Stable contract (tentative G37). It supports keep-alive, but buffers each response until `end()`. The `http.request` client, `Agent`, `https`, and `http2` are not supported. See [Node.js compatibility](/docs/node-compat).
