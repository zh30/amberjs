const assert = require('assert');
const { Tensor, LLM, cosineSimilarity, AgentPipeline, version } = require('amber:ai');

async function main() {
    // 1. Version check (crate version; do not pin a historical string)
    assert.ok(typeof version === 'string' && /^\d+\.\d+\.\d+/.test(version));

    // 2. Tensor 1D creation and dot product
    const t1 = new Tensor([1, 2, 3]);
    const t2 = new Tensor([4, 5, 6]);
    assert.strictEqual(t1.ndim, 1);
    assert.strictEqual(t1.length, 3);
    assert.deepStrictEqual(Array.from(t1.shape), [3]);

    const dotVal = t1.dot(t2);
    // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
    assert.strictEqual(dotVal, 32);

    // 3. Tensor L2 norm and Cosine Similarity
    const vecA = new Tensor([1, 0]);
    const vecB = new Tensor([0, 1]);
    const simOrthogonal = cosineSimilarity(vecA, vecB);
    assert.strictEqual(simOrthogonal, 0);

    const vecC = new Tensor([3, 4]); // norm is 5
    assert.strictEqual(vecC.norm(), 5);

    const simSelf = cosineSimilarity(vecC, vecC);
    assert(Math.abs(simSelf - 1.0) < 1e-6);

    // 4. Tensor 2D creation and Matrix Multiplication
    // A: 2x3, B: 3x2 -> C: 2x2
    const matA = new Tensor([
        [1, 2, 3],
        [4, 5, 6]
    ]);
    assert.strictEqual(matA.ndim, 2);
    assert.deepStrictEqual(Array.from(matA.shape), [2, 3]);

    const matB = new Tensor([
        [7, 8],
        [9, 1],
        [2, 3]
    ]);
    const matC = matA.matmul(matB);
    assert.deepStrictEqual(Array.from(matC.shape), [2, 2]);

    // Row 0: [1*7 + 2*9 + 3*2, 1*8 + 2*1 + 3*3] = [7 + 18 + 6, 8 + 2 + 9] = [31, 19]
    // Row 1: [4*7 + 5*9 + 6*2, 4*8 + 5*1 + 6*3] = [28 + 45 + 12, 32 + 5 + 18] = [85, 55]
    const cArr = matC.toArray();
    assert.strictEqual(cArr[0][0], 31);
    assert.strictEqual(cArr[0][1], 19);
    assert.strictEqual(cArr[1][0], 85);
    assert.strictEqual(cArr[1][1], 55);

    // 5. Softmax
    const logits = new Tensor([1.0, 2.0, 3.0]);
    const probs = logits.softmax();
    let sumProbs = 0;
    for (let i = 0; i < probs.length; i++) sumProbs += probs.data[i];
    assert(Math.abs(sumProbs - 1.0) < 1e-5);

    // 6. LLM Load & Generate
    const model = await LLM.load('qwen-2.5-7b-quant.gguf', { device: 'metal' });
    assert.strictEqual(model.model, 'qwen-2.5-7b-quant.gguf');
    assert.strictEqual(model.device, 'metal');

    const singleRes = await model.generate('Hello');
    assert(singleRes.text.length > 0);
    assert(singleRes.tokens > 0);

    // 7. LLM Stream generation
    const chunks = [];
    for await (const chunk of model.generateStream('Hello Amber')) {
        chunks.push(chunk);
    }
    assert(chunks.length > 0);
    const fullText = chunks.join('');
    assert(fullText.includes('Amber'));

    // 8. Vector Embedding
    const emb = await model.embed('Amber AI Agent');
    assert.strictEqual(emb.ndim, 1);
    assert.strictEqual(emb.length, 64);
    assert(Math.abs(emb.norm() - 1.0) < 1e-4);

    // 9. Agent Pipeline
    const agent = new AgentPipeline({
        model,
        systemPrompt: 'You are an autonomous assistant.'
    });
    const stepRes = await agent.step('Solve task');
    assert.strictEqual(stepRes.status, 'completed');
    assert(stepRes.historyLength >= 2);

    console.log('CONFORMANCE_PASS');
}

main()
    .then(() => {
        process.exit(0);
    })
    .catch(err => {
        console.error(err);
        process.exit(1);
    });
