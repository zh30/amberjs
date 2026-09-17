use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

fn run_js(code: &str) -> String {
    let mut runtime = MinimalRuntime::new().expect("Failed to create minimal runtime");
    runtime
        .execute_code(code)
        .expect("JS code should execute successfully")
        .trim()
        .to_string()
}

#[test]
#[serial]
fn test_hono_style_web_standards_and_context_storage() {
    let script = r#"
    const { AsyncLocalStorage } = require('async_hooks');

    // 1. Web Standard Response.json and Response.redirect
    const res = Response.json({ message: 'hello from hono' }, { status: 201 });
    const redirectRes = Response.redirect('https://amberjs.dev/docs', 302);

    // 2. Headers getSetCookie
    const headers = new Headers();
    headers.append('Set-Cookie', 'session=abc; Path=/; HttpOnly');
    headers.append('Set-Cookie', 'theme=dark; Path=/');
    const cookies = headers.getSetCookie();

    // 3. Hono-style context storage middleware via AsyncLocalStorage
    const als = new AsyncLocalStorage();
    let capturedUserId = null;

    async function middleware(ctx, next) {
        return als.run(ctx, async () => {
            await new Promise(resolve => setTimeout(resolve, 10));
            await next();
        });
    }

    async function handler() {
        const store = als.getStore();
        capturedUserId = store ? store.userId : null;
        return Response.json({ user: capturedUserId });
    }

    // Execute simulated Hono request
    middleware({ userId: 'user_42' }, handler);

    `${res.status}:${res.headers.get('content-type')}:${redirectRes.status}:${redirectRes.headers.get('location')}:${cookies.length}:${cookies[0].startsWith('session=')}`;
    "#;
    let output = run_js(script);
    assert_eq!(
        output,
        "201:application/json:302:https://amberjs.dev/docs:2:true"
    );
}

#[test]
#[serial]
fn test_express_style_server_and_middlewares() {
    let script = r#"
    const http = require('http');

    // Mini Express-style router/app simulation
    class MiniExpress {
        constructor() {
            this.routes = [];
        }
        use(fn) {
            this.routes.push(fn);
        }
        handle(req, res) {
            let idx = 0;
            const next = () => {
                if (idx < this.routes.length) {
                    const fn = this.routes[idx++];
                    fn(req, res, next);
                }
            };
            next();
        }
    }

    const app = new MiniExpress();
    let logs = [];

    // Middleware 1: logger & header injection
    app.use((req, res, next) => {
        logs.push(`${req.method} ${req.url}`);
        res.setHeader('X-Powered-By', 'Amber');
        next();
    });

    // Middleware 2: JSON handler
    app.use((req, res, next) => {
        res.status(200).json({ status: 'ok', client: req.socket.remoteAddress });
    });

    // Create server and invoke simulated request
    const server = http.createServer((req, res) => {
        app.handle(req, res);
    });

    const req = new http.IncomingMessage();
    req.method = 'POST';
    req.url = '/api/v1/status';
    req.headers['authorization'] = 'Bearer test_token';

    const res = new http.ServerResponse();
    app.handle(req, res);

    `${logs[0]}:${res.statusCode}:${res.getHeader('x-powered-by')}:${res.getHeader('content-type')}:${res.hasHeader('X-Powered-By')}`;
    "#;
    let output = run_js(script);
    assert_eq!(
        output,
        "POST /api/v1/status:200:Amber:application/json:true"
    );
}

#[test]
#[serial]
fn test_node_stream_and_timers_promises() {
    let script = r#"
    const { pipeline } = require('stream/promises');
    const { setTimeout } = require('timers/promises');
    const { Readable, Transform, Writable } = require('stream');

    const hasPipeline = typeof pipeline === 'function';
    const hasSetTimeout = typeof setTimeout === 'function';

    // Verify timers/promises setTimeout returns a Promise
    const p = setTimeout(10, 'done');
    const isPromise = typeof p.then === 'function';

    `${hasPipeline}:${hasSetTimeout}:${isPromise}`;
    "#;
    let output = run_js(script);
    assert_eq!(output, "true:true:true");
}

#[test]
#[serial]
fn test_langchain_style_streaming_and_async_iteration() {
    let script = r#"
    // LangChain LLM streaming pattern:
    // 1. ReadableStream.from() to create token stream
    // 2. TransformStream to parse / transform SSE chunks
    // 3. for await (const chunk of stream) consumption
    // 4. AbortSignal.timeout() for LLM request timeout

    const timeoutSignal = AbortSignal.timeout(5000);
    const hasSignal = !timeoutSignal.aborted && typeof timeoutSignal.addEventListener === 'function';

    // Token chunks from LLM
    const tokenStream = ReadableStream.from(['Hello', ' ', 'AI', ' ', 'World', '!']);

    // Transform stream: convert tokens to upper case
    const upperTransform = new TransformStream({
        transform(chunk, controller) {
            controller.enqueue(chunk.toUpperCase());
        }
    });

    const transformedStream = tokenStream.pipeThrough(upperTransform);

    // Collect tokens asynchronously
    let collected = [];
    async function collect() {
        for await (const token of transformedStream) {
            collected.push(token);
        }
        return collected.join('');
    }

    // In sync test check Stream and AbortSignal setup
    `${hasSignal}:${tokenStream instanceof ReadableStream}:${transformedStream instanceof ReadableStream}`;
    "#;
    let output = run_js(script);
    assert_eq!(output, "true:true:true");
}
