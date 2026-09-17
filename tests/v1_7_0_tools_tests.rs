use amberjs::runtime_minimal::MinimalRuntime;
use serial_test::serial;

#[test]
#[serial]
fn test_schema_tool_compilation_and_execution() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const { compileSchemaTool } = require('bee:tools');

    // 1. Define and compile tool
    const searchTool = compileSchemaTool({
        name: 'web_search',
        description: 'Search the web for query',
        parameters: {
            type: 'object',
            properties: {
                query: { type: 'string' },
                limit: { type: 'integer', default: 10 },
                safeMode: { type: 'boolean', default: true }
            },
            required: ['query']
        },
        execute: async (args) => {
            return {
                resultCount: args.limit,
                q: args.query,
                safe: args.safeMode
            };
        }
    });

    (async () => {
        // 2. Execute with full parameters
        const res1 = await searchTool.execute({ query: 'Beejs', limit: 25, safeMode: false });
        if (res1.q !== 'Beejs' || res1.resultCount !== 25 || res1.safe !== false) {
            throw new Error('res1 mismatch: ' + JSON.stringify(res1));
        }

        // 3. Execute with defaults applied
        const res2 = await searchTool.execute({ query: 'V8 Isolate' });
        if (res2.resultCount !== 10 || res2.safe !== true) {
            throw new Error('res2 default filling failed: ' + JSON.stringify(res2));
        }

        // 4. Test validation failure on missing required parameter
        let failed = false;
        try {
            await searchTool.execute({});
        } catch (e) {
            failed = true;
            if (!e.message.includes("Missing required property: 'query'")) {
                throw new Error('Unexpected error message: ' + e.message);
            }
        }
        if (!failed) throw new Error('Validation should have failed for missing query');

        // 5. Test toJSON format
        const jsonDef = searchTool.toJSON();
        if (jsonDef.type !== 'function' || jsonDef.function.name !== 'web_search') {
            throw new Error('toJSON mismatch: ' + JSON.stringify(jsonDef));
        }

        return JSON.stringify({ success: true });
    })()
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_openapi_spec_to_tools_synthesis() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const { fromOpenAPI } = require('bee:tools');

    const mockOpenApiSpec = {
        openapi: '3.0.0',
        info: { title: 'Mock Agent API', version: '1.0.0' },
        servers: [{ url: 'https://api.example.com/v1' }],
        paths: {
            '/users/{id}': {
                get: {
                    operationId: 'getUserById',
                    summary: 'Retrieve user by ID',
                    parameters: [
                        { name: 'id', in: 'path', required: true, schema: { type: 'string' } },
                        { name: 'includeDetails', in: 'query', schema: { type: 'boolean', default: false } }
                    ]
                }
            },
            '/items': {
                post: {
                    operationId: 'createItem',
                    summary: 'Create a new item',
                    requestBody: {
                        required: true,
                        content: {
                            'application/json': {
                                schema: {
                                    type: 'object',
                                    properties: {
                                        title: { type: 'string' },
                                        price: { type: 'number' }
                                    },
                                    required: ['title', 'price']
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    const tools = fromOpenAPI(mockOpenApiSpec, {
        baseUrl: 'https://api.example.com/v1',
        headers: { 'Authorization': 'Bearer test-token' }
    });

    if (tools.length !== 2) throw new Error('Tools count mismatch: ' + tools.length);
    if (!tools.map['getUserById']) throw new Error('getUserById missing');
    if (!tools.map['createItem']) throw new Error('createItem missing');

    // Check getUserById tool parameters
    const getTool = tools.map['getUserById'];
    if (getTool.description !== 'Retrieve user by ID') throw new Error('getTool description mismatch');
    if (!getTool.parameters.properties.id) throw new Error('getTool id param missing');
    if (!getTool.parameters.required.includes('id')) throw new Error('getTool id should be required');

    // Check createItem tool parameters
    const postTool = tools.map['createItem'];
    if (!postTool.parameters.properties.title || !postTool.parameters.properties.price) {
        throw new Error('createItem body properties missing');
    }
    if (!postTool.parameters.required.includes('title')) throw new Error('createItem title should be required');

    JSON.stringify({ success: true, count: tools.length });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}

#[test]
#[serial]
fn test_llm_tool_call_parsing_and_pipeline_registration() {
    let mut runtime = MinimalRuntime::new().expect("MinimalRuntime");

    let code = r#"
    const { parseToolCalls, registerTools, compileSchemaTool } = require('bee:tools');
    const ai = require('bee:ai');

    // 1. Parse markdown code block
    const markdownLlmOutput = "I will check the weather.\n```json\n{\n  \"name\": \"get_weather\",\n  \"arguments\": {\"city\": \"London\"}\n}\n```";
    const calls1 = parseToolCalls(markdownLlmOutput);
    if (calls1.length !== 1 || calls1[0].name !== 'get_weather' || calls1[0].arguments.city !== 'London') {
        throw new Error('Markdown parse failed: ' + JSON.stringify(calls1));
    }

    // 2. Parse OpenAI tool_calls format
    const openAiOutput = JSON.stringify({
        tool_calls: [
            { id: 'call_1', function: { name: 'calc', arguments: JSON.stringify({ a: 10, b: 20 }) } },
            { id: 'call_2', function: { name: 'notify', arguments: { message: 'Done' } } }
        ]
    });
    const calls2 = parseToolCalls(openAiOutput);
    if (calls2.length !== 2 || calls2[0].arguments.a !== 10 || calls2[1].arguments.message !== 'Done') {
        throw new Error('OpenAI format parse failed: ' + JSON.stringify(calls2));
    }

    // 3. Register compiled tool into AgentPipeline
    const pipeline = new ai.AgentPipeline();
    const customTool = compileSchemaTool({
        name: 'reverseText',
        description: 'Reverses input text',
        parameters: {
            type: 'object',
            properties: { text: { type: 'string' } },
            required: ['text']
        },
        execute: (args) => args.text.split('').reverse().join('')
    });

    registerTools(pipeline, [customTool]);
    if (!pipeline.tools.has('reverseText')) throw new Error('Tool was not registered into pipeline');

    JSON.stringify({ success: true });
    "#;

    let res = runtime.execute_code(code).expect("Execution failed");
    assert!(res.contains("\"success\":true"));
}
