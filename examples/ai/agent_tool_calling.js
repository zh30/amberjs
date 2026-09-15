/**
 * Beejs Agent Tool Auto-Calling & Structured Reasoning Demo
 *
 * Demonstrates:
 * 1. Registering deterministic tools in `AgentPipeline`
 * 2. Invoking LLM reasoning steps
 * 3. Execution loop with conversation history
 */

const { LLM, AgentPipeline } = require('bee:ai');

async function main() {
    console.log('=== Beejs AgentPipeline Tool Calling ===\n');

    const llm = await LLM.load('bee-agent-orchestrator', {
        device: 'cpu'
    });

    const pipeline = new AgentPipeline({
        model: llm,
        systemPrompt: 'You are an autonomous operations agent equipped with math and system tools.'
    });

    // Register custom tools
    pipeline.registerTool({
        name: 'math_eval',
        description: 'Evaluates arithmetic expressions',
        execute: (expr) => {
            console.log(`  [Tool math_eval called with: "${expr}"]`);
            return eval(expr);
        }
    });

    pipeline.registerTool({
        name: 'system_info',
        description: 'Returns runtime system information',
        execute: () => {
            return {
                runtime: 'Beejs',
                v8Version: process.versions.v8,
                nodeVersion: process.versions.node,
                arch: process.arch
            };
        }
    });

    console.log('Registered Tools:', Array.from(pipeline.tools.keys()));

    // Execute step
    console.log('\nExecuting Agent Step 1...');
    const step1 = await pipeline.step('Calculate the square of 16');
    console.log('Result 1:', step1);

    console.log('\nExecuting Agent Step 2...');
    const step2 = await pipeline.step('Query runtime system version');
    console.log('Result 2:', step2);

    console.log(`\nAgent session complete. Total history turns: ${step2.historyLength}`);
}

main().catch(console.error);
