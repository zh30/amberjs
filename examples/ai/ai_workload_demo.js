// AI 工作负载演示脚本
// 展示 Amber 的 AI 批量处理能力

console.log("=== Amber AI 工作负载演示 ===\n");

// 模拟 AI 文本生成任务
function simulateAITask(taskId, type) {
    const startTime = Date.now();

    // 模拟 AI 处理延迟
    const processingTime = Math.random() * 100 + 50; // 50-150ms

    return new Promise((resolve) => {
        setTimeout(() => {
            const result = {
                taskId,
                type,
                timestamp: new Date().toISOString(),
                processingTime,
                status: 'completed'
            };
            resolve(result);
        }, processingTime);
    });
}

// 批量 AI 任务处理示例
async function batchAITasks() {
    console.log("🚀 启动 AI 批量处理...");

    const tasks = [];
    const taskTypes = ['text-generation', 'image-classification', 'embedding', 'translation'];

    // 创建 100 个 AI 任务
    for (let i = 0; i < 100; i++) {
        const taskId = `task-${i}`;
        const type = taskTypes[i % taskTypes.length];
        tasks.push(simulateAITask(taskId, type));
    }

    console.log(`📊 创建了 ${tasks.length} 个 AI 任务`);

    const startTime = Date.now();
    const results = await Promise.all(tasks);
    const totalTime = Date.now() - startTime;

    console.log(`✅ 批量处理完成！`);
    console.log(`   总时间: ${totalTime}ms`);
    console.log(`   平均每任务: ${(totalTime / tasks.length).toFixed(2)}ms`);
    console.log(`   吞吐量: ${(tasks.length / (totalTime / 1000)).toFixed(2)} 任务/秒`);

    // 统计结果
    const stats = {};
    results.forEach(result => {
        stats[result.type] = (stats[result.type] || 0) + 1;
    });

    console.log(`\n📈 任务类型统计:`);
    Object.entries(stats).forEach(([type, count]) => {
        console.log(`   ${type}: ${count} 个任务`);
    });

    return results;
}

// 内存使用监控示例
function monitorMemoryUsage() {
    if (typeof process !== 'undefined' && process.memoryUsage) {
        const mem = process.memoryUsage();
        console.log(`\n💾 内存使用情况:`);
        console.log(`   RSS: ${(mem.rss / 1024 / 1024).toFixed(2)} MB`);
        console.log(`   Heap Used: ${(mem.heapUsed / 1024 / 1024).toFixed(2)} MB`);
        console.log(`   Heap Total: ${(mem.heapTotal / 1024 / 1024).toFixed(2)} MB`);
        console.log(`   External: ${(mem.external / 1024 / 1024).toFixed(2)} MB`);
    }
}

// AI 异步队列示例
async function demonstrateAsyncQueue() {
    console.log("\n⚡ AI 异步队列演示...");

    const queue = [];
    const maxConcurrency = 10;
    let activeTasks = 0;
    let completedTasks = 0;

    function processTask(task) {
        return new Promise((resolve) => {
            const processingTime = Math.random() * 50 + 10; // 10-60ms
            setTimeout(() => {
                activeTasks--;
                completedTasks++;
                resolve(task);
            }, processingTime);
        });
    }

    async function runQueue() {
        while (queue.length > 0 || activeTasks > 0) {
            while (activeTasks < maxConcurrency && queue.length > 0) {
                const task = queue.shift();
                activeTasks++;
                processTask(task).then(() => {
                    if (completedTasks % 20 === 0) {
                        console.log(`   已完成: ${completedTasks}/${totalTasks} 任务`);
                    }
                });
            }
            await new Promise(resolve => setTimeout(resolve, 1));
        }
    }

    const totalTasks = 200;
    console.log(`   创建 ${totalTasks} 个异步任务，并发度: ${maxConcurrency}`);

    for (let i = 0; i < totalTasks; i++) {
        queue.push(`task-${i}`);
    }

    const startTime = Date.now();
    await runQueue();
    const totalTime = Date.now() - startTime;

    console.log(`   ✅ 队列处理完成！`);
    console.log(`   总时间: ${totalTime}ms`);
    console.log(`   吞吐量: ${(totalTasks / (totalTime / 1000)).toFixed(2)} 任务/秒`);
}

// 主函数
async function main() {
    monitorMemoryUsage();

    // 演示批量 AI 任务处理
    await batchAITasks();

    monitorMemoryUsage();

    // 演示异步队列
    await demonstrateAsyncQueue();

    monitorMemoryUsage();

    console.log("\n🎉 AI 工作负载演示完成！");
    console.log("Amber 的 AI 优化特性：");
    console.log("  • AI 批量处理器 - 高效批量任务处理");
    console.log("  • AI 内存预分配 - 智能模型内存管理");
    console.log("  • AI 异步队列 - 高性能任务调度");
    console.log("  • AI 模型接口 - 统一多模型调用");
}

// 运行演示
main().catch(error => {
    console.error("演示过程中出错:", error);
});
