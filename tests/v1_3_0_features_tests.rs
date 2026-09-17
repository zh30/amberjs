//! Integration tests for Amber v1.3.0 features
//!
//! Tests:
//! 1. Native Zero-Dependency Embedding Engine (`amber:ai` embed, embedBatch, cosineSimilarity)
//! 2. Model Context Protocol 2.0 (`amber:mcp` McpServer, McpClient, connectLocal, tools, resources, prompts)
//! 3. Deterministic In-Memory Virtual Filesystem (`--virtual-fs`, `amber:vfs`, `fs` interception, COW)

use std::process::Command;

fn amber_path() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

#[test]
fn test_amber_ai_native_embedding_and_similarity() {
    let script = r#"
        const { embed, embedBatch, cosineSimilarity, Tensor } = require('amber:ai');

        // Test single embedding
        const v1 = embed("rust systems programming");
        if (!(v1 instanceof Float32Array)) {
            throw new Error("embed should return Float32Array by default");
        }
        if (v1.length !== 64) {
            throw new Error(`expected length 64, got ${v1.length}`);
        }

        // Test 128 dimensions
        const v128 = embed("rust systems programming", { dimensions: 128 });
        if (v128.length !== 128) {
            throw new Error(`expected length 128, got ${v128.length}`);
        }

        // Test asTensor
        const vTensor = embed("rust systems programming", { asTensor: true });
        if (!(vTensor instanceof Tensor)) {
            throw new Error("expected Tensor when asTensor: true");
        }

        // Test semantic similarity
        const vSimilar = embed("systems programming with rust language");
        const vDifferent = embed("chocolate cake recipe baking oven flour sugar");

        const simHigh = cosineSimilarity(v1, vSimilar);
        const simLow = cosineSimilarity(v1, vDifferent);

        if (simHigh <= simLow) {
            throw new Error(`Expected related texts to have higher similarity: high=${simHigh}, low=${simLow}`);
        }
        if (simHigh < 0.6) {
            throw new Error(`Expected related similarity >= 0.6, got ${simHigh}`);
        }

        // Test embedBatch
        const batch = embedBatch([
            "first text for testing",
            "second text for testing",
            "third text for testing"
        ]);
        if (!Array.isArray(batch) || batch.length !== 3) {
            throw new Error("embedBatch should return array of length 3");
        }

        console.log(`v1_len=${v1.length},simHigh=${simHigh > 0.6},simLow=${simLow < simHigh},batch=${batch.length}`);
    "#;

    let output = Command::new(amber_path())
        .args(["eval", script])
        .output()
        .expect("failed to execute amber eval");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, "v1_len=64,simHigh=true,simLow=true,batch=3");
}

#[test]
fn test_amber_ai_embedding_with_vector_db() {
    let script = r#"
        const { embed } = require('amber:ai');
        const { VectorDB } = require('amber:vector');

        const db = new VectorDB({ dimensions: 64, metric: 'cosine' });

        const doc1 = "Amber runtime with Rust and V8 engine";
        const doc2 = "Node.js JavaScript server runtime";
        const doc3 = "Italian pasta recipes with tomatoes and basil";

        db.insert("doc1", embed(doc1), { title: "Amber" });
        db.insert("doc2", embed(doc2), { title: "NodeJS" });
        db.insert("doc3", embed(doc3), { title: "Pasta" });

        const query = "V8 engine JavaScript runtime in Rust";
        const results = db.search(embed(query), { topK: 1 });

        if (results.length === 0) {
            throw new Error("No results found in VectorDB");
        }
        console.log(`top=${results[0].id},title=${results[0].metadata.title}`);
    "#;

    let output = Command::new(amber_path())
        .args(["eval", script])
        .output()
        .expect("failed to execute amber eval");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, "top=doc1,title=Amber");
}

