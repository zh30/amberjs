//! Integration tests for Amber v1.4.0 features
//!
//! Tests:
//! 1. Native Zero-Dependency C ABI FFI (`amber:ffi`, `dlopen`, math functions, `ptr`, `read`, `write`, `readCString`)
//! 2. High-Density Multi-Tenant IsolatePool (`amber:pool`, `IsolatePool`, concurrent isolated execution, statistics)
//! 3. Edge SLM Token Generation & Constrained JSON Schema Decoding (`amber:ai`, `generate`, `generateStream`, schema)

use std::process::Command;

fn amber_path() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

#[test]
fn test_v1_4_0_ffi_native_c_abi() {
    let script = r#"
        const { dlopen, ptr, read, write, readCString, FFIType } = require('amber:ffi');

        // 1. Test math symbols from the platform C library.
        // glibc keeps cos/sin in libm; libc.so.6 does not always re-export them.
        const specs = { cos: { args: ['f64'], returns: 'f64' }, sin: { args: ['f64'], returns: 'f64' } };
        const candidates = process.platform === 'darwin'
            ? ['/usr/lib/libSystem.B.dylib', null]
            : (process.platform === 'win32'
                ? ['msvcrt.dll', null]
                : ['libm.so.6', 'libc.so.6', null]);

        let lib;
        let lastErr;
        for (const libPath of candidates) {
            try {
                lib = dlopen(libPath, { symbols: specs });
                lastErr = null;
                break;
            } catch (e) {
                lastErr = e;
            }
        }
        if (!lib) {
            throw lastErr || new Error('dlopen failed for cos/sin');
        }

        const cosVal = lib.symbols.cos(0.0);
        if (Math.abs(cosVal - 1.0) > 1e-6) {
            throw new Error(`cos(0.0) expected 1.0, got ${cosVal}`);
        }

        const sinVal = lib.symbols.sin(0.0);
        if (Math.abs(sinVal - 0.0) > 1e-6) {
            throw new Error(`sin(0.0) expected 0.0, got ${sinVal}`);
        }

        // 2. Test raw memory pointer inspection and modification
        const buf = new Uint8Array(32);
        buf[0] = 72;  // 'H'
        buf[1] = 105; // 'i'
        buf[2] = 0;   // null terminator

        const p = ptr(buf);
        if (typeof p !== 'bigint' || p === 0n) {
            throw new Error(`Invalid pointer returned: ${p}`);
        }

        // Read C string directly from memory pointer
        const greeting = readCString(p);
        if (greeting !== 'Hi') {
            throw new Error(`Expected 'Hi', got '${greeting}'`);
        }

        // Test direct write and read
        write(p, 4, 'i32', 123456);
        const readVal = read(p, 4, 'i32');
        if (readVal !== 123456) {
            throw new Error(`Expected 123456 from offset 4, got ${readVal}`);
        }

        lib.close();
        console.log(`ffi_cos=${cosVal.toFixed(1)},ffi_str=${greeting},ffi_val=${readVal}`);
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
    assert_eq!(stdout, "ffi_cos=1.0,ffi_str=Hi,ffi_val=123456");
}

#[test]
fn test_v1_4_0_isolate_pool_execution() {
    let script = r#"
        const { IsolatePool } = require('amber:pool');

        async function main() {
            const pool = new IsolatePool({
                minIsolates: 2,
                maxIsolates: 4,
                timeoutMs: 5000
            });

            // Run tasks concurrently in isolated V8 heaps
            const p1 = pool.run("30 * 40");
            const p2 = pool.run("JSON.stringify({ agent: 'amber', isolated: true, version: '1.4.0' })");

            const [res1, res2] = await Promise.all([p1, p2]);

            if (res1 !== 1200) {
                throw new Error(`Expected 1200, got ${res1}`);
            }
            if (res2.agent !== 'amber' || res2.isolated !== true) {
                throw new Error(`Unexpected payload from isolated worker: ${JSON.stringify(res2)}`);
            }

            const stats = pool.stats();
            if (stats.tasksCompleted < 2) {
                throw new Error(`Expected tasksCompleted >= 2, got ${stats.tasksCompleted}`);
            }

            pool.destroy();
            console.log(`pool_res1=${res1},agent=${res2.agent},completed=${stats.tasksCompleted >= 2}`);
        }

        main().catch(err => {
            console.error(err);
            process.exit(1);
        });
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
    assert_eq!(stdout, "pool_res1=1200,agent=amber,completed=true");
}

#[test]
fn test_v1_4_0_edge_slm_structured_generation() {
    let script = r#"
        const { generate, generateStream, LLM } = require('amber:ai');

        async function main() {
            // 1. Text generation with stop sequences
            const resText = await generate("Explain how Amber provides native speed", {
                maxTokens: 16
            });

            if (!resText.text || resText.tokens === 0) {
                throw new Error("Expected non-empty text generation");
            }

            // 2. Constrained JSON Schema decoding
            const schema = {
                type: "object",
                properties: {
                    status: { type: "string" },
                    score: { type: "number" },
                    verified: { type: "boolean" }
                }
            };

            const resJson = await generate("Verify runtime integrity and schema conformance", {
                schema: schema
            });

            const parsed = JSON.parse(resJson.text);
            if (typeof parsed.status !== 'string' || typeof parsed.score !== 'number' || typeof parsed.verified !== 'boolean') {
                throw new Error(`Generated JSON does not match schema: ${resJson.text}`);
            }

            // 3. Streaming token generation
            const chunks = [];
            for await (const chunk of generateStream("Stream tokens for test")) {
                chunks.push(chunk);
            }

            if (chunks.length === 0) {
                throw new Error("Expected stream chunks");
            }
            const fullStreamText = chunks.join('');

            // 4. LLM class integration
            const model = new LLM("amber-slm-0.5b");
            const llmRes = await model.generate("Hello Amber AI");
            if (!llmRes.text) {
                throw new Error("LLM.generate failed");
            }

            console.log(`schema_ok=${parsed.verified},chunks_cnt=${chunks.length > 0},llm_model=${llmRes.model}`);
        }

        main().catch(err => {
            console.error(err);
            process.exit(1);
        });
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
    assert_eq!(
        stdout,
        "schema_ok=true,chunks_cnt=true,llm_model=amber-slm-0.5b"
    );
}
