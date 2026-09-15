/**
 * Beejs Native AI Inference Demo (`bee:ai`)
 *
 * Demonstrates:
 * 1. Hardware accelerated zero-copy Tensor operations (Candle Core)
 * 2. High-performance sub-millisecond semantic embeddings & cosine similarity
 * 3. Local streaming LLM inference with Apple Metal / CPU fallback
 */

const { Tensor, LLM, embed, cosineSimilarity } = require('bee:ai');

async function main() {
    console.log('=== Beejs Native AI Acceleration (`bee:ai`) ===\n');

    // 1. Zero-Copy Tensor Math
    console.log('--- 1. Native Tensor Matrix Multiplication (Candle) ---');
    const matA = new Tensor([
        [1.0, 2.0, 3.0],
        [4.0, 5.0, 6.0]
    ]);
    const matB = new Tensor([
        [7.0, 8.0],
        [9.0, 1.0],
        [2.0, 3.0]
    ]);
    const result = matA.matmul(matB);
    console.log('Matrix A (2x3) x Matrix B (3x2) =');
    console.log(result.toArray());

    const sm = new Tensor([0.5, 1.2, 3.8, 0.1]).softmax();
    console.log('Softmax Probabilities:', Array.from(sm.data).map(x => x.toFixed(4)));

    // 2. Semantic Text Embeddings
    console.log('\n--- 2. Native Semantic Embeddings ---');
    const prompt1 = 'High performance JavaScript runtime with V8 and Rust';
    const prompt2 = 'Fast server-side JS engine built with Rust memory safety';
    const prompt3 = 'A traditional French croissant baking recipe';

    const v1 = embed(prompt1, { dimensions: 128 });
    const v2 = embed(prompt2, { dimensions: 128 });
    const v3 = embed(prompt3, { dimensions: 128 });

    console.log(`Embedding Vector Dimension: ${v1.length}`);
    console.log(`Similarity (Tech vs Tech): ${(cosineSimilarity(v1, v2) * 100).toFixed(2)}%`);
    console.log(`Similarity (Tech vs Baking): ${(cosineSimilarity(v1, v3) * 100).toFixed(2)}%`);

    // 3. Local LLM Model Loading & Streaming
    console.log('\n--- 3. Local LLM Streaming Generation ---');
    const model = await LLM.load('qwen2.5-0.5b-instruct', {
        device: 'auto', // Auto detects Apple Metal on macOS, CUDA on Linux, CPU
        temperature: 0.7,
        maxTokens: 32
    });

    console.log(`Model Loaded: ${model.model} on [${model.device}]`);
    process.stdout.write('Prompt: "Why is Rust fast?"\nResponse: ');

    for await (const token of model.generateStream('Why is Rust fast?')) {
        process.stdout.write(token);
    }
    console.log('\n\n[Done] Local inference completed successfully.');
}

main().catch(console.error);
