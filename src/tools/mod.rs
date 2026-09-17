//! Amber v1.7.0: Agent Tool Auto-Synthesis & OpenAPI Schema Compiler (`amber:tools` / `amber:ai.tools`)
//!
//! Provides dynamic JSON Schema tool compilation, automatic OpenAPI 3.x to Agent tool
//! synthesis backed by native `fetch`, LLM tool call parsing, and seamless integration
//! with `amber:ai.AgentPipeline`.

use anyhow::Result;
use rusty_v8 as v8;

/// Sets up the `amber:tools` API inside V8 Context
pub fn setup_tools_api(scope: &mut v8::PinScope, _context: &v8::Local<v8::Context>) -> Result<()> {
    let tools_js_bootstrap = r#"
    (function() {
        // --- 1. Schema Validator & Coercer ---
        function validateAgainstSchema(schema, data) {
            if (!schema || typeof schema !== 'object') {
                return { valid: true, value: data };
            }

            if (data === undefined || data === null) {
                if (schema.default !== undefined) {
                    return { valid: true, value: schema.default };
                }
                return { valid: true, value: data };
            }

            const expectedType = schema.type;
            const coerced = { ...data };

            if (expectedType === 'object') {
                if (typeof data !== 'object' || Array.isArray(data)) {
                    return { valid: false, error: `Expected object, received ${typeof data}` };
                }

                // Check required properties
                if (Array.isArray(schema.required)) {
                    for (const reqKey of schema.required) {
                        if (data[reqKey] === undefined) {
                            if (schema.properties && schema.properties[reqKey] && schema.properties[reqKey].default !== undefined) {
                                coerced[reqKey] = schema.properties[reqKey].default;
                            } else {
                                return { valid: false, error: `Missing required property: '${reqKey}'` };
                            }
                        }
                    }
                }

                // Check and coerce property types
                if (schema.properties && typeof schema.properties === 'object') {
                    for (const [key, propSchema] of Object.entries(schema.properties)) {
                        if (coerced[key] !== undefined) {
                            const res = validateAgainstSchema(propSchema, coerced[key]);
                            if (!res.valid) {
                                return { valid: false, error: `Field '${key}': ${res.error}` };
                            }
                            coerced[key] = res.value;
                        } else if (propSchema.default !== undefined) {
                            coerced[key] = propSchema.default;
                        }
                    }
                }
                return { valid: true, value: coerced };
            }

            if (expectedType === 'string') {
                if (typeof data !== 'string') {
                    return { valid: false, error: `Expected string, received ${typeof data}` };
                }
                if (Array.isArray(schema.enum) && !schema.enum.includes(data)) {
                    return { valid: false, error: `Value '${data}' not in allowed enum: [${schema.enum.join(', ')}]` };
                }
                return { valid: true, value: data };
            }

            if (expectedType === 'number' || expectedType === 'integer') {
                const num = Number(data);
                if (isNaN(num)) {
                    return { valid: false, error: `Expected number, received '${data}'` };
                }
                if (expectedType === 'integer' && !Number.isInteger(num)) {
                    return { valid: false, error: `Expected integer, received '${data}'` };
                }
                return { valid: true, value: num };
            }

            if (expectedType === 'boolean') {
                if (typeof data === 'boolean') return { valid: true, value: data };
                if (data === 'true') return { valid: true, value: true };
                if (data === 'false') return { valid: true, value: false };
                return { valid: false, error: `Expected boolean, received '${data}'` };
            }

            if (expectedType === 'array') {
                if (!Array.isArray(data)) {
                    return { valid: false, error: `Expected array, received ${typeof data}` };
                }
                if (schema.items) {
                    const items = [];
                    for (let i = 0; i < data.length; i++) {
                        const res = validateAgainstSchema(schema.items, data[i]);
                        if (!res.valid) {
                            return { valid: false, error: `Array index ${i}: ${res.error}` };
                        }
                        items.push(res.value);
                    }
                    return { valid: true, value: items };
                }
                return { valid: true, value: data };
            }

            return { valid: true, value: data };
        }

        // --- 2. Tool Compiler ---
        class AgentTool {
            constructor({ name, description, parameters, execute }) {
                if (!name || typeof name !== 'string') {
                    throw new TypeError('Tool must have a valid string name');
                }
                if (typeof execute !== 'function') {
                    throw new TypeError(`Tool '${name}' must have an execute function`);
                }
                this.name = name;
                this.description = description || '';
                this.parameters = parameters || { type: 'object', properties: {} };
                this._executeFn = execute;
            }

            async execute(args = {}) {
                const check = validateAgainstSchema(this.parameters, args);
                if (!check.valid) {
                    throw new Error(`Tool '${this.name}' argument validation failed: ${check.error}`);
                }
                return await this._executeFn(check.value);
            }

            toJSON() {
                return {
                    type: 'function',
                    function: {
                        name: this.name,
                        description: this.description,
                        parameters: this.parameters
                    }
                };
            }
        }

        function compileSchemaTool(def) {
            return new AgentTool(def);
        }

        // --- 3. OpenAPI 3.x Auto-Synthesis ---
        function fromOpenAPI(specOrJson, options = {}) {
            let spec = specOrJson;
            if (typeof spec === 'string') {
                try {
                    spec = JSON.parse(spec);
                } catch (e) {
                    throw new Error(`Invalid OpenAPI JSON string: ${e.message}`);
                }
            }

            if (!spec || typeof spec !== 'object' || !spec.paths) {
                throw new Error('Invalid OpenAPI specification: missing paths object');
            }

            const baseUrl = options.baseUrl ||
                (spec.servers && spec.servers[0] && spec.servers[0].url) ||
                'http://localhost';
            const defaultHeaders = options.headers || {};

            const toolsList = [];
            const toolsMap = {};

            const httpMethods = ['get', 'post', 'put', 'delete', 'patch', 'options', 'head'];

            for (const [pathKey, pathItem] of Object.entries(spec.paths)) {
                if (!pathItem || typeof pathItem !== 'object') continue;

                for (const method of httpMethods) {
                    const op = pathItem[method];
                    if (!op || typeof op !== 'object') continue;

                    let toolName = op.operationId;
                    if (!toolName) {
                        const cleanPath = pathKey.replace(/[^a-zA-Z0-9]/g, '_').replace(/^_+|_+$/g, '');
                        toolName = `${method}_${cleanPath}`;
                    }

                    const description = op.summary || op.description || `${method.toUpperCase()} ${pathKey}`;

                    // Build unified parameters schema
                    const properties = {};
                    const required = [];

                    // 1. Path & Query parameters
                    const allParams = [...(pathItem.parameters || []), ...(op.parameters || [])];
                    for (const p of allParams) {
                        if (!p || !p.name) continue;
                        const pSchema = p.schema || { type: 'string' };
                        properties[p.name] = {
                            ...pSchema,
                            description: p.description || pSchema.description
                        };
                        if (p.required) {
                            required.push(p.name);
                        }
                    }

                    // 2. Request body schema
                    let hasRequestBody = false;
                    if (op.requestBody && op.requestBody.content) {
                        const jsonContent = op.requestBody.content['application/json'];
                        if (jsonContent && jsonContent.schema) {
                            hasRequestBody = true;
                            if (jsonContent.schema.type === 'object' && jsonContent.schema.properties) {
                                for (const [k, v] of Object.entries(jsonContent.schema.properties)) {
                                    properties[k] = v;
                                }
                                if (Array.isArray(jsonContent.schema.required)) {
                                    required.push(...jsonContent.schema.required);
                                }
                            } else {
                                properties['body'] = jsonContent.schema;
                                if (op.requestBody.required) required.push('body');
                            }
                        }
                    }

                    const parameters = {
                        type: 'object',
                        properties,
                        required: Array.from(new Set(required))
                    };

                    const execute = async (args = {}) => {
                        let finalUrl = `${baseUrl.replace(/\/+$/, '')}${pathKey}`;
                        const queryParams = new URLSearchParams();
                        let bodyPayload = undefined;

                        for (const [k, v] of Object.entries(args)) {
                            const placeholder = `{${k}}`;
                            if (finalUrl.includes(placeholder)) {
                                finalUrl = finalUrl.replace(placeholder, encodeURIComponent(String(v)));
                            } else if (hasRequestBody && properties['body'] && k === 'body') {
                                bodyPayload = JSON.stringify(v);
                            } else if (['post', 'put', 'patch'].includes(method) && !allParams.some(p => p.name === k)) {
                                if (!bodyPayload) bodyPayload = {};
                                if (typeof bodyPayload === 'object') bodyPayload[k] = v;
                            } else {
                                queryParams.append(k, String(v));
                            }
                        }

                        const queryString = queryParams.toString();
                        if (queryString) {
                            finalUrl += (finalUrl.includes('?') ? '&' : '?') + queryString;
                        }

                        const reqHeaders = {
                            'Accept': 'application/json',
                            ...defaultHeaders
                        };

                        let body = undefined;
                        if (bodyPayload) {
                            reqHeaders['Content-Type'] = 'application/json';
                            body = typeof bodyPayload === 'string' ? bodyPayload : JSON.stringify(bodyPayload);
                        }

                        const resp = await fetch(finalUrl, {
                            method: method.toUpperCase(),
                            headers: reqHeaders,
                            body
                        });

                        const text = await resp.text();
                        try {
                            return JSON.parse(text);
                        } catch {
                            return text;
                        }
                    };

                    const tool = new AgentTool({
                        name: toolName,
                        description,
                        parameters,
                        execute
                    });

                    toolsList.push(tool);
                    toolsMap[toolName] = tool;
                }
            }

            toolsList.map = toolsMap;
            return toolsList;
        }

        // --- 4. LLM Tool Call Parser ---
        function parseToolCalls(llmOutput) {
            if (!llmOutput) return [];
            const text = typeof llmOutput === 'string' ? llmOutput.trim() : JSON.stringify(llmOutput);

            // 1. Direct JSON parse
            try {
                const parsed = JSON.parse(text);
                if (Array.isArray(parsed)) {
                    return parsed.map(normalizeToolCall).filter(Boolean);
                }
                if (parsed && typeof parsed === 'object') {
                    if (Array.isArray(parsed.tool_calls)) {
                        return parsed.tool_calls.map(tc => {
                            const fn = tc.function || tc;
                            return {
                                name: fn.name,
                                arguments: typeof fn.arguments === 'string' ? JSON.parse(fn.arguments) : (fn.arguments || {})
                            };
                        });
                    }
                    if ((parsed.name || parsed.tool) && (parsed.arguments || parsed.parameters)) {
                        return [normalizeToolCall(parsed)];
                    }
                }
            } catch {}

            // 2. Extract from markdown ```json ... ``` blocks
            const codeBlockRegex = /```(?:json)?\s*([\s\S]*?)\s*```/g;
            let match;
            const extracted = [];
            while ((match = codeBlockRegex.exec(text)) !== null) {
                try {
                    const blockJson = JSON.parse(match[1].trim());
                    if (Array.isArray(blockJson)) {
                        extracted.push(...blockJson.map(normalizeToolCall).filter(Boolean));
                    } else if (blockJson && typeof blockJson === 'object') {
                        const norm = normalizeToolCall(blockJson);
                        if (norm) extracted.push(norm);
                    }
                } catch {}
            }
            if (extracted.length > 0) return extracted;

            // 3. Fallback: extract single JSON object or array
            const jsonObjRegex = /\{[\s\S]*?"(?:name|tool)"\s*:[\s\S]*?\}/g;
            while ((match = jsonObjRegex.exec(text)) !== null) {
                try {
                    const parsed = JSON.parse(match[0].trim());
                    const norm = normalizeToolCall(parsed);
                    if (norm) extracted.push(norm);
                } catch {}
            }

            return extracted.filter(Boolean);
        }

        function normalizeToolCall(obj) {
            if (!obj || typeof obj !== 'object') return null;
            const name = obj.name || obj.tool || (obj.function && obj.function.name);
            let args = obj.arguments || obj.parameters || (obj.function && obj.function.arguments) || {};
            if (typeof args === 'string') {
                try { args = JSON.parse(args); } catch {}
            }
            if (!name) return null;
            return { name: String(name), arguments: typeof args === 'object' && args !== null ? args : {} };
        }

        // --- 5. Pipeline Registration Helper ---
        function registerTools(pipeline, tools) {
            if (!pipeline || typeof pipeline.registerTool !== 'function') {
                throw new TypeError('registerTools requires an AgentPipeline instance');
            }
            const list = Array.isArray(tools) ? tools : Object.values(tools || {});
            for (const t of list) {
                if (t && t.name && typeof t.execute === 'function') {
                    pipeline.registerTool(t);
                }
            }
            return pipeline;
        }

        const toolsModule = {
            AgentTool,
            compileSchemaTool,
            fromOpenAPI,
            parseToolCalls,
            registerTools,
            default: {
                AgentTool,
                compileSchemaTool,
                fromOpenAPI,
                parseToolCalls,
                registerTools
            }
        };

        globalThis.__amber_tools = toolsModule;
        globalThis.tools = toolsModule;
    })();
    "#;

    if let Some(code) = v8::String::new(scope, tools_js_bootstrap) {
        if let Some(script) = v8::Script::compile(scope, code, None) {
            let _ = script.run(scope);
        }
    }

    Ok(())
}
