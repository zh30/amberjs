/**
 * Amber 调试器 - 断点调试示例
 *
 * 演示如何使用 Amber 的高级断点功能
 * 包括条件断点、命中次数断点、日志断点等
 */

// 基础函数 - 用于调试演示
function calculateFactorial(n) {
    if (n <= 1) return 1;  // 断点设置在这一行: 条件断点 n == 5
    return n * calculateFactorial(n - 1);
}

function processArray(arr) {
    console.log("开始处理数组...");
    const result = [];

    for (let i = 0; i < arr.length; i++) {
        // 断点设置在这一行: 条件断点 i > 5
        const doubled = arr[i] * 2;

        // 断点设置在这一行: 命中次数断点 (每 3 次命中一次)
        if (doubled > 10) {
            result.push(doubled);
        }
    }

    return result;
}

function fibonacci(n) {
    // 断点设置在这个函数: 记录断点 - 打印参数和返回值
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

class Calculator {
    constructor() {
        this.history = [];
    }

    add(a, b) {
        const result = a + b;
        this.history.push(`${a} + ${b} = ${result}`);
        return result;
    }

    multiply(a, b) {
        const result = a * b;
        this.history.push(`${a} * ${b} = ${result}`);
        return result;
    }

    divide(a, b) {
        if (b === 0) {
            throw new Error("Division by zero");
        }
        const result = a / b;
        this.history.push(`${a} / ${b} = ${result}`);
        return result;
    }

    getHistory() {
        return this.history;
    }
}

// 异步函数调试
async function fetchData(url) {
    console.log(`Fetching from ${url}...`);

    // 模拟网络请求
    await new Promise(resolve => setTimeout(resolve, 100));

    const data = {
        url,
        timestamp: Date.now(),
        status: "success"
    };

    return data;
}

async function processAsyncData() {
    const urls = [
        "https://api.example.com/users",
        "https://api.example.com/posts",
        "https://api.example.com/comments"
    ];

    const results = [];

    for (const url of urls) {
        // 断点设置在这一行: 异步函数断点
        const data = await fetchData(url);
        results.push(data);
    }

    return results;
}

// 主函数
function main() {
    console.log("=== Amber 断点调试示例 ===\n");

    // 示例 1: 递归函数调试
    console.log("示例 1: 递归函数调试");
    const fact5 = calculateFactorial(5);
    console.log(`5! = ${fact5}\n`);

    // 示例 2: 循环调试
    console.log("示例 2: 循环调试");
    const numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    const processed = processArray(numbers);
    console.log(`处理结果: ${processed}\n`);

    // 示例 3: 类方法调试
    console.log("示例 3: 类方法调试");
    const calc = new Calculator();
    calc.add(10, 20);
    calc.multiply(5, 6);
    calc.divide(100, 4);
    console.log("历史记录:", calc.getHistory(), "\n");

    // 示例 4: 斐波那契数列
    console.log("示例 4: 斐波那契数列");
    const fib10 = fibonacci(10);
    console.log(`fibonacci(10) = ${fib10}\n`);

    // 示例 5: 异步函数调试
    console.log("示例 5: 异步函数调试");
    processAsyncData().then(results => {
        console.log("异步处理结果:");
        results.forEach(r => console.log(`  ${r.url}: ${r.status}`));
        console.log("\n✅ 调试示例完成!");
    }).catch(err => {
        console.error("Error:", err);
    });
}

/**
 * 调试器使用指南
 *
 * 1. 启动调试模式:
 *    amber debug examples/debugging/breakpoint_debug.js
 *
 * 2. 断点类型:
 *
 *    a) 普通断点:
 *       在代码行号上点击设置
 *
 *    b) 条件断点:
 *       右键断点 -> 添加条件: i > 5
 *
 *    c) 命中次数断点:
 *       右键断点 -> 设置命中次数: 每 3 次暂停一次
 *
 *    d) 日志断点 (Logpoints):
 *       右键断点 -> 转换为日志断点
 *       日志信息: "i = {i}, arr[i] = {arr[i]}"
 *
 *    e) 异常断点:
 *       调试器面板 -> 异常断点 -> 捕获所有异常
 *
 * 3. 调试命令:
 *
 *    continue (c)     - 继续执行到下一个断点
 *    next (n)         - 执行下一行代码
 *    step (s)         - 进入函数内部
 *    stepout (so)     - 跳出当前函数
 *    finish (f)       - 完成当前函数
 *    backtrace (bt)   - 显示调用栈
 *    print <var>      - 打印变量值
 *    watch <var>      - 监视变量
 *    quit (q)         - 退出调试器
 *
 * 4. 高级功能:
 *
 *    - 异步栈追踪: 自动追踪 async/await 调用链
 *    - 变量修改: 调试时可以修改变量值
 *    - 条件求值: 在调试器控制台执行任意代码
 *    - 源代码映射: TypeScript 源码调试
 *    - 远程调试: 支持 Chrome DevTools 协议
 *
 * 5. VS Code 集成:
 *
 *    安装 "Amber Debugger" 扩展
 *    配置 .vscode/launch.json:
 *
 *    {
 *      "type": "amberjs",
 *      "request": "launch",
 *      "name": "Debug Amber",
 *      "program": "${workspaceFolder}/examples/debugging/breakpoint_debug.js"
 *    }
 */

// 运行主函数
main();
