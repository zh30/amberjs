// Hono-style router over Node `http`. After `amber add hono @hono/node-server`
// this file can be replaced with the official adapter; this 50-line version
// is the P1 acceptance path that does not require npm.
const http = require('http');

type Handler = (req: { method: string; url: string; body: string }, res: {
  writeHead: (code: number, headers?: Record<string, string>) => unknown;
  end: (body?: string) => unknown;
}) => void;

const routes = new Map<string, Handler>();

function on(method: string, path: string, handler: Handler) {
  routes.set(`${method.toUpperCase()} ${path}`, handler);
}

on('GET', '/', (_req, res) => {
  res.writeHead(200, { 'Content-Type': 'text/plain; charset=utf-8' });
  res.end('hono-hello\n');
});

on('POST', '/echo', (req, res) => {
  res.writeHead(200, { 'Content-Type': 'text/plain; charset=utf-8' });
  res.end(req.body || '');
});

const port = Number(process.env.PORT || 3000);
const host = process.env.HOST || '127.0.0.1';

const server = http.createServer((req: any, res: any) => {
  const url = String(req.url || '/');
  const path = url.split('?')[0];
  const method = String(req.method || 'GET').toUpperCase();
  const handler = routes.get(`${method} ${path}`);
  if (!handler) {
    res.writeHead(404, { 'Content-Type': 'text/plain' });
    res.end('not found\n');
    return;
  }

  const chunks: string[] = [];
  req.on('data', (chunk: string) => {
    chunks.push(String(chunk));
  });
  req.on('end', () => {
    handler({ method, url: path, body: chunks.join('') }, res);
  });
});

server.listen(port, host, () => {
  console.log(`hono_hello listening on http://${host}:${port}/`);
});
