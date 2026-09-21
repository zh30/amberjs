use std::process::Command;

fn amber_path() -> &'static str {
    env!("CARGO_BIN_EXE_amber")
}

#[test]
fn test_amber_ai_tensor_ops() {
    let script = r#"
        const { Tensor, cosineSimilarity } = require('amber:ai');
        const a = new Tensor([1, 2, 3]);
        const b = new Tensor([4, 5, 6]);
        const dot = a.dot(b);
        const normA = a.norm();
        const sim = cosineSimilarity(a, a);
        console.log(`dot=${dot},normA=${normA.toFixed(2)},sim=${sim.toFixed(1)}`);
    "#;
    let output = Command::new(amber_path())
        .args(["eval", script])
        .output()
        .expect("failed to execute amber eval");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, "dot=32,normA=3.74,sim=1.0");
}

#[test]
fn test_amber_ai_matmul() {
    let script = r#"
        const { Tensor } = require('amber:ai');
        const m1 = new Tensor([[1, 2], [3, 4]]);
        const m2 = new Tensor([[2, 0], [1, 2]]);
        const res = m1.matmul(m2);
        console.log(JSON.stringify(res.toArray()));
    "#;
    let output = Command::new(amber_path())
        .args(["eval", script])
        .output()
        .expect("failed to execute amber eval");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    // [ [1*2+2*1, 1*0+2*2], [3*2+4*1, 3*0+4*2] ] = [ [4, 4], [10, 8] ]
    assert_eq!(stdout, "[[4,4],[10,8]]");
}

#[test]
fn test_amber_ai_llm_and_agent() {
    let script = r#"
        const { LLM, AgentPipeline } = require('amber:ai');
        async function run() {
            const llm = await LLM.load('test-model', { device: 'cpu' });
            const gen = await llm.generate('hello');
            const agent = new AgentPipeline({ model: llm });
            const step = await agent.step('test input');
            console.log(`status=${step.status},finish=${gen.finishReason}`);
        }
        run();
    "#;
    let output = Command::new(amber_path())
        .args(["eval", script])
        .output()
        .expect("failed to execute amber eval");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    assert_eq!(stdout, "status=completed,finish=stop");
}
