//! Official Model Context Protocol (MCP) implementation for Beejs (`bee:mcp`).
//!
//! Provides first-class support for Anthropic Model Context Protocol (MCP) 2.0 servers
//! and clients in TypeScript and JavaScript.

use anyhow::Result;
use rusty_v8 as v8;
use std::io::{self, BufRead, Write};

pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Setup MCP API in the V8 context
pub fn setup_mcp_api(
    scope: &mut v8::ContextScope<v8::HandleScope>,
    context: &v8::Local<v8::Context>,
) -> Result<()> {
    let global = context.global(scope);

    // Native stdio read_line and write_line callback
    let mcp_native_fn = v8::FunctionTemplate::new(
        scope,
        |scope: &mut v8::PinScope,
         args: v8::FunctionCallbackArguments,
         mut retval: v8::ReturnValue| {
            let action = if args.length() > 0 {
                args.get(0).to_rust_string_lossy(scope)
            } else {
                String::new()
            };

            match action.as_str() {
                "read_line" => {
                    let stdin = io::stdin();
                    let mut reader = stdin.lock();
                    let mut line = String::new();
                    match reader.read_line(&mut line) {
                        Ok(n) if n > 0 => {
                            let str_val = v8::String::new(scope, &line).unwrap();
                            retval.set(str_val.into());
                        }
                        _ => {
                            retval.set(v8::null(scope).into());
                        }
                    }
                }
                "write_line" => {
                    let text = if args.length() > 1 {
                        args.get(1).to_rust_string_lossy(scope)
                    } else {
                        String::new()
                    };
                    let stdout = io::stdout();
                    let mut writer = stdout.lock();
                    let _ = writeln!(writer, "{}", text);
                    let _ = writer.flush();
                    retval.set(v8::Boolean::new(scope, true).into());
                }
                _ => {}
            }
        },
    )
    .get_function(scope)
    .unwrap();

    let native_key = v8::String::new(scope, "__bee_mcp_native").unwrap();
    global.set(scope, native_key.into(), mcp_native_fn.into());

    let js_code = r#"
    (function() {
        const MCP_PROTOCOL_VERSION = '2024-11-05';

        /**
         * Model Context Protocol (MCP) Server
         */
        class McpServer {
            constructor(info = {}) {
                this.name = info.name || 'beejs-mcp-server';
                this.version = info.version || '1.0.0';
                this.tools = new Map();
                this.resources = new Map();
                this.prompts = new Map();
                this._isRunning = false;
            }

            /**
             * Register an MCP tool
             * @param {string} name - Unique tool identifier
             * @param {string|object} descriptionOrOptions - Tool description or options object
             * @param {object} [schema] - JSON Schema for input arguments
             * @param {Function} handler - Async execution handler (args) => any
             */
            tool(name, descriptionOrOptions, schema, handler) {
                if (typeof name !== 'string' || !name) {
                    throw new TypeError('Tool name must be a non-empty string');
                }

                let description = '';
                let inputSchema = { type: 'object' };
                let fn = handler;

                if (typeof descriptionOrOptions === 'function') {
                    fn = descriptionOrOptions;
                } else if (typeof descriptionOrOptions === 'string') {
                    description = descriptionOrOptions;
                    if (typeof schema === 'function') {
                        fn = schema;
                    } else if (schema && typeof schema === 'object') {
                        inputSchema = schema;
                    }
                } else if (descriptionOrOptions && typeof descriptionOrOptions === 'object') {
                    description = descriptionOrOptions.description || '';
                    inputSchema = descriptionOrOptions.inputSchema || descriptionOrOptions.schema || { type: 'object' };
                    fn = schema || descriptionOrOptions.handler;
                }

                if (typeof fn !== 'function') {
                    throw new TypeError(`Handler for tool '${name}' must be a function`);
                }

                // If inputSchema doesn't have properties/type, wrap if simple key-value was passed
                if (!inputSchema.type) {
                    inputSchema = {
                        type: 'object',
                        properties: inputSchema
                    };
                }

                this.tools.set(name, {
                    name,
                    description,
                    inputSchema,
                    handler: fn
                });

                return this;
            }

            /**
             * Register an MCP resource
             */
            resource(first, second, third, fourth, fifth) {
                let uri, name, description = '', mimeType = 'application/json', handler;
                if (typeof first === 'string' && (first.includes('://') || first.startsWith('/'))) {
                    uri = first;
                    if (typeof second === 'function') {
                        name = uri;
                        handler = second;
                        if (typeof third === 'string') mimeType = third;
                    } else if (typeof second === 'string') {
                        name = second;
                        if (typeof third === 'function') {
                            handler = third;
                            if (typeof fourth === 'string') mimeType = fourth;
                        } else if (typeof third === 'string') {
                            description = third;
                            if (typeof fourth === 'function') {
                                handler = fourth;
                                if (typeof fifth === 'string') mimeType = fifth;
                            }
                        }
                    }
                } else if (typeof second === 'string' && (second.includes('://') || second.startsWith('/'))) {
                    name = first;
                    uri = second;
                    if (typeof third === 'function') {
                        handler = third;
                        if (typeof fourth === 'string') mimeType = fourth;
                    } else if (typeof third === 'string') {
                        description = third;
                        if (typeof fourth === 'function') {
                            handler = fourth;
                            if (typeof fifth === 'string') mimeType = fifth;
                        }
                    }
                } else {
                    uri = first;
                    name = second || first;
                    handler = typeof third === 'function' ? third : fifth;
                }

                this.resources.set(uri, {
                    uri,
                    name: name || uri,
                    description,
                    mimeType,
                    handler
                });
                return this;
            }

            /**
             * Register an MCP prompt
             */
            prompt(name, description, args, handler) {
                let fn = handler;
                let promptArgs = args || [];
                let desc = description || '';

                if (typeof description === 'function') {
                    fn = description;
                    desc = '';
                } else if (typeof args === 'function') {
                    fn = args;
                    promptArgs = [];
                }

                this.prompts.set(name, {
                    name,
                    description: desc,
                    arguments: promptArgs,
                    handler: fn
                });
                return this;
            }

            /**
             * Handle an incoming JSON-RPC 2.0 message string or object
             */
            async handleMessage(rawMessage) {
                let req;
                try {
                    req = typeof rawMessage === 'string' ? JSON.parse(rawMessage) : rawMessage;
                } catch (e) {
                    return {
                        jsonrpc: '2.0',
                        id: null,
                        error: { code: -32700, message: 'Parse error: ' + e.message }
                    };
                }

                if (!req || typeof req !== 'object') {
                    return {
                        jsonrpc: '2.0',
                        id: null,
                        error: { code: -32600, message: 'Invalid Request' }
                    };
                }

                const id = req.id !== undefined ? req.id : null;
                const method = req.method;

                try {
                    switch (method) {
                        case 'initialize':
                            return {
                                jsonrpc: '2.0',
                                id,
                                result: {
                                    protocolVersion: MCP_PROTOCOL_VERSION,
                                    capabilities: {
                                        tools: { listChanged: false },
                                        resources: { subscribe: false, listChanged: false },
                                        prompts: { listChanged: false }
                                    },
                                    serverInfo: {
                                        name: this.name,
                                        version: this.version
                                    }
                                }
                            };

                        case 'notifications/initialized':
                            // Client confirmation, no response required
                            return null;

                        case 'ping':
                            return { jsonrpc: '2.0', id, result: {} };

                        case 'tools/list': {
                            const toolList = Array.from(this.tools.values()).map(t => ({
                                name: t.name,
                                description: t.description,
                                inputSchema: t.inputSchema
                            }));
                            return {
                                jsonrpc: '2.0',
                                id,
                                result: { tools: toolList }
                            };
                        }

                        case 'tools/call': {
                            const params = req.params || {};
                            const toolName = params.name || params.tool;
                            const toolArgs = params.arguments || params.args || {};

                            const tool = this.tools.get(toolName);
                            if (!tool) {
                                return {
                                    jsonrpc: '2.0',
                                    id,
                                    error: { code: -32601, message: `Tool not found: '${toolName}'` }
                                };
                            }

                            const output = await tool.handler(toolArgs);
                            // Format according to MCP tools/call content spec
                            let content;
                            if (output && typeof output === 'object' && Array.isArray(output.content)) {
                                content = output.content;
                            } else if (typeof output === 'string') {
                                content = [{ type: 'text', text: output }];
                            } else {
                                content = [{ type: 'text', text: JSON.stringify(output) }];
                            }

                            return {
                                jsonrpc: '2.0',
                                id,
                                result: {
                                    content,
                                    isError: false
                                }
                            };
                        }

                        case 'resources/list': {
                            const resList = Array.from(this.resources.values()).map(r => ({
                                uri: r.uri,
                                name: r.name,
                                description: r.description,
                                mimeType: r.mimeType
                            }));
                            return {
                                jsonrpc: '2.0',
                                id,
                                result: { resources: resList }
                            };
                        }

                        case 'resources/read': {
                            const uri = req.params && req.params.uri;
                            const res = this.resources.get(uri);
                            if (!res) {
                                return {
                                    jsonrpc: '2.0',
                                    id,
                                    error: { code: -32002, message: `Resource not found: '${uri}'` }
                                };
                            }
                            const textOrData = await res.handler(uri);
                            return {
                                jsonrpc: '2.0',
                                id,
                                result: {
                                    contents: [{
                                        uri: res.uri,
                                        mimeType: res.mimeType,
                                        text: typeof textOrData === 'string' ? textOrData : JSON.stringify(textOrData)
                                    }]
                                }
                            };
                        }

                        case 'prompts/list': {
                            const promptList = Array.from(this.prompts.values()).map(p => ({
                                name: p.name,
                                description: p.description,
                                arguments: p.arguments
                            }));
                            return {
                                jsonrpc: '2.0',
                                id,
                                result: { prompts: promptList }
                            };
                        }

                        case 'prompts/get': {
                            const promptName = req.params && req.params.name;
                            const promptArgs = (req.params && req.params.arguments) || {};
                            const prompt = this.prompts.get(promptName);
                            if (!prompt) {
                                return {
                                    jsonrpc: '2.0',
                                    id,
                                    error: { code: -32601, message: `Prompt not found: '${promptName}'` }
                                };
                            }
                            const res = await prompt.handler(promptArgs);
                            return {
                                jsonrpc: '2.0',
                                id,
                                result: res
                            };
                        }

                        default:
                            return {
                                jsonrpc: '2.0',
                                id,
                                error: { code: -32601, message: `Method not found: '${method}'` }
                            };
                    }
                } catch (err) {
                    return {
                        jsonrpc: '2.0',
                        id,
                        error: { code: -32000, message: err.message || String(err) }
                    };
                }
            }

            /**
             * Connect a local in-memory McpClient directly to this server
             */
            connectLocal() {
                return new McpClient({
                    transport: {
                        send: async (request) => {
                            return this.handleMessage(request);
                        }
                    }
                });
            }

            /**
             * Start stdio server loop for Claude Desktop, Cursor, or Agent hosts
             */
            startStdio() {
                this._isRunning = true;
                const native = globalThis.__bee_mcp_native;
                if (!native) {
                    throw new Error('Native MCP stdio driver not found in runtime');
                }

                while (this._isRunning) {
                    const line = native('read_line');
                    if (line === null) {
                        break;
                    }
                    const trimmed = line.trim();
                    if (!trimmed) continue;

                    // Handle synchronously or invoke callback
                    // MinimalRuntime is single-threaded V8 execution
                    const responsePromise = this.handleMessage(trimmed);
                    if (responsePromise && typeof responsePromise.then === 'function') {
                        // In local loop, handle promise
                        responsePromise.then(res => {
                            if (res) {
                                native('write_line', JSON.stringify(res));
                            }
                        });
                    } else if (responsePromise) {
                        native('write_line', JSON.stringify(responsePromise));
                    }
                }
            }
        }

        /**
         * Model Context Protocol (MCP) Client
         */
        class McpClient {
            constructor(options = {}) {
                this.transport = options.transport;
                this._nextId = 1;
                this.serverInfo = null;
                this.capabilities = null;
            }

            async _request(method, params = {}) {
                if (!this.transport || typeof this.transport.send !== 'function') {
                    throw new Error('McpClient transport not configured');
                }
                const id = this._nextId++;
                const payload = {
                    jsonrpc: '2.0',
                    id,
                    method,
                    params
                };
                const res = await this.transport.send(payload);
                if (res.error) {
                    const err = new Error(res.error.message || 'MCP Error');
                    err.code = res.error.code;
                    throw err;
                }
                return res.result;
            }

            async initialize(clientInfo = { name: 'beejs-mcp-client', version: '1.0.0' }) {
                const res = await this._request('initialize', {
                    protocolVersion: MCP_PROTOCOL_VERSION,
                    capabilities: {},
                    clientInfo
                });
                this.serverInfo = res.serverInfo;
                this.capabilities = res.capabilities;
                return res;
            }

            async ping() {
                await this._request('ping');
                return true;
            }

            async listTools() {
                const res = await this._request('tools/list');
                return (res && res.tools) || [];
            }

            async callTool(name, args = {}) {
                const res = await this._request('tools/call', {
                    name,
                    arguments: args
                });
                if (res && res.content && Array.isArray(res.content)) {
                    if (res.content.length === 1 && res.content[0].type === 'text') {
                        const raw = res.content[0].text;
                        try {
                            return JSON.parse(raw);
                        } catch {
                            return raw;
                        }
                    }
                    return res.content;
                }
                return res;
            }

            async listResources() {
                const res = await this._request('resources/list');
                return (res && res.resources) || [];
            }

            async readResource(uri) {
                const res = await this._request('resources/read', { uri });
                return (res && res.contents) || [];
            }

            async listPrompts() {
                const res = await this._request('prompts/list');
                return (res && res.prompts) || [];
            }

            async getPrompt(name, args = {}) {
                return this._request('prompts/get', { name, arguments: args });
            }
        }

        const beeMcp = {
            McpServer,
            McpClient,
            PROTOCOL_VERSION: MCP_PROTOCOL_VERSION,
            version: '1.4.0'
        };

        globalThis.__bee_mcp = beeMcp;
        globalThis.mcp = beeMcp;
        globalThis.McpServer = McpServer;
        globalThis.McpClient = McpClient;
    })();
    "#;

    let script_source = v8::String::new(scope, js_code).unwrap();
    if let Some(script) = v8::Script::compile(scope, script_source, None) {
        let _ = script.run(scope);
    }

    Ok(())
}
