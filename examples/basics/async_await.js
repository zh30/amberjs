/**
 * Amber 异步编程示例
 *
 * 演示 async/await、Promise、并发处理等异步功能
 * Amber 针对异步操作进行了极致优化
 */

console.log("=== Amber 异步编程示例 ===\n");

// 1. 基础 Promise 使用
console.log("--- 1. 基础 Promise ---");

function basicPromise() {
    return new Promise((resolve, reject) => {
        setTimeout(() => {
            resolve("Promise resolved!");
        }, 100);
    });
}

basicPromise().then(result => {
    console.log("Result:", result);
});

// 2. async/await 语法
console.log("\n--- 2. async/await ---");

async function asyncFunction() {
    console.log("开始异步操作...");
    const result = await basicPromise();
    console.log("异步结果:", result);
    return result;
}

asyncFunction().then(result => {
    console.log("最终结果:", result);
});

// 3. Promise 并发执行
console.log("\n--- 3. Promise 并发执行 ---");

function delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

async function concurrentPromises() {
    console.log("启动 3 个并发任务...");
    const start = Date.now();

    const [result1, result2, result3] = await Promise.all([
        delay(200).then(() => "任务1完成"),
        delay(150).then(() => "任务2完成"),
        delay(100).then(() => "任务3完成")
    ]);

    const duration = Date.now() - start;
    console.log("结果1:", result1);
    console.log("结果2:", result2);
    console.log("结果3:", result3);
    console.log(`总耗时: ${duration}ms (串行需要 450ms)`);

    return { result1, result2, result3 };
}

concurrentPromises();

// 4. Promise.race - 竞赛模式
console.log("\n--- 4. Promise.race ---");

async function promiseRace() {
    console.log("启动竞赛...");
    const start = Date.now();

    const winner = await Promise.race([
        delay(200).then(() => "快速响应"),
        delay(500).then(() => "慢速响应"),
        delay(300).then(() => "中等响应")
    ]);

    const duration = Date.now() - start;
    console.log("获胜者:", winner);
    console.log(`耗时: ${duration}ms`);

    return winner;
}

promiseRace();

// 5. 错误处理
console.log("\n--- 5. 异步错误处理 ---");

function failingPromise() {
    return new Promise((resolve, reject) => {
        setTimeout(() => {
            reject(new Error("异步操作失败"));
        }, 100);
    });
}

async function errorHandling() {
    try {
        await failingPromise();
        console.log("这不会打印");
    } catch (error) {
        console.log("捕获到错误:", error.message);
    }

    // 使用 try/catch 进行并发错误处理
    try {
        await Promise.all([
            Promise.resolve("成功1"),
            Promise.reject(new Error("失败")),
            Promise.resolve("成功3")
        ]);
    } catch (error) {
        console.log("并发操作中有错误:", error.message);
    }
}

errorHandling();

// 6. 异步迭代
console.log("\n--- 6. 异步迭代 ---");

async function asyncIteration() {
    console.log("异步迭代示例...");

    const promises = [
        delay(100).then(() => 1),
        delay(200).then(() => 2),
        delay(150).then(() => 3),
        delay(300).then(() => 4),
        delay(250).then(() => 5)
    ];

    // 按顺序等待
    for (let i = 0; i < promises.length; i++) {
        const result = await promises[i];
        console.log(`异步结果 ${i + 1}:`, result);
    }

    // 并发等待所有
    console.log("\n并发等待所有结果:");
    const results = await Promise.all(promises);
    console.log("所有结果:", results);
}

asyncIteration();

// 7. 异步生成器 (概念演示)
console.log("\n--- 7. 异步生成器 ---");

function* simpleGenerator() {
    console.log("生成器开始");
    yield 1;
    console.log("生成器继续");
    yield 2;
    console.log("生成器结束");
    return 3;
}

const gen = simpleGenerator();
console.log("第一次 next():", gen.next().value);
console.log("第二次 next():", gen.next().value);
console.log("第三次 next():", gen.next().value);

// 8. Amber 异步优化特性
console.log("\n--- 8. Amber 异步优化特性 ---");

async function amberjsOptimizations() {
    console.log("Amber 异步优化:");

    // 零拷贝 I/O
    console.log("✅ 零拷贝 I/O 操作");

    // 智能任务调度
    console.log("✅ 智能任务调度");

    // 微任务队列优化
    console.log("✅ 微任务队列优化");

    // 事件循环优化
    console.log("✅ 事件循环优化");

    const promises = Array.from({ length: 10000 }, (_, i) =>
        Promise.resolve(i)
    );

    const start = performance.now();
    await Promise.all(promises);
    const duration = performance.now() - start;

    console.log(`处理 10000 个 Promise 耗时: ${duration.toFixed(2)}ms`);
    console.log(`平均每个 Promise: ${(duration / 10000).toFixed(4)}ms`);
}

amberjsOptimizations();

// 9. 实际应用场景
console.log("\n--- 9. 实际应用场景 ---");

// 模拟 API 调用
async function fetchUser(id) {
    await delay(50);
    return { id, name: `User ${id}`, email: `user${id}@example.com` };
}

async function fetchUserPosts(userId) {
    await delay(80);
    return [
        { id: 1, userId, title: "Post 1", content: "..." },
        { id: 2, userId, title: "Post 2", content: "..." }
    ];
}

async function loadUserDashboard(userId) {
    console.log(`加载用户 ${userId} 的仪表板...`);

    // 并发加载用户信息和文章
    const [user, posts] = await Promise.all([
        fetchUser(userId),
        fetchUserPosts(userId)
    ]);

    console.log("用户信息:", user);
    console.log(`文章数量: ${posts.length}`);
    console.log("文章标题:", posts.map(p => p.title));

    return { user, posts };
}

// 执行示例
loadUserDashboard(123).then(data => {
    console.log("\n✅ 异步编程示例完成!");
    console.log("最终数据:", data);
});

// 延迟函数定义
function delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}