#[test]
fn test_mcp_server_and_client_local_transport() {
    let script = r#"
        const { McpServer } = require('amber:mcp');

        const server = new McpServer({ name: 'test-server', version: '1.0.0' });

        // Register a tool
        server.tool(
            'add',
            'Add two numbers together',
            {
                type: 'object',
                properties: {
                    a: { type: 'number' },
                    b: { type: 'number' }
                },
                required: ['a', 'b']
            },
            ({ a, b }) => {
                return { sum: a + b };
            }
        );

        // Register a resource
        server.resource(
            'memo://project/info',
            'Project Information',
            (uri) => {
                return { uri, name: 'Amber', version: '1.3.0' };
            },
            'application/json'
        );

        // Register a prompt
        server.prompt(
            'code_review',
            'Code review assistant',
            [{ name: 'code', description: 'Source code', required: true }],
            ({ code }) => {
                return { prompt: `Please review this code:\n${code}` };
            }
        );

        const client = server.connectLocal();

        async function run() {
            // Ping
            const isAlive = await client.ping();
            if (!isAlive) throw new Error("Ping failed");

            // List tools
            const tools = await client.listTools();
            if (!Array.isArray(tools) || tools.length !== 1 || tools[0].name !== 'add') {
                throw new Error("listTools failed: " + JSON.stringify(tools));
            }

            // Call tool
            const toolRes = await client.callTool('add', { a: 40, b: 2 });
            if (!toolRes || toolRes.sum !== 42) {
                throw new Error("callTool failed: " + JSON.stringify(toolRes));
            }

            // List resources
            const resources = await client.listResources();
            if (!Array.isArray(resources) || resources.length !== 1 || resources[0].uri !== 'memo://project/info') {
                throw new Error("listResources failed: " + JSON.stringify(resources));
            }

            // Read resource
            const resData = await client.readResource('memo://project/info');
            if (!Array.isArray(resData) || resData.length === 0) {
                throw new Error("readResource returned invalid data: " + JSON.stringify(resData));
            }
            const parsedRes = JSON.parse(resData[0].text);
            if (!parsedRes || parsedRes.version !== '1.3.0') {
                throw new Error("readResource failed: " + JSON.stringify(resData));
            }

            // List prompts
            const prompts = await client.listPrompts();
            if (!Array.isArray(prompts) || prompts.length !== 1 || prompts[0].name !== 'code_review') {
                throw new Error("listPrompts failed: " + JSON.stringify(prompts));
            }

            // Get prompt
            const promptData = await client.getPrompt('code_review', { code: 'const x = 1;' });
            if (!promptData || !promptData.prompt.includes('const x = 1;')) {
                throw new Error("getPrompt failed: " + JSON.stringify(promptData));
            }

            console.log(`ping=${isAlive},tool_sum=${toolRes.sum},res_version=${parsedRes.version},prompts=${prompts.length}`);
        }

        run();
    "#;

    let output = Command::new(amber_path())
        .args(["eval", script])
        .output()
        .expect("failed to execute amber eval");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, "ping=true,tool_sum=42,res_version=1.3.0,prompts=1");
}

#[test]
fn test_mcp_cli_inspect() {
    let output = Command::new(amber_path())
        .args(["mcp", "--inspect"])
        .output()
        .expect("failed to execute amber mcp --inspect");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Inspecting MCP Built-in Tools"));
    assert!(stdout.contains("Discovered 3 tool(s)"));
    assert!(stdout.contains("execute_command"));
    assert!(stdout.contains("read_file"));
    assert!(stdout.contains("write_file"));
}

#[test]
fn test_virtual_fs_memory_sandbox_isolation() {
    let test_virtual_file = "/vfs_isolated_sandbox_test/agent_secret_data.json";

    // Ensure that test file does NOT exist on the real host filesystem before test
    assert!(!std::path::Path::new(test_virtual_file).exists());

    let script = format!(
        r#"
        const fs = require('fs');
        const vfs = require('amber:vfs');

        vfs.enable(true);

        const testPath = '{}';

        // File should not exist initially
        if (fs.existsSync(testPath)) {{
            throw new Error("File should not exist initially in VFS");
        }}

        // Write file inside virtual filesystem
        fs.writeFileSync(testPath, JSON.stringify({{ secretKey: 'vfs_sandboxed_key_123' }}));

        // File should exist inside VFS
        if (!fs.existsSync(testPath)) {{
            throw new Error("File should exist in VFS after writeFileSync");
        }}

        // Read file back from VFS
        const content = fs.readFileSync(testPath, 'utf8');
        const parsed = JSON.parse(content);
        if (parsed.secretKey !== 'vfs_sandboxed_key_123') {{
            throw new Error("Mismatch in read content: " + content);
        }}

        // Check listFiles and snapshot
        const files = vfs.listFiles();
        const snap = vfs.snapshot();

        // Delete file from VFS
        fs.unlinkSync(testPath);
        if (fs.existsSync(testPath)) {{
            throw new Error("File should no longer exist after unlinkSync");
        }}

        vfs.disable();

        console.log(`vfs_ok=true,had_file=${{files.length > 0}},snap_enabled=${{snap.enabled}}`);
    "#,
        test_virtual_file
    );

    let output = Command::new(amber_path())
        .args(["eval", &script])
        .output()
        .expect("failed to execute amber eval");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, "vfs_ok=true,had_file=true,snap_enabled=true");

    // Verify critical guarantee: host physical disk was NEVER touched
    assert!(
        !std::path::Path::new(test_virtual_file).exists(),
        "Host disk was modified by sandboxed write!"
    );
}

#[test]
fn test_cli_virtual_fs_flag() {
    let script = r#"
        const fs = require('fs');
        const vfs = require('amber:vfs');

        if (!vfs.isEnabled()) {
            throw new Error("Virtual FS should be enabled via CLI flag");
        }

        fs.writeFileSync('/isolated_agent_output.log', 'executed safely');
        const read = fs.readFileSync('/isolated_agent_output.log', 'utf8');
        console.log(`enabled=${vfs.isEnabled()},read=${read}`);
    "#;

    let output = Command::new(amber_path())
        .args(["eval", "--virtual-fs", script])
        .output()
        .expect("failed to execute amber with --virtual-fs");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, "enabled=true,read=executed safely");
    assert!(!std::path::Path::new("/isolated_agent_output.log").exists());
}
