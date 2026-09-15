use std::process::Command;

fn bee_path() -> &'static str {
    env!("CARGO_BIN_EXE_bee")
}

#[test]
fn test_candle_tensor_zero_copy_matmul() {
    let script = r#"
        const { Tensor } = require('bee:ai');
        // 2x3 matrix * 3x2 matrix = 2x2 matrix
        const a = new Tensor([
            [1.0, 2.0, 3.0],
            [4.0, 5.0, 6.0]
        ]);
        const b = new Tensor([
            [7.0, 8.0],
            [9.0, 1.0],
            [2.0, 3.0]
        ]);
        const c = a.matmul(b);
        const arr = c.toArray();
        // Row 0: [1*7 + 2*9 + 3*2, 1*8 + 2*1 + 3*3] = [7 + 18 + 6, 8 + 2 + 9] = [31, 19]
        // Row 1: [4*7 + 5*9 + 6*2, 4*8 + 5*1 + 6*3] = [28 + 45 + 12, 32 + 5 + 18] = [85, 55]
        console.log(JSON.stringify({ shape: c.shape, data: arr }));
    "#;
    let output = Command::new(bee_path())
        .args(["eval", script])
        .output()
        .expect("failed to run bee eval");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, r#"{"shape":[2,2],"data":[[31,19],[85,55]]}"#);
}

#[test]
fn test_candle_tensor_softmax_numerical_stability() {
    let script = r#"
        const { Tensor } = require('bee:ai');
        const t = new Tensor([1.0, 2.0, 3.0, 4.0, 5.0]);
        const sm = t.softmax();
        const sum = sm.data.reduce((acc, val) => acc + val, 0);
        console.log(`sum=${sum.toFixed(4)},is_prob=${sm.data[4] > sm.data[0]}`);
    "#;
    let output = Command::new(bee_path())
        .args(["eval", script])
        .output()
        .expect("failed to run bee eval");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, "sum=1.0000,is_prob=true");
}

#[test]
fn test_candle_tensor_dot_and_norm() {
    let script = r#"
        const { Tensor, cosineSimilarity } = require('bee:ai');
        const v1 = new Tensor([3.0, 4.0]); // norm = 5.0
        const v2 = new Tensor([6.0, 8.0]); // norm = 10.0, parallel
        const dot = v1.dot(v2); // 18 + 32 = 50
        const norm1 = v1.norm();
        const sim = cosineSimilarity(v1, v2);
        console.log(`dot=${dot},norm=${norm1},sim=${sim.toFixed(4)}`);
    "#;
    let output = Command::new(bee_path())
        .args(["eval", script])
        .output()
        .expect("failed to run bee eval");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, "dot=50,norm=5,sim=1.0000");
}

#[test]
fn test_candle_embeddings_and_vector_math() {
    let script = r#"
        const { embed, cosineSimilarity } = require('bee:ai');
        const vec1 = embed('artificial intelligence and machine learning', { dimensions: 128 });
        const vec2 = embed('artificial intelligence and machine learning algorithms', { dimensions: 128 });
        const vec3 = embed('completely unrelated culinary recipe with fresh tomatoes', { dimensions: 128 });

        const simRelated = cosineSimilarity(vec1, vec2);
        const simUnrelated = cosineSimilarity(vec1, vec3);
        console.log(`len=${vec1.length},closer=${simRelated > simUnrelated}`);
    "#;
    let output = Command::new(bee_path())
        .args(["eval", script])
        .output()
        .expect("failed to run bee eval");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, "len=128,closer=true");
}

#[test]
fn test_candle_llm_load_and_streaming() {
    let script = r#"
        const { LLM } = require('bee:ai');
        async function test() {
            const llm = await LLM.load('qwen2.5-0.5b-instruct', {
                device: 'cpu',
                maxTokens: 16
            });
            const chunks = [];
            for await (const chunk of llm.generateStream('Explain Rust in 5 words')) {
                chunks.push(chunk);
            }
            console.log(`chunks_count=${chunks.length > 0},has_model=${Boolean(llm.model)}`);
        }
        test();
    "#;
    let output = Command::new(bee_path())
        .args(["eval", script])
        .output()
        .expect("failed to run bee eval");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, "chunks_count=true,has_model=true");
}

#[test]
fn test_candle_agent_tool_pipeline() {
    let script = r#"
        const { LLM, AgentPipeline } = require('bee:ai');
        async function runAgent() {
            const llm = await LLM.load('agent-model');
            const pipeline = new AgentPipeline({
                model: llm,
                systemPrompt: 'You are a helpful calculation agent.'
            });

            pipeline.registerTool({
                name: 'calculate',
                execute: (expr) => eval(expr)
            });

            const result = await pipeline.step('calculate 42 * 2');
            console.log(`status=${result.status},history=${result.historyLength}`);
        }
        runAgent();
    "#;
    let output = Command::new(bee_path())
        .args(["eval", script])
        .output()
        .expect("failed to run bee eval");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, "status=completed,history=2");
}
