/**
 * Amber Agent Tool Auto-Calling & Structured Reasoning Demo
 *
 * Demonstrates:
 * 1. Registering deterministic tools in `AgentPipeline`
 * 2. Invoking LLM reasoning steps
 * 3. Execution loop with conversation history
 */

const { LLM, AgentPipeline } = require('amber:ai');

function evalArithmetic(expr) {
    const source = String(expr).replace(/\s+/g, '');
    if (!source || !/^[0-9+\-]+$/.test(source)) {
        throw new Error('unsupported arithmetic expression');
    }
    let total = 0;
    let sign = 1;
    let digits = '';
    for (const ch of source) {
        if (ch === '+' || ch === '-') {
            if (digits) {
                total += sign * Number(digits);
                digits = '';
            }
            sign = ch === '+' ? 1 : -1;
            continue;
        }
        digits += ch;
    }
    if (digits) {
        total += sign * Number(digits);
    }
    return total;
}

async function main() {
    console.log('=== Amber AgentPipeline Tool Calling ===\n');

    const llm = await LLM.load('amber-agent-orchestrator', {
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
            return evalArithmetic(expr);
        }
    });

    pipeline.registerTool({
        name: 'system_info',
        description: 'Returns runtime system information',
        execute: () => {
            return {
                runtime: 'Amber',
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
